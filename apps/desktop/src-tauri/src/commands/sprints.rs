//! Commands for sprints, the backlog and story points.

use tauri::State;

use crate::dto::{
    NewSprintInput, SprintChangesInput, SprintCompletionInput, SprintDto, SprintReportDto,
    SprintStartInput, TaskDto, parse_id,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the sprints of a project: the active one, the planned ones, the
/// completed ones.
#[tauri::command]
pub async fn list_sprints(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<SprintDto>, CommandError> {
    let sprints = state.sprints.list_for_project(parse_id(&project_id)?)?;
    Ok(sprints.iter().map(SprintDto::from).collect())
}

/// Plans a new sprint.
#[tauri::command]
pub async fn create_sprint(
    state: State<'_, AppState>,
    input: NewSprintInput,
) -> Result<SprintDto, CommandError> {
    let sprint = state.sprints.create(input.try_into()?)?;
    Ok(SprintDto::from(&sprint))
}

/// Replaces the name, the goal and the dates of a planned or active sprint.
#[tauri::command]
pub async fn change_sprint(
    state: State<'_, AppState>,
    sprint_id: String,
    changes: SprintChangesInput,
) -> Result<SprintDto, CommandError> {
    let sprint = state
        .sprints
        .change(parse_id(&sprint_id)?, changes.try_into()?)?;
    Ok(SprintDto::from(&sprint))
}

/// Removes a planned sprint. Its tasks go back to the backlog.
#[tauri::command]
pub async fn delete_sprint(
    state: State<'_, AppState>,
    sprint_id: String,
) -> Result<(), CommandError> {
    state.sprints.delete(parse_id(&sprint_id)?)?;
    Ok(())
}

/// Starts a planned sprint with its name, goal and dates.
#[tauri::command]
pub async fn start_sprint(
    state: State<'_, AppState>,
    sprint_id: String,
    input: SprintStartInput,
) -> Result<SprintDto, CommandError> {
    let sprint = state
        .sprints
        .start(parse_id(&sprint_id)?, input.try_into()?)?;
    Ok(SprintDto::from(&sprint))
}

/// Completes the active sprint, sending its unfinished tasks to the
/// backlog or to another sprint.
#[tauri::command]
pub async fn complete_sprint(
    state: State<'_, AppState>,
    sprint_id: String,
    input: SprintCompletionInput,
) -> Result<SprintDto, CommandError> {
    let move_unfinished_to = input
        .move_unfinished_to
        .as_deref()
        .map(parse_id)
        .transpose()?;
    let sprint = state
        .sprints
        .complete(parse_id(&sprint_id)?, move_unfinished_to)?;
    Ok(SprintDto::from(&sprint))
}

/// Puts a task with its subtasks into a sprint, or back into the backlog.
#[tauri::command]
pub async fn plan_task(
    state: State<'_, AppState>,
    task_id: String,
    sprint_id: Option<String>,
) -> Result<TaskDto, CommandError> {
    let sprint_id = sprint_id.as_deref().map(parse_id).transpose()?;
    let task = state.tasks.plan(parse_id(&task_id)?, sprint_id)?;
    Ok(TaskDto::from(&task))
}

/// Gives a task story points, or takes them away. The points arrive as any
/// number; the use case refuses one that is not whole or out of range.
#[tauri::command]
pub async fn set_task_points(
    state: State<'_, AppState>,
    task_id: String,
    points: Option<f64>,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.set_points(parse_id(&task_id)?, points)?;
    Ok(TaskDto::from(&task))
}

/// Tells how much work remained in a sprint at the end of each of its days.
#[tauri::command]
pub async fn sprint_report(
    state: State<'_, AppState>,
    sprint_id: String,
) -> Result<SprintReportDto, CommandError> {
    let report = state.sprints.report(parse_id(&sprint_id)?)?;
    Ok(SprintReportDto::from(&report))
}
