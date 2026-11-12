//! Commands for projects.

use tauri::State;

use crate::dto::{NewProjectInput, ProjectDto, StatusPropertiesInput, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the projects that are not archived, oldest first.
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectDto>, CommandError> {
    let projects = state.projects.list()?;
    Ok(projects.iter().map(ProjectDto::from).collect())
}

/// Starts a project.
#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    input: NewProjectInput,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.create(input.into())?;
    Ok(ProjectDto::from(&project))
}

/// Gives a project a new name.
#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.rename(parse_id(&project_id)?, &name)?;
    Ok(ProjectDto::from(&project))
}

/// Replaces the description of a project.
#[tauri::command]
pub async fn describe_project(
    state: State<'_, AppState>,
    project_id: String,
    description: String,
) -> Result<ProjectDto, CommandError> {
    let project = state
        .projects
        .describe(parse_id(&project_id)?, &description)?;
    Ok(ProjectDto::from(&project))
}

/// Puts a project away.
#[tauri::command]
pub async fn archive_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.archive(parse_id(&project_id)?)?;
    Ok(ProjectDto::from(&project))
}

/// Lists the projects that were put away, the one put away last first.
#[tauri::command]
pub async fn list_archived_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectDto>, CommandError> {
    let projects = state.projects.list_archived()?;
    Ok(projects.iter().map(ProjectDto::from).collect())
}

/// Brings an archived project back.
#[tauri::command]
pub async fn restore_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.restore(parse_id(&project_id)?)?;
    Ok(ProjectDto::from(&project))
}

/// Removes a project for good, with its tasks and its team.
#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), CommandError> {
    state.projects.delete(parse_id(&project_id)?)?;
    Ok(())
}

/// Adds a status to the workflow of a project.
#[tauri::command]
pub async fn add_status(
    state: State<'_, AppState>,
    project_id: String,
    status: StatusPropertiesInput,
) -> Result<ProjectDto, CommandError> {
    let project = state
        .projects
        .add_status(parse_id(&project_id)?, status.try_into()?)?;
    Ok(ProjectDto::from(&project))
}

/// Gives a status a new name, category and color.
#[tauri::command]
pub async fn change_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    status: StatusPropertiesInput,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.change_status(
        parse_id(&project_id)?,
        parse_id(&status_id)?,
        status.try_into()?,
    )?;
    Ok(ProjectDto::from(&project))
}

/// Moves a status to another place in the workflow.
#[tauri::command]
pub async fn move_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    index: usize,
) -> Result<ProjectDto, CommandError> {
    let project =
        state
            .projects
            .move_status(parse_id(&project_id)?, parse_id(&status_id)?, index)?;
    Ok(ProjectDto::from(&project))
}

/// Removes a status from the workflow. The tasks in it go to the status
/// named by `move_tasks_to`, which is needed only if there are any.
#[tauri::command]
pub async fn remove_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    move_tasks_to: Option<String>,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.remove_status(
        parse_id(&project_id)?,
        parse_id(&status_id)?,
        move_tasks_to.as_deref().map(parse_id).transpose()?,
    )?;
    Ok(ProjectDto::from(&project))
}
