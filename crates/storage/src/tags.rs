//! Stores the tags of the projects. Which tasks carry a tag is stored with
//! each task (see `tasks`).

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{StorageError, TagRepository};
use teamwright_domain::{DomainError, ProjectId, Tag, TagId, TagName, TagSnapshot};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `tags` table that every query reads.
const TAG_COLUMNS: &str = "id, project_id, name, color";

// The row is updated in place, never replaced: replacing it would take the
// tag off its tasks through the cascade of `task_tags`.
const UPSERT_TAG: &str = "
    INSERT INTO tags (id, project_id, name, color)
    VALUES (:id, :project_id, :name, :color)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        color = excluded.color";

/// A tag as the database and the operation log keep it: plain text.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagRecord {
    id: String,
    project_id: String,
    name: String,
    color: String,
}

impl TagRecord {
    /// Takes the stored form of a tag.
    fn of(tag: &Tag) -> Self {
        Self {
            id: tag.id().to_string(),
            project_id: tag.project_id().to_string(),
            name: tag.name().to_string(),
            color: tag.color().to_string(),
        }
    }

    /// Reads the columns of a `tags` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            color: row.get("color")?,
        })
    }

    /// Rebuilds the tag, checking the stored values against the rules.
    fn into_tag(self) -> Result<Tag, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("tag", &self.id, &reason);
        Ok(Tag::restore(TagSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: TagName::parse(&self.name).map_err(broken)?,
            color: codec::parse(&self.color)?,
        }))
    }
}

/// Stores tags in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteTagRepository {
    database: Arc<Database>,
}

impl SqliteTagRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl TagRepository for SqliteTagRepository {
    fn save_all(&self, tags: &[Tag]) -> Result<(), StorageError> {
        if tags.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for tag in tags {
                store_tag(transaction, operations, tag)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: TagId) -> Result<Option<Tag>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {TAG_COLUMNS} FROM tags WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    TagRecord::from_row,
                )
                .optional()?
                .map(TagRecord::into_tag)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Tag>, StorageError> {
        self.database
            .read(|connection| tags_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, id: TagId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            if remove_tag(transaction, &id)? {
                operations.record_delete(Entity::Tag, &id)?;
            }
            Ok(())
        })
    }
}

/// Reads the tags of a project inside a transaction, in no particular
/// order.
pub(crate) fn tags_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Tag>, DatabaseError> {
    let query = format!("SELECT {TAG_COLUMNS} FROM tags WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            TagRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records.into_iter().map(TagRecord::into_tag).collect()
}

/// Writes one tag and logs the change.
pub(crate) fn store_tag(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    tag: &Tag,
) -> Result<(), DatabaseError> {
    let record = TagRecord::of(tag);
    write_rows(transaction, &record)?;
    operations.record_upsert(Entity::Tag, &record.id, &record)
}

/// Removes the row of a tag, without logging it, and says whether there
/// was one. The tasks that still carry it lose it with it.
pub(crate) fn remove_tag(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM tags WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the row of a tag from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: TagRecord = codec::record_from_state("tag", state)?;
    write_rows(transaction, &TagRecord::of(&record.into_tag()?))
}

/// Writes the row of a tag.
fn write_rows(transaction: &Transaction<'_>, record: &TagRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_TAG,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":color": record.color,
        },
    )?;
    Ok(())
}
