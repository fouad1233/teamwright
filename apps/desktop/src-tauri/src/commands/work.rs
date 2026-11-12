//! Commands for the work of the AI specialists.
//!
//! Asking the agent program and starting a turn can take seconds, so those
//! commands are declared `async`: they then run on a thread of their own,
//! never on the main thread, which draws the window.

use tauri::State;

use teamwright_application::ApprovedProposal;

use crate::dto::{
    AgentProgramDto, ApprovedProposalInput, ConversationDto, DecisionAnswerInput,
    InterviewAnswerInput, NewAdviceInput, parse_effort, parse_id,
};
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
    Ok(conversations
        .iter()
        .map(|conversation| ConversationDto::of(conversation, state.this_computer))
        .collect())
}

/// Starts the work on a task: the specialist it is given to gets its brief.
#[tauri::command(async)]
pub fn start_work(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.start_work(parse_id(&task_id)?)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Goes on with a conversation with the manager's message.
#[tauri::command(async)]
pub fn reply(
    state: State<'_, AppState>,
    conversation_id: String,
    message: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.reply(parse_id(&conversation_id)?, &message)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Asks a specialist something, in a conversation that is not about a task.
#[tauri::command(async)]
pub fn ask(
    state: State<'_, AppState>,
    member_id: String,
    message: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.ask(parse_id(&member_id)?, &message)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Answers the question the specialist of a conversation asked, with an
/// option, words of the manager's own, or both. The specialist goes on in
/// the same session with the answer.
#[tauri::command(async)]
pub fn answer_decision(
    state: State<'_, AppState>,
    conversation_id: String,
    answer: DecisionAnswerInput,
) -> Result<ConversationDto, CommandError> {
    let conversation =
        state
            .work
            .answer_decision(parse_id(&conversation_id)?, answer.option, &answer.note)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Starts planning a project with the Advisor: a new conversation that
/// begins with what the manager wants.
#[tauri::command(async)]
pub fn advise(
    state: State<'_, AppState>,
    project_id: String,
    advice: NewAdviceInput,
) -> Result<ConversationDto, CommandError> {
    let effort = advice.effort.as_deref().map(parse_effort).transpose()?;
    let conversation = state.work.advise(
        parse_id(&project_id)?,
        &advice.brief,
        advice.model.as_deref(),
        effort,
    )?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Answers the questions the Advisor asked in a conversation, one answer
/// for each question. The Advisor goes on in the same session.
#[tauri::command(async)]
pub fn answer_interview(
    state: State<'_, AppState>,
    conversation_id: String,
    answers: Vec<InterviewAnswerInput>,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.answer_interview(
        parse_id(&conversation_id)?,
        answers.into_iter().map(Into::into).collect(),
    )?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Approves the proposal of the latest turn of a conversation with the
/// Advisor, as the manager changed it.
#[tauri::command(async)]
pub fn approve_proposal(
    state: State<'_, AppState>,
    conversation_id: String,
    proposal: ApprovedProposalInput,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.approve_proposal(
        parse_id(&conversation_id)?,
        ApprovedProposal::try_from(proposal)?,
    )?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
}

/// Lists the conversations that wait for the manager, in every project that
/// is not archived, the longest wait first: a specialist waits for a
/// decision, or the Advisor for answers or for a word about its proposal.
#[tauri::command]
pub async fn list_conversations_waiting(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationDto>, CommandError> {
    let conversations = state.work.waiting_for_manager()?;
    Ok(conversations
        .iter()
        .map(|conversation| ConversationDto::of(conversation, state.this_computer))
        .collect())
}

/// Stops the turn that is running in a conversation.
#[tauri::command]
pub async fn stop_work(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<ConversationDto, CommandError> {
    let conversation = state.work.stop(parse_id(&conversation_id)?)?;
    Ok(ConversationDto::of(&conversation, state.this_computer))
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
