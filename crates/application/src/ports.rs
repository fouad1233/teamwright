//! The interfaces the use cases need from the outside world.
//!
//! The application declares what it needs; other crates provide it. A
//! database crate implements the repositories, and the program's start-up
//! code hands the implementations to the services.

use std::error::Error;
use std::path::{Path, PathBuf};

use teamwright_domain::{
    Area, AreaId, CalendarDate, ComputerId, Conversation, ConversationId, Handoff, Member,
    MemberId, MemoryEntry, MemoryEntryId, Project, ProjectId, Sprint, SprintId, Tag, TagId, Task,
    TaskId, TeamMessage, Timestamp, Turn, View, ViewId,
};

use crate::places::ChosenPlaces;
use crate::project_events::ProjectEvent;

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

    /// Lists every version a task has been stored in, the oldest first. The
    /// last one is the task as [`find`](Self::find) returns it. A task that
    /// was never stored has none.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn versions_of(&self, id: TaskId) -> Result<Vec<Task>, StorageError>;

    /// The computer that made each version [`versions_of`](Self::versions_of)
    /// lists, in the same order. Storage that does not know them answers
    /// with none.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn computers_of_versions(&self, id: TaskId) -> Result<Vec<ComputerId>, StorageError> {
        let _ = id;
        Ok(Vec::new())
    }

    /// When a task was deleted, if it was and storage knows. The course of
    /// a sprint uses it to tell the day a deleted task left the sprint.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn deleted_at(&self, id: TaskId) -> Result<Option<Timestamp>, StorageError> {
        let _ = id;
        Ok(None)
    }

    /// The identifiers of every task a project has had, those deleted since
    /// included, in no particular order. Storage that forgets deleted tasks
    /// answers with the tasks the project has now.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn ids_ever_in_project(&self, project_id: ProjectId) -> Result<Vec<TaskId>, StorageError> {
        Ok(self
            .list_for_project(project_id)?
            .iter()
            .map(Task::id)
            .collect())
    }
}

/// Stores the sprints of the projects.
///
/// Planning, completing and removing a sprint moves tasks with it, so a
/// sprint is stored together with the tasks that changed with it, in one
/// write.
pub trait SprintRepository: Send + Sync {
    /// Stores a sprint, replacing a stored sprint with the same identifier,
    /// and the tasks that changed with it. Either all of it is stored or
    /// nothing is.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint or a task could not be written.
    fn save(&self, sprint: &Sprint, tasks: &[Task]) -> Result<(), StorageError>;

    /// Finds a sprint by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: SprintId) -> Result<Option<Sprint>, StorageError>;

    /// Lists the sprints of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Sprint>, StorageError>;

    /// Removes a sprint, and stores the tasks that left it. The identifier
    /// of a sprint that is not stored is ignored. Either all of it is done
    /// or nothing is.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint could not be removed or a task could
    /// not be written.
    fn delete(&self, id: SprintId, tasks: &[Task]) -> Result<(), StorageError>;
}

/// Stores the views of the projects.
pub trait ViewRepository: Send + Sync {
    /// Stores views, replacing stored views with the same identifiers.
    /// Either all of them are stored or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the views could not be written.
    fn save_all(&self, views: &[View]) -> Result<(), StorageError>;

    /// Finds a view by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: ViewId) -> Result<Option<View>, StorageError>;

    /// Lists the views of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<View>, StorageError>;

    /// Removes a view. The identifier of a view that is not stored is
    /// ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if the view could not be removed.
    fn delete(&self, id: ViewId) -> Result<(), StorageError>;
}

/// Stores the areas of the projects.
pub trait AreaRepository: Send + Sync {
    /// Stores areas, replacing stored areas with the same identifiers.
    /// Either all of them are stored or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the areas could not be written.
    fn save_all(&self, areas: &[Area]) -> Result<(), StorageError>;

    /// Finds an area by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: AreaId) -> Result<Option<Area>, StorageError>;

    /// Lists the areas of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Area>, StorageError>;

    /// Removes an area. The identifier of an area that is not stored is
    /// ignored. The tasks that were in it are expected to be in another
    /// area first.
    ///
    /// # Errors
    ///
    /// Returns an error if the area could not be removed.
    fn delete(&self, id: AreaId) -> Result<(), StorageError>;
}

/// Stores the tags of the projects.
pub trait TagRepository: Send + Sync {
    /// Stores tags, replacing stored tags with the same identifiers. Either
    /// all of them are stored or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the tags could not be written.
    fn save_all(&self, tags: &[Tag]) -> Result<(), StorageError>;

