//! Stores the sprints of the projects.

use std::sync::Arc;

use rusqlite::{OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{SprintRepository, StorageError};
use teamwright_domain::{
    DomainError, ProjectId, Sprint, SprintDates, SprintId, SprintName, SprintSnapshot,
    SprintSummary, Task,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};
use crate::tasks;

/// The columns of the `sprints` table that every query reads.
const SPRINT_COLUMNS: &str = "id, project_id, name, goal, start_on, end_on, state, created_at, \
     started_at, completed_at, summary_unit, summary_committed, summary_completed";

// The row is updated in place. Replacing it would delete it first, and with
// it the sprint of the tasks that are in it.
const UPSERT_SPRINT: &str = "
    INSERT INTO sprints (
        id, project_id, name, goal, start_on, end_on, state, created_at, started_at,
        completed_at, summary_unit, summary_committed, summary_completed
    )
    VALUES (
        :id, :project_id, :name, :goal, :start_on, :end_on, :state, :created_at, :started_at,
        :completed_at, :summary_unit, :summary_committed, :summary_completed
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        goal = excluded.goal,
        start_on = excluded.start_on,
        end_on = excluded.end_on,
        state = excluded.state,
        created_at = excluded.created_at,
        started_at = excluded.started_at,
        completed_at = excluded.completed_at,
        summary_unit = excluded.summary_unit,
        summary_committed = excluded.summary_committed,
        summary_completed = excluded.summary_completed";

/// What a completed sprint achieved, as the database and the operation log
/// keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SummaryRecord {
    unit: String,
    committed: u32,
    completed: u32,
}

/// A sprint as the database and the operation log keep it: plain text and
/// numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SprintRecord {
    id: String,
    project_id: String,
    name: String,
    goal: String,
    start_on: Option<String>,
    end_on: Option<String>,
    state: String,
    created_at: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    summary: Option<SummaryRecord>,
    /// Whether a row had only a part of a summary. Only rows can; the log
    /// keeps a summary whole.
    #[serde(skip, default)]
    summary_is_partial: bool,
}

impl SprintRecord {
    /// Takes the stored form of a sprint.
    fn of(sprint: &Sprint) -> Self {
        Self {
            id: sprint.id().to_string(),
            project_id: sprint.project_id().to_string(),
            name: sprint.name().to_string(),
            goal: sprint.goal().to_owned(),
            start_on: sprint.dates().start_on().map(codec::date_to_text),
            end_on: sprint.dates().end_on().map(codec::date_to_text),
            state: sprint.state().to_string(),
            created_at: codec::moment_to_text(sprint.created_at()),
            started_at: sprint.started_at().map(codec::moment_to_text),
            completed_at: sprint.completed_at().map(codec::moment_to_text),
            summary: sprint.summary().map(|summary| SummaryRecord {
                unit: summary.unit.to_string(),
                committed: summary.committed,
                completed: summary.completed,
            }),
            summary_is_partial: false,
        }
    }

