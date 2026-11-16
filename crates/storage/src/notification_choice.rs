//! The choice of this computer about notifications, kept in `meta`. It is
//! a fact about this computer: not logged, so never synced
//! ([decision 0026](../../../docs/decisions/0026-notifications.md)).

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, named_params};
use teamwright_application::{
    NotificationChoice, NotificationChoiceStore, NotificationPermission, StorageError,
};

use crate::database::Database;
use crate::error::DatabaseError;

/// The key of whether notifications are allowed.
const PERMISSION: &str = "notifications";
/// The key of whether a notification is sent when someone waits.
const WAITING: &str = "notify_waiting";
/// The key of whether a notification is sent when work is finished.
const FINISHED: &str = "notify_finished";
/// The key of whether a notification is sent when someone stopped.
const STOPPED: &str = "notify_stopped";

/// The choice about notifications in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteNotificationChoice {
    database: Arc<Database>,
}

impl SqliteNotificationChoice {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

fn meta(connection: &Connection, key: &str) -> Result<Option<String>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT value FROM meta WHERE key = :key",
            named_params! { ":key": key },
            |row| row.get(0),
        )
        .optional()?)
}

fn set_meta(connection: &Connection, key: &str, value: &str) -> Result<(), DatabaseError> {
    connection.execute(
        "INSERT INTO meta (key, value) VALUES (:key, :value)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        named_params! { ":key": key, ":value": value },
    )?;
    Ok(())
}

/// A kind is on unless it was turned off.
fn is_on(connection: &Connection, key: &str) -> Result<bool, DatabaseError> {
    Ok(meta(connection, key)?.as_deref() != Some("off"))
}

const fn on_or_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

impl NotificationChoiceStore for SqliteNotificationChoice {
    fn choice(&self) -> Result<NotificationChoice, StorageError> {
        self.database.read(|connection| {
            let permission = match meta(connection, PERMISSION)?.as_deref() {
                Some("allowed") => NotificationPermission::Allowed,
                Some("declined") => NotificationPermission::Declined,
                _ => NotificationPermission::NotAsked,
            };
            Ok(NotificationChoice {
                permission,
                waiting: is_on(connection, WAITING)?,
                finished: is_on(connection, FINISHED)?,
                stopped: is_on(connection, STOPPED)?,
            })
        })
    }

    fn set_choice(&self, choice: &NotificationChoice) -> Result<(), StorageError> {
        let permission = match choice.permission {
            NotificationPermission::NotAsked => "not_asked",
            NotificationPermission::Allowed => "allowed",
            NotificationPermission::Declined => "declined",
        };
        // Written without the recorder: the choice is not logged.
        self.database.write(|transaction, _| {
            set_meta(transaction, PERMISSION, permission)?;
            set_meta(transaction, WAITING, on_or_off(choice.waiting))?;
            set_meta(transaction, FINISHED, on_or_off(choice.finished))?;
            set_meta(transaction, STOPPED, on_or_off(choice.stopped))
        })
    }
}

#[cfg(test)]
mod tests {
    use teamwright_application::NotificationKind;

    use super::*;

    #[test]
    fn the_choice_is_kept_and_not_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqliteNotificationChoice::new(database.clone());
        assert_eq!(store.choice().unwrap(), NotificationChoice::default());

        let choice = NotificationChoice {
            permission: NotificationPermission::Allowed,
            ..NotificationChoice::default()
        }
        .with_kind(NotificationKind::Finished, false);
        store.set_choice(&choice).unwrap();

        assert_eq!(store.choice().unwrap(), choice);
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }
}
