//! Commands for noticing that a newer version of the program is out
//! ([decision 0043](../../../../../docs/decisions/0043-a-notice-of-newer-versions.md)),
//! and for installing it when the manager asks
//! ([decision 0045](../../../../../docs/decisions/0045-installing-updates-from-inside.md)).
//!
//! Looking is a call out to the public list of releases, made on a thread
//! of its own so that the window never waits for it. A look that gets no
//! answer is not an error: the status says so.

use tauri::State;
use teamwright_application::{ApplicationError, UpdateService, UpdateStatus};

use crate::dto::UpdateStatusDto;
use crate::dto::installing::UpdateInstallDto;
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

/// Where updates come from, as the manager reads it.
fn source() -> String {
    format!("github.com/{}", teamwright_integrations::REPOSITORY)
}

/// Where installing an update stands, with how many specialists work.
fn install_state(state: &AppState) -> UpdateInstallDto {
    UpdateInstallDto::new(
        state.installing.state(),
        source(),
        state.work.turns_running_here(),
    )
}

/// Where installing an update stands
/// ([decision 0045](../../../../../docs/decisions/0045-installing-updates-from-inside.md)).
/// Nothing is asked of the network. The window asks this while an install
/// runs, to show its progress.
#[tauri::command]
pub async fn update_install(state: State<'_, AppState>) -> Result<UpdateInstallDto, CommandError> {
    Ok(install_state(&state))
}

/// Installs the newer version, because the manager pressed the button: asks
/// the address of updates, downloads, has the signature checked and puts
/// the version in place, on a thread of its own. Answers at once; the
/// window follows the progress with [`update_install`]. It never restarts
/// the program.
#[tauri::command]
pub async fn install_update(state: State<'_, AppState>) -> Result<UpdateInstallDto, CommandError> {
    let installing = state.installing.clone();
    if installing.begin() {
        // A thread of the operating system, not of the runtime of Tauri:
        // the updater waits on that runtime itself.
        std::thread::spawn(move || {
            let _ = installing.carry_on();
        });
    }
    Ok(install_state(&state))
}

/// Restarts the program into the new version, because the manager pressed
/// "Restart now". While specialists work it refuses with words, unless the
/// manager said yes to stopping them.
#[tauri::command]
pub async fn restart_for_update(
    state: State<'_, AppState>,
    even_while_working: bool,
) -> Result<UpdateInstallDto, CommandError> {
    let turns_running = state.work.turns_running_here();
    let installed = state
        .installing
        .restart(turns_running, even_while_working)?;
    Ok(UpdateInstallDto::new(installed, source(), turns_running))
}
