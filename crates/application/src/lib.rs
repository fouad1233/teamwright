//! Teamwright's use cases.
//!
//! This crate says what the program can do: start a project, open a task,
//! move it across the board, make it wait for another task. Each use case is
//! a method of a service ([`ProjectService`], [`TaskService`]) that loads what
//! it needs, lets the domain apply its rules, and stores the result.
//!
//! The crate reaches the outside world only through the traits in [`ports`]:
//! it does not know which database stores a task or where the time comes
//! from. Whoever builds the program decides that by passing implementations
//! in. Tests pass the ones in [`in_memory`].

pub mod clock;
pub mod error;
pub mod in_memory;
pub mod ports;
pub mod projects;
pub mod tasks;

pub use clock::SystemClock;
pub use error::ApplicationError;
pub use ports::{Clock, ProjectRepository, StorageError, TaskRepository};
pub use projects::{NewProject, ProjectService, WorkflowTemplate};
pub use tasks::{NewTask, TaskService};
