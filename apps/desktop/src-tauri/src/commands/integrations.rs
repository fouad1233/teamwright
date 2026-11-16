//! Commands for the integrations of a project: its GitHub repository, its
//! Slack channel and the web addresses that listen for its events.

use tauri::State;

use crate::dto::{DeliveriesDto, DeliveryDto, ProjectIntegrationsDto, parse_event_kind, parse_id};
use crate::error::CommandError;
use crate::state::AppState;

/// The integrations of a project on this computer.
#[tauri::command]
pub async fn project_integrations(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state.integrations.of(parse_id(&project_id)?)?.into())
}

/// Names the GitHub repository of a project, or, without a name, reads it
/// from the work folder again.
#[tauri::command]
pub async fn set_repository(
    state: State<'_, AppState>,
    project_id: String,
    name: Option<String>,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .set_repository(parse_id(&project_id)?, name.as_deref())?
        .into())
}

/// Sets the address of the project's Slack channel, or takes it away.
#[tauri::command]
pub async fn set_slack_address(
    state: State<'_, AppState>,
    project_id: String,
    address: Option<String>,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .set_slack_address(parse_id(&project_id)?, address.as_deref())?
        .into())
}

/// Tells, or stops telling, a kind of event in the Slack channel.
#[tauri::command]
pub async fn set_slack_event(
    state: State<'_, AppState>,
    project_id: String,
    kind: String,
    on: bool,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .set_slack_event(parse_id(&project_id)?, parse_event_kind(&kind)?, on)?
        .into())
}

/// Adds a web address that listens for the events of a project.
#[tauri::command]
pub async fn add_web_address(
    state: State<'_, AppState>,
    project_id: String,
    address: String,
    secret: Option<String>,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .add_web_address(parse_id(&project_id)?, &address, secret.as_deref())?
        .into())
}

/// Takes away the web address at a position.
#[tauri::command]
pub async fn remove_web_address(
    state: State<'_, AppState>,
    project_id: String,
    index: usize,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .remove_web_address(parse_id(&project_id)?, index)?
        .into())
}

/// Sends, or stops sending, a kind of event to the web address at a
/// position.
#[tauri::command]
pub async fn set_web_address_event(
    state: State<'_, AppState>,
    project_id: String,
    index: usize,
    kind: String,
    on: bool,
) -> Result<ProjectIntegrationsDto, CommandError> {
    Ok(state
        .integrations
        .set_web_address_event(parse_id(&project_id)?, index, parse_event_kind(&kind)?, on)?
        .into())
}

/// What was delivered for a project since the program started: the last
/// failure of its channel, and the last deliveries of each address.
#[tauri::command]
pub async fn integration_deliveries(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DeliveriesDto, CommandError> {
    Ok(DeliveriesDto::from(
        &state.deliveries.deliveries(parse_id(&project_id)?)?,
    ))
}

/// Sends a line to the project's Slack channel to try it. It waits for the
/// answer, which comes within the time limit of the sending.
#[tauri::command]
pub async fn test_channel(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DeliveryDto, CommandError> {
    Ok(DeliveryDto::from(
        &state.deliveries.test_channel(parse_id(&project_id)?)?,
    ))
}

/// Sends the event `test` to the web address at a position.
#[tauri::command]
pub async fn test_web_address(
    state: State<'_, AppState>,
    project_id: String,
    index: usize,
) -> Result<DeliveryDto, CommandError> {
    Ok(DeliveryDto::from(
        &state
            .deliveries
            .test_address(parse_id(&project_id)?, index)?,
    ))
}
