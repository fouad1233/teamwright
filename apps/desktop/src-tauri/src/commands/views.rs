//! Commands for the views of a project.

use tauri::State;

use crate::dto::{NewViewInput, ViewDto, ViewFilterDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the views of a project in the order the manager gave them.
#[tauri::command]
pub async fn list_views(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ViewDto>, CommandError> {
    let views = state.views.list_for_project(parse_id(&project_id)?)?;
    Ok(views.iter().map(ViewDto::from).collect())
}

/// Adds a view at the end of the views of a project, showing every task.
#[tauri::command]
pub async fn add_view(
    state: State<'_, AppState>,
    input: NewViewInput,
) -> Result<ViewDto, CommandError> {
    let view = state.views.add(input.try_into()?)?;
    Ok(ViewDto::from(&view))
}

/// Renames a view.
#[tauri::command]
pub async fn rename_view(
    state: State<'_, AppState>,
    view_id: String,
    name: String,
) -> Result<ViewDto, CommandError> {
    let view = state.views.rename(parse_id(&view_id)?, &name)?;
    Ok(ViewDto::from(&view))
}

/// Replaces the filter of a board, a list, a Gantt chart or a calendar.
#[tauri::command]
pub async fn set_view_filter(
    state: State<'_, AppState>,
    view_id: String,
    filter: ViewFilterDto,
) -> Result<ViewDto, CommandError> {
    let view = state
        .views
        .set_filter(parse_id(&view_id)?, filter.try_into()?)?;
    Ok(ViewDto::from(&view))
}

/// Moves a view to another place among the views of its project, and
/// answers with all of them in their new order.
#[tauri::command]
pub async fn move_view(
    state: State<'_, AppState>,
    view_id: String,
    index: f64,
) -> Result<Vec<ViewDto>, CommandError> {
    // The place arrives as any number and is read as the interface's own
    // arrays read one: cut to a whole number, nothing for a number that is
    // not one. The use case puts a place past either end at that end.
    let index = index as i64;
    let views = state.views.move_to(parse_id(&view_id)?, index)?;
    Ok(views.iter().map(ViewDto::from).collect())
}

/// Removes a view. The last view of a project stays.
#[tauri::command]
pub async fn remove_view(state: State<'_, AppState>, view_id: String) -> Result<(), CommandError> {
    state.views.remove(parse_id(&view_id)?)?;
    Ok(())
}
