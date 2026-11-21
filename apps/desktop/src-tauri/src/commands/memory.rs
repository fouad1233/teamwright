//! Commands for the memory of a project: decisions, facts and contracts.

use tauri::State;
use teamwright_application::NewFact;

use crate::dto::{MemoryEntryDto, NewMemoryFactInput, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Every entry of the memory of a project, current and replaced, the
/// newest first.
#[tauri::command]
pub async fn project_memory(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<MemoryEntryDto>, CommandError> {
    let entries = state.memory.memory_of_project(parse_id(&project_id)?)?;
    Ok(entries.iter().map(MemoryEntryDto::from).collect())
}

/// The current entries of a project that fit a few words, the newest
/// first; every current entry if the words are blank. A specialist's
/// search follows the same rule.
#[tauri::command]
pub async fn search_memory(
    state: State<'_, AppState>,
    project_id: String,
    query: String,
) -> Result<Vec<MemoryEntryDto>, CommandError> {
    let entries = state.memory.search(parse_id(&project_id)?, &query)?;
    Ok(entries.iter().map(MemoryEntryDto::from).collect())
}

/// Adds a fact the manager writes.
#[tauri::command]
pub async fn add_memory_fact(
    state: State<'_, AppState>,
    input: NewMemoryFactInput,
) -> Result<MemoryEntryDto, CommandError> {
    let entry = state.memory.add_fact(NewFact {
        project_id: parse_id(&input.project_id)?,
        title: input.title,
        text: input.text,
        task_id: input.task_id.as_deref().map(parse_id).transpose()?,
    })?;
    Ok(MemoryEntryDto::from(&entry))
}

/// Changes an entry: the manager's words become a newer entry that
/// replaces it. Returns the newer entry.
#[tauri::command]
pub async fn change_memory_entry(
    state: State<'_, AppState>,
    entry_id: String,
    title: String,
    text: String,
) -> Result<MemoryEntryDto, CommandError> {
    let entry = state.memory.change(parse_id(&entry_id)?, &title, &text)?;
    Ok(MemoryEntryDto::from(&entry))
}

/// Removes an entry from memory.
#[tauri::command]
pub async fn remove_memory_entry(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<(), CommandError> {
    Ok(state.memory.remove(parse_id(&entry_id)?)?)
}
