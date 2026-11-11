//! Use cases for projects.

use std::sync::Arc;

use teamwright_domain::{Project, ProjectId, ProjectKey, ProjectName, Workflow};

use crate::error::ApplicationError;
use crate::ports::{Clock, ProjectRepository};

/// The workflows a new project can start with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkflowTemplate {
    /// To do, in progress, in review, done.
    #[default]
    Standard,
    /// To do, in progress, in revision, in test, done.
    Engineering,
}

impl WorkflowTemplate {
    /// Builds a fresh workflow from the template.
    fn build(self) -> Workflow {
        match self {
            Self::Standard => Workflow::standard(),
            Self::Engineering => Workflow::engineering(),
        }
    }
}

/// What is needed to start a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewProject {
    /// The name of the project.
    pub name: String,
    /// The prefix for task numbers. If it is missing or blank, one is
    /// derived from the name.
    pub key: Option<String>,
    /// A free-form description. May be empty.
    pub description: String,
    /// The workflow the project starts with.
    pub workflow: WorkflowTemplate,
}

/// The use cases that start, find and change projects.
#[derive(Clone)]
pub struct ProjectService {
    projects: Arc<dyn ProjectRepository>,
    clock: Arc<dyn Clock>,
}

impl ProjectService {
    /// The highest number that is appended to a suggested key to make it
    /// unique (`IOT2` … `IOT9999`).
    const LAST_KEY_SUFFIX: u32 = 9999;

    /// Creates the service.
    #[must_use]
    pub fn new(projects: Arc<dyn ProjectRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { projects, clock }
    }

    /// Starts a project.
    ///
    /// # Errors
    ///
    /// Returns an error if the name or the key is not valid, if the requested
    /// key belongs to another project, or if storage fails.
    pub fn create(&self, new_project: NewProject) -> Result<Project, ApplicationError> {
        let name = ProjectName::parse(&new_project.name)?;
        let existing = self.projects.list()?;

        let requested_key = new_project
            .key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty());
        let key = match requested_key {
            Some(requested) => {
                let key = ProjectKey::parse(requested)?;
                if is_key_in_use(&existing, &key) {
                    return Err(ApplicationError::ProjectKeyTaken {
                        key: key.to_string(),
                    });
                }
                key
            }
            None => Self::unused_key_for(&name, &existing),
        };

        let project = Project::create(
            name,
            key,
            new_project.description.trim().to_owned(),
            new_project.workflow.build(),
            self.clock.now(),
        );
        self.projects.save(&project)?;
        Ok(project)
    }

    /// Lists the projects that are not archived, oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn list(&self) -> Result<Vec<Project>, ApplicationError> {
        let projects = self.projects.list()?;
        Ok(projects
            .into_iter()
            .filter(|project| !project.is_archived())
            .collect())
    }

    /// Finds a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn get(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Gives a project a new name.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not valid, if there is no such
    /// project, or if storage fails.
    pub fn rename(&self, id: ProjectId, name: &str) -> Result<Project, ApplicationError> {
        let name = ProjectName::parse(name)?;
        self.change(id, |project| project.rename(name))
    }

    /// Replaces the description of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn describe(&self, id: ProjectId, description: &str) -> Result<Project, ApplicationError> {
        self.change(id, |project| {
            project.describe(description.trim().to_owned())
        })
    }

    /// Puts a project away. Its tasks are kept.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn archive(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        let now = self.clock.now();
        self.change(id, |project| project.archive(now))
    }

    /// Loads a project, changes it and stores it.
    fn change(
        &self,
        id: ProjectId,
        apply: impl FnOnce(&mut Project),
    ) -> Result<Project, ApplicationError> {
        let mut project = self.get(id)?;
        apply(&mut project);
        self.projects.save(&project)?;
        Ok(project)
    }

    /// Derives a key from a project name that no existing project uses.
    ///
    /// If the natural key is taken, a number is appended, and letters are
    /// dropped from the end as needed to stay within the length limit:
    /// `IOT`, `IOT2`, `IOT3`, …
    fn unused_key_for(name: &ProjectName, existing: &[Project]) -> ProjectKey {
        let suggested = ProjectKey::suggest(name);
        if !is_key_in_use(existing, &suggested) {
            return suggested;
        }

        (2..=Self::LAST_KEY_SUFFIX)
            .filter_map(|number| {
                let suffix = number.to_string();
                let letters_kept = ProjectKey::MAX_LENGTH.saturating_sub(suffix.len());
                let stem: String = suggested.as_str().chars().take(letters_kept).collect();
                ProjectKey::parse(&format!("{stem}{suffix}")).ok()
            })
            .find(|candidate| !is_key_in_use(existing, candidate))
            // Only reachable with thousands of projects that share a key
            // stem. Task numbers stay usable even with a repeated key.
            .unwrap_or(suggested)
    }
}

