//! Takes in the operations of another computer
//! ([decision 0017](../../../docs/decisions/0017-sync-through-a-folder.md),
//! sections 2 and 3).
//!
//! Everything happens in one transaction: the operations are stored in the
//! log with the computer that made them and their number there, merged
//! field by field into the merged state of their thing, and the rows are
//! written from the merged state. An operation that refers to something
//! that has not arrived yet waits, and is tried again in every round. What
//! breaks a rule across records is repaired afterwards with ordinary logged
//! writes of this computer, which go out in the next round like any other
//! change.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use rusqlite::{ErrorCode, OptionalExtension, Transaction, named_params};
use serde_json::{Map, Value};
use teamwright_application::{ExchangedChange, ExchangedOperation, NoticeKind, TakeInReport};
use teamwright_domain::{
    Area, AreaName, Assignee, Project, Sprint, Status, StatusCategory, Task, TaskId, View,
    ViewAssignee, ViewName,
};

use crate::apply;
use crate::areas;
use crate::codec;
use crate::conversations;
use crate::error::DatabaseError;
use crate::merge::{MergedState, Rule, Stamps, all_fields, object_from_text, stamps_from_text};
use crate::notices;
use crate::operations::{Entity, OperationRecorder, Stamp, latest_changes_seen, wall_clock_millis};
use crate::projects;
use crate::sprints;
use crate::tasks;
use crate::views;

/// Why an operation waits: it refers to something that has not arrived.
pub(crate) const WAITING_FOR_PARENT: &str = "parent";
/// Why an operation waits: a newer version wrote an entity this one does not
/// know.
pub(crate) const UNKNOWN_ENTITY: &str = "unknown entity";
/// Why an operation is not applied: its state breaks a rule.
pub(crate) const DAMAGED: &str = "damaged";
/// Why an operation is not applied: it describes another computer.
pub(crate) const NEVER: &str = "never";

/// The kinds of things whose dropped changes the manager is told about.
const TOLD_WHEN_DROPPED: [Entity; 7] = [
    Entity::Project,
    Entity::Task,
    Entity::Member,
    Entity::Sprint,
    Entity::View,
    Entity::Area,
    Entity::Conversation,
];

/// An operation of another computer, read from the folder or from the log.
#[derive(Debug, Clone)]
struct Incoming {
    stamp: String,
    entity: String,
    entity_id: String,
    change: Change,
}

/// What an operation of another computer does.
#[derive(Debug, Clone)]
enum Change {
    Upsert {
        state: Map<String, Value>,
        field_stamps: Stamps,
    },
    Delete {
        seen: Stamps,
    },
}

/// What became of an operation that was tried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// It was applied, or it changed nothing, or it was dropped because its
    /// thing was deleted.
    Done,
    /// It waits, for the reason given.
    Waits(&'static str),
}

/// Whether something an operation refers to is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Present,
    Deleted,
    Unknown,
}

/// What one take-in has done so far.
#[derive(Debug, Clone, Default)]
struct Round {
    /// The identity of this computer, as text.
    this_computer: String,
    /// The things a deletion of this batch removes, which a deletion of
    /// something else must not repair first.
    deleted_in_batch: BTreeSet<(String, String)>,
    taken_in: usize,
    projects: BTreeSet<String>,
    conversations: BTreeSet<String>,
    turns_to_stop: BTreeSet<String>,
    notices: usize,
}

/// Takes in operations of other computers, in one transaction.
pub(crate) fn take_in(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    incoming: &[ExchangedOperation],
) -> Result<TakeInReport, DatabaseError> {
    let mut round = Round {
        this_computer: operations.device_id().to_string(),
        deleted_in_batch: incoming
            .iter()
            .filter(|operation| matches!(operation.change, ExchangedChange::Delete { .. }))
            .map(|operation| (operation.entity.clone(), operation.entity_id.clone()))
            .collect(),
        ..Round::default()
    };
    let mut sorted: Vec<&ExchangedOperation> = incoming.iter().collect();
    sorted.sort_by(|one, other| one.stamp.cmp(&other.stamp));

    let mut latest_millis = None;
    for exchanged in sorted {
        let Some(sequence) = store(transaction, exchanged)? else {
            continue;
        };
        round.taken_in += 1;
        operations.witness(&exchanged.stamp);
        if let Some(stamp) = Stamp::from_text(&exchanged.stamp) {
            latest_millis = latest_millis.max(Some(stamp.millis()));
        }
        let incoming = Incoming::from_exchanged(exchanged)?;
        let outcome = round.try_apply(transaction, operations, &incoming)?;
        mark(transaction, sequence, outcome)?;
    }
    if round.taken_in > 0 {
        round.retry_waiting(transaction, operations)?;
        round.repair(transaction, operations)?;
    }

    Ok(TakeInReport {
        taken_in: round.taken_in,
        changed_projects: parse_all(&round.projects)?,
        changed_conversations: parse_all(&round.conversations)?,
        turns_to_stop: parse_all(&round.turns_to_stop)?,
        notices: round.notices,
        clock_ahead_by_millis: latest_millis
            .map(|latest| latest - wall_clock_millis())
            .filter(|ahead| *ahead > 0),
    })
}

/// Reads identifiers from their text forms.
fn parse_all<T>(texts: &BTreeSet<String>) -> Result<Vec<T>, DatabaseError>
where
    T: std::str::FromStr<Err = teamwright_domain::DomainError>,
{
    texts.iter().map(|text| codec::parse(text)).collect()
}

/// Stores an operation of another computer in the log, and returns its
/// position there, or nothing if the log has it already.
fn store(
    transaction: &Transaction<'_>,
    operation: &ExchangedOperation,
) -> Result<Option<i64>, DatabaseError> {
    let sequence = i64::try_from(operation.sequence).map_err(|reason| {
        DatabaseError::damaged(
            "the number of an operation",
            &operation.sequence.to_string(),
            reason,
        )
    })?;
    let (kind, payload, field_stamps, seen) = match &operation.change {
        ExchangedChange::Upsert {
            state,
            field_stamps,
        } => (
            "upsert",
            Some(state.as_str()),
            field_stamps.as_deref(),
            None,
        ),
        ExchangedChange::Delete { seen } => ("delete", None, None, seen.as_deref()),
    };
    let inserted = transaction.execute(
        "INSERT OR IGNORE INTO operations (
             device_id, device_sequence, stamp, entity, entity_id, kind, payload,
             field_stamps, seen
         )
         VALUES (
             :device_id, :device_sequence, :stamp, :entity, :entity_id, :kind, :payload,
             :field_stamps, :seen
         )",
        named_params! {
            ":device_id": operation.computer.to_string(),
            ":device_sequence": sequence,
            ":stamp": operation.stamp,
            ":entity": operation.entity,
            ":entity_id": operation.entity_id,
            ":kind": kind,
            ":payload": payload,
            ":field_stamps": field_stamps,
            ":seen": seen,
        },
    )?;
    if inserted == 0 {
        return Ok(None);
    }
    Ok(Some(transaction.last_insert_rowid()))
}

