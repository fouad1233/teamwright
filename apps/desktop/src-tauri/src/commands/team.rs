//! Commands for the team of a project.

use tauri::State;

use crate::dto::{MemberDto, MemberProfileInput, NewMemberInput, parse_effort, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the team of a project. The member who joined first comes first.
#[tauri::command]
pub async fn list_members(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<MemberDto>, CommandError> {
    let team = state.team.list_for_project(parse_id(&project_id)?)?;
    Ok(team.iter().map(MemberDto::from).collect())
}

/// Puts someone on the team of a project.
#[tauri::command]
pub async fn add_member(
    state: State<'_, AppState>,
    input: NewMemberInput,
) -> Result<MemberDto, CommandError> {
    let member = state.team.add(input.try_into()?)?;
    Ok(MemberDto::from(&member))
}

/// Gives a member a new name.
#[tauri::command]
pub async fn rename_member(
    state: State<'_, AppState>,
    member_id: String,
    name: String,
) -> Result<MemberDto, CommandError> {
    let member = state.team.rename(parse_id(&member_id)?, &name)?;
    Ok(MemberDto::from(&member))
}

/// Replaces what a member is on the team for.
#[tauri::command]
pub async fn set_member_responsibility(
    state: State<'_, AppState>,
    member_id: String,
    responsibility: String,
) -> Result<MemberDto, CommandError> {
    let member = state
        .team
        .set_responsibility(parse_id(&member_id)?, &responsibility)?;
    Ok(MemberDto::from(&member))
}

/// Says which model an AI specialist runs on and how hard it thinks. A
/// person can only be given the empty profile.
#[tauri::command]
pub async fn set_member_profile(
    state: State<'_, AppState>,
    member_id: String,
    profile: MemberProfileInput,
) -> Result<MemberDto, CommandError> {
    let effort = profile.effort.as_deref().map(parse_effort).transpose()?;
    let member = state
        .team
        .set_profile(parse_id(&member_id)?, profile.model.as_deref(), effort)?;
    Ok(MemberDto::from(&member))
}

/// Takes a member off the team. The member's tasks are given to nobody.
#[tauri::command]
pub async fn remove_member(
    state: State<'_, AppState>,
    member_id: String,
) -> Result<(), CommandError> {
    state.team.remove(parse_id(&member_id)?)?;
    Ok(())
}
