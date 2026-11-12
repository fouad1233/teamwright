//! Sprints and story points.
//!
//! A sprint is a stretch of time in which the team works on a chosen part
//! of the backlog ([decision 0016](../../../docs/decisions/0016-sprints-and-a-backlog.md)).
//! The rules here concern one sprint, or one number. The rules that look at
//! several stored things, such as "one active sprint in a project", are
//! use cases.

use crate::error::DomainError;
use crate::ids::{ProjectId, SprintId};
use crate::macros::text_enum;
use crate::text::SprintName;
use crate::time::{CalendarDate, Timestamp};

/// How big a task is compared with other work: a whole number from 0 to
/// 999. It is a second measure next to the estimate in hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoryPoints(u16);

impl StoryPoints {
    /// The most points a task can have.
    pub const MAX: u16 = 999;

    /// Checks a number of points.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PointsOutOfRange`] if the number is below 0 or
    /// above [`Self::MAX`].
    pub fn new(value: i64) -> Result<Self, DomainError> {
        u16::try_from(value)
            .ok()
            .filter(|points| *points <= Self::MAX)
            .map(Self)
            .ok_or(DomainError::PointsOutOfRange { max: Self::MAX })
    }

    /// Checks a number of points that came as any number, such as a number
    /// in a message from the interface.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PointsOutOfRange`] if the number is not whole,
    /// or is outside the range of [`StoryPoints::new`].
    pub fn from_number(value: f64) -> Result<Self, DomainError> {
        let refused = DomainError::PointsOutOfRange { max: Self::MAX };
        if !value.is_finite() || value.fract() != 0.0 {
            return Err(refused);
        }
        if !(0.0..=f64::from(Self::MAX)).contains(&value) {
            return Err(refused);
        }
        // The number is whole and within 0 to 999, so nothing is cut off.
        Ok(Self(value as u16))
    }

    /// The number of points.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.0
    }
}

/// Where a sprint stands. A sprint is planned until it is started, active
/// while it runs, and completed for good.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SprintState {
    /// Being prepared: tasks can be planned into it.
    Planned,
    /// Running. At most one sprint of a project is active.
    Active,
    /// Over. It is kept as the record of what was done.
    Completed,
}

text_enum!(SprintState, kind = "sprint state", {
    Planned => "planned",
    Active => "active",
    Completed => "completed",
});

/// What the work of a sprint is counted in: story points if any of its
/// tasks has points, and tasks otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SprintUnit {
    /// Story points. A task without points counts as none.
    Points,
    /// Tasks. Every task counts as one.
    Tasks,
}

text_enum!(SprintUnit, kind = "sprint unit", {
    Points => "points",
    Tasks => "tasks",
});

/// A task of a sprint as its work is counted: what it weighs, when it was
/// finished, and whether it is finished now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountedTask {
    /// The story points of the task, if it has any.
    pub points: Option<StoryPoints>,
    /// When the task was finished, if it was.
    pub completed_at: Option<Timestamp>,
    /// Whether the task is in a status of the category done.
    pub is_finished: bool,
}

impl SprintUnit {
    /// The unit the work of some tasks is counted in: points if any of them
    /// has points.
    #[must_use]
    pub fn of(tasks: &[CountedTask]) -> Self {
        if tasks.iter().any(|task| task.points.is_some()) {
            Self::Points
        } else {
            Self::Tasks
        }
    }

    /// The work of one task in this unit.
    #[must_use]
    pub fn work_of(self, task: &CountedTask) -> u32 {
        match self {
            Self::Points => task.points.map_or(0, |points| u32::from(points.value())),
            Self::Tasks => 1,
        }
    }

    /// The work of some tasks together in this unit.
    #[must_use]
    pub fn work_of_all<'a>(self, tasks: impl IntoIterator<Item = &'a CountedTask>) -> u32 {
        tasks
            .into_iter()
            .map(|task| self.work_of(task))
            .fold(0, u32::saturating_add)
    }
}

/// What a completed sprint took on and what it finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SprintSummary {
    /// What the work is counted in.
    pub unit: SprintUnit,
    /// The work of every task that was in the sprint when it was completed.
    pub committed: u32,
    /// The work of the tasks among them that were finished.
    pub completed: u32,
}

impl SprintSummary {
    /// Sums up the tasks that are in a sprint when it is completed. Subtasks
    /// are not passed in: they are part of their task.
    #[must_use]
    pub fn of(tasks: &[CountedTask]) -> Self {
        let unit = SprintUnit::of(tasks);
        Self {
            unit,
            committed: unit.work_of_all(tasks),
            completed: unit.work_of_all(tasks.iter().filter(|task| task.is_finished)),
        }
    }
}

