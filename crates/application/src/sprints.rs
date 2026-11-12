//! Use cases for sprints: planning, changing, removing, starting and
//! completing them, and the report for their burndown chart.
//!
//! Planning a task into a sprint and giving it points are changes to a
//! task, and live with the other changes to a task in
//! [`TaskService`](crate::TaskService).

use std::cmp::{Ordering, Reverse};
use std::sync::Arc;

use teamwright_domain::{
    CalendarDate, CountedTask, Project, ProjectId, Sprint, SprintDates, SprintId, SprintName,
    SprintState, SprintSummary, SprintUnit, StatusCategory, Task,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, ProjectRepository, SprintRepository, TaskRepository};
use crate::tasks::subtree_of;

/// What is needed to plan a sprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSprint {
    /// The project the sprint belongs to.
    pub project_id: ProjectId,
    /// The name, before it is tidied. Left empty, the sprint is called
    /// "Sprint N".
    pub name: String,
    /// What the sprint is for. May be empty.
    pub goal: String,
    /// The first day, if it is known yet.
    pub start_on: Option<CalendarDate>,
    /// The last day, if it is known yet.
    pub end_on: Option<CalendarDate>,
}

impl NewSprint {
    /// A sprint of a project with nothing else decided: it will be called
    /// "Sprint N" and has no goal and no dates.
    #[must_use]
    pub const fn in_project(project_id: ProjectId) -> Self {
        Self {
            project_id,
            name: String::new(),
            goal: String::new(),
            start_on: None,
            end_on: None,
        }
    }
}

/// What can be changed about a sprint that is planned or active. Every
/// field is replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintChanges {
    /// The name, before it is tidied. It must not be empty.
    pub name: String,
    /// What the sprint is for. May be empty.
    pub goal: String,
    /// The first day. An active sprint must keep one.
    pub start_on: Option<CalendarDate>,
    /// The last day. An active sprint must keep one.
    pub end_on: Option<CalendarDate>,
}

/// What a sprint is started with: its name and goal as they are to be, and
/// its dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintStart {
    /// The name, before it is tidied. It must not be empty.
    pub name: String,
    /// What the sprint is for. May be empty.
    pub goal: String,
    /// The first day.
    pub start_on: CalendarDate,
    /// The last day.
    pub end_on: CalendarDate,
}

/// One day of a sprint in its report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SprintDay {
    /// The day.
    pub date: CalendarDate,
    /// The work that remained at the end of the day. Nothing for a day that
    /// has not come yet, or that came after the sprint was completed.
    pub remaining: Option<u32>,
}

/// The course of a sprint, for its burndown chart.
///
/// It counts the tasks that are in the sprint now, from the moments at which
/// they were finished. Tasks that joined or left the sprint while it ran are
/// not followed: a task added in the middle counts as if it had been there
/// from the first day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintReport {
    /// The sprint the report is about.
    pub sprint_id: SprintId,
    /// What the work is counted in.
    pub unit: SprintUnit,
    /// The work of all the tasks of the sprint.
    pub total: u32,
    /// Every day from the start date to the end date, in order. Empty
    /// without both dates.
    pub days: Vec<SprintDay>,
}

/// The use cases that plan, change, remove, start and complete sprints.
#[derive(Clone)]
pub struct SprintService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    sprints: Arc<dyn SprintRepository>,
    clock: Arc<dyn Clock>,
}

