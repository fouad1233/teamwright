//! Commands for the notifications of the operating system
//! ([decision 0026](../../../../../docs/decisions/0026-notifications.md)):
//! the choice of this computer, and a notification to try them.

use tauri::{AppHandle, State};

use crate::dto::{NotificationChoiceDto, parse_notification_kind};
use crate::error::CommandError;
use crate::events;
use crate::state::AppState;

/// What the manager chose about notifications on this computer.
#[tauri::command]
pub async fn notification_choice(
    state: State<'_, AppState>,
) -> Result<NotificationChoiceDto, CommandError> {
    Ok(state.notifications.choice()?.into())
}

/// Allows notifications on this computer.
#[tauri::command]
pub async fn allow_notifications(
    state: State<'_, AppState>,
) -> Result<NotificationChoiceDto, CommandError> {
    Ok(state.notifications.allow()?.into())
}

/// Turns notifications off on this computer, or says no to the question
/// whether to allow them. The program does not ask again.
#[tauri::command]
pub async fn decline_notifications(
    state: State<'_, AppState>,
) -> Result<NotificationChoiceDto, CommandError> {
    Ok(state.notifications.decline()?.into())
}

/// Turns one kind of notification on or off: `waiting`, `finished` or
/// `stopped`.
#[tauri::command]
pub async fn set_notification_kind(
    state: State<'_, AppState>,
    kind: String,
    on: bool,
) -> Result<NotificationChoiceDto, CommandError> {
    let kind = parse_notification_kind(&kind)?;
    Ok(state.notifications.set_kind(kind, on)?.into())
}

/// Sends a notification to try them, whether the window is in front or
/// not. Refused while they are not allowed (`notifications_off`).
#[tauri::command]
pub async fn send_test_notification(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    let notification = state.notifications.test_notification()?;
    events::send(&app, &notification, None)
}