/// The first and the last day of a sprint. Either may be missing while the
/// sprint is planned. If both are given, the start is not after the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SprintDates {
    start_on: Option<CalendarDate>,
    end_on: Option<CalendarDate>,
}

impl SprintDates {
    /// Checks the dates of a sprint.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::StartAfterEnd`] if the start date is after the
    /// end date.
    pub fn new(
        start_on: Option<CalendarDate>,
        end_on: Option<CalendarDate>,
    ) -> Result<Self, DomainError> {
        if let (Some(start), Some(end)) = (start_on, end_on)
            && start > end
        {
            return Err(DomainError::StartAfterEnd { start, end });
        }
        Ok(Self { start_on, end_on })
    }

    /// The first day of the sprint.
    #[must_use]
    pub const fn start_on(&self) -> Option<CalendarDate> {
        self.start_on
    }

    /// The last day of the sprint.
    #[must_use]
    pub const fn end_on(&self) -> Option<CalendarDate> {
        self.end_on
    }

    /// Whether both dates are given.
    #[must_use]
    pub const fn are_complete(&self) -> bool {
        self.start_on.is_some() && self.end_on.is_some()
    }
}

/// The plain data of a sprint, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintSnapshot {
    /// The identifier of the sprint.
    pub id: SprintId,
    /// The project the sprint belongs to.
    pub project_id: ProjectId,
    /// Its name.
    pub name: SprintName,
    /// What it is for. May be empty.
    pub goal: String,
    /// Its first and last day.
    pub dates: SprintDates,
    /// Where it stands.
    pub state: SprintState,
    /// When it was planned.
    pub created_at: Timestamp,
    /// When it was started, if it was.
    pub started_at: Option<Timestamp>,
    /// When it was completed, if it was.
    pub completed_at: Option<Timestamp>,
    /// What it achieved, once it is completed.
    pub summary: Option<SprintSummary>,
}

/// A stretch of time in which the team works on a chosen part of the
/// backlog. A task is in at most one sprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprint {
    id: SprintId,
    project_id: ProjectId,
    name: SprintName,
    goal: String,
    dates: SprintDates,
    state: SprintState,
    created_at: Timestamp,
    started_at: Option<Timestamp>,
    completed_at: Option<Timestamp>,
    summary: Option<SprintSummary>,
}

impl Sprint {
    /// Plans a new sprint in a project. The goal is trimmed.
    #[must_use]
    pub fn plan(
        project_id: ProjectId,
        name: SprintName,
        goal: &str,
        dates: SprintDates,
        now: Timestamp,
    ) -> Self {
        Self {
            id: SprintId::generate(),
            project_id,
            name,
            goal: goal.trim().to_owned(),
            dates,
            state: SprintState::Planned,
            created_at: now,
            started_at: None,
            completed_at: None,
            summary: None,
        }
    }

