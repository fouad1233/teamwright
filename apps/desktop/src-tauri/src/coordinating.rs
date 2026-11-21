//! Where the coordinator acts: one thread of its own, woken by a nudge
//! whenever something happened that may change what can start
//! ([decision 0037](../../../../docs/decisions/0037-the-coordinator.md)).
//!
//! The thread waits on a channel, so it costs nothing while nothing
//! happens: nothing polls. A nudge comes when a turn ends, when a task
//! changes, when the coordinator, its limit or a budget changes, when the
//! manager answers about a budget, when syncing took in a change, and once
//! when the program starts. Acting on its own thread matters for the end
//! of a turn: that is told while the work of the specialists holds its
//! lock, and starting the next turn waits for that lock.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use teamwright_domain::ProjectId;

use crate::state::AppState;

/// The event the interface hears after the coordinator of a project acted.
/// Its payload names the project (`CoordinatorChangedDto`).
const COORDINATOR_CHANGED: &str = "coordinator-changed";

/// The payload of `coordinator-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CoordinatorChangedDto {
    project_id: String,
}

/// What wakes the coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nudge {
    /// Something happened in one project.
    Project(ProjectId),
    /// The program started: every project whose coordinator runs here.
    Everywhere,
}

/// Wakes the coordinator. Cloned into whatever learns that something
/// happened.
#[derive(Debug, Clone)]
pub struct CoordinatorNudge {
    sender: Sender<Nudge>,
}

impl CoordinatorNudge {
    /// A nudge and the inbox the coordinator's thread reads.
    pub fn new() -> (Self, Receiver<Nudge>) {
        let (sender, inbox) = channel();
        (Self { sender }, inbox)
    }

    /// Something happened in a project.
    pub fn nudge(&self, project_id: ProjectId) {
        // The thread is gone only when the program closes.
        let _ = self.sender.send(Nudge::Project(project_id));
    }

    /// The program started.
    pub fn nudge_everywhere(&self) {
        let _ = self.sender.send(Nudge::Everywhere);
    }
}

/// Starts the coordinator's thread. It reads nudges until the program
/// closes, and acts for each project once, however many nudges came for it
/// meanwhile.
pub fn start_coordinating(app: AppHandle, inbox: Receiver<Nudge>) {
    // Without the thread the coordinator does nothing by itself; the
    // program goes on, and the manager starts work by hand.
    let _ = thread::Builder::new()
        .name("coordinator".to_owned())
        .spawn(move || {
            while let Ok(first) = inbox.recv() {
                let mut nudges = vec![first];
                nudges.extend(inbox.try_iter());
                act(&app, &nudges);
            }
        });
}

fn act(app: &AppHandle, nudges: &[Nudge]) {
    // Before the state is managed nothing can act; the program nudges
    // every project once it is.
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let mut projects: Vec<ProjectId> = Vec::new();
    for nudge in nudges {
        let more = match nudge {
            Nudge::Project(project_id) => vec![*project_id],
            Nudge::Everywhere => state.coordinator.running_here().unwrap_or_default(),
        };
        for project_id in more {
            if !projects.contains(&project_id) {
                projects.push(project_id);
            }
        }
    }
    for project_id in projects {
        // What went wrong in storage is tried again at the next nudge; what
        // the coordinator did is in its record.
        let _ = state.coordinator.act(project_id);
        let _ = app.emit(
            COORDINATOR_CHANGED,
            CoordinatorChangedDto {
                project_id: project_id.to_string(),
            },
        );
    }
}
