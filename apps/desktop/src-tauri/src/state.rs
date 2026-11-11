//! The services the commands work with, built once when the application
//! starts.

use std::fs;
use std::sync::Arc;

use tauri::{AppHandle, Manager};
use teamwright_application::{ProjectService, StorageError, SystemClock, TaskService};
use teamwright_storage::{Database, SqliteProjectRepository, SqliteTaskRepository};

/// The name of the database file inside the application's data directory.
const DATABASE_FILE_NAME: &str = "teamwright.db";

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
}

impl AppState {
    /// Opens the database in the application's data directory and builds the
    /// services on top of it.
    ///
    /// This is the one place where the program decides which implementation
    /// stands behind each interface of the application crate.
    pub fn open(app: &AppHandle) -> Result<Self, StartupError> {
        let data_directory = app.path().app_data_dir()?;
        fs::create_dir_all(&data_directory)?;
        let database = Arc::new(Database::open(data_directory.join(DATABASE_FILE_NAME))?);

        let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
        let task_repository = Arc::new(SqliteTaskRepository::new(database));
        let clock = Arc::new(SystemClock);

        Ok(Self {
            projects: ProjectService::new(project_repository.clone(), clock.clone()),
            tasks: TaskService::new(project_repository, task_repository, clock),
        })
    }
}