/// Records whether an operation in the log is applied or waits.
fn mark(
    transaction: &Transaction<'_>,
    sequence: i64,
    outcome: Outcome,
) -> Result<(), DatabaseError> {
    let waiting = match outcome {
        Outcome::Done => None,
        Outcome::Waits(reason) => Some(reason),
    };
    transaction.execute(
        "UPDATE operations SET waiting = :waiting WHERE sequence = :sequence",
        named_params! { ":waiting": waiting, ":sequence": sequence },
    )?;
    Ok(())
}

impl Incoming {
    /// Reads an operation as it arrived.
    fn from_exchanged(operation: &ExchangedOperation) -> Result<Self, DatabaseError> {
        let change = match &operation.change {
            ExchangedChange::Upsert {
                state,
                field_stamps,
            } => Change::Upsert {
                state: object_from_text(state, "a state from another computer")?,
                field_stamps: field_stamps
                    .as_deref()
                    .map(stamps_from_text)
                    .transpose()?
                    .unwrap_or_default(),
            },
            ExchangedChange::Delete { seen } => Change::Delete {
                seen: seen
                    .as_deref()
                    .map(stamps_from_text)
                    .transpose()?
                    .unwrap_or_default(),
            },
        };
        Ok(Self {
            stamp: operation.stamp.clone(),
            entity: operation.entity.clone(),
            entity_id: operation.entity_id.clone(),
            change,
        })
    }
}

/// The operations of the log that wait for something to arrive, the oldest
/// first, with their positions in the log.
fn waiting_for_parent(
    transaction: &Transaction<'_>,
) -> Result<Vec<(i64, Incoming)>, DatabaseError> {
    let mut statement = transaction.prepare(
        "SELECT sequence, stamp, entity, entity_id, payload, field_stamps, seen
         FROM operations WHERE waiting = :waiting ORDER BY stamp",
    )?;
    let rows = statement
        .query_map(named_params! { ":waiting": WAITING_FOR_PARENT }, |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(
            |(sequence, stamp, entity, entity_id, payload, field_stamps, seen)| {
                let change = match payload {
                    Some(state) => Change::Upsert {
                        state: object_from_text(&state, "a state from another computer")?,
                        field_stamps: field_stamps
                            .as_deref()
                            .map(stamps_from_text)
                            .transpose()?
                            .unwrap_or_default(),
                    },
                    None => Change::Delete {
                        seen: seen
                            .as_deref()
                            .map(stamps_from_text)
                            .transpose()?
                            .unwrap_or_default(),
                    },
                };
                Ok((
                    sequence,
                    Incoming {
                        stamp,
                        entity,
                        entity_id,
                        change,
                    },
                ))
            },
        )
        .collect()
}

/// Whether an error means that an operation cannot be applied (its state is
/// damaged, or the database refuses its rows), rather than that the
/// database failed.
fn is_refusal(error: &DatabaseError) -> bool {
    match error {
        DatabaseError::Damaged(_) | DatabaseError::Deleted { .. } => true,
        DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(failure, _)) => {
            failure.code == ErrorCode::ConstraintViolation
        }
        _ => false,
    }
}

/// Whether something is there, deleted, or not known yet.
fn presence(
    transaction: &Transaction<'_>,
    entity: Entity,
    id: &str,
) -> Result<Presence, DatabaseError> {
    Ok(match MergedState::read(transaction, entity, id)? {
        Some(merged) if merged.deleted.is_some() => Presence::Deleted,
        Some(merged) if merged.state.is_some() => Presence::Present,
        _ => Presence::Unknown,
    })
}

/// The text of a key of a state, if it has one.
fn text_of<'state>(state: &'state Map<String, Value>, key: &str) -> Option<&'state str> {
    state.get(key).and_then(Value::as_str)
}

/// The things a thing cannot exist without: a task without its project, a
/// step without its turn.
fn parents_of(
    entity: Entity,
    entity_id: &str,
    state: &Map<String, Value>,
) -> Vec<(Entity, String)> {
    let parent = match entity {
        Entity::Task
        | Entity::Member
        | Entity::Sprint
        | Entity::View
        | Entity::Area
        | Entity::Conversation => {
            text_of(state, "project_id").map(|id| (Entity::Project, id.to_owned()))
        }
        Entity::Turn => {
            text_of(state, "conversation_id").map(|id| (Entity::Conversation, id.to_owned()))
        }
        Entity::TurnStep => entity_id
            .rsplit_once('/')
            .map(|(turn_id, _)| (Entity::Turn, turn_id.to_owned())),
        Entity::Decision | Entity::Interview | Entity::Proposal => {
            Some((Entity::Turn, entity_id.to_owned()))
        }
        Entity::Project | Entity::ProjectPlaces => None,
    };
    parent.into_iter().collect()
}

/// The other things a state refers to: what a task waits for, its parent,
/// who it is given to, its sprint; the member and the task of a
/// conversation; the member whose tasks a view shows.
fn references_of(entity: Entity, state: &Map<String, Value>) -> Vec<(Entity, String)> {
    let mut references = Vec::new();
    match entity {
        Entity::Task => {
            if let Some(parent) = text_of(state, "parent_id") {
                references.push((Entity::Task, parent.to_owned()));
            }
            if let Some(Value::Array(blockers)) = state.get("blocked_by") {
                references.extend(
                    blockers
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|blocker| (Entity::Task, blocker.to_owned())),
                );
            }
            if text_of(state, "assignee_kind") == Some("member")
                && let Some(member) = text_of(state, "assignee_member_id")
            {
                references.push((Entity::Member, member.to_owned()));
            }
            if let Some(sprint) = text_of(state, "sprint_id") {
                references.push((Entity::Sprint, sprint.to_owned()));
            }
            if let Some(area) = text_of(state, "area_id") {
                references.push((Entity::Area, area.to_owned()));
            }
        }
        Entity::Conversation => {
            if let Some(member) = text_of(state, "member_id") {
                references.push((Entity::Member, member.to_owned()));
            }
            if let Some(task) = text_of(state, "task_id") {
                references.push((Entity::Task, task.to_owned()));
            }
        }
        Entity::View => {
            if text_of(state, "assignee_kind") == Some("member")
                && let Some(member) = text_of(state, "assignee_member_id")
            {
                references.push((Entity::Member, member.to_owned()));
            }
        }
        _ => {}
    }
    references
}

