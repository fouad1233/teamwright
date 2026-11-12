//! The services the commands work with, built once when the application
//! starts.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use chrono::Utc;
use tauri::{AppHandle, Emitter, Manager};
use teamwright_application::{
    AgentRuntime, ApplicationError, FileSystem, PlacesService, ProjectFolders, ProjectService,
    RoundReport, SprintService, StorageError, SyncPorts, SyncService, SystemClock, TaskService,
    TeamService, WorkListener, WorkPorts, WorkService,
};
use teamwright_domain::{ComputerId, Conversation, Project, Turn};
use teamwright_runtime::ClaudeCode;
use teamwright_storage::{
    Database, FolderOfComputers, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqlitePlanningRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteSyncLedger, SqliteTaskRepository, SqliteViewRepository,
};

use crate::dto::{ConversationDto, DataChangedDto, SyncStateDto};
use crate::error::CommandError;

/// The name of the database file inside the application's data directory.
const DATABASE_FILE_NAME: &str = "teamwright.db";

/// Names a folder to keep the data in, instead of the place the operating
/// system assigns to the application. Tests use it to stay away from the
/// user's real data, and it allows an installation that carries its data
/// along.
const DATA_DIRECTORY_VARIABLE: &str = "TEAMWRIGHT_DATA_DIR";

/// Names the program that runs the specialists, instead of the Claude Code
/// that is looked for on the computer. It serves a Claude Code that is
/// installed where nobody would look. The tools that drive the program for
/// tests and pictures point it at a stand-in, so that what they see does
/// not depend on what is installed on the computer they run on, and never
/// shows how its user is signed in.
const AGENT_PROGRAM_VARIABLE: &str = "TEAMWRIGHT_AGENT_PROGRAM";

/// The argument with which this program serves the team bus instead of
/// opening its window. Claude Code starts it that way for every turn of a
/// specialist, so that the specialist can ask the manager to decide.
pub const TEAM_BUS_ARGUMENT: &str = "--team-bus";

/// The folder in the user's documents that holds the folders of the
/// projects ([decision 0011](../../../../docs/decisions/0011-what-a-specialist-may-do.md)).
const PROJECTS_FOLDER_NAME: &str = "Teamwright";

/// The folder that holds the folders of the projects when the data is kept
/// in the folder named by `TEAMWRIGHT_DATA_DIR`. A test, and an installation
/// that carries its data along, then keep the work of the specialists there
/// too, and out of the user's documents.
const PROJECTS_FOLDER_IN_DATA_DIRECTORY: &str = "projects";

/// The event the interface listens to. Its payload is the whole
/// conversation that changed.
const CONVERSATION_CHANGED: &str = "conversation-changed";

/// The event that says that syncing changed something in projects. Its
/// payload names the projects (`DataChangedDto`).
const DATA_CHANGED: &str = "data-changed";

/// The event that says where syncing stands, after every round. Its payload
/// is the whole state (`SyncStateDto`).
const SYNC_CHANGED: &str = "sync-changed";

/// How often the background thread of syncing looks whether a round is due.
const LOOK_EVERY: Duration = Duration::from_secs(5);

/// How long this computer stays quiet before its changes are written into
/// the sync folder.
const QUIET_FOR: Duration = Duration::from_secs(5);

/// How long changes wait at most to be written while a specialist keeps
/// changing things.
const WRITE_AT_LEAST_EVERY: Duration = Duration::from_secs(30);

/// How often the files of the other computers are read
/// ([decision 0017](../../../../docs/decisions/0017-sync-through-a-folder.md)).
const READ_EVERY: Duration = Duration::from_secs(20);

