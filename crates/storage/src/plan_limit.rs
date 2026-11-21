//! What this computer knows about the limits of its Claude plan, kept in
//! `meta` as one JSON value. Like the choice about notifications and the
//! theme it is a fact about this computer, and here also about the Claude
//! account signed in on it: not logged, so never synced
//! ([decision 0040](../../../docs/decisions/0040-the-limit-of-the-plan.md)).
//!
//! What cannot be read back is forgotten, not refused: the next turn
//! reports the plan again.

use std::sync::Arc;

use rusqlite::{OptionalExtension, named_params};
use serde::{Deserialize, Serialize};
use teamwright_application::{
    PlanLimitState, PlanLimitStore, PlanWindow, PlanWindowKind, ReachedLimit, StorageError,
};

use crate::codec;
use crate::database::Database;

/// The key of what is known about the plan in `meta`.
const PLAN_LIMIT: &str = "plan_limit";

/// What is known about the plan in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqlitePlanLimitStore {
    database: Arc<Database>,
}

impl SqlitePlanLimitStore {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

/// The stored form.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PlanLimitRecord {
    #[serde(default)]
    reached: Option<ReachedRecord>,
    #[serde(default)]
    windows: Vec<WindowRecord>,
    #[serde(default)]
    reported_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReachedRecord {
    at: String,
    #[serde(default)]
    window: Option<String>,
    #[serde(default)]
    resets_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowRecord {
    kind: String,
    #[serde(default)]
    used: Option<f64>,
    #[serde(default)]
    resets_at: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

fn kind_to_text(kind: &PlanWindowKind) -> String {
    match kind {
        PlanWindowKind::FiveHours => "five_hour".to_owned(),
        PlanWindowKind::Week => "seven_day".to_owned(),
        PlanWindowKind::Other(name) => name.clone(),
    }
}

fn kind_from_text(text: &str) -> PlanWindowKind {
    match text {
        "five_hour" => PlanWindowKind::FiveHours,
        "seven_day" => PlanWindowKind::Week,
        other => PlanWindowKind::Other(other.to_owned()),
    }
}

impl PlanLimitRecord {
    fn of(state: &PlanLimitState) -> Self {
        Self {
            reached: state.reached.as_ref().map(|limit| ReachedRecord {
                at: codec::moment_to_text(limit.at),
                window: limit.window.as_ref().map(kind_to_text),
                resets_at: limit.resets_at.map(codec::moment_to_text),
            }),
            windows: state
                .windows
                .iter()
                .map(|window| WindowRecord {
                    kind: kind_to_text(&window.kind),
                    used: window.used,
                    resets_at: window.resets_at.map(codec::moment_to_text),
                    status: window.status.clone(),
                })
                .collect(),
            reported_at: state.reported_at.map(codec::moment_to_text),
        }
    }

    /// The state, with every moment that cannot be read left out.
    fn into_state(self) -> PlanLimitState {
        let moment =
            |text: Option<String>| text.and_then(|text| codec::moment_from_text(&text).ok());
        PlanLimitState {
            reached: self.reached.and_then(|limit| {
                Some(ReachedLimit {
                    at: codec::moment_from_text(&limit.at).ok()?,
                    window: limit.window.as_deref().map(kind_from_text),
                    resets_at: moment(limit.resets_at),
                })
            }),
            windows: self
                .windows
                .into_iter()
                .map(|window| PlanWindow {
                    kind: kind_from_text(&window.kind),
                    used: window.used,
                    resets_at: moment(window.resets_at),
                    status: window.status,
                })
                .collect(),
            reported_at: moment(self.reported_at),
        }
    }
}

impl PlanLimitStore for SqlitePlanLimitStore {
    fn plan_limit(&self) -> Result<PlanLimitState, StorageError> {
        self.database.read(|connection| {
            let stored: Option<String> = connection
                .query_row(
                    "SELECT value FROM meta WHERE key = :key",
                    named_params! { ":key": PLAN_LIMIT },
                    |row| row.get(0),
                )
                .optional()?;
            Ok(stored
                .and_then(|json| serde_json::from_str::<PlanLimitRecord>(&json).ok())
                .unwrap_or_default()
                .into_state())
        })
    }

    fn set_plan_limit(&self, state: &PlanLimitState) -> Result<(), StorageError> {
        let json = serde_json::to_string(&PlanLimitRecord::of(state)).map_err(|error| {
            StorageError::caused_by("what is known about the plan could not be written", error)
        })?;
        // Written without the recorder: what is known about the plan is not
        // logged.
        self.database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO meta (key, value) VALUES (:key, :value)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                named_params! { ":key": PLAN_LIMIT, ":value": json },
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    #[test]
    fn what_is_known_is_kept_whole_and_not_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqlitePlanLimitStore::new(database.clone());
        assert_eq!(store.plan_limit().unwrap(), PlanLimitState::default());

        let moment = |seconds| Utc.timestamp_opt(seconds, 0).unwrap();
        let state = PlanLimitState {
            reached: Some(ReachedLimit {
                at: moment(1_795_000_000),
                window: Some(PlanWindowKind::FiveHours),
                resets_at: Some(moment(1_795_010_000)),
            }),
            windows: vec![
                PlanWindow {
                    kind: PlanWindowKind::Week,
                    used: Some(0.25),
                    resets_at: Some(moment(1_795_500_000)),
                    status: Some("allowed".to_owned()),
                },
                PlanWindow {
                    kind: PlanWindowKind::Other("seven_day_opus".to_owned()),
                    used: None,
                    resets_at: None,
                    status: None,
                },
            ],
            reported_at: Some(moment(1_795_000_000)),
        };
        store.set_plan_limit(&state).unwrap();

        assert_eq!(store.plan_limit().unwrap(), state);
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }

    #[test]
    fn what_cannot_be_read_is_forgotten() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        database
            .write(|transaction, _| {
                transaction.execute(
                    "INSERT INTO meta (key, value) VALUES ('plan_limit', 'not json')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let store = SqlitePlanLimitStore::new(database);
        assert_eq!(store.plan_limit().unwrap(), PlanLimitState::default());
    }
}
