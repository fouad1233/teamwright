//! Commands for the library of skills and connectors of a project, and for
//! what each specialist is given of it (decision 0039).

use std::path::Path;

use tauri::State;

use crate::dto::{LibraryDto, MemberKitDto, NewConnectorInput, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// The library of a project, with what this computer has of it, and the
/// kits of its specialists.
#[tauri::command]
pub async fn project_library(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<LibraryDto, CommandError> {
    let library = state.kits.library(parse_id(&project_id)?)?;
    Ok(LibraryDto::from(&library))
}

/// Adds the skill in a folder the manager chose to the library, keeping a
/// copy of it on this computer, and answers with the library.
#[tauri::command]
pub async fn add_skill(
    state: State<'_, AppState>,
    project_id: String,
    folder: String,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state.kits.add_skill(project_id, Path::new(&folder))?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Removes a skill from the library and from every specialist, and answers
/// with the library.
#[tauri::command]
pub async fn remove_skill(
    state: State<'_, AppState>,
    project_id: String,
    skill_id: String,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state.kits.remove_skill(project_id, parse_id(&skill_id)?)?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Adds a connector to the library, and answers with the library.
#[tauri::command]
pub async fn add_connector(
    state: State<'_, AppState>,
    project_id: String,
    input: NewConnectorInput,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state.kits.add_connector(project_id, input.into())?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Removes a connector from the library and from every specialist, and
/// answers with the library.
#[tauri::command]
pub async fn remove_connector(
    state: State<'_, AppState>,
    project_id: String,
    connector_id: String,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state
        .kits
        .remove_connector(project_id, parse_id(&connector_id)?)?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Gives a specialist these skills and connectors and nothing else, and
/// answers with the library.
#[tauri::command]
pub async fn set_member_kit(
    state: State<'_, AppState>,
    project_id: String,
    member_id: String,
    kit: MemberKitDto,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state
        .kits
        .set_kit(parse_id(&member_id)?, kit.into_choice()?)?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Confirms, on this computer, the definition of a connector that arrived
/// or changed through the sync folder, exactly as the manager was shown it
/// (`fingerprint`), and answers with the library. A definition that
/// changed since is not confirmed (decision 0039, note of 22 November
/// 2026).
#[tauri::command]
pub async fn confirm_connector(
    state: State<'_, AppState>,
    project_id: String,
    connector_id: String,
    fingerprint: String,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state
        .kits
        .confirm_connector(project_id, parse_id(&connector_id)?, &fingerprint)?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}

/// Confirms, on this computer, the tools a specialist was given on another
/// computer that the manager was shown (`kit`), and answers with the
/// library. Tools the kit no longer gives, or gained since, are not
/// confirmed.
#[tauri::command]
pub async fn confirm_member_kit(
    state: State<'_, AppState>,
    project_id: String,
    member_id: String,
    kit: MemberKitDto,
) -> Result<LibraryDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    state
        .kits
        .confirm_kit(parse_id(&member_id)?, &kit.into_seen_tools()?)?;
    Ok(LibraryDto::from(&state.kits.library(project_id)?))
}
