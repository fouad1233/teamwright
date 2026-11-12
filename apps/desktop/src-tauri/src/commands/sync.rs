//! Commands for keeping this computer in step with the manager's other
//! computers, through a folder the manager owns.
//!
//! A round reads and writes files and can take seconds, so the commands
//! that run one are declared `async`: they then run on a thread of their
//! own, never on the main thread, which draws the window.

use std::path::Path;

use tauri::{AppHandle, State};

use crate::dto::{SyncNoticeDto, SyncStateDto, parse_id};
use crate::error::CommandError;
use crate::state::{self, AppState};

/// Where syncing stands, with the other computers.
#[tauri::command]
pub async fn sync_state(state: State<'_, AppState>) -> Result<SyncStateDto, CommandError> {
    state::sync_state_dto(&state)
}

/// Chooses the folder to keep computers in step through, and names this
/// computer. The first round writes the whole log of this computer into
/// it, and starts at once.
#[tauri::command]
pub async fn choose_sync_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    folder: String,
    computer_name: String,
) -> Result<SyncStateDto, CommandError> {
    state
        .sync
        .choose_folder(Path::new(&folder), &computer_name)?;
    // The folder is chosen whatever the first round finds; what went wrong
    // is in the state that goes back.
    let _ = state::run_round(&app, &state);
    state::sync_state_dto(&state)
}

/// Stops syncing. Nothing in the folder is removed.
#[tauri::command]
pub async fn stop_syncing(state: State<'_, AppState>) -> Result<SyncStateDto, CommandError> {
    state.sync.stop()?;
    state::sync_state_dto(&state)
}

/// Runs a round now, instead of waiting for the next one.
#[tauri::command]
pub async fn sync_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<SyncStateDto, CommandError> {
    state::run_round(&app, &state)?;
    state::sync_state_dto(&state)
}

/// Gives this computer another name, which the other computers see after
/// the next round.
#[tauri::command]
pub async fn rename_this_computer(
    state: State<'_, AppState>,
    name: String,
) -> Result<SyncStateDto, CommandError> {
    state.sync.rename_this_computer(&name)?;
    state::sync_state_dto(&state)
}

/// Takes in the operations of another computer from now on.
#[tauri::command]
pub async fn accept_computer(
    app: AppHandle,
    state: State<'_, AppState>,
    computer_id: String,
) -> Result<SyncStateDto, CommandError> {
    state.sync.accept(parse_id(&computer_id)?)?;
    // What went wrong in the round is in the state that goes back.
    let _ = state::run_round(&app, &state);
    state::sync_state_dto(&state)
}

/// Takes in nothing more of another computer.
#[tauri::command]
pub async fn ignore_computer(
    state: State<'_, AppState>,
    computer_id: String,
) -> Result<SyncStateDto, CommandError> {
    state.sync.ignore(parse_id(&computer_id)?)?;
    state::sync_state_dto(&state)
}

/// The notices syncing keeps for the manager, the newest first.
#[tauri::command]
pub async fn sync_notices(state: State<'_, AppState>) -> Result<Vec<SyncNoticeDto>, CommandError> {
    let notices = state.sync.notices()?;
    Ok(notices.iter().map(SyncNoticeDto::from).collect())
}

/// Forgets a notice.
#[tauri::command]
pub async fn dismiss_sync_notice(
    state: State<'_, AppState>,
    notice_id: i64,
) -> Result<(), CommandError> {
    state.sync.dismiss(notice_id)?;
    Ok(())
}

/// Gives this computer a new identity, for when its data folder was copied
/// to another computer. Syncing waits until Teamwright starts again.
#[tauri::command]
pub async fn give_this_computer_a_new_identity(
    state: State<'_, AppState>,
) -> Result<SyncStateDto, CommandError> {
    state.sync.new_identity()?;
    state::sync_state_dto(&state)
}