impl SprintService {
    /// Creates the service. The projects are needed for their workflows,
    /// which tell which tasks are finished, and for the number of their
    /// next sprint. The tasks are needed because they are what a sprint is
    /// made of.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        sprints: Arc<dyn SprintRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            sprints,
            clock,
        }
    }

    /// Lists the sprints of a project: the active one first, then the
    /// planned ones in the order they were planned, then the completed ones,
    /// the one completed last first.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Sprint>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let mut sprints = self.sprints.list_for_project(project.id())?;
        sprints.sort_by(as_listed);
        Ok(sprints)
    }

    /// Finds a sprint.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint or if storage fails.
    pub fn get(&self, id: SprintId) -> Result<Sprint, ApplicationError> {
        self.require_sprint(id)
    }

    /// Plans a new sprint. Its name is tidied and has at most 60
    /// characters; left empty, it is "Sprint N", N being one more than the
    /// number of sprints the project has had. The goal is trimmed.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name is too
    /// long, if the start is after the end, or if storage fails.
    pub fn create(&self, new_sprint: NewSprint) -> Result<Sprint, ApplicationError> {
        let mut project = self.require_project(new_sprint.project_id)?;
        let name = SprintName::parse_or_numbered(&new_sprint.name, project.next_sprint_number())?;
        let dates = SprintDates::new(new_sprint.start_on, new_sprint.end_on)?;
        project.allocate_sprint_number();
        let sprint = Sprint::plan(
            project.id(),
            name,
            &new_sprint.goal,
            dates,
            self.clock.now(),
        );

        // The project is stored first, as when a task is opened: if storing
        // the sprint then fails, one number is skipped, which is harmless.
        self.projects.save(&project)?;
        self.sprints.save(&sprint, &[])?;
        Ok(sprint)
    }

    /// Replaces the name, the goal and the dates of a sprint that is
    /// planned or active.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint, if it is completed, if
    /// the name is empty or too long, if the start is after the end, if an
    /// active sprint would lose a date, or if storage fails.
    pub fn change(&self, id: SprintId, changes: SprintChanges) -> Result<Sprint, ApplicationError> {
        let mut sprint = self.require_sprint(id)?;
        sprint.change(
            &changes.name,
            &changes.goal,
            changes.start_on,
            changes.end_on,
        )?;
        self.sprints.save(&sprint, &[])?;
        Ok(sprint)
    }

    /// Removes a planned sprint. Its tasks go back to the backlog.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint, if it is active or
    /// completed, or if storage fails.
    pub fn delete(&self, id: SprintId) -> Result<(), ApplicationError> {
        let sprint = self.require_sprint(id)?;
        sprint.check_can_be_deleted()?;
        let now = self.clock.now();
        let back_to_backlog: Vec<Task> = self
            .tasks_of_sprint(&sprint)?
            .into_iter()
            .map(|mut task| {
                task.plan_into(None, now);
                task
            })
            .collect();
        self.sprints.delete(sprint.id(), &back_to_backlog)?;
        Ok(())
    }

    /// Starts a planned sprint with its name, its goal and its dates. It
    /// needs at least one task, and no other sprint of the project may be
    /// active.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint, if it is not planned,
    /// if another sprint of the project is active, if it has no tasks, if
    /// the name is empty or too long, if the start is after the end, or if
    /// storage fails.
    pub fn start(&self, id: SprintId, start: SprintStart) -> Result<Sprint, ApplicationError> {
        let mut sprint = self.require_sprint(id)?;
        sprint.check_can_start()?;
        if let Some(active) = self
            .sprints
            .list_for_project(sprint.project_id())?
            .into_iter()
            .find(|other| other.is_active() && other.id() != sprint.id())
        {
            return Err(ApplicationError::AnotherSprintActive {
                name: active.name().to_string(),
            });
        }
        if self.tasks_of_sprint(&sprint)?.is_empty() {
            return Err(ApplicationError::SprintEmpty {
                name: sprint.name().to_string(),
            });
        }
        sprint.start(
            &start.name,
            &start.goal,
            start.start_on,
            start.end_on,
            self.clock.now(),
        )?;
        self.sprints.save(&sprint, &[])?;
        Ok(sprint)
    }

    /// Completes the active sprint. Its unfinished tasks, with their
    /// subtasks, go to `move_unfinished_to`, a planned or active sprint of
    /// the same project, or with `None` to the backlog. The finished ones
    /// stay in it as its record, and the sprint keeps a summary of what was
    /// committed and what was completed.
    ///
    /// A task is finished when it is in a status of the category done,
    /// whatever the status is called.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint, if it is not active, if
    /// the sprint for the unfinished tasks does not exist, belongs to
    /// another project, is this sprint or is completed, or if storage
    /// fails.
    pub fn complete(
        &self,
        id: SprintId,
        move_unfinished_to: Option<SprintId>,
    ) -> Result<Sprint, ApplicationError> {
        let mut sprint = self.require_sprint(id)?;
        sprint.check_can_complete()?;
        if let Some(target_id) = move_unfinished_to {
            let target = self.require_sprint(target_id)?;
            if target.project_id() != sprint.project_id() {
                return Err(ApplicationError::UnfinishedToAnotherProject);
            }
            if target.id() == sprint.id() {
                return Err(ApplicationError::UnfinishedStayInSprint);
            }
            target.check_takes_unfinished_tasks()?;
        }

        let project = self.require_project(sprint.project_id())?;
        let tasks_of_project = self.tasks.list_for_project(project.id())?;
        // The tasks are counted as the manager sees them: a subtask is part
        // of its task and goes where its task goes.
        let main_tasks: Vec<&Task> = tasks_of_project
            .iter()
            .filter(|task| task.sprint_id() == Some(sprint.id()) && task.parent_id().is_none())
            .collect();
        let summary = SprintSummary::of(
            &main_tasks
                .iter()
                .map(|task| counted(&project, task))
                .collect::<Vec<_>>(),
        );

        let now = self.clock.now();
        let mut moving = std::collections::BTreeSet::new();
        for task in main_tasks
            .iter()
            .filter(|task| !is_finished(&project, task))
        {
            moving.extend(subtree_of(task.id(), &tasks_of_project));
        }
        let moved: Vec<Task> = tasks_of_project
            .iter()
            .filter(|task| moving.contains(&task.id()))
            .cloned()
            .map(|mut task| {
                task.plan_into(move_unfinished_to, now);
                task
            })
            .collect();

        sprint.complete(summary, now)?;
        self.sprints.save(&sprint, &moved)?;
        Ok(sprint)
    }

    /// Tells how much work remained in a sprint at the end of each of its
    /// days.
    ///
    /// A task that was finished counts as done from the day it was
    /// finished, or from the first day if that was earlier. Days after
    /// today, and after the day the sprint was completed, have not come for
    /// the sprint: they have no value. A planned sprint has not come at
    /// all. Days are those of the clock, where the program runs.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such sprint or if storage fails.
    pub fn report(&self, id: SprintId) -> Result<SprintReport, ApplicationError> {
        let sprint = self.require_sprint(id)?;
        let project = self.require_project(sprint.project_id())?;
        let counted_tasks: Vec<CountedTask> = self
            .tasks_of_sprint(&sprint)?
            .iter()
            .filter(|task| task.parent_id().is_none())
            .map(|task| counted(&project, task))
            .collect();

        let unit = SprintUnit::of(&counted_tasks);
        let total = unit.work_of_all(&counted_tasks);
        let finished: Vec<(CalendarDate, u32)> = counted_tasks
            .iter()
            .filter(|task| task.is_finished)
            .filter_map(|task| {
                task.completed_at
                    .map(|moment| (self.clock.day_of(moment), unit.work_of(task)))
            })
            .collect();

        let today = self.clock.day_of(self.clock.now());
        let last_day_come = match sprint.state() {
            SprintState::Planned => None,
            SprintState::Active | SprintState::Completed => Some(
                sprint
                    .completed_at()
                    .map_or(today, |moment| today.min(self.clock.day_of(moment))),
            ),
        };

        let dates = sprint.dates();
        let days = match (dates.start_on(), dates.end_on()) {
            (Some(start_on), Some(end_on)) => start_on
                .iter_days()
                .take_while(|day| *day <= end_on)
                .map(|date| {
                    let has_come = last_day_come.is_some_and(|last| date <= last);
                    let done = finished
                        .iter()
                        .filter(|(day, _)| *day <= date)
                        .map(|(_, work)| *work)
                        .fold(0, u32::saturating_add);
                    SprintDay {
                        date,
                        remaining: has_come.then(|| total.saturating_sub(done)),
                    }
                })
                .collect(),
            _ => Vec::new(),
        };

        Ok(SprintReport {
            sprint_id: sprint.id(),
            unit,
            total,
            days,
        })
    }

    /// The tasks of a sprint, its subtasks included.
    fn tasks_of_sprint(&self, sprint: &Sprint) -> Result<Vec<Task>, ApplicationError> {
        Ok(self
            .tasks
            .list_for_project(sprint.project_id())?
            .into_iter()
            .filter(|task| task.sprint_id() == Some(sprint.id()))
            .collect())
    }

    /// Loads a project, or reports that it does not exist.
    fn require_project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Loads a sprint, or reports that it does not exist.
    fn require_sprint(&self, id: SprintId) -> Result<Sprint, ApplicationError> {
        self.sprints
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "sprint",
                id: id.to_string(),
            })
    }
}

