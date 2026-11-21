//! The data that travels between the interface and the commands.
//!
//! These types are the contract between the Rust side and the TypeScript side
//! of the application. `src/shared/api/types.ts` mirrors them, and the two
//! must be changed together. Field names are camelCase on the wire.

use std::path::Path;
use std::str::FromStr;

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use teamwright_application::{
    Acceptance, AgentProgram, AgentUsage, ApprovedMember, ApprovedOwner, ApprovedProposal,
    ApprovedTask, ConversationWithTurns, Deliveries, Delivery, DriveFinding, DriveFolder,
    GitHubItem, GitHubOverview, GitHubSignIn, GoogleDrive, ItemKind, ItemState, KnownComputer,
    NewMember, NewProject, NewSprint, NewTask, NewView, NotificationChoice, NotificationKind,
    NotificationPermission, ProjectChange, ProjectIntegrations, ProjectPlaces, ProjectUsage,
    SprintChanges, SprintReport, SprintStart, StatusProperties, SyncNotice, SyncState,
    TaskOnGitHub, WorkflowTemplate, sign_in_problem,
};
use teamwright_domain::{
    Area, Assignee, CalendarDate, Color, ComputerId, ContextUse, Conversation, DecisionOption,
    DecisionRequest, DomainError, Effort, EventKind, EventKinds, Interview, InterviewAnswer,
    Member, MemberKind, Priority, Project, Proposal, ProposedOwner, Schedule, Sprint,
    SprintSummary, Status, StatusCategory, Step, StepContent, StoryPoints, Task, TaskChange,
    TaskChangeKind, Timestamp, TokenCounts, Turn, View, ViewAssignee, ViewFilter, ViewKind,
    context_of,
};

use crate::error::CommandError;

/// The form calendar dates have on the wire.
const DATE_FORMAT: &str = "%Y-%m-%d";

/// A project as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    id: String,
    key: String,
    name: String,
    description: String,
    statuses: Vec<StatusDto>,
    created_at: String,
    archived_at: Option<String>,
}

impl From<&Project> for ProjectDto {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id().to_string(),
            key: project.key().to_string(),
            name: project.name().to_string(),
            description: project.description().to_owned(),
            statuses: project
                .workflow()
                .statuses()
                .iter()
                .map(StatusDto::from)
                .collect(),
            created_at: moment_to_text(project.created_at()),
            archived_at: project.archived_at().map(moment_to_text),
        }
    }
}

/// A status of a workflow as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    id: String,
    name: String,
    category: &'static str,
    color: &'static str,
}

impl From<&Status> for StatusDto {
    fn from(status: &Status) -> Self {
        Self {
            id: status.id().to_string(),
            name: status.name().to_string(),
            category: status.category().as_str(),
            color: status.color().as_str(),
        }
    }
}

/// A task as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    id: String,
    project_id: String,
    area_id: String,
    number: u32,
    title: String,
    description: String,
    status_id: String,
    priority: &'static str,
    assignee: Option<AssigneeDto>,
    parent_id: Option<String>,
    blocked_by: Vec<String>,
    estimate_minutes: Option<u32>,
    sprint_id: Option<String>,
    points: Option<u16>,
    start_on: Option<String>,
    due_on: Option<String>,
    rank: f64,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
}

impl From<&Task> for TaskDto {
    fn from(task: &Task) -> Self {
        Self {
            id: task.id().to_string(),
            project_id: task.project_id().to_string(),
            area_id: task.area_id().to_string(),
            number: task.number(),
            title: task.title().to_string(),
            description: task.description().to_owned(),
            status_id: task.status_id().to_string(),
            priority: task.priority().as_str(),
            assignee: task.assignee().map(AssigneeDto::from),
            parent_id: task.parent_id().map(|parent_id| parent_id.to_string()),
            blocked_by: task.blocked_by().iter().map(ToString::to_string).collect(),
            estimate_minutes: task.estimate_minutes(),
            sprint_id: task.sprint_id().map(|sprint_id| sprint_id.to_string()),
            points: task.points().map(StoryPoints::value),
            start_on: task.schedule().start_on().map(date_to_text),
            due_on: task.schedule().due_on().map(date_to_text),
            rank: task.rank().value(),
            created_at: moment_to_text(task.created_at()),
            updated_at: moment_to_text(task.updated_at()),
            completed_at: task.completed_at().map(moment_to_text),
        }
    }
}

/// A sprint as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintDto {
    id: String,
    project_id: String,
    name: String,
    goal: String,
    start_on: Option<String>,
    end_on: Option<String>,
    state: &'static str,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    summary: Option<SprintSummaryDto>,
}

impl From<&Sprint> for SprintDto {
    fn from(sprint: &Sprint) -> Self {
        Self {
            id: sprint.id().to_string(),
            project_id: sprint.project_id().to_string(),
            name: sprint.name().to_string(),
            goal: sprint.goal().to_owned(),
            start_on: sprint.dates().start_on().map(date_to_text),
            end_on: sprint.dates().end_on().map(date_to_text),
            state: sprint.state().as_str(),
            created_at: moment_to_text(sprint.created_at()),
            started_at: sprint.started_at().map(moment_to_text),
            completed_at: sprint.completed_at().map(moment_to_text),
            summary: sprint.summary().map(SprintSummaryDto::from),
        }
    }
}

/// What a completed sprint took on and what it finished, as the interface
/// sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintSummaryDto {
    unit: &'static str,
    committed: u32,
    completed: u32,
}

impl From<SprintSummary> for SprintSummaryDto {
    fn from(summary: SprintSummary) -> Self {
        Self {
            unit: summary.unit.as_str(),
            committed: summary.committed,
            completed: summary.completed,
        }
    }
}

/// The course of a sprint, for its burndown chart, as the interface sees
/// it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintReportDto {
    sprint_id: String,
    unit: &'static str,
    total: u32,
    days: Vec<SprintDayDto>,
}

/// One day of a sprint in its report.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintDayDto {
    date: String,
    remaining: Option<u32>,
}

impl From<&SprintReport> for SprintReportDto {
    fn from(report: &SprintReport) -> Self {
        Self {
            sprint_id: report.sprint_id.to_string(),
            unit: report.unit.as_str(),
            total: report.total,
            days: report
                .days
                .iter()
                .map(|day| SprintDayDto {
                    date: date_to_text(day.date),
                    remaining: day.remaining,
                })
                .collect(),
        }
    }
}

/// A view of a project as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewDto {
    id: String,
    project_id: String,
    kind: &'static str,
    name: String,
    filter: ViewFilterDto,
    /// What the view keeps of its own, as the interface wrote it; `null`
    /// for a view that has none.
    content: Option<String>,
}

impl From<&View> for ViewDto {
    fn from(view: &View) -> Self {
        Self {
            id: view.id().to_string(),
            project_id: view.project_id().to_string(),
            kind: view.kind().as_str(),
            name: view.name().to_string(),
            filter: ViewFilterDto::from(view.filter()),
            content: view.content().map(str::to_owned),
        }
    }
}

/// An area of a project as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaDto {
    id: String,
    project_id: String,
    name: String,
}

impl From<&Area> for AreaDto {
    fn from(area: &Area) -> Self {
        Self {
            id: area.id().to_string(),
            project_id: area.project_id().to_string(),
            name: area.name().to_string(),
        }
    }
}

/// Which tasks a view shows:
/// `{"assignee": {"kind": "anyone"}, "hidesFinished": false}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewFilterDto {
    assignee: ViewAssigneeDto,
    hides_finished: bool,
}

impl From<ViewFilter> for ViewFilterDto {
    fn from(filter: ViewFilter) -> Self {
        Self {
            assignee: ViewAssigneeDto::from(filter.assignee),
            hides_finished: filter.hides_finished,
        }
    }
}

impl TryFrom<ViewFilterDto> for ViewFilter {
    type Error = CommandError;

    fn try_from(filter: ViewFilterDto) -> Result<Self, Self::Error> {
        Ok(Self {
            assignee: ViewAssignee::try_from(filter.assignee)?,
            hides_finished: filter.hides_finished,
        })
    }
}

/// Whose tasks a view shows: `{"kind": "anyone"}`, `{"kind": "manager"}`,
/// `{"kind": "member", "memberId": "…"}` or `{"kind": "nobody"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ViewAssigneeDto {
    /// Everybody's tasks.
    Anyone,
    /// The tasks given to the manager.
    Manager,
    /// The tasks given to one member of the team.
    #[serde(rename_all = "camelCase")]
    Member {
        /// The identifier of the team member.
        member_id: String,
    },
    /// The tasks given to nobody.
    Nobody,
}

