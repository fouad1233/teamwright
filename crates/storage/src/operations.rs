//! The operation log: every change, who made it, and in what order.
//!
//! Each write to the database also appends an operation here, in the same
//! transaction, so the log can never disagree with the state. An operation
//! carries the identity of the computer that made it and a stamp from a
//! hybrid logical clock.
//!
//! A hybrid logical clock combines the time of day with a counter. Its
//! stamps follow the real clock when the real clock moves forward, and keep
//! increasing anyway when it stands still or jumps back. Changes are
//! therefore always ordered the way they happened on one computer, and
//! sensibly ordered between computers whose clocks disagree a little.

use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, Transaction, named_params};
use serde::Serialize;
use serde_json::{Map, Value};
use teamwright_domain::ComputerId;

use crate::error::DatabaseError;
use crate::merge::{MergedState, Stamps, all_fields, same_value, stamps_to_text};

/// One entry of the operation log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// The position of the operation in this computer's log.
    pub sequence: i64,
    /// The computer that made the change.
    pub device_id: ComputerId,
    /// The hybrid logical clock stamp. Stamps sort as text in the order the
    /// changes happened.
    pub stamp: String,
    /// What changed: `project` or `task`.
    pub entity: String,
    /// The identifier of the thing that changed.
    pub entity_id: String,
    /// `upsert` or `delete`.
    pub kind: String,
    /// For an upsert, the complete new state as JSON.
    pub payload: Option<String>,
}

/// The kinds of things whose changes are logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entity {
    /// A project together with its workflow.
    Project,
    /// A task together with its dependencies.
    Task,
    /// A member of a team.
    Member,
    /// A conversation with a specialist, without its turns.
    Conversation,
    /// A turn of a conversation, without its steps.
    Turn,
    /// One step of a turn. Its identifier is the turn's and its position.
    TurnStep,
    /// The question a turn asked the manager, with its options and its
    /// answer. Its identifier is the turn's.
    Decision,
    /// The interview the Advisor asked in a turn, with its questions, their
    /// options and the answers. Its identifier is the turn's.
    Interview,
    /// The proposal the Advisor made in a turn, with its members, its
    /// tasks and what they wait for. Its identifier is the turn's.
    Proposal,
    /// The folders chosen for a project, with all its reference folders.
    /// Its identifier is the project's. These are facts about this
    /// computer, which must not be applied on another one.
    ProjectPlaces,
    /// A sprint with its summary.
    Sprint,
    /// A view of a project with its filter.
    View,
}

impl Entity {
    /// Every kind of thing whose changes are logged.
    pub(crate) const ALL: [Self; 12] = [
        Self::Project,
        Self::Task,
        Self::Member,
        Self::Conversation,
        Self::Turn,
        Self::TurnStep,
        Self::Decision,
        Self::Interview,
        Self::Proposal,
        Self::ProjectPlaces,
        Self::Sprint,
        Self::View,
    ];

    /// The entity of a name in the log, if this version knows it.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|entity| entity.as_str() == name)
    }

    /// The name of the entity in the log.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Task => "task",
            Self::Member => "member",
            Self::Conversation => "conversation",
            Self::Turn => "turn",
            Self::TurnStep => "turn_step",
            Self::Decision => "decision",
            Self::Interview => "interview",
            Self::Proposal => "proposal",
            Self::ProjectPlaces => "project_places",
            Self::Sprint => "sprint",
            Self::View => "view",
        }
    }
}

/// A point on the hybrid logical clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) struct Stamp {
    /// Milliseconds since 1 January 1970, UTC.
    millis: i64,
    /// Distinguishes stamps issued within the same millisecond.
    counter: u32,
}

impl Stamp {
    /// The number of digits the milliseconds are padded to. Thirteen digits
    /// last until the year 2286.
    const MILLIS_DIGITS: usize = 13;
    /// The number of digits the counter is padded to.
    const COUNTER_DIGITS: usize = 5;
    /// The largest counter that fits in its digits.
    const MAX_COUNTER: u32 = 99_999;

