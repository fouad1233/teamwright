//! The database file and the two ways of using it: reading, and writing in a
//! logged transaction.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use teamwright_application::{
    ExchangedChange, ExchangedOperation, StorageError, SyncNotice, TakeInReport,
};
use teamwright_domain::ComputerId;

use crate::codec;
use crate::error::DatabaseError;
use crate::migrations;
use crate::notices;
use crate::operations::{HybridLogicalClock, Operation, OperationRecorder, Stamp};
use crate::take_in;

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
    /// The file, or nothing for a database in memory.
    path: Option<PathBuf>,
}

/// What a write needs exclusive access to.
#[derive(Debug)]
struct State {
    connection: Connection,
    clock: HybridLogicalClock,
    /// The identity of this computer. It changes only when the manager
    /// gives the computer a new one.
    device_id: ComputerId,
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
        let connection = Connection::open(path.as_ref()).map_err(DatabaseError::from)?;
        let mut database = Self::prepare(connection)?;
        database.path = Some(path.as_ref().to_path_buf());
        Ok(database)
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
            state: Mutex::new(State {
                connection,
                clock,
                device_id,
            }),
            path: None,
        })
    }

    /// The identity of this computer. It is created when the database is
    /// first opened, and changes only when the manager gives the computer a
    /// new one because its data was copied to another computer.
    #[must_use]
    pub fn device_id(&self) -> ComputerId {
        self.lock().device_id
    }

    /// The file of the database, or nothing for one in memory.
    pub(crate) fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Gives this computer a new identity, in one transaction with `also`,
    /// which is handed the old and the new identity. Every change from then
    /// on carries the new identity.
    pub(crate) fn change_identity(
        &self,
        also: impl FnOnce(&Transaction<'_>, ComputerId, ComputerId) -> Result<(), DatabaseError>,
    ) -> Result<ComputerId, StorageError> {
        let mut state = self.lock();
        let old = state.device_id;
        let new = ComputerId::generate();
        let transaction = state
            .connection
            .transaction()
            .map_err(DatabaseError::from)?;
        transaction
            .execute(
                "UPDATE meta SET value = :value WHERE key = :key",
                named_params! { ":value": new.to_string(), ":key": DEVICE_ID_KEY },
            )
            .map_err(DatabaseError::from)?;
        also(&transaction, old, new)?;
        transaction.commit().map_err(DatabaseError::from)?;
        state.device_id = new;
        Ok(new)
    }

    /// Copies the whole database into a new file, consistently, while it is
    /// in use.
    pub(crate) fn copy_to(&self, target: &Path) -> Result<(), StorageError> {
        let state = self.lock();
        state
            .connection
            .execute("VACUUM INTO ?1", [target.to_string_lossy().into_owned()])
            .map_err(DatabaseError::from)?;
        Ok(())
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

    /// The operations this computer made after the one numbered `after`, in
    /// the order it made them, as they go to other computers. The folders of
    /// the projects never leave the computer and are not among them.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    pub fn operations_to_send(&self, after: u64) -> Result<Vec<ExchangedOperation>, StorageError> {
        let this_computer = self.device_id();
        self.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT device_sequence, stamp, entity, entity_id, payload, field_stamps, seen
                 FROM operations
                 WHERE device_id = :device_id AND device_sequence > :after
                   AND entity <> 'project_places'
                 ORDER BY device_sequence",
            )?;
            let after = i64::try_from(after).unwrap_or(i64::MAX);
            let rows = statement
                .query_map(
                    named_params! {
                        ":device_id": this_computer.to_string(),
                        ":after": after,
                    },
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            rows.into_iter()
                .map(
                    |(sequence, stamp, entity, entity_id, payload, field_stamps, seen)| {
                        let sequence = u64::try_from(sequence).map_err(|reason| {
                            DatabaseError::damaged(
                                "the number of an operation",
                                &sequence.to_string(),
                                reason,
                            )
                        })?;
                        Ok(ExchangedOperation {
                            computer: this_computer,
                            sequence,
                            stamp,
                            entity,
                            entity_id,
                            change: match payload {
                                Some(state) => ExchangedChange::Upsert {
                                    state,
                                    field_stamps,
                                },
                                None => ExchangedChange::Delete { seen },
                            },
                        })
                    },
                )
                .collect()
        })
    }

    /// Takes in operations of other computers, in one transaction: each is
    /// stored in the log once, merged field by field into what this
    /// computer has, and written to the tables; what waits for something
    /// that has not arrived yet is tried again later; and what breaks a
    /// rule across records is repaired with logged changes of this
    /// computer. Taking in the same operations again changes nothing.
    ///
    /// # Errors
    ///
    /// Returns an error, and keeps nothing of the batch, if the database
    /// fails. An operation whose state is damaged does not fail the batch:
    /// it is kept in the log and never applied.
    pub fn take_in(&self, operations: &[ExchangedOperation]) -> Result<TakeInReport, StorageError> {
        self.write(|transaction, recorder| take_in::take_in(transaction, recorder, operations))
    }

    /// The notices syncing keeps for the manager, the newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if they cannot be read.
    pub fn notices(&self) -> Result<Vec<SyncNotice>, StorageError> {
        self.read(notices::all)
    }

    /// Forgets a notice. A notice that is not kept is ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be removed.
    pub fn dismiss_notice(&self, id: i64) -> Result<(), StorageError> {
        // A notice is bookkeeping of syncing: nothing is logged.
        self.write(|transaction, _| notices::dismiss(transaction, id))
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
        let State {
            connection,
            clock,
            device_id,
        } = &mut *state;

        let transaction = connection.transaction().map_err(DatabaseError::from)?;
        let value = {
            let mut operations = OperationRecorder::new(&transaction, *device_id, clock);
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
