//! Whether the manager switched command permissions on, on this computer,
//! kept in `meta`. It is a fact about this computer: not logged, so never
//! synced, and off until switched on ([decision
//! 0038](../../../docs/decisions/0038-permission-requests-routed-to-the-manager.md),
//! note of 22 November 2026).

use std::sync::Arc;

use rusqlite::{OptionalExtension, named_params};
use teamwright_application::{CommandChoiceStore, StorageError};

use crate::database::Database;

/// The key of the choice.
const KEY: &str = "command_permissions";

/// The value of the choice when switched on. Anything else, or nothing, is
/// off.
const ON: &str = "on";

/// The choice about command permissions in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteCommandChoice {
    database: Arc<Database>,
}

impl SqliteCommandChoice {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl CommandChoiceStore for SqliteCommandChoice {
    fn commands_switched_on(&self) -> Result<bool, StorageError> {
        self.database.read(|connection| {
            let value: Option<String> = connection
                .query_row(
                    "SELECT value FROM meta WHERE key = :key",
                    named_params! { ":key": KEY },
                    |row| row.get(0),
                )
                .optional()?;
            Ok(value.as_deref() == Some(ON))
        })
    }

    fn switch_commands(&self, on: bool) -> Result<(), StorageError> {
        // Written without the recorder: the choice is not logged.
        self.database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO meta (key, value) VALUES (:key, :value)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                named_params! { ":key": KEY, ":value": if on { ON } else { "off" } },
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_is_off_until_switched_on_and_is_not_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqliteCommandChoice::new(database.clone());
        assert!(!store.commands_switched_on().unwrap());

        store.switch_commands(true).unwrap();
        assert!(store.commands_switched_on().unwrap());
        store.switch_commands(false).unwrap();
        assert!(!store.commands_switched_on().unwrap());

        assert!(database.operations_to_send(0).unwrap().is_empty());
    }
}
