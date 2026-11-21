//! Work-in-progress limits: how many tasks should stand in a status at once,
//! and how full each status is.
//!
//! A limit is a signal, not a wall
//! ([decision 0033](../../../docs/decisions/0033-work-in-progress-limits-and-templates.md)).
//! Nothing here refuses a task a status because the status is full: a
//! specialist that finishes its work must always be able to hand it on.
//! What is here says how full a status is, so that the board can show it.

use crate::error::DomainError;
use crate::ids::StatusId;
use crate::task::Task;
use crate::workflow::{StatusCategory, Workflow};

/// The largest number of tasks that should stand in a status at once: a
/// whole number from 1 to 999.
///
/// Nothing is a limit of 0: a status in which no task should ever stand has
/// no use on a board. A status without a limit has none at all, which is
/// written as `Option::None` wherever a limit may be missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkInProgressLimit(u16);

impl WorkInProgressLimit {
    /// The smallest limit.
    pub const MIN: u16 = 1;
    /// The largest limit.
    pub const MAX: u16 = 999;

    /// Checks a limit.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::LimitOutOfRange`] if the number is below
    /// [`Self::MIN`] or above [`Self::MAX`].
    pub fn new(value: i64) -> Result<Self, DomainError> {
        u16::try_from(value)
            .ok()
            .filter(|limit| (Self::MIN..=Self::MAX).contains(limit))
            .map(Self)
            .ok_or(DomainError::LimitOutOfRange {
                min: Self::MIN,
                max: Self::MAX,
            })
    }

    /// A limit written in the program itself, such as one of a template.
    /// The number is known to be in range; a test of every template proves
    /// it. Out of range, it is brought to the nearest end of the range.
    #[must_use]
    pub(crate) const fn from_trusted(value: u16) -> Self {
        if value < Self::MIN {
            Self(Self::MIN)
        } else if value > Self::MAX {
            Self(Self::MAX)
        } else {
            Self(value)
        }
    }

    /// The number of tasks.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Tells whether a status of a category may carry a limit. A status of
    /// the category done may not: finished work only ever piles up there,
    /// and a limit on it would always end up exceeded and say nothing.
    #[must_use]
    pub const fn fits(category: StatusCategory) -> bool {
        !matches!(category, StatusCategory::Done)
    }
}

/// How full a status with a limit is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fill {
    /// Fewer tasks than the limit: there is room for more.
    Under,
    /// As many tasks as the limit: the status is full.
    At,
    /// More tasks than the limit.
    Over,
}

impl Fill {
    /// The fill of a status that holds `count` tasks under a limit.
    #[must_use]
    pub fn of(count: usize, limit: WorkInProgressLimit) -> Self {
        match count.cmp(&usize::from(limit.value())) {
            std::cmp::Ordering::Less => Self::Under,
            std::cmp::Ordering::Equal => Self::At,
            std::cmp::Ordering::Greater => Self::Over,
        }
    }

    /// The fill as the word the program uses for it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Under => "under",
            Self::At => "at",
            Self::Over => "over",
        }
    }
}

/// How many tasks stand in one status, against its limit if it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusLoad {
    status_id: StatusId,
    count: usize,
    limit: Option<WorkInProgressLimit>,
}

impl StatusLoad {
    /// The status that is counted.
    #[must_use]
    pub const fn status_id(&self) -> StatusId {
        self.status_id
    }

    /// How many tasks stand in it.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Its limit, if it has one.
    #[must_use]
    pub const fn limit(&self) -> Option<WorkInProgressLimit> {
        self.limit
    }

    /// How full it is, if it has a limit.
    #[must_use]
    pub fn fill(&self) -> Option<Fill> {
        self.limit.map(|limit| Fill::of(self.count, limit))
    }

