//! Commands for syncing through Google Drive: the folder that Google Drive
//! for desktop keeps on this computer, found and offered in one press.
//!
//! Looking reads the disk, and a drive of the network can be slow to
//! answer, so the commands are declared `async`: they run on a thread of
//! their own, never on the main thread, which draws the window.

use std::path::Path;

use tauri::State;

use crate::dto::GoogleDriveDto;
use crate::error::CommandError;
use crate::state::AppState;

/// What there is of Google Drive on this computer, and whether syncing runs
/// through it.
#[tauri::command]
pub async fn google_drive(state: State<'_, AppState>) -> Result<GoogleDriveDto, CommandError> {
    let sync = state.sync.state()?;
    let drive = state.google_drive.look(sync.folder.as_deref());
    Ok(GoogleDriveDto::from(&drive))
}

/// Makes the folder `Teamwright` inside a folder of Drive that was found,
/// and answers with it: the folder to sync through, which the manager then
/// chooses as any other.
#[tauri::command]
pub async fn prepare_google_drive_folder(
    state: State<'_, AppState>,
    folder: String,
) -> Result<String, CommandError> {
    let ours = state.google_drive.prepare(Path::new(&folder))?;
    Ok(ours.to_string_lossy().into_owned())
}
