//! The data that travels between the interface and the commands.
//!
//! These types are the contract between the Rust side and the TypeScript side
//! of the application. `src/shared/api/types.ts` mirrors them, and the two
//! must be changed together. Field names are camelCase on the wire.

use std::str::FromStr;

use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use teamwright_application::{NewMember, NewProject, NewTask, WorkflowTemplate};
use teamwright_domain::{
    Assignee, CalendarDate, DomainError, Member, MemberKind, Priority, Project, Status, Task,
    Timestamp,
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
