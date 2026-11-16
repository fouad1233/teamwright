//! The integrations of a project: GitHub, a Slack channel, and web
//! addresses that listen
//! ([decision 0027](../../../docs/decisions/0027-integrations.md)).
//!
//! What is set here is true of this computer and of one project. An
//! address is a secret, and another computer of the same manager may sit
//! behind other doors, so the settings are neither logged nor synced
//! (`AGENTS.md`, rule 5).

pub mod delivery;
pub mod github;
pub mod messages;

use std::path::PathBuf;
use std::sync::Arc;

use teamwright_domain::{
    EventKind, EventKinds, ProjectId, RepositoryName, WebAddress, integration::MOST_WEB_ADDRESSES,
};

use crate::error::ApplicationError;
use crate::ports::{ProjectRepository, StorageError};

/// The settings of the integrations of one project on this computer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectIntegrations {
    /// The GitHub repository the manager named, or nothing to read it from
    /// the work folder.
    pub repository: Option<RepositoryName>,
    /// The Slack channel events are told in, if one is set.
    pub slack: Option<SlackChannel>,
    /// The web addresses events are sent to, in the order they were added.
    pub web_addresses: Vec<ListeningAddress>,
}

/// A Slack channel, through the incoming webhook address Slack gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlackChannel {
    /// The address of the channel.
    pub address: WebAddress,
    /// The kinds of events that are told in it.
    pub events: EventKinds,
}

/// A web address that listens for events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningAddress {
    /// The address.
    pub address: WebAddress,
    /// The secret the body is signed with, if there is one.
    pub secret: Option<String>,
    /// The kinds of events that are sent to it.
    pub events: EventKinds,
}

/// The kinds of events a new Slack channel is told: what a person in the
/// channel wants to hear of.
#[must_use]
pub fn events_told_in_a_channel() -> EventKinds {
    [
        EventKind::TaskFinished,
        EventKind::WaitsForManager,
        EventKind::AgentStopped,
        EventKind::SprintStarted,
        EventKind::SprintCompleted,
    ]
    .into_iter()
    .collect()
}

/// The kinds of events a new web address is sent: all of them, since a
/// program can choose for itself.
#[must_use]
pub fn events_sent_to_an_address() -> EventKinds {
    EventKind::ALL.iter().copied().collect()
}

/// Keeps the settings of the integrations on this computer.
pub trait IntegrationStore: Send + Sync {
    /// The settings of a project; the default if nothing was set.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    fn integrations(&self, project_id: ProjectId) -> Result<ProjectIntegrations, StorageError>;

    /// Keeps the settings of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    fn save_integrations(
        &self,
        project_id: ProjectId,
        integrations: &ProjectIntegrations,
    ) -> Result<(), StorageError>;

    /// The GitHub program the manager named on this computer, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    fn github_program(&self) -> Result<Option<PathBuf>, StorageError>;

    /// Keeps the GitHub program the manager named, or forgets it.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    fn set_github_program(&self, program: Option<&std::path::Path>) -> Result<(), StorageError>;
}

/// The use cases of the settings of the integrations.
#[derive(Clone)]
pub struct IntegrationSettings {
    projects: Arc<dyn ProjectRepository>,
    store: Arc<dyn IntegrationStore>,
}

impl IntegrationSettings {
    /// Creates the use cases.
    #[must_use]
    pub fn new(projects: Arc<dyn ProjectRepository>, store: Arc<dyn IntegrationStore>) -> Self {
        Self { projects, store }
    }

    /// The settings of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, or if storage fails.
    pub fn of(&self, project_id: ProjectId) -> Result<ProjectIntegrations, ApplicationError> {
        self.require_project(project_id)?;
        Ok(self.store.integrations(project_id)?)
    }

    /// Names the GitHub repository of a project, or forgets the name so
    /// that it is read from the work folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not `owner/name`, if there is no
    /// such project, or if storage fails.
    pub fn set_repository(
        &self,
        project_id: ProjectId,
        name: Option<&str>,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        let repository = name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(RepositoryName::parse)
            .transpose()?;
        self.change(project_id, |integrations| {
            integrations.repository = repository;
            Ok(())
        })
    }

