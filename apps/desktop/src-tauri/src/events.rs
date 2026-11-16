//! Where what happened in a project goes once the shell learns of it. The
//! work of the specialists tells the end of a turn through its listener
//! (`state::EventToInterface`); the commands of sprints tell a sprint that
//! was started or completed. Today the events go to the notifications of
//! the operating system; other listeners join them here.
//!
//! What an event is worth is decided in the application crate
//! (`teamwright_application::notifications`). The shell only knows whether
//! its window is in front and how to send.

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::{ActionPerformed, NotificationExt};
use teamwright_application::{
    Agent, Happening, Notification, NotificationKind, NotificationService, ProjectEvent,
};
use teamwright_domain::MemberId;

use crate::dto::NotificationPressedDto;
use crate::error::CommandError;

/// The label of the one window of the program.
const MAIN_WINDOW: &str = "main";

/// The event the interface hears when the manager pressed a notification.
/// Its payload is a `NotificationPressedDto`.
const NOTIFICATION_PRESSED: &str = "notification-pressed";

/// Tells the listeners outside the window what happened in a project. A
/// notification is sent only if the event is worth one with the choice of
/// this computer, and only while the window is not in front: in front, the
/// manager sees the change itself.
pub fn tell<R: Runtime>(
    app: &AppHandle<R>,
    notifications: &NotificationService,
    event: &ProjectEvent,
) {
    // A choice that cannot be read sends nothing: silence is the default.
    let Ok(Some(notification)) = notifications.notification_for(event) else {
        return;
    };
    if window_is_in_front(app) {
        return;
    }
    // A notification that cannot be shown has nobody to be reported to;
    // the change it was about is in the window.
    let _ = send(app, &notification, Some(&PressTarget::of(event)));
}

/// Sends a notification through the plugin. `target` is what pressing it
/// opens.
pub fn send<R: Runtime>(
    app: &AppHandle<R>,
    notification: &Notification,
    target: Option<&PressTarget>,
) -> Result<(), CommandError> {
    let mut builder = app
        .notification()
        .builder()
        .title(&notification.title)
        .body(&notification.body)
        .extra(KIND, kind_name(notification.kind));
    if let Some(target) = target {
        builder = builder.extra(PROJECT, &target.project_id);
        if let Some(task_id) = &target.task_id {
            builder = builder.extra(TASK, task_id);
        }
        if let Some(member_id) = &target.member_id {
            builder = builder.extra(MEMBER, member_id);
        }
        if target.is_with_advisor {
            builder = builder.extra(ADVISOR, "yes");
        }
    }
    builder.show().map_err(|error| {
        CommandError::not_available(format!("the notification could not be sent: {error}"))
    })
}

/// Lets the shell hear when the manager presses a notification: the window
/// comes to the front, and the interface is told what it was about.
pub fn listen_for_presses<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    // On desktop this never fails; a notification that is pressed without
    // the handler only does what the system does with it.
    let _ = app.notification().on_action(move |performed| {
        pressed(&handle, performed);
    });
}

/// Brings the window to the front and tells the interface what the
/// notification was about.
fn pressed<R: Runtime>(app: &AppHandle<R>, performed: &ActionPerformed) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // Each step may fail on a platform that does not allow it; the next
        // still helps.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let extra = performed
        .notification()
        .map(|notification| notification.extra().clone())
        .unwrap_or_default();
    let text = |key: &str| {
        extra
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    };
    let _ = app.emit(
        NOTIFICATION_PRESSED,
        NotificationPressedDto {
            kind: text(KIND),
            project_id: text(PROJECT),
            task_id: text(TASK),
            member_id: text(MEMBER),
            is_with_advisor: text(ADVISOR).is_some(),
        },
    );
}

/// Whether the window of the program is in front. A window that cannot be
/// asked counts as not in front, so that nothing is missed.
fn window_is_in_front<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window(MAIN_WINDOW)
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false)
}

/// The keys of what a notification carries to be opened when pressed.
const KIND: &str = "kind";
const PROJECT: &str = "projectId";
const TASK: &str = "taskId";
const MEMBER: &str = "memberId";
const ADVISOR: &str = "advisor";

/// The name of a kind on the wire.
pub const fn kind_name(kind: NotificationKind) -> &'static str {
    match kind {
        NotificationKind::Waiting => "waiting",
        NotificationKind::Finished => "finished",
        NotificationKind::Stopped => "stopped",
    }
}

/// What pressing a notification opens.
pub struct PressTarget {
    project_id: String,
    task_id: Option<String>,
    member_id: Option<String>,
    is_with_advisor: bool,
}

impl PressTarget {
    fn of(event: &ProjectEvent) -> Self {
        let (task, member_id, is_with_advisor) = match &event.happening {
            Happening::TaskFinished {
                task, specialist, ..
            } => (Some(task), member_of(specialist), false),
            Happening::WaitsForManager { agent, task, .. }
            | Happening::AgentStopped { agent, task, .. } => (
                task.as_ref(),
                member_of(agent),
                matches!(agent, Agent::Advisor),
            ),
            Happening::SprintStarted { .. } | Happening::SprintCompleted { .. } => {
                (None, None, false)
            }
        };
        Self {
            project_id: event.project_id.to_string(),
            task_id: task.map(|task| task.task_id.to_string()),
            member_id: member_id.map(|id| id.to_string()),
            is_with_advisor,
        }
    }
}

const fn member_of(agent: &Agent) -> Option<MemberId> {
    match agent {
        Agent::Specialist { member_id, .. } => *member_id,
        Agent::Advisor => None,
    }
}
