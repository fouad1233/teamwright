//! What is known about the limits of the user's Claude plan on this
//! computer, and whether work may be started
//! ([decision 0040](../../../docs/decisions/0040-the-limit-of-the-plan.md)).
//!
//! The specialists run on the user's own Claude Code, so they share the
//! limits of one plan: a window of five hours and one of a week. When a
//! limit is reached, every turn that runs ends the same way, and every turn
//! that starts before the limit resets ends at once. That is no failure of
//! the specialists. This module keeps what the program said about the plan,
//! says whether the limit is reached now and when it resets, and answers
//! whether new work may start.
//!
//! What is kept is a fact about the Claude account of this computer, not
//! about a project: it is kept on this computer and never synced. Nothing
//! is guessed. How much of a window is used is shown only as the program
//! reported it, and a reset that the program did not give is unknown.

use std::sync::Arc;

use chrono::{Datelike, Days, Months, NaiveDate, NaiveDateTime, TimeZone, Utc};
use teamwright_domain::Timestamp;

use crate::agent_runtime::{
    PlanLimitHit, PlanReset, PlanUsage, PlanWindow, PlanWindowKind, SaidReset,
};
use crate::error::ApplicationError;
use crate::notifications::{Notification, NotificationKind};
use crate::ports::{Clock, StorageError};

/// The status the program gives a plan that allows work.
const ALLOWED: &[&str] = &["allowed", "allowed_warning"];

/// What this computer knows about the limits of its Claude plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlanLimitState {
    /// The limit that was reached last, if it has not been seen to lift.
    /// Once its reset has passed it is no longer reached, whether or not
    /// this is written down yet.
    pub reached: Option<ReachedLimit>,
    /// The windows of the plan as the program last reported them.
    pub windows: Vec<PlanWindow>,
    /// When the program last reported them.
    pub reported_at: Option<Timestamp>,
}

/// A limit of the plan that was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachedLimit {
    /// When a turn ended because of it.
    pub at: Timestamp,
    /// Which limit it is, if the program said.
    pub window: Option<PlanWindowKind>,
    /// When it resets, if the program said in a way that is understood.
    pub resets_at: Option<Timestamp>,
}

impl PlanLimitState {
    /// The limit, if it is reached at `now`: reached, and its reset, if
    /// known, still to come.
    #[must_use]
    pub fn reached_at(&self, now: Timestamp) -> Option<&ReachedLimit> {
        self.reached
            .as_ref()
            .filter(|limit| limit.resets_at.is_none_or(|resets_at| now < resets_at))
    }

    /// What may be shown at `now`: the limit only if it is still reached,
    /// and of each window what is still true. A window whose reset has
    /// passed has started again, so how much of it is used is no longer
    /// known.
    #[must_use]
    pub fn as_of(&self, now: Timestamp) -> Self {
        Self {
            reached: self.reached_at(now).cloned(),
            windows: self
                .windows
                .iter()
                .map(|window| {
                    if window.resets_at.is_some_and(|resets_at| resets_at <= now) {
                        PlanWindow {
                            used: None,
                            resets_at: None,
                            status: None,
                            ..window.clone()
                        }
                    } else {
                        window.clone()
                    }
                })
                .collect(),
            reported_at: self.reported_at,
        }
    }
}

/// What the work of the specialists learned about the plan.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanReport {
    /// The program reported how much of the windows is used.
    Usage(PlanUsage),
    /// A turn ended because a limit was reached.
    LimitReached(PlanLimitHit),
    /// A turn that started at this moment finished: the plan allowed it.
    Worked {
        /// When the turn started.
        started_at: Timestamp,
    },
}

/// Whether what was learned changed whether the limit is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanLimitChange {
    /// Nothing that matters to the manager changed.
    Nothing,
    /// The limit is reached now, and was not before.
    Reached(ReachedLimit),
    /// The limit was reached and is no longer.
    Lifted,
}

