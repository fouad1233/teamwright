//! Commands for projects.

use tauri::State;
use teamwright_domain::ProjectTemplate;

use crate::dto::{
    NewProjectInput, ProjectDto, ProjectTemplateDto, StatusLoadDto, StatusPropertiesInput, parse_id,
};
use crate::error::CommandError;
use crate::state::AppState;

/// Lists the projects that are not archived, oldest first.
#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectDto>, CommandError> {
    let projects = state.projects.list()?;
    Ok(projects.iter().map(ProjectDto::from).collect())
}

/// Starts a project.
#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    input: NewProjectInput,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.create(input.into())?;
    Ok(ProjectDto::from(&project))
}

/// Gives a project a new name.
#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.rename(parse_id(&project_id)?, &name)?;
    Ok(ProjectDto::from(&project))
}

/// Replaces the description of a project.
#[tauri::command]
pub async fn describe_project(
    state: State<'_, AppState>,
    project_id: String,
    description: String,
) -> Result<ProjectDto, CommandError> {
    let project = state
        .projects
        .describe(parse_id(&project_id)?, &description)?;
    Ok(ProjectDto::from(&project))
}

/// Puts a project away.
#[tauri::command]
pub async fn archive_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.archive(parse_id(&project_id)?)?;
    Ok(ProjectDto::from(&project))
}

/// Lists the projects that were put away, the one put away last first.
#[tauri::command]
pub async fn list_archived_projects(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectDto>, CommandError> {
    let projects = state.projects.list_archived()?;
    Ok(projects.iter().map(ProjectDto::from).collect())
}

/// Brings an archived project back.
#[tauri::command]
pub async fn restore_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.restore(parse_id(&project_id)?)?;
    Ok(ProjectDto::from(&project))
}

/// Removes a project for good, with its tasks and its team.
#[tauri::command]
pub async fn delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<(), CommandError> {
    state.projects.delete(parse_id(&project_id)?)?;
    Ok(())
}

/// Adds a status to the workflow of a project.
#[tauri::command]
pub async fn add_status(
    state: State<'_, AppState>,
    project_id: String,
    status: StatusPropertiesInput,
) -> Result<ProjectDto, CommandError> {
    let project = state
        .projects
        .add_status(parse_id(&project_id)?, status.try_into()?)?;
    Ok(ProjectDto::from(&project))
}

/// Gives a status a new name, category and color.
#[tauri::command]
pub async fn change_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    status: StatusPropertiesInput,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.change_status(
        parse_id(&project_id)?,
        parse_id(&status_id)?,
        status.try_into()?,
    )?;
    Ok(ProjectDto::from(&project))
}

/// Moves a status to another place in the workflow.
#[tauri::command]
pub async fn move_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    index: usize,
) -> Result<ProjectDto, CommandError> {
    let project =
        state
            .projects
            .move_status(parse_id(&project_id)?, parse_id(&status_id)?, index)?;
    Ok(ProjectDto::from(&project))
}

/// Gives a status a work-in-progress limit, another one, or none
/// (`limit` left out or `null`).
#[tauri::command]
pub async fn set_status_limit(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    limit: Option<i64>,
) -> Result<ProjectDto, CommandError> {
    let project =
        state
            .projects
            .set_status_limit(parse_id(&project_id)?, parse_id(&status_id)?, limit)?;
    Ok(ProjectDto::from(&project))
}

/// How many tasks stand in each status of a project, against its limit, in
/// board order.
#[tauri::command]
pub async fn list_status_loads(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<StatusLoadDto>, CommandError> {
    let loads = state.projects.status_loads(parse_id(&project_id)?)?;
    Ok(loads.iter().map(StatusLoadDto::from).collect())
}

/// Lists the templates a new project can start from, in the order the
/// manager is shown them.
#[tauri::command]
pub async fn list_project_templates() -> Result<Vec<ProjectTemplateDto>, CommandError> {
    Ok(project_templates())
}

/// The templates in their form on the wire.
fn project_templates() -> Vec<ProjectTemplateDto> {
    ProjectTemplate::ALL
        .iter()
        .map(ProjectTemplateDto::from)
        .collect()
}

/// Removes a status from the workflow. The tasks in it go to the status
/// named by `move_tasks_to`, which is needed only if there are any.
#[tauri::command]
pub async fn remove_status(
    state: State<'_, AppState>,
    project_id: String,
    status_id: String,
    move_tasks_to: Option<String>,
) -> Result<ProjectDto, CommandError> {
    let project = state.projects.remove_status(
        parse_id(&project_id)?,
        parse_id(&status_id)?,
        move_tasks_to.as_deref().map(parse_id).transpose()?,
    )?;
    Ok(ProjectDto::from(&project))
}

#[cfg(test)]
mod tests {
    // The helpers below are test code as well, but the linter only
    // recognizes functions marked `#[test]` as such.
    #![allow(clippy::unwrap_used)]

    use std::path::PathBuf;

    use super::*;
    use crate::dto::WorkflowTemplateDto;

    /// The interface's copy of the templates. The demo makes projects from
    /// it, and the chooser of a new project draws its lanes from it.
    fn copy_for_the_interface() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/shared/api/projectTemplates.json")
    }

    fn what_the_program_knows() -> String {
        let known = serde_json::json!({
            "limitRange": {
                "min": teamwright_domain::WorkInProgressLimit::MIN,
                "max": teamwright_domain::WorkInProgressLimit::MAX,
            },
            "templates": project_templates(),
        });
        let mut text = serde_json::to_string_pretty(&known).unwrap();
        text.push('\n');
        text
    }

    /// Fails when the interface's copy differs from the program. Run the
    /// tests with `TEAMWRIGHT_WRITE_TEMPLATES=1` to write the copy anew.
    #[test]
    fn the_interface_has_a_true_copy_of_the_project_templates() {
        let expected = what_the_program_knows();
        if std::env::var_os("TEAMWRIGHT_WRITE_TEMPLATES").is_some() {
            std::fs::write(copy_for_the_interface(), &expected).unwrap();
        }
        let copy = std::fs::read_to_string(copy_for_the_interface()).unwrap();
        assert!(
            copy.replace("\r\n", "\n") == expected,
            "src/shared/api/projectTemplates.json differs from the program; run the tests of the \
             desktop crate with TEAMWRIGHT_WRITE_TEMPLATES=1 to write it anew"
        );
    }

    /// Every template the program knows can be asked for by its key, and
    /// makes the project of that template.
    #[test]
    fn every_template_can_be_chosen_on_the_wire_by_its_key() {
        for template in ProjectTemplate::ALL {
            let chosen: WorkflowTemplateDto =
                serde_json::from_value(serde_json::json!(template.key)).unwrap();
            let made: teamwright_application::WorkflowTemplate = chosen.into();
            assert_eq!(made.template(), template);
        }
    }
}
