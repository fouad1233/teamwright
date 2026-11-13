//! Implementations of the ports that keep everything in memory.
//!
//! They exist for tests and demonstrations. Nothing is written to disk, and
//! everything is gone when the value is dropped. The agent runtime here
//! starts no program: it plays what a test tells it to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::TimeDelta;
use teamwright_domain::{
    Area, AreaId, Conversation, ConversationId, Member, MemberId, Project, ProjectId, Sprint,
    SprintId, Task, TaskId, Timestamp, Turn, TurnId, View, ViewId,
};

use crate::agent_runtime::{
    AgentEvent, AgentRuntime, AgentRuntimeError, Installation, ProgramVersion, RunningTurn,
    SessionId, SignIn, TurnOutcome, TurnRequest,
};
use crate::places::ChosenPlaces;
use crate::ports::{
    ApprovedPlan, AreaRepository, Clock, ConversationRepository, FileSystem, MemberRepository,
    PlacesRepository, PlanningRepository, ProjectFolders, ProjectRepository, SprintRepository,
    StorageError, TaskRepository, ViewRepository, WorkListener,
};

/// Locks a mutex, and keeps working even if another thread panicked while it
/// held the lock. The values guarded here are valid after every single
/// operation, so there is never anything to repair.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps projects in memory.
#[derive(Debug, Default)]
pub struct InMemoryProjectRepository {
    projects: Mutex<BTreeMap<ProjectId, Project>>,
}

impl ProjectRepository for InMemoryProjectRepository {
    fn save(&self, project: &Project) -> Result<(), StorageError> {
        lock(&self.projects).insert(project.id(), project.clone());
        Ok(())
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        Ok(lock(&self.projects).get(&id).cloned())
    }

    fn list(&self) -> Result<Vec<Project>, StorageError> {
        // Identifiers are ordered by creation time, so walking the map in key
        // order lists the oldest project first.
        Ok(lock(&self.projects).values().cloned().collect())
    }

    fn delete(&self, id: ProjectId) -> Result<(), StorageError> {
        lock(&self.projects).remove(&id);
        Ok(())
    }
}

/// Keeps tasks in memory, together with every version they were stored in.
#[derive(Debug, Default)]
pub struct InMemoryTaskRepository {
    tasks: Mutex<BTreeMap<TaskId, Task>>,
    /// Every version of every task, in the order they were stored. Real
    /// storage reads them from its log of operations.
    versions: Mutex<BTreeMap<TaskId, Vec<Task>>>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn save(&self, task: &Task) -> Result<(), StorageError> {
        self.save_all(std::slice::from_ref(task))
    }

    fn save_all(&self, tasks: &[Task]) -> Result<(), StorageError> {
        let mut stored = lock(&self.tasks);
        let mut versions = lock(&self.versions);
        for task in tasks {
            stored.insert(task.id(), task.clone());
            versions.entry(task.id()).or_default().push(task.clone());
        }
        Ok(())
    }

