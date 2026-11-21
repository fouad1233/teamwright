//! The course of a sprint, read off the versions of its tasks.
//!
//! Storage keeps every version a task has had, in its log of operations
//! ([`task_history`](crate::task_history) reads the same versions). From
//! them this module tells what was in a sprint at the end of each of its
//! days, so that its burndown follows the tasks that joined or left it
//! while it ran, and which tasks moved on when it was completed.
//!
//! A task is in a sprint while a version of it names the sprint and it is
//! not a subtask: a subtask is part of its task and is counted with it.

use crate::ids::{SprintId, TaskId};
use crate::sprint::{Sprint, SprintState, SprintUnit};
use crate::task::Task;
use crate::text::TaskTitle;
use crate::time::{CalendarDate, Timestamp};
use crate::workflow::{StatusCategory, Workflow};

/// Everything storage knows of the past of one task: every version it has
/// had, the oldest first, and when it was deleted, if it was.
#[derive(Debug, Clone, Copy)]
pub struct TaskPast<'a> {
    /// Every version of the task, in the order they were stored.
    pub versions: &'a [Task],
    /// When the task was deleted. Nothing for a task that is still there,
    /// or for storage that does not know.
    pub deleted_at: Option<Timestamp>,
}

/// A task as it stood in a sprint: which one, and what it weighed there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeTask {
    /// The task.
    pub task_id: TaskId,
    /// Its number in the project.
    pub number: u32,
    /// Its title at the time.
    pub title: TaskTitle,
    /// Its work in the unit of the sprint.
    pub work: u32,
    /// Whether it has no points while the sprint counts points. Such a task
    /// weighs nothing, and the manager is told so.
    pub has_no_points: bool,
}

/// What changed in what a sprint held, on one of its days.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeChange {
    /// The day.
    pub date: CalendarDate,
    /// The tasks that came into the sprint that day, by their number.
    pub joined: Vec<ScopeTask>,
    /// The tasks that went out of it that day: planned elsewhere, taken
    /// back to the backlog, made a subtask, or deleted. By their number.
    pub left: Vec<ScopeTask>,
    /// How much the work of the tasks that stayed changed that day, because
    /// their points changed. Positive when it grew.
    pub reestimated: i64,
}

impl ScopeChange {
    /// The work that came in with the tasks that joined.
    #[must_use]
    pub fn work_joined(&self) -> u32 {
        sum_of_work(&self.joined)
    }

    /// The work that went out with the tasks that left.
    #[must_use]
    pub fn work_left(&self) -> u32 {
        sum_of_work(&self.left)
    }
}

/// A task that was not finished when its sprint was completed, and where
/// it went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedOn {
    /// The task as it stood in the sprint just before.
    pub task: ScopeTask,
    /// The sprint it went to, or nothing for the backlog.
    pub to: Option<SprintId>,
}

/// One day of a sprint in its course.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseDay {
    /// The day.
    pub date: CalendarDate,
    /// The work that remained at the end of the day. Nothing for a day that
    /// has not come yet, or that came after the sprint was completed.
    pub remaining: Option<u32>,
    /// All the work the sprint held at the end of the day, finished or
    /// not. Nothing for a day that has not come.
    pub scope: Option<u32>,
}

/// The course of a sprint, for its burndown chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintCourse {
    /// What the work is counted in.
    pub unit: SprintUnit,
    /// The work the sprint held when it was started. For a sprint that is
    /// planned, the work it holds now.
    pub committed: u32,
    /// The work the sprint holds now, or held just before it was completed.
    pub total: u32,
    /// Every day from the start date to the end date, in order. Empty
    /// without both dates.
    pub days: Vec<CourseDay>,
    /// The days on which tasks joined or left the sprint, or changed their
    /// points, in order. Only days that have come.
    pub changes: Vec<ScopeChange>,
    /// The tasks that moved on when the sprint was completed, by their
    /// number. Empty for a sprint that is not completed.
    pub moved_on: Vec<MovedOn>,
}

