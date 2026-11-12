//! Commands for tasks.

use tauri::State;
use teamwright_domain::{Assignee, ComputerId};

use crate::dto::{
    AssigneeDto, NewTaskInput, TaskChangeDto, TaskDto, parse_id, parse_optional_date,
    parse_priority,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the tasks of a project in the order the manager arranged them.
#[tauri::command]
pub async fn list_tasks(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<TaskDto>, CommandError> {
    let tasks = state.tasks.list_for_project(parse_id(&project_id)?)?;
    Ok(tasks.iter().map(TaskDto::from).collect())
}

/// Opens a task.
#[tauri::command]
pub async fn create_task(
    state: State<'_, AppState>,
    input: NewTaskInput,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.create(input.try_into()?)?;
    Ok(TaskDto::from(&task))
}

/// Gives a task a new title.
#[tauri::command]
pub async fn rename_task(
    state: State<'_, AppState>,
    task_id: String,
    title: String,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.rename(parse_id(&task_id)?, &title)?;
    Ok(TaskDto::from(&task))
}

/// Replaces the description of a task.
#[tauri::command]
pub async fn describe_task(
    state: State<'_, AppState>,
    task_id: String,
    description: String,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.describe(parse_id(&task_id)?, &description)?;
    Ok(TaskDto::from(&task))
}

/// Changes how urgent a task is.
#[tauri::command]
pub async fn prioritize_task(
    state: State<'_, AppState>,
    task_id: String,
    priority: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .tasks
        .prioritize(parse_id(&task_id)?, parse_priority(&priority)?)?;
    Ok(TaskDto::from(&task))
}

/// Makes someone responsible for a task, or nobody.
#[tauri::command]
pub async fn assign_task(
    state: State<'_, AppState>,
    task_id: String,
    assignee: Option<AssigneeDto>,
) -> Result<TaskDto, CommandError> {
    let assignee = assignee.map(Assignee::try_from).transpose()?;
    let task = state.tasks.assign(parse_id(&task_id)?, assignee)?;
    Ok(TaskDto::from(&task))
}

/// Sets or clears the expected working time of a task.
#[tauri::command]
pub async fn estimate_task(
    state: State<'_, AppState>,
    task_id: String,
    minutes: Option<u32>,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.estimate(parse_id(&task_id)?, minutes)?;
    Ok(TaskDto::from(&task))
}

/// Changes the planned start and due dates of a task.
#[tauri::command]
pub async fn reschedule_task(
    state: State<'_, AppState>,
    task_id: String,
    start_on: Option<String>,
    due_on: Option<String>,
) -> Result<TaskDto, CommandError> {
    let task = state.tasks.reschedule(
        parse_id(&task_id)?,
        parse_optional_date(start_on.as_deref())?,
        parse_optional_date(due_on.as_deref())?,
    )?;
    Ok(TaskDto::from(&task))
}

/// Moves a task to a status and to a position among the tasks there.
#[tauri::command]
pub async fn move_task(
    state: State<'_, AppState>,
    task_id: String,
    status_id: String,
    index: usize,
) -> Result<TaskDto, CommandError> {
    let task = state
        .tasks
        .move_to(parse_id(&task_id)?, parse_id(&status_id)?, index)?;
    Ok(TaskDto::from(&task))
}

/// Makes a task wait until another task is finished.
#[tauri::command]
pub async fn add_task_dependency(
    state: State<'_, AppState>,
    task_id: String,
    blocker_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .tasks
        .add_dependency(parse_id(&task_id)?, parse_id(&blocker_id)?)?;
    Ok(TaskDto::from(&task))
}

/// Stops a task from waiting for another task.
#[tauri::command]
pub async fn remove_task_dependency(
    state: State<'_, AppState>,
    task_id: String,
    blocker_id: String,
) -> Result<TaskDto, CommandError> {
    let task = state
        .tasks
        .remove_dependency(parse_id(&task_id)?, parse_id(&blocker_id)?)?;
    Ok(TaskDto::from(&task))
}

/// Removes a task together with its subtasks.
#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, task_id: String) -> Result<(), CommandError> {
    state.tasks.delete(parse_id(&task_id)?)?;
    Ok(())
}

/// Tells the history of a task, the oldest change first.
#[tauri::command]
pub async fn list_task_history(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Vec<TaskChangeDto>, CommandError> {
    let history = state.tasks.history_with_computers(parse_id(&task_id)?)?;
    // The other computers by name; a change made here, or on a computer
    // whose card was never read, names none.
    let computers = state.sync.state()?.computers;
    let name_of = |computer: Option<ComputerId>| {
        let computer = computer.filter(|computer| *computer != state.this_computer)?;
        computers
            .iter()
            .find(|known| known.id == computer)
            .and_then(|known| known.name.clone())
    };
    Ok(history
        .iter()
        .map(|entry| TaskChangeDto::from(&entry.change).made_on(name_of(entry.computer)))
        .collect())
}