/// Merges an incoming state into the merged state of its thing, field by
/// field. A field takes the incoming value if its stamp is later; a field
/// whose larger value wins takes the larger one. The result is the same in
/// any order and however often it is done.
fn merge_incoming(
    entity: Entity,
    base: Option<MergedState>,
    incoming: &Map<String, Value>,
    incoming_stamps: &Stamps,
    stamp: &str,
) -> MergedState {
    let stamp_of = |name: &str| {
        incoming_stamps
            .get(name)
            .cloned()
            .unwrap_or_else(|| stamp.to_owned())
    };
    let Some(mut merged) = base else {
        let stamps = all_fields(entity, incoming)
            .into_iter()
            .map(|(name, _, _)| {
                let field_stamp = stamp_of(&name);
                (name, field_stamp)
            })
            .collect();
        return MergedState {
            state: Some(incoming.clone()),
            stamps,
            deleted: None,
            deleted_seen: Stamps::new(),
        };
    };
    let mut state = merged.state.take().unwrap_or_default();
    for (name, keys, rule) in all_fields(entity, incoming) {
        let theirs = stamp_of(&name);
        let ours = merged.stamps.get(&name).cloned();
        let take = match (rule, &ours) {
            (_, None) => true,
            (Rule::Later, Some(ours)) => theirs > *ours,
            (Rule::Larger, Some(_)) => keys
                .iter()
                .any(|key| is_larger(incoming.get(key), state.get(key))),
        };
        if take {
            for key in &keys {
                if let Some(value) = incoming.get(key) {
                    state.insert(key.clone(), value.clone());
                }
            }
        }
        let field_stamp = match (rule, ours) {
            (Rule::Larger, Some(ours)) => ours.max(theirs),
            (Rule::Later, Some(ours)) if !take => ours,
            _ => theirs,
        };
        merged.stamps.insert(name, field_stamp);
    }
    merged.state = Some(state);
    merged
}

/// Whether a value is larger than another: numbers by their value, text in
/// its order. Nothing is smaller than anything.
fn is_larger(one: Option<&Value>, other: Option<&Value>) -> bool {
    match (one, other) {
        (Some(Value::Number(one)), Some(Value::Number(other))) => {
            one.as_f64().unwrap_or_default() > other.as_f64().unwrap_or_default()
        }
        (Some(Value::String(one)), Some(Value::String(other))) => one > other,
        (Some(value), None | Some(Value::Null)) => !value.is_null(),
        _ => false,
    }
}

impl Round {
    /// Tries to apply one operation. If its state is damaged, or the
    /// database refuses it, nothing of it is kept and it is marked as
    /// damaged; the batch goes on.
    fn try_apply(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        incoming: &Incoming,
    ) -> Result<Outcome, DatabaseError> {
        let Some(entity) = Entity::from_name(&incoming.entity) else {
            return Ok(Outcome::Waits(UNKNOWN_ENTITY));
        };
        if entity == Entity::ProjectPlaces {
            return Ok(Outcome::Waits(NEVER));
        }
        let before = self.clone();
        transaction.execute_batch("SAVEPOINT taking_in")?;
        let result = match &incoming.change {
            Change::Upsert {
                state,
                field_stamps,
            } => self.apply_upsert(
                transaction,
                operations,
                entity,
                incoming,
                state,
                field_stamps,
            ),
            Change::Delete { seen } => {
                self.apply_delete(transaction, operations, entity, incoming, seen)
            }
        };
        match result {
            Ok(outcome) => {
                transaction.execute_batch("RELEASE taking_in")?;
                Ok(outcome)
            }
            Err(error) if is_refusal(&error) => {
                transaction.execute_batch("ROLLBACK TO taking_in; RELEASE taking_in")?;
                *self = before;
                Ok(Outcome::Waits(DAMAGED))
            }
            Err(error) => Err(error),
        }
    }

    /// Tries the operations that wait for something to arrive again, until
    /// a pass makes no progress.
    fn retry_waiting(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
    ) -> Result<(), DatabaseError> {
        loop {
            let mut progress = false;
            for (sequence, incoming) in waiting_for_parent(transaction)? {
                let outcome = self.try_apply(transaction, operations, &incoming)?;
                if outcome != Outcome::Waits(WAITING_FOR_PARENT) {
                    mark(transaction, sequence, outcome)?;
                    progress = true;
                }
            }
            if !progress {
                return Ok(());
            }
        }
    }

    /// Merges a state from another computer and writes the rows of the
    /// merged state.
    fn apply_upsert(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        entity: Entity,
        incoming: &Incoming,
        state: &Map<String, Value>,
        field_stamps: &Stamps,
    ) -> Result<Outcome, DatabaseError> {
        let id = incoming.entity_id.as_str();
        let old = MergedState::read(transaction, entity, id)?;
        if let Some(deleted) = old.as_ref().and_then(|old| old.deleted.as_deref()) {
            // A deletion is final; only a step of a turn, whose identifier
            // is its position, can be written again by a later change.
            let written_again = entity == Entity::TurnStep && incoming.stamp.as_str() > deleted;
            if !written_again {
                return Ok(Outcome::Done);
            }
        }
        for (parent, parent_id) in parents_of(entity, id, state) {
            match presence(transaction, parent, &parent_id)? {
                Presence::Present => {}
                Presence::Deleted => return Ok(Outcome::Done),
                Presence::Unknown => return Ok(Outcome::Waits(WAITING_FOR_PARENT)),
            }
        }
        let base = old.filter(|old| old.deleted.is_none() && old.state.is_some());
        let merged = merge_incoming(entity, base.clone(), state, field_stamps, &incoming.stamp);
        if base.as_ref() == Some(&merged) {
            return Ok(Outcome::Done);
        }
        let merged_state = merged.state.clone().unwrap_or_default();
        for (reference, reference_id) in references_of(entity, &merged_state) {
            if presence(transaction, reference, &reference_id)? == Presence::Unknown {
                return Ok(Outcome::Waits(WAITING_FOR_PARENT));
            }
        }
        merged.write(transaction, entity, id)?;
        self.note_change(transaction, entity, id, Some(&merged_state))?;
        self.write_rows(transaction, operations, entity, id, &merged_state)?;
        Ok(Outcome::Done)
    }

    /// Writes the rows of a merged state. A reference to something that was
    /// deleted, or to a status the workflow no longer has, is repaired as
    /// the rows are written, with a logged change of this computer.
    fn write_rows(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        entity: Entity,
        id: &str,
        state: &Map<String, Value>,
    ) -> Result<(), DatabaseError> {
        match entity {
            Entity::Project => {
                let project = projects::write_state_keeping_statuses(transaction, state)?;
                self.move_tasks_out_of_removed_statuses(transaction, operations, &project)?;
                projects::remove_statuses_not_in_workflow(transaction, &project)
            }
            Entity::Task => {
                let mut task = tasks::task_from_state(state)?;
                if self.repair_references_of_task(transaction, &mut task)? {
                    tasks::store_task(transaction, operations, &task)
                } else {
                    tasks::write_state(transaction, state)
                }
            }
            Entity::Conversation => {
                let mut conversation = conversations::conversation_from_state(state)?;
                if let Some(task_id) = conversation.task_id()
                    && presence(transaction, Entity::Task, &task_id.to_string())?
                        == Presence::Deleted
                {
                    // A conversation about a task that is gone goes too, as
                    // it does when a task is deleted here. This computer
                    // deletes it, and the deletion goes out like any other.
                    self.note_turns_to_stop(transaction, Entity::Conversation, id)?;
                    conversations::remove_conversation(transaction, operations, id)?;
                    return operations.record_delete(Entity::Conversation, id);
                }
                if let Some(member_id) = conversation.member_id()
                    && presence(transaction, Entity::Member, &member_id.to_string())?
                        == Presence::Deleted
                {
                    conversation.forget_member();
                    return conversations::store_conversation(
                        transaction,
                        operations,
                        &conversation,
                    );
                }
                conversations::write_conversation_state(transaction, state)
            }
            Entity::View => {
                let mut view = views::view_from_state(state)?;
                if let ViewAssignee::Member(member_id) = view.filter().assignee
                    && presence(transaction, Entity::Member, &member_id.to_string())?
                        == Presence::Deleted
                {
                    // A view that shows only the tasks of a member who left
                    // the team on another computer shows everybody's again,
                    // as it does when the member leaves here.
                    view.forget_member(member_id);
                    return views::store_view(transaction, operations, &view);
                }
                views::write_state(transaction, state)
            }
            other => apply::write_state(transaction, other, state),
        }
    }