    /// Writes the stamp as text that sorts in clock order: zero-padded
    /// milliseconds, zero-padded counter, and the device as a tie-breaker.
    fn to_text(self, device_id: ComputerId) -> String {
        format!(
            "{millis:0millis_digits$}-{counter:0counter_digits$}-{device_id}",
            millis = self.millis,
            counter = self.counter,
            millis_digits = Self::MILLIS_DIGITS,
            counter_digits = Self::COUNTER_DIGITS,
        )
    }

    /// Reads the clock part of a stamp written by [`Stamp::to_text`].
    pub(crate) fn from_text(text: &str) -> Option<Self> {
        let mut parts = text.splitn(3, '-');
        let millis = parts.next()?.parse().ok()?;
        let counter = parts.next()?.parse().ok()?;
        Some(Self { millis, counter })
    }

    /// Milliseconds since 1 January 1970, UTC.
    pub(crate) const fn millis(self) -> i64 {
        self.millis
    }
}

/// Issues stamps that always increase.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct HybridLogicalClock {
    last: Stamp,
}

impl HybridLogicalClock {
    /// Creates a clock that continues after the latest stamp already in the
    /// log, so stamps keep increasing across restarts.
    pub(crate) const fn continuing_after(last: Stamp) -> Self {
        Self { last }
    }

    /// Moves the clock on to at least a stamp seen elsewhere, so that the
    /// next stamp it issues is later than it. The clock never moves back.
    pub(crate) fn witness(&mut self, seen: Stamp) {
        self.last = self.last.max(seen);
    }

    /// Issues the next stamp, given the time of day in milliseconds.
    pub(crate) fn tick(&mut self, wall_clock_millis: i64) -> Stamp {
        self.last = if wall_clock_millis > self.last.millis {
            Stamp {
                millis: wall_clock_millis,
                counter: 0,
            }
        } else if self.last.counter < Stamp::MAX_COUNTER {
            Stamp {
                millis: self.last.millis,
                counter: self.last.counter + 1,
            }
        } else {
            // The counter is full: borrow the next millisecond.
            Stamp {
                millis: self.last.millis + 1,
                counter: 0,
            }
        };
        self.last
    }
}

/// Reads every state something has been stored in, the oldest first, each as
/// the JSON the log keeps. Deletions are passed over: they have no state.
///
/// The states are ordered by their stamps, which is the order the changes
/// happened in, also for changes that came from another computer.
pub(crate) fn states_of(
    connection: &Connection,
    entity: Entity,
    entity_id: &str,
) -> Result<Vec<String>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT payload FROM operations
         WHERE entity = :entity AND entity_id = :entity_id AND payload IS NOT NULL
           AND waiting IS NULL
         ORDER BY stamp, sequence",
    )?;
    let states = statement
        .query_map(
            named_params! { ":entity": entity.as_str(), ":entity_id": entity_id },
            |row| row.get(0),
        )?
        .collect::<Result<_, _>>()?;
    Ok(states)
}

/// The time of day in milliseconds since 1 January 1970, UTC.
pub(crate) fn wall_clock_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        // A clock set before 1970 gives no usable time. The logical clock
        // then carries on from its last stamp by counting.
        .unwrap_or(0)
}

/// Appends operations to the log as part of a write transaction.
pub(crate) struct OperationRecorder<'transaction> {
    transaction: &'transaction Transaction<'transaction>,
    device_id: ComputerId,
    clock: &'transaction mut HybridLogicalClock,
}

impl<'transaction> OperationRecorder<'transaction> {
    /// Creates a recorder for one transaction.
    pub(crate) const fn new(
        transaction: &'transaction Transaction<'transaction>,
        device_id: ComputerId,
        clock: &'transaction mut HybridLogicalClock,
    ) -> Self {
        Self {
            transaction,
            device_id,
            clock,
        }
    }

    /// The identity of the computer the recorder logs for.
    pub(crate) const fn device_id(&self) -> ComputerId {
        self.device_id
    }

