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

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, Transaction, named_params};
use serde::Serialize;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Identifies a computer that runs Teamwright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(Uuid);

impl DeviceId {
    /// Creates the identity of a computer that has none yet.
    pub(crate) fn generate() -> Self {
        Self(Uuid::now_v7())
    }

    /// Reads an identity from its text form.
    pub(crate) fn from_text(text: &str) -> Result<Self, DatabaseError> {
        Uuid::parse_str(text)
            .map(Self)
            .map_err(|reason| DatabaseError::damaged("the device identifier", text, reason))
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One entry of the operation log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    /// The position of the operation in this computer's log.
    pub sequence: i64,
    /// The computer that made the change.
    pub device_id: DeviceId,
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
}

impl Entity {
    /// The name of the entity in the log.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Task => "task",
            Self::Member => "member",
            Self::Conversation => "conversation",
            Self::Turn => "turn",
            Self::TurnStep => "turn_step",
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
    fn to_text(self, device_id: DeviceId) -> String {
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
fn wall_clock_millis() -> i64 {
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
    device_id: DeviceId,
    clock: &'transaction mut HybridLogicalClock,
}

impl<'transaction> OperationRecorder<'transaction> {
    /// Creates a recorder for one transaction.
    pub(crate) const fn new(
        transaction: &'transaction Transaction<'transaction>,
        device_id: DeviceId,
        clock: &'transaction mut HybridLogicalClock,
    ) -> Self {
        Self {
            transaction,
            device_id,
            clock,
        }
    }

    /// Logs that something was created or changed, with its complete new
    /// state.
    pub(crate) fn record_upsert(
        &mut self,
        entity: Entity,
        entity_id: &str,
        state: &impl Serialize,
    ) -> Result<(), DatabaseError> {
        let payload = serde_json::to_string(state).map_err(|reason| {
            DatabaseError::Damaged(format!(
                "the {} {entity_id} cannot be written to the log: {reason}",
                entity.as_str()
            ))
        })?;
        self.append(entity, entity_id, "upsert", Some(&payload))
    }

    /// Logs that something was removed.
    pub(crate) fn record_delete(
        &mut self,
        entity: Entity,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        self.append(entity, entity_id, "delete", None)
    }

    /// Writes one row of the log.
    fn append(
        &mut self,
        entity: Entity,
        entity_id: &str,
        kind: &str,
        payload: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let stamp = self.clock.tick(wall_clock_millis()).to_text(self.device_id);
        self.transaction.execute(
            "INSERT INTO operations (device_id, stamp, entity, entity_id, kind, payload)
             VALUES (:device_id, :stamp, :entity, :entity_id, :kind, :payload)",
            named_params! {
                ":device_id": self.device_id.to_string(),
                ":stamp": stamp,
                ":entity": entity.as_str(),
                ":entity_id": entity_id,
                ":kind": kind,
                ":payload": payload,
            },
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let device_id = DeviceId::generate();
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

        let text = stamp.to_text(DeviceId::generate());

        assert_eq!(Stamp::from_text(&text), Some(stamp));
        assert!(text.starts_with("1762851600000-00042-"));
    }

    #[test]
    fn text_that_is_not_a_stamp_is_not_read() {
        assert_eq!(Stamp::from_text("not a stamp"), None);
    }
}
