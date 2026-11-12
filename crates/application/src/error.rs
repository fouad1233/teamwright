//! The errors of the use cases.

use teamwright_domain::DomainError;

use crate::agent_runtime::AgentRuntimeError;
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

    /// A task was to be given to a member of the team of another project.
    #[error("a task can only be given to a member of the team of its own project")]
    AssigneeFromAnotherTeam,

    /// A status was to be removed while tasks are in it and no other status
    /// was named for them.
    #[error(
        "the status \"{name}\" still has {}, which need another status to go to",
        count_of_tasks(*.count)
    )]
    StatusInUse {
        /// The name of the status.
        name: String,
        /// How many tasks are in it, subtasks included.
        count: usize,
    },

    /// Work was to start on a task that is not given to an AI specialist.
    #[error("the task is not given to an AI specialist")]
    TaskNotGivenToSpecialist,

    /// A conversation was to go on whose specialist has left the team.
    #[error("the specialist of this conversation has left the team")]
    SpecialistLeft,

    /// A member was asked something who is not an AI specialist.
    #[error("{name} is not an AI specialist")]
    NotASpecialist {
        /// The name of the member.
        name: String,
    },

    /// Work was to start on a task a second time.
    #[error("work on {task} has already started")]
    WorkAlreadyStarted {
        /// The task, as the manager knows it (`IOT-4`).
        task: String,
    },

    /// A specialist was given something while it still works on something
    /// else, or was to leave the team while it works.
    #[error("{name} is still working on {work}")]
    MemberBusy {
        /// The name of the specialist.
        name: String,
        /// What it works on: a task as the manager knows it, or "a
        /// question".
        work: String,
    },

    /// Something was to be removed or put away that a specialist is
    /// working on.
    #[error("{who} is still working on {work}; stop the work first")]
    WorkInProgress {
        /// The name of the specialist, or "A specialist" if it has left.
        who: String,
        /// What it works on: a task as the manager knows it, or "a
        /// question".
        work: String,
    },

    /// A question of a specialist was to be answered in a conversation in
    /// which none is open.
    #[error("no question is open in this conversation")]
    NoDecisionOpen,

    /// A conversation was to go on with an ordinary message while its
    /// specialist waits for the answer to a question.
    #[error("{name} waits for a decision: answer its question first")]
    DecisionOpen {
        /// The name of the specialist.
        name: String,
    },

    /// A conversation was to be opened in the agent program before the
    /// program had started its session.
    #[error("the specialist has not started its session yet")]
    SessionNotStarted,

    /// The agent program could not be used.
    #[error(transparent)]
    AgentProgram(#[from] AgentRuntimeError),

    /// Stored data could not be read or written.
    #[error("the data could not be read or written: {0}")]
    Storage(#[from] StorageError),
}

/// Writes a number of tasks: "1 task", "3 tasks".
fn count_of_tasks(count: usize) -> String {
    if count == 1 {
        "1 task".to_owned()
    } else {
        format!("{count} tasks")
    }
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
            Self::AssigneeFromAnotherTeam => "assignee_from_another_team",
            Self::StatusInUse { .. } => "status_in_use",
            Self::TaskNotGivenToSpecialist | Self::SpecialistLeft | Self::NotASpecialist { .. } => {
                "no_specialist"
            }
            Self::WorkAlreadyStarted { .. } => "work_already_started",
            Self::MemberBusy { .. } => "member_busy",
            Self::WorkInProgress { .. } => "work_in_progress",
            Self::NoDecisionOpen => "no_decision_open",
            Self::DecisionOpen { .. } => "decision_open",
            Self::SessionNotStarted => "session_not_started",
            Self::AgentProgram(error) => error.code(),
            Self::Storage(_) => "storage_failed",
        }
    }
}
