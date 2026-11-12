//! Stores the views of the projects.

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{StorageError, ViewRepository};
use teamwright_domain::{
    DomainError, ProjectId, View, ViewAssignee, ViewFilter, ViewId, ViewName, ViewSnapshot,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `views` table that every query reads.
const VIEW_COLUMNS: &str =
    "id, project_id, kind, name, assignee_kind, assignee_member_id, hides_finished, position";

// The row is updated in place, never replaced.
const UPSERT_VIEW: &str = "
    INSERT INTO views (
        id, project_id, kind, name, assignee_kind, assignee_member_id, hides_finished, position
    )
    VALUES (
        :id, :project_id, :kind, :name, :assignee_kind, :assignee_member_id, :hides_finished,
        :position
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        kind = excluded.kind,
        name = excluded.name,
        assignee_kind = excluded.assignee_kind,
        assignee_member_id = excluded.assignee_member_id,
        hides_finished = excluded.hides_finished,
        position = excluded.position";

/// A view as the database and the operation log keep it: plain text,
/// numbers and a flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewRecord {
    id: String,
    project_id: String,
    kind: String,
    name: String,
    assignee_kind: String,
    assignee_member_id: Option<String>,
    hides_finished: bool,
    position: u32,
}

impl ViewRecord {
    /// Takes the stored form of a view.
    fn of(view: &View) -> Self {
        let filter = view.filter();
        let (assignee_kind, assignee_member_id) = match filter.assignee {
            ViewAssignee::Anyone => ("anyone", None),
            ViewAssignee::Manager => ("manager", None),
            ViewAssignee::Member(member) => ("member", Some(member.to_string())),
            ViewAssignee::Nobody => ("nobody", None),
        };
        Self {
            id: view.id().to_string(),
            project_id: view.project_id().to_string(),
            kind: view.kind().to_string(),
            name: view.name().to_string(),
            assignee_kind: assignee_kind.to_owned(),
            assignee_member_id,
            hides_finished: filter.hides_finished,
            position: view.position(),
        }
    }

    /// Reads the columns of a `views` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            kind: row.get("kind")?,
            name: row.get("name")?,
            assignee_kind: row.get("assignee_kind")?,
            assignee_member_id: row.get("assignee_member_id")?,
            hides_finished: row.get::<_, i64>("hides_finished")? != 0,
            position: row.get("position")?,
        })
    }

    /// Rebuilds the view, checking the stored values against the rules.
    fn into_view(self) -> Result<View, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("view", &self.id, &reason);
        let assignee = match (self.assignee_kind.as_str(), &self.assignee_member_id) {
            ("anyone", None) => ViewAssignee::Anyone,
            ("manager", None) => ViewAssignee::Manager,
            ("member", Some(member)) => ViewAssignee::Member(codec::parse(member)?),
            ("nobody", None) => ViewAssignee::Nobody,
            _ => {
                return Err(DatabaseError::Damaged(format!(
                    "the view {} shows the tasks of '{}' with the member {:?}",
                    self.id, self.assignee_kind, self.assignee_member_id
                )));
            }
        };
        Ok(View::restore(ViewSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            kind: codec::parse(&self.kind)?,
            name: ViewName::parse(&self.name).map_err(broken)?,
            filter: ViewFilter {
                assignee,
                hides_finished: self.hides_finished,
            },
            position: self.position,
        }))
    }
}

/// Stores views in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteViewRepository {
    database: Arc<Database>,
}

impl SqliteViewRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ViewRepository for SqliteViewRepository {
    fn save_all(&self, views: &[View]) -> Result<(), StorageError> {
        if views.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for view in views {
                store_view(transaction, operations, view)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: ViewId) -> Result<Option<View>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {VIEW_COLUMNS} FROM views WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    ViewRecord::from_row,
                )
                .optional()?
                .map(ViewRecord::into_view)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<View>, StorageError> {
        self.database
            .read(|connection| views_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, id: ViewId) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| delete_view(transaction, operations, &id.to_string()))
    }
}

/// Reads the views of a project inside a transaction, in no particular
/// order.
pub(crate) fn views_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<View>, DatabaseError> {
    let query = format!("SELECT {VIEW_COLUMNS} FROM views WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            ViewRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records.into_iter().map(ViewRecord::into_view).collect()
}

/// Writes one view and logs the change.
pub(crate) fn store_view(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    view: &View,
) -> Result<(), DatabaseError> {
    let record = ViewRecord::of(view);
    write_rows(transaction, &record)?;
    operations.record_upsert(Entity::View, &record.id, &record)
}

/// Removes the row of a view and logs the removal. A view that is not
/// stored is ignored.
pub(crate) fn delete_view(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    id: &str,
) -> Result<(), DatabaseError> {
    if remove_view(transaction, id)? {
        operations.record_delete(Entity::View, id)?;
    }
    Ok(())
}

/// Removes the row of a view, without logging it, and says whether there
/// was one.
pub(crate) fn remove_view(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM views WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Rebuilds a view from a state the log keeps, checking it against the
/// rules of the domain.
fn view_from_state(state: &Map<String, Value>) -> Result<View, DatabaseError> {
    let record: ViewRecord = codec::record_from_state("view", state)?;
    record.into_view()
}

/// Writes the row of a view from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    write_rows(transaction, &ViewRecord::of(&view_from_state(state)?))
}

/// Writes the row of a view.
fn write_rows(transaction: &Transaction<'_>, record: &ViewRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_VIEW,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":kind": record.kind,
            ":name": record.name,
            ":assignee_kind": record.assignee_kind,
            ":assignee_member_id": record.assignee_member_id,
            ":hides_finished": i64::from(record.hides_finished),
            ":position": record.position,
        },
    )?;
    Ok(())
}