/// Whether new work may be started now, as far as the plan's limit is
/// known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkPermit {
    /// It may: the limit is not known to be reached.
    Allowed,
    /// It may not until this moment, when the limit resets.
    NotUntil(Timestamp),
    /// It may not, and when the limit resets is not known. Only a turn
    /// that is tried can tell whether it has; the manager may try one.
    NotUntilUnknown,
}

/// Whether new work may be started at `now`, as far as the plan's limit is
/// known. It is for the coordinator, which starts work by itself: work it
/// starts while the limit is reached ends at once, and only uses the
/// manager's patience. The manager may still start work by hand.
#[must_use]
pub fn may_start_work(state: &PlanLimitState, now: Timestamp) -> WorkPermit {
    match state.reached_at(now) {
        None => WorkPermit::Allowed,
        Some(ReachedLimit {
            resets_at: Some(resets_at),
            ..
        }) => WorkPermit::NotUntil(*resets_at),
        Some(ReachedLimit {
            resets_at: None, ..
        }) => WorkPermit::NotUntilUnknown,
    }
}

/// Takes what the work learned into what is known, at `now`. Returns the
/// new state and what changed for the manager.
///
/// A limit that is already reached is not reached again: eight turns that
/// end at the same limit are told once. A later reset that becomes known
/// replaces an unknown one. The limit lifts when its reset passes, when the
/// program says the plan allows work and the reset is not known to be
/// still to come, and when a turn that started after the limit was reached
/// finished.
#[must_use]
pub fn take_report(
    state: &PlanLimitState,
    report: &PlanReport,
    now: Timestamp,
    clock: &dyn Clock,
) -> (PlanLimitState, PlanLimitChange) {
    let mut next = state.clone();
    let was_reached = state.reached_at(now).is_some();
    match report {
        PlanReport::Usage(usage) => {
            if !usage.windows.is_empty() {
                next.windows.clone_from(&usage.windows);
            }
            next.reported_at = Some(now);
            let allows = usage
                .status
                .as_deref()
                .is_some_and(|status| ALLOWED.contains(&status));
            let reset_still_to_come = state
                .reached_at(now)
                .and_then(|limit| limit.resets_at)
                .is_some();
            if allows && !reset_still_to_come {
                next.reached = None;
            }
        }
        PlanReport::LimitReached(hit) => {
            let resets_at = reset_moment(&hit.resets, now, clock);
            match next.reached.as_mut().filter(|_| was_reached) {
                Some(limit) => {
                    if resets_at.is_some() {
                        limit.resets_at = resets_at;
                    }
                    if limit.window.is_none() {
                        limit.window.clone_from(&hit.window);
                    }
                }
                None => {
                    next.reached = Some(ReachedLimit {
                        at: now,
                        window: hit.window.clone(),
                        resets_at,
                    });
                }
            }
        }
        PlanReport::Worked { started_at } => {
            if state
                .reached_at(now)
                .is_some_and(|limit| limit.at <= *started_at)
            {
                next.reached = None;
            }
        }
    }
    if !was_reached && next.reached_at(now).is_none() && state.reached.is_some() {
        // The reset passed before anything was learned: it lifted then.
        next.reached = None;
    }
    let change = match (was_reached, next.reached_at(now)) {
        (false, Some(limit)) => PlanLimitChange::Reached(limit.clone()),
        (true, None) => PlanLimitChange::Lifted,
        _ => PlanLimitChange::Nothing,
    };
    (next, change)
}

/// The moment a limit resets, from what the program said, at `now`.
///
/// A time of day without a date is the next time the clocks of its zone
/// show it after `now`: "10:40pm" said at 20:15 is today, said at 23:00 it
/// is tomorrow. A date without a year is the next such date. The zone is
/// the one named, or that of this computer; a zone the clock does not know
/// leaves the moment unknown rather than guessed.
#[must_use]
pub fn reset_moment(resets: &PlanReset, now: Timestamp, clock: &dyn Clock) -> Option<Timestamp> {
    match resets {
        PlanReset::At(moment) => Some(*moment),
        PlanReset::Unknown => None,
        PlanReset::Said(said) => said_moment(said, now, clock),
    }
}