/// Works out the course of a sprint from the past of the tasks of its
/// project.
///
/// What the sprint held at the end of a day is read off the last version
/// of each task stored by then. The tasks that were in the sprint when it
/// was started count from the first day, whenever that was, so a sprint
/// started late or early does not show its first tasks as joining. Nothing
/// stored after the moment the sprint was completed counts, and the
/// unfinished tasks that left with the completion are still in its last
/// day. A task counts as finished while its version is in a status of the
/// category done.
///
/// A task that left the sprint when it was completed moved on: its version
/// stored at that very moment names where it went. Completing a sprint
/// stores the sprint and the tasks that move in one write with one moment,
/// so this needs nothing kept of its own.
///
/// `today` and `day_of` give days where the program runs. Days after today,
/// and after the day the sprint was completed, have not come.
#[must_use]
pub fn course_of(
    sprint: &Sprint,
    workflow: &Workflow,
    pasts: &[TaskPast<'_>],
    today: CalendarDate,
    day_of: &dyn Fn(Timestamp) -> CalendarDate,
) -> SprintCourse {
    let reader = Reader {
        sprint,
        workflow,
        unit: unit_of(sprint.id(), pasts),
    };
    let completed_at = sprint.completed_at();
    let started_at = sprint.started_at();
    let before_completion = |moment: Timestamp| completed_at.is_none_or(|end| moment <= end);
    let by_start = |moment: Timestamp| started_at.is_some_and(|start| moment <= start);

    let held_last = reader.held(pasts, &before_completion, &before_completion);
    let held_first = if started_at.is_some() {
        reader.held(pasts, &by_start, &by_start)
    } else {
        held_last.clone()
    };

    let last_day_come = match sprint.state() {
        SprintState::Planned => None,
        SprintState::Active | SprintState::Completed => {
            Some(completed_at.map_or(today, |moment| today.min(day_of(moment))))
        }
    };

    let mut days = Vec::new();
    let mut changes = Vec::new();
    let dates = sprint.dates();
    if let (Some(start_on), Some(end_on)) = (dates.start_on(), dates.end_on()) {
        let mut before = held_first.clone();
        for date in start_on.iter_days().take_while(|day| *day <= end_on) {
            if !last_day_come.is_some_and(|last| date <= last) {
                days.push(CourseDay {
                    date,
                    remaining: None,
                    scope: None,
                });
                continue;
            }
            let by_end_of_day = |moment| before_completion(moment) && day_of(moment) <= date;
            let held = reader.held(
                pasts,
                &|moment| by_end_of_day(moment) || (before_completion(moment) && by_start(moment)),
                &by_end_of_day,
            );
            days.push(CourseDay {
                date,
                remaining: Some(work_of(held.iter().filter(|each| !each.is_finished))),
                scope: Some(work_of(held.iter())),
            });
            if let Some(change) = change_between(&before, &held, date) {
                changes.push(change);
            }
            before = held;
        }
    }

    let moved_on = completed_at.map_or_else(Vec::new, |end| {
        held_last
            .iter()
            .filter(|each| !each.is_finished)
            .filter_map(|each| {
                let past = pasts.get(each.past)?;
                let at_completion = past
                    .versions
                    .iter()
                    .find(|version| version.updated_at() == end)?;
                (at_completion.sprint_id() != Some(sprint.id())).then(|| MovedOn {
                    task: each.task.clone(),
                    to: at_completion.sprint_id(),
                })
            })
            .collect()
    });

    SprintCourse {
        unit: reader.unit,
        committed: work_of(held_first.iter()),
        total: work_of(held_last.iter()),
        days,
        changes,
        moved_on,
    }
}

/// A task as a sprint held it at some moment.
#[derive(Debug, Clone)]
struct Held {
    /// Where its past is in the list that was given.
    past: usize,
    task: ScopeTask,
    is_finished: bool,
}

/// Reads what a sprint held.
struct Reader<'a> {
    sprint: &'a Sprint,
    workflow: &'a Workflow,
    unit: SprintUnit,
}

