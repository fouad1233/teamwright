//! Tasks.

use std::collections::BTreeSet;

use crate::error::DomainError;
use crate::ids::{MemberId, ProjectId, StatusId, TaskId};
use crate::macros::text_enum;
use crate::text::TaskTitle;
use crate::time::{CalendarDate, Timestamp};
use crate::workflow::{Status, StatusCategory};

/// How urgent a task is. Priorities are ordered: `Low < Normal < High < Urgent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Priority {
    /// Can wait.
    Low,
    /// The usual priority of a task.
    #[default]
    Normal,
    /// Should be done before normal work.
    High,
    /// Needs attention now.
    Urgent,
}

text_enum!(Priority, kind = "priority", {
    Low => "low",
    Normal => "normal",
    High => "high",
    Urgent => "urgent",
});

/// Who is responsible for a task.
///
/// A task can be given to anyone on the project: the manager, an AI agent
/// such as a hardware designer, or a human teammate. Agents and human
/// teammates are both team members, so a task is assigned to either in the
/// same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Assignee {
    /// The manager who owns the project.
    Manager,
    /// A member of the project's team: an AI agent or a human teammate.
    Member(MemberId),
}

/// The position of a task among the tasks of the same status.
///
/// Ranks let a manager order cards by hand. Moving a card changes only that
/// card's rank: it receives a value between the ranks of its new neighbours,
/// and no other task is touched.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rank(f64);

impl Rank {
    /// The distance left between a new rank and the end of the list it is
    /// added to. A generous step leaves room for many later insertions in
    /// between.
    const STEP: f64 = 1024.0;

    /// The rank of the only task in a list.
    pub const FIRST: Self = Self(0.0);

    /// Recreates a rank from its stored value. Values that are not ordinary
    /// numbers are replaced by [`Rank::FIRST`].
    #[must_use]
    pub const fn from_value(value: f64) -> Self {
        if value.is_finite() {
            Self(value)
        } else {
            Self::FIRST
        }
    }

    /// The value to store.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }

    /// A rank that sorts after `before` and ahead of `after`.
    ///
    /// Either neighbour can be missing: a task placed at the top of a list
    /// has nothing before it, one placed at the bottom has nothing after it,
    /// and the first task of an empty list has neither.
    #[must_use]
    pub fn between(before: Option<Self>, after: Option<Self>) -> Self {
        match (before, after) {
            (None, None) => Self::FIRST,
            (Some(before), None) => Self(before.0 + Self::STEP),
            (None, Some(after)) => Self(after.0 - Self::STEP),
            (Some(before), Some(after)) => Self(before.0.midpoint(after.0)),
        }
    }
}

/// When a task is planned to start and when it is due.
///
/// Both dates are optional. If both are given, the start is not after the
/// due date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Schedule {
    start_on: Option<CalendarDate>,
    due_on: Option<CalendarDate>,
}

impl Schedule {
    /// Creates a schedule.
    ///
    /// # Errors
    ///
    /// Returns an error if the start date is after the due date.
    pub fn new(
        start_on: Option<CalendarDate>,
        due_on: Option<CalendarDate>,
    ) -> Result<Self, DomainError> {
        if let (Some(start), Some(due)) = (start_on, due_on)
            && start > due
        {
            return Err(DomainError::StartAfterDue { start, due });
        }
        Ok(Self { start_on, due_on })
    }

    /// A schedule with no dates.
    #[must_use]
    pub const fn unscheduled() -> Self {
        Self {
            start_on: None,
            due_on: None,
        }
    }

    /// The day work is planned to start.
    #[must_use]
    pub const fn start_on(&self) -> Option<CalendarDate> {
        self.start_on
    }

    /// The day the task is due.
    #[must_use]
    pub const fn due_on(&self) -> Option<CalendarDate> {
        self.due_on
    }
}

/// What a manager or a team member provides to open a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDraft {
    /// What the task is about.
    pub title: TaskTitle,
    /// Details, acceptance criteria, links. May be empty.
    pub description: String,
    /// How urgent the task is.
    pub priority: Priority,
    /// Who is responsible, if anyone yet.
    pub assignee: Option<Assignee>,
    /// The task this one is a part of, if it is a subtask.
    pub parent_id: Option<TaskId>,
    /// The expected working time in minutes.
    pub estimate_minutes: Option<u32>,
    /// The planned start and due dates.
    pub schedule: Schedule,
}

