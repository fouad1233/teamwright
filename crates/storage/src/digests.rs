//! Stores the digests of earlier conversations that the manager kept with a
//! project ([decision 0041](../../../docs/decisions/0041-earlier-conversations-as-digests.md)).

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{DigestRepository, StorageError};
use teamwright_domain::{
    Digest, DigestId, DigestSnapshot, DigestSource, DigestText, DigestTitle, DomainError, ProjectId,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::Entity;

/// The columns of the `digests` table that every query reads.
const DIGEST_COLUMNS: &str =
    "id, project_id, source, held_in, title, text, started_at, ended_at, kept_at";

// The row is updated in place, never replaced.
const UPSERT_DIGEST: &str = "
    INSERT INTO digests (
        id, project_id, source, held_in, title, text, started_at, ended_at, kept_at
    )
    VALUES (
        :id, :project_id, :source, :held_in, :title, :text, :started_at, :ended_at, :kept_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        source = excluded.source,
        held_in = excluded.held_in,
        title = excluded.title,
        text = excluded.text,
        started_at = excluded.started_at,
        ended_at = excluded.ended_at,
        kept_at = excluded.kept_at";

/// A digest as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DigestRecord {
    id: String,
    project_id: String,
    source: String,
    held_in: String,
    title: String,
    text: String,
    started_at: Option<String>,
    ended_at: Option<String>,
    kept_at: String,
}

impl DigestRecord {
    /// Takes the stored form of a digest.
    fn of(digest: &Digest) -> Self {
        Self {
            id: digest.id().to_string(),
            project_id: digest.project_id().to_string(),
            source: digest.source().source().to_owned(),
            held_in: digest.source().held_in().to_owned(),
            title: digest.title().to_string(),
            text: digest.text().as_str().to_owned(),
            started_at: digest.started_at().map(codec::moment_to_text),
            ended_at: digest.ended_at().map(codec::moment_to_text),
            kept_at: codec::moment_to_text(digest.kept_at()),
        }
    }

    /// Reads the columns of a `digests` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            source: row.get("source")?,
            held_in: row.get("held_in")?,
            title: row.get("title")?,
            text: row.get("text")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            kept_at: row.get("kept_at")?,
        })
    }

    /// Rebuilds the digest, checking the stored values against the rules.
    fn into_digest(self) -> Result<Digest, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("digest", &self.id, &reason);
        let moment =
            |text: &Option<String>| text.as_deref().map(codec::moment_from_text).transpose();
        Ok(Digest::restore(DigestSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            source: DigestSource::new(&self.source, &self.held_in).map_err(broken)?,
            title: DigestTitle::parse(&self.title).map_err(broken)?,
            text: DigestText::parse(&self.text).map_err(broken)?,
            started_at: moment(&self.started_at)?,
            ended_at: moment(&self.ended_at)?,
            kept_at: codec::moment_from_text(&self.kept_at)?,
        }))
    }
}

/// Stores digests in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteDigestRepository {
    database: Arc<Database>,
}

impl SqliteDigestRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl DigestRepository for SqliteDigestRepository {
    fn save(&self, digest: &Digest) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let record = DigestRecord::of(digest);
            write_rows(transaction, &record)?;
            operations.record_upsert(Entity::Digest, &record.id, &record)
        })
    }

    fn find(&self, id: DigestId) -> Result<Option<Digest>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {DIGEST_COLUMNS} FROM digests WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    DigestRecord::from_row,
                )
                .optional()?
                .map(DigestRecord::into_digest)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Digest>, StorageError> {
        self.database
            .read(|connection| digests_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, id: DigestId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            if remove_digest(transaction, &id)? {
                operations.record_delete(Entity::Digest, &id)?;
            }
            Ok(())
        })
    }
}

/// Reads the digests of a project, in no particular order.
pub(crate) fn digests_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Digest>, DatabaseError> {
    let query = format!("SELECT {DIGEST_COLUMNS} FROM digests WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            DigestRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records.into_iter().map(DigestRecord::into_digest).collect()
}

/// Removes the row of a digest, without logging it, and says whether there
/// was one.
pub(crate) fn remove_digest(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM digests WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the row of a digest from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: DigestRecord = codec::record_from_state("digest", state)?;
    write_rows(transaction, &DigestRecord::of(&record.into_digest()?))
}

/// Writes the row of a digest.
fn write_rows(transaction: &Transaction<'_>, record: &DigestRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_DIGEST,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":source": record.source,
            ":held_in": record.held_in,
            ":title": record.title,
            ":text": record.text,
            ":started_at": record.started_at,
            ":ended_at": record.ended_at,
            ":kept_at": record.kept_at,
        },
    )?;
    Ok(())
}
