//! Commands for projects.

use tauri::State;

use crate::dto::{NewProjectInput, ProjectDto, parse_id};
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
