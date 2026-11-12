//! The errors of the domain.

use crate::ids::StatusId;
use crate::time::CalendarDate;
use crate::workflow::StatusCategory;

/// A rule of the domain that an operation would have broken.
///
/// The messages are written for a manager to read, because they can end up in
/// front of one.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A text that must say something was empty.
    #[error("the {field} must not be empty")]
    EmptyText {
        /// What the text names, for example "task title".
        field: &'static str,
    },

    /// A text was longer than its limit.
    #[error(
        "the {field} can have at most {max_length} characters, but this one has {actual_length}"
    )]
    TextTooLong {
        /// What the text names, for example "task title".
        field: &'static str,
        /// The largest number of characters allowed.
        max_length: usize,
        /// The number of characters that were given.
        actual_length: usize,
    },

    /// A text could not be read as an identifier.
    #[error("'{value}' is not a valid {kind}")]
    InvalidIdentifier {
        /// The kind of identifier that was expected, for example "task identifier".
        kind: &'static str,
        /// The text that was given.
        value: String,
    },

    /// A text could not be read as one of a fixed set of values.
    #[error("'{value}' is not a known {kind}")]
    UnknownValue {
        /// The kind of value that was expected, for example "priority".
        kind: &'static str,
        /// The text that was given.
        value: String,
    },

    /// A project key did not have the required form.
    #[error(
        "a project key has 2 to 6 characters, starts with a letter and contains only letters \
         and digits, which '{value}' does not"
    )]
    InvalidProjectKey {
        /// The text that was given.
        value: String,
    },

    /// A workflow lacked a kind of status it cannot work without.
    #[error("a workflow needs at least one status in the \"{category}\" category")]
    MissingStatusCategory {
        /// The category that has no status.
        category: StatusCategory,
    },

    /// Two statuses of one workflow had the same name.
    #[error("the workflow already has a status named \"{name}\"")]
    DuplicateStatusName {
        /// The name that was used twice.
        name: String,
    },

    /// A status was used that the workflow does not contain.
    #[error("the workflow has no status with the identifier {status_id}")]
    UnknownStatus {
        /// The identifier that was not found.
        status_id: StatusId,
    },

    /// A task was asked to wait for itself.
    #[error("a task cannot depend on itself")]
    SelfDependency,

    /// A task was made a subtask of itself.
    #[error("a task cannot be its own parent")]
    SelfParent,

    /// Something was to happen in a turn that has already ended: a step, or
    /// a second ending.
    #[error("the turn has already ended")]
    TurnEnded,

    /// A tool use was to be finished that the turn does not contain.
    #[error("the turn has no tool use with the identifier '{id}'")]
    UnknownToolUse {
        /// The identifier of the tool use that was given.
        id: String,
    },

    /// The name of a model was empty, had whitespace inside, or was too
    /// long.
    #[error("the name of a model is one word of at most {max_length} characters")]
    InvalidModelName {
        /// The largest number of characters allowed.
        max_length: usize,
    },

    /// A person was to be given a model or an effort, which only an AI
    /// specialist has.
    #[error("only an AI specialist has a model and an effort")]
    ProfileOfPerson,

    /// A start date was later than the due date.
    #[error("the start date {start} is after the due date {due}")]
    StartAfterDue {
        /// The start date that was given.
        start: CalendarDate,
        /// The due date that was given.
        due: CalendarDate,
    },
}
