//! Teamwright's use cases.
//!
//! This crate says what the program can do: start a project, open a task,
//! move it across the board, make it wait for another task, put someone on
//! the team, choose the folders of a project, let an AI specialist work on a
//! task. Each use case is a method of a service ([`ProjectService`],
//! [`TaskService`], [`TeamService`], [`PlacesService`], [`WorkService`])
//! that loads what it needs, lets the domain apply its rules, and stores the
//! result.
//!
//! The crate reaches the outside world only through the traits in [`ports`]:
//! it does not know which database stores a task or where the time comes
//! from. Whoever builds the program decides that by passing implementations
//! in. Tests pass the ones in [`in_memory`].

pub mod agent_runtime;
pub mod clock;
pub mod error;
pub mod in_memory;
pub mod places;
pub mod ports;
pub mod projects;
pub mod tasks;
pub mod team;
pub mod work;

pub use agent_runtime::{
    AgentEvent, AgentRuntime, AgentRuntimeError, Effort, Installation, InterviewQuestionAsked,
    ManagerInterview, ManagerQuestion, McpServer, ModelUsage, PermissionDenial, PermissionMode,
    PermissionProfile, PlanProposal, PlanUsage, PlanWindow, PlanWindowKind, PlannedMember,
    PlannedOwner, PlannedTask, ProgramVersion, QuestionOption, RunningTurn, SessionChoice,
    SessionId, SessionStarted, SignIn, TeamTool, TokenCounts, TurnFailure, TurnOutcome,
    TurnRequest,
};
pub use clock::SystemClock;
pub use error::ApplicationError;
pub use places::{ChosenPlaces, PlacesService, ProjectPlaces, REFERENCES_FOLDER};
pub use ports::{
    ApprovedPlan, Clock, ConversationRepository, FileSystem, MemberRepository, PlacesRepository,
    PlanningRepository, ProjectFolders, ProjectRepository, StorageError, TaskRepository,
    WorkListener,
};
pub use projects::{NewProject, ProjectService, StatusProperties, WorkflowTemplate};
pub use tasks::{NewTask, TaskService};
pub use team::{NewMember, TeamService};
pub use work::{
    AgentProgram, AgentProgramState, ApprovedMember, ApprovedOwner, ApprovedProposal, ApprovedTask,
    ConversationWithTurns, WorkPorts, WorkService, answers_in_words, brief_for, instructions_for,
    instructions_for_advisor,
};