impl From<ViewAssignee> for ViewAssigneeDto {
    fn from(assignee: ViewAssignee) -> Self {
        match assignee {
            ViewAssignee::Anyone => Self::Anyone,
            ViewAssignee::Manager => Self::Manager,
            ViewAssignee::Member(member_id) => Self::Member {
                member_id: member_id.to_string(),
            },
            ViewAssignee::Nobody => Self::Nobody,
        }
    }
}

impl TryFrom<ViewAssigneeDto> for ViewAssignee {
    type Error = CommandError;

    fn try_from(assignee: ViewAssigneeDto) -> Result<Self, Self::Error> {
        Ok(match assignee {
            ViewAssigneeDto::Anyone => Self::Anyone,
            ViewAssigneeDto::Manager => Self::Manager,
            ViewAssigneeDto::Member { member_id } => Self::Member(parse_id(&member_id)?),
            ViewAssigneeDto::Nobody => Self::Nobody,
        })
    }
}

/// One change in the life of a task, as the interface sees it:
/// `{"at": "…", "kind": "moved", "fromStatusId": "…", "toStatusId": "…"}`,
/// and so on for the other kinds.
#[derive(Debug, Serialize)]
pub struct TaskChangeDto {
    at: String,
    /// The name of the computer that made the change, when it was another
    /// computer whose name is known; left out otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    computer: Option<String>,
    #[serde(flatten)]
    change: TaskChangeKindDto,
}

impl TaskChangeDto {
    /// Names the computer that made the change.
    pub fn made_on(mut self, computer: Option<String>) -> Self {
        self.computer = computer;
        self
    }
}

/// What happened in a change, with its kind as the field `kind`.
#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TaskChangeKindDto {
    /// The task was opened.
    Created,
    /// It got another title.
    Renamed {
        /// The title before.
        from: String,
        /// The title since.
        to: String,
    },
    /// Its description changed.
    Described,
    /// It went to another status.
    Moved {
        /// The status before.
        from_status_id: String,
        /// The status since.
        to_status_id: String,
    },
    /// It was given to someone else, or to nobody.
    Assigned {
        /// Who was responsible before.
        from: Option<AssigneeDto>,
        /// Who is responsible since.
        to: Option<AssigneeDto>,
    },
    /// Its priority changed.
    Prioritized {
        /// The priority before.
        from: &'static str,
        /// The priority since.
        to: &'static str,
    },
    /// Its estimate changed. The numbers are minutes.
    Estimated {
        /// The estimate before.
        from: Option<u32>,
        /// The estimate since.
        to: Option<u32>,
    },
    /// Its dates changed.
    Rescheduled {
        /// The dates before.
        from: TaskDatesDto,
        /// The dates since.
        to: TaskDatesDto,
    },
    /// It began to wait for another task.
    NowWaitsFor {
        /// The task it waits for.
        task_id: String,
    },
    /// It stopped waiting for another task.
    NoLongerWaitsFor {
        /// The task it waited for.
        task_id: String,
    },
}

/// The dates of a task: when it starts and when it is due.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDatesDto {
    start_on: Option<String>,
    due_on: Option<String>,
}

impl From<Schedule> for TaskDatesDto {
    fn from(schedule: Schedule) -> Self {
        Self {
            start_on: schedule.start_on().map(date_to_text),
            due_on: schedule.due_on().map(date_to_text),
        }
    }
}

impl From<&TaskChange> for TaskChangeDto {
    fn from(change: &TaskChange) -> Self {
        let kind = match &change.kind {
            TaskChangeKind::Created => TaskChangeKindDto::Created,
            TaskChangeKind::Renamed { from, to } => TaskChangeKindDto::Renamed {
                from: from.to_string(),
                to: to.to_string(),
            },
            TaskChangeKind::Described => TaskChangeKindDto::Described,
            TaskChangeKind::Moved { from, to } => TaskChangeKindDto::Moved {
                from_status_id: from.to_string(),
                to_status_id: to.to_string(),
            },
            TaskChangeKind::Assigned { from, to } => TaskChangeKindDto::Assigned {
                from: from.map(AssigneeDto::from),
                to: to.map(AssigneeDto::from),
            },
            TaskChangeKind::Prioritized { from, to } => TaskChangeKindDto::Prioritized {
                from: from.as_str(),
                to: to.as_str(),
            },
            TaskChangeKind::Estimated { from, to } => TaskChangeKindDto::Estimated {
                from: *from,
                to: *to,
            },
            TaskChangeKind::Rescheduled { from, to } => TaskChangeKindDto::Rescheduled {
                from: TaskDatesDto::from(*from),
                to: TaskDatesDto::from(*to),
            },
            TaskChangeKind::NowWaitsFor { blocker } => TaskChangeKindDto::NowWaitsFor {
                task_id: blocker.to_string(),
            },
            TaskChangeKind::NoLongerWaitsFor { blocker } => TaskChangeKindDto::NoLongerWaitsFor {
                task_id: blocker.to_string(),
            },
        };
        Self {
            at: moment_to_text(change.at),
            computer: None,
            change: kind,
        }
    }
}

/// One change in the history of a project: a change of a task, as
/// [`TaskChangeDto`] has it, with the task it is about under `task`.
#[derive(Debug, Serialize)]
pub struct ProjectChangeDto {
    #[serde(flatten)]
    change: TaskChangeDto,
    task: HistoryTaskDto,
}

/// The task a change in the history of a project is about. A task deleted
/// since keeps its number and title and is `gone`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTaskDto {
    id: String,
    number: u32,
    title: String,
    gone: bool,
}

impl From<&ProjectChange> for ProjectChangeDto {
    fn from(change: &ProjectChange) -> Self {
        Self {
            change: TaskChangeDto::from(&change.change),
            task: HistoryTaskDto {
                id: change.task_id.to_string(),
                number: change.task_number,
                title: change.task_title.to_string(),
                gone: change.task_gone,
            },
        }
    }
}

/// Who is responsible for a task: `{"kind": "manager"}` or
/// `{"kind": "member", "memberId": "…"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssigneeDto {
    /// The manager who owns the project.
    Manager,
    /// A member of the project's team: an AI agent or a human teammate.
    #[serde(rename_all = "camelCase")]
    Member {
        /// The identifier of the team member.
        member_id: String,
    },
}

impl From<Assignee> for AssigneeDto {
    fn from(assignee: Assignee) -> Self {
        match assignee {
            Assignee::Manager => Self::Manager,
            Assignee::Member(member_id) => Self::Member {
                member_id: member_id.to_string(),
            },
        }
    }
}

impl TryFrom<AssigneeDto> for Assignee {
    type Error = CommandError;

    fn try_from(assignee: AssigneeDto) -> Result<Self, Self::Error> {
        match assignee {
            AssigneeDto::Manager => Ok(Self::Manager),
            AssigneeDto::Member { member_id } => Ok(Self::Member(parse_id(&member_id)?)),
        }
    }
}

/// A member of a team as the interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberDto {
    id: String,
    project_id: String,
    name: String,
    kind: &'static str,
    responsibility: String,
    model: Option<String>,
    effort: Option<&'static str>,
    /// The instructions of an AI specialist; empty if it has none, and
    /// always empty for a person.
    instructions: String,
    color: &'static str,
    joined_at: String,
    updated_at: String,
}

impl From<&Member> for MemberDto {
    fn from(member: &Member) -> Self {
        Self {
            id: member.id().to_string(),
            project_id: member.project_id().to_string(),
            name: member.name().to_string(),
            kind: member.kind().as_str(),
            responsibility: member.responsibility().to_owned(),
            model: member.model().map(ToString::to_string),
            effort: member.effort().map(|effort| effort.as_str()),
            instructions: member.instructions().as_str().to_owned(),
            color: member.color().as_str(),
            joined_at: moment_to_text(member.joined_at()),
            updated_at: moment_to_text(member.updated_at()),
        }
    }
}

/// A ready-made specialist as the interface sees it: a role that ships with
/// Teamwright and can be put on a team in one step.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyMadeSpecialistDto {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    instructions: &'static str,
    model: &'static str,
    effort: &'static str,
}

