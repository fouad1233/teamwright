//! The integrations of each project on this computer. They are facts about
//! this computer and their addresses are secrets: written without the
//! recorder, so never logged and never synced
//! ([decision 0027](../../../docs/decisions/0027-integrations.md)).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, named_params};
use teamwright_application::{
    IntegrationStore, ListeningAddress, ProjectIntegrations, SlackChannel, StorageError,
};
use teamwright_domain::{EventKind, EventKinds, ProjectId, RepositoryName, WebAddress};

use crate::database::Database;
use crate::error::DatabaseError;

/// The key in `meta` of the GitHub program the manager named.
const GITHUB_PROGRAM: &str = "github_program";

/// Keeps the integrations of each project in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteIntegrationStore {
    database: Arc<Database>,
}

impl SqliteIntegrationStore {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

/// The stored form of a choice of events: their names, separated by
/// spaces.
fn events_to_text(events: &EventKinds) -> String {
    events
        .iter()
        .map(EventKind::as_str)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A choice of events read back. A name this version does not know, kept
/// by a newer one, is left out.
fn events_of_text(text: &str) -> EventKinds {
    text.split_whitespace()
        .filter_map(|name| name.parse().ok())
        .collect()
}

/// An address read back. One that is no longer well formed is left out
/// rather than failing the whole page.
fn address_of_text(text: &str) -> Option<WebAddress> {
    WebAddress::parse(text, "a stored address").ok()
}

fn read(connection: &Connection, project_id: &str) -> Result<ProjectIntegrations, DatabaseError> {
    let row = connection
        .query_row(
            "SELECT repository, slack_address, slack_events
             FROM project_integrations WHERE project_id = :project_id",
            named_params! { ":project_id": project_id },
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    let (repository, slack_address, slack_events) = row.unwrap_or_default();
    let mut statement = connection.prepare(
        "SELECT address, secret, events FROM project_web_addresses
         WHERE project_id = :project_id ORDER BY position",
    )?;
    let web_addresses = statement
        .query_map(named_params! { ":project_id": project_id }, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|(address, secret, events)| {
            Some(ListeningAddress {
                address: address_of_text(&address)?,
                secret,
                events: events_of_text(&events),
            })
        })
        .collect();
    Ok(ProjectIntegrations {
        repository: repository.and_then(|name| RepositoryName::parse(&name).ok()),
        slack: slack_address
            .as_deref()
            .and_then(address_of_text)
            .map(|address| SlackChannel {
                address,
                events: events_of_text(&slack_events),
            }),
        web_addresses,
    })
}

impl IntegrationStore for SqliteIntegrationStore {
    fn integrations(&self, project_id: ProjectId) -> Result<ProjectIntegrations, StorageError> {
        let project_id = project_id.to_string();
        self.database
            .read(|connection| read(connection, &project_id))
    }

    fn save_integrations(
        &self,
        project_id: ProjectId,
        integrations: &ProjectIntegrations,
    ) -> Result<(), StorageError> {
        let project_id = project_id.to_string();
        // Written without the recorder: the integrations are not logged.
        self.database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO project_integrations
                     (project_id, repository, slack_address, slack_events)
                 VALUES (:project_id, :repository, :slack_address, :slack_events)
                 ON CONFLICT (project_id) DO UPDATE SET
                     repository = excluded.repository,
                     slack_address = excluded.slack_address,
                     slack_events = excluded.slack_events",
                named_params! {
                    ":project_id": project_id,
                    ":repository": integrations.repository.as_ref().map(ToString::to_string),
                    ":slack_address": integrations.slack.as_ref().map(|slack| slack.address.as_str()),
                    ":slack_events": integrations
                        .slack
                        .as_ref()
                        .map(|slack| events_to_text(&slack.events))
                        .unwrap_or_default(),
                },
            )?;
            for (position, listening) in integrations.web_addresses.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO project_web_addresses
                         (project_id, position, address, secret, events)
                     VALUES (:project_id, :position, :address, :secret, :events)
                     ON CONFLICT (project_id, position) DO UPDATE SET
                         address = excluded.address,
                         secret = excluded.secret,
                         events = excluded.events",
                    named_params! {
                        ":project_id": project_id,
                        ":position": i64::try_from(position).unwrap_or(i64::MAX),
                        ":address": listening.address.as_str(),
                        ":secret": listening.secret,
                        ":events": events_to_text(&listening.events),
                    },
                )?;
            }
            transaction.execute(
                "DELETE FROM project_web_addresses
                 WHERE project_id = :project_id AND position >= :count",
                named_params! {
                    ":project_id": project_id,
                    ":count": i64::try_from(integrations.web_addresses.len()).unwrap_or(i64::MAX),
                },
            )?;
            Ok(())
        })
    }

    fn github_program(&self) -> Result<Option<PathBuf>, StorageError> {
        self.database.read(|connection| {
            Ok(connection
                .query_row(
                    "SELECT value FROM meta WHERE key = :key",
                    named_params! { ":key": GITHUB_PROGRAM },
                    |row| row.get::<_, String>(0),
                )
                .optional()?
                .map(PathBuf::from))
        })
    }

    fn set_github_program(&self, program: Option<&Path>) -> Result<(), StorageError> {
        self.database.write(|transaction, _| {
            match program {
                Some(program) => transaction.execute(
                    "INSERT INTO meta (key, value) VALUES (:key, :value)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    named_params! {
                        ":key": GITHUB_PROGRAM,
                        ":value": program.to_string_lossy(),
                    },
                )?,
                None => transaction.execute(
                    "DELETE FROM meta WHERE key = :key",
                    named_params! { ":key": GITHUB_PROGRAM },
                )?,
            };
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use teamwright_application::{ProjectRepository, events_sent_to_an_address};
    use teamwright_domain::{Project, ProjectKey, ProjectName, Workflow};

    use super::*;
    use crate::SqliteProjectRepository;

    fn store_with_project() -> (Arc<Database>, SqliteIntegrationStore, ProjectId) {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::standard(),
            chrono::Utc::now(),
        );
        SqliteProjectRepository::new(database.clone())
            .save(&project)
            .unwrap();
        (
            database.clone(),
            SqliteIntegrationStore::new(database),
            project.id(),
        )
    }

    fn listening(address: &str, secret: Option<&str>) -> ListeningAddress {
        ListeningAddress {
            address: WebAddress::parse(address, "it").unwrap(),
            secret: secret.map(str::to_owned),
            events: events_sent_to_an_address(),
        }
    }

    #[test]
    fn the_integrations_are_kept_and_not_logged() {
        let (database, store, project) = store_with_project();
        let operations_before = database.operations_to_send(0).unwrap().len();
        assert_eq!(
            store.integrations(project).unwrap(),
            ProjectIntegrations::default()
        );

        let integrations = ProjectIntegrations {
            repository: RepositoryName::parse("octo-org/sensor-node").ok(),
            slack: Some(SlackChannel {
                address: WebAddress::parse("https://hooks.slack.com/services/A", "it").unwrap(),
                events: [EventKind::TaskFinished, EventKind::SprintStarted]
                    .into_iter()
                    .collect(),
            }),
            web_addresses: vec![
                listening("https://example.com/one", Some("s3cret")),
                listening("https://example.com/two", None),
            ],
        };
        store.save_integrations(project, &integrations).unwrap();
        assert_eq!(store.integrations(project).unwrap(), integrations);

        // One address fewer: the last row goes.
        let fewer = ProjectIntegrations {
            web_addresses: vec![listening("https://example.com/two", None)],
            slack: None,
            ..integrations
        };
        store.save_integrations(project, &fewer).unwrap();
        assert_eq!(store.integrations(project).unwrap(), fewer);

        assert_eq!(
            database.operations_to_send(0).unwrap().len(),
            operations_before
        );
    }

    #[test]
    fn the_github_program_is_kept_and_forgotten() {
        let (_, store, _) = store_with_project();
        assert_eq!(store.github_program().unwrap(), None);
        store
            .set_github_program(Some(Path::new("C:\\Tools\\gh.exe")))
            .unwrap();
        assert_eq!(
            store.github_program().unwrap(),
            Some(PathBuf::from("C:\\Tools\\gh.exe"))
        );
        store.set_github_program(None).unwrap();
        assert_eq!(store.github_program().unwrap(), None);
    }

    #[test]
    fn unknown_names_of_events_are_left_out() {
        assert_eq!(
            events_of_text("task.finished  sprint.someday"),
            [EventKind::TaskFinished].into_iter().collect()
        );
    }
}
