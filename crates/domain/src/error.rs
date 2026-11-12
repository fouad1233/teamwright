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

    /// A question for the manager had too few or too many options.
    #[error("a question has two to four options, but this one has {count}")]
    OptionCount {
        /// The number of options that were given.
        count: usize,
    },

    /// An option was named that a question does not have.
    #[error("the question has no such option")]
    NoSuchOption,

    /// A turn was to ask the manager a second time, or to ask and to
    /// propose. A turn carries at most one question, interview or proposal.
    #[error("the turn has already asked the manager a question")]
    AlreadyAsked,

    /// An interview had no questions or too many.
    #[error("an interview has one to five questions, but this one has {count}")]
    QuestionCount {
        /// The number of questions that were given.
        count: usize,
    },

    /// A question of an interview had one option or too many.
    #[error("a question of an interview has two to four options or none, but this one has {count}")]
    InterviewOptionCount {
        /// The number of options that were given.
        count: usize,
    },

    /// An interview was to be answered without one answer for each
    /// question.
    #[error("every question needs an answer")]
    AnswerCount,

    /// An answer to a question of an interview named an option the
    /// question does not have.
    #[error("question {number} has no such option")]
    NoSuchOptionOfQuestion {
        /// The question, counted from 1.
        number: usize,
    },

    /// An answer to a question of an interview chose no option and said
    /// nothing.
    #[error("the answer to question {number} names one of the options or says something in words")]
    EmptyAnswerToQuestion {
        /// The question, counted from 1.
        number: usize,
    },

    /// Two members of a proposal had the same name, whatever the capitals.
    #[error("two members of the proposal are called \"{name}\"")]
    DuplicateProposedMember {
        /// The second of the two names, as it was given.
        name: String,
    },

    /// A task of a proposal waited for itself, or for a place that is not
    /// a task of the proposal.
    #[error("\"{title}\" waits for a task that is not another task of the proposal")]
    WaitsForNoOtherTask {
        /// The title of the task.
        title: String,
    },

    /// Tasks of a proposal waited for each other in a circle.
    #[error("the tasks of the proposal wait for each other in a circle")]
    TasksWaitInCircle,

    /// A proposal had neither members nor tasks.
    #[error("the proposal has no members and no tasks")]
    EmptyProposal,

    /// A proposal was to be approved in a turn that proposed nothing.
    #[error("the turn proposed nothing")]
    NothingProposed,

    /// A proposal was to be approved a second time.
    #[error("the proposal has already been approved")]
    AlreadyApproved,

    /// A question was to be answered while its turn was still running.
    #[error("the turn that asks the question has not ended yet")]
    TurnStillRunning,

    /// A question or an interview was to be answered in a turn that asked
    /// none.
    #[error("the turn asked no question")]
    NothingAsked,

    /// A question was to be answered a second time.
    #[error("the question has already been answered")]
    AlreadyAnswered,

    /// An answer to a question chose no option and said nothing.
    #[error("an answer names one of the options or says something in words")]
    EmptyAnswer,

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