impl TaskDraft {
    /// A draft with a title and nothing else decided.
    #[must_use]
    pub fn titled(title: TaskTitle) -> Self {
        Self {
            title,
            description: String::new(),
            priority: Priority::default(),
            assignee: None,
            parent_id: None,
            estimate_minutes: None,
            schedule: Schedule::unscheduled(),
        }
    }
}

/// The plain data of a task, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskSnapshot {
    /// The identifier of the task.
    pub id: TaskId,
    /// The project the task belongs to.
    pub project_id: ProjectId,
    /// The number of the task within its project.
    pub number: u32,
    /// What the task is about.
    pub title: TaskTitle,
    /// Details. May be empty.
    pub description: String,
    /// The status the task is in.
    pub status_id: StatusId,
    /// How urgent the task is.
    pub priority: Priority,
    /// Who is responsible.
    pub assignee: Option<Assignee>,
    /// The task this one is a part of.
    pub parent_id: Option<TaskId>,
    /// The tasks that must be finished before this one can start.
    pub blocked_by: BTreeSet<TaskId>,
    /// The expected working time in minutes.
    pub estimate_minutes: Option<u32>,
    /// The planned start and due dates.
    pub schedule: Schedule,
    /// The position among the tasks of the same status.
    pub rank: Rank,
    /// When the task was opened.
    pub created_at: Timestamp,
    /// When the task was last changed.
    pub updated_at: Timestamp,
    /// When the task reached a "done" status, if it has.
    pub completed_at: Option<Timestamp>,
}

/// A unit of work in a project.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    id: TaskId,
    project_id: ProjectId,
    number: u32,
    title: TaskTitle,
    description: String,
    status_id: StatusId,
    priority: Priority,
    assignee: Option<Assignee>,
    parent_id: Option<TaskId>,
    blocked_by: BTreeSet<TaskId>,
    estimate_minutes: Option<u32>,
    schedule: Schedule,
    rank: Rank,
    created_at: Timestamp,
    updated_at: Timestamp,
    completed_at: Option<Timestamp>,
}

impl Task {
    /// Opens a task in a project.
    ///
    /// `number` comes from [`Project::allocate_task_number`], and `status` is
    /// one of the statuses of that project's workflow.
    ///
    /// [`Project::allocate_task_number`]: crate::Project::allocate_task_number
    #[must_use]
    pub fn create(
        project_id: ProjectId,
        number: u32,
        status: &Status,
        rank: Rank,
        draft: TaskDraft,
        now: Timestamp,
    ) -> Self {
        Self {
            id: TaskId::generate(),
            project_id,
            number,
            title: draft.title,
            description: draft.description,
            status_id: status.id(),
            priority: draft.priority,
            assignee: draft.assignee,
            parent_id: draft.parent_id,
            blocked_by: BTreeSet::new(),
            estimate_minutes: draft.estimate_minutes,
            schedule: draft.schedule,
            rank,
            created_at: now,
            updated_at: now,
            completed_at: completion_time(status, None, now),
        }
    }