    /// Repairs what a task refers to that is gone: a status the workflow no
    /// longer has, a deleted parent, task it waits for, member or sprint.
    /// Says whether anything was repaired.
    fn repair_references_of_task(
        &mut self,
        transaction: &Transaction<'_>,
        task: &mut Task,
    ) -> Result<bool, DatabaseError> {
        let now = Utc::now();
        let mut repaired = false;
        let project_id = task.project_id().to_string();
        if let Some(project) = projects::find_project(transaction, &project_id)?
            && project.workflow().status(task.status_id()).is_none()
        {
            let category =
                category_of_status(transaction, &project_id, &task.status_id().to_string())?;
            let target = target_status(&project, category);
            task.move_to(&target, task.rank(), now);
            self.tell_moved(transaction, &project, task, &target)?;
            repaired = true;
        }
        if let Some(parent) = task.parent_id()
            && presence(transaction, Entity::Task, &parent.to_string())? == Presence::Deleted
        {
            task.set_parent(None, now)
                .map_err(|reason| codec::damaged_record("task", &task.id().to_string(), &reason))?;
            repaired = true;
        }
        let blockers: Vec<TaskId> = task.blocked_by().iter().copied().collect();
        for blocker in blockers {
            if presence(transaction, Entity::Task, &blocker.to_string())? == Presence::Deleted {
                task.unblock_from(blocker, now);
                repaired = true;
            }
        }
        if let Some(Assignee::Member(member)) = task.assignee()
            && presence(transaction, Entity::Member, &member.to_string())? == Presence::Deleted
        {
            task.assign(None, now);
            repaired = true;
        }
        if let Some(sprint) = task.sprint_id()
            && presence(transaction, Entity::Sprint, &sprint.to_string())? == Presence::Deleted
        {
            task.plan_into(None, now);
            repaired = true;
        }
        Ok(repaired)
    }

    /// Moves the tasks whose status a merged workflow no longer has into the
    /// first status of the same category, or else the first status, before
    /// the status is removed.
    fn move_tasks_out_of_removed_statuses(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project: &Project,
    ) -> Result<(), DatabaseError> {
        let project_id = project.id().to_string();
        let now = Utc::now();
        for mut task in tasks::tasks_of_project(transaction, &project_id)? {
            if project.workflow().status(task.status_id()).is_some() {
                continue;
            }
            let category =
                category_of_status(transaction, &project_id, &task.status_id().to_string())?;
            let target = target_status(project, category);
            task.move_to(&target, task.rank(), now);
            tasks::store_task(transaction, operations, &task)?;
            self.tell_moved(transaction, project, &task, &target)?;
        }
        Ok(())
    }

    /// Tells the manager that a task moved because its status was removed.
    fn tell_moved(
        &mut self,
        transaction: &Transaction<'_>,
        project: &Project,
        task: &Task,
        target: &Status,
    ) -> Result<(), DatabaseError> {
        self.tell(
            transaction,
            NoticeKind::TaskMoved,
            Some(&project.id().to_string()),
            Some(&task.id().to_string()),
            &format!(
                "{}-{} \"{}\" moved to \"{}\": its status was removed on another computer.",
                project.key(),
                task.number(),
                task.title(),
                target.name()
            ),
        )
    }

    /// Keeps a notice for the manager.
    fn tell(
        &mut self,
        transaction: &Transaction<'_>,
        kind: NoticeKind,
        project_id: Option<&str>,
        task_id: Option<&str>,
        message: &str,
    ) -> Result<(), DatabaseError> {
        notices::add(transaction, kind, project_id, task_id, message)?;
        self.notices += 1;
        Ok(())
    }

    /// Applies the deletion of a thing from another computer. What still
    /// refers to it is repaired first, with logged changes of this computer;
    /// what belongs to it and this computer still has is removed with
    /// logged deletions of this computer.
    fn apply_delete(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        entity: Entity,
        incoming: &Incoming,
        seen: &Stamps,
    ) -> Result<Outcome, DatabaseError> {
        let id = incoming.entity_id.as_str();
        let old = MergedState::read(transaction, entity, id)?;
        let mut merged = old.clone().unwrap_or(MergedState {
            state: None,
            stamps: Stamps::new(),
            deleted: None,
            deleted_seen: Stamps::new(),
        });
        if let Some(deleted) = merged.deleted.clone() {
            // Deleted on both: the earlier deletion, and all both had seen.
            merged.deleted = Some(deleted.min(incoming.stamp.clone()));
            for (computer, stamp) in seen {
                let kept = merged.deleted_seen.entry(computer.clone()).or_default();
                if stamp > kept {
                    kept.clone_from(stamp);
                }
            }
            if old.as_ref() != Some(&merged) {
                merged.write(transaction, entity, id)?;
            }
            return Ok(Outcome::Done);
        }
        if entity == Entity::TurnStep
            && merged
                .latest_stamp()
                .is_some_and(|latest| latest > incoming.stamp.as_str())
        {
            // The step was written again after this deletion.
            return Ok(Outcome::Done);
        }
        if merged.state.is_some() && TOLD_WHEN_DROPPED.contains(&entity) {
            self.tell_if_own_change_dropped(transaction, entity, id, merged.state.as_ref(), seen)?;
        }
        self.note_change(transaction, entity, id, merged.state.as_ref())?;
        self.note_turns_to_stop(transaction, entity, id)?;
        self.release_references_to(transaction, operations, entity, id)?;
        apply::remove(transaction, operations, entity, id)?;
        merged.deleted = Some(incoming.stamp.clone());
        merged.deleted_seen = seen.clone();
        merged.write(transaction, entity, id)?;
        Ok(Outcome::Done)
    }

    /// Tells the manager if a change this computer made to a thing is one
    /// that its deletion elsewhere threw away.
    fn tell_if_own_change_dropped(
        &mut self,
        transaction: &Transaction<'_>,
        entity: Entity,
        id: &str,
        state: Option<&Map<String, Value>>,
        seen: &Stamps,
    ) -> Result<(), DatabaseError> {
        let own = latest_changes_seen(transaction, entity, id)?.remove(&self.this_computer);
        let Some(own) = own else {
            return Ok(());
        };
        let unseen = seen
            .get(&self.this_computer)
            .is_none_or(|seen_there| own > *seen_there);
        if !unseen {
            return Ok(());
        }
        let name = state
            .and_then(|state| text_of(state, "title").or_else(|| text_of(state, "name")))
            .unwrap_or("it");
        let what = match entity {
            Entity::Project => format!("the project \"{name}\""),
            Entity::Task => format!("the task \"{name}\""),
            Entity::Member => format!("the member \"{name}\""),
            Entity::Sprint => format!("the sprint \"{name}\""),
            Entity::View => format!("the view \"{name}\""),
            Entity::Area => format!("the area \"{name}\""),
            _ => "a conversation".to_owned(),
        };
        let project_id = match entity {
            Entity::Project => Some(id.to_owned()),
            _ => state
                .and_then(|state| text_of(state, "project_id"))
                .map(str::to_owned),
        };
        let task_id = (entity == Entity::Task).then(|| id.to_owned());
        self.tell(
            transaction,
            NoticeKind::ChangeDropped,
            project_id.as_deref(),
            task_id.as_deref(),
            &format!(
                "A change you made to {what} was dropped: it was deleted on another computer."
            ),
        )
    }

