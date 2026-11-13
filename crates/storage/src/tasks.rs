//! Stores tasks together with their dependencies.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{StorageError, TaskRepository};
use teamwright_domain::{
    Area, Assignee, ComputerId, DomainError, ProjectId, Rank, Schedule, StoryPoints, Task, TaskId,
    TaskSnapshot, TaskTitle,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{self, Entity, OperationRecorder};

/// The columns of the `tasks` table that every query reads.
const TASK_COLUMNS: &str = "id, project_id, area_id, number, title, description, status_id, priority, \
     assignee_kind, assignee_member_id, parent_id, estimate_minutes, sprint_id, points, start_on, \
     due_on, rank, created_at, updated_at, completed_at";

/// The value of `assignee_kind` for a task given to the manager.
const ASSIGNEE_MANAGER: &str = "manager";
/// The value of `assignee_kind` for a task given to a team member.
const ASSIGNEE_MEMBER: &str = "member";

// The row is updated in place. Replacing it would delete it first, and with
// it the dependencies and subtask links that refer to it.
const UPSERT_TASK: &str = "
    INSERT INTO tasks (
        id, project_id, area_id, number, title, description, status_id, priority,
        assignee_kind, assignee_member_id, parent_id, estimate_minutes,
        sprint_id, points, start_on, due_on, rank, created_at, updated_at, completed_at
    )
    VALUES (
        :id, :project_id, :area_id, :number, :title, :description, :status_id, :priority,
        :assignee_kind, :assignee_member_id, :parent_id, :estimate_minutes,
        :sprint_id, :points, :start_on, :due_on, :rank, :created_at, :updated_at, :completed_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        area_id = excluded.area_id,
        number = excluded.number,
        title = excluded.title,
        description = excluded.description,
        status_id = excluded.status_id,
        priority = excluded.priority,
        assignee_kind = excluded.assignee_kind,
        assignee_member_id = excluded.assignee_member_id,
        parent_id = excluded.parent_id,
        estimate_minutes = excluded.estimate_minutes,
        sprint_id = excluded.sprint_id,
        points = excluded.points,
        start_on = excluded.start_on,
        due_on = excluded.due_on,
        rank = excluded.rank,
        created_at = excluded.created_at,
        updated_at = excluded.updated_at,
        completed_at = excluded.completed_at";

/// A task as the database and the operation log keep it: plain text and
/// numbers.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRecord {
    id: String,
    project_id: String,
    // A task logged before there were areas, or a row written before, is
    // in the area its project started with, which the migration to areas
    // gave every project.
    #[serde(default)]
    area_id: Option<String>,
    number: u32,
    title: String,
    description: String,
    status_id: String,
    priority: String,
    assignee_kind: Option<String>,
    assignee_member_id: Option<String>,
    parent_id: Option<String>,
    estimate_minutes: Option<u32>,
    // A task logged before there were sprints is in the backlog and has no
    // points.
    #[serde(default)]
    sprint_id: Option<String>,
    #[serde(default)]
    points: Option<u16>,
    start_on: Option<String>,
    due_on: Option<String>,
    rank: f64,
    created_at: String,
    updated_at: String,
    completed_at: Option<String>,
    blocked_by: Vec<String>,
}

impl TaskRecord {
    /// Takes the stored form of a task.
    fn of(task: &Task) -> Self {
        let (assignee_kind, assignee_member_id) = match task.assignee() {
            None => (None, None),
            Some(Assignee::Manager) => (Some(ASSIGNEE_MANAGER.to_owned()), None),
            Some(Assignee::Member(member_id)) => (
                Some(ASSIGNEE_MEMBER.to_owned()),
                Some(member_id.to_string()),
            ),
        };
        Self {
            id: task.id().to_string(),
            project_id: task.project_id().to_string(),
            area_id: Some(task.area_id().to_string()),
            number: task.number(),
            title: task.title().to_string(),
            description: task.description().to_owned(),
            status_id: task.status_id().to_string(),
            priority: task.priority().to_string(),
            assignee_kind,
            assignee_member_id,
            parent_id: task.parent_id().map(|parent_id| parent_id.to_string()),
            estimate_minutes: task.estimate_minutes(),
            sprint_id: task.sprint_id().map(|sprint_id| sprint_id.to_string()),
            points: task.points().map(StoryPoints::value),
            start_on: task.schedule().start_on().map(codec::date_to_text),
            due_on: task.schedule().due_on().map(codec::date_to_text),
            rank: task.rank().value(),
            created_at: codec::moment_to_text(task.created_at()),
            updated_at: codec::moment_to_text(task.updated_at()),
            completed_at: task.completed_at().map(codec::moment_to_text),
            blocked_by: task.blocked_by().iter().map(ToString::to_string).collect(),
        }
    }

    /// Reads the columns of a `tasks` row. The dependencies live in another
    /// table and are added by the caller.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            area_id: row.get("area_id")?,
            number: row.get("number")?,
            title: row.get("title")?,
            description: row.get("description")?,
            status_id: row.get("status_id")?,
            priority: row.get("priority")?,
            assignee_kind: row.get("assignee_kind")?,
            assignee_member_id: row.get("assignee_member_id")?,
            parent_id: row.get("parent_id")?,
            estimate_minutes: row.get("estimate_minutes")?,
            sprint_id: row.get("sprint_id")?,
            points: row.get("points")?,
            start_on: row.get("start_on")?,
            due_on: row.get("due_on")?,
            rank: row.get("rank")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
            blocked_by: Vec::new(),
        })
    }

    /// Rebuilds the task, checking the stored values against the rules.
    fn into_task(self) -> Result<Task, DatabaseError> {
        let record_id = self.id.clone();
        let broken = |reason: DomainError| codec::damaged_record("task", &record_id, &reason);

        let schedule = Schedule::new(
            self.start_on
                .as_deref()
                .map(codec::date_from_text)
                .transpose()?,
            self.due_on
                .as_deref()
                .map(codec::date_from_text)
                .transpose()?,
        )
        .map_err(broken)?;
        let assignee = self.assignee()?;
        let blocked_by = self
            .blocked_by
            .iter()
            .map(|blocker_id| codec::parse(blocker_id))
            .collect::<Result<BTreeSet<TaskId>, _>>()?;

        let project_id: ProjectId = codec::parse(&self.project_id)?;
        let area_id = match self.area_id.as_deref() {
            Some(area_id) => codec::parse(area_id)?,
            None => Area::starting_id(project_id),
        };
        Ok(Task::restore(TaskSnapshot {
            id: codec::parse(&self.id)?,
            project_id,
            area_id,
            number: self.number,
            title: TaskTitle::parse(&self.title).map_err(broken)?,
            description: self.description,
            status_id: codec::parse(&self.status_id)?,
            priority: codec::parse(&self.priority)?,
            assignee,
            parent_id: self.parent_id.as_deref().map(codec::parse).transpose()?,
            blocked_by,
            estimate_minutes: self.estimate_minutes,
            sprint_id: self.sprint_id.as_deref().map(codec::parse).transpose()?,
            points: self
                .points
                .map(|points| StoryPoints::new(i64::from(points)))
                .transpose()
                .map_err(broken)?,
            schedule,
            rank: Rank::from_value(self.rank),
            created_at: codec::moment_from_text(&self.created_at)?,
            updated_at: codec::moment_from_text(&self.updated_at)?,
            completed_at: self
                .completed_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        }))
    }

    /// Reads who is responsible for the task from its two columns.
    fn assignee(&self) -> Result<Option<Assignee>, DatabaseError> {
        match (
            self.assignee_kind.as_deref(),
            self.assignee_member_id.as_deref(),
        ) {
            (None, _) => Ok(None),
            (Some(ASSIGNEE_MANAGER), _) => Ok(Some(Assignee::Manager)),
            (Some(ASSIGNEE_MEMBER), Some(member_id)) => {
                Ok(Some(Assignee::Member(codec::parse(member_id)?)))
            }
            (Some(kind), _) => Err(DatabaseError::Damaged(format!(
                "the task {} has an assignee of the kind '{kind}' that cannot be read",
                self.id
            ))),
        }
    }
}