fn said_moment(said: &SaidReset, now: Timestamp, clock: &dyn Clock) -> Option<Timestamp> {
    let zone = said.zone.as_deref();
    let now_there = zone_time(now, zone, clock)?;
    let moment_of = |date: NaiveDate| {
        let local = NaiveDateTime::new(date, said.time);
        let offset = clock.offset_in(zone, local)?;
        offset
            .from_local_datetime(&local)
            .single()
            .map(|moment| moment.with_timezone(&Utc))
    };
    match said.date {
        None => {
            let today = now_there.date();
            // Today, or else tomorrow; a day whose clocks skip the time is
            // passed over.
            (0..=2)
                .filter_map(|days| today.checked_add_days(Days::new(days)))
                .filter_map(moment_of)
                .find(|moment| *moment > now)
        }
        Some(date) => {
            let year = date.year.unwrap_or_else(|| now_there.year());
            let first = NaiveDate::from_ymd_opt(year, date.month, date.day)?;
            let moment = moment_of(first)?;
            if date.year.is_some() || moment > now {
                return Some(moment);
            }
            // Said without a year and already past: next year's.
            first
                .checked_add_months(Months::new(12))
                .and_then(moment_of)
        }
    }
}

/// The time on the clocks of a zone at a moment.
fn zone_time(now: Timestamp, zone: Option<&str>, clock: &dyn Clock) -> Option<NaiveDateTime> {
    if zone.is_none() {
        return Some(clock.local_time(now));
    }
    // The offset depends on the time there, which is not known yet: take
    // the offset at the same time in Coordinated Universal Time, and look
    // again with the time it gives, which settles it unless the clocks
    // there change within those hours.
    let guess = clock.offset_in(zone, now.naive_utc())?;
    let first = now.naive_utc() + guess;
    let offset = clock.offset_in(zone, first).unwrap_or(guess);
    Some(now.naive_utc() + offset)
}

