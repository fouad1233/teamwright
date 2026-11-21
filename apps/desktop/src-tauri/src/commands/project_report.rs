//! Commands for the report of a whole project
//! ([decision 0042](../../../../../docs/decisions/0042-the-report-of-a-whole-project.md)):
//! gathering it for the interface to draw, and keeping it as a Markdown
//! file, by default in the work folder of the project.

use std::path::PathBuf;

use tauri::State;
use teamwright_application::{Clock, SystemClock, report_file_name, report_in_markdown};
use teamwright_workbook::write_whole_file;

use crate::dto::{ProjectReportDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// The report of a project as it stands now. Gathering it reads every
/// conversation of the project, so it runs on a thread of its own.
#[tauri::command(async)]
pub fn project_report(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectReportDto, CommandError> {
    let report = state.project_report.report_of(parse_id(&project_id)?)?;
    Ok(ProjectReportDto::from(report))
}

/// Where the save window suggests keeping the report: in the work folder
/// of the project, named after the project and the day, so that the
/// report stays with the work.
#[tauri::command(async)]
pub fn suggested_report_path(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<String, CommandError> {
    let project = state.projects.get(parse_id(&project_id)?)?;
    let today = SystemClock.day_of(SystemClock.now());
    let folder = state.places.work_folder_of(&project)?;
    Ok(folder
        .join(report_file_name(project.name().as_str(), today))
        .display()
        .to_string())
}

/// Writes the report of a project as Markdown to the file the manager
/// chose in the save window, and answers with its path. The text is made
/// whole first and written through a temporary file that is then renamed,
/// so that a half-written report is never left. A link at the name is
/// refused: the work folder is one a specialist can write in, and a link
/// placed there must not lead the report onto another file.
#[tauri::command(async)]
pub fn save_project_report(
    state: State<'_, AppState>,
    project_id: String,
    path: String,
) -> Result<String, CommandError> {
    let report = state.project_report.report_of(parse_id(&project_id)?)?;
    let path = PathBuf::from(path);
    write_whole_file(&path, report_in_markdown(&report).as_bytes()).map_err(|error| {
        CommandError::not_written(format!("{} could not be written: {error}", path.display()))
    })?;
    Ok(path.display().to_string())
}
