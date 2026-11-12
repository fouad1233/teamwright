//! Commands for the work of the AI specialists.
//!
//! Asking the agent program and starting a turn can take seconds, so those
//! commands are declared `async`: they then run on a thread of their own,
//! never on the main thread, which draws the window.

use tauri::State;

use crate::dto::{AgentProgramDto, ConversationDto, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Says whether the program that runs the specialists can be used. It
/// never fails: what went wrong is part of the answer.
#[tauri::command(async)]
pub fn agent_program(state: State<'_, AppState>) -> Result<AgentProgramDto, CommandError> {
    Ok(AgentProgramDto::from(state.work.program()))
}

/// Lists the conversations of a project with their turns, the oldest
/// conversation first.
#[tauri::command]
pub async fn list_conversations(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ConversationDto>, CommandError> {
    let conversations = state.work.list_for_project(parse_id(&project_id)?)?;
    Ok(conversations.iter().map(ConversationDto::from).collect())
}

/// Starts the work on a task: the specialist it is given to gets its brief.
#[tauri::command(async)]
pub fn start_work(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.start_work(parse_id(&task_id)?)?;
    Ok(ConversationDto::from(&conversation))
}

/// Goes on with a conversation with the manager's message.
#[tauri::command(async)]
pub fn reply(
    state: State<'_, AppState>,
    conversation_id: String,
    message: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.reply(parse_id(&conversation_id)?, &message)?;
    Ok(ConversationDto::from(&conversation))
}

/// Asks a specialist something, in a conversation that is not about a task.
#[tauri::command(async)]
pub fn ask(
    state: State<'_, AppState>,
    member_id: String,
    message: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.ask(parse_id(&member_id)?, &message)?;
    Ok(ConversationDto::from(&conversation))
}

/// Stops the turn that is running in a conversation.
#[tauri::command]
pub async fn stop_work(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.stop(parse_id(&conversation_id)?)?;
    Ok(ConversationDto::from(&conversation))
}

/// Opens the session of a conversation in the Claude desktop app.
#[tauri::command(async)]
pub fn open_in_claude(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), CommandError> {
    state
        .work
        .open_in_desktop_app(parse_id(&conversation_id)?)?;
    Ok(())
}