    /// Recreates a sprint from stored data.
    #[must_use]
    pub fn restore(snapshot: SprintSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            goal: snapshot.goal,
            dates: snapshot.dates,
            state: snapshot.state,
            created_at: snapshot.created_at,
            started_at: snapshot.started_at,
            completed_at: snapshot.completed_at,
            summary: snapshot.summary,
        }
    }

    /// The identifier of the sprint.
    #[must_use]
    pub const fn id(&self) -> SprintId {
        self.id
    }

    /// The project the sprint belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name of the sprint.
    #[must_use]
    pub const fn name(&self) -> &SprintName {
        &self.name
    }

    /// What the sprint is for, in a sentence. May be empty.
    #[must_use]
    pub fn goal(&self) -> &str {
        &self.goal
    }

    /// The first and the last day of the sprint.
    #[must_use]
    pub const fn dates(&self) -> SprintDates {
        self.dates
    }

    /// Where the sprint stands.
    #[must_use]
    pub const fn state(&self) -> SprintState {
        self.state
    }

    /// When the sprint was planned.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// When the sprint was started. Nothing while it is planned.
    #[must_use]
    pub const fn started_at(&self) -> Option<Timestamp> {
        self.started_at
    }

    /// When the sprint was completed. Nothing before.
    #[must_use]
    pub const fn completed_at(&self) -> Option<Timestamp> {
        self.completed_at
    }

    /// What the sprint achieved. Nothing until it is completed.
    #[must_use]
    pub const fn summary(&self) -> Option<SprintSummary> {
        self.summary
    }

    /// Whether the sprint is running.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state == SprintState::Active
    }

    /// Takes an active sprint back to planned, as if it had not started.
    /// Only keeping two computers in step does this: when both started a
    /// sprint of one project at the same time, the one started later goes
    /// back. A sprint that is not active is left as it is.
    pub fn return_to_planned(&mut self) {
        if self.is_active() {
            self.state = SprintState::Planned;
            self.started_at = None;
        }
    }

    /// Replaces the name, the goal and the dates of a planned or an active
    /// sprint. The name and the goal are tidied.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint is completed, if the name is empty or
    /// too long, if the start is after the end, or if a sprint that has
    /// started would lose one of its dates. Nothing changes then.
    pub fn change(
        &mut self,
        name: &str,
        goal: &str,
        start_on: Option<CalendarDate>,
        end_on: Option<CalendarDate>,
    ) -> Result<(), DomainError> {
        self.refuse_if_completed("can no longer be changed")?;
        let name = SprintName::parse(name)?;
        let dates = SprintDates::new(start_on, end_on)?;
        if self.is_active() && !dates.are_complete() {
            return Err(DomainError::StartedSprintWithoutDates);
        }
        self.name = name;
        self.goal = goal.trim().to_owned();
        self.dates = dates;
        Ok(())
    }

    /// Makes sure that the sprint can be removed: only a planned sprint
    /// can.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint is completed or active.
    pub fn check_can_be_deleted(&self) -> Result<(), DomainError> {
        self.refuse_if_completed("is kept as a record")?;
        if self.is_active() {
            return Err(DomainError::ActiveSprintDeleted {
                name: self.name.to_string(),
            });
        }
        Ok(())
    }

    /// Makes sure that tasks can be planned into the sprint: into a planned
    /// or an active one, never into a completed one.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint is completed.
    pub fn check_takes_tasks(&self) -> Result<(), DomainError> {
        self.refuse_if_completed("takes no more tasks")
    }

    /// Makes sure that the unfinished tasks of another sprint that is
    /// completed can go to this one.
    ///
    /// # Errors
    ///
    /// Returns an error if this sprint is completed.
    pub fn check_takes_unfinished_tasks(&self) -> Result<(), DomainError> {
        self.refuse_if_completed("cannot take the unfinished tasks")
    }

    /// Makes sure that the sprint can be started: it must be planned.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint is completed or has already started.
    pub fn check_can_start(&self) -> Result<(), DomainError> {
        self.refuse_if_completed("cannot be started again")?;
        if self.is_active() {
            return Err(DomainError::SprintAlreadyStarted {
                name: self.name.to_string(),
            });
        }
        Ok(())
    }

    /// Starts a planned sprint with its name and goal as they are to be, and
    /// its dates.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint cannot be started, if the name is
    /// empty or too long, or if the start is after the end. Nothing changes
    /// then.
    pub fn start(
        &mut self,
        name: &str,
        goal: &str,
        start_on: CalendarDate,
        end_on: CalendarDate,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        self.check_can_start()?;
        let name = SprintName::parse(name)?;
        let dates = SprintDates::new(Some(start_on), Some(end_on))?;
        self.name = name;
        self.goal = goal.trim().to_owned();
        self.dates = dates;
        self.state = SprintState::Active;
        self.started_at = Some(now);
        Ok(())
    }

    /// Makes sure that the sprint can be completed: it must be active.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint is completed or has not started.
    pub fn check_can_complete(&self) -> Result<(), DomainError> {
        self.refuse_if_completed("cannot be completed again")?;
        if !self.is_active() {
            return Err(DomainError::SprintNotStarted {
                name: self.name.to_string(),
            });
        }
        Ok(())
    }

    /// Completes the active sprint, and keeps what it achieved.
    ///
    /// # Errors
    ///
    /// Returns an error if the sprint cannot be completed. Nothing changes
    /// then.
    pub fn complete(&mut self, summary: SprintSummary, now: Timestamp) -> Result<(), DomainError> {
        self.check_can_complete()?;
        self.state = SprintState::Completed;
        self.completed_at = Some(now);
        self.summary = Some(summary);
        Ok(())
    }

    /// Refuses to touch a completed sprint: it is the record of what was
    /// done. `refused` says what cannot be done to it.
    fn refuse_if_completed(&self, refused: &'static str) -> Result<(), DomainError> {
        if self.state == SprintState::Completed {
            return Err(DomainError::SprintCompleted {
                name: self.name.to_string(),
                refused,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn moment(hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 11, hour, 0, 0)
            .unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    fn planned() -> Sprint {
        Sprint::plan(
            ProjectId::generate(),
            SprintName::numbered(1),
            "",
            SprintDates::default(),
            moment(9),
        )
    }

    fn active() -> Sprint {
        let mut sprint = planned();
        sprint
            .start("Sprint 1", "", day(10), day(23), moment(10))
            .unwrap();
        sprint
    }

    fn completed() -> Sprint {
        let mut sprint = active();
        sprint.complete(SprintSummary::of(&[]), moment(11)).unwrap();
        sprint
    }

    #[test]
    fn an_active_sprint_can_go_back_to_planned_and_a_completed_one_cannot() {
        let mut sprint = active();
        sprint.return_to_planned();
        assert_eq!(sprint.state(), SprintState::Planned);
        assert_eq!(sprint.started_at(), None);
        assert_eq!(sprint.dates(), active().dates());

        let mut done = completed();
        let before = done.clone();
        done.return_to_planned();
        assert_eq!(done, before);
    }

    fn counted(points: Option<u16>, is_finished: bool) -> CountedTask {
        CountedTask {
            points: points.map(|points| StoryPoints::new(i64::from(points)).unwrap()),
            completed_at: is_finished.then(|| moment(10)),
            is_finished,
        }
    }

    #[test]
    fn an_empty_name_becomes_the_number_of_the_sprint() {
        assert_eq!(
            SprintName::parse_or_numbered("   ", 2).unwrap().as_str(),
            "Sprint 2"
        );
        assert_eq!(
            SprintName::parse_or_numbered("  The   first board  ", 2)
                .unwrap()
                .as_str(),
            "The first board"
        );
    }

    #[test]
    fn a_sprint_name_has_at_most_60_characters() {
        let error = SprintName::parse(&"x".repeat(61)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "the sprint name can have at most 60 characters, but this one has 61"
        );
        assert!(SprintName::parse(&"x".repeat(60)).is_ok());
    }

    #[test]
    fn a_numbered_name_follows_the_rules_of_a_name() {
        let numbered = SprintName::numbered(u32::MAX);

        assert_eq!(SprintName::parse(numbered.as_str()).unwrap(), numbered);
    }

    #[test]
    fn a_sprint_cannot_start_after_it_ends() {
        let error = SprintDates::new(Some(day(20)), Some(day(19))).unwrap_err();

        assert_eq!(
            error.to_string(),
            "the start date 2026-11-20 is after the end date 2026-11-19"
        );
        assert!(SprintDates::new(Some(day(20)), Some(day(20))).is_ok());
        assert!(SprintDates::new(Some(day(20)), None).is_ok());
    }

    #[test]
    fn a_new_sprint_is_planned_with_a_trimmed_goal() {
        let sprint = Sprint::plan(
            ProjectId::generate(),
            SprintName::numbered(1),
            " A schematic that can be routed ",
            SprintDates::default(),
            moment(9),
        );

        assert_eq!(sprint.state(), SprintState::Planned);
        assert_eq!(sprint.goal(), "A schematic that can be routed");
        assert_eq!(sprint.created_at(), moment(9));
        assert_eq!(sprint.started_at(), None);
        assert_eq!(sprint.summary(), None);
    }

    #[test]
    fn story_points_are_a_whole_number_from_0_to_999() {
        assert_eq!(StoryPoints::new(0).unwrap().value(), 0);
        assert_eq!(StoryPoints::new(999).unwrap().value(), 999);
        assert_eq!(StoryPoints::from_number(8.0).unwrap().value(), 8);
        for refused in [-1.0, 1000.0, 2.5, f64::NAN, f64::INFINITY] {
            let error = StoryPoints::from_number(refused).unwrap_err();
            assert_eq!(
                error.to_string(),
                "story points are a whole number from 0 to 999"
            );
        }
        assert!(StoryPoints::new(-1).is_err());
        assert!(StoryPoints::new(1000).is_err());
    }

    #[test]
    fn starting_a_sprint_takes_its_name_goal_and_dates() {
        let mut sprint = planned();

        sprint
            .start(" Schematic ", " Routable ", day(11), day(24), moment(10))
            .unwrap();

        assert_eq!(sprint.name().as_str(), "Schematic");
        assert_eq!(sprint.goal(), "Routable");
        assert_eq!(sprint.dates().start_on(), Some(day(11)));
        assert_eq!(sprint.dates().end_on(), Some(day(24)));
        assert_eq!(sprint.state(), SprintState::Active);
        assert_eq!(sprint.started_at(), Some(moment(10)));
    }

    #[test]
    fn a_sprint_that_fails_to_start_stays_as_it_was() {
        let mut sprint = planned();

        let error = sprint
            .start("Sprint 1", "", day(24), day(11), moment(10))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(sprint, planned_like(&sprint));
    }

    /// The sprint as `planned` makes it, with the identifier of `sprint`.
    fn planned_like(sprint: &Sprint) -> Sprint {
        Sprint {
            id: sprint.id(),
            project_id: sprint.project_id(),
            ..planned()
        }
    }

    #[test]
    fn an_active_sprint_keeps_both_its_dates() {
        let mut sprint = active();

        let error = sprint.change("Sprint 1", "", None, None).unwrap_err();

        assert_eq!(error, DomainError::StartedSprintWithoutDates);
        assert_eq!(
            error.to_string(),
            "a sprint that has started has a start and an end date"
        );
    }

    #[test]
    fn a_planned_sprint_may_lose_its_dates() {
        let mut sprint = planned();
        sprint
            .change("Sprint 1", "", Some(day(16)), Some(day(20)))
            .unwrap();

        sprint
            .change(" Bring-up ", " Power on ", None, None)
            .unwrap();

        assert_eq!(sprint.name().as_str(), "Bring-up");
        assert_eq!(sprint.goal(), "Power on");
        assert_eq!(sprint.dates(), SprintDates::default());
    }

    #[test]
    fn a_completed_sprint_refuses_everything_with_its_own_code() {
        let mut sprint = completed();

        let errors = [
            sprint.change("Again", "", None, None).unwrap_err(),
            sprint.check_can_be_deleted().unwrap_err(),
            sprint.check_takes_tasks().unwrap_err(),
            sprint.check_takes_unfinished_tasks().unwrap_err(),
            sprint.check_can_start().unwrap_err(),
            sprint.check_can_complete().unwrap_err(),
        ];

        assert!(
            errors
                .iter()
                .all(|error| error.code() == "sprint_completed")
        );
        assert_eq!(
            errors.map(|error| error.to_string()),
            [
                "\"Sprint 1\" is completed and can no longer be changed",
                "\"Sprint 1\" is completed and is kept as a record",
                "\"Sprint 1\" is completed and takes no more tasks",
                "\"Sprint 1\" is completed and cannot take the unfinished tasks",
                "\"Sprint 1\" is completed and cannot be started again",
                "\"Sprint 1\" is completed and cannot be completed again",
            ]
        );
    }

    #[test]
    fn an_active_sprint_is_neither_deleted_nor_started_again() {
        let sprint = active();

        let deleted = sprint.check_can_be_deleted().unwrap_err();
        let started = sprint.check_can_start().unwrap_err();

        assert_eq!(deleted.code(), "sprint_active");
        assert_eq!(
            deleted.to_string(),
            "\"Sprint 1\" has started and cannot be deleted; complete it instead"
        );
        assert_eq!(started.code(), "sprint_active");
        assert_eq!(started.to_string(), "\"Sprint 1\" has already started");
    }

    #[test]
    fn a_planned_sprint_cannot_be_completed() {
        let mut sprint = planned();

        let error = sprint
            .complete(SprintSummary::of(&[]), moment(11))
            .unwrap_err();

        assert_eq!(error.code(), "sprint_not_active");
        assert_eq!(
            error.to_string(),
            "\"Sprint 1\" has not started, so it cannot be completed"
        );
    }

    #[test]
    fn completing_a_sprint_keeps_the_moment_and_the_summary() {
        let mut sprint = active();
        let summary = SprintSummary::of(&[counted(None, true), counted(None, false)]);

        sprint.complete(summary, moment(11)).unwrap();

        assert_eq!(sprint.state(), SprintState::Completed);
        assert_eq!(sprint.completed_at(), Some(moment(11)));
        assert_eq!(sprint.summary(), Some(summary));
    }

    #[test]
    fn a_summary_counts_points_when_any_task_has_points() {
        let summary = SprintSummary::of(&[
            counted(Some(5), false),
            counted(Some(3), true),
            counted(None, false),
        ]);

        assert_eq!(
            summary,
            SprintSummary {
                unit: SprintUnit::Points,
                committed: 8,
                completed: 3
            }
        );
    }

    #[test]
    fn a_summary_counts_tasks_when_no_task_has_points() {
        let summary = SprintSummary::of(&[counted(None, false), counted(None, true)]);

        assert_eq!(
            summary,
            SprintSummary {
                unit: SprintUnit::Tasks,
                committed: 2,
                completed: 1
            }
        );
    }
}
