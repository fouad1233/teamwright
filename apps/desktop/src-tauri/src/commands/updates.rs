//! Commands for noticing that a newer version of the program is out
//! ([decision 0043](../../../../../docs/decisions/0043-a-notice-of-newer-versions.md)).
//!
//! Looking is a call out to the public list of releases, made on a thread
//! of its own so that the window never waits for it. A look that gets no
//! answer is not an error: the status says so.

use tauri::State;
use teamwright_application::{ApplicationError, UpdateService, UpdateStatus};

use crate::dto::UpdateStatusDto;
use crate::error::CommandError;
use crate::state::AppState;

/// What a look returns.
type Look = fn(&UpdateService) -> Result<UpdateStatus, ApplicationError>;

/// Runs a look on a thread that may wait for the network.
async fn on_a_thread(updates: UpdateService, look: Look) -> Result<UpdateStatusDto, CommandError> {
    let status = tauri::async_runtime::spawn_blocking(move || look(&updates))
        .await
        .map_err(|_| CommandError::not_available("the look for a newer version was cut short"))??;
    Ok(status.into())
}

/// What is known about versions. Nothing is looked up.
#[tauri::command]
pub async fn update_status(state: State<'_, AppState>) -> Result<UpdateStatusDto, CommandError> {
    Ok(state.updates.status()?.into())
}

/// Looks for a newer version if the manager switched looking on and the
/// last look is a day old, and tells what is known either way.
#[tauri::command]
pub async fn look_for_update_if_due(
    state: State<'_, AppState>,
) -> Result<UpdateStatusDto, CommandError> {
    on_a_thread(state.updates.clone(), UpdateService::look_if_due).await
}

/// Looks for a newer version now, because the manager pressed the button.
#[tauri::command]
pub async fn look_for_update_now(
    state: State<'_, AppState>,
) -> Result<UpdateStatusDto, CommandError> {
    on_a_thread(state.updates.clone(), UpdateService::look_now).await
}

/// Switches looking by itself on or off.
#[tauri::command]
pub async fn switch_update_look(
    state: State<'_, AppState>,
    on: bool,
) -> Result<UpdateStatusDto, CommandError> {
    Ok(state.updates.switch(on)?.into())
}
