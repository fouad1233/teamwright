//! The choice of this computer about looking for newer versions, kept in
//! `meta`. It is a fact about this computer: not logged, so never synced
//! ([decision 0043](../../../docs/decisions/0043-a-notice-of-newer-versions.md)).

use std::sync::Arc;

use chrono::{DateTime, SecondsFormat};
use rusqlite::{Connection, OptionalExtension, named_params};
use teamwright_application::{StorageError, UpdateChoice, UpdateChoiceStore};
use teamwright_domain::{Release, Timestamp, Version, WebAddress};

use crate::database::Database;
use crate::error::DatabaseError;

/// Whether the program looks by itself: `on`, or nothing.
const ON: &str = "update_look";
/// When it last looked.
const LAST_LOOK: &str = "update_last_look";
/// Whether the last look got no answer: `failed`, or nothing.
const FAILED: &str = "update_look_failed";
/// The version of the release the last answer offered.
const OFFERED_VERSION: &str = "update_offered_version";
/// Its page.
const OFFERED_ADDRESS: &str = "update_offered_address";
/// Whether it is a pre-release: `yes`, or nothing.
const OFFERED_PRE: &str = "update_offered_pre";

/// The choice about newer versions in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteUpdateChoice {
    database: Arc<Database>,
}

impl SqliteUpdateChoice {
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

fn clear_meta(connection: &Connection, key: &str) -> Result<(), DatabaseError> {
    connection.execute(
        "DELETE FROM meta WHERE key = :key",
        named_params! { ":key": key },
    )?;
    Ok(())
}

/// The release that was offered, if what was kept still reads.
fn offered(connection: &Connection) -> Result<Option<Release>, DatabaseError> {
    let (Some(version), Some(address)) = (
        meta(connection, OFFERED_VERSION)?,
        meta(connection, OFFERED_ADDRESS)?,
    ) else {
        return Ok(None);
    };
    let is_pre_release = meta(connection, OFFERED_PRE)?.as_deref() == Some("yes");
    Ok(Version::parse(&version)
        .zip(WebAddress::parse(&address, "release").ok())
        .map(|(version, address)| Release {
            version,
            is_pre_release,
            address,
        }))
}

impl UpdateChoiceStore for SqliteUpdateChoice {
    fn choice(&self) -> Result<UpdateChoice, StorageError> {
        self.database.read(|connection| {
            let last_look = meta(connection, LAST_LOOK)?
                .and_then(|text| DateTime::parse_from_rfc3339(&text).ok())
                .map(|moment| moment.to_utc());
            Ok(UpdateChoice {
                is_on: meta(connection, ON)?.as_deref() == Some("on"),
                last_look,
                last_look_failed: meta(connection, FAILED)?.as_deref() == Some("failed"),
                offered: offered(connection)?,
            })
        })
    }

    fn set_on(&self, is_on: bool) -> Result<(), StorageError> {
        // Written without the recorder: the choice is not logged.
        self.database.write(|transaction, _| {
            if is_on {
                set_meta(transaction, ON, "on")
            } else {
                clear_meta(transaction, ON)
            }
        })
    }

    fn record_look(
        &self,
        at: Timestamp,
        answer: Option<Option<&Release>>,
    ) -> Result<(), StorageError> {
        self.database.write(|transaction, _| {
            set_meta(
                transaction,
                LAST_LOOK,
                &at.to_rfc3339_opts(SecondsFormat::Millis, true),
            )?;
            let Some(answer) = answer else {
                // No answer: what was offered before stays.
                return set_meta(transaction, FAILED, "failed");
            };
            clear_meta(transaction, FAILED)?;
            match answer {
                Some(release) => {
                    set_meta(transaction, OFFERED_VERSION, &release.version.text())?;
                    set_meta(transaction, OFFERED_ADDRESS, release.address.as_str())?;
                    if release.is_pre_release {
                        set_meta(transaction, OFFERED_PRE, "yes")
                    } else {
                        clear_meta(transaction, OFFERED_PRE)
                    }
                }
                None => {
                    clear_meta(transaction, OFFERED_VERSION)?;
                    clear_meta(transaction, OFFERED_ADDRESS)?;
                    clear_meta(transaction, OFFERED_PRE)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn release() -> Release {
        Release {
            version: Version::parse("v0.2.0").unwrap(),
            is_pre_release: true,
            address: WebAddress::parse(
                "https://github.com/fouad1233/teamwright/releases/tag/v0.2.0",
                "release",
            )
            .unwrap(),
        }
    }

    #[test]
    fn nothing_is_chosen_until_the_manager_chooses_and_nothing_is_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqliteUpdateChoice::new(database.clone());
        assert_eq!(store.choice().unwrap(), UpdateChoice::default());

        store.set_on(true).unwrap();
        assert!(store.choice().unwrap().is_on);
        store.set_on(false).unwrap();
        assert!(!store.choice().unwrap().is_on);
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }

    #[test]
    fn a_look_is_kept_with_what_it_found_and_a_failed_one_keeps_what_was_found() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let store = SqliteUpdateChoice::new(database.clone());
        let first = chrono::Utc.with_ymd_and_hms(2026, 11, 21, 8, 0, 0).unwrap();
        let second = chrono::Utc.with_ymd_and_hms(2026, 11, 22, 8, 0, 0).unwrap();

        store.record_look(first, Some(Some(&release()))).unwrap();
        let kept = store.choice().unwrap();
        assert_eq!(kept.last_look, Some(first));
        assert!(!kept.last_look_failed);
        assert_eq!(kept.offered, Some(release()));

        store.record_look(second, None).unwrap();
        let kept = store.choice().unwrap();
        assert_eq!(kept.last_look, Some(second));
        assert!(kept.last_look_failed);
        assert_eq!(kept.offered, Some(release()));

        store.record_look(second, Some(None)).unwrap();
        let kept = store.choice().unwrap();
        assert!(!kept.last_look_failed);
        assert_eq!(kept.offered, None);
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }
}
