//! The Teamwright domain model.
//!
//! This crate holds the concepts a manager works with (projects, workflows,
//! tasks) and the rules that keep them consistent. It performs no input or
//! output: nothing here reads a file, opens a database or talks to an agent.
//!
//! Two properties follow from that and the rest of the system relies on them:
//!
//! - Every rule can be tested in isolation.
//! - A value of one of these types is always valid. Types keep their fields
//!   private and can only be created through constructors that check the
//!   rules, so an empty task title or a workflow without a "done" status
//!   cannot exist.

mod macros;

pub mod dependencies;
pub mod error;
pub mod ids;
pub mod project;
pub mod task;
pub mod text;
pub mod time;
pub mod workflow;

pub use dependencies::DependencyGraph;
pub use error::DomainError;
pub use ids::{MemberId, ProjectId, StatusId, TaskId};
pub use project::{Project, ProjectKey, ProjectSnapshot};
pub use task::{Assignee, Priority, Rank, Schedule, Task, TaskDraft, TaskSnapshot};
pub use text::{ProjectName, StatusName, TaskTitle};
pub use time::{CalendarDate, Timestamp};
pub use workflow::{Status, StatusCategory, StatusColor, Workflow};
