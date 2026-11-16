//! Commands for the files of a project: what lies in its folders, opening a
//! file with the program the system has for it, and showing it in the file
//! manager.
//!
//! A path from the interface is never handed to the system as it came: the
//! use case checks that it stays inside a folder of the project, links
//! followed, and answers with its full form. That form is handed over as a
//! single argument, without a shell.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use tauri::State;
use teamwright_domain::project_files::runs_as_program;

use crate::commands::workbook::show_in_file_manager;
use crate::dto::{FolderListingDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// What lies in a folder of a project, at a path inside it (empty for the
/// folder itself). Reading a large folder can take a moment, so it runs on
/// a thread of its own.
#[tauri::command(async)]
pub fn list_project_files(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
    path: String,
    shows_hidden: bool,
) -> Result<FolderListingDto, CommandError> {
    let listing = state.files.list(
        parse_id(&project_id)?,
        Path::new(&folder),
        &path,
        shows_hidden,
    )?;
    Ok(FolderListingDto::from(listing))
}

/// Opens a file of a project with the program the system has for it. A
/// folder is opened in the view instead, and a program is not run: it is
/// shown in its folder (`invalid_request`).
#[tauri::command]
pub async fn open_project_file(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
    path: String,
) -> Result<(), CommandError> {
    let file = state
        .files
        .locate(parse_id(&project_id)?, Path::new(&folder), &path)?;
    if file.is_dir() {
        return Err(CommandError::invalid_request(
            "a folder is opened in the view, not with another program",
        ));
    }
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if runs_as_program(&name) {
        return Err(CommandError::invalid_request(format!(
            "{name} is a program, and Teamwright does not run programs; show it in its folder \
             instead"
        )));
    }
    // A path on Windows cannot hold a quotation mark; one that does, on
    // another system, is not handed over.
    if file.to_string_lossy().contains('"') {
        return Err(CommandError::invalid_request(
            "a file whose path holds a quotation mark is not opened",
        ));
    }
    let mut opener = open_command(&file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            CommandError::not_available(format!("the file could not be opened: {error}"))
        })?;
    // The command does not wait for the program. A thread waits for the
    // opener instead, so that it does not linger as a finished process.
    thread::spawn(move || {
        let _ = opener.wait();
    });
    Ok(())
}

/// Shows a file or folder of a project in the file manager of the system.
#[tauri::command]
pub async fn show_project_file(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
    path: String,
) -> Result<(), CommandError> {
    let found = state
        .files
        .locate(parse_id(&project_id)?, Path::new(&folder), &path)?;
    show_in_file_manager(&found)
}

/// The command that opens a file with the program the system has for it.
#[cfg(target_os = "windows")]
fn open_command(path: &Path) -> Command {
    use std::os::windows::process::CommandExt;

    let mut command = Command::new("explorer");
    // Explorer splits its arguments at commas unless they are quoted. The
    // checked path holds no quotation mark, so it cannot break out.
    command.raw_arg(format!("\"{}\"", path.display()));
    command
}

/// The command that opens a file with the program the system has for it.
#[cfg(target_os = "macos")]
fn open_command(path: &Path) -> Command {
    let mut command = Command::new("open");
    command.arg(path);
    command
}

/// The command that opens a file with the program the system has for it.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_command(path: &Path) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(path);
    command
}