    /// The loads of every status of a workflow, in board order.
    ///
    /// Every task of the project is counted, wherever it is shown: a limit
    /// is about the work the whole team has under way, not about what one
    /// view or one area happens to show. A subtask is not counted: it is a
    /// part of the work of its task, which the board shows as one card, and
    /// counting both would count the same work twice. A task in a status the
    /// workflow does not have is counted nowhere.
    #[must_use]
    pub fn of_workflow<'tasks>(
        workflow: &Workflow,
        tasks: impl IntoIterator<Item = &'tasks Task>,
    ) -> Vec<Self> {
        let mut loads: Vec<Self> = workflow
            .statuses()
            .iter()
            .map(|status| Self {
                status_id: status.id(),
                count: 0,
                limit: status.limit(),
            })
            .collect();
        for task in tasks {
            if task.parent_id().is_some() {
                continue;
            }
            if let Some(load) = loads
                .iter_mut()
                .find(|load| load.status_id == task.status_id())
            {
                load.count += 1;
            }
        }
        loads
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::color::Color;
    use crate::ids::{AreaId, ProjectId};
    use crate::task::{Rank, TaskDraft};
    use crate::text::{StatusName, TaskTitle};
    use crate::time::Timestamp;
    use crate::workflow::Status;

    fn moment() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap()
    }

    fn limit(value: i64) -> WorkInProgressLimit {
        WorkInProgressLimit::new(value).unwrap()
    }

    fn status(name: &str, category: StatusCategory, limit: Option<i64>) -> Status {
        Status::new(StatusName::parse(name).unwrap(), category, Color::Gray)
            .with_limit(limit.map(self::limit))
            .unwrap()
    }

    fn workflow() -> Workflow {
        Workflow::new(vec![
            status("Ready", StatusCategory::Todo, None),
            status("Doing", StatusCategory::Active, Some(2)),
            status("Checking", StatusCategory::Review, Some(1)),
            status("Shipped", StatusCategory::Done, None),
        ])
        .unwrap()
    }

    fn task_in(workflow: &Workflow, name: &str, number: u32, parent: Option<&Task>) -> Task {
        let status = workflow
            .statuses()
            .iter()
            .find(|status| status.name().as_str() == name)
            .unwrap();
        let mut draft = TaskDraft::titled(TaskTitle::parse(&format!("Task {number}")).unwrap());
        draft.parent_id = parent.map(Task::id);
        Task::create(
            ProjectId::generate(),
            AreaId::generate(),
            number,
            status,
            Rank::FIRST,
            draft,
            moment(),
        )
    }

    #[test]
    fn a_limit_is_a_whole_number_from_one_to_nine_hundred_ninety_nine() {
        assert_eq!(limit(1).value(), 1);
        assert_eq!(limit(999).value(), 999);
        for refused in [0, -1, 1000, i64::MAX] {
            assert_eq!(
                WorkInProgressLimit::new(refused),
                Err(DomainError::LimitOutOfRange { min: 1, max: 999 }),
                "{refused} should not be a limit"
            );
        }
    }

    #[test]
    fn only_a_status_that_is_not_done_may_carry_a_limit() {
        assert!(WorkInProgressLimit::fits(StatusCategory::Todo));
        assert!(WorkInProgressLimit::fits(StatusCategory::Active));
        assert!(WorkInProgressLimit::fits(StatusCategory::Review));
        assert!(!WorkInProgressLimit::fits(StatusCategory::Done));
    }

    #[test]
    fn the_fill_compares_the_count_with_the_limit() {
        assert_eq!(Fill::of(1, limit(2)), Fill::Under);
        assert_eq!(Fill::of(2, limit(2)), Fill::At);
        assert_eq!(Fill::of(3, limit(2)), Fill::Over);
        assert_eq!(Fill::Over.as_str(), "over");
    }

    #[test]
    fn every_status_is_counted_in_board_order_with_its_limit() {
        let workflow = workflow();
        let tasks = [
            task_in(&workflow, "Doing", 1, None),
            task_in(&workflow, "Doing", 2, None),
            task_in(&workflow, "Doing", 3, None),
            task_in(&workflow, "Checking", 4, None),
        ];

        let loads = StatusLoad::of_workflow(&workflow, &tasks);

        let seen: Vec<(usize, Option<u16>, Option<Fill>)> = loads
            .iter()
            .map(|load| {
                (
                    load.count(),
                    load.limit().map(WorkInProgressLimit::value),
                    load.fill(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                (0, None, None),
                (3, Some(2), Some(Fill::Over)),
                (1, Some(1), Some(Fill::At)),
                (0, None, None),
            ]
        );
        assert_eq!(loads[1].status_id(), workflow.statuses()[1].id());
    }

    #[test]
    fn a_subtask_is_not_counted_apart_from_its_task() {
        let workflow = workflow();
        let parent = task_in(&workflow, "Doing", 1, None);
        let tasks = [
            task_in(&workflow, "Doing", 2, Some(&parent)),
            task_in(&workflow, "Doing", 3, Some(&parent)),
            parent,
        ];

        let loads = StatusLoad::of_workflow(&workflow, &tasks);

        assert_eq!(loads[1].count(), 1);
        assert_eq!(loads[1].fill(), Some(Fill::Under));
    }

    #[test]
    fn a_task_in_a_status_the_workflow_lacks_is_counted_nowhere() {
        let workflow = workflow();
        let other = Workflow::standard();
        let stray = task_in(&other, "To do", 1, None);

        let loads = StatusLoad::of_workflow(&workflow, [&stray]);

        assert!(loads.iter().all(|load| load.count() == 0));
    }
}
