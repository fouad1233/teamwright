//! Commands for the permission requests of the specialists (decision
//! 0038).

use tauri::State;

use crate::dto::{ConversationDto, PermissionAnswerInput, PermissionRequestDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Every permission request of a project, answered or not, the oldest
/// first. The interface finds the standing permissions among them.
#[tauri::command(async)]
pub fn permission_requests(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<PermissionRequestDto>, CommandError> {
    let requests = state
        .permissions
        .requests_of_project(parse_id(&project_id)?)?;
    Ok(requests
        .iter()
        .map(|request| PermissionRequestDto::of(request, state.this_computer))
        .collect())
}

/// The requests that wait for the manager, in every project that is not
/// put away, the oldest first.
#[tauri::command(async)]
pub fn open_permission_requests(
    state: State<'_, AppState>,
) -> Result<Vec<PermissionRequestDto>, CommandError> {
    let requests = state.permissions.open_requests()?;
    Ok(requests
        .iter()
        .map(|request| PermissionRequestDto::of(request, state.this_computer))
        .collect())
}

/// Answers a request. The specialist goes on in the same session with the
/// answer, and with the rule if it was allowed.
#[tauri::command(async)]
pub fn answer_permission(
    state: State<'_, AppState>,
    request_id: String,
    answer: PermissionAnswerInput,
) -> Result<ConversationDto, CommandError> {
    let conversation = state
        .work
        .answer_permission(parse_id(&request_id)?, answer.into_reply()?)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Takes back a permission that was allowed for a task or for always.
#[tauri::command(async)]
pub fn withdraw_permission(
    state: State<'_, AppState>,
    request_id: String,
) -> Result<PermissionRequestDto, CommandError> {
    let request = state.permissions.withdraw(parse_id(&request_id)?)?;
    Ok(PermissionRequestDto::of(&request, state.this_computer))
}