/// When a limit resets, in words, as this computer's clocks show it: "at
/// 22:40" on the same day, "tomorrow at 07:00", or "on 23 November at
/// 10:00".
#[must_use]
pub fn reset_in_words(resets_at: Timestamp, now: Timestamp, clock: &dyn Clock) -> String {
    let then = clock.local_time(resets_at);
    let today = clock.local_time(now).date();
    let time = then.format("%H:%M");
    if then.date() == today {
        format!("at {time}")
    } else if today.succ_opt() == Some(then.date()) {
        format!("tomorrow at {time}")
    } else {
        let month = MONTHS
            .get(usize::try_from(then.month0()).unwrap_or_default())
            .copied()
            .unwrap_or_default();
        format!("on {} {month} at {time}", then.day())
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The notification of a change of the limit, or nothing if nothing
/// changed. It is of the kind of a specialist that could not go on: work
/// stops.
#[must_use]
pub fn notification_of_plan_limit(
    change: &PlanLimitChange,
    now: Timestamp,
    clock: &dyn Clock,
) -> Option<Notification> {
    let (title, body) = match change {
        PlanLimitChange::Nothing => return None,
        PlanLimitChange::Reached(limit) => (
            "The limit of your Claude plan is reached".to_owned(),
            match limit.resets_at {
                Some(resets_at) => format!(
                    "It resets {}. No new work starts before then.",
                    reset_in_words(resets_at, now, clock)
                ),
                None => "When it resets is not known. No new work starts by itself.".to_owned(),
            },
        ),
        PlanLimitChange::Lifted => (
            "The limit of your Claude plan has reset".to_owned(),
            "The specialists can work again.".to_owned(),
        ),
    };
    Some(Notification {
        kind: NotificationKind::Stopped,
        title,
        body,
    })
}

/// Keeps what this computer knows about its Claude plan.
pub trait PlanLimitStore: Send + Sync {
    /// What is known, or nothing known if nothing was ever kept.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be read.
    fn plan_limit(&self) -> Result<PlanLimitState, StorageError>;

    /// Keeps what is known in place of what was before.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be kept.
    fn set_plan_limit(&self, state: &PlanLimitState) -> Result<(), StorageError>;
}

/// The use cases of the plan's limit on this computer.
#[derive(Clone)]
pub struct PlanLimitService {
    store: Arc<dyn PlanLimitStore>,
    clock: Arc<dyn Clock>,
}

impl PlanLimitService {
    /// Creates the service on its store and a clock.
    #[must_use]
    pub fn new(store: Arc<dyn PlanLimitStore>, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// What is known now: the limit if it is still reached, and the
    /// windows as far as they are still true.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be read.
    pub fn standing(&self) -> Result<PlanLimitState, ApplicationError> {
        Ok(self.store.plan_limit()?.as_of(self.clock.now()))
    }

    /// Whether new work may be started now (see [`may_start_work`]).
    ///
    /// # Errors
    ///
    /// Returns an error if what is known could not be read.
    pub fn may_start_work(&self) -> Result<WorkPermit, ApplicationError> {
        Ok(may_start_work(&self.store.plan_limit()?, self.clock.now()))
    }

    /// Takes what the work learned, keeps it, and says what changed.
    ///
    /// # Errors
    ///
    /// Returns an error if what is known could not be read or kept.
    pub fn take(&self, report: &PlanReport) -> Result<PlanLimitChange, ApplicationError> {
        let state = self.store.plan_limit()?;
        let (next, change) = take_report(&state, report, self.clock.now(), self.clock.as_ref());
        if next != state {
            self.store.set_plan_limit(&next)?;
        }
        Ok(change)
    }

    /// Looks whether a reached limit has reset by now, and forgets it if
    /// it has. Called once at the moment it resets, not over and over.
    ///
    /// # Errors
    ///
    /// Returns an error if what is known could not be read or kept.
    pub fn look_at_reset(&self) -> Result<PlanLimitChange, ApplicationError> {
        let mut state = self.store.plan_limit()?;
        let now = self.clock.now();
        if state.reached.is_some() && state.reached_at(now).is_none() {
            state.reached = None;
            self.store.set_plan_limit(&state)?;
            return Ok(PlanLimitChange::Lifted);
        }
        Ok(PlanLimitChange::Nothing)
    }

    /// The notification of a change, in words.
    #[must_use]
    pub fn notification_of(&self, change: &PlanLimitChange) -> Option<Notification> {
        notification_of_plan_limit(change, self.clock.now(), self.clock.as_ref())
    }

    /// When the reached limit resets, if it is reached and that is known:
    /// the moment at which to look again.
    ///
    /// # Errors
    ///
    /// Returns an error if what is known could not be read.
    pub fn next_reset(&self) -> Result<Option<Timestamp>, ApplicationError> {
        Ok(self
            .store
            .plan_limit()?
            .reached_at(self.clock.now())
            .and_then(|limit| limit.resets_at))
    }
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, NaiveTime};

    use super::*;
    use crate::agent_runtime::SaidDate;
    use crate::in_memory::{InMemoryPlanLimitStore, ManualClock};

    /// A clock in a zone of a fixed offset, called by a name, which knows
    /// no other zone than Coordinated Universal Time.
    struct ZoneClock {
        now: Timestamp,
        name: &'static str,
        offset: FixedOffset,
    }

    impl Clock for ZoneClock {
        fn now(&self) -> Timestamp {
            self.now
        }

        fn local_time(&self, moment: Timestamp) -> NaiveDateTime {
            moment.with_timezone(&self.offset).naive_local()
        }

        fn offset_in(&self, zone: Option<&str>, _local: NaiveDateTime) -> Option<FixedOffset> {
            match zone {
                None => Some(self.offset),
                Some(name) if name == self.name => Some(self.offset),
                Some(name) => crate::ports::utc_offset_of(name),
            }
        }
    }

    fn istanbul(now: Timestamp) -> ZoneClock {
        ZoneClock {
            now,
            name: "Europe/Istanbul",
            offset: FixedOffset::east_opt(3 * 3600).unwrap(),
        }
    }

    fn at(year: i32, month: u32, date: u32, hour: u32, minute: u32) -> Timestamp {
        Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(year, month, date)
                .unwrap()
                .and_hms_opt(hour, minute, 0)
                .unwrap(),
        )
    }

    fn said(hour: u32, minute: u32, date: Option<SaidDate>, zone: Option<&str>) -> PlanReset {
        PlanReset::Said(SaidReset {
            time: NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
            date,
            zone: zone.map(str::to_owned),
        })
    }

    fn hit(resets: PlanReset) -> PlanLimitHit {
        PlanLimitHit {
            window: Some(PlanWindowKind::FiveHours),
            resets,
        }
    }

    #[test]
    fn the_reset_of_tonight_is_the_same_evening() {
        // 20:15 in Istanbul is 17:15 in Coordinated Universal Time.
        let now = at(2026, 11, 21, 17, 15);
        let clock = istanbul(now);
        let resets = said(22, 40, None, Some("Europe/Istanbul"));

        assert_eq!(
            reset_moment(&resets, now, &clock),
            Some(at(2026, 11, 21, 19, 40))
        );
    }

    #[test]
    fn a_time_already_past_today_is_tomorrow_also_across_midnight() {
        let clock = istanbul(at(2026, 11, 21, 20, 30));
        // 23:30 in Istanbul: 22:40 has passed, so it is tomorrow's.
        assert_eq!(
            reset_moment(&said(22, 40, None, None), clock.now, &clock),
            Some(at(2026, 11, 22, 19, 40))
        );
        // 23:30 in Istanbul, a reset at 00:10: twenty minutes later, on the
        // next day there, which is still the same day in UTC.
        assert_eq!(
            reset_moment(&said(0, 10, None, None), clock.now, &clock),
            Some(at(2026, 11, 21, 21, 10))
        );
        // 00:30 in Istanbul (21:30 UTC the day before), a reset at 02:00:
        // the same day there.
        let after_midnight = istanbul(at(2026, 11, 21, 21, 30));
        assert_eq!(
            reset_moment(&said(2, 0, None, None), after_midnight.now, &after_midnight),
            Some(at(2026, 11, 21, 23, 0))
        );
        // Exactly now is not after now: the next one.
        let exact = istanbul(at(2026, 11, 21, 19, 40));
        assert_eq!(
            reset_moment(&said(22, 40, None, None), exact.now, &exact),
            Some(at(2026, 11, 22, 19, 40))
        );
    }

    #[test]
    fn a_reset_in_coordinated_universal_time_is_known_anywhere() {
        let clock = istanbul(at(2026, 11, 21, 17, 15));
        assert_eq!(
            reset_moment(&said(22, 40, None, Some("UTC")), clock.now, &clock),
            Some(at(2026, 11, 21, 22, 40))
        );
    }

    #[test]
    fn a_zone_the_clock_does_not_know_leaves_the_reset_unknown() {
        let clock = istanbul(at(2026, 11, 21, 17, 15));
        assert_eq!(
            reset_moment(
                &said(22, 40, None, Some("America/New_York")),
                clock.now,
                &clock
            ),
            None
        );
        assert_eq!(reset_moment(&PlanReset::Unknown, clock.now, &clock), None);
    }

    #[test]
    fn a_date_is_this_year_or_the_next() {
        let clock = istanbul(at(2026, 11, 21, 17, 15));
        let november = said(
            10,
            0,
            Some(SaidDate {
                month: 11,
                day: 23,
                year: None,
            }),
            Some("Europe/Istanbul"),
        );
        assert_eq!(
            reset_moment(&november, clock.now, &clock),
            Some(at(2026, 11, 23, 7, 0))
        );
        let january = said(
            10,
            0,
            Some(SaidDate {
                month: 1,
                day: 2,
                year: None,
            }),
            None,
        );
        assert_eq!(
            reset_moment(&january, clock.now, &clock),
            Some(at(2027, 1, 2, 7, 0))
        );
        let impossible = said(
            10,
            0,
            Some(SaidDate {
                month: 2,
                day: 30,
                year: Some(2027),
            }),
            None,
        );
        assert_eq!(reset_moment(&impossible, clock.now, &clock), None);
    }

    #[test]
    fn eight_turns_at_the_same_limit_are_told_once_and_work_waits_for_the_reset() {
        let now = at(2026, 11, 21, 17, 15);
        let clock = istanbul(now);
        let mut state = PlanLimitState::default();
        let mut changes = Vec::new();
        for _ in 0..8 {
            let (next, change) = take_report(
                &state,
                &PlanReport::LimitReached(hit(said(22, 40, None, Some("Europe/Istanbul")))),
                now,
                &clock,
            );
            state = next;
            changes.push(change);
        }
        let reached = ReachedLimit {
            at: now,
            window: Some(PlanWindowKind::FiveHours),
            resets_at: Some(at(2026, 11, 21, 19, 40)),
        };
        assert_eq!(changes[0], PlanLimitChange::Reached(reached));
        assert!(
            changes[1..]
                .iter()
                .all(|change| *change == PlanLimitChange::Nothing)
        );

        assert_eq!(
            may_start_work(&state, now),
            WorkPermit::NotUntil(at(2026, 11, 21, 19, 40))
        );
        assert_eq!(
            may_start_work(&state, at(2026, 11, 21, 19, 39)),
            WorkPermit::NotUntil(at(2026, 11, 21, 19, 40))
        );
        assert_eq!(
            may_start_work(&state, at(2026, 11, 21, 19, 40)),
            WorkPermit::Allowed
        );
        assert_eq!(
            may_start_work(&PlanLimitState::default(), now),
            WorkPermit::Allowed
        );
    }

    #[test]
    fn a_limit_without_a_known_reset_holds_work_until_the_plan_is_seen_to_allow_it() {
        let now = at(2026, 11, 21, 17, 15);
        let clock = ManualClock::starting_at(now);
        let (state, _) = take_report(
            &PlanLimitState::default(),
            &PlanReport::LimitReached(hit(PlanReset::Unknown)),
            now,
            &clock,
        );
        assert_eq!(may_start_work(&state, now), WorkPermit::NotUntilUnknown);

        // A turn that started before the limit and finished after it says
        // nothing about the limit.
        let (same, change) = take_report(
            &state,
            &PlanReport::Worked {
                started_at: at(2026, 11, 21, 17, 0),
            },
            now,
            &clock,
        );
        assert_eq!(change, PlanLimitChange::Nothing);
        assert_eq!(same, state);

        // One that started after it, by the manager's hand, and finished.
        let later = at(2026, 11, 21, 20, 0);
        let (lifted, change) = take_report(
            &state,
            &PlanReport::Worked { started_at: later },
            later,
            &clock,
        );
        assert_eq!(change, PlanLimitChange::Lifted);
        assert_eq!(may_start_work(&lifted, later), WorkPermit::Allowed);

        // Or the program saying the plan allows work.
        let usage = PlanUsage {
            windows: Vec::new(),
            status: Some("allowed".to_owned()),
        };
        let (lifted, change) = take_report(&state, &PlanReport::Usage(usage), later, &clock);
        assert_eq!(change, PlanLimitChange::Lifted);
        assert_eq!(lifted.reached, None);
    }

    #[test]
    fn the_plan_allowing_work_before_a_known_reset_does_not_lift_it() {
        let now = at(2026, 11, 21, 17, 15);
        let clock = istanbul(now);
        let (state, _) = take_report(
            &PlanLimitState::default(),
            &PlanReport::LimitReached(hit(PlanReset::At(at(2026, 11, 21, 19, 40)))),
            now,
            &clock,
        );
        let usage = PlanUsage {
            windows: vec![PlanWindow {
                kind: PlanWindowKind::Week,
                used: Some(0.4),
                resets_at: Some(at(2026, 11, 25, 0, 0)),
                status: Some("allowed".to_owned()),
            }],
            status: Some("allowed".to_owned()),
        };
        let (next, change) = take_report(&state, &PlanReport::Usage(usage), now, &clock);
        assert_eq!(change, PlanLimitChange::Nothing);
        assert!(next.reached_at(now).is_some());
        assert_eq!(next.windows.len(), 1);
        assert_eq!(next.reported_at, Some(now));
    }

    #[test]
    fn a_window_whose_reset_passed_is_no_longer_known() {
        let state = PlanLimitState {
            reached: None,
            windows: vec![PlanWindow {
                kind: PlanWindowKind::FiveHours,
                used: Some(0.9),
                resets_at: Some(at(2026, 11, 21, 19, 40)),
                status: Some("allowed_warning".to_owned()),
            }],
            reported_at: Some(at(2026, 11, 21, 17, 0)),
        };
        assert_eq!(
            state.as_of(at(2026, 11, 21, 19, 0)).windows[0].used,
            Some(0.9)
        );
        let later = state.as_of(at(2026, 11, 21, 19, 40));
        assert_eq!(later.windows[0].used, None);
        assert_eq!(later.windows[0].kind, PlanWindowKind::FiveHours);
    }

    #[test]
    fn the_service_keeps_tells_and_lifts_at_the_reset() {
        let store = Arc::new(InMemoryPlanLimitStore::default());
        let clock = Arc::new(ManualClock::starting_at(at(2026, 11, 21, 17, 15)));
        let service = PlanLimitService::new(store, clock.clone());

        let change = service
            .take(&PlanReport::LimitReached(hit(PlanReset::At(at(
                2026, 11, 21, 19, 40,
            )))))
            .unwrap();
        let notification = service.notification_of(&change).unwrap();
        assert_eq!(
            notification.title,
            "The limit of your Claude plan is reached"
        );
        assert_eq!(
            notification.body,
            "It resets at 19:40. No new work starts before then."
        );
        assert_eq!(notification.kind, NotificationKind::Stopped);
        assert_eq!(
            service.next_reset().unwrap(),
            Some(at(2026, 11, 21, 19, 40))
        );
        assert_eq!(
            service.may_start_work().unwrap(),
            WorkPermit::NotUntil(at(2026, 11, 21, 19, 40))
        );
        assert_eq!(service.look_at_reset().unwrap(), PlanLimitChange::Nothing);

        clock.set(at(2026, 11, 21, 19, 40));
        assert_eq!(service.standing().unwrap().reached, None);
        assert_eq!(service.look_at_reset().unwrap(), PlanLimitChange::Lifted);
        assert_eq!(service.look_at_reset().unwrap(), PlanLimitChange::Nothing);
        assert_eq!(service.next_reset().unwrap(), None);
        assert_eq!(
            service
                .notification_of(&PlanLimitChange::Lifted)
                .unwrap()
                .title,
            "The limit of your Claude plan has reset"
        );
    }

    #[test]
    fn a_reset_is_worded_by_the_clocks_of_this_computer() {
        let now = at(2026, 11, 21, 17, 15);
        let clock = istanbul(now);
        assert_eq!(
            reset_in_words(at(2026, 11, 21, 19, 40), now, &clock),
            "at 22:40"
        );
        assert_eq!(
            reset_in_words(at(2026, 11, 22, 4, 0), now, &clock),
            "tomorrow at 07:00"
        );
        assert_eq!(
            reset_in_words(at(2026, 11, 25, 7, 0), now, &clock),
            "on 25 November at 10:00"
        );
    }
}