/// Whether a task is finished: in a status of the category done. A status
/// the workflow no longer has counts as not done.
fn is_finished(project: &Project, task: &Task) -> bool {
    project
        .workflow()
        .status(task.status_id())
        .is_some_and(|status| status.category() == StatusCategory::Done)
}

/// A task as a sprint counts it.
fn counted(project: &Project, task: &Task) -> CountedTask {
    CountedTask {
        points: task.points(),
        completed_at: task.completed_at(),
        is_finished: is_finished(project, task),
    }
}

/// Orders sprints the way they are listed: the active one, the planned ones
/// as they were planned, the completed ones, the one completed last first.
fn as_listed(first: &Sprint, second: &Sprint) -> Ordering {
    let place = |sprint: &Sprint| match sprint.state() {
        SprintState::Active => 0,
        SprintState::Planned => 1,
        SprintState::Completed => 2,
    };
    place(first).cmp(&place(second)).then_with(|| {
        if first.state() == SprintState::Completed {
            Reverse(first.completed_at()).cmp(&Reverse(second.completed_at()))
        } else {
            first.created_at().cmp(&second.created_at())
        }
        // Sprints planned or completed at the same moment keep the order in
        // which they were planned.
        .then_with(|| first.id().cmp(&second.id()))
    })
}

#[cfg(test)]
mod tests;