    /// Moves the clock on to at least a stamp seen elsewhere, so that the
    /// next change made here is stamped later than everything seen.
    pub(crate) fn witness(&mut self, stamp: &str) {
        if let Some(seen) = Stamp::from_text(stamp) {
            self.clock.witness(seen);
        }
    }

    /// Logs that something was created or changed, with its complete new
    /// state.
    ///
    /// The fields whose value changed get the stamp of this operation; the
    /// others keep the stamp of the write that set them. The merged state
    /// of the thing is kept up to date, and the operation carries the
    /// stamps of its fields that are older than itself, so that another
    /// computer can merge it field by field.
    ///
    /// Something that was deleted stays deleted: writing it again is
    /// refused, except for a step of a turn, whose identifier is its
    /// position and can be used again.
    pub(crate) fn record_upsert(
        &mut self,
        entity: Entity,
        entity_id: &str,
        state: &impl Serialize,
    ) -> Result<(), DatabaseError> {
        let cannot_write = |reason: &dyn std::fmt::Display| {
            DatabaseError::Damaged(format!(
                "the {} {entity_id} cannot be written to the log: {reason}",
                entity.as_str()
            ))
        };
        let new_state = match serde_json::to_value(state) {
            Ok(Value::Object(object)) => object,
            Ok(_) => return Err(cannot_write(&"it is not a JSON object")),
            Err(reason) => return Err(cannot_write(&reason)),
        };
        let stamp = self.next_stamp();

        let merged = match MergedState::read(self.transaction, entity, entity_id)? {
            Some(old) if old.deleted.is_some() && entity != Entity::TurnStep => {
                return Err(DatabaseError::Deleted {
                    entity: entity.as_str(),
                    id: entity_id.to_owned(),
                });
            }
            Some(old) if old.deleted.is_none() && old.state.is_some() => {
                merged_with_local_change(entity, old, new_state, &stamp)
            }
            // New, or a step written again after it was removed.
            _ => MergedState::created(entity, new_state, &stamp),
        };
        merged.write(self.transaction, entity, entity_id)?;

        let older: Stamps = merged
            .stamps
            .iter()
            .filter(|(_, field_stamp)| **field_stamp != stamp)
            .map(|(name, field_stamp)| (name.clone(), field_stamp.clone()))
            .collect();
        let payload = merged.state.map(|state| Value::Object(state).to_string());
        self.append(&Appended {
            entity,
            entity_id,
            kind: "upsert",
            stamp: &stamp,
            payload: payload.as_deref(),
            field_stamps: stamps_to_text(&older).as_deref(),
            seen: None,
        })
    }

    /// Logs that something was removed.
    ///
    /// The deletion records, for each computer, the latest change of the
    /// thing it knows of. A change another computer made that is not among
    /// them is one this deletion throws away.
    pub(crate) fn record_delete(
        &mut self,
        entity: Entity,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        let stamp = self.next_stamp();
        let seen = latest_changes_seen(self.transaction, entity, entity_id)?;
        let mut merged =
            MergedState::read(self.transaction, entity, entity_id)?.unwrap_or(MergedState {
                state: None,
                stamps: Stamps::new(),
                deleted: None,
                deleted_seen: Stamps::new(),
            });
        merged.deleted = Some(stamp.clone());
        merged.deleted_seen.clone_from(&seen);
        merged.write(self.transaction, entity, entity_id)?;
        self.append(&Appended {
            entity,
            entity_id,
            kind: "delete",
            stamp: &stamp,
            payload: None,
            field_stamps: None,
            seen: stamps_to_text(&seen).as_deref(),
        })
    }

    /// Issues the stamp of the next operation of this computer.
    fn next_stamp(&mut self) -> String {
        self.clock.tick(wall_clock_millis()).to_text(self.device_id)
    }

