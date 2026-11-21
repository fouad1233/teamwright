//! Commands for the views of a project.

use tauri::State;

use crate::dto::{NewViewInput, TaskGroupDto, ViewDto, ViewFilterDto, parse_id};
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

/// Replaces the filter of a view of a kind that shows some of the tasks.
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

/// Changes what a board or a list groups its tasks by.
#[tauri::command]
pub async fn set_view_grouping(
    state: State<'_, AppState>,
    view_id: String,
    grouping: String,
) -> Result<ViewDto, CommandError> {
    let view = state
        .views
        .set_grouping(parse_id(&view_id)?, grouping.parse()?)?;
    Ok(ViewDto::from(&view))
}

/// The groups of the tasks of a view: the lanes of a board and the sections
/// of a list, in their order.
#[tauri::command]
pub async fn list_task_groups(
    state: State<'_, AppState>,
    view_id: String,
) -> Result<Vec<TaskGroupDto>, CommandError> {
    let groups = state.groups.groups_of_view(parse_id(&view_id)?)?;
    Ok(groups.iter().map(TaskGroupDto::from).collect())
}

/// Replaces what a view keeps of its own, or clears it with `null`. The
/// interface writes the content as JSON; the backend does not read it.
#[tauri::command]
pub async fn set_view_content(
    state: State<'_, AppState>,
    view_id: String,
    content: Option<String>,
) -> Result<ViewDto, CommandError> {
    let view = state.views.set_content(parse_id(&view_id)?, content)?;
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
