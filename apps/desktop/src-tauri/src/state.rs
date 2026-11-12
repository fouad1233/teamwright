//! The services the commands work with, built once when the application
//! starts.

use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

use tauri::{AppHandle, Emitter, Manager};
use teamwright_application::{
    AgentRuntime, ApplicationError, FileSystem, PlacesService, ProjectFolders, ProjectService,
    StorageError, SystemClock, TaskService, TeamService, WorkListener, WorkPorts, WorkService,
};
use teamwright_domain::{Conversation, Project, Turn};
use teamwright_runtime::ClaudeCode;
use teamwright_storage::{
    Database, SqliteConversationRepository, SqliteMemberRepository, SqlitePlacesRepository,
    SqliteProjectRepository, SqliteTaskRepository,
};

use crate::dto::ConversationDto;

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
    /// The use cases for the team of a project.
    pub team: TeamService,
    /// The use cases for the folders of a project.
    pub places: PlacesService,
    /// The use cases for the work of the AI specialists.
    pub work: WorkService,
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

        let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
        let task_repository = Arc::new(SqliteTaskRepository::new(database.clone()));
        let member_repository = Arc::new(SqliteMemberRepository::new(database.clone()));
        let places_repository = Arc::new(SqlitePlacesRepository::new(database.clone()));
        let conversation_repository = Arc::new(SqliteConversationRepository::new(database));
        let runtime: Arc<dyn AgentRuntime> = Arc::new(agent_program());
        let clock = Arc::new(SystemClock);
        let folders: Arc<dyn ProjectFolders> = Arc::new(FoldersOfProjects {
            root: projects_directory(app).map_err(|error| error.to_string()),
        });
        let file_system: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);

        let work = WorkService::new(WorkPorts {
            projects: project_repository.clone(),
            tasks: task_repository.clone(),
            members: member_repository.clone(),
            conversations: conversation_repository.clone(),
            folders: folders.clone(),
            places: places_repository.clone(),
            file_system: file_system.clone(),
            runtime,
            listener: Arc::new(EventToInterface { app: app.clone() }),
            clock: clock.clone(),
        });
        work.recover()?;

        Ok(Self {
            projects: ProjectService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                places_repository.clone(),
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
                clock.clone(),
            ),
            team: TeamService::new(
                project_repository,
                member_repository,
                task_repository,
                conversation_repository,
                clock,
            ),
            work,
        })
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
}

impl WorkListener for EventToInterface {
    fn conversation_changed(&self, conversation: &Conversation, turns: &[Turn]) {
        // A window that is closing cannot be told anything. The change is
        // stored, and the interface reads it when it lists the
        // conversations again.
        let _ = self.app.emit(
            CONVERSATION_CHANGED,
            ConversationDto::new(conversation, turns),
        );
    }
}
