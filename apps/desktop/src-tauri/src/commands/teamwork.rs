//! Commands for what the specialists hand each other: the handoffs of
//! tasks and the messages between teammates.

use tauri::State;

use crate::dto::{TeamTalkDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Every handoff and every message of a project, the oldest first.
#[tauri::command]
pub async fn team_talk(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<TeamTalkDto, CommandError> {
    let talk = state.teamwork.talk_of_project(parse_id(&project_id)?)?;
    Ok(TeamTalkDto::from(&talk))
}
