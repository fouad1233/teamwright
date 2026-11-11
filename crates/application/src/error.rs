//! The errors of the use cases.

use teamwright_domain::DomainError;

use crate::ports::StorageError;

/// Why a use case could not be carried out.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    /// A rule of the domain would have been broken.
    #[error(transparent)]
    Rule(#[from] DomainError),

    /// Something the request refers to does not exist.
    #[error("there is no {kind} with the identifier {id}")]
    NotFound {
        /// What was looked for, for example "task".
        kind: &'static str,
        /// The identifier that was given.
        id: String,
    },

    /// Another project already uses the requested key.
    #[error("the key {key} is already used by another project")]
    ProjectKeyTaken {
        /// The key that was requested.
        key: String,
    },

    /// Two tasks of different projects were to be connected.
    #[error("tasks of different projects cannot be connected")]
    DifferentProjects,

    /// A dependency would leave tasks waiting for each other forever.
    #[error("{task} cannot wait for {blocker}, because {blocker} already waits for {task}")]
    DependencyCycle {
        /// The task that was to wait, as the manager knows it (`IOT-4`).
        task: String,
        /// The task it was to wait for.
        blocker: String,
    },

    /// Another member of the team already has the requested name.
    #[error("the team already has a member called \"{name}\"")]
    MemberNameTaken {
        /// The name that was requested.
        name: String,
    },

    /// Stored data could not be read or written.
    #[error("the data could not be read or written: {0}")]
    Storage(#[from] StorageError),
}

impl ApplicationError {
    /// A short, stable name for the kind of error.
    ///
    /// Interfaces use it to decide how to present the error. The message is
    /// for people and may be reworded; the code is not.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Rule(_) => "rule_broken",
            Self::NotFound { .. } => "not_found",
            Self::ProjectKeyTaken { .. } => "project_key_taken",
            Self::DifferentProjects => "different_projects",
            Self::DependencyCycle { .. } => "dependency_cycle",
            Self::MemberNameTaken { .. } => "member_name_taken",
            Self::Storage(_) => "storage_failed",
        }
    }
}
