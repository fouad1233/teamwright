//! SQLite storage for Teamwright.
//!
//! This crate implements the storage interfaces that the application crate
//! declares. It keeps the current state of projects, tasks and teams in a
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

mod codec;
mod database;
mod error;
mod members;
mod migrations;
mod operations;
mod projects;
mod tasks;

pub use database::Database;
pub use members::SqliteMemberRepository;
pub use operations::{DeviceId, Operation};
pub use projects::SqliteProjectRepository;
pub use tasks::SqliteTaskRepository;
