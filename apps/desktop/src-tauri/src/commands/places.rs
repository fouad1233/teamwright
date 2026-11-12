//! Commands for the folders of a project: the work folder, the reference
//! folders, and the files copied in for reference.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use tauri::State;
use teamwright_application::ApplicationError;

use crate::dto::{ProjectPlacesDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// The program that shows a folder in the file manager of the operating
/// system.
#[cfg(target_os = "windows")]
const FILE_MANAGER: &str = "explorer";
/// The program that shows a folder in the file manager of the operating
/// system.
#[cfg(target_os = "macos")]
const FILE_MANAGER: &str = "open";
/// The program that shows a folder in the file manager of the operating
/// system.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const FILE_MANAGER: &str = "xdg-open";

/// The folders of a project.
#[tauri::command]
pub async fn project_places(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectPlacesDto, CommandError> {
    let places = state.places.places(parse_id(&project_id)?)?;
    Ok(ProjectPlacesDto::from(places))
}

/// Makes a folder the work folder of a project, or, without one, goes back
/// to the folder Teamwright gives it.
#[tauri::command]
pub async fn set_work_folder(
    state: State<'_, AppState>,
    project_id: String,
    folder: Option<String>,
) -> Result<ProjectPlacesDto, CommandError> {
    let places = state
        .places
        .set_work_folder(parse_id(&project_id)?, folder.as_deref().map(Path::new))?;
    Ok(ProjectPlacesDto::from(places))
}

/// Lets the specialists of a project read a folder.
#[tauri::command]
pub async fn add_reference_folder(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> Result<ProjectPlacesDto, CommandError> {
    let places = state
        .places
        .add_reference_folder(parse_id(&project_id)?, Path::new(&folder))?;
    Ok(ProjectPlacesDto::from(places))
}

/// Takes a reference folder away from the specialists of a project.
#[tauri::command]
pub async fn remove_reference_folder(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> Result<ProjectPlacesDto, CommandError> {
    let places = state
        .places
        .remove_reference_folder(parse_id(&project_id)?, Path::new(&folder))?;
    Ok(ProjectPlacesDto::from(places))
}

/// Copies files into the folder `references` inside the work folder of a
/// project, and answers with the names they got there. Copying can take a
/// while, so it runs on a thread of its own.
#[tauri::command(async)]
pub fn add_reference_files(
    state: State<'_, AppState>,
    project_id: String,
    files: Vec<String>,
) -> Result<Vec<String>, CommandError> {
    let files: Vec<PathBuf> = files.into_iter().map(PathBuf::from).collect();
    Ok(state
        .places
        .add_reference_files(parse_id(&project_id)?, &files)?)
}

/// Shows the work folder of a project in the file manager of the operating
/// system. The folder Teamwright gives a project is created first if it is
/// missing; a chosen folder that is gone is reported.
#[tauri::command]
pub async fn show_work_folder(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), CommandError> {
    let project = state.projects.get(parse_id(&project_id)?)?;
    let folder = state.places.work_folder_of(&project)?;
    if !folder.is_dir() {
        return Err(ApplicationError::FolderNotFound {
            folder: folder.display().to_string(),
        }
        .into());
    }
    let mut file_manager = Command::new(FILE_MANAGER)
        .arg(&folder)
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
