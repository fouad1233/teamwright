//! The errors of the use cases.

use teamwright_domain::{ComputerId, DomainError};

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

    /// A conversation with the Advisor was to start, or to go on, while
    /// the Advisor works on the project. One conversation with the Advisor
    /// runs in a project at a time.
    #[error("the Advisor is still working on this project")]
    AdvisorBusy,

    /// A conversation with the Advisor was to go on with an ordinary
    /// message while the Advisor waits for the answers to its questions.
    #[error("the Advisor waits for your answers: answer its questions first")]
    InterviewOpen,

    /// The questions of the Advisor were to be answered in a conversation
    /// in which none are open.
    #[error("no questions are open in this conversation")]
    NoInterviewOpen,

    /// A proposal was to be approved in a conversation in which none is
    /// open.
    #[error("no proposal is open in this conversation")]
    NoProposalOpen,

    /// A task of an approved proposal was given to somebody who is
    /// neither on the team nor in the proposal.
    #[error("\"{title}\" is given to \"{name}\", who is neither on the team nor in the proposal")]
    OwnerNotThere {
        /// The title of the task.
        title: String,
        /// The name it was given to.
        name: String,
    },

    /// A conversation was to be opened in the agent program before the
    /// program had started its session.
    #[error("the specialist has not started its session yet")]
    SessionNotStarted,

    /// A conversation was to be acted on (replied to, answered, approved,
    /// stopped or opened) on a computer that is not its home. Its session
    /// lives on its home computer.
    #[error("this conversation runs on another computer, and can only be continued there")]
    ConversationRunsElsewhere {
        /// The computer the conversation runs on.
        computer: ComputerId,
    },

    /// A round of syncing was to run while no sync folder is chosen.
    #[error("no sync folder is chosen")]
    SyncOff,

    /// The sync folder could not be reached: it is gone, or a drive or a
    /// share is not there.
    #[error("the sync folder {folder} cannot be reached")]
    SyncFolderUnreachable {
        /// The folder, as it is kept.
        folder: String,
    },

    /// The sync folder was to lie inside the folder Teamwright keeps its
    /// own data in.
    #[error("the sync folder {folder} lies inside Teamwright's own data folder; choose another")]
    SyncFolderInDataFolder {
        /// The folder, as it was given.
        folder: String,
    },

    /// Another computer writes into the sync folder under the identity of
    /// this one: its data folder was copied. Syncing stops until this
    /// computer gets a new identity, or until Teamwright starts again with
    /// the new identity it got.
    #[error(
        "another computer uses the identity of this computer; give this computer a new identity \
         and start Teamwright again"
    )]
    IdentityInUse,

    /// A folder was to be used without a name.
    #[error("the folder must be named")]
    FolderNotNamed,

    /// A folder was named that does not exist.
    #[error("the folder {folder} does not exist")]
    FolderNotFound {
        /// The folder, as it was given.
        folder: String,
    },

    /// The work folder and a reference folder were to be the same folder.
    #[error("the work folder and a reference folder cannot be the same folder: {work_folder}")]
    WorkFolderIsReference {
        /// The work folder, as it was given.
        work_folder: String,
    },

    /// A reference folder was to lie inside the work folder, where the
    /// specialists read everything already.
    #[error(
        "{reference_folder} lies inside the work folder {work_folder}, where the specialists \
         read everything already"
    )]
    ReferenceInsideWorkFolder {
        /// The reference folder, as it was given.
        reference_folder: String,
        /// The work folder, as it was given.
        work_folder: String,
    },

    /// The work folder was to lie inside a reference folder, where the
    /// specialists may not write.
    #[error(
        "the work folder {work_folder} lies inside the reference folder {reference_folder}, \
         where the specialists may not write"
    )]
    WorkFolderInsideReference {
        /// The work folder, as it was given.
        work_folder: String,
        /// The reference folder, as it was given.
        reference_folder: String,
    },

    /// A sprint was to start while another sprint of its project runs. A
    /// project has one active sprint at a time.
    #[error("\"{name}\" is still running; complete it before you start another sprint")]
    AnotherSprintActive {
        /// The name of the sprint that runs.
        name: String,
    },

    /// A sprint without tasks was to start.
    #[error("\"{name}\" has no tasks; plan some into it before you start it")]
    SprintEmpty {
        /// The name of the sprint.
        name: String,
    },

    /// A subtask was to be planned into a sprint, or out of one, by
    /// itself. It goes where the task it is a part of goes.
    #[error("{task} is a part of {parent} and goes where it goes")]
    SubtaskPlannedAlone {
        /// The subtask, as the manager knows it (`IOT-4`).
        task: String,
        /// The task it is a part of.
        parent: String,
    },

    /// A task was to be planned into a sprint of another project.
    #[error("a task can only be planned into a sprint of its own project")]
    SprintOfAnotherProject,

    /// The unfinished tasks of a sprint that is completed were to go to a
    /// sprint of another project.
    #[error("the unfinished tasks can only go to a sprint of the same project")]
    UnfinishedToAnotherProject,

    /// The unfinished tasks of a sprint that is completed were to stay in
    /// it.
    #[error("the unfinished tasks cannot stay in the sprint that is completed")]
    UnfinishedStayInSprint,

    /// A view was to get a name that another view of its project has,
    /// whatever the capitals.
    #[error("the project already has a view called \"{name}\"")]
    ViewNameTaken {
        /// The name that was requested, tidied.
        name: String,
    },

    /// A second backlog or a second dashboard was to be added to a project.
    #[error("the project already has a {kind}, \"{name}\", and can have only one")]
    ViewKindTaken {
        /// The kind, as text: "backlog" or "dashboard".
        kind: &'static str,
        /// The name of the view of that kind the project has.
        name: String,
    },

    /// A filter of a view named a member who is not on the team of the
    /// project of the view.
    #[error("the filter names a member who is not on the team of the project")]
    MemberNotOnTeam,

    /// The one view a project has left was to be removed.
    #[error("\"{name}\" is the last view of the project and cannot be removed")]
    LastView {
        /// The name of the view.
        name: String,
    },

    /// An area was to get a name that another area of its project has,
    /// whatever the capitals.
    #[error("the project already has an area called \"{name}\"")]
    AreaNameTaken {
        /// The name that was requested, tidied.
        name: String,
    },

    /// The one area a project has left was to be removed.
    #[error("\"{name}\" is the last area of the project and cannot be removed")]
    LastArea {
        /// The name of the area.
        name: String,
    },

    /// An area was to be removed while tasks are in it and no other area
    /// was named for them.
    #[error(
        "the area \"{name}\" still has {}, which need another area to go to",
        count_of_tasks(*.count)
    )]
    AreaInUse {
        /// The name of the area.
        name: String,
        /// How many tasks are in it, subtasks included.
        count: usize,
    },

    /// A task was to be put into an area of another project.
    #[error("a task can only be put into an area of its own project")]
    AreaOfAnotherProject,

    /// The tasks of an area that is removed were to go to an area of
    /// another project.
    #[error("the tasks of an area can only go to another area of the same project")]
    TasksToAreaOfAnotherProject,

    /// A subtask was to be moved into another area by itself. It goes
    /// where the task it is a part of goes.
    #[error("{task} is a part of {parent} and goes where it goes")]
    SubtaskMovedAlone {
        /// The subtask, as the manager knows it (`IOT-4`).
        task: String,
        /// The task it is a part of.
        parent: String,
    },

    /// A new subtask was to go into another area than the task it is a
    /// part of.
    #[error("a subtask is in the area of the task it is a part of, {parent}")]
    SubtaskInAnotherArea {
        /// The task it is a part of, as the manager knows it (`IOT-4`).
        parent: String,
    },

    /// A project was found without areas, which only damaged data can
    /// leave behind: every project has at least one.
    #[error("the project has no areas")]
    ProjectWithoutAreas,

    /// A file to copy was named by a path that has no file name, such as
    /// the root of a drive.
    #[error("a file to copy has no name")]
    FileWithoutName,

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
            // The rules about folders concern several things at once, the
            // stored folders and the computer, so they are checked here and
            // not in the domain. To the interface they are rules like any
            // other.
            Self::Rule(rule) => rule.code(),
            Self::FolderNotNamed
            | Self::WorkFolderIsReference { .. }
            | Self::ReferenceInsideWorkFolder { .. }
            | Self::WorkFolderInsideReference { .. }
            | Self::FileWithoutName
            | Self::OwnerNotThere { .. }
            | Self::UnfinishedStayInSprint
            | Self::SubtaskInAnotherArea { .. }
            | Self::ProjectWithoutAreas => "rule_broken",
            Self::FolderNotFound { .. } => "folder_not_found",
            Self::NotFound { .. } => "not_found",
            Self::ProjectKeyTaken { .. } => "project_key_taken",
            Self::DifferentProjects
            | Self::SprintOfAnotherProject
            | Self::UnfinishedToAnotherProject
            | Self::AreaOfAnotherProject
            | Self::TasksToAreaOfAnotherProject => "different_projects",
            Self::AnotherSprintActive { .. } => "another_sprint_active",
            Self::SprintEmpty { .. } => "sprint_empty",
            Self::SubtaskPlannedAlone { .. } => "subtask_planned_alone",
            Self::ViewNameTaken { .. } => "view_name_taken",
            Self::ViewKindTaken { .. } => "view_kind_taken",
            Self::MemberNotOnTeam => "member_not_on_team",
            Self::LastView { .. } => "last_view",
            Self::AreaNameTaken { .. } => "area_name_taken",
            Self::LastArea { .. } => "last_area",
            Self::AreaInUse { .. } => "area_in_use",
            Self::SubtaskMovedAlone { .. } => "subtask_moved_alone",
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
            Self::AdvisorBusy => "advisor_busy",
            Self::InterviewOpen => "interview_open",
            Self::NoInterviewOpen => "no_interview_open",
            Self::NoProposalOpen => "no_proposal_open",
            Self::SessionNotStarted => "session_not_started",
            Self::ConversationRunsElsewhere { .. } => "runs_elsewhere",
            Self::SyncOff => "sync_off",
            Self::SyncFolderUnreachable { .. } => "sync_folder_unreachable",
            Self::SyncFolderInDataFolder { .. } => "sync_folder_in_data",
            Self::IdentityInUse => "identity_in_use",
            Self::AgentProgram(error) => error.code(),
            Self::Storage(_) => "storage_failed",
        }
    }
}
