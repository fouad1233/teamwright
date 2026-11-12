//! The bookkeeping of syncing: what this computer knows of the others,
//! which files it wrote into the sync folder, and the settings. None of it
//! is logged: it describes this computer's view of the folder, as `meta`
//! does.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use teamwright_application::{
    Acceptance, ExchangedOperation, FoundComputer, KnownComputer, NoticeKind, StorageError,
    SyncLedger, SyncNotice, SyncSettings, TakeInReport,
};
use teamwright_domain::{ComputerId, Timestamp};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::notices;
use crate::operations::Stamp;
use crate::take_in;

/// The key of the chosen folder in `meta`.
const SYNC_FOLDER: &str = "sync_folder";
/// The key of the folder the written files are in.
const FILES_FOLDER: &str = "files_folder";
/// The key of the name of this computer.
const COMPUTER_NAME: &str = "computer_name";
/// The key of the number of the last operation written into the folder.
const WRITTEN_THROUGH: &str = "written_through";
/// The key that is set once the database was copied before the first
/// take-in.
const COPIED_BEFORE_SYNC: &str = "copied_before_sync";

/// The bookkeeping of syncing in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteSyncLedger {
    database: Arc<Database>,
}

impl SqliteSyncLedger {
    /// Creates a ledger that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

/// Reads a value of `meta`.
fn meta(connection: &Connection, key: &str) -> Result<Option<String>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT value FROM meta WHERE key = :key",
            named_params! { ":key": key },
            |row| row.get(0),
        )
        .optional()?)
}

/// Sets a value of `meta`, or removes it.
fn set_meta(connection: &Connection, key: &str, value: Option<&str>) -> Result<(), DatabaseError> {
    match value {
        Some(value) => connection.execute(
            "INSERT INTO meta (key, value) VALUES (:key, :value)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            named_params! { ":key": key, ":value": value },
        )?,
        None => connection.execute(
            "DELETE FROM meta WHERE key = :key",
            named_params! { ":key": key },
        )?,
    };
    Ok(())
}

/// The number of the last operation written into the folder.
fn written_through(connection: &Connection) -> Result<u64, DatabaseError> {
    Ok(meta(connection, WRITTEN_THROUGH)?
        .and_then(|text| text.parse().ok())
        .unwrap_or(0))
}

/// Turns a number of the log into the number SQLite keeps.
fn stored_number(number: u64) -> i64 {
    i64::try_from(number).unwrap_or(i64::MAX)
}

impl SyncLedger for SqliteSyncLedger {
    fn this_computer(&self) -> ComputerId {
        self.database.device_id()
    }

    fn operations_after(&self, after: u64) -> Result<Vec<ExchangedOperation>, StorageError> {
        self.database.operations_to_send(after)
    }

    fn unwritten_since(&self) -> Result<Option<Timestamp>, StorageError> {
        let me = self.database.device_id().to_string();
        self.database.read(|connection| {
            let through = stored_number(written_through(connection)?);
            let latest: Option<String> = connection.query_row(
                "SELECT max(stamp) FROM operations
                 WHERE device_id = :me AND device_sequence > :through
                   AND entity <> 'project_places'",
                named_params! { ":me": me, ":through": through },
                |row| row.get(0),
            )?;
            Ok(latest
                .as_deref()
                .and_then(Stamp::from_text)
                .and_then(|stamp| DateTime::<Utc>::from_timestamp_millis(stamp.millis())))
        })
    }

    fn take_in(
        &self,
        from: ComputerId,
        operations: &[ExchangedOperation],
        read_up_to: u64,
    ) -> Result<TakeInReport, StorageError> {
        self.database.write(|transaction, recorder| {
            let report = take_in::take_in(transaction, recorder, operations)?;
            transaction.execute(
                "UPDATE computers SET read_up_to = max(read_up_to, :read_up_to) WHERE id = :id",
                named_params! { ":read_up_to": stored_number(read_up_to), ":id": from.to_string() },
            )?;
            Ok(report)
        })
    }

