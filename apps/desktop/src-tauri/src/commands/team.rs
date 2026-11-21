//! Commands for the team of a project.

use tauri::State;
use teamwright_domain::ReadyMadeSpecialist;

use crate::dto::{
    MemberDto, MemberProfileInput, NewMemberInput, ReadyMadeSpecialistDto, parse_effort, parse_id,
};
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

/// Replaces the instructions of an AI specialist. A person can only be
/// given empty instructions.
#[tauri::command]
pub async fn set_member_instructions(
    state: State<'_, AppState>,
    member_id: String,
    instructions: String,
) -> Result<MemberDto, CommandError> {
    let member = state
        .team
        .set_instructions(parse_id(&member_id)?, &instructions)?;
    Ok(MemberDto::from(&member))
}

/// Lists the ready-made specialists that ship with Teamwright, in the order
/// the manager is shown them.
#[tauri::command]
pub async fn list_ready_made_specialists() -> Result<Vec<ReadyMadeSpecialistDto>, CommandError> {
    Ok(ready_made_specialists())
}

/// Puts a ready-made specialist on the team of a project.
#[tauri::command]
pub async fn add_ready_made_member(
    state: State<'_, AppState>,
    project_id: String,
    key: String,
) -> Result<MemberDto, CommandError> {
    let member = state.team.add_ready_made(parse_id(&project_id)?, &key)?;
    Ok(MemberDto::from(&member))
}

/// The ready-made specialists in their form on the wire.
fn ready_made_specialists() -> Vec<ReadyMadeSpecialistDto> {
    ReadyMadeSpecialist::ALL
        .iter()
        .map(ReadyMadeSpecialistDto::from)
        .collect()
}

#[cfg(test)]
mod tests {
    // The helpers below are test code as well, but the linter only
    // recognizes functions marked `#[test]` as such.
    #![allow(clippy::unwrap_used)]

    use std::path::PathBuf;

    use teamwright_domain::MemberInstructions;

    use super::*;

    /// The interface's copy of what the program knows about ready-made
    /// specialists. The demo offers the same set from it, and the
    /// interface counts instructions against the same limit.
    fn copy_for_the_interface() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/shared/api/readyMade.json")
    }

    fn what_the_program_knows() -> String {
        let known = serde_json::json!({
            "memberInstructionsLimit": MemberInstructions::MAX_LENGTH,
            "specialists": ready_made_specialists(),
        });
        let mut text = serde_json::to_string_pretty(&known).unwrap();
        text.push('\n');
        text
    }

    /// Fails when the interface's copy differs from the program. Run the
    /// tests with `TEAMWRIGHT_WRITE_READY_MADE=1` to write the copy anew.
    #[test]
    fn the_interface_has_a_true_copy_of_the_ready_made_specialists() {
        let expected = what_the_program_knows();
        if std::env::var_os("TEAMWRIGHT_WRITE_READY_MADE").is_some() {
            std::fs::write(copy_for_the_interface(), &expected).unwrap();
        }
        let copy = std::fs::read_to_string(copy_for_the_interface()).unwrap();
        assert!(
            copy.replace("\r\n", "\n") == expected,
            "src/shared/api/readyMade.json differs from the program; run the tests of the \
             desktop crate with TEAMWRIGHT_WRITE_READY_MADE=1 to write it anew"
        );
    }
}
