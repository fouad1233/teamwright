//! The interfaces the use cases need from the outside world.
//!
//! The application declares what it needs; other crates provide it. A
//! database crate implements the repositories, and the program's start-up
//! code hands the implementations to the services.

use std::error::Error;

use teamwright_domain::{Member, MemberId, Project, ProjectId, Task, TaskId, Timestamp};

/// A failure to read or write stored data.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct StorageError {
    message: String,
    #[source]
    source: Option<Box<dyn Error + Send + Sync + 'static>>,
}

impl StorageError {
    /// Describes a failure that has no underlying error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    /// Describes a failure and keeps the error that caused it.
    #[must_use]
    pub fn caused_by(
        message: impl Into<String>,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Stores projects together with their workflows.
pub trait ProjectRepository: Send + Sync {
    /// Stores a project, replacing a stored project with the same
    /// identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the project could not be written.
    fn save(&self, project: &Project) -> Result<(), StorageError>;

    /// Finds a project by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: ProjectId) -> Result<Option<Project>, StorageError>;

    /// Lists all projects, archived ones included, oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list(&self) -> Result<Vec<Project>, StorageError>;

    /// Removes a project with its workflow, for good. The identifier of a
    /// project that is not stored is ignored.
    ///
    /// The tasks and the team of the project are expected to be removed
    /// through their own repositories first. An implementation whose storage
    /// would otherwise keep them, or drop them without a trace, removes the
    /// ones that are left together with the project.
    ///
    /// # Errors
    ///
    /// Returns an error if the project could not be removed.
    fn delete(&self, id: ProjectId) -> Result<(), StorageError>;
}

/// Stores tasks together with their dependencies.
pub trait TaskRepository: Send + Sync {
    /// Stores a task, replacing a stored task with the same identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the task could not be written.
    fn save(&self, task: &Task) -> Result<(), StorageError>;

    /// Stores several tasks. Either all of them are stored or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the tasks could not be written.
    fn save_all(&self, tasks: &[Task]) -> Result<(), StorageError>;

    /// Finds a task by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: TaskId) -> Result<Option<Task>, StorageError>;

    /// Lists the tasks of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Task>, StorageError>;

    /// Removes tasks. Identifiers of tasks that are not stored are ignored.
    /// Either all of the stored ones are removed or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the tasks could not be removed.
    fn delete(&self, ids: &[TaskId]) -> Result<(), StorageError>;
}

/// Stores the members of the teams.
pub trait MemberRepository: Send + Sync {
    /// Stores a member, replacing a stored member with the same identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the member could not be written.
    fn save(&self, member: &Member) -> Result<(), StorageError>;

    /// Finds a member by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: MemberId) -> Result<Option<Member>, StorageError>;

    /// Lists the members of the team of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Member>, StorageError>;

    /// Removes a member. The identifier of a member that is not stored is
    /// ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if the member could not be removed.
    fn delete(&self, id: MemberId) -> Result<(), StorageError>;
}

/// Tells the time.
///
/// Use cases ask a clock instead of the operating system so that tests can
/// decide what time it is.
pub trait Clock: Send + Sync {
    /// The current moment.
    fn now(&self) -> Timestamp;
}