    fn keep_copy_before_first_take_in(&self) -> Result<(), StorageError> {
        let copied = self
            .database
            .read(|connection| meta(connection, COPIED_BEFORE_SYNC))?;
        if copied.is_some() {
            return Ok(());
        }
        if let Some(path) = self.database.path() {
            let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
            let date = Utc::now().format("%Y-%m-%d");
            let mut target = folder.join(format!("teamwright-before-sync-{date}.db"));
            let mut number = 2;
            while target.exists() {
                target = folder.join(format!("teamwright-before-sync-{date}-{number}.db"));
                number += 1;
            }
            self.database.copy_to(&target)?;
        }
        self.database.write(|transaction, _| {
            set_meta(
                transaction,
                COPIED_BEFORE_SYNC,
                Some(&codec::moment_to_text(Utc::now())),
            )
        })
    }

    fn computers(&self) -> Result<Vec<KnownComputer>, StorageError> {
        let me = self.database.device_id().to_string();
        self.database.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, name, version, accepted, read_up_to, last_heard, clock_ahead,
                        newer_format
                 FROM computers WHERE id <> :me ORDER BY id",
            )?;
            let rows = statement
                .query_map(named_params! { ":me": me }, |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<bool>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<i64>>(6)?,
                        row.get::<_, Option<u32>>(7)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            rows.into_iter()
                .map(
                    |(id, name, version, accepted, read_up_to, last_heard, ahead, newer)| {
                        Ok(KnownComputer {
                            id: codec::parse(&id)?,
                            name,
                            version,
                            acceptance: match accepted {
                                None => Acceptance::Undecided,
                                Some(true) => Acceptance::Accepted,
                                Some(false) => Acceptance::Ignored,
                            },
                            read_up_to: u64::try_from(read_up_to).unwrap_or(0),
                            last_heard: last_heard
                                .as_deref()
                                .map(codec::moment_from_text)
                                .transpose()?,
                            clock_ahead_by_millis: ahead,
                            newer_format: newer,
                        })
                    },
                )
                .collect()
        })
    }

    fn heard_from(&self, computer: &FoundComputer) -> Result<bool, StorageError> {
        self.database.write(|transaction, _| {
            let id = computer.id.to_string();
            let added = transaction.execute(
                "INSERT INTO computers (id) VALUES (:id) ON CONFLICT (id) DO NOTHING",
                named_params! { ":id": id },
            )?;
            if let Some(card) = &computer.card {
                transaction.execute(
                    "UPDATE computers
                     SET name = :name, version = :version, format = :format,
                         last_heard = :last_heard
                     WHERE id = :id",
                    named_params! {
                        ":name": card.name,
                        ":version": card.version,
                        ":format": card.format,
                        ":last_heard": codec::moment_to_text(card.written_at),
                        ":id": id,
                    },
                )?;
            }
            Ok(added > 0)
        })
    }

    fn set_acceptance(
        &self,
        computer: ComputerId,
        acceptance: Acceptance,
    ) -> Result<(), StorageError> {
        let accepted = match acceptance {
            Acceptance::Undecided => None,
            Acceptance::Accepted => Some(true),
            Acceptance::Ignored => Some(false),
        };
        self.database.write(|transaction, _| {
            transaction.execute(
                "UPDATE computers SET accepted = :accepted WHERE id = :id",
                named_params! { ":accepted": accepted, ":id": computer.to_string() },
            )?;
            Ok(())
        })
    }

    fn mark_newer_format(&self, computer: ComputerId, format: u32) -> Result<(), StorageError> {
        self.database.write(|transaction, _| {
            transaction.execute(
                "UPDATE computers SET newer_format = :format WHERE id = :id",
                named_params! { ":format": format, ":id": computer.to_string() },
            )?;
            Ok(())
        })
    }

    fn set_clock_ahead(
        &self,
        computer: ComputerId,
        ahead_by_millis: Option<i64>,
    ) -> Result<(), StorageError> {
        self.database.write(|transaction, _| {
            transaction.execute(
                "UPDATE computers SET clock_ahead = :ahead WHERE id = :id",
                named_params! { ":ahead": ahead_by_millis, ":id": computer.to_string() },
            )?;
            Ok(())
        })
    }

    fn written_files(&self) -> Result<Vec<String>, StorageError> {
        self.database.read(|connection| {
            Ok(connection
                .prepare("SELECT name FROM written_files ORDER BY name")?
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?)
        })
    }

    fn remember_written_file(&self, name: &str, written_through: u64) -> Result<(), StorageError> {
        self.database.write(|transaction, _| {
            transaction.execute(
                "INSERT INTO written_files (name, written_at) VALUES (:name, :at)
                 ON CONFLICT (name) DO UPDATE SET written_at = excluded.written_at",
                named_params! { ":name": name, ":at": codec::moment_to_text(Utc::now()) },
            )?;
            set_meta(
                transaction,
                WRITTEN_THROUGH,
                Some(&written_through.to_string()),
            )
        })
    }

    fn settings(&self) -> Result<SyncSettings, StorageError> {
        self.database.read(|connection| {
            Ok(SyncSettings {
                folder: meta(connection, SYNC_FOLDER)?.map(PathBuf::from),
                computer_name: meta(connection, COMPUTER_NAME)?,
                written_through: written_through(connection)?,
            })
        })
    }

    fn set_folder(&self, folder: Option<&Path>) -> Result<(), StorageError> {
        let folder = folder.map(|folder| folder.to_string_lossy().into_owned());
        self.database.write(|transaction, _| {
            if let Some(folder) = &folder
                && meta(transaction, FILES_FOLDER)?.as_ref() != Some(folder)
            {
                // Another folder: nothing of this computer is written there
                // yet, so the first round writes the whole log.
                transaction.execute("DELETE FROM written_files", [])?;
                set_meta(transaction, WRITTEN_THROUGH, Some("0"))?;
                set_meta(transaction, FILES_FOLDER, Some(folder))?;
            }
            set_meta(transaction, SYNC_FOLDER, folder.as_deref())
        })
    }

    fn set_computer_name(&self, name: &str) -> Result<(), StorageError> {
        self.database
            .write(|transaction, _| set_meta(transaction, COMPUTER_NAME, Some(name)))
    }

    fn notices(&self) -> Result<Vec<SyncNotice>, StorageError> {
        self.database.notices()
    }

    fn add_notice(&self, kind: NoticeKind, message: &str) -> Result<(), StorageError> {
        self.database
            .write(|transaction, _| notices::add(transaction, kind, None, None, message))
    }

    fn dismiss_notice(&self, id: i64) -> Result<(), StorageError> {
        self.database.dismiss_notice(id)
    }

    fn new_identity(&self) -> Result<ComputerId, StorageError> {
        self.database.change_identity(become_another_computer)
    }
}

