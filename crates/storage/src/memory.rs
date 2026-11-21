//! Stores the memory of the projects: decisions, facts and contracts
//! (decision 0036).

use std::sync::Arc;

use rusqlite::{Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{MemoryRepository, StorageError};
use teamwright_domain::{
    MemoryContent, MemoryEntry, MemoryEntryId, MemoryEntrySnapshot, MemoryOrigin, ProjectId, SaidIn,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `memory_entries` table that every query reads.
const COLUMNS: &str = "id, project_id, kind, title, text, source, member_id, conversation_id, \
                       turn_id, task_id, recorded_at, replaced_by";

// The rows are updated in place, never replaced.
const UPSERT_ENTRY: &str = "
    INSERT INTO memory_entries (
        id, project_id, kind, title, text, source, member_id, conversation_id, turn_id, task_id,
        recorded_at, replaced_by
    )
    VALUES (
        :id, :project_id, :kind, :title, :text, :source, :member_id, :conversation_id, :turn_id,
        :task_id, :recorded_at, :replaced_by
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        kind = excluded.kind,
        title = excluded.title,
        text = excluded.text,
        source = excluded.source,
        member_id = excluded.member_id,
        conversation_id = excluded.conversation_id,
        turn_id = excluded.turn_id,
        task_id = excluded.task_id,
        recorded_at = excluded.recorded_at,
        replaced_by = excluded.replaced_by";

/// An entry as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryRecord {
    id: String,
    project_id: String,
    kind: String,
    title: String,
    text: String,
    source: String,
    member_id: Option<String>,
    conversation_id: Option<String>,
    turn_id: Option<String>,
    task_id: Option<String>,
    recorded_at: String,
    replaced_by: Option<String>,
}

impl EntryRecord {
    /// Takes the stored form of an entry.
    fn of(entry: &MemoryEntry) -> Self {
        let origin = entry.origin();
        let said_in = origin.said_in();
        Self {
            id: entry.id().to_string(),
            project_id: entry.project_id().to_string(),
            kind: entry.kind().to_string(),
            title: entry.title().to_owned(),
            text: entry.text().to_owned(),
            source: origin.source().to_string(),
            member_id: said_in.map(|said| said.member_id.to_string()),
            conversation_id: said_in.map(|said| said.conversation_id.to_string()),
            turn_id: said_in.map(|said| said.turn_id.to_string()),
            task_id: entry.task_id().map(|id| id.to_string()),
            recorded_at: codec::moment_to_text(entry.recorded_at()),
            replaced_by: entry.replaced_by().map(|id| id.to_string()),
        }
    }

    /// Reads the columns of a `memory_entries` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            kind: row.get("kind")?,
            title: row.get("title")?,
            text: row.get("text")?,
            source: row.get("source")?,
            member_id: row.get("member_id")?,
            conversation_id: row.get("conversation_id")?,
            turn_id: row.get("turn_id")?,
            task_id: row.get("task_id")?,
            recorded_at: row.get("recorded_at")?,
            replaced_by: row.get("replaced_by")?,
        })
    }

    /// Rebuilds the entry, checking the stored values against the rules.
    fn into_entry(self) -> Result<MemoryEntry, DatabaseError> {
        let broken = |reason| codec::damaged_record("memory entry", &self.id, &reason);
        let said_in = match (&self.member_id, &self.conversation_id, &self.turn_id) {
            (None, None, None) => None,
            (Some(member_id), Some(conversation_id), Some(turn_id)) => Some(SaidIn {
                member_id: codec::parse(member_id)?,
                conversation_id: codec::parse(conversation_id)?,
                turn_id: codec::parse(turn_id)?,
            }),
            _ => {
                return Err(DatabaseError::Damaged(format!(
                    "the memory entry {} names part of where it was said",
                    self.id
                )));
            }
        };
        let origin =
            MemoryOrigin::from_parts(codec::parse(&self.source)?, said_in).map_err(broken)?;
        let content = MemoryContent::new(&self.title, &self.text).map_err(broken)?;
        MemoryEntry::restore(MemoryEntrySnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            kind: codec::parse(&self.kind)?,
            content,
            origin,
            task_id: self.task_id.as_deref().map(codec::parse).transpose()?,
            recorded_at: codec::moment_from_text(&self.recorded_at)?,
            replaced_by: self.replaced_by.as_deref().map(codec::parse).transpose()?,
        })
        .map_err(broken)
    }
}

/// Stores the memory of the projects in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteMemoryRepository {
    database: Arc<Database>,
}

impl SqliteMemoryRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl MemoryRepository for SqliteMemoryRepository {
    fn save_entries(&self, entries: &[MemoryEntry]) -> Result<(), StorageError> {
        if entries.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for entry in entries {
                store_entry(transaction, operations, entry)?;
            }
            Ok(())
        })
    }

    fn entries_of_project(&self, project_id: ProjectId) -> Result<Vec<MemoryEntry>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {COLUMNS} FROM memory_entries WHERE project_id = :project_id
                 ORDER BY recorded_at, id"
            );
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    EntryRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(EntryRecord::into_entry).collect()
        })
    }

    fn find_entry(&self, id: MemoryEntryId) -> Result<Option<MemoryEntry>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {COLUMNS} FROM memory_entries WHERE id = :id");
            let mut statement = connection.prepare(&query)?;
            let mut records = statement
                .query_map(
                    named_params! { ":id": id.to_string() },
                    EntryRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.pop().map(EntryRecord::into_entry).transpose()
        })
    }

    fn delete_entry(&self, id: MemoryEntryId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            if remove_entry(transaction, &id)? {
                operations.record_delete(Entity::MemoryEntry, &id)?;
            }
            Ok(())
        })
    }
}

/// Writes an entry and logs the change.
pub(crate) fn store_entry(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    entry: &MemoryEntry,
) -> Result<(), DatabaseError> {
    let record = EntryRecord::of(entry);
    write_entry_row(transaction, &record)?;
    operations.record_upsert(Entity::MemoryEntry, &record.id, &record)
}

/// Removes the row of an entry, without logging it, and says whether there
/// was one.
pub(crate) fn remove_entry(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM memory_entries WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the row of an entry from a state the log keeps, without logging
/// anything. A state that breaks a rule is refused, and nothing is written.
pub(crate) fn write_entry_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: EntryRecord = codec::record_from_state("memory entry", state)?;
    let record = EntryRecord::of(&record.into_entry()?);
    write_entry_row(transaction, &record)
}

/// Writes the row of an entry.
fn write_entry_row(
    transaction: &Transaction<'_>,
    record: &EntryRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_ENTRY,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":kind": record.kind,
            ":title": record.title,
            ":text": record.text,
            ":source": record.source,
            ":member_id": record.member_id,
            ":conversation_id": record.conversation_id,
            ":turn_id": record.turn_id,
            ":task_id": record.task_id,
            ":recorded_at": record.recorded_at,
            ":replaced_by": record.replaced_by,
        },
    )?;
    Ok(())
}