    /// Recreates a task from stored data.
    #[must_use]
    pub fn restore(snapshot: TaskSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            number: snapshot.number,
            title: snapshot.title,
            description: snapshot.description,
            status_id: snapshot.status_id,
            priority: snapshot.priority,
            assignee: snapshot.assignee,
            parent_id: snapshot.parent_id,
            blocked_by: snapshot.blocked_by,
            estimate_minutes: snapshot.estimate_minutes,
            schedule: snapshot.schedule,
            rank: snapshot.rank,
            created_at: snapshot.created_at,
            updated_at: snapshot.updated_at,
            completed_at: snapshot.completed_at,
        }
    }

    /// The identifier of the task.
    #[must_use]
    pub const fn id(&self) -> TaskId {
        self.id
    }

    /// The project the task belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The number of the task within its project.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    /// What the task is about.
    #[must_use]
    pub const fn title(&self) -> &TaskTitle {
        &self.title
    }

    /// Details, acceptance criteria, links. May be empty.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The status the task is in.
    #[must_use]
    pub const fn status_id(&self) -> StatusId {
        self.status_id
    }

    /// How urgent the task is.
    #[must_use]
    pub const fn priority(&self) -> Priority {
        self.priority
    }

    /// Who is responsible for the task.
    #[must_use]
    pub const fn assignee(&self) -> Option<Assignee> {
        self.assignee
    }

    /// The task this one is a part of, if it is a subtask.
    #[must_use]
    pub const fn parent_id(&self) -> Option<TaskId> {
        self.parent_id
    }

    /// The tasks that must be finished before this one can start.
    #[must_use]
    pub const fn blocked_by(&self) -> &BTreeSet<TaskId> {
        &self.blocked_by
    }

    /// The expected working time in minutes.
    #[must_use]
    pub const fn estimate_minutes(&self) -> Option<u32> {
        self.estimate_minutes
    }

    /// The planned start and due dates.
    #[must_use]
    pub const fn schedule(&self) -> Schedule {
        self.schedule
    }

    /// The position among the tasks of the same status.
    #[must_use]
    pub const fn rank(&self) -> Rank {
        self.rank
    }

    /// When the task was opened.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// When the task was last changed.
    #[must_use]
    pub const fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// When the task reached a "done" status, if it has.
    #[must_use]
    pub const fn completed_at(&self) -> Option<Timestamp> {
        self.completed_at
    }

    /// Whether the task is in a "done" status.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    /// Gives the task a new title.
    pub fn rename(&mut self, title: TaskTitle, now: Timestamp) {
        self.title = title;
        self.touch(now);
    }

    /// Replaces the description of the task.
    pub fn describe(&mut self, description: String, now: Timestamp) {
        self.description = description;
        self.touch(now);
    }

    /// Changes how urgent the task is.
    pub fn prioritize(&mut self, priority: Priority, now: Timestamp) {
        self.priority = priority;
        self.touch(now);
    }

    /// Makes someone responsible for the task, or nobody.
    pub fn assign(&mut self, assignee: Option<Assignee>, now: Timestamp) {
        self.assignee = assignee;
        self.touch(now);
    }

    /// Sets or clears the expected working time.
    pub fn estimate(&mut self, minutes: Option<u32>, now: Timestamp) {
        self.estimate_minutes = minutes;
        self.touch(now);
    }

    /// Changes the planned start and due dates.
    pub fn reschedule(&mut self, schedule: Schedule, now: Timestamp) {
        self.schedule = schedule;
        self.touch(now);
    }

    /// Moves the task to a status and a position within it.
    ///
    /// Reaching a "done" status records the completion time. Leaving one
    /// clears it. Moving between two "done" statuses keeps the original time.
    pub fn move_to(&mut self, status: &Status, rank: Rank, now: Timestamp) {
        self.status_id = status.id();
        self.rank = rank;
        self.completed_at = completion_time(status, self.completed_at, now);
        self.touch(now);
    }

    /// Makes the task a subtask of another task, or a top-level task.
    ///
    /// # Errors
    ///
    /// Returns an error if the task is given itself as its parent.
    pub fn set_parent(
        &mut self,
        parent_id: Option<TaskId>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if parent_id == Some(self.id) {
            return Err(DomainError::SelfParent);
        }
        self.parent_id = parent_id;
        self.touch(now);
        Ok(())
    }

    /// Makes the task wait until another task is finished.
    ///
    /// Longer loops ("A waits for B, B waits for A") involve several tasks
    /// and are checked with a [`DependencyGraph`](crate::DependencyGraph).
    ///
    /// # Errors
    ///
    /// Returns an error if the task is asked to wait for itself.
    pub fn block_on(&mut self, blocker_id: TaskId, now: Timestamp) -> Result<(), DomainError> {
        if blocker_id == self.id {
            return Err(DomainError::SelfDependency);
        }
        self.blocked_by.insert(blocker_id);
        self.touch(now);
        Ok(())
    }

    /// Stops the task from waiting for another task.
    pub fn unblock_from(&mut self, blocker_id: TaskId, now: Timestamp) {
        self.blocked_by.remove(&blocker_id);
        self.touch(now);
    }

    /// Records that the task changed.
    const fn touch(&mut self, now: Timestamp) {
        self.updated_at = now;
    }
}