    /// Finds a tag by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: TagId) -> Result<Option<Tag>, StorageError>;

    /// Lists the tags of a project in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Tag>, StorageError>;

    /// Removes a tag. The identifier of a tag that is not stored is
    /// ignored. The tasks that carried it, and the views that showed only
    /// the tasks with it, are expected to have been changed first.
    ///
    /// # Errors
    ///
    /// Returns an error if the tag could not be removed.
    fn delete(&self, id: TagId) -> Result<(), StorageError>;
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

/// Stores the conversations with specialists together with their turns.
pub trait ConversationRepository: Send + Sync {
    /// Stores a conversation, replacing a stored conversation with the same
    /// identifier. Its turns are stored on their own.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation could not be written.
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), StorageError>;

    /// Stores a turn with its steps, replacing a stored turn with the same
    /// identifier. Its conversation is stored already.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn could not be written.
    fn save_turn(&self, turn: &Turn) -> Result<(), StorageError>;

    /// Finds a conversation by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError>;

    /// Lists the turns of a conversation, the oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn turns_of(&self, id: ConversationId) -> Result<Vec<Turn>, StorageError>;

    /// Lists the conversations of a project, the oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Conversation>, StorageError>;

    /// Lists the turns of every project that are still running.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn running_turns(&self) -> Result<Vec<Turn>, StorageError>;

    /// Removes conversations together with their turns. Identifiers of
    /// conversations that are not stored are ignored. Either all of the
    /// stored ones are removed or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversations could not be removed.
    fn delete(&self, ids: &[ConversationId]) -> Result<(), StorageError>;
}

/// Stores what specialists hand each other: the handoffs of tasks and the
/// messages between teammates.
pub trait TeamworkRepository: Send + Sync {
    /// Stores a handoff. A handoff never changes once it is stored.
    ///
    /// # Errors
    ///
    /// Returns an error if the handoff could not be written.
    fn save_handoff(&self, handoff: &Handoff) -> Result<(), StorageError>;

    /// Lists the handoffs of the tasks of a project, the oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn handoffs_of_project(&self, project_id: ProjectId) -> Result<Vec<Handoff>, StorageError>;

    /// Stores messages, replacing stored messages with the same
    /// identifiers. Either all of them are stored or none is.
    ///
    /// # Errors
    ///
    /// Returns an error if the messages could not be written.
    fn save_messages(&self, messages: &[TeamMessage]) -> Result<(), StorageError>;

    /// Lists the messages between the members of a project, the oldest
    /// first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn messages_of_project(&self, project_id: ProjectId) -> Result<Vec<TeamMessage>, StorageError>;
}

/// Stores the memory of the projects: decisions, facts and contracts.
pub trait MemoryRepository: Send + Sync {
    /// Stores entries, replacing stored entries with the same identifiers.
    /// Either all of them are stored or none is: a newer entry and the
    /// older one it replaces are stored together.
    ///
    /// # Errors
    ///
    /// Returns an error if the entries could not be written.
    fn save_entries(&self, entries: &[MemoryEntry]) -> Result<(), StorageError>;

    /// Lists the entries of a project, current and replaced, the oldest
    /// first.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn entries_of_project(&self, project_id: ProjectId) -> Result<Vec<MemoryEntry>, StorageError>;

    /// Finds an entry by its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find_entry(&self, id: MemoryEntryId) -> Result<Option<MemoryEntry>, StorageError>;

    /// Removes an entry. An entry that is not there is no error. An entry
    /// it replaced keeps saying so: what was replaced stays replaced.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry could not be removed.
    fn delete_entry(&self, id: MemoryEntryId) -> Result<(), StorageError>;
}

/// What approving a proposal of the Advisor creates and changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedPlan {
    /// The project, with the task numbers the new tasks took.
    pub project: Project,
    /// The members that join the team.
    pub members: Vec<Member>,
    /// The tasks that are created, each waiting for the tasks it waits for.
    pub tasks: Vec<Task>,
    /// The turn that proposed, with its proposal approved.
    pub turn: Turn,
}

/// Stores what approving a proposal creates, in one write.
pub trait PlanningRepository: Send + Sync {
    /// Stores the project, the new members, the new tasks and the turn of
    /// an approved plan. Either all of it is stored or nothing is.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan could not be written.
    fn save_approved_plan(&self, plan: &ApprovedPlan) -> Result<(), StorageError>;
}