impl From<&teamwright_domain::ReadyMadeSpecialist> for ReadyMadeSpecialistDto {
    fn from(specialist: &teamwright_domain::ReadyMadeSpecialist) -> Self {
        Self {
            key: specialist.key,
            name: specialist.name,
            description: specialist.description,
            instructions: specialist.instructions,
            model: specialist.model,
            effort: specialist.effort.as_str(),
        }
    }
}

/// Whether the program that runs the specialists can be used, as the
/// interface sees it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProgramDto {
    state: &'static str,
    version: Option<String>,
    sign_in_method: Option<String>,
    subscription: Option<String>,
    problem: Option<String>,
}

impl From<AgentProgram> for AgentProgramDto {
    fn from(program: AgentProgram) -> Self {
        Self {
            state: program.state.as_str(),
            version: program.version.map(|version| version.to_string()),
            sign_in_method: program.sign_in_method,
            subscription: program.subscription,
            problem: program.problem,
        }
    }
}

/// The folders of a project as the interface sees them. Paths travel as
/// text.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPlacesDto {
    work_folder: String,
    is_work_folder_chosen: bool,
    reference_folders: Vec<String>,
}

impl From<ProjectPlaces> for ProjectPlacesDto {
    fn from(places: ProjectPlaces) -> Self {
        Self {
            work_folder: path_to_text(&places.work_folder),
            is_work_folder_chosen: places.is_work_folder_chosen,
            reference_folders: places
                .reference_folders
                .iter()
                .map(|folder| path_to_text(folder))
                .collect(),
        }
    }
}

/// Writes a path as text for the interface. The paths come from the
/// interface as text, so nothing is lost.
fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// What lies in a folder of a project: `{"entries": [...], "hasMore": false}`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderListingDto {
    entries: Vec<FileEntryDto>,
    has_more: bool,
}

/// One entry of a folder. `kind` is `folder`, `file` or `link` (a link that
/// leads outside the folder or nowhere, not followed); `size` is in bytes,
/// for a file only.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    name: String,
    kind: &'static str,
    size: Option<u64>,
    changed_at: Option<String>,
}

impl From<teamwright_application::FolderListing> for FolderListingDto {
    fn from(listing: teamwright_application::FolderListing) -> Self {
        Self {
            entries: listing
                .entries
                .into_iter()
                .map(|entry| FileEntryDto {
                    kind: match entry.kind {
                        teamwright_domain::EntryKind::Folder => "folder",
                        teamwright_domain::EntryKind::File => "file",
                        teamwright_domain::EntryKind::Link => "link",
                    },
                    name: entry.name,
                    size: entry.size,
                    changed_at: entry.changed.map(moment_to_text),
                })
                .collect(),
            has_more: listing.has_more,
        }
    }
}

/// A conversation with a specialist and all of its turns, as the interface
/// sees it. It is also the payload of the event `conversation-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDto {
    id: String,
    project_id: String,
    /// Whether the conversation is with the Advisor, which has no member
    /// and no task.
    is_with_advisor: bool,
    member_id: Option<String>,
    task_id: Option<String>,
    started_at: String,
    /// The identity of the computer the conversation runs on.
    home_computer: String,
    /// Whether that is this computer. Only there can the manager reply,
    /// answer, approve or stop.
    is_here: bool,
    turns: Vec<TurnDto>,
    /// How full the context of the conversation's session is: what its
    /// last turn that reported one says.
    context: Option<ContextDto>,
}

impl ConversationDto {
    /// Takes a conversation and its turns, the oldest first, as seen from
    /// the computer `this_computer`.
    pub fn new(conversation: &Conversation, turns: &[Turn], this_computer: ComputerId) -> Self {
        Self {
            id: conversation.id().to_string(),
            project_id: conversation.project_id().to_string(),
            is_with_advisor: conversation.is_with_advisor(),
            member_id: conversation.member_id().map(|id| id.to_string()),
            task_id: conversation.task_id().map(|id| id.to_string()),
            started_at: moment_to_text(conversation.started_at()),
            home_computer: conversation.home_computer().to_string(),
            is_here: conversation.is_at_home_on(this_computer),
            turns: turns.iter().map(TurnDto::from).collect(),
            context: context_of(turns).map(ContextDto::from),
        }
    }

    /// Takes a conversation with its turns, as seen from the computer
    /// `this_computer`.
    pub fn of(conversation: &ConversationWithTurns, this_computer: ComputerId) -> Self {
        Self::new(
            &conversation.conversation,
            &conversation.turns,
            this_computer,
        )
    }
}

/// One request to a specialist and what came of it, as the interface sees
/// it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnDto {
    id: String,
    request: String,
    origin: &'static str,
    state: &'static str,
    started_at: String,
    ended_at: Option<String>,
    steps: Vec<StepDto>,
    answer: String,
    failure: Option<String>,
    model: Option<String>,
    tokens: Option<TokenCountsDto>,
    context: Option<ContextDto>,
    decision: Option<DecisionRequestDto>,
    interview: Option<InterviewDto>,
    proposal: Option<ProposalDto>,
}

impl From<&Turn> for TurnDto {
    fn from(turn: &Turn) -> Self {
        Self {
            id: turn.id().to_string(),
            request: turn.request().to_owned(),
            origin: turn.origin().as_str(),
            state: turn.state().as_str(),
            started_at: moment_to_text(turn.started_at()),
            ended_at: turn.ended_at().map(moment_to_text),
            steps: turn.steps().iter().map(StepDto::from).collect(),
            answer: turn.answer().to_owned(),
            failure: turn.failure().map(str::to_owned),
            model: turn.model().map(str::to_owned),
            tokens: turn.tokens().map(TokenCountsDto::from),
            context: turn.context().map(ContextDto::from),
            decision: turn.decision().map(DecisionRequestDto::from),
            interview: turn.interview().map(InterviewDto::from),
            proposal: turn.proposal().map(ProposalDto::from),
        }
    }
}

/// The questions the Advisor asked, as the interface sees them. `answers`
/// and `answeredAt` are null while the interview is open.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterviewDto {
    questions: Vec<InterviewQuestionDto>,
    answers: Option<Vec<InterviewAnswerDto>>,
    answered_at: Option<String>,
}

/// One question of an interview.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterviewQuestionDto {
    question: String,
    reason: String,
    /// Two to four options, or none for a question answered in words.
    options: Vec<DecisionOptionDto>,
    /// The recommended option, by its place counted from 0.
    recommended: Option<usize>,
}

/// The manager's answer to one question of an interview.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterviewAnswerDto {
    option: Option<usize>,
    note: String,
}

impl From<&Interview> for InterviewDto {
    fn from(interview: &Interview) -> Self {
        Self {
            questions: interview
                .questions()
                .iter()
                .map(|question| InterviewQuestionDto {
                    question: question.question.clone(),
                    reason: question.reason.clone(),
                    options: question
                        .options
                        .iter()
                        .map(DecisionOptionDto::from)
                        .collect(),
                    recommended: question.recommended,
                })
                .collect(),
            answers: interview.answers().map(|answers| {
                answers
                    .iter()
                    .map(|answer| InterviewAnswerDto {
                        option: answer.option,
                        note: answer.note.clone(),
                    })
                    .collect()
            }),
            answered_at: interview.answered_at().map(moment_to_text),
        }
    }
}

/// What the Advisor proposed, as the interface sees it. `approvedAt` is
/// null while it is not approved.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDto {
    summary: String,
    members: Vec<ProposedMemberDto>,
    tasks: Vec<ProposedTaskDto>,
    approved_at: Option<String>,
}

/// A member of a proposal.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedMemberDto {
    name: String,
    responsibility: String,
    reason: String,
    model: Option<String>,
    effort: Option<&'static str>,
}

/// Who a proposed task is given to: `{"kind": "manager"}` or
/// `{"kind": "member", "name": "…"}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProposedOwnerDto {
    /// The manager.
    Manager,
    /// A member by name.
    Member {
        /// The name of the member.
        name: String,
    },
}

/// A task of a proposal.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedTaskDto {
    title: String,
    description: String,
    owner: Option<ProposedOwnerDto>,
    estimate_minutes: Option<u32>,
    priority: &'static str,
    /// The tasks of the same proposal it waits for, by their places from 0.
    waits_for: Vec<usize>,
}