/// Works out the completion time of a task that is in `status` now.
///
/// A task in a "done" status keeps the completion time it already had, or
/// gets `now`. A task in any other status has none.
fn completion_time(
    status: &Status,
    previous: Option<Timestamp>,
    now: Timestamp,
) -> Option<Timestamp> {
    match status.category() {
        StatusCategory::Done => previous.or(Some(now)),
        StatusCategory::Todo | StatusCategory::Active | StatusCategory::Review => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::workflow::Workflow;

    fn moment(hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 11, hour, 0, 0)
            .unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    fn draft(title: &str) -> TaskDraft {
        TaskDraft::titled(TaskTitle::parse(title).unwrap())
    }

    fn task_in(workflow: &Workflow) -> Task {
        Task::create(
            ProjectId::generate(),
            1,
            workflow.initial_status(),
            Rank::FIRST,
            draft("Choose the microcontroller"),
            moment(9),
        )
    }

    #[test]
    fn a_new_task_takes_everything_from_its_draft() {
        let workflow = Workflow::standard();
        let project_id = ProjectId::generate();
        let mut draft = draft("Choose the microcontroller");
        draft.description = "Cheap, small, with Wi-Fi.".to_owned();
        draft.priority = Priority::High;
        draft.assignee = Some(Assignee::Manager);
        draft.estimate_minutes = Some(90);

        let task = Task::create(
            project_id,
            7,
            workflow.initial_status(),
            Rank::FIRST,
            draft,
            moment(9),
        );

        assert_eq!(task.project_id(), project_id);
        assert_eq!(task.number(), 7);
        assert_eq!(task.title().as_str(), "Choose the microcontroller");
        assert_eq!(task.description(), "Cheap, small, with Wi-Fi.");
        assert_eq!(task.priority(), Priority::High);
        assert_eq!(task.assignee(), Some(Assignee::Manager));
        assert_eq!(task.estimate_minutes(), Some(90));
        assert_eq!(task.status_id(), workflow.initial_status().id());
        assert_eq!(task.created_at(), moment(9));
        assert_eq!(task.updated_at(), moment(9));
        assert!(task.blocked_by().is_empty());
        assert!(!task.is_complete());
    }

    #[test]
    fn a_draft_with_only_a_title_has_normal_priority_and_nothing_else_decided() {
        let draft = draft("Choose the microcontroller");

        assert_eq!(draft.priority, Priority::Normal);
        assert_eq!(draft.assignee, None);
        assert_eq!(draft.schedule, Schedule::unscheduled());
    }

    #[test]
    fn priorities_are_ordered_from_low_to_urgent() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Urgent);
    }

    #[test]
    fn every_change_records_when_it_happened() {
        let mut task = task_in(&Workflow::standard());

        task.rename(
            TaskTitle::parse("Choose the Wi-Fi module").unwrap(),
            moment(10),
        );
        assert_eq!(task.updated_at(), moment(10));

        task.prioritize(Priority::Urgent, moment(11));
        assert_eq!(task.updated_at(), moment(11));

        task.assign(Some(Assignee::Member(MemberId::generate())), moment(12));
        assert_eq!(task.updated_at(), moment(12));

        assert_eq!(task.created_at(), moment(9));
    }

    #[test]
    fn reaching_a_done_status_records_the_completion_time() {
        let workflow = Workflow::standard();
        let mut task = task_in(&workflow);
        let done = workflow.first_in(StatusCategory::Done).unwrap();

        task.move_to(done, Rank::FIRST, moment(15));

        assert!(task.is_complete());
        assert_eq!(task.completed_at(), Some(moment(15)));
        assert_eq!(task.status_id(), done.id());
    }

    #[test]
    fn leaving_a_done_status_clears_the_completion_time() {
        let workflow = Workflow::standard();
        let mut task = task_in(&workflow);
        task.move_to(
            workflow.first_in(StatusCategory::Done).unwrap(),
            Rank::FIRST,
            moment(15),
        );

        task.move_to(
            workflow.first_in(StatusCategory::Active).unwrap(),
            Rank::FIRST,
            moment(16),
        );

        assert!(!task.is_complete());
    }

    #[test]
    fn reordering_within_a_done_status_keeps_the_original_completion_time() {
        let workflow = Workflow::standard();
        let mut task = task_in(&workflow);
        let done = workflow.first_in(StatusCategory::Done).unwrap();
        task.move_to(done, Rank::FIRST, moment(15));

        task.move_to(done, Rank::between(Some(Rank::FIRST), None), moment(18));

        assert_eq!(task.completed_at(), Some(moment(15)));
    }

    #[test]
    fn a_task_created_directly_in_a_done_status_is_complete() {
        let workflow = Workflow::standard();

        let task = Task::create(
            ProjectId::generate(),
            1,
            workflow.first_in(StatusCategory::Done).unwrap(),
            Rank::FIRST,
            draft("Order the samples"),
            moment(9),
        );

        assert_eq!(task.completed_at(), Some(moment(9)));
    }

    #[test]
    fn a_task_cannot_wait_for_itself() {
        let mut task = task_in(&Workflow::standard());

        let result = task.block_on(task.id(), moment(10));

        assert_eq!(result, Err(DomainError::SelfDependency));
        assert!(task.blocked_by().is_empty());
    }

    #[test]
    fn a_task_can_wait_for_other_tasks_and_stop_waiting() {
        let mut task = task_in(&Workflow::standard());
        let schematic = TaskId::generate();
        let pin_map = TaskId::generate();

        task.block_on(schematic, moment(10)).unwrap();
        task.block_on(pin_map, moment(10)).unwrap();
        task.unblock_from(schematic, moment(11));

        assert_eq!(
            task.blocked_by().iter().copied().collect::<Vec<_>>(),
            [pin_map]
        );
    }

    #[test]
    fn a_task_cannot_be_its_own_parent() {
        let mut task = task_in(&Workflow::standard());

        let result = task.set_parent(Some(task.id()), moment(10));

        assert_eq!(result, Err(DomainError::SelfParent));
        assert_eq!(task.parent_id(), None);
    }

    #[test]
    fn a_schedule_may_start_and_end_on_the_same_day() {
        assert!(Schedule::new(Some(day(12)), Some(day(12))).is_ok());
    }

    #[test]
    fn a_schedule_that_starts_after_it_is_due_is_rejected() {
        let error = Schedule::new(Some(day(20)), Some(day(12))).unwrap_err();

        assert_eq!(
            error,
            DomainError::StartAfterDue {
                start: day(20),
                due: day(12)
            }
        );
    }

    #[test]
    fn a_schedule_may_have_only_one_of_its_dates() {
        assert!(Schedule::new(Some(day(20)), None).is_ok());
        assert!(Schedule::new(None, Some(day(12))).is_ok());
    }

    #[test]
    fn the_first_rank_of_an_empty_list_is_the_starting_rank() {
        assert_eq!(Rank::between(None, None), Rank::FIRST);
    }

    #[test]
    fn a_rank_at_the_end_of_a_list_sorts_after_the_last_one() {
        let last = Rank::FIRST;

        assert!(Rank::between(Some(last), None) > last);
    }

    #[test]
    fn a_rank_at_the_start_of_a_list_sorts_before_the_first_one() {
        let first = Rank::FIRST;

        assert!(Rank::between(None, Some(first)) < first);
    }

    #[test]
    fn a_rank_between_two_neighbours_sorts_between_them() {
        let upper = Rank::FIRST;
        let lower = Rank::between(Some(upper), None);

        let middle = Rank::between(Some(upper), Some(lower));

        assert!(upper < middle && middle < lower);
    }

    #[test]
    fn a_stored_rank_that_is_not_a_number_becomes_the_starting_rank() {
        assert_eq!(Rank::from_value(f64::NAN), Rank::FIRST);
        assert_eq!(Rank::from_value(f64::INFINITY), Rank::FIRST);
    }
}
