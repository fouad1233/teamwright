//! What the shell does when something is learned about the limit of the
//! Claude plan ([decision 0040](../../../../docs/decisions/0040-the-limit-of-the-plan.md)):
//! it tells the interface, sends one notification when the limit is
//! reached and one when it has reset, and wakes once at the moment of the
//! reset to look. Nothing polls: a reset that is not known is not waited
//! for, and is learned from the next turn instead.
//!
//! What the limit means is decided in the application crate
//! (`teamwright_application::plan_limit`).

use std::thread;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter, Runtime};
use teamwright_application::{NotificationService, PlanLimitChange, PlanLimitService, PlanReport};
use teamwright_domain::Timestamp;

use crate::dto::PlanLimitDto;
use crate::events;

/// The event that says what is known about the plan changed. Its payload
/// is the whole of it (`PlanLimitDto`).
const PLAN_LIMIT_CHANGED: &str = "plan-limit-changed";

/// How long after the moment of the reset the shell looks, so that the
/// clocks of the program and of the plan's servers agree that it passed.
const AFTER_THE_RESET: Duration = Duration::from_secs(5);

/// Takes what a turn told about the plan, tells the interface, and tells
/// the manager if the limit was reached or lifted.
pub fn take<R: Runtime>(
    app: &AppHandle<R>,
    plan_limit: &PlanLimitService,
    notifications: &NotificationService,
    report: &PlanReport,
) {
    // What cannot be kept is learned again from the next turn.
    let Ok(change) = plan_limit.take(report) else {
        return;
    };
    tell(app, plan_limit, notifications, &change);
    if matches!(change, PlanLimitChange::Reached(_)) {
        wake_at_the_reset(app, plan_limit, notifications);
    }
}

/// Wakes once at the moment the reached limit resets, if that is known, and
/// tells what came of it. Called when the limit is reached and when the
/// program starts.
pub fn wake_at_the_reset<R: Runtime>(
    app: &AppHandle<R>,
    plan_limit: &PlanLimitService,
    notifications: &NotificationService,
) {
    let Ok(Some(resets_at)) = plan_limit.next_reset() else {
        return;
    };
    let app = app.clone();
    let plan_limit = plan_limit.clone();
    let notifications = notifications.clone();
    thread::spawn(move || {
        thread::sleep(time_until(resets_at) + AFTER_THE_RESET);
        if let Ok(change) = plan_limit.look_at_reset() {
            tell(&app, &plan_limit, &notifications, &change);
        }
        // A reset that moved later while this thread slept has its own
        // thread, started when it became known.
    });
}

/// How long until a moment, or nothing if it has passed.
fn time_until(moment: Timestamp) -> Duration {
    (moment - Utc::now()).to_std().unwrap_or_default()
}

/// Tells the interface what is known now, and sends the notification of a
/// change if the manager allows that kind and the window is not in front.
fn tell<R: Runtime>(
    app: &AppHandle<R>,
    plan_limit: &PlanLimitService,
    notifications: &NotificationService,
    change: &PlanLimitChange,
) {
    if let Ok(standing) = plan_limit.standing() {
        // A window that is closing cannot be told; it asks again.
        let _ = app.emit(PLAN_LIMIT_CHANGED, PlanLimitDto::from(&standing));
    }
    let Some(notification) = plan_limit.notification_of(change) else {
        return;
    };
    let allowed = notifications
        .choice()
        .is_ok_and(|choice| choice.sends(notification.kind));
    if allowed && !events::window_is_in_front(app) {
        // A notification that cannot be shown has nobody to be reported
        // to; the window says the same.
        let _ = events::send(app, &notification, None);
    }
}