impl From<&Proposal> for ProposalDto {
    fn from(proposal: &Proposal) -> Self {
        Self {
            summary: proposal.summary().to_owned(),
            members: proposal
                .members()
                .iter()
                .map(|member| ProposedMemberDto {
                    name: member.name.to_string(),
                    responsibility: member.responsibility.clone(),
                    reason: member.reason.clone(),
                    model: member.model.as_ref().map(ToString::to_string),
                    effort: member.effort.map(|effort| effort.as_str()),
                })
                .collect(),
            tasks: proposal
                .tasks()
                .iter()
                .map(|task| ProposedTaskDto {
                    title: task.title.to_string(),
                    description: task.description.clone(),
                    owner: task.owner.as_ref().map(|owner| match owner {
                        ProposedOwner::Manager => ProposedOwnerDto::Manager,
                        ProposedOwner::Member(name) => ProposedOwnerDto::Member {
                            name: name.to_string(),
                        },
                    }),
                    estimate_minutes: task.estimate_minutes,
                    priority: task.priority.as_str(),
                    waits_for: task.waits_for.clone(),
                })
                .collect(),
            approved_at: proposal.approved_at().map(moment_to_text),
        }
    }
}

/// What the interface sends to start planning with the Advisor.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAdviceInput {
    /// What the manager wants, in their own words.
    pub brief: String,
    /// The model of the Advisor, or none for the default of the program.
    #[serde(default)]
    pub model: Option<String>,
    /// The effort of the Advisor, in its text form, or none.
    #[serde(default)]
    pub effort: Option<String>,
}

/// What the interface sends to answer one question of an interview.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterviewAnswerInput {
    /// The chosen option, by its place from 0, or none.
    #[serde(default)]
    option: Option<f64>,
    /// Words of the manager's own. Needed if no option is chosen.
    #[serde(default)]
    note: String,
}

impl From<InterviewAnswerInput> for InterviewAnswer {
    fn from(input: InterviewAnswerInput) -> Self {
        Self {
            option: input.option.map(place_from_number),
            note: input.note,
        }
    }
}

/// What the interface sends to approve a proposal: the members and the
/// tasks the manager left in it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovedProposalInput {
    members: Vec<ProposedMemberInput>,
    tasks: Vec<ProposedTaskInput>,
}

/// A member of a proposal as the manager approves it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedMemberInput {
    name: String,
    #[serde(default)]
    responsibility: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
}

/// A task of a proposal as the manager approves it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedTaskInput {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    owner: Option<ProposedOwnerDto>,
    #[serde(default)]
    estimate_minutes: Option<u32>,
    priority: String,
    /// Places of other tasks of the proposal, from 0. A number that is not
    /// a place, such as -1 or 1.5, is read as a place that is not there,
    /// which the use case refuses with the rule it breaks.
    #[serde(default)]
    waits_for: Vec<f64>,
}

impl TryFrom<ApprovedProposalInput> for ApprovedProposal {
    type Error = CommandError;

    fn try_from(input: ApprovedProposalInput) -> Result<Self, Self::Error> {
        Ok(Self {
            members: input
                .members
                .into_iter()
                .map(|member| {
                    Ok(ApprovedMember {
                        name: member.name,
                        responsibility: member.responsibility,
                        reason: member.reason,
                        model: member.model,
                        effort: member.effort.as_deref().map(parse_effort).transpose()?,
                    })
                })
                .collect::<Result<_, CommandError>>()?,
            tasks: input
                .tasks
                .into_iter()
                .map(|task| {
                    Ok(ApprovedTask {
                        title: task.title,
                        description: task.description,
                        owner: task.owner.map(|owner| match owner {
                            ProposedOwnerDto::Manager => ApprovedOwner::Manager,
                            ProposedOwnerDto::Member { name } => ApprovedOwner::Member(name),
                        }),
                        estimate_minutes: task.estimate_minutes,
                        priority: parse_priority(&task.priority)?,
                        waits_for: task.waits_for.into_iter().map(place_from_number).collect(),
                    })
                })
                .collect::<Result<_, CommandError>>()?,
        })
    }
}

/// A place in a list that the interface sent as a number. A number that is
/// not a whole number from 0 becomes a place no list has, so that the use
/// case refuses it as a place that is not there.
fn place_from_number(number: f64) -> usize {
    let is_place = number.trunc().to_bits() == number.to_bits()
        && (0.0..=f64::from(u32::MAX)).contains(&number);
    if is_place {
        // A whole number from 0 to the largest u32 fits without loss.
        usize::try_from(number as u32).unwrap_or(usize::MAX)
    } else {
        usize::MAX
    }
}

/// One change the manager makes to an open proposal, as the interface
/// sends it: `{"kind": "retitle", "task": 3, "title": "…"}` and so on.
/// Tasks and members are named by their places from 0.
#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProposalEditInput {
    /// Gives a task another title.
    Retitle { task: f64, title: String },
    /// Gives a task another description.
    Describe { task: f64, description: String },
    /// Gives a task another size in minutes, or none.
    Estimate {
        task: f64,
        estimate_minutes: Option<u32>,
    },
    /// Gives a task another priority.
    Prioritize { task: f64, priority: String },
    /// Gives a task to somebody else, or to nobody.
    GiveTo {
        task: f64,
        owner: Option<ProposedOwnerDto>,
    },
    /// Lets a task wait for another task of the proposal.
    AddWait { task: f64, waits_for: f64 },
    /// Lets a task stop waiting for another task of the proposal.
    RemoveWait { task: f64, waits_for: f64 },
    /// Adds a task of the manager's own at the end of the plan.
    AddTask { title: String },
    /// Takes a task out of the proposal.
    RemoveTask { task: f64 },
    /// Takes a member out of the proposal.
    RemoveMember { member: f64 },
    /// Chooses the model and the effort of a member; null for the default.
    SetProfile {
        member: f64,
        model: Option<String>,
        effort: Option<String>,
    },
}

impl TryFrom<ProposalEditInput> for teamwright_application::ProposalEdit {
    type Error = CommandError;

    fn try_from(input: ProposalEditInput) -> Result<Self, Self::Error> {
        use teamwright_application::ProposalEdit as Edit;
        let place = place_from_number;
        Ok(match input {
            ProposalEditInput::Retitle { task, title } => Edit::Retitle {
                task: place(task),
                title,
            },
            ProposalEditInput::Describe { task, description } => Edit::Describe {
                task: place(task),
                description,
            },
            ProposalEditInput::Estimate {
                task,
                estimate_minutes,
            } => Edit::Estimate {
                task: place(task),
                estimate_minutes,
            },
            ProposalEditInput::Prioritize { task, priority } => Edit::Prioritize {
                task: place(task),
                priority: parse_priority(&priority)?,
            },
            ProposalEditInput::GiveTo { task, owner } => Edit::GiveTo {
                task: place(task),
                owner: owner.map(|owner| match owner {
                    ProposedOwnerDto::Manager => ApprovedOwner::Manager,
                    ProposedOwnerDto::Member { name } => ApprovedOwner::Member(name),
                }),
            },
            ProposalEditInput::AddWait { task, waits_for } => Edit::AddWait {
                task: place(task),
                waits_for: place(waits_for),
            },
            ProposalEditInput::RemoveWait { task, waits_for } => Edit::RemoveWait {
                task: place(task),
                waits_for: place(waits_for),
            },
            ProposalEditInput::AddTask { title } => Edit::AddTask { title },
            ProposalEditInput::RemoveTask { task } => Edit::RemoveTask { task: place(task) },
            ProposalEditInput::RemoveMember { member } => Edit::RemoveMember {
                member: place(member),
            },
            ProposalEditInput::SetProfile {
                member,
                model,
                effort,
            } => Edit::SetProfile {
                member: place(member),
                model,
                effort: effort.as_deref().map(parse_effort).transpose()?,
            },
        })
    }
}

impl From<&DecisionOption> for DecisionOptionDto {
    fn from(option: &DecisionOption) -> Self {
        Self {
            label: option.label.clone(),
            consequence: option.consequence.clone(),
        }
    }
}

/// A question a specialist asked the manager to decide, as the interface
/// sees it. `answer` is null while the question is open.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRequestDto {
    question: String,
    reason: String,
    options: Vec<DecisionOptionDto>,
    /// The recommended option, by its place counted from 0.
    recommended: Option<usize>,
    answer: Option<DecisionAnswerDto>,
}

/// One of the answers a specialist offers for a decision.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionOptionDto {
    label: String,
    consequence: String,
}

/// What the manager decided.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionAnswerDto {
    /// The chosen option, by its place counted from 0, or null for an
    /// answer in words only.
    option: Option<usize>,
    note: String,
    at: String,
}