impl Reader<'_> {
    /// The last version of a task stored at a moment `counts` lets through.
    /// The version that took the task out of the sprint as it was completed
    /// does not count: until the end, the sprint held its unfinished tasks.
    fn last_version<'p>(
        &self,
        past: &TaskPast<'p>,
        counts: &dyn Fn(Timestamp) -> bool,
    ) -> Option<&'p Task> {
        let completed_at = self.sprint.completed_at();
        past.versions.iter().rev().find(|version| {
            counts(version.updated_at())
                && !(Some(version.updated_at()) == completed_at
                    && !is_in(self.sprint.id(), version))
        })
    }

    /// What the sprint held once the versions stored at the moments `counts`
    /// lets through were stored, by the number of the task. Whether a task
    /// was finished is read off the versions `finished_by` lets through:
    /// the tasks a sprint was started with are in it from its first day,
    /// but they were not finished any earlier than they were.
    fn held(
        &self,
        pasts: &[TaskPast<'_>],
        counts: &dyn Fn(Timestamp) -> bool,
        finished_by: &dyn Fn(Timestamp) -> bool,
    ) -> Vec<Held> {
        let mut held: Vec<Held> = pasts
            .iter()
            .enumerate()
            .filter(|(_, past)| !past.deleted_at.is_some_and(counts))
            .filter_map(|(index, past)| {
                let version = self.last_version(past, counts)?;
                let status = self.last_version(past, finished_by).map(Task::status_id);
                is_in(self.sprint.id(), version).then(|| Held {
                    past: index,
                    task: ScopeTask {
                        task_id: version.id(),
                        number: version.number(),
                        title: version.title().clone(),
                        work: work_in(self.unit, version),
                        has_no_points: self.unit == SprintUnit::Points
                            && version.points().is_none(),
                    },
                    is_finished: status
                        .and_then(|status| self.workflow.status(status))
                        .is_some_and(|status| status.category() == StatusCategory::Done),
                })
            })
            .collect();
        held.sort_by_key(|each| each.task.number);
        held
    }
}

/// Whether a version of a task is in a sprint as its own work.
fn is_in(sprint_id: SprintId, version: &Task) -> bool {
    version.sprint_id() == Some(sprint_id) && version.parent_id().is_none()
}

/// The unit of a sprint: points if any task had points while it was in it.
fn unit_of(sprint_id: SprintId, pasts: &[TaskPast<'_>]) -> SprintUnit {
    let has_points = pasts
        .iter()
        .flat_map(|past| past.versions)
        .any(|version| is_in(sprint_id, version) && version.points().is_some());
    if has_points {
        SprintUnit::Points
    } else {
        SprintUnit::Tasks
    }
}

/// The work of a version of a task in a unit.
fn work_in(unit: SprintUnit, version: &Task) -> u32 {
    match unit {
        SprintUnit::Points => version
            .points()
            .map_or(0, |points| u32::from(points.value())),
        SprintUnit::Tasks => 1,
    }
}

fn work_of<'a>(held: impl Iterator<Item = &'a Held>) -> u32 {
    held.map(|each| each.task.work).fold(0, u32::saturating_add)
}

fn sum_of_work(tasks: &[ScopeTask]) -> u32 {
    tasks
        .iter()
        .map(|task| task.work)
        .fold(0, u32::saturating_add)
}