/// Tells whether any project, archived or not, already has the key.
fn is_key_in_use(projects: &[Project], key: &ProjectKey) -> bool {
    projects.iter().any(|project| project.key() == key)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{DomainError, StatusCategory, Timestamp};

    use super::*;
    use crate::in_memory::{InMemoryProjectRepository, ManualClock};

    fn start_of_day() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn service() -> ProjectService {
        ProjectService::new(
            Arc::new(InMemoryProjectRepository::default()),
            Arc::new(ManualClock::starting_at(start_of_day())),
        )
    }

    fn named(name: &str) -> NewProject {
        NewProject {
            name: name.to_owned(),
            ..NewProject::default()
        }
    }

    #[test]
    fn a_new_project_is_stored_and_can_be_found_again() {
        let service = service();

        let created = service.create(named("Smart sensor node")).unwrap();

        assert_eq!(service.get(created.id()).unwrap(), created);
        assert_eq!(created.name().as_str(), "Smart sensor node");
        assert_eq!(created.created_at(), start_of_day());
    }

    #[test]
    fn a_project_without_a_requested_key_gets_one_from_its_name() {
        let created = service().create(named("Smart sensor node")).unwrap();

        assert_eq!(created.key().as_str(), "SSN");
    }

    #[test]
    fn a_blank_requested_key_counts_as_no_key() {
        let new_project = NewProject {
            key: Some("   ".to_owned()),
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.key().as_str(), "SSN");
    }

    #[test]
    fn a_requested_key_is_used() {
        let new_project = NewProject {
            key: Some("iot".to_owned()),
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.key().as_str(), "IOT");
    }

    #[test]
    fn a_requested_key_that_another_project_uses_is_refused() {
        let service = service();
        service
            .create(NewProject {
                key: Some("IOT".to_owned()),
                ..named("First")
            })
            .unwrap();

        let error = service
            .create(NewProject {
                key: Some("iot".to_owned()),
                ..named("Second")
            })
            .unwrap_err();

        assert!(matches!(error, ApplicationError::ProjectKeyTaken { key } if key == "IOT"));
    }

    #[test]
    fn a_derived_key_that_is_taken_gets_a_number() {
        let service = service();

        let keys: Vec<String> = (0..3)
            .map(|_| {
                service
                    .create(named("Smart sensor node"))
                    .unwrap()
                    .key()
                    .to_string()
            })
            .collect();

        assert_eq!(keys, ["SSN", "SSN2", "SSN3"]);
    }

    #[test]
    fn a_numbered_key_stays_within_the_length_limit() {
        let service = service();
        let long_name = "A very long name with a lot of words in it";
        service.create(named(long_name)).unwrap();

        let second = service.create(named(long_name)).unwrap();

        assert_eq!(second.key().as_str(), "AVLNW2");
    }

    #[test]
    fn the_key_of_an_archived_project_stays_reserved() {
        let service = service();
        let first = service.create(named("Smart sensor node")).unwrap();
        service.archive(first.id()).unwrap();

        let second = service.create(named("Smart sensor node")).unwrap();

        assert_eq!(second.key().as_str(), "SSN2");
    }

    #[test]
    fn a_project_without_a_name_is_refused() {
        let error = service().create(named("   ")).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::EmptyText {
                field: "project name"
            })
        ));
    }

    #[test]
    fn a_project_starts_with_the_workflow_that_was_chosen() {
        let new_project = NewProject {
            workflow: WorkflowTemplate::Engineering,
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        let names: Vec<&str> = created
            .workflow()
            .statuses()
            .iter()
            .map(|status| status.name().as_str())
            .collect();
        assert_eq!(
            names,
            ["To do", "In progress", "In revision", "In test", "Done"]
        );
        assert!(
            created
                .workflow()
                .first_in(StatusCategory::Review)
                .is_some()
        );
    }

    #[test]
    fn the_description_is_stored_without_surrounding_whitespace() {
        let new_project = NewProject {
            description: "  A connected sensor.\n".to_owned(),
            ..named("Node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.description(), "A connected sensor.");
    }

    #[test]
    fn projects_are_listed_oldest_first() {
        let service = service();
        let first = service.create(named("First")).unwrap();
        let second = service.create(named("Second")).unwrap();

        let listed = service.list().unwrap();

        assert_eq!(listed, [first, second]);
    }

    #[test]
    fn archived_projects_are_not_listed_but_can_still_be_found() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let archived = service.archive(project.id()).unwrap();

        assert!(service.list().unwrap().is_empty());
        assert_eq!(service.get(project.id()).unwrap(), archived);
        assert_eq!(archived.archived_at(), Some(start_of_day()));
    }

    #[test]
    fn renaming_a_project_changes_its_name_and_keeps_its_key() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let renamed = service.rename(project.id(), "Air quality monitor").unwrap();

        assert_eq!(renamed.name().as_str(), "Air quality monitor");
        assert_eq!(renamed.key().as_str(), "SSN");
        assert_eq!(service.get(project.id()).unwrap(), renamed);
    }

    #[test]
    fn describing_a_project_replaces_its_description() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let described = service
            .describe(project.id(), "Measures temperature.")
            .unwrap();

        assert_eq!(described.description(), "Measures temperature.");
    }

    #[test]
    fn a_project_that_does_not_exist_is_reported_as_not_found() {
        let missing = ProjectId::generate();

        let error = service().get(missing).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "project", id } if id == missing.to_string()
        ));
    }
}
