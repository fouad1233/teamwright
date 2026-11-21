//! What really happened to a task, against what was planned for it.
//!
//! A task is planned with a start and a due date. What happened is read off
//! the versions of the task that storage keeps in its log: when it first
//! went into a status of a started category (under way, in review or done),
//! and when it reached the category done. Only the category of a status
//! counts, never its name.

use crate::task::{Schedule, Task};
use crate::time::{CalendarDate, Timestamp};
use crate::workflow::{StatusCategory, Workflow};

/// When work on a task really began and ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActualDates {
    /// When it first went into a status of a started category. Nothing if
    /// it never did.
    pub started_at: Option<Timestamp>,
    /// When it reached the category done, if it is there now. A task that
    /// was finished and opened again is not finished until it is done
    /// again, and then counts from that last time.
    pub finished_at: Option<Timestamp>,
    /// Whether it was finished once and opened again.
    pub reopened: bool,
}

/// Reads what happened to a task off its versions, the oldest first. A
/// status the workflow no longer has counts as not started.
#[must_use]
pub fn actual_dates_of(versions: &[Task], workflow: &Workflow) -> ActualDates {
    let category = |version: &Task| {
        workflow
            .status(version.status_id())
            .map(|status| status.category())
    };
    let started_at = versions
        .iter()
        .find(|version| {
            matches!(
                category(version),
                Some(StatusCategory::Active | StatusCategory::Review | StatusCategory::Done)
            )
        })
        .map(Task::updated_at);
    let mut was_done = false;
    let mut reopened = false;
    for version in versions {
        let is_done = category(version) == Some(StatusCategory::Done);
        if was_done && !is_done {
            reopened = true;
        }
        was_done = is_done;
    }
    let finished_at = versions
        .last()
        .filter(|last| category(last) == Some(StatusCategory::Done))
        .and_then(|last| last.completed_at().or(Some(last.updated_at())));
    ActualDates {
        started_at,
        finished_at,
        reopened,
    }
}

/// How what happened compares with the plan, in days. Each is nothing when
/// it does not apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Timing {
    /// Work began this many days after the planned start, or has not begun
    /// this many days after it.
    pub started_late: Option<u32>,
    /// Whether work has not begun at all, while it is late to begin.
    pub not_started: bool,
    /// It was finished this many days after it was due.
    pub finished_late: Option<u32>,
    /// It is not finished, and its due day was this many days ago.
    pub running_late: Option<u32>,
}

