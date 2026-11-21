//! Stores tasks together with their dependencies, their tags and their
//! checklists.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{StorageError, TaskRepository};
use teamwright_domain::{
    Area, Assignee, ChecklistItem, ChecklistText, ComputerId, DomainError, FieldId, FieldValue,
    ProjectId, Rank, Schedule, StoryPoints, TagId, Task, TaskId, TaskSnapshot, TaskTitle,
    Timestamp,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::fields;
use crate::merge::MergedState;
use crate::operations::{self, Entity, OperationRecorder};

/// The start of the key of a tag in the logged state of a task:
/// `tag:<identifier of the tag>`, true while the task carries the tag and
/// false once it was taken off (decision 0030).
pub(crate) const TAG_KEY: &str = "tag:";
/// The start of the key of an item of the checklist in the logged state of
/// a task: `item:<identifier of the item>`, the item as an object, or null
/// once it was removed.
pub(crate) const ITEM_KEY: &str = "item:";
/// The start of the key of the value of a custom field in the logged state
/// of a task: `field:<identifier of the field>`, the value as
/// [`fields::value_to_json`] writes it, or null once it was cleared
/// (decision 0035).
pub(crate) const FIELD_KEY: &str = "field:";

/// An item of a checklist as the operation log keeps it, under its key in
/// the state of its task.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemRecord {
    text: String,
    done: bool,
    rank: f64,
}

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
    /// The tags and the items of the checklist, one key each: see
    /// [`TAG_KEY`] and [`ITEM_KEY`]. Read from a state, this also holds the
    /// keys a newer version wrote, which are passed over.
    #[serde(flatten)]
    parts: BTreeMap<String, Value>,
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
            parts: parts_of(task),
        }
    }

    /// Reads the columns of a `tasks` row. The dependencies, the tags and
    /// the checklist live in other tables and are added by the caller.
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
            parts: BTreeMap::new(),
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

        let (tags, checklist) = self.tags_and_checklist()?;
        let field_values = self.field_values()?;
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
            tags,
            checklist,
            field_values,
            created_at: codec::moment_from_text(&self.created_at)?,
            updated_at: codec::moment_from_text(&self.updated_at)?,
            completed_at: self
                .completed_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        }))
    }

    /// Reads the tags the task carries and the items of its checklist from
    /// their keys. A tag taken off and an item removed are passed over, and
    /// so is a key a newer version wrote.
    fn tags_and_checklist(&self) -> Result<(BTreeSet<TagId>, Vec<ChecklistItem>), DatabaseError> {
        let mut tags = BTreeSet::new();
        let mut checklist = Vec::new();
        for (key, value) in &self.parts {
            if let Some(tag_id) = key.strip_prefix(TAG_KEY) {
                if *value == Value::Bool(true) {
                    tags.insert(codec::parse(tag_id)?);
                }
            } else if let Some(item_id) = key.strip_prefix(ITEM_KEY)
                && !value.is_null()
            {
                let item: ItemRecord = serde_json::from_value(value.clone()).map_err(|reason| {
                    DatabaseError::damaged("an item of a checklist", &value.to_string(), reason)
                })?;
                let text = ChecklistText::parse(&item.text)
                    .map_err(|reason| codec::damaged_record("task", &self.id, &reason))?;
                checklist.push(ChecklistItem::restore(
                    codec::parse(item_id)?,
                    text,
                    item.done,
                    Rank::from_value(item.rank),
                ));
            }
        }
        Ok((tags, checklist))
    }

    /// Reads the values of the custom fields from their keys. A value
    /// cleared is passed over.
    fn field_values(&self) -> Result<BTreeMap<FieldId, FieldValue>, DatabaseError> {
        let mut values = BTreeMap::new();
        for (key, value) in &self.parts {
            if let Some(field_id) = key.strip_prefix(FIELD_KEY)
                && !value.is_null()
            {
                values.insert(codec::parse(field_id)?, fields::value_from_json(value)?);
            }
        }
        Ok(values)
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
                // The handoffs of the task go with it, each logged.
                crate::teamwork::delete_handoffs_of_task(transaction, operations, &id)?;
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

    fn deleted_at(&self, id: TaskId) -> Result<Option<Timestamp>, StorageError> {
        self.database.read(|connection| {
            // The log keeps the deletion with the stamp of the moment it was
            // made, also when another computer made it.
            Ok(
                operations::deleted_millis_of(connection, Entity::Task, &id.to_string())?
                    .and_then(chrono::DateTime::from_timestamp_millis),
            )
        })
    }

    fn ids_ever_in_project(&self, project_id: ProjectId) -> Result<Vec<TaskId>, StorageError> {
        self.database.read(|connection| {
            // Every state of a task in the log names its project, so the
            // log knows the tasks that were deleted since as well.
            let mut statement = connection.prepare(
                "SELECT DISTINCT entity_id FROM operations
                 WHERE entity = :entity AND payload IS NOT NULL AND waiting IS NULL
                   AND json_extract(payload, '$.project_id') = :project_id",
            )?;
            let ids: Vec<String> = statement
                .query_map(
                    named_params! {
                        ":entity": Entity::Task.as_str(),
                        ":project_id": project_id.to_string(),
                    },
                    |row| row.get(0),
                )?
                .collect::<Result<_, _>>()?;
            ids.iter().map(|id| codec::parse(id)).collect()
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

/// Writes the stored form of one task with its dependencies, its tags and
/// its checklist, and logs the change.
///
/// A tag the task no longer carries, and an item it no longer has, are
/// logged as such (false, null): a key left out of a logged state keeps its
/// value, so that each tag and each item merges on its own.
fn store(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    record: &TaskRecord,
) -> Result<(), DatabaseError> {
    write_rows(transaction, record)?;
    let mut logged = record.clone();
    if let Some(state) =
        MergedState::read(transaction, Entity::Task, &record.id)?.and_then(|merged| merged.state)
    {
        for (key, value) in state {
            if logged.parts.contains_key(&key) {
                continue;
            }
            if key.starts_with(TAG_KEY) && value == Value::Bool(true) {
                logged.parts.insert(key, Value::Bool(false));
            } else if (key.starts_with(ITEM_KEY) || key.starts_with(FIELD_KEY)) && !value.is_null()
            {
                logged.parts.insert(key, Value::Null);
            }
        }
    }
    operations.record_upsert(Entity::Task, &record.id, &logged)
}

/// The keys of the tags a task carries and of the items of its checklist,
/// as the log keeps them.
fn parts_of(task: &Task) -> BTreeMap<String, Value> {
    let tags = task
        .tags()
        .iter()
        .map(|tag| (format!("{TAG_KEY}{tag}"), Value::Bool(true)));
    let items = task.checklist().items().iter().map(|item| {
        (
            format!("{ITEM_KEY}{}", item.id()),
            item_value(item.text().as_str(), item.is_done(), item.rank().value()),
        )
    });
    let values = task
        .field_values()
        .iter()
        .map(|(field, value)| (format!("{FIELD_KEY}{field}"), fields::value_to_json(value)));
    tags.chain(items).chain(values).collect()
}

/// An item of a checklist as the log keeps it.
fn item_value(text: &str, done: bool, rank: f64) -> Value {
    serde_json::json!({ "text": text, "done": done, "rank": rank })
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
    record.parts = parts_in(connection, "task.id = :id", id)?
        .remove(&record.id)
        .unwrap_or_default();
    record.into_task().map(Some)
}

/// Reads the tasks of a project inside a transaction, in no particular
/// order.
pub(crate) fn tasks_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Task>, DatabaseError> {
    let mut blockers_by_task = blockers_in_project(connection, project_id)?;
    let mut parts_by_task = parts_in(connection, "task.project_id = :id", project_id)?;
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
            record.parts = parts_by_task.remove(&record.id).unwrap_or_default();
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
    write_tags_and_checklist(transaction, record)
}

/// Writes the rows of the tags a task carries and of the items of its
/// checklist. Rows are added and updated in place, and the rows of a tag
/// taken off or of an item removed are deleted.
fn write_tags_and_checklist(
    transaction: &Transaction<'_>,
    record: &TaskRecord,
) -> Result<(), DatabaseError> {
    let (tags, checklist) = record.tags_and_checklist()?;
    let tags: Vec<String> = tags.iter().map(ToString::to_string).collect();
    let item_ids: Vec<String> = checklist.iter().map(|item| item.id().to_string()).collect();
    let as_json_list = |ids: &[String]| Value::from(ids.to_vec()).to_string();

    transaction.execute(
        "DELETE FROM task_tags WHERE task_id = :task_id
           AND tag_id NOT IN (SELECT value FROM json_each(:kept))",
        named_params! { ":task_id": record.id, ":kept": as_json_list(&tags) },
    )?;
    for tag_id in &tags {
        transaction.execute(
            "INSERT INTO task_tags (task_id, tag_id) VALUES (:task_id, :tag_id)
             ON CONFLICT (task_id, tag_id) DO NOTHING",
            named_params! { ":task_id": record.id, ":tag_id": tag_id },
        )?;
    }

    transaction.execute(
        "DELETE FROM checklist_items WHERE task_id = :task_id
           AND id NOT IN (SELECT value FROM json_each(:kept))",
        named_params! { ":task_id": record.id, ":kept": as_json_list(&item_ids) },
    )?;
    for item in &checklist {
        transaction.execute(
            "INSERT INTO checklist_items (id, task_id, text, done, rank)
             VALUES (:id, :task_id, :text, :done, :rank)
             ON CONFLICT (id) DO UPDATE SET
                 task_id = excluded.task_id,
                 text = excluded.text,
                 done = excluded.done,
                 rank = excluded.rank",
            named_params! {
                ":id": item.id().to_string(),
                ":task_id": record.id,
                ":text": item.text().as_str(),
                ":done": i64::from(item.is_done()),
                ":rank": item.rank().value(),
            },
        )?;
    }
    write_field_values(transaction, record)
}

/// Writes the rows of the values a task holds for the custom fields of its
/// project. Rows are added and updated in place, and the row of a value
/// cleared is deleted.
fn write_field_values(
    transaction: &Transaction<'_>,
    record: &TaskRecord,
) -> Result<(), DatabaseError> {
    let values = record.field_values()?;
    let field_ids: Vec<String> = values.keys().map(ToString::to_string).collect();
    transaction.execute(
        "DELETE FROM task_field_values WHERE task_id = :task_id
           AND field_id NOT IN (SELECT value FROM json_each(:kept))",
        named_params! { ":task_id": record.id, ":kept": Value::from(field_ids).to_string() },
    )?;
    for (field_id, value) in &values {
        transaction.execute(
            "INSERT INTO task_field_values (task_id, field_id, value)
             VALUES (:task_id, :field_id, :value)
             ON CONFLICT (task_id, field_id) DO UPDATE SET value = excluded.value",
            named_params! {
                ":task_id": record.id,
                ":field_id": field_id.to_string(),
                ":value": fields::value_to_json(value).to_string(),
            },
        )?;
    }
    Ok(())
}

/// Reads, for every task a condition on `task` picks, the keys of the tags
/// it carries and of the items of its checklist, as the log keeps them.
/// The condition names one parameter, `:id`.
fn parts_in(
    connection: &Connection,
    condition: &str,
    id: &str,
) -> Result<HashMap<String, BTreeMap<String, Value>>, DatabaseError> {
    let mut parts: HashMap<String, BTreeMap<String, Value>> = HashMap::new();

    let query = format!(
        "SELECT carried.task_id, carried.tag_id FROM task_tags AS carried
         JOIN tasks AS task ON task.id = carried.task_id
         WHERE {condition}"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(named_params! { ":id": id }, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (task_id, tag_id) = row?;
        parts
            .entry(task_id)
            .or_default()
            .insert(format!("{TAG_KEY}{tag_id}"), Value::Bool(true));
    }

    let query = format!(
        "SELECT item.task_id, item.id, item.text, item.done, item.rank
         FROM checklist_items AS item
         JOIN tasks AS task ON task.id = item.task_id
         WHERE {condition}"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(named_params! { ":id": id }, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)? != 0,
            row.get::<_, f64>(4)?,
        ))
    })?;
    for row in rows {
        let (task_id, item_id, text, done, rank) = row?;
        parts.entry(task_id).or_default().insert(
            format!("{ITEM_KEY}{item_id}"),
            item_value(&text, done, rank),
        );
    }

    let query = format!(
        "SELECT held.task_id, held.field_id, held.value
         FROM task_field_values AS held
         JOIN tasks AS task ON task.id = held.task_id
         WHERE {condition}"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(named_params! { ":id": id }, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (task_id, field_id, value) = row?;
        let value: Value = serde_json::from_str(&value)
            .map_err(|reason| DatabaseError::damaged("a value of a field", &value, reason))?;
        parts
            .entry(task_id)
            .or_default()
            .insert(format!("{FIELD_KEY}{field_id}"), value);
    }
    Ok(parts)
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