/// Stores tasks in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteTaskRepository {
    database: Arc<Database>,
}

impl SqliteTaskRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl TaskRepository for SqliteTaskRepository {
    fn save(&self, task: &Task) -> Result<(), StorageError> {
        self.save_all(std::slice::from_ref(task))
    }

    fn save_all(&self, tasks: &[Task]) -> Result<(), StorageError> {
        let records: Vec<TaskRecord> = tasks.iter().map(TaskRecord::of).collect();
        self.database.write(|transaction, operations| {
            for record in &records {
                store(transaction, operations, record)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        self.database
            .read(|connection| find_task(connection, &id.to_string()))
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Task>, StorageError> {
        self.database
            .read(|connection| tasks_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, ids: &[TaskId]) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            for id in ids {
                let id = id.to_string();
                if remove_task(transaction, &id)? {
                    operations.record_delete(Entity::Task, &id)?;
                }
            }
            Ok(())
        })
    }

    fn versions_of(&self, id: TaskId) -> Result<Vec<Task>, StorageError> {
        self.database.read(|connection| {
            // The log holds the complete state of the task after every
            // write, so the versions need no table of their own.
            operations::states_of(connection, Entity::Task, &id.to_string())?
                .iter()
                .map(|state| {
                    let record: TaskRecord = serde_json::from_str(state).map_err(|reason| {
                        DatabaseError::Damaged(format!(
                            "a version of the task {id} in the log cannot be read: {reason}"
                        ))
                    })?;
                    record.into_task()
                })
                .collect()
        })
    }

    fn computers_of_versions(&self, id: TaskId) -> Result<Vec<ComputerId>, StorageError> {
        self.database.read(|connection| {
            operations::computers_of_states(connection, Entity::Task, &id.to_string())?
                .iter()
                .map(|computer| codec::parse(computer))
                .collect()
        })
    }
}

/// Writes one task with its dependencies and logs the change.
pub(crate) fn store_task(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    task: &Task,
) -> Result<(), DatabaseError> {
    store(transaction, operations, &TaskRecord::of(task))
}

/// Writes the stored form of one task with its dependencies and logs the
/// change.
fn store(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    record: &TaskRecord,
) -> Result<(), DatabaseError> {
    write_rows(transaction, record)?;
    operations.record_upsert(Entity::Task, &record.id, record)
}

/// Writes the rows of a task from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: TaskRecord = codec::record_from_state("task", state)?;
    write_rows(transaction, &TaskRecord::of(&record.into_task()?))
}

/// Rebuilds a task from a state the log keeps, checking it against the
/// rules of the domain.
pub(crate) fn task_from_state(state: &Map<String, Value>) -> Result<Task, DatabaseError> {
    let record: TaskRecord = codec::record_from_state("task", state)?;
    record.into_task()
}

/// Removes the row of a task, without logging it, and says whether there
/// was one. What it waits for goes with it.
pub(crate) fn remove_task(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM tasks WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Reads a task inside a transaction, if it is stored.
pub(crate) fn find_task(connection: &Connection, id: &str) -> Result<Option<Task>, DatabaseError> {
    let query = format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = :id");
    let found = connection
        .query_row(&query, named_params! { ":id": id }, TaskRecord::from_row)
        .optional()?;
    let Some(mut record) = found else {
        return Ok(None);
    };
    record.blocked_by = blockers_of(connection, &record.id)?;
    record.into_task().map(Some)
}

/// Reads the tasks of a project inside a transaction, in no particular
/// order.
pub(crate) fn tasks_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Task>, DatabaseError> {
    let mut blockers_by_task = blockers_in_project(connection, project_id)?;
    let query = format!("SELECT {TASK_COLUMNS} FROM tasks WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            TaskRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records
        .into_iter()
        .map(|mut record| {
            record.blocked_by = blockers_by_task.remove(&record.id).unwrap_or_default();
            record.into_task()
        })
        .collect()
}

/// Writes the rows of one task with its dependencies.
fn write_rows(transaction: &Transaction<'_>, record: &TaskRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_TASK,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":area_id": record.area_id,
            ":number": record.number,
            ":title": record.title,
            ":description": record.description,
            ":status_id": record.status_id,
            ":priority": record.priority,
            ":assignee_kind": record.assignee_kind,
            ":assignee_member_id": record.assignee_member_id,
            ":parent_id": record.parent_id,
            ":estimate_minutes": record.estimate_minutes,
            ":sprint_id": record.sprint_id,
            ":points": record.points,
            ":start_on": record.start_on,
            ":due_on": record.due_on,
            ":rank": record.rank,
            ":created_at": record.created_at,
            ":updated_at": record.updated_at,
            ":completed_at": record.completed_at,
        },
    )?;

    transaction.execute(
        "DELETE FROM task_dependencies WHERE task_id = :task_id",
        named_params! { ":task_id": record.id },
    )?;
    for blocker_id in &record.blocked_by {
        transaction.execute(
            "INSERT INTO task_dependencies (task_id, blocker_id) VALUES (:task_id, :blocker_id)",
            named_params! { ":task_id": record.id, ":blocker_id": blocker_id },
        )?;
    }
    Ok(())
}

/// Reads the identifiers of the tasks one task waits for.
fn blockers_of(connection: &Connection, task_id: &str) -> Result<Vec<String>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT blocker_id FROM task_dependencies WHERE task_id = :task_id ORDER BY blocker_id",
    )?;
    let blockers = statement
        .query_map(named_params! { ":task_id": task_id }, |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(blockers)
}