impl From<&DecisionRequest> for DecisionRequestDto {
    fn from(decision: &DecisionRequest) -> Self {
        Self {
            question: decision.question().to_owned(),
            reason: decision.reason().to_owned(),
            options: decision
                .options()
                .iter()
                .map(|option| DecisionOptionDto {
                    label: option.label.clone(),
                    consequence: option.consequence.clone(),
                })
                .collect(),
            recommended: decision.recommended(),
            answer: decision.answer().map(|answer| DecisionAnswerDto {
                option: answer.option,
                note: answer.note.clone(),
                at: moment_to_text(answer.at),
            }),
        }
    }
}

/// What the interface sends to answer a specialist's question: an option,
/// counted from 0, or none, and words of the manager's own.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionAnswerInput {
    /// The chosen option, or none.
    #[serde(default)]
    pub option: Option<usize>,
    /// Words of the manager's own. Needed if no option is chosen.
    #[serde(default)]
    pub note: String,
}

/// One step of a turn: `{"at": "…", "kind": "text", "text": "…"}`, and so
/// on for the other kinds.
#[derive(Debug, Clone, Serialize)]
pub struct StepDto {
    at: String,
    #[serde(flatten)]
    content: StepContentDto,
}

/// What happened in a step, with its kind as the field `kind`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StepContentDto {
    /// The specialist wrote something.
    Text {
        /// What it wrote.
        text: String,
    },
    /// The specialist used a tool.
    Tool {
        /// The identifier of the use.
        id: String,
        /// The name of the tool.
        tool: String,
        /// What the tool was used on. May be empty.
        summary: String,
        /// `running`, `done` or `failed`.
        outcome: &'static str,
    },
    /// A tool was refused.
    Denied {
        /// The name of the tool.
        tool: String,
    },
    /// The program said something about the run itself.
    Notice {
        /// What it said.
        message: String,
    },
}

impl From<&Step> for StepDto {
    fn from(step: &Step) -> Self {
        let content = match &step.content {
            StepContent::Text { text } => StepContentDto::Text { text: text.clone() },
            StepContent::Tool {
                use_id,
                tool,
                summary,
                outcome,
            } => StepContentDto::Tool {
                id: use_id.clone(),
                tool: tool.clone(),
                summary: summary.clone(),
                outcome: outcome.as_str(),
            },
            StepContent::Denied { tool } => StepContentDto::Denied { tool: tool.clone() },
            StepContent::Notice { message } => StepContentDto::Notice {
                message: message.clone(),
            },
        };
        Self {
            at: moment_to_text(step.at),
            content,
        }
    }
}

/// Counts of tokens, by kind, as the interface sees them.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCountsDto {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_written: u64,
}

impl From<TokenCounts> for TokenCountsDto {
    fn from(tokens: TokenCounts) -> Self {
        Self {
            input: tokens.input,
            output: tokens.output,
            cache_read: tokens.cache_read,
            cache_written: tokens.cache_written,
        }
    }
}

/// How full the context of a session is, as the interface sees it. Whether
/// it is nearly full is a rule of the domain, carried so that the interface
/// does not work it out again.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDto {
    held: u64,
    window: Option<u64>,
    is_nearly_full: bool,
}

impl From<ContextUse> for ContextDto {
    fn from(context: ContextUse) -> Self {
        Self {
            held: context.held,
            window: context.window,
            is_nearly_full: context.is_nearly_full(),
        }
    }
}

/// What one agent, or a group of them, has spent, as the interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUsageDto {
    tokens: TokenCountsDto,
    /// Every kind of token together.
    all_tokens: u64,
    turns: u32,
    last_model: Option<String>,
    context: Option<ContextDto>,
}

impl From<AgentUsage> for AgentUsageDto {
    fn from(usage: AgentUsage) -> Self {
        Self {
            tokens: TokenCountsDto::from(usage.tokens),
            all_tokens: usage.tokens.all(),
            turns: usage.turns,
            last_model: usage.last_model,
            context: usage.context.map(ContextDto::from),
        }
    }
}

/// What a member of the team has spent, as the interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberUsageDto {
    member_id: String,
    usage: AgentUsageDto,
}

/// What a project has spent, and who spent it, as the interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsageDto {
    /// The one that spent the most first.
    members: Vec<MemberUsageDto>,
    advisor: Option<AgentUsageDto>,
    former: Option<AgentUsageDto>,
    total: AgentUsageDto,
}

impl From<ProjectUsage> for ProjectUsageDto {
    fn from(usage: ProjectUsage) -> Self {
        Self {
            members: usage
                .members
                .into_iter()
                .map(|member| MemberUsageDto {
                    member_id: member.member_id.to_string(),
                    usage: AgentUsageDto::from(member.usage),
                })
                .collect(),
            advisor: usage.advisor.map(AgentUsageDto::from),
            former: usage.former.map(AgentUsageDto::from),
            total: AgentUsageDto::from(usage.total),
        }
    }
}

/// What the interface sends to put someone on a team.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMemberInput {
    project_id: String,
    name: String,
    kind: String,
    #[serde(default)]
    responsibility: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
}

impl TryFrom<NewMemberInput> for NewMember {
    type Error = CommandError;

    fn try_from(input: NewMemberInput) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: parse_id(&input.project_id)?,
            name: input.name,
            kind: input.kind.parse::<MemberKind>()?,
            responsibility: input.responsibility,
            model: input.model,
            effort: input.effort.as_deref().map(parse_effort).transpose()?,
        })
    }
}

/// What the interface sends to say which model an AI specialist runs on and
/// how hard it thinks. Either may be absent, which leaves it to the program.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberProfileInput {
    /// The model, by a name the program understands. Empty means none.
    #[serde(default)]
    pub model: Option<String>,
    /// The effort, in its text form, such as `high`.
    #[serde(default)]
    pub effort: Option<String>,
}

/// What the interface sends to add or change a status: its name, category
/// and color.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPropertiesInput {
    name: String,
    category: String,
    color: String,
}

impl TryFrom<StatusPropertiesInput> for StatusProperties {
    type Error = CommandError;

    fn try_from(input: StatusPropertiesInput) -> Result<Self, Self::Error> {
        Ok(Self {
            name: input.name,
            category: input.category.parse::<StatusCategory>()?,
            color: input.color.parse::<Color>()?,
        })
    }
}

/// The workflow a new project starts with.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkflowTemplateDto {
    /// To do, in progress, in review, done.
    Standard,
    /// To do, in progress, in revision, in test, done.
    Engineering,
}

impl From<WorkflowTemplateDto> for WorkflowTemplate {
    fn from(template: WorkflowTemplateDto) -> Self {
        match template {
            WorkflowTemplateDto::Standard => Self::Standard,
            WorkflowTemplateDto::Engineering => Self::Engineering,
        }
    }
}

/// What the interface sends to start a project.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProjectInput {
    name: String,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    description: String,
    workflow: WorkflowTemplateDto,
}

impl From<NewProjectInput> for NewProject {
    fn from(input: NewProjectInput) -> Self {
        Self {
            name: input.name,
            key: input.key,
            description: input.description,
            workflow: input.workflow.into(),
        }
    }
}

/// What the interface sends to open a task.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTaskInput {
    project_id: String,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    assignee: Option<AssigneeDto>,
    #[serde(default)]
    status_id: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    area_id: Option<String>,
    #[serde(default)]
    estimate_minutes: Option<u32>,
    #[serde(default)]
    start_on: Option<String>,
    #[serde(default)]
    due_on: Option<String>,
}

impl TryFrom<NewTaskInput> for NewTask {
    type Error = CommandError;

    fn try_from(input: NewTaskInput) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: parse_id(&input.project_id)?,
            title: input.title,
            description: input.description,
            priority: input
                .priority
                .as_deref()
                .map(parse_priority)
                .transpose()?
                .unwrap_or_default(),
            assignee: input.assignee.map(Assignee::try_from).transpose()?,
            status_id: input.status_id.as_deref().map(parse_id).transpose()?,
            parent_id: input.parent_id.as_deref().map(parse_id).transpose()?,
            area_id: input.area_id.as_deref().map(parse_id).transpose()?,
            estimate_minutes: input.estimate_minutes,
            start_on: parse_optional_date(input.start_on.as_deref())?,
            due_on: parse_optional_date(input.due_on.as_deref())?,
        })
    }
}

/// What the interface sends to plan a sprint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewSprintInput {
    project_id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    start_on: Option<String>,
    #[serde(default)]
    end_on: Option<String>,
}

impl TryFrom<NewSprintInput> for NewSprint {
    type Error = CommandError;