    /// Writes one row of the log, numbered after the last operation of this
    /// computer.
    fn append(&self, operation: &Appended<'_>) -> Result<(), DatabaseError> {
        self.transaction.execute(
            "INSERT INTO operations (
                 device_id, device_sequence, stamp, entity, entity_id, kind, payload,
                 field_stamps, seen
             )
             VALUES (
                 :device_id,
                 (SELECT coalesce(max(device_sequence), 0) + 1 FROM operations
                  WHERE device_id = :device_id),
                 :stamp, :entity, :entity_id, :kind, :payload, :field_stamps, :seen
             )",
            named_params! {
                ":device_id": self.device_id.to_string(),
                ":stamp": operation.stamp,
                ":entity": operation.entity.as_str(),
                ":entity_id": operation.entity_id,
                ":kind": operation.kind,
                ":payload": operation.payload,
                ":field_stamps": operation.field_stamps,
                ":seen": operation.seen,
            },
        )?;
        Ok(())
    }
}

/// One operation of this computer, about to be written to the log.
struct Appended<'text> {
    entity: Entity,
    entity_id: &'text str,
    kind: &'text str,
    stamp: &'text str,
    payload: Option<&'text str>,
    field_stamps: Option<&'text str>,
    seen: Option<&'text str>,
}

/// The merged state of a thing after this computer changed it: the fields
/// whose value changed get the new stamp, the others keep theirs, and keys
/// this version does not write are kept as they were.
fn merged_with_local_change(
    entity: Entity,
    old: MergedState,
    new_state: Map<String, Value>,
    stamp: &str,
) -> MergedState {
    let old_state = old.state.unwrap_or_default();
    let mut state = old_state.clone();
    for (key, value) in &new_state {
        state.insert(key.clone(), value.clone());
    }
    let stamps = all_fields(entity, &state)
        .into_iter()
        .map(|(name, keys, _)| {
            let written = keys.iter().any(|key| new_state.contains_key(key));
            let kept = old.stamps.get(&name).cloned();
            let field_stamp = match kept {
                Some(kept) if !written || same_value(&old_state, &state, &keys) => kept,
                _ => stamp.to_owned(),
            };
            (name, field_stamp)
        })
        .collect();
    MergedState {
        state: Some(state),
        stamps,
        deleted: None,
        deleted_seen: Stamps::new(),
    }
}

