//! Commands for earlier conversations and their digests
//! ([decision 0041](../../../../../docs/decisions/0041-earlier-conversations-as-digests.md)):
//! finding the conversations held with Claude Code on this computer,
//! making the digest of one, and keeping, listing and removing digests.

use tauri::State;
use teamwright_application::DigestToKeep;

use crate::dto::digests::{ConversationsOfFolderDto, DigestDraftDto, DigestDto};
use crate::dto::parse_id;
use crate::error::CommandError;
use crate::state::AppState;

/// The earlier conversations on this computer, grouped by the folder they
/// were held in, the folders of the project first. Reading them takes a
/// moment, so it runs on a thread of its own.
#[tauri::command(async)]
pub fn find_earlier_conversations(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ConversationsOfFolderDto>, CommandError> {
    let found = state.digests.find_earlier(parse_id(&project_id)?)?;
    Ok(found
        .into_iter()
        .map(ConversationsOfFolderDto::from)
        .collect())
}

/// The digest of an earlier conversation, made by the rules, for the
/// manager to read and correct. Nothing is kept. A long conversation takes
/// a moment to read, so it runs on a thread of its own.
#[tauri::command(async)]
pub fn draft_digest(
    state: State<'_, AppState>,
    project_id: String,
    source: String,
) -> Result<DigestDraftDto, CommandError> {
    let draft = state.digests.draft(parse_id(&project_id)?, &source)?;
    Ok(DigestDraftDto::from(draft))
}

/// Keeps a digest with a project, as the manager corrected it.
#[tauri::command]
pub async fn keep_digest(
    state: State<'_, AppState>,
    project_id: String,
    digest: DigestDraftDto,
) -> Result<DigestDto, CommandError> {
    let kept = state
        .digests
        .keep(parse_id(&project_id)?, DigestToKeep::try_from(digest)?)?;
    Ok(DigestDto::from(&kept))
}

/// The digests kept with a project, the one kept last first.
#[tauri::command]
pub async fn list_digests(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<DigestDto>, CommandError> {
    let digests = state.digests.digests_of(parse_id(&project_id)?)?;
    Ok(digests.iter().map(DigestDto::from).collect())
}

/// Removes a digest from its project.
#[tauri::command]
pub async fn remove_digest(
    state: State<'_, AppState>,
    digest_id: String,
) -> Result<(), CommandError> {
    state.digests.remove(parse_id(&digest_id)?)?;
    Ok(())
}
