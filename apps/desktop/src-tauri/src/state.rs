//! The services the commands work with, built once when the application
//! starts.

use std::path::PathBuf;
use std::sync::Arc;
use std::{env, fs};

use tauri::{AppHandle, Manager};
use teamwright_application::{ProjectService, StorageError, SystemClock, TaskService, TeamService};
use teamwright_storage::{
    Database, SqliteConversationRepository, SqliteMemberRepository, SqliteProjectRepository,
    SqliteTaskRepository,
};

/// The name of the database file inside the application's data directory.
const DATABASE_FILE_NAME: &str = "teamwright.db";

/// Names a folder to keep the data in, instead of the place the operating
/// system assigns to the application. Tests use it to stay away from the
/// user's real data, and it allows an installation that carries its data
/// along.
const DATA_DIRECTORY_VARIABLE: &str = "TEAMWRIGHT_DATA_DIR";

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
}

/// Everything a command needs to do its work.
pub struct AppState {
    /// The use cases for projects.
    pub projects: ProjectService,
    /// The use cases for tasks.
    pub tasks: TaskService,
    /// The use cases for the team of a project.
    pub team: TeamService,
}

impl AppState {
    /// Opens the database in the application's data directory and builds the
    /// services on top of it.
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
        let conversation_repository = Arc::new(SqliteConversationRepository::new(database));
        let clock = Arc::new(SystemClock);

        Ok(Self {
            projects: ProjectService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                clock.clone(),
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
        })
    }
}

/// The folder the data is kept in: the one named by `TEAMWRIGHT_DATA_DIR` if
/// that is set, and otherwise the place the operating system assigns to the
/// application.
fn data_directory(app: &AppHandle) -> Result<PathBuf, StartupError> {
    match env::var_os(DATA_DIRECTORY_VARIABLE) {
        Some(directory) if !directory.is_empty() => Ok(PathBuf::from(directory)),
        _ => Ok(app.path().app_data_dir()?),
    }
}
