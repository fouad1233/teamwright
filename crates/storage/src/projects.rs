//! Stores projects together with their workflows.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::Serialize;
use teamwright_application::{ProjectRepository, StorageError};
use teamwright_domain::{
    DomainError, Project, ProjectId, ProjectKey, ProjectName, ProjectSnapshot, Status, StatusName,
    Workflow,
};

use crate::codec;
use crate::conversations;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `projects` table that every query reads.
const PROJECT_COLUMNS: &str =
    "id, key, name, description, next_task_number, created_at, archived_at";

const UPSERT_PROJECT: &str = "
    INSERT INTO projects (id, key, name, description, next_task_number, created_at, archived_at)
    VALUES (:id, :key, :name, :description, :next_task_number, :created_at, :archived_at)
    ON CONFLICT (id) DO UPDATE SET
        key = excluded.key,
        name = excluded.name,
        description = excluded.description,
        next_task_number = excluded.next_task_number,
        created_at = excluded.created_at,
        archived_at = excluded.archived_at";

const UPSERT_STATUS: &str = "
    INSERT INTO statuses (id, project_id, position, name, category, color)
    VALUES (:id, :project_id, :position, :name, :category, :color)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        position = excluded.position,
        name = excluded.name,
        category = excluded.category,
        color = excluded.color";

/// A status as the database and the operation log keep it.
#[derive(Debug, Serialize)]
struct StatusRecord {
    id: String,
    name: String,
    category: String,
    color: String,
}

impl StatusRecord {
    /// Takes the stored form of a status.
    fn of(status: &Status) -> Self {
        Self {
            id: status.id().to_string(),
            name: status.name().to_string(),
            category: status.category().to_string(),
            color: status.color().to_string(),
        }
    }

    /// Rebuilds the status, checking the stored values against the rules.
    fn into_status(self) -> Result<Status, DatabaseError> {
        let name = StatusName::parse(&self.name)
            .map_err(|reason| codec::damaged_record("status", &self.id, &reason))?;
        Ok(Status::restore(
            codec::parse(&self.id)?,
            name,
            codec::parse(&self.category)?,
            codec::parse(&self.color)?,
        ))
    }
}

/// A project as the database and the operation log keep it: plain text and
/// numbers, with its statuses in board order.
#[derive(Debug, Serialize)]
struct ProjectRecord {
    id: String,
    key: String,
    name: String,
    description: String,
    next_task_number: u32,
    created_at: String,
    archived_at: Option<String>,
    statuses: Vec<StatusRecord>,
}

impl ProjectRecord {
    /// Takes the stored form of a project.
    fn of(project: &Project) -> Self {
        Self {
            id: project.id().to_string(),
            key: project.key().to_string(),
            name: project.name().to_string(),
            description: project.description().to_owned(),
            next_task_number: project.next_task_number(),
            created_at: codec::moment_to_text(project.created_at()),
            archived_at: project.archived_at().map(codec::moment_to_text),
            statuses: project
                .workflow()
                .statuses()
                .iter()
                .map(StatusRecord::of)
                .collect(),
        }
    }

    /// Reads the columns of a `projects` row. The statuses live in another
    /// table and are added by the caller.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            key: row.get("key")?,
            name: row.get("name")?,
            description: row.get("description")?,
            next_task_number: row.get("next_task_number")?,
            created_at: row.get("created_at")?,
            archived_at: row.get("archived_at")?,
            statuses: Vec::new(),
        })
    }

    /// Rebuilds the project, checking the stored values against the rules.
    fn into_project(self) -> Result<Project, DatabaseError> {
        let record_id = self.id.clone();
        let broken = |reason: DomainError| codec::damaged_record("project", &record_id, &reason);

        let statuses = self
            .statuses
            .into_iter()
            .map(StatusRecord::into_status)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Project::restore(ProjectSnapshot {
            id: codec::parse(&self.id)?,
            key: ProjectKey::parse(&self.key).map_err(broken)?,
            name: ProjectName::parse(&self.name).map_err(broken)?,
            description: self.description,
            workflow: Workflow::new(statuses).map_err(broken)?,
            next_task_number: self.next_task_number,
            created_at: codec::moment_from_text(&self.created_at)?,
            archived_at: self
                .archived_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        }))
    }
}

/// Stores projects in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteProjectRepository {
    database: Arc<Database>,
}

