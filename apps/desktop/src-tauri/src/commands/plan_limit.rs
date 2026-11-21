//! The command of the limit of the Claude plan
//! ([decision 0040](../../../../../docs/decisions/0040-the-limit-of-the-plan.md)).

use tauri::State;

use crate::dto::PlanLimitDto;
use crate::error::CommandError;
use crate::state::AppState;

/// What this computer knows about the limits of its Claude plan: whether
/// the limit is reached and when it resets, and the windows as Claude Code
/// last reported them.
#[tauri::command]
pub async fn plan_limit(state: State<'_, AppState>) -> Result<PlanLimitDto, CommandError> {
    Ok(PlanLimitDto::from(&state.plan_limit.standing()?))
}
