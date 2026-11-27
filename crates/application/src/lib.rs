//! Teamwright's use cases.
//!
//! This crate says what the program can do: start a project, open a task,
//! move it across the board, make it wait for another task, plan it into a
//! sprint, put someone on the team, choose the folders of a project, let an
//! AI specialist work on a task. Each use case is a method of a service
//! ([`ProjectService`], [`TaskService`], [`SprintService`], [`ViewService`],
//! [`AreaService`], [`TagService`], [`ChecklistService`], [`FieldService`], [`TeamService`],
//! [`PlacesService`], [`WorkService`],
//! [`WorkbookService`], [`ProjectReportService`])
//! that loads what it
//! needs, lets the
//! domain apply its rules, and stores the result.
//!
//! The crate reaches the outside world only through the traits in [`ports`]:
//! it does not know which database stores a task or where the time comes
//! from. Whoever builds the program decides that by passing implementations
//! in. Tests pass the ones in [`in_memory`].

pub mod agent_runtime;
pub mod appearance;
pub mod areas;
pub mod checklists;
pub mod clock;
pub mod coordinator;
pub mod digests;
pub mod error;
pub mod fields;
pub mod files;
pub mod google_drive;
pub mod groups;
pub mod in_memory;
pub mod integrations;
pub mod kits;
pub mod marks;
pub mod memory;
pub mod notifications;
pub mod permissions;
pub mod places;
pub mod plan_limit;
pub mod ports;
pub mod project_events;
pub mod project_report;
pub mod projects;
pub mod sprints;
pub mod sync;
pub mod tags;
pub mod tasks;
pub mod team;
pub mod teamwork;
pub mod update_install;
pub mod updates;
pub mod usage;
pub mod views;
pub mod work;
pub mod workbook;