/// Compares the days on which work on a task began and ended with its plan.
/// A task without a start day is never late to begin, and one without a
/// due day never late to end. Today is the day where the program runs.
#[must_use]
pub fn timing_of(
    plan: Schedule,
    started_on: Option<CalendarDate>,
    finished_on: Option<CalendarDate>,
    today: CalendarDate,
) -> Timing {
    let days_after = |later: CalendarDate, earlier: CalendarDate| {
        u32::try_from((later - earlier).num_days())
            .ok()
            .filter(|days| *days > 0)
    };
    let (started_late, not_started) = match (plan.start_on(), started_on) {
        (Some(start), Some(started)) => (days_after(started, start), false),
        (Some(start), None) if finished_on.is_none() => {
            let late = days_after(today, start);
            (late, late.is_some())
        }
        _ => (None, false),
    };
    let (finished_late, running_late) = match (plan.due_on(), finished_on) {
        (Some(due), Some(finished)) => (days_after(finished, due), None),
        (Some(due), None) => (None, days_after(today, due)),
        (None, _) => (None, None),
    };
    Timing {
        started_late,
        not_started,
        finished_late,
        running_late,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};

    use super::*;
    use crate::ids::{AreaId, ProjectId};
    use crate::task::{Rank, TaskDraft};
    use crate::text::TaskTitle;

    fn at(day_of_month: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, day_of_month, 9, 0, 0)
            .unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    /// The versions of a task moved through statuses of these categories,
    /// one a day from the 10th.
    fn moved_through(workflow: &Workflow, categories: &[StatusCategory]) -> Vec<Task> {
        let mut task = Task::create(
            ProjectId::generate(),
            AreaId::generate(),
            1,
            workflow.initial_status(),
            Rank::FIRST,
            TaskDraft::titled(TaskTitle::parse("Draw the schematic").unwrap()),
            at(10),
        );
        let mut versions = vec![task.clone()];
        for (offset, category) in categories.iter().enumerate() {
            let status = workflow.first_in(*category).unwrap();
            let moment = at(10) + TimeDelta::days(i64::try_from(offset).unwrap() + 1);
            task.move_to(status, Rank::FIRST, moment);
            versions.push(task.clone());
        }
        versions
    }

    #[test]
    fn work_begins_when_the_task_first_enters_a_started_category() {
        let workflow = Workflow::standard();
        let versions = moved_through(
            &workflow,
            &[
                StatusCategory::Todo,
                StatusCategory::Active,
                StatusCategory::Done,
            ],
        );

        assert_eq!(
            actual_dates_of(&versions, &workflow),
            ActualDates {
                started_at: Some(at(12)),
                finished_at: Some(at(13)),
                reopened: false
            }
        );
    }

    #[test]
    fn a_task_that_was_never_started_has_no_actual_dates() {
        let workflow = Workflow::standard();
        let versions = moved_through(&workflow, &[]);

        let actual = actual_dates_of(&versions, &workflow);

        assert_eq!((actual.started_at, actual.finished_at), (None, None));
    }

    #[test]
    fn a_reopened_task_is_finished_only_when_it_is_done_again() {
        let workflow = Workflow::standard();
        let reopened = moved_through(
            &workflow,
            &[
                StatusCategory::Active,
                StatusCategory::Done,
                StatusCategory::Active,
            ],
        );
        let done_again = moved_through(
            &workflow,
            &[
                StatusCategory::Active,
                StatusCategory::Done,
                StatusCategory::Active,
                StatusCategory::Done,
            ],
        );

        let running = actual_dates_of(&reopened, &workflow);
        let finished = actual_dates_of(&done_again, &workflow);

        assert_eq!(running.started_at, Some(at(11)));
        assert_eq!(running.finished_at, None);
        assert!(running.reopened);
        assert_eq!(finished.finished_at, Some(at(14)));
        assert!(finished.reopened);
    }

    #[test]
    fn a_task_moved_straight_to_done_began_when_it_was_finished() {
        let workflow = Workflow::standard();
        let versions = moved_through(&workflow, &[StatusCategory::Done]);

        let actual = actual_dates_of(&versions, &workflow);

        assert_eq!(actual.started_at, Some(at(11)));
        assert_eq!(actual.finished_at, Some(at(11)));
    }

    #[test]
    fn a_late_start_a_late_finish_and_work_running_past_its_due_day_are_told_apart() {
        let plan = Schedule::new(Some(day(10)), Some(day(12))).unwrap();

        assert_eq!(
            timing_of(plan, Some(day(11)), Some(day(14)), day(20)),
            Timing {
                started_late: Some(1),
                not_started: false,
                finished_late: Some(2),
                running_late: None
            }
        );
        assert_eq!(
            timing_of(plan, Some(day(10)), None, day(15)),
            Timing {
                running_late: Some(3),
                ..Timing::default()
            }
        );
        assert_eq!(
            timing_of(plan, None, None, day(11)),
            Timing {
                started_late: Some(1),
                not_started: true,
                ..Timing::default()
            }
        );
        assert_eq!(
            timing_of(plan, Some(day(9)), Some(day(12)), day(20)),
            Timing::default()
        );
    }

    #[test]
    fn a_task_without_planned_dates_is_never_late() {
        assert_eq!(
            timing_of(Schedule::unscheduled(), Some(day(11)), None, day(30)),
            Timing::default()
        );
    }
}
