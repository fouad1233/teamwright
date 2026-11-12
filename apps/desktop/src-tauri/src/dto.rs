//! The data that travels between the interface and the commands.
//!
//! These types are the contract between the Rust side and the TypeScript side
//! of the application. `src/shared/api/types.ts` mirrors them, and the two
//! must be changed together. Field names are camelCase on the wire.

use std::str::FromStr;

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use teamwright_application::{
    AgentProgram, ConversationWithTurns, NewMember, NewProject, NewTask, StatusProperties,
    WorkflowTemplate,
};
use teamwright_domain::{
    Assignee, CalendarDate, Color, Conversation, DomainError, Member, MemberKind, Priority,
    Project, Schedule, Status, StatusCategory, Step, StepContent, Task, TaskChange, TaskChangeKind,
    Timestamp, TokenCounts, Turn,
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
    number: u32,
    title: String,
    description: String,
    status_id: String,
    priority: &'static str,
    assignee: Option<AssigneeDto>,
    parent_id: Option<String>,
    blocked_by: Vec<String>,
    estimate_minutes: Option<u32>,
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
            number: task.number(),
            title: task.title().to_string(),
            description: task.description().to_owned(),
            status_id: task.status_id().to_string(),
            priority: task.priority().as_str(),
            assignee: task.assignee().map(AssigneeDto::from),
            parent_id: task.parent_id().map(|parent_id| parent_id.to_string()),
            blocked_by: task.blocked_by().iter().map(ToString::to_string).collect(),
            estimate_minutes: task.estimate_minutes(),
            start_on: task.schedule().start_on().map(date_to_text),
            due_on: task.schedule().due_on().map(date_to_text),
            rank: task.rank().value(),
            created_at: moment_to_text(task.created_at()),
            updated_at: moment_to_text(task.updated_at()),
            completed_at: task.completed_at().map(moment_to_text),
        }
    }
}

/// One change in the life of a task, as the interface sees it:
/// `{"at": "…", "kind": "moved", "fromStatusId": "…", "toStatusId": "…"}`,
/// and so on for the other kinds.
#[derive(Debug, Serialize)]
pub struct TaskChangeDto {
    at: String,
    #[serde(flatten)]
    change: TaskChangeKindDto,
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
            change: kind,
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
            color: member.color().as_str(),
            joined_at: moment_to_text(member.joined_at()),
            updated_at: moment_to_text(member.updated_at()),
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

/// A conversation with a specialist and all of its turns, as the interface
/// sees it. It is also the payload of the event `conversation-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationDto {
    id: String,
    project_id: String,
    member_id: Option<String>,
    task_id: Option<String>,
    started_at: String,
    turns: Vec<TurnDto>,
}

impl ConversationDto {
    /// Takes a conversation and its turns, the oldest first.
    pub fn new(conversation: &Conversation, turns: &[Turn]) -> Self {
        Self {
            id: conversation.id().to_string(),
            project_id: conversation.project_id().to_string(),
            member_id: conversation.member_id().map(|id| id.to_string()),
            task_id: conversation.task_id().map(|id| id.to_string()),
            started_at: moment_to_text(conversation.started_at()),
            turns: turns.iter().map(TurnDto::from).collect(),
        }
    }
}

impl From<&ConversationWithTurns> for ConversationDto {
    fn from(conversation: &ConversationWithTurns) -> Self {
        Self::new(&conversation.conversation, &conversation.turns)
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
        }
    }
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

/// What the interface sends to put someone on a team.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMemberInput {
    project_id: String,
    name: String,
    kind: String,
    #[serde(default)]
    responsibility: String,
}

impl TryFrom<NewMemberInput> for NewMember {
    type Error = CommandError;

    fn try_from(input: NewMemberInput) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: parse_id(&input.project_id)?,
            name: input.name,
            kind: input.kind.parse::<MemberKind>()?,
            responsibility: input.responsibility,
        })
    }
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
            estimate_minutes: input.estimate_minutes,
            start_on: parse_optional_date(input.start_on.as_deref())?,
            due_on: parse_optional_date(input.due_on.as_deref())?,
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

/// Reads a calendar date the interface sent, if it sent one.
pub fn parse_optional_date(text: Option<&str>) -> Result<Option<CalendarDate>, CommandError> {
    text.map(|text| {
        CalendarDate::parse_from_str(text, DATE_FORMAT).map_err(|_| {
            CommandError::invalid_request(format!(
                "'{text}' is not a date in the form year-month-day"
            ))
        })
    })
    .transpose()
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