/// For each computer, the stamp of the latest change of a thing that this
/// computer knows of.
pub(crate) fn latest_changes_seen(
    connection: &Connection,
    entity: Entity,
    entity_id: &str,
) -> Result<Stamps, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT device_id, max(stamp) FROM operations
         WHERE entity = :entity AND entity_id = :entity_id AND kind = 'upsert'
         GROUP BY device_id",
    )?;
    let seen = statement
        .query_map(
            named_params! { ":entity": entity.as_str(), ":entity_id": entity_id },
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<Result<_, _>>()?;
    Ok(seen)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;
    use teamwright_application::{ProjectRepository, TaskRepository};
    use teamwright_domain::{
        Priority, Project, ProjectKey, ProjectName, Rank, Schedule, Task, TaskDraft, TaskTitle,
        Timestamp, Workflow,
    };

    use super::*;
    use crate::database::Database;
    use crate::projects::SqliteProjectRepository;
    use crate::tasks::SqliteTaskRepository;

    fn moment(minutes: i64) -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
            + chrono::TimeDelta::minutes(minutes)
    }

    /// A database with a project and one task in it.
    fn database_with_a_task() -> (Arc<Database>, Task) {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let name = ProjectName::parse("Smart sensor node").unwrap();
        let mut project = Project::create(
            name.clone(),
            ProjectKey::suggest(&name),
            String::new(),
            Workflow::standard(),
            moment(0),
        );
        let number = project.allocate_task_number();
        SqliteProjectRepository::new(database.clone())
            .save(&project)
            .unwrap();
        let task = Task::create(
            project.id(),
            number,
            project.workflow().initial_status(),
            Rank::between(None, None),
            TaskDraft {
                title: TaskTitle::parse("Design the board").unwrap(),
                description: "Two layers.".to_owned(),
                priority: Priority::High,
                assignee: None,
                parent_id: None,
                estimate_minutes: None,
                schedule: Schedule::unscheduled(),
            },
            moment(1),
        );
        SqliteTaskRepository::new(database.clone())
            .save(&task)
            .unwrap();
        (database, task)
    }

    fn merged(database: &Database, entity: Entity, id: &str) -> MergedState {
        database
            .read(|connection| MergedState::read(connection, entity, id))
            .unwrap()
            .unwrap()
    }

    fn latest_operation(database: &Database) -> (String, Option<String>, Option<String>) {
        database
            .read(|connection| {
                Ok(connection.query_row(
                    "SELECT stamp, field_stamps, seen FROM operations
                     ORDER BY sequence DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?)
            })
            .unwrap()
    }

    #[test]
    fn a_changed_field_gets_the_new_stamp_and_the_others_keep_theirs() {
        let (database, mut task) = database_with_a_task();
        let id = task.id().to_string();
        let before = merged(&database, Entity::Task, &id);

        task.rename(TaskTitle::parse("Route the board").unwrap(), moment(2));
        SqliteTaskRepository::new(database.clone())
            .save(&task)
            .unwrap();

        let after = merged(&database, Entity::Task, &id);
        let (stamp, field_stamps, _) = latest_operation(&database);
        assert_eq!(after.stamps["title"], stamp);
        assert_eq!(after.stamps["updated_at"], stamp);
        for unchanged in ["description", "status", "priority", "rank", "number"] {
            assert_eq!(
                after.stamps[unchanged], before.stamps[unchanged],
                "{unchanged}"
            );
        }
        let older = crate::merge::stamps_from_text(&field_stamps.unwrap()).unwrap();
        assert_eq!(older["description"], before.stamps["description"]);
        assert!(!older.contains_key("title"));
    }

    #[test]
    fn writing_the_same_state_again_changes_no_stamp() {
        let (database, task) = database_with_a_task();
        let id = task.id().to_string();
        let before = merged(&database, Entity::Task, &id);

        SqliteTaskRepository::new(database.clone())
            .save(&task)
            .unwrap();

        assert_eq!(merged(&database, Entity::Task, &id), before);
    }

    #[test]
    fn a_deletion_records_the_latest_change_it_had_seen() {
        let (database, task) = database_with_a_task();
        let id = task.id().to_string();
        let last_change = merged(&database, Entity::Task, &id)
            .latest_stamp()
            .unwrap()
            .to_owned();

        SqliteTaskRepository::new(database.clone())
            .delete(&[task.id()])
            .unwrap();

        let deleted = merged(&database, Entity::Task, &id);
        let (stamp, _, seen) = latest_operation(&database);
        assert_eq!(deleted.deleted.as_deref(), Some(stamp.as_str()));
        let this_computer = database.device_id().to_string();
        assert_eq!(deleted.deleted_seen[&this_computer], last_change);
        assert_eq!(
            crate::merge::stamps_from_text(&seen.unwrap()).unwrap()[&this_computer],
            last_change
        );
    }

    #[test]
    fn a_deleted_thing_is_not_written_again() {
        let (database, task) = database_with_a_task();
        let tasks = SqliteTaskRepository::new(database.clone());
        tasks.delete(&[task.id()]).unwrap();
        let log = database.operations().unwrap().len();

        let error = tasks.save(&task).unwrap_err();

        assert!(error.to_string().contains("was deleted"));
        assert_eq!(database.operations().unwrap().len(), log);
        assert!(tasks.find(task.id()).unwrap().is_none());
    }

    #[test]
    fn every_operation_of_this_computer_is_numbered_on_it() {
        let (database, mut task) = database_with_a_task();
        task.rename(TaskTitle::parse("Route the board").unwrap(), moment(2));
        SqliteTaskRepository::new(database.clone())
            .save(&task)
            .unwrap();

        let numbers: Vec<(i64, i64)> = database
            .read(|connection| {
                Ok(connection
                    .prepare("SELECT sequence, device_sequence FROM operations ORDER BY sequence")?
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<Result<_, _>>()?)
            })
            .unwrap();
        assert_eq!(numbers, [(1, 1), (2, 2), (3, 3)]);
    }

    #[test]
    fn the_clock_follows_the_time_of_day_when_it_moves_forward() {
        let mut clock = HybridLogicalClock::default();

        let first = clock.tick(1_000);
        let second = clock.tick(2_000);

        assert_eq!(
            first,
            Stamp {
                millis: 1_000,
                counter: 0
            }
        );
        assert_eq!(
            second,
            Stamp {
                millis: 2_000,
                counter: 0
            }
        );
    }

    #[test]
    fn stamps_within_one_millisecond_are_told_apart_by_the_counter() {
        let mut clock = HybridLogicalClock::default();

        let stamps = [clock.tick(1_000), clock.tick(1_000), clock.tick(1_000)];

        assert_eq!(stamps.map(|stamp| stamp.counter), [0, 1, 2]);
        assert!(stamps[0] < stamps[1] && stamps[1] < stamps[2]);
    }

    #[test]
    fn stamps_keep_increasing_when_the_time_of_day_jumps_back() {
        let mut clock = HybridLogicalClock::default();
        let before_the_jump = clock.tick(5_000);

        let after_the_jump = clock.tick(1_000);

        assert!(after_the_jump > before_the_jump);
        assert_eq!(after_the_jump.millis, 5_000);
    }

    #[test]
    fn a_full_counter_borrows_the_next_millisecond() {
        let mut clock = HybridLogicalClock::continuing_after(Stamp {
            millis: 1_000,
            counter: Stamp::MAX_COUNTER,
        });

        let next = clock.tick(1_000);

        assert_eq!(
            next,
            Stamp {
                millis: 1_001,
                counter: 0
            }
        );
    }

    #[test]
    fn a_clock_continues_after_the_latest_stamp_it_is_given() {
        let mut clock = HybridLogicalClock::continuing_after(Stamp {
            millis: 9_000,
            counter: 7,
        });

        let next = clock.tick(100);

        assert_eq!(
            next,
            Stamp {
                millis: 9_000,
                counter: 8
            }
        );
    }

    #[test]
    fn stamps_as_text_sort_in_clock_order() {
        let device_id = ComputerId::generate();
        let earlier = Stamp {
            millis: 999,
            counter: 12,
        }
        .to_text(device_id);
        let later = Stamp {
            millis: 1_000,
            counter: 0,
        }
        .to_text(device_id);
        let later_still = Stamp {
            millis: 1_000,
            counter: 1,
        }
        .to_text(device_id);

        assert!(earlier < later && later < later_still);
    }

    #[test]
    fn the_clock_part_of_a_stamp_can_be_read_back_from_its_text() {
        let stamp = Stamp {
            millis: 1_762_851_600_000,
            counter: 42,
        };

        let text = stamp.to_text(ComputerId::generate());

        assert_eq!(Stamp::from_text(&text), Some(stamp));
        assert!(text.starts_with("1762851600000-00042-"));
    }

    #[test]
    fn witnessing_a_later_stamp_makes_the_next_tick_later_still() {
        let mut clock = HybridLogicalClock::continuing_after(Stamp {
            millis: 1_000,
            counter: 0,
        });
        let from_the_future = Stamp {
            millis: 90_000,
            counter: 3,
        };

        clock.witness(from_the_future);
        let next = clock.tick(1_500);

        assert!(next > from_the_future);
    }

    #[test]
    fn witnessing_an_earlier_stamp_never_moves_the_clock_back() {
        let mut clock = HybridLogicalClock::continuing_after(Stamp {
            millis: 9_000,
            counter: 7,
        });

        clock.witness(Stamp {
            millis: 1_000,
            counter: 0,
        });

        assert_eq!(
            clock.tick(100),
            Stamp {
                millis: 9_000,
                counter: 8
            }
        );
    }

    #[test]
    fn text_that_is_not_a_stamp_is_not_read() {
        assert_eq!(Stamp::from_text("not a stamp"), None);
    }
}