impl SqliteProjectRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ProjectRepository for SqliteProjectRepository {
    fn save(&self, project: &Project) -> Result<(), StorageError> {
        let record = ProjectRecord::of(project);
        self.database.write(|transaction, operations| {
            upsert_project(transaction, &record)?;
            replace_statuses(transaction, &record)?;
            operations.record_upsert(Entity::Project, &record.id, &record)
        })
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = :id");
            let found = connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    ProjectRecord::from_row,
                )
                .optional()?;
            let Some(mut record) = found else {
                return Ok(None);
            };
            record.statuses = statuses_of(connection, &record.id)?;
            record.into_project().map(Some)
        })
    }

    fn list(&self) -> Result<Vec<Project>, StorageError> {
        self.database.read(|connection| {
            let mut statuses_by_project = statuses_of_all_projects(connection)?;
            let query = format!("SELECT {PROJECT_COLUMNS} FROM projects ORDER BY created_at, id");
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map([], ProjectRecord::from_row)?
                .collect::<Result<Vec<_>, _>>()?;

            records
                .into_iter()
                .map(|mut record| {
                    record.statuses = statuses_by_project.remove(&record.id).unwrap_or_default();
                    record.into_project()
                })
                .collect()
        })
    }

    fn delete(&self, id: ProjectId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            // The database would remove the conversations, tasks and members
            // of the project by itself, but without a trace in the log. They
            // are removed here one by one, each with its own entry, like any
            // other deletion. Normally the use case has removed them already
            // and nothing is left to do. The conversations go first, because
            // they refer to the tasks and the members.
            conversations::delete_conversations_of_project(transaction, operations, &id)?;
            delete_rows_of_project(transaction, operations, "tasks", Entity::Task, &id)?;
            delete_rows_of_project(transaction, operations, "members", Entity::Member, &id)?;
            // The statuses are part of the project in the log, so their
            // removal is covered by the deletion of the project.
            transaction.execute(
                "DELETE FROM statuses WHERE project_id = :project_id",
                named_params! { ":project_id": id },
            )?;
            let rows_removed = transaction.execute(
                "DELETE FROM projects WHERE id = :id",
                named_params! { ":id": id },
            )?;
            if rows_removed > 0 {
                operations.record_delete(Entity::Project, &id)?;
            }
            Ok(())
        })
    }
}

/// Removes the rows of one table that belong to a project, and logs each
/// removal. `table` is one of the fixed table names above, never input.
fn delete_rows_of_project(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    table: &str,
    entity: Entity,
    project_id: &str,
) -> Result<(), DatabaseError> {
    let ids: Vec<String> = transaction
        .prepare(&format!(
            "SELECT id FROM {table} WHERE project_id = :project_id"
        ))?
        .query_map(named_params! { ":project_id": project_id }, |row| {
            row.get(0)
        })?
        .collect::<Result<_, _>>()?;
    for id in ids {
        transaction.execute(
            &format!("DELETE FROM {table} WHERE id = :id"),
            named_params! { ":id": id },
        )?;
        operations.record_delete(entity, &id)?;
    }
    Ok(())
}

/// Writes the `projects` row.
fn upsert_project(
    transaction: &Transaction<'_>,
    record: &ProjectRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_PROJECT,
        named_params! {
            ":id": record.id,
            ":key": record.key,
            ":name": record.name,
            ":description": record.description,
            ":next_task_number": record.next_task_number,
            ":created_at": record.created_at,
            ":archived_at": record.archived_at,
        },
    )?;
    Ok(())
}

/// Makes the stored statuses of a project equal to the record's: existing
/// ones are updated in place, new ones are added, and those that are no
/// longer part of the workflow are removed.
///
/// Statuses are updated rather than deleted and inserted again, because
/// tasks refer to them.
fn replace_statuses(
    transaction: &Transaction<'_>,
    record: &ProjectRecord,
) -> Result<(), DatabaseError> {
    for (position, status) in (0_i64..).zip(&record.statuses) {
        transaction.execute(
            UPSERT_STATUS,
            named_params! {
                ":id": status.id,
                ":project_id": record.id,
                ":position": position,
                ":name": status.name,
                ":category": status.category,
                ":color": status.color,
            },
        )?;
    }

    let kept: HashSet<&str> = record
        .statuses
        .iter()
        .map(|status| status.id.as_str())
        .collect();
    let stored: Vec<String> = transaction
        .prepare("SELECT id FROM statuses WHERE project_id = :project_id")?
        .query_map(named_params! { ":project_id": record.id }, |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for obsolete in stored.iter().filter(|id| !kept.contains(id.as_str())) {
        transaction.execute(
            "DELETE FROM statuses WHERE id = :id",
            named_params! { ":id": obsolete },
        )?;
    }
    Ok(())
}

/// Reads the statuses of one project in board order.
fn statuses_of(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<StatusRecord>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT id, name, category, color
         FROM statuses
         WHERE project_id = :project_id
         ORDER BY position",
    )?;
    let statuses = statement
        .query_map(named_params! { ":project_id": project_id }, status_from_row)?
        .collect::<Result<_, _>>()?;
    Ok(statuses)
}

/// Reads the statuses of every project, grouped by project and in board
/// order within each.
fn statuses_of_all_projects(
    connection: &Connection,
) -> Result<HashMap<String, Vec<StatusRecord>>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT project_id, id, name, category, color
         FROM statuses
         ORDER BY project_id, position",
    )?;
    let rows = statement.query_map([], |row| {
        let project_id: String = row.get("project_id")?;
        Ok((project_id, status_from_row(row)?))
    })?;

    let mut statuses_by_project: HashMap<String, Vec<StatusRecord>> = HashMap::new();
    for row in rows {
        let (project_id, status) = row?;
        statuses_by_project
            .entry(project_id)
            .or_default()
            .push(status);
    }
    Ok(statuses_by_project)
}

/// Reads the columns of a `statuses` row.
fn status_from_row(row: &Row<'_>) -> rusqlite::Result<StatusRecord> {
    Ok(StatusRecord {
        id: row.get("id")?,
        name: row.get("name")?,
        category: row.get("category")?,
        color: row.get("color")?,
    })
}
