//! The theme of this computer, kept in `meta`. Like the choice about
//! notifications it is a fact about this computer: not logged, so never
//! synced ([decision 0028](../../../docs/decisions/0028-themes.md)).

use std::sync::Arc;

use rusqlite::{OptionalExtension, named_params};
use teamwright_application::{StorageError, Theme, ThemeStore};

use crate::database::Database;

/// The key of the theme in `meta`.
const THEME: &str = "theme";

/// The theme in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteThemeStore {
    database: Arc<Database>,
}

impl SqliteThemeStore {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ThemeStore for SqliteThemeStore {
    fn theme(&self) -> Result<Theme, StorageError> {
        self.database.read(|connection| {
            let name: Option<String> = connection
                .query_row(
                    "SELECT value FROM meta WHERE key = :key",
                    named_params! { ":key": THEME },
                    |row| row.get(0),
                )
                .optional()?;
            Ok(name.map_or_else(Theme::default, |name| Theme::or_default(&name)))
        })
    }

    fn set_theme(&self, theme: Theme) -> Result<(), StorageError> {
        // Written without the recorder: the theme is not logged.
        self.database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO meta (key, value) VALUES (:key, :value)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                named_params! { ":key": THEME, ":value": theme.name() },
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_is_kept_and_not_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqliteThemeStore::new(database.clone());
        assert_eq!(store.theme().unwrap(), Theme::Lamplight);

        store.set_theme(Theme::Tide).unwrap();
        assert_eq!(store.theme().unwrap(), Theme::Tide);
        store.set_theme(Theme::Paper).unwrap();
        assert_eq!(store.theme().unwrap(), Theme::Paper);
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }
}
