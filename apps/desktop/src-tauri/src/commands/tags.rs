//! Commands for the tags of a project and the tags of a task.

use tauri::State;

use crate::dto::{TagDto, TaggedTaskDto, TaskDto, parse_color, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the tags of a project by name.
#[tauri::command]
pub async fn list_tags(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<TagDto>, CommandError> {
    let tags = state.tags.list_for_project(parse_id(&project_id)?)?;
    Ok(tags.iter().map(TagDto::from).collect())
}

/// Adds a tag to a project, with a color or the next one in turn.
#[tauri::command]
pub async fn add_tag(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    color: Option<String>,
) -> Result<TagDto, CommandError> {
    let color = color.as_deref().map(parse_color).transpose()?;
    let tag = state.tags.add(parse_id(&project_id)?, &name, color)?;
    Ok(TagDto::from(&tag))
}

/// Renames a tag.
#[tauri::command]
pub async fn rename_tag(
    state: State<'_, AppState>,
    tag_id: String,
    name: String,
) -> Result<TagDto, CommandError> {
    let tag = state.tags.rename(parse_id(&tag_id)?, &name)?;
    Ok(TagDto::from(&tag))
}

/// Gives a tag another color.
#[tauri::command]
pub async fn recolor_tag(
    state: State<'_, AppState>,
    tag_id: String,
    color: String,
) -> Result<TagDto, CommandError> {
    let tag = state
        .tags
        .recolor(parse_id(&tag_id)?, parse_color(&color)?)?;
    Ok(TagDto::from(&tag))
}

/// Removes a tag: it comes off its tasks, and out of the filters of views.
#[tauri::command]
pub async fn remove_tag(state: State<'_, AppState>, tag_id: String) -> Result<(), CommandError> {
    state.tags.remove(parse_id(&tag_id)?)?;
    Ok(())
}

/// Puts a tag of its project on a task, and answers with the task.
#[tauri::command]
pub async fn put_tag_on_task(
    state: State<'_, AppState>,
    task_id: String,
    tag_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state.tags.put_on(parse_id(&task_id)?, parse_id(&tag_id)?)?;
    Ok(TaskDto::from(&task))
}

/// Puts a tag on a task by its name: the tag of the project with that name,
/// or a new one. Answers with the task and the tag.
#[tauri::command]
pub async fn put_named_tag_on_task(
    state: State<'_, AppState>,
    task_id: String,
    name: String,
) -> Result<TaggedTaskDto, CommandError> {
    let (task, tag) = state.tags.put_on_by_name(parse_id(&task_id)?, &name)?;
    Ok(TaggedTaskDto::of(&task, &tag))
}

/// Takes a tag off a task, and answers with the task.
#[tauri::command]
pub async fn take_tag_off_task(
    state: State<'_, AppState>,
    task_id: String,
    tag_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .tags
        .take_off(parse_id(&task_id)?, parse_id(&tag_id)?)?;
    Ok(TaskDto::from(&task))
}