    /// Sets the address of the project's Slack channel, keeping the
    /// choice of events, or takes the channel away.
    ///
    /// # Errors
    ///
    /// Returns an error if the address is not https, if there is no such
    /// project, or if storage fails.
    pub fn set_slack_address(
        &self,
        project_id: ProjectId,
        address: Option<&str>,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        let address = address
            .filter(|address| !address.trim().is_empty())
            .map(|address| WebAddress::parse(address, "the address of a channel"))
            .transpose()?;
        self.change(project_id, |integrations| {
            integrations.slack = address.map(|address| SlackChannel {
                address,
                events: integrations
                    .slack
                    .take()
                    .map_or_else(events_told_in_a_channel, |slack| slack.events),
            });
            Ok(())
        })
    }

    /// Tells, or stops telling, a kind of event in the Slack channel.
    ///
    /// # Errors
    ///
    /// [`ApplicationError::NoChannel`] if no channel is set; an error if
    /// there is no such project, or if storage fails.
    pub fn set_slack_event(
        &self,
        project_id: ProjectId,
        kind: EventKind,
        on: bool,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        self.change(project_id, |integrations| {
            let slack = integrations
                .slack
                .as_mut()
                .ok_or(ApplicationError::NoChannel)?;
            choose(&mut slack.events, kind, on);
            Ok(())
        })
    }

    /// Adds a web address that listens for every kind of event.
    ///
    /// # Errors
    ///
    /// Returns an error if the address is not https, if the project has
    /// as many addresses as it may, if there is no such project, or if
    /// storage fails.
    pub fn add_web_address(
        &self,
        project_id: ProjectId,
        address: &str,
        secret: Option<&str>,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        let address = WebAddress::parse(address, "a web address")?;
        let secret = secret
            .map(str::trim)
            .filter(|secret| !secret.is_empty())
            .map(str::to_owned);
        self.change(project_id, |integrations| {
            if integrations.web_addresses.len() >= MOST_WEB_ADDRESSES {
                return Err(teamwright_domain::DomainError::TooManyWebAddresses {
                    max: MOST_WEB_ADDRESSES,
                }
                .into());
            }
            integrations.web_addresses.push(ListeningAddress {
                address,
                secret,
                events: events_sent_to_an_address(),
            });
            Ok(())
        })
    }

    /// Takes away the web address at a position.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no address there, if there is no such
    /// project, or if storage fails.
    pub fn remove_web_address(
        &self,
        project_id: ProjectId,
        index: usize,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        self.change(project_id, |integrations| {
            address_at(integrations, index)?;
            integrations.web_addresses.remove(index);
            Ok(())
        })
    }

    /// Sends, or stops sending, a kind of event to the web address at a
    /// position.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no address there, if there is no such
    /// project, or if storage fails.
    pub fn set_web_address_event(
        &self,
        project_id: ProjectId,
        index: usize,
        kind: EventKind,
        on: bool,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        self.change(project_id, |integrations| {
            choose(&mut address_at(integrations, index)?.events, kind, on);
            Ok(())
        })
    }

    /// The GitHub program the manager named on this computer, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn github_program(&self) -> Result<Option<PathBuf>, ApplicationError> {
        Ok(self.store.github_program()?)
    }

    /// Names the GitHub program of this computer, or forgets it so that it
    /// is looked for.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn set_github_program(&self, program: Option<&str>) -> Result<(), ApplicationError> {
        let program = program
            .map(str::trim)
            .filter(|program| !program.is_empty())
            .map(PathBuf::from);
        Ok(self.store.set_github_program(program.as_deref())?)
    }

    fn require_project(&self, project_id: ProjectId) -> Result<(), ApplicationError> {
        match self.projects.find(project_id)? {
            Some(_) => Ok(()),
            None => Err(ApplicationError::NotFound {
                kind: "project",
                id: project_id.to_string(),
            }),
        }
    }

    fn change(
        &self,
        project_id: ProjectId,
        change: impl FnOnce(&mut ProjectIntegrations) -> Result<(), ApplicationError>,
    ) -> Result<ProjectIntegrations, ApplicationError> {
        let mut integrations = self.of(project_id)?;
        change(&mut integrations)?;
        self.store.save_integrations(project_id, &integrations)?;
        Ok(integrations)
    }
}

fn choose(events: &mut EventKinds, kind: EventKind, on: bool) {
    if on {
        events.insert(kind);
    } else {
        events.remove(&kind);
    }
}

