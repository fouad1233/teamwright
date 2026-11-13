//! Stores the areas of the projects.

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{AreaRepository, StorageError};
use teamwright_domain::{Area, AreaId, AreaName, AreaSnapshot, DomainError, ProjectId};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `areas` table that every query reads.
const AREA_COLUMNS: &str = "id, project_id, name, position";

// The row is updated in place, never replaced.
const UPSERT_AREA: &str = "
    INSERT INTO areas (id, project_id, name, position)
    VALUES (:id, :project_id, :name, :position)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        position = excluded.position";

/// An area as the database and the operation log keep it: plain text and a
/// number.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AreaRecord {
    id: String,
    project_id: String,
    name: String,
    position: u32,
}

impl AreaRecord {
    /// Takes the stored form of an area.
    fn of(area: &Area) -> Self {
        Self {
            id: area.id().to_string(),
            project_id: area.project_id().to_string(),
            name: area.name().to_string(),
            position: area.position(),
        }
    }

    /// Reads the columns of an `areas` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            position: row.get("position")?,
        })
    }

    /// Rebuilds the area, checking the stored values against the rules.
    fn into_area(self) -> Result<Area, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("area", &self.id, &reason);
        Ok(Area::restore(AreaSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: AreaName::parse(&self.name).map_err(broken)?,
            position: self.position,
        }))
    }
}

/// Stores areas in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteAreaRepository {
    database: Arc<Database>,
}

impl SqliteAreaRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl AreaRepository for SqliteAreaRepository {
    fn save_all(&self, areas: &[Area]) -> Result<(), StorageError> {
        if areas.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for area in areas {
                store_area(transaction, operations, area)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: AreaId) -> Result<Option<Area>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {AREA_COLUMNS} FROM areas WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    AreaRecord::from_row,
                )
                .optional()?
                .map(AreaRecord::into_area)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Area>, StorageError> {
        self.database
            .read(|connection| areas_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, id: AreaId) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| delete_area(transaction, operations, &id.to_string()))
    }
}

/// Reads the areas of a project inside a transaction, in no particular
/// order.
pub(crate) fn areas_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Area>, DatabaseError> {
    let query = format!("SELECT {AREA_COLUMNS} FROM areas WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            AreaRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records.into_iter().map(AreaRecord::into_area).collect()
}

/// Writes one area and logs the change.
pub(crate) fn store_area(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    area: &Area,
) -> Result<(), DatabaseError> {
    let record = AreaRecord::of(area);
    write_rows(transaction, &record)?;
    operations.record_upsert(Entity::Area, &record.id, &record)
}

/// Removes the row of an area and logs the removal. An area that is not
/// stored is ignored.
pub(crate) fn delete_area(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    id: &str,
) -> Result<(), DatabaseError> {
    if remove_area(transaction, id)? {
        operations.record_delete(Entity::Area, id)?;
    }
    Ok(())
}

/// Removes the row of an area, without logging it, and says whether there
/// was one.
pub(crate) fn remove_area(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM areas WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the row of an area from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: AreaRecord = codec::record_from_state("area", state)?;
    write_rows(transaction, &AreaRecord::of(&record.into_area()?))
}

/// Writes the row of an area.
fn write_rows(transaction: &Transaction<'_>, record: &AreaRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_AREA,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":position": record.position,
        },
    )?;
    Ok(())
}