/// Reads, for every task of a project that waits for something, the
/// identifiers of the tasks it waits for.
fn blockers_in_project(
    connection: &Connection,
    project_id: &str,
) -> Result<HashMap<String, Vec<String>>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT dependency.task_id, dependency.blocker_id
         FROM task_dependencies AS dependency
         JOIN tasks AS task ON task.id = dependency.task_id
         WHERE task.project_id = :project_id",
    )?;
    let rows = statement.query_map(named_params! { ":project_id": project_id }, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut blockers_by_task: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (task_id, blocker_id) = row?;
        blockers_by_task
            .entry(task_id)
            .or_default()
            .push(blocker_id);
    }
    Ok(blockers_by_task)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_logged_before_there_were_sprints_is_read_into_the_backlog_without_points() {
        let logged = r#"{
            "id": "0190a000-0000-7000-8000-000000000001",
            "project_id": "0190a000-0000-7000-8000-000000000002",
            "number": 1,
            "title": "Design the board",
            "description": "",
            "status_id": "0190a000-0000-7000-8000-000000000003",
            "priority": "high",
            "assignee_kind": null,
            "assignee_member_id": null,
            "parent_id": null,
            "estimate_minutes": 240,
            "start_on": null,
            "due_on": null,
            "rank": 0.0,
            "created_at": "2026-11-11T09:00:00.000Z",
            "updated_at": "2026-11-11T09:00:00.000Z",
            "completed_at": null,
            "blocked_by": []
        }"#;

        let record: TaskRecord = serde_json::from_str(logged).unwrap();
        let task = record.into_task().unwrap();

        assert_eq!(task.title().as_str(), "Design the board");
        assert_eq!(task.estimate_minutes(), Some(240));
        assert_eq!(task.sprint_id(), None);
        assert_eq!(task.points(), None);
    }

    #[test]
    fn a_task_logged_before_there_were_areas_is_read_into_the_area_its_project_started_with() {
        let logged = r#"{
            "id": "0190a000-0000-7000-8000-000000000001",
            "project_id": "0190a000-0000-7000-8000-000000000002",
            "number": 1,
            "title": "Design the board",
            "description": "",
            "status_id": "0190a000-0000-7000-8000-000000000003",
            "priority": "normal",
            "assignee_kind": null,
            "assignee_member_id": null,
            "parent_id": null,
            "estimate_minutes": null,
            "start_on": null,
            "due_on": null,
            "rank": 0.0,
            "created_at": "2026-11-11T09:00:00.000Z",
            "updated_at": "2026-11-11T09:00:00.000Z",
            "completed_at": null,
            "blocked_by": []
        }"#;

        let record: TaskRecord = serde_json::from_str(logged).unwrap();
        let task = record.into_task().unwrap();

        assert_eq!(
            task.area_id(),
            Area::starting_id(task.project_id()),
            "the area \"General\" the migration gave the project"
        );
        assert_eq!(
            task.area_id().to_string(),
            "0190a000-0000-0000-8000-000000000002"
        );
    }
}