/// What changed from what the sprint held `before` to what it holds
/// `after`, or nothing if nothing did.
fn change_between(before: &[Held], after: &[Held], date: CalendarDate) -> Option<ScopeChange> {
    let find = |list: &[Held], id: TaskId| -> Option<u32> {
        list.iter()
            .find(|each| each.task.task_id == id)
            .map(|each| each.task.work)
    };
    let joined: Vec<ScopeTask> = after
        .iter()
        .filter(|each| find(before, each.task.task_id).is_none())
        .map(|each| each.task.clone())
        .collect();
    let left: Vec<ScopeTask> = before
        .iter()
        .filter(|each| find(after, each.task.task_id).is_none())
        .map(|each| each.task.clone())
        .collect();
    let reestimated: i64 = after
        .iter()
        .filter_map(|each| {
            find(before, each.task.task_id)
                .map(|earlier| i64::from(each.task.work) - i64::from(earlier))
        })
        .sum();
    (!joined.is_empty() || !left.is_empty() || reestimated != 0).then_some(ScopeChange {
        date,
        joined,
        left,
        reestimated,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};

    use super::*;
    use crate::ids::{AreaId, ProjectId};
    use crate::sprint::{SprintDates, SprintSummary, StoryPoints};
    use crate::task::{Rank, TaskDraft};
    use crate::text::SprintName;

    /// Nine o'clock on a day of November 2026, plus some hours.
    fn at(day_of_month: u32, hours: i64) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, day_of_month, 9, 0, 0)
            .unwrap()
            + TimeDelta::hours(hours)
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    fn day_of(moment: Timestamp) -> CalendarDate {
        moment.date_naive()
    }

    /// The story of one task: its versions, each made from the one before.
    struct Story {
        versions: Vec<Task>,
        deleted_at: Option<Timestamp>,
    }

    impl Story {
        fn opened(number: u32, workflow: &Workflow, moment: Timestamp) -> Self {
            let task = Task::create(
                ProjectId::generate(),
                AreaId::generate(),
                number,
                workflow.initial_status(),
                Rank::FIRST,
                TaskDraft::titled(TaskTitle::parse(&format!("Task {number}")).unwrap()),
                moment,
            );
            Self {
                versions: vec![task],
                deleted_at: None,
            }
        }

        fn then(mut self, change: impl FnOnce(&mut Task)) -> Self {
            let mut next = self.versions.last().unwrap().clone();
            change(&mut next);
            self.versions.push(next);
            self
        }

        fn past(&self) -> TaskPast<'_> {
            TaskPast {
                versions: &self.versions,
                deleted_at: self.deleted_at,
            }
        }
    }

    fn points(value: u16) -> Option<StoryPoints> {
        Some(StoryPoints::new(i64::from(value)).unwrap())
    }

    fn done(workflow: &Workflow) -> &crate::workflow::Status {
        workflow.first_in(StatusCategory::Done).unwrap()
    }

    /// A sprint from the 10th to the 14th, started on the 10th at nine.
    fn running_sprint() -> Sprint {
        let mut sprint = Sprint::plan(
            ProjectId::generate(),
            SprintName::numbered(1),
            "",
            SprintDates::default(),
            at(9, 0),
        );
        sprint
            .start("Sprint 1", "", day(10), day(14), at(10, 0))
            .unwrap();
        sprint
    }

    fn pasts(stories: &[Story]) -> Vec<TaskPast<'_>> {
        stories.iter().map(Story::past).collect()
    }

    fn remaining(course: &SprintCourse) -> Vec<Option<u32>> {
        course.days.iter().map(|each| each.remaining).collect()
    }

    fn scope(course: &SprintCourse) -> Vec<Option<u32>> {
        course.days.iter().map(|each| each.scope).collect()
    }

    #[test]
    fn a_task_that_joins_raises_the_line_on_its_day_and_is_named() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let stories = [
            Story::opened(1, &workflow, at(9, 0)).then(|task| {
                task.set_points(points(5), at(9, 1));
                task.plan_into(Some(id), at(9, 2));
            }),
            Story::opened(2, &workflow, at(9, 0)).then(|task| {
                task.set_points(points(3), at(11, 4));
                task.plan_into(Some(id), at(12, 4));
            }),
        ];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(13), &day_of);

        assert_eq!(course.unit, SprintUnit::Points);
        assert_eq!(course.committed, 5);
        assert_eq!(course.total, 8);
        assert_eq!(
            remaining(&course),
            [Some(5), Some(5), Some(8), Some(8), None]
        );
        assert_eq!(scope(&course), [Some(5), Some(5), Some(8), Some(8), None]);
        assert_eq!(course.changes.len(), 1);
        let change = &course.changes[0];
        assert_eq!(change.date, day(12));
        assert_eq!(change.joined.len(), 1);
        assert_eq!(change.joined[0].number, 2);
        assert_eq!(change.work_joined(), 3);
        assert_eq!(change.left, []);
        assert_eq!(change.reestimated, 0);
    }

    #[test]
    fn a_task_that_leaves_lowers_the_line_and_one_finished_lowers_only_what_remains() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let done_status = done(&workflow).clone();
        let stories = [
            Story::opened(1, &workflow, at(9, 0)).then(|task| {
                task.set_points(points(5), at(9, 1));
                task.plan_into(Some(id), at(9, 2));
            }),
            Story::opened(2, &workflow, at(9, 0))
                .then(|task| {
                    task.set_points(points(3), at(9, 1));
                    task.plan_into(Some(id), at(9, 2));
                })
                .then(|task| task.plan_into(None, at(11, 3))),
            Story::opened(3, &workflow, at(9, 0))
                .then(|task| {
                    task.set_points(points(2), at(9, 1));
                    task.plan_into(Some(id), at(9, 2));
                })
                .then(|task| task.move_to(&done_status, Rank::FIRST, at(12, 3))),
        ];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(14), &day_of);

        assert_eq!(course.committed, 10);
        assert_eq!(course.total, 7);
        assert_eq!(
            remaining(&course),
            [Some(10), Some(7), Some(5), Some(5), Some(5)]
        );
        assert_eq!(
            scope(&course),
            [Some(10), Some(7), Some(7), Some(7), Some(7)]
        );
        assert_eq!(course.changes.len(), 1);
        assert_eq!(course.changes[0].date, day(11));
        assert_eq!(course.changes[0].left[0].number, 2);
        assert_eq!(course.changes[0].work_left(), 3);
    }

    #[test]
    fn a_deleted_task_leaves_on_the_day_it_was_deleted() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let mut story = Story::opened(1, &workflow, at(9, 0)).then(|task| {
            task.set_points(points(5), at(9, 1));
            task.plan_into(Some(id), at(9, 2));
        });
        story.deleted_at = Some(at(12, 0));

        let course = course_of(&sprint, &workflow, &[story.past()], day(13), &day_of);

        assert_eq!(
            remaining(&course),
            [Some(5), Some(5), Some(0), Some(0), None]
        );
        assert_eq!(course.changes[0].date, day(12));
        assert_eq!(course.changes[0].left[0].number, 1);
        assert_eq!(course.total, 0);
    }

    #[test]
    fn new_points_on_a_task_that_stays_are_told_as_a_change() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let stories = [Story::opened(1, &workflow, at(9, 0))
            .then(|task| {
                task.set_points(points(3), at(9, 1));
                task.plan_into(Some(id), at(9, 2));
            })
            .then(|task| task.set_points(points(8), at(11, 0)))];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(11), &day_of);

        assert_eq!(remaining(&course), [Some(3), Some(8), None, None, None]);
        assert_eq!(course.changes[0].reestimated, 5);
        assert_eq!(course.changes[0].joined, []);
    }

    #[test]
    fn a_task_without_points_in_a_sprint_of_points_weighs_nothing_and_says_so() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let stories = [
            Story::opened(1, &workflow, at(9, 0)).then(|task| {
                task.set_points(points(3), at(9, 1));
                task.plan_into(Some(id), at(9, 2));
            }),
            Story::opened(2, &workflow, at(9, 0)).then(|task| task.plan_into(Some(id), at(11, 2))),
        ];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(11), &day_of);

        let joined = &course.changes[0].joined[0];
        assert_eq!(joined.work, 0);
        assert!(joined.has_no_points);
    }

    #[test]
    fn a_sprint_without_points_counts_tasks() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let stories = [
            Story::opened(1, &workflow, at(9, 0)).then(|task| task.plan_into(Some(id), at(9, 2))),
            Story::opened(2, &workflow, at(9, 0)).then(|task| task.plan_into(Some(id), at(13, 2))),
        ];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(14), &day_of);

        assert_eq!(course.unit, SprintUnit::Tasks);
        assert_eq!(
            remaining(&course),
            [Some(1), Some(1), Some(1), Some(2), Some(2)]
        );
        assert!(!course.changes[0].joined[0].has_no_points);
    }

    #[test]
    fn a_subtask_is_counted_with_its_task_and_not_on_its_own() {
        let workflow = Workflow::standard();
        let sprint = running_sprint();
        let id = sprint.id();
        let parent =
            Story::opened(1, &workflow, at(9, 0)).then(|task| task.plan_into(Some(id), at(9, 2)));
        let parent_id = parent.versions[0].id();
        let child = Story::opened(2, &workflow, at(9, 0)).then(|task| {
            task.set_parent(Some(parent_id), at(9, 1)).unwrap();
            task.plan_into(Some(id), at(9, 2));
        });

        let stories = [parent, child];
        let course = course_of(&sprint, &workflow, &pasts(&stories), day(10), &day_of);

        assert_eq!(course.total, 1);
    }

    #[test]
    fn a_completed_sprint_keeps_its_unfinished_tasks_to_the_end_and_tells_where_they_went() {
        let workflow = Workflow::standard();
        let mut sprint = running_sprint();
        let id = sprint.id();
        let next = SprintId::generate();
        let done_status = done(&workflow).clone();
        let end = at(12, 6);
        let stories = [
            Story::opened(1, &workflow, at(9, 0))
                .then(|task| {
                    task.set_points(points(5), at(9, 1));
                    task.plan_into(Some(id), at(9, 2));
                })
                .then(|task| task.move_to(&done_status, Rank::FIRST, at(11, 0))),
            Story::opened(2, &workflow, at(9, 0))
                .then(|task| {
                    task.set_points(points(3), at(9, 1));
                    task.plan_into(Some(id), at(9, 2));
                })
                .then(|task| task.plan_into(Some(next), end)),
            Story::opened(3, &workflow, at(9, 0))
                .then(|task| {
                    task.set_points(points(2), at(9, 1));
                    task.plan_into(Some(id), at(9, 2));
                })
                .then(|task| task.plan_into(None, end)),
        ];
        sprint.complete(SprintSummary::of(&[]), end).unwrap();

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(20), &day_of);

        assert_eq!(course.total, 10);
        assert_eq!(remaining(&course), [Some(10), Some(5), Some(5), None, None]);
        assert_eq!(course.changes, []);
        assert_eq!(
            course
                .moved_on
                .iter()
                .map(|moved| (moved.task.number, moved.to))
                .collect::<Vec<_>>(),
            [(2, Some(next)), (3, None)]
        );
    }

    #[test]
    fn tasks_planned_before_a_late_start_count_from_the_first_day() {
        let workflow = Workflow::standard();
        let mut sprint = Sprint::plan(
            ProjectId::generate(),
            SprintName::numbered(1),
            "",
            SprintDates::default(),
            at(9, 0),
        );
        sprint
            .start("Sprint 1", "", day(10), day(14), at(12, 0))
            .unwrap();
        let id = sprint.id();
        let stories =
            [Story::opened(1, &workflow, at(9, 0))
                .then(|task| task.plan_into(Some(id), at(11, 5)))];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(12), &day_of);

        assert_eq!(remaining(&course), [Some(1), Some(1), Some(1), None, None]);
        assert_eq!(course.changes, []);
    }

    #[test]
    fn a_planned_sprint_has_no_day_that_has_come_and_counts_what_it_holds() {
        let workflow = Workflow::standard();
        let sprint = Sprint::plan(
            ProjectId::generate(),
            SprintName::numbered(1),
            "",
            SprintDates::new(Some(day(10)), Some(day(11))).unwrap(),
            at(9, 0),
        );
        let id = sprint.id();
        let stories = [Story::opened(1, &workflow, at(9, 0)).then(|task| {
            task.set_points(points(8), at(9, 1));
            task.plan_into(Some(id), at(9, 2));
        })];

        let course = course_of(&sprint, &workflow, &pasts(&stories), day(12), &day_of);

        assert_eq!(course.committed, 8);
        assert_eq!(course.total, 8);
        assert_eq!(remaining(&course), [None, None]);
        assert_eq!(course.moved_on, []);
    }
}
