//! The Teamwright domain model.
//!
//! This crate holds the concepts a manager works with (projects, workflows,
//! tasks, team members) and the rules that keep them consistent. It performs
//! no input or output: nothing here reads a file, opens a database or talks
//! to an agent.
//!
//! Two properties follow from that and the rest of the system relies on them:
//!
//! - Every rule can be tested in isolation.
//! - A value of one of these types is always valid. Types keep their fields
//!   private and can only be created through constructors that check the
//!   rules, so an empty task title or a workflow without a "done" status
//!   cannot exist.

mod macros;

pub mod area;
pub mod checklist;
pub mod color;
pub mod conversation;
pub mod dependencies;
pub mod error;
pub mod ids;
pub mod integration;
pub mod member;
pub mod memory;
pub mod planning;
pub mod project;
pub mod project_files;
pub mod ready_made;
pub mod sprint;
pub mod sprint_course;
pub mod tag;
pub mod task;
pub mod task_history;
pub mod teamwork;
pub mod template;
pub mod text;
pub mod time;
pub mod velocity;
pub mod view;
pub mod work_in_progress;
pub mod workflow;

pub use area::{Area, AreaSnapshot};
pub use checklist::{Checklist, ChecklistItem};
pub use color::Color;
pub use conversation::{
    ContextUse, Conversation, ConversationSnapshot, DecisionAnswer, DecisionOption,
    DecisionRequest, DecisionSnapshot, Step, StepContent, StepKind, TokenCounts, ToolOutcome, Turn,
    TurnOrigin, TurnSnapshot, TurnState, context_of,
};
pub use dependencies::DependencyGraph;
pub use error::DomainError;
pub use ids::{
    AreaId, ChecklistItemId, ComputerId, ConversationId, HandoffId, MemberId, MemoryEntryId,
    ProjectId, SprintId, StatusId, TagId, TaskId, TeamMessageId, TurnId, ViewId,
};
pub use integration::{EventKind, EventKinds, RepositoryName, WebAddress};
pub use member::{Effort, Member, MemberInstructions, MemberKind, MemberProfile, MemberSnapshot};
pub use memory::{
    MemoryContent, MemoryEntry, MemoryEntrySnapshot, MemoryKind, MemoryOrigin, MemorySource,
    SearchWords,
};
pub use planning::{
    Interview, InterviewAnswer, InterviewQuestion, InterviewSnapshot, Proposal, ProposalChange,
    ProposalSnapshot, ProposedMember, ProposedOwner, ProposedTask,
};
pub use project::{Project, ProjectKey, ProjectSnapshot};
pub use project_files::{EntryKind, FolderEntry, PathInFolder};
pub use ready_made::ReadyMadeSpecialist;
pub use sprint::{
    CountedTask, Sprint, SprintDates, SprintSnapshot, SprintState, SprintSummary, SprintUnit,
    StoryPoints,
};
pub use sprint_course::{
    CourseDay, MovedOn, ScopeChange, ScopeTask, SprintCourse, TaskPast, course_of,
};
pub use tag::{Tag, TagSnapshot};
pub use task::{Assignee, Priority, Rank, Schedule, Task, TaskDraft, TaskSnapshot};
pub use task_history::{TaskChange, TaskChangeKind, changes_between, history_of};
pub use teamwork::{
    Handoff, HandoffContent, HandoffSnapshot, MessageDelivery, MessageKind, SaidIn, TeamMessage,
    TeamMessageSnapshot,
};
pub use template::{ProjectTemplate, TemplateStatus};
pub use text::{
    AreaName, ChecklistText, ComputerName, MemberName, ModelName, ProjectName, SprintName,
    StatusName, TagName, TaskTitle, ViewName,
};
pub use time::{CalendarDate, Timestamp};
pub use velocity::{
    FEWEST_SPRINTS_FOR_A_PACE, Load, MOST_SPRINTS_FOR_A_PACE, Pace, SprintsNeeded, Velocity,
};
pub use view::{View, ViewAssignee, ViewFilter, ViewKind, ViewSnapshot};
pub use work_in_progress::{Fill, StatusLoad, WorkInProgressLimit};
pub use workflow::{Status, StatusCategory, Workflow};
