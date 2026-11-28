//! The errors of the use cases.

use teamwright_domain::{ComputerId, DomainError};

use crate::agent_runtime::AgentRuntimeError;
use crate::kits::ChosenFolderProblem;
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

    /// A task was to be made a part of itself.
    #[error("{task} cannot be a part of itself")]
    PartOfItself {
        /// The task, as the manager knows it (`IOT-4`), or its title in
        /// quotes if it is a new task of a proposal.
        task: String,
    },

    /// A task was to be made a part of one of its own parts, directly or
    /// further down.
    #[error("{task} cannot be a part of {whole}, which is a part of {task} already")]
    PartOfItsOwnPart {
        /// The task, as the manager knows it (`IOT-4`), or its title in
        /// quotes if it is a new task of a proposal.
        task: String,
        /// The task it was to be a part of, named the same way.
        whole: String,
    },

    /// A task was to be made a part of a task it waits for, directly or
    /// through other tasks. A task with parts starts only once its parts
    /// are finished, so neither could ever start.
    #[error(
        "{task} cannot be a part of {whole}, because {task} or one of its parts waits for \
         {whole}, and a task with parts starts only once its parts are finished"
    )]
    PartWaitsForWhole {
        /// The task, as the manager knows it (`IOT-4`), or its title in
        /// quotes if it is a new task of a proposal.
        task: String,
        /// The task it was to be a part of, named the same way.
        whole: String,
    },

    /// A proposal names a task that is no longer in the project: it was
    /// deleted, or the proposal names a task of another project.
    #[error(
        "a task this proposal moves or adds a part to is no longer in the project; take that out \
         of the proposal, or ask the Advisor to propose again"
    )]
    ProposedTaskGone,

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

    /// A member of a proposal whose name the team has already was to be
    /// taken out. Approving does not add it again, so there is nothing to
    /// take out, and its tasks stay with the member the team has.
    #[error("{name} is on the team already: approving does not add them again")]
    ProposedMemberOnTeam {
        /// The name of the member.
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

    /// A notification was to be sent to try them while the manager has not
    /// allowed them on this computer.
    #[error("notifications are not allowed on this computer")]
    NotificationsOff,

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

    /// A digest of an earlier conversation was to be kept a second time
    /// with the same project.
    #[error("a digest of this conversation is kept with the project already: \"{title}\"")]
    DigestAlreadyKept {
        /// The title of the digest that is kept.
        title: String,
    },

    /// A folder was to be used without a name.
    #[error("the folder must be named")]
    FolderNotNamed,

    /// A folder was named that does not exist.
    #[error("the folder {folder} does not exist")]
    FolderNotFound {
        /// The folder, as it was given.
        folder: String,
    },

    /// A folder was named as one of Google Drive that is not among those
    /// found on this computer.
    #[error("the folder {folder} is not one that Google Drive keeps on this computer")]
    NotAGoogleDriveFolder {
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

    /// A folder to write in was to be, lie inside or contain another folder
    /// of the project. A folder of a project has one kind: the work folder,
    /// a folder to write in, or a folder only to read (decision 0044).
    #[error(
        "{folder} {relation} {other_kind} {other}; a folder of a project is the work folder, a \
         folder to write in or a folder only to read, never two of them"
    )]
    WrittenFolderOverlaps {
        /// The folder that was to be added or chosen, as it was given.
        folder: String,
        /// How it stands to the other: "is", "lies inside" or "contains".
        relation: &'static str,
        /// What the other folder is: "the work folder", "the folder to
        /// write in", "the reference folder".
        other_kind: &'static str,
        /// The other folder, as it is kept.
        other: String,
    },

    /// A folder to write in was to be, lie inside or contain a place no
    /// specialist may change (decision 0044).
    #[error("{folder} {relation} {place_kind} ({place}), {why}")]
    FolderProtected {
        /// The folder, as it was given.
        folder: String,
        /// How it stands to the place: "is", "lies inside" or "contains".
        relation: &'static str,
        /// What the place is, in words.
        place_kind: &'static str,
        /// The place.
        place: String,
        /// Why that is refused, in words.
        why: &'static str,
    },

    /// A turn with full access was to start in a folder that lies inside a
    /// place no specialist may change, so that it could not write there.
    #[error(
        "full access cannot be used here: the folder {folder} lies inside {place_kind} \
         ({place}), which no specialist may change; choose another work folder or switch full \
         access off"
    )]
    FullAccessInsideProtected {
        /// The folder of the turn.
        folder: String,
        /// What the place is, in words.
        place_kind: &'static str,
        /// The place.
        place: String,
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

    /// A tag was to get a name that another tag of its project has,
    /// whatever the capitals.
    #[error("the project already has a tag called \"{name}\"")]
    TagNameTaken {
        /// The name that was requested, tidied.
        name: String,
    },

    /// A tag was to be added to a project that has as many as it may.
    #[error("a project has at most {max} tags")]
    TooManyTags {
        /// The most tags a project may have.
        max: usize,
    },

    /// A tag of one project was to be put on a task of another, or to
    /// filter a view of another.
    #[error("a tag can only be used in its own project")]
    TagOfAnotherProject,

    /// A custom field was to get a name that another field of its project
    /// has, whatever the capitals.
    #[error("the project already has a field called \"{name}\"")]
    FieldNameTaken {
        /// The name that was requested, tidied.
        name: String,
    },

    /// A custom field was to be added to a project that has as many as it
    /// may.
    #[error("a project has at most {max} fields")]
    TooManyFields {
        /// The most fields a project may have.
        max: usize,
    },

    /// A custom field of one project was to hold a value of a task of
    /// another, or to filter a view of another.
    #[error("a field can only be used in its own project")]
    FieldOfAnotherProject,

    /// A view was to be filtered by a field that is neither a choice nor
    /// yes or no.
    #[error("a view can be filtered only by a field of choices or of yes or no")]
    FieldNotFilterable,

    /// A file to copy was named by a path that has no file name, such as
    /// the root of a drive.
    #[error("a file to copy has no name")]
    FileWithoutName,

    /// A folder was to be read as one of a project that is neither its work
    /// folder nor one of its reference folders.
    #[error("{folder} is not a folder of this project")]
    NotAFolderOfProject {
        /// The folder, as it was given.
        folder: String,
    },

    /// Nothing that can be read lies at a path inside a folder of a
    /// project: it is gone, or it is a file where a folder was expected.
    #[error("nothing that can be opened lies at {path}")]
    NothingAtPath {
        /// The path inside the folder, as it was given.
        path: String,
    },

    /// The agent program could not be used.
    #[error(transparent)]
    AgentProgram(#[from] AgentRuntimeError),

    /// GitHub's command line program could not be used, or GitHub refused
    /// what it was asked. The reason is said in plain words.
    #[error("{reason}")]
    GitHubUnavailable {
        /// Why, for the manager.
        reason: String,
    },

    /// Something of GitHub was asked while the project has no repository.
    #[error(
        "this project has no GitHub repository: name one, or choose a work folder that is a \
         clone of one"
    )]
    NoRepository,

    /// A test message was to be sent while the project has no channel.
    #[error("no Slack channel is set for this project")]
    NoChannel,

    /// The program was to restart into a new version that is not ready.
    #[error("no update is downloaded and ready yet")]
    UpdateNotReady,

    /// The program was to restart for an update while specialists work,
    /// and the manager had not said yes to stopping them.
    #[error(
        "{} at work; restarting now would stop {}",
        specialists_in_words(*.count),
        if *.count == 1 { "it" } else { "them" }
    )]
    SpecialistsAtWork {
        /// How many turns run on this computer.
        count: usize,
    },

    /// A folder the manager chose cannot be used: it does not exist, is not
    /// a folder, or may not be read. The choice is wrong, or the folder is
    /// closed to Teamwright; its own storage did not fail.
    #[error("{}", .0.message())]
    ChosenFolder(ChosenFolderProblem),

    /// Stored data could not be read or written.
    #[error("the data could not be read or written: {0}")]
    Storage(#[from] StorageError),
}

