//! Commands for the project as an Excel workbook
//! ([decision 0025](../../../../../docs/decisions/0025-the-project-as-a-workbook.md)):
//! writing it to a file the manager chose, and showing that file in its
//! folder.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::PoisonError;
use std::thread;

use tauri::State;
use teamwright_workbook::write_workbook;

use crate::dto::parse_id;
use crate::error::CommandError;
use crate::state::{AppState, suggested_computer_name};

/// Writes a project as an Excel workbook to the file the manager chose in
/// the save window, and answers with the path of the file. Gathering and
/// writing can take a moment, so it runs on a thread of its own.
#[tauri::command(async)]
pub fn export_workbook(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<String, CommandError> {
    let project_id = parse_id(&project_id)?;
    let project = state.projects.get(project_id)?;
    // The name the manager gave this computer for syncing, or else the one
    // the operating system gives it.
    let computer = state
        .sync
        .state()?
        .computer_name
        .filter(|name| !name.trim().is_empty())
        .or_else(suggested_computer_name)
        .unwrap_or_default();
    let workbook = state.workbook.workbook_of(project_id, &computer)?;
    let path = PathBuf::from(path);
    write_workbook(&workbook, project.name().as_str(), &path)
        .map_err(|error| CommandError::not_written(error.to_string()))?;
    let shown = path.display().to_string();
    state
        .exported_workbooks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(path);
    Ok(shown)
}

/// Shows a workbook in its folder, chosen in the file manager of the
/// operating system. Only a file this program wrote since it started is
/// shown, so that the interface can never make it open another place.
#[tauri::command]
pub async fn show_exported_workbook(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), CommandError> {
    let path = PathBuf::from(path);
    let was_written_here = state
        .exported_workbooks
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&path);
    if !was_written_here {
        return Err(CommandError::invalid_request(
            "only a workbook written since the program started can be shown",
        ));
    }
    if !path.is_file() {
        return Err(CommandError::not_available(format!(
            "{} is not there any more",
            path.display()
        )));
    }
    show_in_file_manager(&path)
}

/// Shows a file in its folder, chosen in the file manager of the operating
/// system. The caller has checked the path; it is handed over as one
/// argument, without a shell.
pub fn show_in_file_manager(path: &Path) -> Result<(), CommandError> {
    let mut file_manager = show_command(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            CommandError::not_available(format!("the file manager could not be started: {error}"))
        })?;
    // The command does not wait for the file manager. A thread waits for it
    // instead, so that it does not linger as a finished process.
    thread::spawn(move || {
        let _ = file_manager.wait();
    });
    Ok(())
}

/// The command that opens the folder of a file with the file chosen.
#[cfg(target_os = "windows")]
fn show_command(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new("explorer");
    // Explorer wants the path quoted after "/select,", not the whole
    // argument quoted, as Rust would quote a path with spaces. A path on
    // Windows cannot hold a quotation mark, so it cannot break out.
    command.raw_arg(format!("/select,\"{}\"", path.display()));
    command
}

/// The command that opens the folder of a file with the file chosen.
#[cfg(target_os = "macos")]
fn show_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg("-R").arg(path);
    command
}

/// The command that opens the folder of a file. Linux file managers have no
/// common way to choose the file, so the folder is opened.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn show_command(path: &Path) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path.parent().unwrap_or(path));
    command
}