/// Knows the folder Teamwright gives each project. The specialists work
/// there unless the manager chooses another work folder.
pub trait ProjectFolders: Send + Sync {
    /// The folder Teamwright gives a project. It is created if it is
    /// missing.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder does not exist and cannot be created.
    fn folder_of(&self, project: &Project) -> Result<PathBuf, StorageError>;
}

/// Stores the folders the manager has chosen for each project.
///
/// These are facts about one computer: a folder on this computer means
/// nothing on another one.
pub trait PlacesRepository: Send + Sync {
    /// The folders chosen for a project. A project for which nothing was
    /// chosen has no work folder and no reference folders.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn find(&self, project_id: ProjectId) -> Result<ChosenPlaces, StorageError>;

    /// Stores the folders chosen for a project, replacing what was stored
    /// for it before.
    ///
    /// # Errors
    ///
    /// Returns an error if the folders could not be written.
    fn save(&self, project_id: ProjectId, places: &ChosenPlaces) -> Result<(), StorageError>;

    /// Forgets the folders chosen for a project. The folders themselves are
    /// not touched. A project for which nothing is stored is ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if the stored folders could not be removed.
    fn delete(&self, project_id: ProjectId) -> Result<(), StorageError>;
}

/// The files and folders of the computer, as far as the folders of a project
/// need them.
pub trait FileSystem: Send + Sync {
    /// Whether a folder exists at the path.
    fn is_folder(&self, path: &Path) -> bool;

    /// The full form of an existing folder, in which two paths of the same
    /// folder are equal, so that two folders can be compared.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder does not exist or cannot be reached.
    fn resolve(&self, path: &Path) -> Result<PathBuf, StorageError>;

    /// The names of the files and folders in a folder, in no particular
    /// order. A folder that does not exist has none.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder exists and cannot be read.
    fn file_names_in(&self, folder: &Path) -> Result<Vec<String>, StorageError>;

    /// Copies a file. The folders above `to` are created if they are
    /// missing.
    ///
    /// # Errors
    ///
    /// Returns an error that names the file if it cannot be read, or if the
    /// copy cannot be written.
    fn copy_file(&self, from: &Path, to: &Path) -> Result<(), StorageError>;
}

/// One entry of a folder as the disk describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskEntry {
    /// Its name.
    pub name: String,
    /// Whether it is a link (on Windows also a junction). Whether it may be
    /// followed is for the use case to decide.
    pub is_link: bool,
    /// Whether it is a folder; for a link, whether it leads to one.
    pub is_folder: bool,
    /// Its size in bytes; for a link, the size of what it leads to, and 0
    /// if it leads nowhere.
    pub size: u64,
    /// When it was last changed, if the system says.
    pub changed: Option<Timestamp>,
}

/// Reads what lies in the folders of a project, for the view of its files.
/// It only reads; it never changes anything on the disk.
pub trait FolderReader: Send + Sync {
    /// The full form of a path that exists, with every link on the way
    /// followed, in which two paths of one place are equal.
    ///
    /// # Errors
    ///
    /// Returns an error if nothing is at the path or it cannot be reached.
    fn full_form(&self, path: &Path) -> Result<PathBuf, StorageError>;

    /// Whether a folder is at the path, links followed.
    fn is_folder(&self, path: &Path) -> bool;

    /// The entries directly inside a folder, in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read.
    fn entries_in(&self, folder: &Path) -> Result<Vec<DiskEntry>, StorageError>;
}

/// Is told when a conversation changes, for example to show the change at
/// once.
///
/// It is called while the work of the specialists is held still, so it must
/// return quickly and must not call back into the work of the specialists.
pub trait WorkListener: Send + Sync {
    /// A conversation has changed. The turns are all of its turns, the
    /// oldest first.
    fn conversation_changed(&self, conversation: &Conversation, turns: &[Turn]);

    /// Something happened in a project: a turn ended in a way worth telling
    /// outside the window. It is called once for each event, after the
    /// change it comes from was stored and told with
    /// [`Self::conversation_changed`]. A listener that does not care leaves
    /// it as it is.
    fn happened(&self, _event: &ProjectEvent) {}
}

/// Tells the time.
///
/// Use cases ask a clock instead of the operating system so that tests can
/// decide what time it is.
pub trait Clock: Send + Sync {
    /// The current moment.
    fn now(&self) -> Timestamp;

    /// The day on the calendar a moment falls on where the program runs.
    /// A sprint counts its days this way.
    ///
    /// Without a time zone of its own a clock counts days in Coordinated
    /// Universal Time.
    fn day_of(&self, moment: Timestamp) -> CalendarDate {
        moment.date_naive()
    }
}
