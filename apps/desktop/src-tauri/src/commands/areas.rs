//! Commands for the areas of a project.

use tauri::State;

use crate::dto::{AreaDto, TaskDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the areas of a project in the order the manager gave them.
#[tauri::command]
pub async fn list_areas(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<AreaDto>, CommandError> {
    let areas = state.areas.list_for_project(parse_id(&project_id)?)?;
    Ok(areas.iter().map(AreaDto::from).collect())
}

/// Adds an area at the end of the areas of a project.
#[tauri::command]
pub async fn add_area(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<AreaDto, CommandError> {
    let area = state.areas.add(parse_id(&project_id)?, &name)?;
    Ok(AreaDto::from(&area))
}

/// Renames an area.
#[tauri::command]
pub async fn rename_area(
    state: State<'_, AppState>,
    area_id: String,
    name: String,
) -> Result<AreaDto, CommandError> {
    let area = state.areas.rename(parse_id(&area_id)?, &name)?;
    Ok(AreaDto::from(&area))
}

/// Moves an area to another place among the areas of its project, and
/// answers with all of them in their new order.
#[tauri::command]
pub async fn move_area(
    state: State<'_, AppState>,
    area_id: String,
    index: f64,
) -> Result<Vec<AreaDto>, CommandError> {
    // The place arrives as any number and is read as the interface's own
    // arrays read one: cut to a whole number, nothing for a number that is
    // not one. The use case puts a place past either end at that end.
    let index = index as i64;
    let areas = state.areas.move_to(parse_id(&area_id)?, index)?;
    Ok(areas.iter().map(AreaDto::from).collect())
}

/// Removes an area. The last area of a project stays. The tasks in it go to
/// the area named by `move_tasks_to`, which is looked at only if there are
/// any.
#[tauri::command]
pub async fn remove_area(
    state: State<'_, AppState>,
    area_id: String,
    move_tasks_to: Option<String>,
) -> Result<(), CommandError> {
    let move_tasks_to = move_tasks_to.as_deref().map(parse_id).transpose()?;
    state.areas.remove(parse_id(&area_id)?, move_tasks_to)?;
    Ok(())
}

/// Moves a task, with its subtasks, into another area of its project, and
/// answers with the task.
#[tauri::command]
pub async fn move_task_to_area(
    state: State<'_, AppState>,
    task_id: String,
    area_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .areas
        .move_task(parse_id(&task_id)?, parse_id(&area_id)?)?;
    Ok(TaskDto::from(&task))
}