    /// Reads the columns of a `sprints` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let unit: Option<String> = row.get("summary_unit")?;
        let committed: Option<u32> = row.get("summary_committed")?;
        let completed: Option<u32> = row.get("summary_completed")?;
        // A summary is stored whole or not at all. A row that has only a
        // part of one is reported when the sprint is rebuilt.
        let (summary, summary_is_partial) = match (unit, committed, completed) {
            (Some(unit), Some(committed), Some(completed)) => (
                Some(SummaryRecord {
                    unit,
                    committed,
                    completed,
                }),
                false,
            ),
            (None, None, None) => (None, false),
            _ => (None, true),
        };
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            goal: row.get("goal")?,
            start_on: row.get("start_on")?,
            end_on: row.get("end_on")?,
            state: row.get("state")?,
            created_at: row.get("created_at")?,
            started_at: row.get("started_at")?,
            completed_at: row.get("completed_at")?,
            summary,
            summary_is_partial,
        })
    }

    /// Rebuilds the sprint, checking the stored values against the rules.
    fn into_sprint(self) -> Result<Sprint, DatabaseError> {
        let record_id = self.id.clone();
        let broken = |reason: DomainError| codec::damaged_record("sprint", &record_id, &reason);
        if self.summary_is_partial {
            return Err(DatabaseError::Damaged(format!(
                "the sprint {} has only a part of its summary",
                self.id
            )));
        }

        let dates = SprintDates::new(
            self.start_on
                .as_deref()
                .map(codec::date_from_text)
                .transpose()?,
            self.end_on
                .as_deref()
                .map(codec::date_from_text)
                .transpose()?,
        )
        .map_err(broken)?;
        let summary = self
            .summary
            .map(|summary| {
                Ok::<_, DatabaseError>(SprintSummary {
                    unit: codec::parse(&summary.unit)?,
                    committed: summary.committed,
                    completed: summary.completed,
                })
            })
            .transpose()?;

        Ok(Sprint::restore(SprintSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: SprintName::parse(&self.name).map_err(broken)?,
            goal: self.goal,
            dates,
            state: codec::parse(&self.state)?,
            created_at: codec::moment_from_text(&self.created_at)?,
            started_at: self
                .started_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
            completed_at: self
                .completed_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
            summary,
        }))
    }
}

/// Stores sprints in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteSprintRepository {
    database: Arc<Database>,
}

impl SqliteSprintRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl SprintRepository for SqliteSprintRepository {
    fn save(&self, sprint: &Sprint, tasks: &[Task]) -> Result<(), StorageError> {
        let record = SprintRecord::of(sprint);
        self.database.write(|transaction, operations| {
            // The sprint goes first: the tasks may refer to it.
            store(transaction, operations, &record)?;
            for task in tasks {
                tasks::store_task(transaction, operations, task)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: SprintId) -> Result<Option<Sprint>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {SPRINT_COLUMNS} FROM sprints WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    SprintRecord::from_row,
                )
                .optional()?
                .map(SprintRecord::into_sprint)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Sprint>, StorageError> {
        self.database.read(|connection| {
            let query =
                format!("SELECT {SPRINT_COLUMNS} FROM sprints WHERE project_id = :project_id");
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    SprintRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(SprintRecord::into_sprint).collect()
        })
    }

    fn delete(&self, id: SprintId, tasks: &[Task]) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            // The tasks leave the sprint first, each with its own entry in
            // the log, so the database never has to take them out by
            // itself.
            for task in tasks {
                tasks::store_task(transaction, operations, task)?;
            }
            let id = id.to_string();
            if remove_sprint(transaction, &id)? {
                operations.record_delete(Entity::Sprint, &id)?;
            }
            Ok(())
        })
    }
}

/// Removes the row of a sprint, without logging it, and says whether there
/// was one.
pub(crate) fn remove_sprint(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM sprints WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes one sprint and logs the change.
fn store(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    record: &SprintRecord,
) -> Result<(), DatabaseError> {
    write_rows(transaction, record)?;
    operations.record_upsert(Entity::Sprint, &record.id, record)
}

/// Writes the row of a sprint from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: SprintRecord = codec::record_from_state("sprint", state)?;
    write_rows(transaction, &SprintRecord::of(&record.into_sprint()?))
}

/// Writes the row of a sprint.
fn write_rows(transaction: &Transaction<'_>, record: &SprintRecord) -> Result<(), DatabaseError> {
    let summary = record.summary.as_ref();
    transaction.execute(
        UPSERT_SPRINT,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":goal": record.goal,
            ":start_on": record.start_on,
            ":end_on": record.end_on,
            ":state": record.state,
            ":created_at": record.created_at,
            ":started_at": record.started_at,
            ":completed_at": record.completed_at,
            ":summary_unit": summary.map(|summary| summary.unit.as_str()),
            ":summary_committed": summary.map(|summary| summary.committed),
            ":summary_completed": summary.map(|summary| summary.completed),
        },
    )?;
    Ok(())
}