    fn try_from(input: NewSprintInput) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: parse_id(&input.project_id)?,
            name: input.name.unwrap_or_default(),
            goal: input.goal.unwrap_or_default(),
            start_on: parse_optional_date(input.start_on.as_deref())?,
            end_on: parse_optional_date(input.end_on.as_deref())?,
        })
    }
}

/// What the interface sends to change a sprint. Every field is replaced.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintChangesInput {
    name: String,
    goal: String,
    #[serde(default)]
    start_on: Option<String>,
    #[serde(default)]
    end_on: Option<String>,
}

impl TryFrom<SprintChangesInput> for SprintChanges {
    type Error = CommandError;

    fn try_from(input: SprintChangesInput) -> Result<Self, Self::Error> {
        Ok(Self {
            name: input.name,
            goal: input.goal,
            start_on: parse_optional_date(input.start_on.as_deref())?,
            end_on: parse_optional_date(input.end_on.as_deref())?,
        })
    }
}

/// What the interface sends to start a sprint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintStartInput {
    name: String,
    goal: String,
    start_on: String,
    end_on: String,
}

impl TryFrom<SprintStartInput> for SprintStart {
    type Error = CommandError;

    fn try_from(input: SprintStartInput) -> Result<Self, Self::Error> {
        Ok(Self {
            name: input.name,
            goal: input.goal,
            start_on: parse_date(&input.start_on)?,
            end_on: parse_date(&input.end_on)?,
        })
    }
}

/// What the interface sends to complete a sprint: where its unfinished
/// tasks go, a sprint or nothing for the backlog.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SprintCompletionInput {
    #[serde(default)]
    pub move_unfinished_to: Option<String>,
}

/// What the interface sends to add a view to a project. Without a name, or
/// with an empty one, the view is named after its kind.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewViewInput {
    project_id: String,
    kind: String,
    #[serde(default)]
    name: Option<String>,
}

impl TryFrom<NewViewInput> for NewView {
    type Error = CommandError;

    fn try_from(input: NewViewInput) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: parse_id(&input.project_id)?,
            kind: input.kind.parse::<ViewKind>()?,
            name: input.name,
        })
    }
}

/// Reads an identifier the interface sent.
pub fn parse_id<T>(text: &str) -> Result<T, CommandError>
where
    T: FromStr<Err = DomainError>,
{
    Ok(text.parse::<T>()?)
}

/// Reads a priority the interface sent.
pub fn parse_priority(text: &str) -> Result<Priority, CommandError> {
    Ok(text.parse::<Priority>()?)
}

/// Reads an effort the interface sent. The interface only offers the known
/// ones, so another one is a request that cannot be understood.
pub fn parse_effort(text: &str) -> Result<Effort, CommandError> {
    text.parse::<Effort>()
        .map_err(|error| CommandError::invalid_request(error.to_string()))
}

/// Reads a calendar date the interface sent, if it sent one.
pub fn parse_optional_date(text: Option<&str>) -> Result<Option<CalendarDate>, CommandError> {
    text.map(parse_date).transpose()
}

/// Reads a calendar date the interface sent.
pub fn parse_date(text: &str) -> Result<CalendarDate, CommandError> {
    CalendarDate::parse_from_str(text, DATE_FORMAT).map_err(|_| {
        CommandError::invalid_request(format!("'{text}' is not a date in the form year-month-day"))
    })
}

/// Reads a moment the interface sent, in RFC 3339, as it writes them.
pub fn parse_moment(text: &str) -> Result<Timestamp, CommandError> {
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|moment| moment.with_timezone(&chrono::Utc))
        .map_err(|_| CommandError::invalid_request(format!("'{text}' is not a moment")))
}

/// Writes a moment the way the interface expects it: UTC, RFC 3339, with
/// milliseconds.
fn moment_to_text(moment: Timestamp) -> String {
    moment.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Writes a calendar date the way the interface expects it.
fn date_to_text(date: CalendarDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

/// Where syncing through a folder stands, as the interface sees it. It is
/// also the payload of the event `sync-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStateDto {
    /// The chosen folder, or nothing while syncing is off.
    folder: Option<String>,
    /// The identity of this computer.
    this_computer: String,
    /// The name of this computer, once the manager gave one.
    computer_name: Option<String>,
    /// A name to offer when the manager is asked for one: the name the
    /// operating system gives the computer. Only a suggestion; it often
    /// holds a person's name.
    suggested_name: Option<String>,
    /// `off`, `in_step`, `writing`, `unreachable` or `stopped`.
    phase: &'static str,
    /// When the last round ended, in this run of the program.
    last_round: Option<String>,
    /// What went wrong in the last round, in words.
    problem: Option<String>,
    /// What went wrong, as a code to decide by: `sync_folder_unreachable`,
    /// `identity_in_use`, `restart_needed`, `storage_failed`, or another
    /// code of an error; nothing when nothing went wrong.
    problem_code: Option<&'static str>,
    /// The other computers.
    computers: Vec<ComputerDto>,
}

impl SyncStateDto {
    /// Takes the state of syncing, with a suggestion for the name of this
    /// computer.
    pub fn new(state: &SyncState, suggested_name: Option<String>) -> Self {
        Self {
            folder: state.folder.as_deref().map(path_to_text),
            this_computer: state.this_computer.to_string(),
            computer_name: state.computer_name.clone(),
            suggested_name,
            phase: state.phase.as_str(),
            last_round: state.last_round.map(moment_to_text),
            problem: state.problem.clone(),
            problem_code: state.problem_code,
            computers: state.computers.iter().map(ComputerDto::from).collect(),
        }
    }
}

/// A folder of the manager's Google Drive on this computer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFolderDto {
    path: String,
    /// The address of the account, when the path names it.
    account: Option<String>,
}

impl From<&DriveFolder> for DriveFolderDto {
    fn from(folder: &DriveFolder) -> Self {
        Self {
            path: path_to_text(&folder.path),
            account: folder.account.clone(),
        }
    }
}

/// What there is of Google Drive on this computer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleDriveDto {
    /// `found`, `not_installed` or `not_for_this_system`.
    finding: &'static str,
    /// The folders found; empty unless something was found.
    folders: Vec<DriveFolderDto>,
    /// The folder of Drive that holds the sync folder, when syncing runs
    /// through Drive.
    syncs_through: Option<DriveFolderDto>,
}

impl From<&GoogleDrive> for GoogleDriveDto {
    fn from(drive: &GoogleDrive) -> Self {
        let (finding, folders) = match &drive.finding {
            DriveFinding::Found(folders) => ("found", folders.iter().map(Into::into).collect()),
            DriveFinding::NotInstalled => ("not_installed", Vec::new()),
            DriveFinding::NotForThisSystem => ("not_for_this_system", Vec::new()),
        };
        Self {
            finding,
            folders,
            syncs_through: drive.syncs_through.as_ref().map(Into::into),
        }
    }
}

/// Another computer in the sync folder, as the interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputerDto {
    id: String,
    /// The name it gave itself, once its card was read.
    name: Option<String>,
    /// The version of Teamwright it runs.
    version: Option<String>,
    /// `undecided`, `accepted` or `ignored`.
    acceptance: &'static str,
    /// When it last wrote into the folder.
    last_heard: Option<String>,
    /// Whether its clock is ahead of this computer's by more than five
    /// minutes.
    clock_ahead: bool,
    /// Whether it uses a newer Teamwright, whose files this one cannot
    /// read.
    newer_format: bool,
}

impl From<&KnownComputer> for ComputerDto {
    fn from(computer: &KnownComputer) -> Self {
        Self {
            id: computer.id.to_string(),
            name: computer.name.clone(),
            version: computer.version.clone(),
            acceptance: match computer.acceptance {
                Acceptance::Undecided => "undecided",
                Acceptance::Accepted => "accepted",
                Acceptance::Ignored => "ignored",
            },
            last_heard: computer.last_heard.map(moment_to_text),
            clock_ahead: computer.clock_ahead_by_millis.is_some(),
            newer_format: computer.newer_format.is_some(),
        }
    }
}

/// Something syncing tells the manager, as the interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncNoticeDto {
    id: i64,
    at: String,
    /// `task_renumbered`, `project_rekeyed`, `change_dropped`, `task_moved`
    /// or `clock_ahead`.
    kind: &'static str,
    project_id: Option<String>,
    task_id: Option<String>,
    message: String,
}