fn address_at(
    integrations: &mut ProjectIntegrations,
    index: usize,
) -> Result<&mut ListeningAddress, ApplicationError> {
    integrations
        .web_addresses
        .get_mut(index)
        .ok_or_else(|| ApplicationError::NotFound {
            kind: "web address",
            id: (index + 1).to_string(),
        })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use teamwright_domain::{Project, ProjectKey, ProjectName, Workflow};

    use super::*;
    use crate::in_memory::InMemoryProjectRepository;

    #[derive(Default)]
    pub(crate) struct MemoryStore {
        integrations: Mutex<Vec<(ProjectId, ProjectIntegrations)>>,
        program: Mutex<Option<PathBuf>>,
    }

    impl IntegrationStore for MemoryStore {
        fn integrations(&self, project_id: ProjectId) -> Result<ProjectIntegrations, StorageError> {
            Ok(self
                .integrations
                .lock()
                .unwrap()
                .iter()
                .find(|(id, _)| *id == project_id)
                .map(|(_, integrations)| integrations.clone())
                .unwrap_or_default())
        }

        fn save_integrations(
            &self,
            project_id: ProjectId,
            integrations: &ProjectIntegrations,
        ) -> Result<(), StorageError> {
            let mut kept = self.integrations.lock().unwrap();
            kept.retain(|(id, _)| *id != project_id);
            kept.push((project_id, integrations.clone()));
            Ok(())
        }

        fn github_program(&self) -> Result<Option<PathBuf>, StorageError> {
            Ok(self.program.lock().unwrap().clone())
        }

        fn set_github_program(
            &self,
            program: Option<&std::path::Path>,
        ) -> Result<(), StorageError> {
            *self.program.lock().unwrap() = program.map(std::path::Path::to_path_buf);
            Ok(())
        }
    }

    fn settings() -> (IntegrationSettings, ProjectId) {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::standard(),
            chrono::Utc::now(),
        );
        projects.save(&project).unwrap();
        (
            IntegrationSettings::new(projects, Arc::new(MemoryStore::default())),
            project.id(),
        )
    }

    #[test]
    fn a_channel_keeps_its_choice_of_events_when_its_address_changes() {
        let (settings, project) = settings();
        assert!(matches!(
            settings.set_slack_event(project, EventKind::TaskCreated, true),
            Err(ApplicationError::NoChannel)
        ));
        settings
            .set_slack_address(project, Some("https://hooks.slack.com/services/A"))
            .unwrap();
        settings
            .set_slack_event(project, EventKind::AgentStopped, false)
            .unwrap();
        let changed = settings
            .set_slack_address(project, Some("https://hooks.slack.com/services/B"))
            .unwrap();
        let slack = changed.slack.unwrap();
        assert_eq!(slack.address.as_str(), "https://hooks.slack.com/services/B");
        assert!(!slack.events.contains(&EventKind::AgentStopped));
        assert!(slack.events.contains(&EventKind::TaskFinished));
        assert!(
            settings
                .set_slack_address(project, Some("http://hooks.slack.com/x"))
                .is_err()
        );
        assert_eq!(
            settings.set_slack_address(project, None).unwrap().slack,
            None
        );
    }

    #[test]
    fn a_project_has_at_most_five_web_addresses() {
        let (settings, project) = settings();
        for number in 0..MOST_WEB_ADDRESSES {
            settings
                .add_web_address(project, &format!("https://example.com/{number}"), None)
                .unwrap();
        }
        assert!(matches!(
            settings.add_web_address(project, "https://example.com/6", None),
            Err(ApplicationError::Rule(
                teamwright_domain::DomainError::TooManyWebAddresses { max: 5 }
            ))
        ));
        let after = settings.remove_web_address(project, 0).unwrap();
        assert_eq!(
            after.web_addresses[0].address.as_str(),
            "https://example.com/1"
        );
        let after = settings
            .set_web_address_event(project, 0, EventKind::TaskCreated, false)
            .unwrap();
        assert_eq!(
            after.web_addresses[0].events.len(),
            EventKind::ALL.len() - 1
        );
        assert!(settings.remove_web_address(project, 9).is_err());
    }

    #[test]
    fn a_repository_is_named_as_owner_and_name_or_forgotten() {
        let (settings, project) = settings();
        let named = settings
            .set_repository(project, Some(" octo-org/sensor-node "))
            .unwrap();
        assert_eq!(
            named.repository.unwrap().to_string(),
            "octo-org/sensor-node"
        );
        assert!(
            settings
                .set_repository(project, Some("sensor-node"))
                .is_err()
        );
        assert_eq!(
            settings
                .set_repository(project, Some(""))
                .unwrap()
                .repository,
            None
        );
    }
}
