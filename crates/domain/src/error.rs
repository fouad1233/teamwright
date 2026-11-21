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

    /// An earlier conversation held no request of the user, so there is
    /// nothing to give an account of.
    #[error("the conversation holds no request of yours, so there is nothing to digest")]
    NothingToDigest,

    /// A change named a task that a proposal does not have.
    #[error("the proposal has no task {number}")]
    NoSuchProposedTask {
        /// The place that was named, counted from 1.
        number: usize,
    },

    /// A change named a member that a proposal does not have.
    #[error("the proposal has no member {number}")]
    NoSuchProposedMember {
        /// The place that was named, counted from 1.
        number: usize,
    },

    /// A task of a proposal was to wait for a task that waits for it,
    /// directly or through other tasks.
    #[error("\"{title}\" cannot wait for \"{other}\", which waits for it already")]
    WaitWouldCloseCircle {
        /// The title of the task that was to wait.
        title: String,
        /// The title of the task it was to wait for.
        other: String,
    },

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

    /// A person was to be given instructions, which only an AI specialist
    /// follows.
    #[error("only an AI specialist has instructions")]
    InstructionsOfPerson,

    /// A start date was later than the due date.
    #[error("the start date {start} is after the due date {due}")]
    StartAfterDue {
        /// The start date that was given.
        start: CalendarDate,
        /// The due date that was given.
        due: CalendarDate,
    },

    /// The start date of a sprint was later than its end date.
    #[error("the start date {start} is after the end date {end}")]
    StartAfterEnd {
        /// The start date that was given.
        start: CalendarDate,
        /// The end date that was given.
        end: CalendarDate,
    },

    /// Story points were not a whole number in their range.
    #[error("story points are a whole number from 0 to {max}")]
    PointsOutOfRange {
        /// The most points a task can have.
        max: u16,
    },

    /// A work-in-progress limit was not a whole number in its range.
    #[error("a limit is a whole number of tasks from {min} to {max}")]
    LimitOutOfRange {
        /// The smallest limit.
        min: u16,
        /// The largest limit.
        max: u16,
    },

    /// A status of the category done was to be given a work-in-progress
    /// limit. Finished work only piles up there.
    #[error("\"{name}\" is a status in which tasks are done, and such a status has no limit")]
    LimitOnDoneStatus {
        /// The name of the status.
        name: String,
    },

    /// A sprint that has started was to lose one of its dates.
    #[error("a sprint that has started has a start and an end date")]
    StartedSprintWithoutDates,

    /// A completed sprint was to be changed, removed, started, completed or
    /// given tasks. It is the record of what was done.
    #[error("\"{name}\" is completed and {refused}")]
    SprintCompleted {
        /// The name of the sprint.
        name: String,
        /// What cannot be done to it, as the end of a sentence, such as
        /// "can no longer be changed".
        refused: &'static str,
    },

    /// An active sprint was to be removed.
    #[error("\"{name}\" has started and cannot be deleted; complete it instead")]
    ActiveSprintDeleted {
        /// The name of the sprint.
        name: String,
    },

    /// An active sprint was to be started again.
    #[error("\"{name}\" has already started")]
    SprintAlreadyStarted {
        /// The name of the sprint.
        name: String,
    },

    /// A sprint that has not started was to be completed.
    #[error("\"{name}\" has not started, so it cannot be completed")]
    SprintNotStarted {
        /// The name of the sprint.
        name: String,
    },

    /// A view of a kind that cannot be filtered, such as a backlog or a
    /// document, was to be given a filter.
    #[error("\"{name}\" is a {kind}, which cannot be filtered")]
    ViewNotFiltered {
        /// The name of the view.
        name: String,
        /// Its kind, as text: "backlog", "dashboard", "doc" and so on.
        kind: &'static str,
    },

    /// A view of a kind that has no groups, such as a calendar, was to group
    /// its tasks by something other than status.
    #[error("\"{name}\" is a {kind}, which cannot group its tasks")]
    ViewNotGrouped {
        /// The name of the view.
        name: String,
        /// Its kind, as text.
        kind: &'static str,
    },

    /// The content of a view was longer than a view may keep.
    #[error(
        "the content of a view can have at most {max_bytes} bytes, but this one has {actual_bytes}"
    )]
    ViewContentTooLong {
        /// The largest number of bytes allowed.
        max_bytes: usize,
        /// The number of bytes that were given.
        actual_bytes: usize,
    },

    /// A web address was not one the program sends to. The address itself
    /// is not repeated: it may be a secret.
    #[error("{field} starts with https:// and names a host, with no spaces")]
    InvalidWebAddress {
        /// What the address is for, for example "the address of a channel".
        field: &'static str,
    },

    /// The name of a GitHub repository was not of the form `owner/name`.
    #[error("a repository is named as owner/name, such as octo-org/sensor-node")]
    InvalidRepositoryName,

    /// More web addresses were given than a project may have.
    #[error("a project sends to at most {max} web addresses")]
    TooManyWebAddresses {
        /// The most addresses a project may have.
        max: usize,
    },

    /// An item was to be added to a checklist that is full.
    #[error("a checklist has at most {max} items")]
    ChecklistFull {
        /// The most items a checklist may have.
        max: usize,
    },

    /// An item was named that the checklist of the task does not have.
    #[error("the checklist of the task has no such item")]
    NoSuchChecklistItem,

    /// A path inside a folder of a project would lead outside it, by how
    /// it is written or through a link.
    #[error("the path {path} does not stay inside the folder")]
    PathLeavesFolder {
        /// The path, as it was given.
        path: String,
    },

    /// A handoff named more files than one may.
    #[error("a handoff names at most {max} files, but this one names {count}")]
    TooManyFiles {
        /// The most files a handoff may name.
        max: usize,
        /// How many were given.
        count: usize,
    },

    /// A specialist sent a message to itself.
    #[error("a message goes to another member of the team")]
    MessageToOneself,

    /// A budget in tokens was outside its range.
    #[error("a budget is a whole number of tokens from {min} to {max}")]
    BudgetOutOfRange {
        /// The smallest budget.
        min: u64,
        /// The largest budget.
        max: u64,
    },

    /// A limit of specialists at work at once was outside its range.
    #[error("the coordinator lets {min} to {max} specialists work at once")]
    ConcurrencyOutOfRange {
        /// The smallest limit.
        min: u8,
        /// The largest limit.
        max: u8,
    },

    /// A question was to be asked about a budget that has no tokens.
    #[error("the budget has no tokens set")]
    NoBudget,

    /// An answer was given about a budget about which no question is open.
    #[error("nothing is asked about this budget")]
    NoBudgetQuestion,

    /// A handoff named the same file twice.
    #[error("the file {path} is named twice")]
    FileNamedTwice {
        /// The path, as it was named the second time.
        path: String,
    },

    /// The title of an entry of memory had more than one line.
    #[error("the title of a memory entry is one line")]
    TitleOnSeveralLines,

    /// An entry of memory came from a specialist without saying which, or
    /// from the manager and named a specialist.
    #[error("a memory entry names a specialist exactly when one wrote it")]
    MemoryOriginIncomplete,

    /// An entry of memory that a newer one replaced was to be replaced
    /// again. Only the current entry can be.
    #[error("this memory entry was replaced by a newer one already")]
    MemoryEntryReplaced,

    /// An entry of memory named itself as its replacement.
    #[error("a memory entry cannot replace itself")]
    MemoryEntryReplacesItself,

    /// A search in memory had more words than one may.
    #[error("a search has at most {max} words, but this one has {count}")]
    TooManySearchWords {
        /// The most words of a search.
        max: usize,
        /// How many were given.
        count: usize,
    },

    /// A folder cannot be given to a specialist as a skill.
    #[error("this cannot be a skill: {reason}")]
    NotASkill {
        /// Why, for the manager, as the end of a sentence.
        reason: String,
    },

    /// A connector, or what a specialist is given of one, breaks a rule.
    /// The reason never repeats a value that may be a secret.
    #[error("{reason}")]
    InvalidConnector {
        /// What is wrong, for the manager, as a sentence without its full
        /// stop.
        reason: String,
    },
}

impl DomainError {
    /// A short, stable name for the kind of rule that was broken.
    ///
    /// Most rules are simply broken rules. Those about the state of a
    /// sprint have codes of their own, so that an interface can tell the
    /// manager what to do instead.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SprintCompleted { .. } => "sprint_completed",
            Self::ActiveSprintDeleted { .. } | Self::SprintAlreadyStarted { .. } => "sprint_active",
            Self::SprintNotStarted { .. } => "sprint_not_active",
            Self::NoSuchChecklistItem => "not_found",
            _ => "rule_broken",
        }
    }
}