impl From<&SyncNotice> for SyncNoticeDto {
    fn from(notice: &SyncNotice) -> Self {
        Self {
            id: notice.id,
            at: moment_to_text(notice.at),
            kind: notice.kind.as_str(),
            project_id: notice.project_id.map(|id| id.to_string()),
            task_id: notice.task_id.map(|id| id.to_string()),
            message: notice.message.clone(),
        }
    }
}

/// The payload of the event `data-changed`: the projects in which syncing
/// changed something.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataChangedDto {
    project_ids: Vec<String>,
}

impl From<&[teamwright_domain::ProjectId]> for DataChangedDto {
    fn from(projects: &[teamwright_domain::ProjectId]) -> Self {
        Self {
            project_ids: projects.iter().map(ToString::to_string).collect(),
        }
    }
}

/// What the manager chose about notifications on this computer, as the
/// interface sees it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationChoiceDto {
    /// `not_asked`, `allowed` or `declined`.
    pub permission: &'static str,
    /// Whether a notification is sent when someone waits for the manager.
    pub waiting: bool,
    /// Whether one is sent when work on a task is finished.
    pub finished: bool,
    /// Whether one is sent when someone could not go on.
    pub stopped: bool,
}

impl From<NotificationChoice> for NotificationChoiceDto {
    fn from(choice: NotificationChoice) -> Self {
        Self {
            permission: match choice.permission {
                NotificationPermission::NotAsked => "not_asked",
                NotificationPermission::Allowed => "allowed",
                NotificationPermission::Declined => "declined",
            },
            waiting: choice.waiting,
            finished: choice.finished,
            stopped: choice.stopped,
        }
    }
}

/// Reads a kind of notification the interface names.
pub fn parse_notification_kind(text: &str) -> Result<NotificationKind, CommandError> {
    match text {
        "waiting" => Ok(NotificationKind::Waiting),
        "finished" => Ok(NotificationKind::Finished),
        "stopped" => Ok(NotificationKind::Stopped),
        other => Err(CommandError::invalid_request(format!(
            "{other} is not a kind of notification"
        ))),
    }
}

/// What a notification the manager pressed was about, for the interface to
/// open. A field the notification did not carry, or that the platform did
/// not report, is nothing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPressedDto {
    /// `waiting`, `finished` or `stopped`.
    pub kind: Option<String>,
    /// The project.
    pub project_id: Option<String>,
    /// The task it was about, if any.
    pub task_id: Option<String>,
    /// The specialist it was about, if any.
    pub member_id: Option<String>,
    /// Whether it was about the Advisor.
    pub is_with_advisor: bool,
}

/// The integrations of a project on this computer, as the interface sees
/// them. An address is a secret: only its host is sent back.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectIntegrationsDto {
    repository: Option<String>,
    slack: Option<SlackChannelDto>,
    web_addresses: Vec<ListeningAddressDto>,
}

/// The Slack channel of a project.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlackChannelDto {
    address: String,
    events: Vec<&'static str>,
}

/// A web address that listens for the events of a project.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningAddressDto {
    address: String,
    is_signed: bool,
    events: Vec<&'static str>,
}

fn event_names(events: &EventKinds) -> Vec<&'static str> {
    events.iter().map(EventKind::as_str).collect()
}

impl From<ProjectIntegrations> for ProjectIntegrationsDto {
    fn from(integrations: ProjectIntegrations) -> Self {
        Self {
            repository: integrations.repository.map(|name| name.to_string()),
            slack: integrations.slack.map(|slack| SlackChannelDto {
                address: slack.address.shown(),
                events: event_names(&slack.events),
            }),
            web_addresses: integrations
                .web_addresses
                .iter()
                .map(|listening| ListeningAddressDto {
                    address: listening.address.shown(),
                    is_signed: listening.secret.is_some(),
                    events: event_names(&listening.events),
                })
                .collect(),
        }
    }
}

/// Reads a kind of event the interface names.
pub fn parse_event_kind(text: &str) -> Result<EventKind, CommandError> {
    text.parse::<EventKind>()
        .map_err(|error| CommandError::invalid_request(error.to_string()))
}

/// Whether the manager's `gh` is there and signed in, with what is missing
/// in words.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSignInDto {
    /// `not_installed`, `not_signed_in` or `signed_in`.
    state: &'static str,
    program: Option<String>,
    account: Option<String>,
    problem: Option<&'static str>,
}

impl From<&GitHubSignIn> for GitHubSignInDto {
    fn from(sign_in: &GitHubSignIn) -> Self {
        let (state, program, account) = match sign_in {
            GitHubSignIn::NotInstalled => ("not_installed", None, None),
            GitHubSignIn::NotSignedIn { program } => ("not_signed_in", Some(program), None),
            GitHubSignIn::SignedIn { program, account } => {
                ("signed_in", Some(program), account.clone())
            }
        };
        Self {
            state,
            program: program.map(|program| path_to_text(program)),
            account,
            problem: sign_in_problem(sign_in),
        }
    }
}

/// A pull request or an issue.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubItemDto {
    /// `pull_request` or `issue`.
    kind: &'static str,
    number: u64,
    title: String,
    /// `open`, `draft`, `merged` or `closed`.
    state: &'static str,
    url: String,
    /// The branch of a pull request; null for an issue.
    branch: Option<String>,
}

impl From<&GitHubItem> for GitHubItemDto {
    fn from(item: &GitHubItem) -> Self {
        Self {
            kind: match item.kind {
                ItemKind::PullRequest => "pull_request",
                ItemKind::Issue => "issue",
            },
            number: item.number,
            title: item.title.clone(),
            state: match item.state {
                ItemState::Open => "open",
                ItemState::Draft => "draft",
                ItemState::Merged => "merged",
                ItemState::Closed => "closed",
            },
            url: item.url.clone(),
            branch: (!item.branch.is_empty()).then(|| item.branch.clone()),
        }
    }
}

/// GitHub for a project, as the integrations page shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubOverviewDto {
    sign_in: GitHubSignInDto,
    repository: Option<String>,
    is_from_work_folder: bool,
    read_at: Option<String>,
    linked_pull_requests: usize,
    linked_issues: usize,
    unlinked: Vec<GitHubItemDto>,
    unlinked_count: usize,
}

impl From<&GitHubOverview> for GitHubOverviewDto {
    fn from(overview: &GitHubOverview) -> Self {
        Self {
            sign_in: GitHubSignInDto::from(&overview.sign_in),
            repository: overview.repository.as_ref().map(ToString::to_string),
            is_from_work_folder: overview.is_from_work_folder,
            read_at: overview.read_at.map(moment_to_text),
            linked_pull_requests: overview.linked_pull_requests,
            linked_issues: overview.linked_issues,
            unlinked: overview.unlinked.iter().map(GitHubItemDto::from).collect(),
            unlinked_count: overview.unlinked_count,
        }
    }
}

/// The pull requests and issues about a task.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOnGitHubDto {
    repository: Option<String>,
    read_at: Option<String>,
    items: Vec<GitHubItemDto>,
}

impl From<&TaskOnGitHub> for TaskOnGitHubDto {
    fn from(on_github: &TaskOnGitHub) -> Self {
        Self {
            repository: on_github.repository.as_ref().map(ToString::to_string),
            read_at: on_github.read_at.map(moment_to_text),
            items: on_github.items.iter().map(GitHubItemDto::from).collect(),
        }
    }
}

/// One delivery to a channel or a web address.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDto {
    at: String,
    /// The name of the event, or `test`.
    event: &'static str,
    /// The status of the answer, if there was one.
    status: Option<u16>,
    succeeded: bool,
    /// Why it was not taken, in words, if it was not.
    failure: Option<String>,
}

impl From<&Delivery> for DeliveryDto {
    fn from(delivery: &Delivery) -> Self {
        Self {
            at: moment_to_text(delivery.at),
            event: delivery.event,
            status: delivery.outcome.as_ref().ok().copied(),
            succeeded: delivery.succeeded(),
            failure: delivery.failure(),
        }
    }
}

/// What was delivered for a project since the program started.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveriesDto {
    channel_failure: Option<DeliveryDto>,
    /// For each web address, in their order, its last deliveries, newest
    /// first.
    addresses: Vec<Vec<DeliveryDto>>,
}

impl From<&Deliveries> for DeliveriesDto {
    fn from(deliveries: &Deliveries) -> Self {
        Self {
            channel_failure: deliveries.channel_failure.as_ref().map(DeliveryDto::from),
            addresses: deliveries
                .addresses
                .iter()
                .map(|kept| kept.iter().map(DeliveryDto::from).collect())
                .collect(),
        }
    }
}