pub use agent_runtime::{
    AgentEvent, AgentRuntime, AgentRuntimeError, Effort, Installation, InterviewQuestionAsked,
    ManagerInterview, ManagerQuestion, McpServer, MemoryKind, MessageKind, ModelUsage,
    PermissionAsk, PermissionDenial, PermissionMode, PermissionProfile, PlanLimitHit, PlanProposal,
    PlanReset, PlanUsage, PlanWindow, PlanWindowKind, PlannedMember, PlannedOwner, PlannedTask,
    ProgramVersion, PublishedHandoff, QuestionOption, RememberedEntry, RememberedFact, RunningTurn,
    SaidDate, SaidReset, SentMessage, SessionChoice, SessionContext, SessionId, SessionStarted,
    SignIn, TeamTool, TokenCounts, TurnFailure, TurnOutcome, TurnRequest,
};
pub use agent_runtime::{ConnectorStart, KitConnector, KitSkill, TurnKit};
pub use agent_runtime::{ProtectedKind, ProtectedPlace};
pub use appearance::{AppearanceService, Theme, ThemeStore};
pub use areas::AreaService;
pub use checklists::ChecklistService;
pub use clock::SystemClock;
pub use coordinator::{
    BudgetQuestionOfProject, CoordinatorPorts, CoordinatorService, CoordinatorState, GO_ON_REQUEST,
    GoOn, MESSAGE_TURNS_PER_HOUR, Plan, Situation, Spending, Start, WAKE_REQUEST, WaitReason,
    Waiting, Wake, next_steps, spending_of,
};
pub use digests::{
    ConversationsOfFolder, DigestService, DigestToKeep, EarlierConversationCard, group_by_folder,
    newest_kept_first,
};
pub use error::ApplicationError;
pub use fields::FieldService;
pub use files::{FolderListing, ProjectFilesService};
pub use google_drive::{
    DriveFinding, DriveFolder, DriveSurroundings, FOLDER_IN_DRIVE, GoogleDrive, GoogleDriveService,
    OperatingSystem, find_drive_folders,
};
pub use groups::GroupingService;
pub use integrations::delivery::{
    Deliveries, Delivery, DeliveryPorts, DeliveryService, Target, WebSender, targets_of,
};
pub use integrations::github::{
    GitHubItem, GitHubOverview, GitHubPorts, GitHubProgram, GitHubReading, GitHubService,
    GitHubSignIn, ItemKind, ItemState, TaskOnGitHub, mentions, sign_in_problem,
};
pub use integrations::messages::{
    EventPayload, PAYLOAD_VERSION, SprintOfPayload, channel_message_of, kind_of, payload_of,
    test_payload,
};
pub use integrations::{
    IntegrationSettings, IntegrationStore, ListeningAddress, ProjectIntegrations, SlackChannel,
    events_sent_to_an_address, events_told_in_a_channel,
};
pub use kits::{
    ChosenFolderProblem, ConnectorTrust, InMemoryConnectorTrust, InMemoryKitRepository, KeptLayout,
    KeptSkill, KitChoice, KitRepository, KitService, Library, LibraryConnector, LibrarySkill,
    MANIFEST_FILE, MANIFEST_FOLDER, MAX_SKILL_BYTES, MAX_SKILL_FILES, NewConnector,
    NewConnectorStart, NewHeader, PretendShelf, SKILL_FILE, SKILLS_FOLDER, SkillFolder, SkillShelf,
    WaitingTools, check_kept_layout, turn_kit_of, variables_of_the_library,
};
pub use marks::{FixedMark, MarkSource, RandomMarks, test_mark};
pub use memory::{
    MOST_FACTS_PER_TURN, MemoryService, NewFact, Searchable, contract_entries, decision_entry,
    memory_in_words, remembered_entries, search,
};
pub use notifications::{
    Notification, NotificationChoice, NotificationChoiceStore, NotificationKind,
    NotificationPermission, NotificationService, notification_for, notification_of,
};
pub use permissions::{CommandChoiceStore, CommandPermissions, PermissionService};
pub use places::{ChosenPlaces, PlacesService, ProjectPlaces, REFERENCES_FOLDER};
pub use plan_limit::{
    PlanLimitChange, PlanLimitService, PlanLimitState, PlanLimitStore, PlanReport, ReachedLimit,
    WorkPermit, may_start_work, reset_moment,
};
pub use ports::{
    ApprovedPlan, AreaRepository, Clock, ConversationRepository, CoordinatorRepository,
    DigestRepository, DiskEntry, EarlierConversations, FieldRepository, FileSystem, FolderReader,
    FoundConversation, MemberRepository, MemoryRepository, PermissionRepository, PlacesRepository,
    PlanningRepository, ProjectFolders, ProjectRepository, SprintRepository, StorageError,
    TagRepository, TaskRepository, TeamworkRepository, ViewRepository, WorkListener,
};
pub use project_events::{
    Agent, Happening, ProjectEvent, StatusOfEvent, TaskOfEvent, WaitsFor,
    event_of_completed_sprint, event_of_created_task, event_of_ended_turn, event_of_moved_task,
    event_of_permission_request, event_of_started_sprint,
};
pub use project_report::markdown::{report_file_name, report_in_markdown};
pub use project_report::{
    DueOutcome, PlanAgainstActual, ProjectReport, ProjectReportPorts, ProjectReportService,
    ReportBrief, ReportDecision, ReportHandoff, ReportTask, ReportUsage, SprintOutcome, UsageLine,
    WorkShare,
};
pub use projects::{NewProject, ProjectService, StatusProperties, WorkflowTemplate};
pub use sprints::{
    NamedTask, NewSprint, PlannedLoad, SprintChanges, SprintDay, SprintForecast, SprintReport,
    SprintService, SprintStart, WorkAhead,
};
pub use sync::{
    Acceptance, CLOCK_AHEAD_LIMIT_MILLIS, ComputerCard, ExchangedChange, ExchangedOperation,
    FileContent, FoundComputer, KnownComputer, NoticeKind, RESTART_NEEDED, RoundReport,
    SYNC_FORMAT, SyncFolder, SyncLedger, SyncNotice, SyncPhase, SyncPorts, SyncService,
    SyncSettings, SyncState, TakeInReport,
};
pub use tags::TagService;
pub use tasks::{
    HistoryEntry, MOST_PROJECT_CHANGES, NewTask, ProjectChange, TaskActual, TaskService,
};
pub use team::{NewMember, TeamService};
pub use teamwork::{
    TeamTalk, TeamworkService, handoffs_in_words, latest_handoff_of, messages_in_words,
    messages_waiting_for,
};
pub use update_install::{
    InstallFailure, InstallPhase, InstallService, InstallState, UpdatePackages, UpdateProblem,
};
pub use updates::{
    LOOK_EVERY, ReleaseSource, UpdateChoice, UpdateChoiceStore, UpdateService, UpdateStatus,
};
pub use usage::{AgentUsage, MemberUsage, ProjectUsage, usage_of};
pub use views::{NewTaskDemands, NewView, ViewService};
pub use work::{
    AgentProgram, AgentProgramState, ApprovedMember, ApprovedOwner, ApprovedProposal, ApprovedTask,
    ConversationWithTurns, FoldersOfTurn, PermissionReply, ProposalEdit, WorkPorts, WorkService,
    answers_in_words, brief_for, brief_with_memory, digests_for_advisor, instructions_for,
    instructions_for_advisor,
};
pub use workbook::{
    Cell, Column, Section, Sheet, SheetContent, Table, Workbook, WorkbookPorts, WorkbookService,
};