    /// Repairs, with logged changes of this computer, what refers to a
    /// thing about to be deleted: the tasks, conversations and views of a
    /// member,
    /// the tasks of a sprint, the subtasks of a task, the tasks that wait
    /// for it and the conversations about it. What the same batch deletes
    /// itself is left to its own deletion.
    fn release_references_to(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        entity: Entity,
        id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let going = |kind: Entity, other: &str| {
            self.deleted_in_batch
                .contains(&(kind.as_str().to_owned(), other.to_owned()))
        };
        match entity {
            Entity::Task => {
                let project_id: Option<String> = transaction
                    .query_row(
                        "SELECT project_id FROM tasks WHERE id = :id",
                        named_params! { ":id": id },
                        |row| row.get(0),
                    )
                    .optional()?;
                let task_id: TaskId = codec::parse(id)?;
                if let Some(project_id) = project_id {
                    for mut other in tasks::tasks_of_project(transaction, &project_id)? {
                        if other.id() == task_id || going(Entity::Task, &other.id().to_string()) {
                            continue;
                        }
                        let mut changed = false;
                        if other.parent_id() == Some(task_id) {
                            other.set_parent(None, now).map_err(|reason| {
                                codec::damaged_record("task", &other.id().to_string(), &reason)
                            })?;
                            changed = true;
                        }
                        if other.blocked_by().contains(&task_id) {
                            other.unblock_from(task_id, now);
                            changed = true;
                        }
                        if changed {
                            tasks::store_task(transaction, operations, &other)?;
                        }
                    }
                }
                for conversation_id in conversations::conversations_about_task(transaction, id)? {
                    if !going(Entity::Conversation, &conversation_id) {
                        conversations::delete_conversation(
                            transaction,
                            operations,
                            &conversation_id,
                        )?;
                    }
                }
            }
            Entity::Member => {
                let member_id = codec::parse(id)?;
                let project_id: Option<String> = transaction
                    .query_row(
                        "SELECT project_id FROM members WHERE id = :id",
                        named_params! { ":id": id },
                        |row| row.get(0),
                    )
                    .optional()?;
                if let Some(project_id) = project_id {
                    for mut task in tasks::tasks_of_project(transaction, &project_id)? {
                        if task.assignee() == Some(Assignee::Member(member_id))
                            && !going(Entity::Task, &task.id().to_string())
                        {
                            task.assign(None, now);
                            tasks::store_task(transaction, operations, &task)?;
                        }
                    }
                }
                for mut conversation in conversations::conversations_with_member(transaction, id)? {
                    if !going(Entity::Conversation, &conversation.id().to_string()) {
                        conversation.forget_member();
                        conversations::store_conversation(transaction, operations, &conversation)?;
                    }
                }
                for mut view in views::views_showing_member(transaction, id)? {
                    if !going(Entity::View, &view.id().to_string()) {
                        view.forget_member(member_id);
                        views::store_view(transaction, operations, &view)?;
                    }
                }
            }
            Entity::Sprint => {
                let sprint_id = codec::parse(id)?;
                let project_id: Option<String> = transaction
                    .query_row(
                        "SELECT project_id FROM sprints WHERE id = :id",
                        named_params! { ":id": id },
                        |row| row.get(0),
                    )
                    .optional()?;
                if let Some(project_id) = project_id {
                    for mut task in tasks::tasks_of_project(transaction, &project_id)? {
                        if task.sprint_id() == Some(sprint_id)
                            && !going(Entity::Task, &task.id().to_string())
                        {
                            task.plan_into(None, now);
                            tasks::store_task(transaction, operations, &task)?;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Remembers the turns that a deletion removes while they run on this
    /// computer, so that their programs are stopped.
    fn note_turns_to_stop(
        &mut self,
        transaction: &Transaction<'_>,
        entity: Entity,
        id: &str,
    ) -> Result<(), DatabaseError> {
        let condition = match entity {
            Entity::Turn => "turn.id = :id",
            Entity::Conversation => "conversation.id = :id",
            Entity::Task => "conversation.task_id = :id",
            Entity::Project => "conversation.project_id = :id",
            _ => return Ok(()),
        };
        let query = format!(
            "SELECT turn.id FROM turns AS turn
             JOIN conversations AS conversation ON conversation.id = turn.conversation_id
             WHERE turn.state = 'running' AND conversation.home_computer = :here
               AND {condition}"
        );
        let turns: Vec<String> = transaction
            .prepare(&query)?
            .query_map(
                named_params! { ":here": self.this_computer, ":id": id },
                |row| row.get(0),
            )?
            .collect::<Result<_, _>>()?;
        self.turns_to_stop.extend(turns);
        Ok(())
    }

    /// Remembers which project and which conversation a change touched, to
    /// report them and to check the rules of the project afterwards.
    fn note_change(
        &mut self,
        transaction: &Transaction<'_>,
        entity: Entity,
        id: &str,
        state: Option<&Map<String, Value>>,
    ) -> Result<(), DatabaseError> {
        let conversation_of_turn = |turn_id: &str| -> Result<Option<String>, DatabaseError> {
            Ok(transaction
                .query_row(
                    "SELECT conversation_id FROM turns WHERE id = :id",
                    named_params! { ":id": turn_id },
                    |row| row.get(0),
                )
                .optional()?)
        };
        let conversation = match entity {
            Entity::Conversation => Some(id.to_owned()),
            Entity::Turn => match state.and_then(|state| text_of(state, "conversation_id")) {
                Some(conversation) => Some(conversation.to_owned()),
                None => conversation_of_turn(id)?,
            },
            Entity::TurnStep => match id.rsplit_once('/') {
                Some((turn_id, _)) => conversation_of_turn(turn_id)?,
                None => None,
            },
            Entity::Decision | Entity::Interview | Entity::Proposal => conversation_of_turn(id)?,
            _ => None,
        };
        let project = match entity {
            Entity::Project => Some(id.to_owned()),
            Entity::Task
            | Entity::Member
            | Entity::Sprint
            | Entity::View
            | Entity::Area
            | Entity::Conversation => state
                .and_then(|state| text_of(state, "project_id"))
                .map(str::to_owned),
            _ => match &conversation {
                Some(conversation) => transaction
                    .query_row(
                        "SELECT project_id FROM conversations WHERE id = :id",
                        named_params! { ":id": conversation },
                        |row| row.get(0),
                    )
                    .optional()?,
                None => None,
            },
        };
        self.projects.extend(project);
        self.conversations.extend(conversation);
        Ok(())
    }

    /// Checks the rules that span records in the projects this take-in
    /// changed, and repairs what broke, with ordinary logged writes of this
    /// computer.
    fn repair(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
    ) -> Result<(), DatabaseError> {
        let projects: Vec<String> = self.projects.iter().cloned().collect();
        for project_id in &projects {
            if projects::find_project(transaction, project_id)?.is_some() {
                self.repair_project(transaction, operations, project_id)?;
            }
        }
        if !projects.is_empty() {
            self.repair_keys(transaction, operations)?;
        }
        Ok(())
    }

    /// Repairs the rules of one project: one active sprint, unique task
    /// numbers, no task that waits for itself or is a part of itself
    /// through a chain, one backlog and one dashboard, and views with names
    /// of their own.
    fn repair_project(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now();

        // Two active sprints: the one started later goes back to planned.
        let mut active: Vec<Sprint> = sprints::sprints_of_project(transaction, project_id)?
            .into_iter()
            .filter(Sprint::is_active)
            .collect();
        active.sort_by_key(|sprint| (sprint.started_at(), sprint.id()));
        for sprint in active.iter_mut().skip(1) {
            sprint.return_to_planned();
            sprints::store_sprint(transaction, operations, sprint)?;
        }

        // Two tasks with one number: the one whose number was set later
        // gets the next free number, but only from the computer that
        // created it, because a number is handed out once.
        let mut tasks = tasks::tasks_of_project(transaction, project_id)?;
        let mut by_number: BTreeMap<u32, Vec<(String, String)>> = BTreeMap::new();
        for task in &tasks {
            let id = task.id().to_string();
            let stamp = self.field_stamp(transaction, Entity::Task, &id, "number")?;
            by_number
                .entry(task.number())
                .or_default()
                .push((stamp, id));
        }
        let mut to_renumber = Vec::new();
        for (_, mut holders) in by_number {
            holders.sort();
            for (_, id) in holders.into_iter().skip(1) {
                if self.created_here(transaction, Entity::Task, &id)? {
                    to_renumber.push(id);
                }
            }
        }
        if !to_renumber.is_empty()
            && let Some(mut project) = projects::find_project(transaction, project_id)?
        {
            let highest = tasks.iter().map(Task::number).max().unwrap_or_default();
            for id in to_renumber {
                let Some(task) = tasks.iter_mut().find(|task| task.id().to_string() == id) else {
                    continue;
                };
                let mut number = project.allocate_task_number();
                while number <= highest {
                    number = project.allocate_task_number();
                }
                let old_number = task.number();
                task.renumber(number, now);
                tasks::store_task(transaction, operations, task)?;
                self.tell(
                    transaction,
                    NoticeKind::TaskRenumbered,
                    Some(project_id),
                    Some(&id),
                    &format!(
                        "{key}-{old_number} is now {key}-{number}: another computer gave the \
                         number {old_number} to another task at the same time.",
                        key = project.key()
                    ),
                )?;
            }
            projects::store_project(transaction, operations, &project)?;
        }

        // Chains of waiting and of parts that close on themselves: the link
        // set last is removed, until none is left.
        for field in ["waits_for", "parent"] {
            loop {
                let tasks = tasks::tasks_of_project(transaction, project_id)?;
                let Some(cycle) = find_cycle(&tasks, field) else {
                    break;
                };
                let mut latest: Option<(String, TaskId, TaskId)> = None;
                for (task, next) in cycle {
                    let stamp =
                        self.field_stamp(transaction, Entity::Task, &task.to_string(), field)?;
                    if latest.as_ref().is_none_or(|(kept, _, _)| stamp > *kept) {
                        latest = Some((stamp, task, next));
                    }
                }
                let Some((_, task_id, next)) = latest else {
                    break;
                };
                let Some(mut task) = tasks.into_iter().find(|task| task.id() == task_id) else {
                    break;
                };
                if field == "waits_for" {
                    task.unblock_from(next, now);
                } else {
                    task.set_parent(None, now).map_err(|reason| {
                        codec::damaged_record("task", &task_id.to_string(), &reason)
                    })?;
                }
                tasks::store_task(transaction, operations, &task)?;
            }
        }

        self.repair_views(transaction, operations, project_id)?;
        self.give_a_board_if_no_view_is_left(transaction, operations, project_id)?;
        self.repair_area_names(transaction, operations, project_id)?;
        self.give_general_if_no_area_is_left(transaction, operations, project_id)?;
        self.put_tasks_into_areas(transaction, operations, project_id)
    }

    /// Repairs the names of the areas of a project after two computers
    /// added or renamed areas at the same time. Of two areas with one name,
    /// whatever the capitals, the older keeps it and the younger gets the
    /// name with the first free number from 2 on. An area is as old as its
    /// first change in the log. Every computer that sees the same areas
    /// repairs them the same way.
    fn repair_area_names(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut aged: Vec<(String, Area)> = Vec::new();
        for area in areas::areas_of_project(transaction, project_id)? {
            let created = first_stamp(transaction, Entity::Area, &area.id().to_string())?;
            aged.push((created, area));
        }
        aged.sort_by(|(one_stamp, one), (other_stamp, other)| {
            one_stamp
                .cmp(other_stamp)
                .then_with(|| one.id().cmp(&other.id()))
        });

        let mut names: Vec<String> = aged
            .iter()
            .map(|(_, area)| area.name().as_str().to_lowercase())
            .collect();
        let mut names_kept: BTreeSet<String> = BTreeSet::new();
        for (_, mut area) in aged {
            if names_kept.insert(area.name().as_str().to_lowercase()) {
                continue;
            }
            let renamed = AreaName::numbered(area.name().as_str(), &|candidate: &str| {
                !names.contains(&candidate.to_lowercase())
            });
            names.push(renamed.as_str().to_lowercase());
            names_kept.insert(renamed.as_str().to_lowercase());
            area.rename(renamed);
            areas::store_area(transaction, operations, &area)?;
        }
        Ok(())
    }

    /// A project left with no areas, because two computers each removed
    /// other areas, gets "General" again, so that its tasks have somewhere
    /// to be. Every computer gives it the same area
    /// ([`Area::general_again`]): the first identifier that was not
    /// deleted.
    fn give_general_if_no_area_is_left(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        if !areas::areas_of_project(transaction, project_id)?.is_empty() {
            return Ok(());
        }
        let project = codec::parse(project_id)?;
        for attempt in 0..=Area::LAST_GENERAL_ATTEMPT {
            let Some(general) = Area::general_again(project, attempt) else {
                break;
            };
            if presence(transaction, Entity::Area, &general.id().to_string())? != Presence::Deleted
            {
                return areas::store_area(transaction, operations, &general);
            }
        }
        Ok(())
    }

    /// Puts every task of a project into an area it has. A task whose area
    /// was removed on another computer goes into the first area of the
    /// project, and a subtask goes into the area of the task at the top of
    /// its chain of parts, which two computers can leave apart when one
    /// moves a task and the other gives it a subtask.
    fn put_tasks_into_areas(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut areas = areas::areas_of_project(transaction, project_id)?;
        areas.sort_by_key(|area| (area.position(), area.id()));
        let Some(first) = areas.first().map(Area::id) else {
            return Ok(());
        };
        let tasks = tasks::tasks_of_project(transaction, project_id)?;
        let kept_area = |task: &Task| {
            if areas.iter().any(|area| area.id() == task.area_id()) {
                task.area_id()
            } else {
                first
            }
        };
        let by_id: BTreeMap<TaskId, &Task> = tasks.iter().map(|task| (task.id(), task)).collect();
        let now = Utc::now();
        for task in &tasks {
            // The chains of parts are free of loops by now; the count only
            // guards against damaged data.
            let mut top = task;
            for _ in 0..tasks.len() {
                match top.parent_id().and_then(|parent| by_id.get(&parent)) {
                    Some(parent) => top = parent,
                    None => break,
                }
            }
            let area_id = kept_area(top);
            if task.area_id() != area_id {
                let mut moved = task.clone();
                moved.put_into_area(area_id, now);
                tasks::store_task(transaction, operations, &moved)?;
            }
        }
        Ok(())
    }

    /// A project left with no views, because two computers each removed
    /// other views, gets a board again, so that the manager has something
    /// to look at. Every computer gives it the same board
    /// ([`View::board_again`]): the first identifier that was not deleted.
    fn give_a_board_if_no_view_is_left(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        if !views::views_of_project(transaction, project_id)?.is_empty() {
            return Ok(());
        }
        let project = codec::parse(project_id)?;
        for attempt in 0..=View::LAST_BOARD_ATTEMPT {
            let Some(board) = View::board_again(project, attempt) else {
                break;
            };
            if presence(transaction, Entity::View, &board.id().to_string())? != Presence::Deleted {
                return views::store_view(transaction, operations, &board);
            }
        }
        Ok(())
    }

    /// Repairs the views of a project after two computers added or renamed
    /// views at the same time. Of two backlogs, or two dashboards, the
    /// younger is removed. Of two views with one name, whatever the
    /// capitals, the older keeps it and the younger gets the name with the
    /// first free number from 2 on, as a new view would. A view is as old as
    /// its first change in the log.
    ///
    /// Every computer that sees the same views repairs them the same way,
    /// so two computers that both repair agree. No notice is kept: no kind
    /// of notice is about views, and the change shows in the row of views.
    fn repair_views(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut aged: Vec<(String, View)> = Vec::new();
        for view in views::views_of_project(transaction, project_id)? {
            let created = first_stamp(transaction, Entity::View, &view.id().to_string())?;
            aged.push((created, view));
        }
        aged.sort_by(|(one_stamp, one), (other_stamp, other)| {
            one_stamp
                .cmp(other_stamp)
                .then_with(|| one.id().cmp(&other.id()))
        });

        let mut kinds_seen = BTreeSet::new();
        let mut kept: Vec<View> = Vec::new();
        for (_, view) in aged {
            if view.kind().is_single() && !kinds_seen.insert(view.kind()) {
                views::delete_view(transaction, operations, &view.id().to_string())?;
            } else {
                kept.push(view);
            }
        }

        let mut names: Vec<String> = kept
            .iter()
            .map(|view| view.name().as_str().to_lowercase())
            .collect();
        let mut names_kept: BTreeSet<String> = BTreeSet::new();
        for mut view in kept {
            if names_kept.insert(view.name().as_str().to_lowercase()) {
                continue;
            }
            let renamed = ViewName::numbered(view.name().as_str(), &|candidate: &str| {
                !names.contains(&candidate.to_lowercase())
            });
            names.push(renamed.as_str().to_lowercase());
            names_kept.insert(renamed.as_str().to_lowercase());
            view.rename(renamed);
            views::store_view(transaction, operations, &view)?;
        }
        Ok(())
    }

    /// Two projects with one key: the one whose key was set later gets the
    /// key with a number, as a new project would, but only from the
    /// computer that created it.
    fn repair_keys(
        &mut self,
        transaction: &Transaction<'_>,
        operations: &mut OperationRecorder<'_>,
    ) -> Result<(), DatabaseError> {
        let mut all = projects::all_projects(transaction)?;
        let mut by_key: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for project in &all {
            let id = project.id().to_string();
            let stamp = self.field_stamp(transaction, Entity::Project, &id, "key")?;
            by_key
                .entry(project.key().to_string())
                .or_default()
                .push((stamp, id));
        }
        for (_, mut holders) in by_key {
            holders.sort();
            for (_, id) in holders.into_iter().skip(1) {
                if !self.created_here(transaction, Entity::Project, &id)? {
                    continue;
                }
                let taken: Vec<String> = all
                    .iter()
                    .map(|project| project.key().to_string())
                    .collect();
                let Some(project) = all
                    .iter_mut()
                    .find(|project| project.id().to_string() == id)
                else {
                    continue;
                };
                let old_key = project.key().clone();
                let new_key = old_key
                    .first_free(|candidate| taken.iter().any(|key| key == candidate.as_str()));
                project.rekey(new_key.clone());
                projects::store_project(transaction, operations, project)?;
                self.tell(
                    transaction,
                    NoticeKind::ProjectRekeyed,
                    Some(&id),
                    None,
                    &format!(
                        "The project \"{}\" now has the key {new_key}: another computer gave \
                         {old_key} to another project at the same time.",
                        project.name()
                    ),
                )?;
            }
        }
        Ok(())
    }

    /// The stamp of a field of a thing, or nothing if it has none.
    fn field_stamp(
        &self,
        transaction: &Transaction<'_>,
        entity: Entity,
        id: &str,
        field: &str,
    ) -> Result<String, DatabaseError> {
        Ok(MergedState::read(transaction, entity, id)?
            .and_then(|merged| merged.stamps.get(field).cloned())
            .unwrap_or_default())
    }

    /// Whether this computer created a thing: the first operation of the
    /// thing is its own.
    fn created_here(
        &self,
        transaction: &Transaction<'_>,
        entity: Entity,
        id: &str,
    ) -> Result<bool, DatabaseError> {
        let creator: Option<String> = transaction
            .query_row(
                "SELECT device_id FROM operations
                 WHERE entity = :entity AND entity_id = :entity_id
                 ORDER BY stamp LIMIT 1",
                named_params! { ":entity": entity.as_str(), ":entity_id": id },
                |row| row.get(0),
            )
            .optional()?;
        Ok(creator.as_deref() == Some(self.this_computer.as_str()))
    }
}

/// The stamp of the first change of a thing in the log, or nothing if the
/// log has none.
fn first_stamp(
    transaction: &Transaction<'_>,
    entity: Entity,
    id: &str,
) -> Result<String, DatabaseError> {
    let first: Option<String> = transaction.query_row(
        "SELECT min(stamp) FROM operations WHERE entity = :entity AND entity_id = :entity_id",
        named_params! { ":entity": entity.as_str(), ":entity_id": id },
        |row| row.get(0),
    )?;
    Ok(first.unwrap_or_default())
}

/// The status a task goes to whose status was removed: the first status of
/// the same category, else the first status.
fn target_status(project: &Project, category: Option<StatusCategory>) -> Status {
    category
        .and_then(|category| project.workflow().first_in(category))
        .unwrap_or_else(|| project.workflow().initial_status())
        .clone()
}

/// The category of a status of a project, from its row if it is still
/// there, or else from the latest state of the project in the log that has
/// it.
fn category_of_status(
    transaction: &Transaction<'_>,
    project_id: &str,
    status_id: &str,
) -> Result<Option<StatusCategory>, DatabaseError> {
    let from_row: Option<String> = transaction
        .query_row(
            "SELECT category FROM statuses WHERE id = :id",
            named_params! { ":id": status_id },
            |row| row.get(0),
        )
        .optional()?;
    if let Some(category) = from_row {
        return codec::parse(&category).map(Some);
    }
    let mut statement = transaction.prepare(
        "SELECT payload FROM operations
         WHERE entity = 'project' AND entity_id = :id AND payload IS NOT NULL
         ORDER BY stamp DESC",
    )?;
    let states: Vec<String> = statement
        .query_map(named_params! { ":id": project_id }, |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for state in states {
        let Ok(state) = object_from_text(&state, "a state of a project") else {
            continue;
        };
        let Some(Value::Array(statuses)) = state.get("statuses") else {
            continue;
        };
        for status in statuses {
            if status.get("id").and_then(Value::as_str) == Some(status_id)
                && let Some(category) = status.get("category").and_then(Value::as_str)
            {
                return codec::parse(category).map(Some);
            }
        }
    }
    Ok(None)
}

/// Finds a chain of tasks that closes on itself, by what they wait for
/// (`waits_for`) or by their parents (`parent`), as the links of the chain:
/// each task with the task it points to.
fn find_cycle(tasks: &[Task], field: &str) -> Option<Vec<(TaskId, TaskId)>> {
    let links: BTreeMap<TaskId, Vec<TaskId>> = tasks
        .iter()
        .map(|task| {
            let targets = if field == "parent" {
                task.parent_id().into_iter().collect()
            } else {
                task.blocked_by().iter().copied().collect()
            };
            (task.id(), targets)
        })
        .collect();
    // A depth-first search that remembers the path it is on.
    let mut finished: BTreeSet<TaskId> = BTreeSet::new();
    for start in links.keys() {
        let mut path: Vec<(TaskId, usize)> = vec![(*start, 0)];
        let mut on_path: BTreeSet<TaskId> = BTreeSet::from([*start]);
        while let Some((task, next_index)) = path.last().copied() {
            let targets = links.get(&task).map(Vec::as_slice).unwrap_or_default();
            let Some(target) = targets.get(next_index).copied() else {
                finished.insert(task);
                on_path.remove(&task);
                path.pop();
                continue;
            };
            if let Some(last) = path.last_mut() {
                last.1 += 1;
            }
            if on_path.contains(&target) {
                let from = path.iter().position(|(on, _)| *on == target).unwrap_or(0);
                let chain: Vec<TaskId> = path[from..].iter().map(|(on, _)| *on).collect();
                let mut cycle: Vec<(TaskId, TaskId)> =
                    chain.windows(2).map(|pair| (pair[0], pair[1])).collect();
                if let Some(last) = chain.last() {
                    cycle.push((*last, target));
                }
                return Some(cycle);
            }
            if !finished.contains(&target) && links.contains_key(&target) {
                path.push((target, 0));
                on_path.insert(target);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rusqlite::Connection;
    use teamwright_domain::{Area, ProjectId};

    use crate::database::Database;
    use crate::migrations::testing::migrate_to;

    /// A project both computers had before there were areas.
    const PROJECT: &str = "0196f3a2-8000-7abc-9def-0123456789ab";
    /// The computer that made the project, and gave it to both.
    const ORIGIN: &str = "0196f3a2-7c1e-7d53-9b6f-000000000001";

    /// Writes a database of version 13 that knows the project, its two
    /// statuses and a task, and that took the project in from the computer
    /// that made it.
    fn kept_before_areas(path: &Path, computer: &str) {
        let mut connection = Connection::open(path).unwrap();
        migrate_to(&mut connection, 13);
        let stamp = format!("1762851600000-00000-{ORIGIN}");
        connection
            .execute_batch(&format!(
                "INSERT INTO meta (key, value) VALUES ('device_id', '{computer}');
                 INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('{PROJECT}', 'AQN', 'Air quality node', '', 2,
                         '2026-11-11T09:00:00.000Z');
                 INSERT INTO statuses (id, project_id, position, name, category, color)
                 VALUES ('0196f3a2-8000-7abc-9def-0000000000a1', '{PROJECT}', 0, 'To do',
                         'todo', 'gray'),
                        ('0196f3a2-8000-7abc-9def-0000000000a2', '{PROJECT}', 1, 'Done',
                         'done', 'green');
                 INSERT INTO tasks (
                     id, project_id, number, title, description, status_id, priority,
                     assignee_kind, assignee_member_id, parent_id, estimate_minutes,
                     start_on, due_on, rank, created_at, updated_at, completed_at
                 )
                 VALUES (
                     '0196f3a2-8000-7abc-9def-0000000000b1', '{PROJECT}', 1,
                     'Choose the sensors', '', '0196f3a2-8000-7abc-9def-0000000000a1',
                     'normal', NULL, NULL, NULL, NULL, NULL, NULL, 0.0,
                     '2026-11-11T09:00:00.000Z', '2026-11-11T09:00:00.000Z', NULL
                 );
                 INSERT INTO operations (
                     device_id, device_sequence, stamp, entity, entity_id, kind, payload
                 )
                 VALUES ('{ORIGIN}', 1, '{stamp}', 'project', '{PROJECT}', 'upsert', '{{}}');
                 INSERT INTO merged_states (entity, entity_id, state, base_stamp)
                 VALUES ('project', '{PROJECT}', '{{}}', '{stamp}');"
            ))
            .unwrap();
    }

    #[test]
    fn two_computers_that_kept_a_project_before_areas_give_it_one_area_general() {
        let folder = tempfile::tempdir().unwrap();
        let desk_file = folder.path().join("desk.db");
        let laptop_file = folder.path().join("laptop.db");
        kept_before_areas(&desk_file, "0196f3a2-7c1e-7d53-9b6f-00000000000d");
        kept_before_areas(&laptop_file, "0196f3a2-7c1e-7d53-9b6f-00000000000e");
        let desk = Database::open(&desk_file).unwrap();
        let laptop = Database::open(&laptop_file).unwrap();

        for _ in 0..2 {
            desk.take_in(&laptop.operations_to_send(0).unwrap())
                .unwrap();
            laptop
                .take_in(&desk.operations_to_send(0).unwrap())
                .unwrap();
        }

        let project: ProjectId = PROJECT.parse().unwrap();
        for computer in [&desk, &laptop] {
            let areas = computer
                .read(|connection| crate::areas::areas_of_project(connection, PROJECT))
                .unwrap();
            assert_eq!(areas, [Area::starting(project)]);
            let tasks = computer
                .read(|connection| crate::tasks::tasks_of_project(connection, PROJECT))
                .unwrap();
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0].area_id(), areas[0].id());
        }
        let notices = desk.notices().unwrap().len() + laptop.notices().unwrap().len();
        assert_eq!(notices, 0);
    }
}