    fn find(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        Ok(lock(&self.tasks).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Task>, StorageError> {
        Ok(lock(&self.tasks)
            .values()
            .filter(|task| task.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, ids: &[TaskId]) -> Result<(), StorageError> {
        let mut stored = lock(&self.tasks);
        for id in ids {
            stored.remove(id);
        }
        Ok(())
    }

    fn versions_of(&self, id: TaskId) -> Result<Vec<Task>, StorageError> {
        Ok(lock(&self.versions).get(&id).cloned().unwrap_or_default())
    }
}

/// Keeps sprints in memory, and stores the tasks that change with them in
/// the in-memory task repository it was given.
#[derive(Debug)]
pub struct InMemorySprintRepository {
    sprints: Mutex<BTreeMap<SprintId, Sprint>>,
    tasks: Arc<InMemoryTaskRepository>,
}

impl InMemorySprintRepository {
    /// Stores the tasks that change with a sprint into `tasks`.
    #[must_use]
    pub const fn new(tasks: Arc<InMemoryTaskRepository>) -> Self {
        Self {
            sprints: Mutex::new(BTreeMap::new()),
            tasks,
        }
    }
}

impl SprintRepository for InMemorySprintRepository {
    fn save(&self, sprint: &Sprint, tasks: &[Task]) -> Result<(), StorageError> {
        lock(&self.sprints).insert(sprint.id(), sprint.clone());
        self.tasks.save_all(tasks)
    }

    fn find(&self, id: SprintId) -> Result<Option<Sprint>, StorageError> {
        Ok(lock(&self.sprints).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Sprint>, StorageError> {
        Ok(lock(&self.sprints)
            .values()
            .filter(|sprint| sprint.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: SprintId, tasks: &[Task]) -> Result<(), StorageError> {
        self.tasks.save_all(tasks)?;
        lock(&self.sprints).remove(&id);
        Ok(())
    }
}

/// Keeps the views of the projects in memory.
#[derive(Debug, Default)]
pub struct InMemoryViewRepository {
    views: Mutex<BTreeMap<ViewId, View>>,
}

impl ViewRepository for InMemoryViewRepository {
    fn save_all(&self, views: &[View]) -> Result<(), StorageError> {
        let mut stored = lock(&self.views);
        for view in views {
            stored.insert(view.id(), view.clone());
        }
        Ok(())
    }

    fn find(&self, id: ViewId) -> Result<Option<View>, StorageError> {
        Ok(lock(&self.views).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<View>, StorageError> {
        Ok(lock(&self.views)
            .values()
            .filter(|view| view.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: ViewId) -> Result<(), StorageError> {
        lock(&self.views).remove(&id);
        Ok(())
    }
}

/// Keeps the areas of the projects in memory.
#[derive(Debug, Default)]
pub struct InMemoryAreaRepository {
    areas: Mutex<BTreeMap<AreaId, Area>>,
}

impl AreaRepository for InMemoryAreaRepository {
    fn save_all(&self, areas: &[Area]) -> Result<(), StorageError> {
        let mut stored = lock(&self.areas);
        for area in areas {
            stored.insert(area.id(), area.clone());
        }
        Ok(())
    }

    fn find(&self, id: AreaId) -> Result<Option<Area>, StorageError> {
        Ok(lock(&self.areas).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Area>, StorageError> {
        Ok(lock(&self.areas)
            .values()
            .filter(|area| area.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: AreaId) -> Result<(), StorageError> {
        lock(&self.areas).remove(&id);
        Ok(())
    }
}

/// Keeps the members of the teams in memory.
#[derive(Debug, Default)]
pub struct InMemoryMemberRepository {
    members: Mutex<BTreeMap<MemberId, Member>>,
}

impl MemberRepository for InMemoryMemberRepository {
    fn save(&self, member: &Member) -> Result<(), StorageError> {
        lock(&self.members).insert(member.id(), member.clone());
        Ok(())
    }

    fn find(&self, id: MemberId) -> Result<Option<Member>, StorageError> {
        Ok(lock(&self.members).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Member>, StorageError> {
        Ok(lock(&self.members)
            .values()
            .filter(|member| member.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: MemberId) -> Result<(), StorageError> {
        lock(&self.members).remove(&id);
        Ok(())
    }
}

/// Keeps conversations and their turns in memory.
#[derive(Debug, Default)]
pub struct InMemoryConversationRepository {
    stored: Mutex<StoredConversations>,
}

/// The conversations and the turns, each by identifier.
#[derive(Debug, Default)]
struct StoredConversations {
    conversations: BTreeMap<ConversationId, Conversation>,
    turns: BTreeMap<TurnId, Turn>,
}

impl ConversationRepository for InMemoryConversationRepository {
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), StorageError> {
        lock(&self.stored)
            .conversations
            .insert(conversation.id(), conversation.clone());
        Ok(())
    }

    fn save_turn(&self, turn: &Turn) -> Result<(), StorageError> {
        lock(&self.stored).turns.insert(turn.id(), turn.clone());
        Ok(())
    }

    fn find(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError> {
        Ok(lock(&self.stored).conversations.get(&id).cloned())
    }

    fn turns_of(&self, id: ConversationId) -> Result<Vec<Turn>, StorageError> {
        let mut turns: Vec<Turn> = lock(&self.stored)
            .turns
            .values()
            .filter(|turn| turn.conversation_id() == id)
            .cloned()
            .collect();
        turns.sort_by_key(|turn| (turn.started_at(), turn.id()));
        Ok(turns)
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Conversation>, StorageError> {
        let mut conversations: Vec<Conversation> = lock(&self.stored)
            .conversations
            .values()
            .filter(|conversation| conversation.project_id() == project_id)
            .cloned()
            .collect();
        conversations.sort_by_key(|conversation| (conversation.started_at(), conversation.id()));
        Ok(conversations)
    }

    fn running_turns(&self) -> Result<Vec<Turn>, StorageError> {
        Ok(lock(&self.stored)
            .turns
            .values()
            .filter(|turn| turn.is_running())
            .cloned()
            .collect())
    }

    fn delete(&self, ids: &[ConversationId]) -> Result<(), StorageError> {
        let mut stored = lock(&self.stored);
        for id in ids {
            stored.conversations.remove(id);
            stored.turns.retain(|_, turn| turn.conversation_id() != *id);
        }
        Ok(())
    }
}

/// Stores an approved plan in the in-memory repositories it was given.
/// Nothing in memory can fail half-way, so the plan is stored whole.
pub struct InMemoryPlanningRepository {
    projects: Arc<InMemoryProjectRepository>,
    members: Arc<InMemoryMemberRepository>,
    tasks: Arc<InMemoryTaskRepository>,
    conversations: Arc<InMemoryConversationRepository>,
}

impl InMemoryPlanningRepository {
    /// Stores into these repositories.
    #[must_use]
    pub const fn new(
        projects: Arc<InMemoryProjectRepository>,
        members: Arc<InMemoryMemberRepository>,
        tasks: Arc<InMemoryTaskRepository>,
        conversations: Arc<InMemoryConversationRepository>,
    ) -> Self {
        Self {
            projects,
            members,
            tasks,
            conversations,
        }
    }
}

impl PlanningRepository for InMemoryPlanningRepository {
    fn save_approved_plan(&self, plan: &ApprovedPlan) -> Result<(), StorageError> {
        self.projects.save(&plan.project)?;
        for member in &plan.members {
            self.members.save(member)?;
        }
        self.tasks.save_all(&plan.tasks)?;
        self.conversations.save_turn(&plan.turn)
    }
}

/// Puts the folder of every project into one folder, named by the key of
/// the project. Nothing is created on disk.
#[derive(Debug, Clone)]
pub struct FixedProjectFolders {
    root: PathBuf,
}

impl FixedProjectFolders {
    /// Puts the folders of the projects into `root`.
    #[must_use]
    pub fn inside(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ProjectFolders for FixedProjectFolders {
    fn folder_of(&self, project: &Project) -> Result<PathBuf, StorageError> {
        Ok(self.root.join(project.key().as_str()))
    }
}

/// Keeps the folders chosen for each project in memory.
#[derive(Debug, Default)]
pub struct InMemoryPlacesRepository {
    places: Mutex<BTreeMap<ProjectId, ChosenPlaces>>,
}

impl PlacesRepository for InMemoryPlacesRepository {
    fn find(&self, project_id: ProjectId) -> Result<ChosenPlaces, StorageError> {
        Ok(lock(&self.places)
            .get(&project_id)
            .cloned()
            .unwrap_or_default())
    }

    fn save(&self, project_id: ProjectId, places: &ChosenPlaces) -> Result<(), StorageError> {
        lock(&self.places).insert(project_id, places.clone());
        Ok(())
    }

    fn delete(&self, project_id: ProjectId) -> Result<(), StorageError> {
        lock(&self.places).remove(&project_id);
        Ok(())
    }
}

/// A file system that exists only in what a test says exists: a set of
/// folders and a set of files. Nothing is read from or written to disk.
///
/// It pretends to be a file system that does not tell capitals apart and
/// accepts both kinds of slash, as the one of Windows does: two paths name
/// the same folder or file when they are equal in that sense.
#[derive(Debug, Default)]
pub struct PretendFileSystem {
    entries: Mutex<PretendEntries>,
}

/// The folders and files that exist, as they were given.
#[derive(Debug, Default)]
struct PretendEntries {
    folders: Vec<PathBuf>,
    files: Vec<PathBuf>,
}

impl PretendFileSystem {
    /// A file system in which the given folders exist, and no files.
    #[must_use]
    pub fn with_folders<I, P>(folders: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let file_system = Self::default();
        for folder in folders {
            file_system.add_folder(folder);
        }
        file_system
    }

    /// Makes a folder exist.
    pub fn add_folder(&self, folder: impl Into<PathBuf>) {
        lock(&self.entries).folders.push(folder.into());
    }

    /// Makes a file exist.
    pub fn add_file(&self, file: impl Into<PathBuf>) {
        lock(&self.entries).files.push(file.into());
    }

    /// Every file that exists, those given by the test first, then the
    /// copies in the order they were made.
    #[must_use]
    pub fn files(&self) -> Vec<PathBuf> {
        lock(&self.entries).files.clone()
    }

    /// A path in the form in which two paths are compared.
    fn comparable(path: &Path) -> String {
        path.to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase()
    }

    /// Whether two paths name the same folder or file.
    fn is_same(first: &Path, second: &Path) -> bool {
        Self::comparable(first) == Self::comparable(second)
    }
}

impl FileSystem for PretendFileSystem {
    fn is_folder(&self, path: &Path) -> bool {
        lock(&self.entries)
            .folders
            .iter()
            .any(|folder| Self::is_same(folder, path))
    }

    fn resolve(&self, path: &Path) -> Result<PathBuf, StorageError> {
        lock(&self.entries)
            .folders
            .iter()
            .find(|folder| Self::is_same(folder, path))
            .cloned()
            .ok_or_else(|| {
                StorageError::new(format!("the folder {} does not exist", path.display()))
            })
    }

    fn file_names_in(&self, folder: &Path) -> Result<Vec<String>, StorageError> {
        Ok(lock(&self.entries)
            .files
            .iter()
            .filter(|file| {
                file.parent()
                    .is_some_and(|parent| Self::is_same(parent, folder))
            })
            .filter_map(|file| file.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .collect())
    }

    fn copy_file(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        let mut entries = lock(&self.entries);
        if !entries.files.iter().any(|file| Self::is_same(file, from)) {
            return Err(StorageError::new(format!(
                "the file {} could not be read",
                from.display()
            )));
        }
        entries.files.push(to.to_path_buf());
        Ok(())
    }
}

/// Remembers every change it is told about.
#[derive(Debug, Default)]
pub struct RecordingWorkListener {
    changes: Mutex<Vec<(Conversation, Vec<Turn>)>>,
}

impl RecordingWorkListener {
    /// Every change it was told about, in order: the conversation and its
    /// turns as they were then.
    #[must_use]
    pub fn changes(&self) -> Vec<(Conversation, Vec<Turn>)> {
        lock(&self.changes).clone()
    }

    /// Forgets the changes it was told about so far.
    pub fn forget(&self) {
        lock(&self.changes).clear();
    }
}

impl WorkListener for RecordingWorkListener {
    fn conversation_changed(&self, conversation: &Conversation, turns: &[Turn]) {
        lock(&self.changes).push((conversation.clone(), turns.to_vec()));
    }
}

/// An agent runtime that plays what a test tells it to.
///
/// It starts no program. A turn it starts is recorded and returns at once;
/// the test then delivers the events of that turn with
/// [`ScriptedRuntime::deliver`], on its own thread, as the real runtime
/// would on another.
pub struct ScriptedRuntime {
    installation: Mutex<Result<Installation, AgentRuntimeError>>,
    sign_in: Mutex<Result<SignIn, AgentRuntimeError>>,
    refusal: Mutex<Option<AgentRuntimeError>>,
    turns: Mutex<Vec<ScriptedTurn>>,
    opened: Mutex<Vec<(SessionId, PathBuf)>>,
}

/// A turn the scripted runtime has started.
struct ScriptedTurn {
    request: TurnRequest,
    /// Taken out while an event is delivered, so that the lock on the turns
    /// is not held while the listener runs.
    listener: Option<Box<dyn FnMut(AgentEvent) + Send>>,
    stopped: Arc<AtomicBool>,
}

impl Default for ScriptedRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptedRuntime {
    /// A runtime whose program is installed in version 2.1.322 and signed in
    /// with a subscription.
    #[must_use]
    pub fn new() -> Self {
        Self {
            installation: Mutex::new(Ok(Installation {
                program: PathBuf::from("claude"),
                version: ProgramVersion::new(2, 1, 322),
            })),
            sign_in: Mutex::new(Ok(SignIn::SignedIn {
                method: "claude.ai".to_owned(),
                provider: "firstParty".to_owned(),
                subscription: Some("max".to_owned()),
            })),
            refusal: Mutex::new(None),
            turns: Mutex::new(Vec::new()),
            opened: Mutex::new(Vec::new()),
        }
    }

    /// Decides what the runtime says about the installed program.
    pub fn set_installation(&self, installation: Result<Installation, AgentRuntimeError>) {
        *lock(&self.installation) = installation;
    }

    /// Decides what the runtime says about the sign-in.
    pub fn set_sign_in(&self, sign_in: Result<SignIn, AgentRuntimeError>) {
        *lock(&self.sign_in) = sign_in;
    }

    /// Makes the next start of a turn fail with an error.
    pub fn refuse_next_turn(&self, error: AgentRuntimeError) {
        *lock(&self.refusal) = Some(error);
    }

    /// The requests of the turns started so far, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<TurnRequest> {
        lock(&self.turns)
            .iter()
            .map(|turn| turn.request.clone())
            .collect()
    }

    /// Delivers an event to the listener of a turn, counted from 0 in the
    /// order the turns were started. A turn that was not started is ignored.
    pub fn deliver(&self, turn: usize, event: AgentEvent) {
        let listener = lock(&self.turns)
            .get_mut(turn)
            .and_then(|started| started.listener.take());
        let Some(mut listener) = listener else {
            return;
        };
        listener(event);
        if let Some(started) = lock(&self.turns).get_mut(turn) {
            started.listener = Some(listener);
        }
    }

    /// Whether a turn, counted from 0, was told to stop.
    #[must_use]
    pub fn was_stopped(&self, turn: usize) -> bool {
        lock(&self.turns)
            .get(turn)
            .is_some_and(|started| started.stopped.load(Ordering::SeqCst))
    }

    /// The sessions opened in the desktop app so far, with their folders.
    #[must_use]
    pub fn opened_sessions(&self) -> Vec<(SessionId, PathBuf)> {
        lock(&self.opened).clone()
    }
}

impl AgentRuntime for ScriptedRuntime {
    fn installation(&self) -> Result<Installation, AgentRuntimeError> {
        lock(&self.installation).clone()
    }

    fn sign_in(&self) -> Result<SignIn, AgentRuntimeError> {
        lock(&self.sign_in).clone()
    }

    fn start_turn(
        &self,
        request: TurnRequest,
        listener: Box<dyn FnMut(AgentEvent) + Send>,
    ) -> Result<Box<dyn RunningTurn>, AgentRuntimeError> {
        if let Some(error) = lock(&self.refusal).take() {
            return Err(error);
        }
        let stopped = Arc::new(AtomicBool::new(false));
        lock(&self.turns).push(ScriptedTurn {
            request,
            listener: Some(listener),
            stopped: Arc::clone(&stopped),
        });
        Ok(Box::new(ScriptedHandle { stopped }))
    }

    fn open_in_desktop_app(
        &self,
        session: &SessionId,
        working_folder: &Path,
    ) -> Result<(), AgentRuntimeError> {
        lock(&self.opened).push((session.clone(), working_folder.to_path_buf()));
        Ok(())
    }
}

/// The handle of a turn of the scripted runtime. Stopping it only records
/// that it was stopped: the test delivers the end of the turn itself.
struct ScriptedHandle {
    stopped: Arc<AtomicBool>,
}

impl RunningTurn for ScriptedHandle {
    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    fn is_finished(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    fn wait(self: Box<Self>) -> Result<TurnOutcome, AgentRuntimeError> {
        Err(AgentRuntimeError::CouldNotRun {
            message: "a scripted turn ends only when the test says so".to_owned(),
        })
    }
}

/// A clock that shows the time it was last set to.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<Timestamp>,
}

impl ManualClock {
    /// Creates a clock that shows `now` until it is changed.
    #[must_use]
    pub const fn starting_at(now: Timestamp) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Makes the clock show a different time.
    pub fn set(&self, now: Timestamp) {
        *lock(&self.now) = now;
    }

    /// Moves the clock forward.
    pub fn advance(&self, by: TimeDelta) {
        *lock(&self.now) += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        *lock(&self.now)
    }
}
