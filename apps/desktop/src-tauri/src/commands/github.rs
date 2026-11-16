//! Commands for GitHub, through the manager's own `gh`: what a project's
//! repository holds about its tasks, and an issue opened for a task.

use tauri::State;

use crate::dto::{
    GitHubItemDto, GitHubOverviewDto, GitHubSignInDto, IssueDraftDto, TaskOnGitHubDto, parse_id,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Whether `gh` is there and signed in.
#[tauri::command]
pub async fn github_sign_in(state: State<'_, AppState>) -> Result<GitHubSignInDto, CommandError> {
    Ok(GitHubSignInDto::from(&state.github.sign_in()?))
}

/// Names the `gh` of this computer, or, without one, looks for it again.
#[tauri::command]
pub async fn set_github_program(
    state: State<'_, AppState>,
    program: Option<String>,
) -> Result<GitHubSignInDto, CommandError> {
    state.integrations.set_github_program(program.as_deref())?;
    Ok(GitHubSignInDto::from(&state.github.sign_in()?))
}

/// GitHub for a project, from what was last read. Nothing is read.
#[tauri::command]
pub async fn github_overview(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<GitHubOverviewDto, CommandError> {
    Ok(GitHubOverviewDto::from(
        &state.github.overview(parse_id(&project_id)?)?,
    ))
}

/// Reads the pull requests and issues of a project's repository again.
#[tauri::command]
pub async fn refresh_github(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<GitHubOverviewDto, CommandError> {
    Ok(GitHubOverviewDto::from(
        &state.github.refresh(parse_id(&project_id)?)?,
    ))
}

/// The pull requests and issues about a task. With `refresh`, the
/// repository is read again first.
#[tauri::command]
pub async fn task_on_github(
    state: State<'_, AppState>,
    task_id: String,
    refresh: bool,
) -> Result<TaskOnGitHubDto, CommandError> {
    let task_id = parse_id(&task_id)?;
    if refresh {
        let task = state.tasks.get(task_id)?;
        state.github.refresh(task.project_id())?;
    }
    Ok(TaskOnGitHubDto::from(&state.github.of_task(task_id)?))
}

/// What an issue opened for a task would say, and where it would go. Nothing
/// is sent.
#[tauri::command]
pub async fn issue_draft(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<IssueDraftDto, CommandError> {
    let task_id = parse_id(&task_id)?;
    let on_github = state.github.of_task(task_id)?;
    let repository = on_github
        .repository
        .ok_or(teamwright_application::ApplicationError::NoRepository)?;
    let (title, body) = state.github.issue_for_task(task_id)?;
    Ok(IssueDraftDto {
        repository: repository.to_string(),
        title,
        body,
    })
}

/// Opens an issue for a task, after the manager confirmed the draft.
#[tauri::command]
pub async fn open_issue(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<GitHubItemDto, CommandError> {
    Ok(GitHubItemDto::from(
        &state.github.open_issue(parse_id(&task_id)?)?,
    ))
}
