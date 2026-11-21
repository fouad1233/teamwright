//! Commands for the coordinator of a project: its switch, its limit, the
//! budgets, and the questions about used-up budgets (decision 0037).
//!
//! Every change nudges the coordinator, which then acts on its own thread
//! (`state::CoordinatorNudge`): a command never waits for turns to start.

use tauri::State;
use teamwright_application::{CoordinatorState, SyncState};
use teamwright_domain::{BudgetAnswer, ComputerId, ProjectId};

use crate::dto::{
    BudgetQuestionDto, BudgetScopeInput, CoordinatorDto, CoordinatorSurroundings, parse_id,
};
use crate::error::CommandError;
use crate::state::AppState;

/// The state of a coordinator, put into words for the interface.
fn dto_of(
    state: &AppState,
    project_id: ProjectId,
    coordinator: &CoordinatorState,
) -> Result<CoordinatorDto, CommandError> {
    let project = state.projects.get(project_id)?;
    let tasks = state.tasks.list_for_project(project_id)?;
    let members = state.team.list_for_project(project_id)?;
    // Without the state of syncing, the other computer is not named.
    let computer_names: Vec<(ComputerId, String)> = state
        .sync
        .state()
        .ok()
        .map(|sync: SyncState| {
            sync.computers
                .into_iter()
                .filter_map(|computer| computer.name.map(|name| (computer.id, name)))
                .collect()
        })
        .unwrap_or_default();
    Ok(CoordinatorDto::new(
        coordinator,
        &CoordinatorSurroundings {
            project: &project,
            tasks: &tasks,
            members: &members,
            this_computer: state.this_computer,
            computer_names: &computer_names,
        },
    ))
}

/// The coordinator of a project: whether and where it runs, its limit, the
/// budgets against what was spent, what waits, and its latest record.
#[tauri::command]
pub async fn coordinator(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<CoordinatorDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    let coordinator = state.coordinator.state(project_id)?;
    dto_of(&state, project_id, &coordinator)
}

/// Switches the coordinator of a project on, on this computer, or off,
/// wherever it runs.
#[tauri::command]
pub async fn switch_coordinator(
    state: State<'_, AppState>,
    project_id: String,
    on: bool,
) -> Result<CoordinatorDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    let coordinator = if on {
        state.coordinator.switch_on(project_id)?
    } else {
        state.coordinator.switch_off(project_id)?
    };
    state.coordinator_nudge.nudge(project_id);
    dto_of(&state, project_id, &coordinator)
}

/// Sets how many specialists the coordinator of a project lets work at
/// once.
#[tauri::command]
pub async fn set_coordinator_limit(
    state: State<'_, AppState>,
    project_id: String,
    limit: i64,
) -> Result<CoordinatorDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    let coordinator = state.coordinator.set_limit(project_id, limit)?;
    state.coordinator_nudge.nudge(project_id);
    dto_of(&state, project_id, &coordinator)
}

/// Sets the budget of a project, a member or a task in tokens, or takes it
/// away with null.
#[tauri::command]
pub async fn set_budget(
    state: State<'_, AppState>,
    project_id: String,
    scope: BudgetScopeInput,
    tokens: Option<u64>,
) -> Result<CoordinatorDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    let coordinator = state
        .coordinator
        .set_budget(project_id, scope.parse()?, tokens)?;
    state.coordinator_nudge.nudge(project_id);
    dto_of(&state, project_id, &coordinator)
}

/// Answers the question about a used-up budget: raise it by `raise_by`
/// tokens, or, with null, leave the work against it stopped.
#[tauri::command]
pub async fn answer_budget(
    state: State<'_, AppState>,
    project_id: String,
    scope: BudgetScopeInput,
    raise_by: Option<u64>,
) -> Result<CoordinatorDto, CommandError> {
    let project_id = parse_id(&project_id)?;
    let answer = raise_by.map_or(BudgetAnswer::LeftStopped, |by| BudgetAnswer::Raised { by });
    let coordinator = state
        .coordinator
        .answer_budget(project_id, scope.parse()?, answer)?;
    state.coordinator_nudge.nudge(project_id);
    dto_of(&state, project_id, &coordinator)
}

/// The questions about used-up budgets that wait for the manager, in every
/// project that is not put away, the one asked first first.
#[tauri::command]
pub async fn budget_questions(
    state: State<'_, AppState>,
) -> Result<Vec<BudgetQuestionDto>, CommandError> {
    Ok(state
        .coordinator
        .budget_questions()?
        .iter()
        .map(BudgetQuestionDto::from)
        .collect())
}