/// Writes how many specialists work: "1 specialist is", "3 specialists are".
fn specialists_in_words(count: usize) -> String {
    if count == 1 {
        "1 specialist is".to_owned()
    } else {
        format!("{count} specialists are")
    }
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
            | Self::WrittenFolderOverlaps { .. }
            | Self::FolderProtected { .. }
            | Self::FullAccessInsideProtected { .. }
            | Self::FileWithoutName
            | Self::NotAFolderOfProject { .. }
            | Self::OwnerNotThere { .. }
            | Self::ProposedMemberOnTeam { .. }
            | Self::UnfinishedStayInSprint
            | Self::SubtaskInAnotherArea { .. }
            | Self::TooManyTags { .. }
            | Self::TooManyFields { .. }
            | Self::FieldNotFilterable
            | Self::ProjectWithoutAreas => "rule_broken",
            Self::FolderNotFound { .. } | Self::NotAGoogleDriveFolder { .. } => "folder_not_found",
            Self::NotFound { .. } | Self::NothingAtPath { .. } => "not_found",
            Self::ProjectKeyTaken { .. } => "project_key_taken",
            Self::DigestAlreadyKept { .. } => "digest_already_kept",
            Self::DifferentProjects
            | Self::SprintOfAnotherProject
            | Self::UnfinishedToAnotherProject
            | Self::AreaOfAnotherProject
            | Self::TasksToAreaOfAnotherProject
            | Self::TagOfAnotherProject
            | Self::FieldOfAnotherProject => "different_projects",
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
            Self::TagNameTaken { .. } => "tag_name_taken",
            Self::FieldNameTaken { .. } => "field_name_taken",
            Self::SubtaskMovedAlone { .. } => "subtask_moved_alone",
            Self::DependencyCycle { .. } => "dependency_cycle",
            Self::PartOfItself { .. }
            | Self::PartOfItsOwnPart { .. }
            | Self::PartWaitsForWhole { .. } => "part_in_circle",
            Self::ProposedTaskGone => "rule_broken",
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
            Self::NotificationsOff => "notifications_off",
            Self::SyncFolderUnreachable { .. } => "sync_folder_unreachable",
            Self::SyncFolderInDataFolder { .. } => "sync_folder_in_data",
            Self::IdentityInUse => "identity_in_use",
            Self::AgentProgram(error) => error.code(),
            Self::GitHubUnavailable { .. } => "github_unavailable",
            Self::NoRepository => "no_repository",
            Self::NoChannel => "no_channel",
            Self::UpdateNotReady => "update_not_ready",
            Self::SpecialistsAtWork { .. } => "specialists_at_work",
            Self::ChosenFolder(problem) => problem.code(),
            Self::Storage(_) => "storage_failed",
        }
    }
}
