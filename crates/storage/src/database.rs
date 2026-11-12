//! The database file and the two ways of using it: reading, and writing in a
//! logged transaction.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::{Connection, OptionalExtension, Transaction};
use teamwright_application::StorageError;
use teamwright_domain::ComputerId;

use crate::codec;
use crate::error::DatabaseError;
use crate::migrations;
use crate::operations::{HybridLogicalClock, Operation, OperationRecorder, Stamp};

/// The key under which the identity of this computer is kept in the `meta`
/// table.
const DEVICE_ID_KEY: &str = "device_id";

/// A Teamwright database.
///
/// One `Database` is shared by all repositories. It serializes access to the
/// SQLite connection, which is all a single-user desktop program needs.
#[derive(Debug)]
pub struct Database {
    state: Mutex<State>,
    device_id: ComputerId,
}

/// What a write needs exclusive access to.
#[derive(Debug)]
struct State {
    connection: Connection,
    clock: HybridLogicalClock,
}

impl Database {
    /// Opens the database in a file, creating the file and bringing its
    /// format up to date as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, if it was written by a
    /// newer version of the program, or if it is damaged.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path).map_err(DatabaseError::from)?;
        Ok(Self::prepare(connection)?)
    }

    /// Opens a database that lives in memory and is gone when it is dropped.
    /// Used by tests.
    ///
    /// # Errors
    ///
    /// Returns an error if SQLite cannot create the database.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory().map_err(DatabaseError::from)?;
        Ok(Self::prepare(connection)?)
    }

    /// Configures a fresh connection and reads what the program needs to
    /// know before the first query.
    fn prepare(mut connection: Connection) -> Result<Self, DatabaseError> {
        // Write-ahead logging lets readers continue while a write is in
        // progress and survives a crash in the middle of a write.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        // SQLite only enforces foreign keys when asked to.
        connection.pragma_update(None, "foreign_keys", "ON")?;
        migrations::migrate(&mut connection)?;

        let device_id = identify_device(&connection)?;
        let clock = HybridLogicalClock::continuing_after(latest_stamp(&connection)?);
        Ok(Self {
            state: Mutex::new(State { connection, clock }),
            device_id,
        })
    }

    /// The identity of this computer. It is created when the database is
    /// first opened and never changes.
    #[must_use]
    pub const fn device_id(&self) -> ComputerId {
        self.device_id
    }

    /// Reads the whole operation log, oldest operation first.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn operations(&self) -> Result<Vec<Operation>, StorageError> {
        self.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT sequence, device_id, stamp, entity, entity_id, kind, payload
                 FROM operations
                 ORDER BY sequence",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, i64>("sequence")?,
                    row.get::<_, String>("device_id")?,
                    row.get::<_, String>("stamp")?,
                    row.get::<_, String>("entity")?,
                    row.get::<_, String>("entity_id")?,
                    row.get::<_, String>("kind")?,
                    row.get::<_, Option<String>>("payload")?,
                ))
            })?;

            let mut operations = Vec::new();
            for row in rows {
                let (sequence, device_id, stamp, entity, entity_id, kind, payload) = row?;
                operations.push(Operation {
                    sequence,
                    device_id: codec::parse(&device_id)?,
                    stamp,
                    entity,
                    entity_id,
                    kind,
                    payload,
                });
            }
            Ok(operations)
        })
    }

    /// Runs a query.
    pub(crate) fn read<T>(
        &self,
        query: impl FnOnce(&Connection) -> Result<T, DatabaseError>,
    ) -> Result<T, StorageError> {
        let state = self.lock();
        Ok(query(&state.connection)?)
    }

    /// Runs a change in a transaction. Either everything the change does is
    /// stored, including the operations it logs, or nothing is.
    pub(crate) fn write<T>(
        &self,
        change: impl FnOnce(&Transaction<'_>, &mut OperationRecorder<'_>) -> Result<T, DatabaseError>,
    ) -> Result<T, StorageError> {
        let mut state = self.lock();
        let State { connection, clock } = &mut *state;

        let transaction = connection.transaction().map_err(DatabaseError::from)?;
        let value = {
            let mut operations = OperationRecorder::new(&transaction, self.device_id, clock);
            change(&transaction, &mut operations)?
        };
        transaction.commit().map_err(DatabaseError::from)?;
        Ok(value)
    }

    /// Locks the connection. A panic in another thread while it held the
    /// lock does not make the connection unusable: SQLite rolled the
    /// unfinished transaction back when it was dropped.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Reads the identity of this computer from the database, or creates and
/// stores one if the database is new.
fn identify_device(connection: &Connection) -> Result<ComputerId, DatabaseError> {
    let stored: Option<String> = connection
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            [DEVICE_ID_KEY],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(text) = stored {
        return codec::parse(&text);
    }

    let device_id = ComputerId::generate();
    connection.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)",
        [DEVICE_ID_KEY, &device_id.to_string()],
    )?;
    Ok(device_id)
}

/// Finds the latest clock stamp in the operation log. An empty log, or a
/// stamp that cannot be read, gives the start of time, from which the clock
/// simply follows the time of day.
fn latest_stamp(connection: &Connection) -> Result<Stamp, DatabaseError> {
    let latest: Option<String> =
        connection.query_row("SELECT max(stamp) FROM operations", [], |row| row.get(0))?;
    Ok(latest
        .as_deref()
        .and_then(Stamp::from_text)
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_database_file_keeps_the_identity_of_its_computer() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("teamwright.db");

        let first_identity = Database::open(&path).unwrap().device_id();
        let second_identity = Database::open(&path).unwrap().device_id();

        assert_eq!(first_identity, second_identity);
    }

    #[test]
    fn two_databases_have_different_identities() {
        let one = Database::open_in_memory().unwrap();
        let other = Database::open_in_memory().unwrap();

        assert_ne!(one.device_id(), other.device_id());
    }

    #[test]
    fn a_new_database_has_an_empty_operation_log() {
        let database = Database::open_in_memory().unwrap();

        assert!(database.operations().unwrap().is_empty());
    }

    #[test]
    fn a_change_that_fails_leaves_nothing_behind() {
        let database = Database::open_in_memory().unwrap();

        let result: Result<(), StorageError> = database.write(|transaction, _| {
            transaction.execute("INSERT INTO meta (key, value) VALUES ('half', 'done')", [])?;
            Err(DatabaseError::Damaged(
                "something went wrong afterwards".to_owned(),
            ))
        });

        assert!(result.is_err());
        let rows_left: i64 = database
            .read(|connection| {
                Ok(connection.query_row(
                    "SELECT count(*) FROM meta WHERE key = 'half'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(rows_left, 0);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let database = Database::open_in_memory().unwrap();

        let result: Result<(), StorageError> = database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO statuses (id, project_id, position, name, category, color)
                 VALUES ('s', 'no-such-project', 0, 'To do', 'todo', 'gray')",
                [],
            )?;
            Ok(())
        });

        assert!(result.is_err());
    }
}
