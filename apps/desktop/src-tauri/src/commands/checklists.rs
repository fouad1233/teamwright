//! Commands for the checklist of a task. Each answers with the task, which
//! carries its whole checklist.

use tauri::State;

use crate::dto::{TaskDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Adds an item, not done, at the end of the checklist of a task.
#[tauri::command]
pub async fn add_checklist_item(
    state: State<'_, AppState>,
    task_id: String,
    text: String,
) -> Result<TaskDto, CommandError> {
    let task = state.checklists.add_item(parse_id(&task_id)?, &text)?;
    Ok(TaskDto::from(&task))
}

/// Gives an item of the checklist of a task another text.
#[tauri::command]
pub async fn rename_checklist_item(
    state: State<'_, AppState>,
    task_id: String,
    item_id: String,
    text: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .checklists
        .rename_item(parse_id(&task_id)?, parse_id(&item_id)?, &text)?;
    Ok(TaskDto::from(&task))
}

/// Marks an item of the checklist of a task done, or not done.
#[tauri::command]
pub async fn tick_checklist_item(
    state: State<'_, AppState>,
    task_id: String,
    item_id: String,
    done: bool,
) -> Result<TaskDto, CommandError> {
    let task = state
        .checklists
        .tick_item(parse_id(&task_id)?, parse_id(&item_id)?, done)?;
    Ok(TaskDto::from(&task))
}

/// Moves an item of the checklist of a task to another place.
#[tauri::command]
pub async fn move_checklist_item(
    state: State<'_, AppState>,
    task_id: String,
    item_id: String,
    index: f64,
) -> Result<TaskDto, CommandError> {
    // The place arrives as any number and is read as the interface's own
    // arrays read one: cut to a whole number, nothing for a number that is
    // not one. The use case puts a place past either end at that end.
    let index = index as i64;
    let task = state
        .checklists
        .move_item(parse_id(&task_id)?, parse_id(&item_id)?, index)?;
    Ok(TaskDto::from(&task))
}

/// Removes an item from the checklist of a task.
#[tauri::command]
pub async fn remove_checklist_item(
    state: State<'_, AppState>,
    task_id: String,
    item_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .checklists
        .remove_item(parse_id(&task_id)?, parse_id(&item_id)?)?;
    Ok(TaskDto::from(&task))
}