/// Makes the old identity of this computer another computer, accepted,
/// whose operations this computer has up to the last one it wrote into the
/// folder, so that what the computer that keeps the identity writes from
/// then on is taken in. The operations of this computer that were never
/// written into the folder go on under the new identity, numbered from 1,
/// so that none of them is mistaken for one of the other computer.
fn become_another_computer(
    transaction: &Transaction<'_>,
    old: ComputerId,
    new: ComputerId,
) -> Result<(), DatabaseError> {
    let through = stored_number(written_through(transaction)?);
    let name: Option<String> = meta(transaction, COMPUTER_NAME)?;
    transaction.execute(
        "UPDATE operations
         SET device_id = :new, device_sequence = device_sequence - :through
         WHERE device_id = :old AND device_sequence > :through",
        named_params! {
            ":new": new.to_string(),
            ":old": old.to_string(),
            ":through": through,
        },
    )?;
    transaction.execute(
        "INSERT INTO computers (id, name, accepted, read_up_to)
         VALUES (:id, :name, 1, :through)
         ON CONFLICT (id) DO UPDATE SET accepted = 1, read_up_to = excluded.read_up_to",
        named_params! { ":id": old.to_string(), ":name": name, ":through": through },
    )?;
    transaction.execute("DELETE FROM written_files", [])?;
    set_meta(transaction, WRITTEN_THROUGH, Some("0"))
}