/// Why the application could not get ready.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    /// The operating system did not say where application data belongs.
    #[error("the folder for application data could not be determined: {0}")]
    NoDataDirectory(#[from] tauri::Error),

    /// The folder for application data could not be created.
    #[error("the folder for application data could not be created: {0}")]
    DataDirectoryNotCreated(#[from] std::io::Error),

    /// The database could not be opened.
    #[error("the database could not be opened: {0}")]
    DatabaseNotOpened(#[from] StorageError),

    /// The turns that were cut off by closing the program could not be
    /// ended.
    #[error("the work of the specialists could not be brought up to date: {0}")]
    WorkNotRecovered(#[from] ApplicationError),
}

/// Everything a command needs to do its work.
pub struct AppState {
    /// The use cases for projects.
    pub projects: ProjectService,
    /// The use cases for tasks.
    pub tasks: TaskService,
    /// The use cases for sprints.
    pub sprints: SprintService,
    /// The use cases for the team of a project.
    pub team: TeamService,
    /// The use cases for the folders of a project.
    pub places: PlacesService,
    /// The use cases for the work of the AI specialists.
    pub work: WorkService,
    /// The use cases for keeping this computer in step with the manager's
    /// other computers.
    pub sync: SyncService,
    /// The identity of this computer, which tells a conversation that runs
    /// here from one that runs on another computer.
    pub this_computer: ComputerId,
}

impl AppState {
    /// Opens the database in the application's data directory and builds the
    /// services on top of it. Turns that closing the program cut off are
    /// ended.
    ///
    /// This is the one place where the program decides which implementation
    /// stands behind each interface of the application crate.
    pub fn open(app: &AppHandle) -> Result<Self, StartupError> {
        let data_directory = data_directory(app)?;
        fs::create_dir_all(&data_directory)?;
        let database = Arc::new(Database::open(data_directory.join(DATABASE_FILE_NAME))?);
        let this_computer = database.device_id();
        let sync_ledger = Arc::new(SqliteSyncLedger::new(database.clone()));

        let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
        let task_repository = Arc::new(SqliteTaskRepository::new(database.clone()));
        let member_repository = Arc::new(SqliteMemberRepository::new(database.clone()));
        let places_repository = Arc::new(SqlitePlacesRepository::new(database.clone()));
        let sprint_repository = Arc::new(SqliteSprintRepository::new(database.clone()));
        let view_repository = Arc::new(SqliteViewRepository::new(database.clone()));
        let conversation_repository = Arc::new(SqliteConversationRepository::new(database.clone()));
        let planning_repository = Arc::new(SqlitePlanningRepository::new(database));
        let runtime: Arc<dyn AgentRuntime> = Arc::new(agent_program());
        let clock = Arc::new(SystemClock);
        let folders: Arc<dyn ProjectFolders> = Arc::new(FoldersOfProjects {
            root: projects_directory(app).map_err(|error| error.to_string()),
        });
        let file_system: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let sync = SyncService::new(SyncPorts {
            ledger: sync_ledger,
            folder: Arc::new(FolderOfComputers::new()),
            file_system: file_system.clone(),
            clock: clock.clone(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            data_folder: data_directory,
        });

        let work = WorkService::new(WorkPorts {
            projects: project_repository.clone(),
            tasks: task_repository.clone(),
            members: member_repository.clone(),
            conversations: conversation_repository.clone(),
            planning: planning_repository,
            folders: folders.clone(),
            places: places_repository.clone(),
            file_system: file_system.clone(),
            runtime,
            listener: Arc::new(EventToInterface {
                app: app.clone(),
                this_computer,
            }),
            clock: clock.clone(),
            this_computer,
        });
        work.recover()?;

        Ok(Self {
            projects: ProjectService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                places_repository.clone(),
                sprint_repository.clone(),
                view_repository.clone(),
                clock.clone(),
            ),
            places: PlacesService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                places_repository,
                folders,
                file_system,
            ),
            tasks: TaskService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                sprint_repository.clone(),
                clock.clone(),
            ),
            sprints: SprintService::new(
                project_repository.clone(),
                task_repository.clone(),
                sprint_repository,
                clock.clone(),
            ),
            team: TeamService::new(
                project_repository,
                member_repository,
                task_repository,
                conversation_repository,
                view_repository,
                clock,
            ),
            work,
            sync,
            this_computer,
        })
    }
}

/// Where syncing stands, for the interface, with the name the operating
/// system gives this computer as a suggestion.
pub fn sync_state_dto(state: &AppState) -> Result<SyncStateDto, CommandError> {
    Ok(SyncStateDto::new(
        &state.sync.state()?,
        suggested_computer_name(),
    ))
}

/// The name the operating system gives this computer, if it says. It is
/// offered as the name of the computer, never used without the manager.
fn suggested_computer_name() -> Option<String> {
    ["COMPUTERNAME", "HOSTNAME"]
        .into_iter()
        .filter_map(|variable| env::var(variable).ok())
        .map(|name| name.trim().to_owned())
        .find(|name| !name.is_empty())
}

/// Runs a round of syncing and tells the interface what it changed: the
/// event `data-changed` with the projects, `conversation-changed` for every
/// conversation, and `sync-changed` with the state. Turns that another
/// computer deleted while they ran here are stopped. A round started here
/// and one started by the background thread never run at once: the
/// service runs one at a time.
pub fn run_round(app: &AppHandle, state: &AppState) -> Result<RoundReport, CommandError> {
    let result = state.sync.round();
    if let Ok(report) = &result {
        tell_interface(app, state, report);
    }
    // A window that is closing cannot be told anything; the interface reads
    // the state again when it asks.
    if let Ok(dto) = sync_state_dto(state) {
        let _ = app.emit(SYNC_CHANGED, dto);
    }
    Ok(result?)
}

/// Tells the interface what a round changed, and stops the turns that were
/// deleted while they ran here.
fn tell_interface(app: &AppHandle, state: &AppState, report: &RoundReport) {
    state.work.stop_turns(&report.turns_to_stop);
    if !report.changed_projects.is_empty() {
        let _ = app.emit(
            DATA_CHANGED,
            DataChangedDto::from(report.changed_projects.as_slice()),
        );
    }
    for id in &report.changed_conversations {
        if let Ok(Some(conversation)) = state.work.conversation(*id) {
            let _ = app.emit(
                CONVERSATION_CHANGED,
                ConversationDto::of(&conversation, state.this_computer),
            );
        }
    }
}

/// Starts the thread that runs the rounds of syncing by itself. Every five
/// seconds it looks: the changes of this computer are written once they are
/// five seconds old, or at the latest thirty seconds after the last round,
/// and the other computers are read every twenty seconds. Nothing happens
/// while no sync folder is chosen.
pub fn start_syncing_in_the_background(app: AppHandle) {
    // Without the thread, syncing still runs when the manager asks for a
    // round; the program goes on either way.
    let _ = thread::Builder::new()
        .name("sync".to_owned())
        .spawn(move || background_rounds(&app));
}

/// The loop of the thread that runs the rounds of syncing.
fn background_rounds(app: &AppHandle) {
    let mut last_round: Option<Instant> = None;
    loop {
        thread::sleep(LOOK_EVERY);
        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        let is_on = state.sync.state().is_ok_and(|sync| sync.folder.is_some());
        if !is_on {
            continue;
        }
        let since_last_round = last_round.map(|at| at.elapsed());
        let changes_to_write = match state.sync.unwritten_since() {
            Ok(Some(changed_at)) => {
                let quiet = (Utc::now() - changed_at)
                    .to_std()
                    .is_ok_and(|quiet| quiet >= QUIET_FOR);
                quiet || since_last_round.is_none_or(|since| since >= WRITE_AT_LEAST_EVERY)
            }
            Ok(None) | Err(_) => false,
        };
        let time_to_read = since_last_round.is_none_or(|since| since >= READ_EVERY);
        if changes_to_write || time_to_read {
            // What went wrong is in the state the interface was told.
            let _ = run_round(app, &state);
            last_round = Some(Instant::now());
        }
    }
}

/// The folder data is kept in: the one named by `TEAMWRIGHT_DATA_DIR` if
/// that is set, and otherwise the place the operating system assigns to the
/// application.
fn data_directory(app: &AppHandle) -> Result<PathBuf, StartupError> {
    match env::var_os(DATA_DIRECTORY_VARIABLE) {
        Some(directory) if !directory.is_empty() => Ok(PathBuf::from(directory)),
        _ => Ok(app.path().app_data_dir()?),
    }
}

/// The program that runs the specialists: the one named by
/// `TEAMWRIGHT_AGENT_PROGRAM` if that is set, and otherwise the Claude Code
/// that is found on the computer.
///
/// It starts the team bus as this very program with `--team-bus`. If the
/// path of this program cannot be found, the specialists work without the
/// tool to ask the manager.
fn agent_program() -> ClaudeCode {
    let runtime = match env::var_os(AGENT_PROGRAM_VARIABLE) {
        Some(program) if !program.is_empty() => {
            ClaudeCode::new().with_program(PathBuf::from(program))
        }
        _ => ClaudeCode::new(),
    };
    match env::current_exe() {
        Ok(this_program) => runtime.with_team_bus(this_program, vec![TEAM_BUS_ARGUMENT.to_owned()]),
        Err(_) => runtime,
    }
}

/// The folder that holds the folders of the projects: `projects` in the
/// folder named by `TEAMWRIGHT_DATA_DIR` if that is set, and otherwise
/// `Teamwright` in the user's documents.
fn projects_directory(app: &AppHandle) -> Result<PathBuf, tauri::Error> {
    match env::var_os(DATA_DIRECTORY_VARIABLE) {
        Some(directory) if !directory.is_empty() => {
            Ok(PathBuf::from(directory).join(PROJECTS_FOLDER_IN_DATA_DIRECTORY))
        }
        _ => Ok(app.path().document_dir()?.join(PROJECTS_FOLDER_NAME)),
    }
}

/// Gives each project a folder of its own, named by the key of the project.
struct FoldersOfProjects {
    /// The folder that holds them, or why the operating system could not say
    /// where it is. A computer without a folder for documents can still plan
    /// a project: only the work of a specialist needs the folder, so only
    /// that work is refused.
    root: Result<PathBuf, String>,
}

impl ProjectFolders for FoldersOfProjects {
    fn folder_of(&self, project: &Project) -> Result<PathBuf, StorageError> {
        let root = self.root.as_ref().map_err(|problem| {
            StorageError::new(format!(
                "the folder for the projects could not be determined: {problem}"
            ))
        })?;
        let folder = root.join(project.key().as_str());
        fs::create_dir_all(&folder).map_err(|error| {
            StorageError::caused_by(
                format!(
                    "the folder of the project, {}, could not be created",
                    folder.display()
                ),
                error,
            )
        })?;
        Ok(folder)
    }
}

/// The files and folders of this computer, through the standard library.
struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn is_folder(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn resolve(&self, path: &Path) -> Result<PathBuf, StorageError> {
        // The form the agent program knows a folder in. A specialist is
        // told about its reference folders in this form, and must name
        // them in it.
        teamwright_runtime::full_form(path).map_err(|error| {
            StorageError::caused_by(
                format!("the folder {} could not be reached", path.display()),
                error,
            )
        })
    }

    fn file_names_in(&self, folder: &Path) -> Result<Vec<String>, StorageError> {
        let could_not_read = |error: io::Error| {
            StorageError::caused_by(
                format!("the folder {} could not be read", folder.display()),
                error,
            )
        };
        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(could_not_read(error)),
        };
        entries
            .map(|entry| {
                entry
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .map_err(could_not_read)
            })
            .collect()
    }

    fn copy_file(&self, from: &Path, to: &Path) -> Result<(), StorageError> {
        if let Some(folder) = to.parent() {
            fs::create_dir_all(folder).map_err(|error| {
                StorageError::caused_by(
                    format!("the folder {} could not be created", folder.display()),
                    error,
                )
            })?;
        }
        fs::copy(from, to).map_err(|error| {
            StorageError::caused_by(
                format!(
                    "the file {} could not be copied to {}",
                    from.display(),
                    to.display()
                ),
                error,
            )
        })?;
        Ok(())
    }
}

/// Tells the interface about every change to a conversation, with the event
/// `conversation-changed`.
struct EventToInterface {
    app: AppHandle,
    this_computer: ComputerId,
}

impl WorkListener for EventToInterface {
    fn conversation_changed(&self, conversation: &Conversation, turns: &[Turn]) {
        // A window that is closing cannot be told anything. The change is
        // stored, and the interface reads it when it lists the
        // conversations again.
        let _ = self.app.emit(
            CONVERSATION_CHANGED,
            ConversationDto::new(conversation, turns, self.this_computer),
        );
    }
}
