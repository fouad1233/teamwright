//! SQLite storage for Teamwright.
//!
//! This crate implements the storage interfaces that the application crate
//! declares. It keeps the current state of projects, tasks, sprints, views,
//! areas, teams, the folders of the projects and the conversations with specialists
//! in a
//! SQLite database, and it appends every change to an operation log in the same
//! transaction. The log records which computer made each change and in what
//! order, which is what later allows several computers to synchronize.
//!
//! ```no_run
//! use std::sync::Arc;
//! use teamwright_storage::{Database, SqliteProjectRepository, SqliteTaskRepository};
//!
//! # fn main() -> Result<(), teamwright_application::StorageError> {
//! let database = Arc::new(Database::open("teamwright.db")?);
//! let projects = SqliteProjectRepository::new(database.clone());
//! let tasks = SqliteTaskRepository::new(database);
//! # Ok(())
//! # }
//! ```

mod apply;
mod areas;
mod codec;
mod conversations;
mod database;
mod error;
mod folder;
mod ledger;
mod members;
mod merge;
mod migrations;
mod notices;
mod operations;
mod places;
mod planning;
mod projects;
mod sprints;
mod take_in;
mod tasks;
mod views;

pub use areas::SqliteAreaRepository;
pub use conversations::SqliteConversationRepository;
pub use database::Database;
pub use folder::FolderOfComputers;
pub use ledger::SqliteSyncLedger;
pub use members::SqliteMemberRepository;
pub use operations::Operation;
pub use places::SqlitePlacesRepository;
pub use planning::SqlitePlanningRepository;
pub use projects::SqliteProjectRepository;
pub use sprints::SqliteSprintRepository;
pub use tasks::SqliteTaskRepository;
pub use views::SqliteViewRepository;