/// What an issue opened for a task would say, and where it would go.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueDraftDto {
    /// The repository, `owner/name`.
    pub repository: String,
    /// The title, with the task's reference first.
    pub title: String,
    /// The body.
    pub body: String,
}

/// Everything the specialists of a project handed each other, as the
/// interface sees it: the handoffs of its tasks and the messages between
/// its members, each the oldest first.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTalkDto {
    handoffs: Vec<HandoffDto>,
    messages: Vec<TeamMessageDto>,
}

/// The result of a task, handed over by the specialist who did it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffDto {
    id: String,
    task_id: String,
    /// The specialist who published it. It may have left the team since.
    author_id: String,
    conversation_id: String,
    turn_id: String,
    summary: String,
    /// The interface contract. Empty if there is none.
    contract: String,
    /// Paths inside the work folder of the project, with `/` between
    /// their parts.
    files: Vec<String>,
    published_at: String,
}

/// A question or a note from one specialist to another.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMessageDto {
    id: String,
    from_id: String,
    to_id: String,
    conversation_id: String,
    turn_id: String,
    /// "question" or "note".
    kind: &'static str,
    text: String,
    sent_at: String,
    /// The turn of the receiver it came with, and when; both null while it
    /// waits.
    delivered_turn_id: Option<String>,
    delivered_at: Option<String>,
}

impl From<&teamwright_application::TeamTalk> for TeamTalkDto {
    fn from(talk: &teamwright_application::TeamTalk) -> Self {
        Self {
            handoffs: talk.handoffs.iter().map(HandoffDto::from).collect(),
            messages: talk.messages.iter().map(TeamMessageDto::from).collect(),
        }
    }
}

impl From<&teamwright_domain::Handoff> for HandoffDto {
    fn from(handoff: &teamwright_domain::Handoff) -> Self {
        let content = handoff.content();
        Self {
            id: handoff.id().to_string(),
            task_id: handoff.task_id().to_string(),
            author_id: handoff.author_id().to_string(),
            conversation_id: handoff.conversation_id().to_string(),
            turn_id: handoff.turn_id().to_string(),
            summary: content.summary().to_owned(),
            contract: content.contract().to_owned(),
            files: content
                .files()
                .iter()
                .map(teamwright_domain::PathInFolder::as_text)
                .collect(),
            published_at: moment_to_text(handoff.published_at()),
        }
    }
}

impl From<&teamwright_domain::TeamMessage> for TeamMessageDto {
    fn from(message: &teamwright_domain::TeamMessage) -> Self {
        let said_in = message.said_in();
        let delivery = message.delivery();
        Self {
            id: message.id().to_string(),
            from_id: said_in.member_id.to_string(),
            to_id: message.to().to_string(),
            conversation_id: said_in.conversation_id.to_string(),
            turn_id: said_in.turn_id.to_string(),
            kind: message.kind().as_str(),
            text: message.text().to_owned(),
            sent_at: moment_to_text(message.sent_at()),
            delivered_turn_id: delivery.map(|delivery| delivery.turn_id.to_string()),
            delivered_at: delivery.map(|delivery| moment_to_text(delivery.at)),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use chrono::TimeZone;
    use teamwright_domain::ComputerId;

    use super::*;

    #[test]
    fn the_state_of_syncing_goes_out_in_camel_case_with_its_phase_in_words() {
        let desk = ComputerId::generate();
        let state = SyncState {
            folder: Some(std::path::PathBuf::from("sync")),
            this_computer: ComputerId::generate(),
            computer_name: Some("Laptop".to_owned()),
            computers: vec![KnownComputer {
                id: desk,
                name: Some("Desk".to_owned()),
                version: Some("0.1.0".to_owned()),
                acceptance: Acceptance::Undecided,
                read_up_to: 0,
                last_heard: Some(chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap()),
                clock_ahead_by_millis: None,
                newer_format: Some(2),
            }],
            phase: teamwright_application::SyncPhase::InStep,
            last_round: None,
            problem: None,
            problem_code: None,
        };

        let json =
            serde_json::to_value(SyncStateDto::new(&state, Some("SAMPLE".to_owned()))).unwrap();

        assert_eq!(json["phase"], "in_step");
        assert_eq!(json["problemCode"], serde_json::Value::Null);
        assert_eq!(json["computerName"], "Laptop");
        assert_eq!(json["suggestedName"], "SAMPLE");
        assert_eq!(json["computers"][0]["acceptance"], "undecided");
        assert_eq!(json["computers"][0]["newerFormat"], true);
        assert_eq!(
            json["computers"][0]["lastHeard"],
            "2026-11-12T09:00:00.000Z"
        );
    }

    #[test]
    fn a_change_names_the_other_computer_that_made_it_and_none_made_here() {
        let change = TaskChange {
            at: chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
            kind: TaskChangeKind::Created,
        };

        let elsewhere =
            serde_json::to_value(TaskChangeDto::from(&change).made_on(Some("Laptop".to_owned())))
                .unwrap();
        let here = serde_json::to_value(TaskChangeDto::from(&change)).unwrap();

        assert_eq!(elsewhere["computer"], "Laptop");
        assert_eq!(elsewhere["kind"], "created");
        assert!(here.get("computer").is_none());
    }

    #[test]
    fn a_change_of_a_project_names_its_task_beside_the_change() {
        let task_id = teamwright_domain::TaskId::generate();
        let change = ProjectChange {
            change: TaskChange {
                at: chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
                kind: TaskChangeKind::Created,
            },
            task_id,
            task_number: 7,
            task_title: teamwright_domain::TaskTitle::parse("Route the board").unwrap(),
            task_gone: true,
        };

        let sent = serde_json::to_value(ProjectChangeDto::from(&change)).unwrap();

        assert_eq!(sent["at"], "2026-11-12T09:00:00.000Z");
        assert_eq!(sent["kind"], "created");
        assert_eq!(sent["task"]["id"], task_id.to_string());
        assert_eq!(sent["task"]["number"], 7);
        assert_eq!(sent["task"]["title"], "Route the board");
        assert_eq!(sent["task"]["gone"], true);
    }

    #[test]
    fn a_moment_is_read_as_the_interface_writes_it() {
        let moment = parse_moment("2026-11-12T09:00:00.250Z").unwrap();

        assert_eq!(moment.timestamp_millis() % 1000, 250);
        assert!(parse_moment("yesterday").is_err());
    }

    #[test]
    fn a_change_of_a_proposal_is_read_as_the_interface_writes_it() {
        use teamwright_application::ProposalEdit;

        let read = |json: serde_json::Value| {
            ProposalEdit::try_from(serde_json::from_value::<ProposalEditInput>(json).unwrap())
        };

        assert_eq!(
            read(serde_json::json!({ "kind": "retitle", "task": 2, "title": "Route it" })).unwrap(),
            ProposalEdit::Retitle {
                task: 2,
                title: "Route it".to_owned()
            }
        );
        assert_eq!(
            read(serde_json::json!({ "kind": "estimate", "task": 0, "estimateMinutes": null }))
                .unwrap(),
            ProposalEdit::Estimate {
                task: 0,
                estimate_minutes: None
            }
        );
        assert_eq!(
            read(serde_json::json!({ "kind": "addWait", "task": 3, "waitsFor": 1 })).unwrap(),
            ProposalEdit::AddWait {
                task: 3,
                waits_for: 1
            }
        );
        assert_eq!(
            read(serde_json::json!({
                "kind": "giveTo", "task": 1, "owner": { "kind": "member", "name": "Ada" }
            }))
            .unwrap(),
            ProposalEdit::GiveTo {
                task: 1,
                owner: Some(ApprovedOwner::Member("Ada".to_owned()))
            }
        );
        assert_eq!(
            read(serde_json::json!({
                "kind": "setProfile", "member": 0, "model": null, "effort": "low"
            }))
            .unwrap(),
            ProposalEdit::SetProfile {
                member: 0,
                model: None,
                effort: Some(Effort::Low)
            }
        );
        // A place that is not a place is read as one no proposal has.
        assert_eq!(
            read(serde_json::json!({ "kind": "removeTask", "task": -1 })).unwrap(),
            ProposalEdit::RemoveTask { task: usize::MAX }
        );
        assert!(
            read(serde_json::json!({ "kind": "prioritize", "task": 0, "priority": "huge" }))
                .is_err()
        );
    }
}
