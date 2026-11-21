//! Commands for the custom fields of a project and the values of a task.

use tauri::State;
use teamwright_domain::{FieldKind, FieldValue};

use crate::dto::{FieldDto, FieldValueDto, TaskDto, parse_color, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the fields of a project in their order.
#[tauri::command]
pub async fn list_fields(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<FieldDto>, CommandError> {
    let fields = state.fields.list_for_project(parse_id(&project_id)?)?;
    Ok(fields.iter().map(FieldDto::from).collect())
}

/// Adds a field at the end of the fields of a project. Its kind never
/// changes; a number field may have a unit.
#[tauri::command]
pub async fn add_field(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
    kind: String,
    unit: Option<String>,
) -> Result<FieldDto, CommandError> {
    let kind: FieldKind = kind.parse()?;
    let field = state
        .fields
        .add(parse_id(&project_id)?, &name, kind, unit.as_deref())?;
    Ok(FieldDto::from(&field))
}

/// Renames a field.
#[tauri::command]
pub async fn rename_field(
    state: State<'_, AppState>,
    field_id: String,
    name: String,
) -> Result<FieldDto, CommandError> {
    let field = state.fields.rename(parse_id(&field_id)?, &name)?;
    Ok(FieldDto::from(&field))
}

/// Gives a number field a unit, or takes it away.
#[tauri::command]
pub async fn set_field_unit(
    state: State<'_, AppState>,
    field_id: String,
    unit: Option<String>,
) -> Result<FieldDto, CommandError> {
    let field = state
        .fields
        .set_unit(parse_id(&field_id)?, unit.as_deref())?;
    Ok(FieldDto::from(&field))
}

/// Moves a field to a place among the fields of its project, and answers
/// with the fields in their new order.
#[tauri::command]
pub async fn move_field(
    state: State<'_, AppState>,
    field_id: String,
    index: i64,
) -> Result<Vec<FieldDto>, CommandError> {
    let fields = state.fields.move_to(parse_id(&field_id)?, index)?;
    Ok(fields.iter().map(FieldDto::from).collect())
}

/// Removes a field: its values are cleared from the tasks, and views
/// filtered by it show every task again.
#[tauri::command]
pub async fn remove_field(
    state: State<'_, AppState>,
    field_id: String,
) -> Result<(), CommandError> {
    state.fields.remove(parse_id(&field_id)?)?;
    Ok(())
}

/// Adds a choice to a choice field, with a color or the next one in turn.
#[tauri::command]
pub async fn add_field_choice(
    state: State<'_, AppState>,
    field_id: String,
    name: String,
    color: Option<String>,
) -> Result<FieldDto, CommandError> {
    let color = color.as_deref().map(parse_color).transpose()?;
    let field = state
        .fields
        .add_choice(parse_id(&field_id)?, &name, color)?;
    Ok(FieldDto::from(&field))
}

/// Renames a choice of a choice field.
#[tauri::command]
pub async fn rename_field_choice(
    state: State<'_, AppState>,
    field_id: String,
    choice_id: String,
    name: String,
) -> Result<FieldDto, CommandError> {
    let field = state
        .fields
        .rename_choice(parse_id(&field_id)?, parse_id(&choice_id)?, &name)?;
    Ok(FieldDto::from(&field))
}

/// Gives a choice of a choice field another color.
#[tauri::command]
pub async fn recolor_field_choice(
    state: State<'_, AppState>,
    field_id: String,
    choice_id: String,
    color: String,
) -> Result<FieldDto, CommandError> {
    let field = state.fields.recolor_choice(
        parse_id(&field_id)?,
        parse_id(&choice_id)?,
        parse_color(&color)?,
    )?;
    Ok(FieldDto::from(&field))
}

/// Removes a choice of a choice field: the tasks that held it are cleared.
#[tauri::command]
pub async fn remove_field_choice(
    state: State<'_, AppState>,
    field_id: String,
    choice_id: String,
) -> Result<FieldDto, CommandError> {
    let field = state
        .fields
        .remove_choice(parse_id(&field_id)?, parse_id(&choice_id)?)?;
    Ok(FieldDto::from(&field))
}

/// Gives a task a value for a field of its project, and answers with the
/// task.
#[tauri::command]
pub async fn set_field_value(
    state: State<'_, AppState>,
    task_id: String,
    field_id: String,
    value: FieldValueDto,
) -> Result<TaskDto, CommandError> {
    let value = FieldValue::try_from(value)?;
    let task = state
        .fields
        .set_value(parse_id(&task_id)?, parse_id(&field_id)?, value)?;
    Ok(TaskDto::from(&task))
}

/// Clears the value of a field from a task, and answers with the task.
#[tauri::command]
pub async fn clear_field_value(
    state: State<'_, AppState>,
    task_id: String,
    field_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .fields
        .clear_value(parse_id(&task_id)?, parse_id(&field_id)?)?;
    Ok(TaskDto::from(&task))
}
