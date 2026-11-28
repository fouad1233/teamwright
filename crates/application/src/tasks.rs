//! Use cases for tasks.

use std::collections::BTreeSet;
use std::sync::Arc;

use teamwright_domain::{ActualDates, Timing, actual_dates_of, timing_of};
use teamwright_domain::{
    AreaId, Assignee, CalendarDate, ComputerId, DependencyGraph, Outline, PartMove, Priority,
    Project, ProjectId, Rank, RegroupingProblem, Schedule, SprintId, Status, StatusId, StoryPoints,
    Task, TaskChange, TaskDraft, TaskId, TaskInPlan, TaskTitle, Timestamp, changes_between,
    check_regrouping, history_of,
};

/// What really happened to a task, against its plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskActual {
    /// The task.
    pub task_id: TaskId,
    /// When work on it began and ended, read from its versions.
    pub dates: ActualDates,
    /// The day work began, where the program runs.
    pub started_on: Option<CalendarDate>,
    /// The day it was finished, where the program runs.
    pub finished_on: Option<CalendarDate>,
    /// How that compares with its planned start and due day.
    pub timing: Timing,
}

/// One change of a task, with the computer that made it, if it is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// What changed, and when.
    pub change: TaskChange,
    /// The computer that made the change.
    pub computer: Option<ComputerId>,
}

/// One change of a task in the history of its project: what the history
/// of the task says, and which task it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChange {
    /// What changed, and when.
    pub change: TaskChange,
    /// The task it is about.
    pub task_id: TaskId,
    /// The number of the task, as it is now or as it was when it was last
    /// stored, if it is gone.
    pub task_number: u32,
    /// The title of the task, as it is now or as it was when it was last
    /// stored, if it is gone.
    pub task_title: TaskTitle,
    /// Whether the task has been deleted since.
    pub task_gone: bool,
}

/// The most changes one page of the history of a project has.
pub const MOST_PROJECT_CHANGES: usize = 200;

use crate::areas::area_for_new_task;
use crate::error::ApplicationError;
use crate::ports::{
    AreaRepository, Clock, ConversationRepository, MemberRepository, ProjectRepository,
    SprintRepository, TaskRepository,
};
use crate::views::NewTaskDemands;
use crate::work::WorkInProgress;

/// What is needed to open a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTask {
    /// The project the task belongs to.
    pub project_id: ProjectId,
    /// What the task is about.
    pub title: String,
    /// Details, acceptance criteria, links. May be empty.
    pub description: String,
    /// How urgent the task is.
    pub priority: Priority,
    /// Who is responsible, if anyone yet.
    pub assignee: Option<Assignee>,
    /// The status the task starts in. If it is missing, the task starts in
    /// the project's first "todo" status.
    pub status_id: Option<StatusId>,
    /// The task this one is a part of, if it is a subtask.
    pub parent_id: Option<TaskId>,
    /// The area the task goes into. If it is missing, the task goes into
    /// the first area of the project, or, as a subtask, into the area of
    /// its task. A subtask cannot name another area than its task's.
    pub area_id: Option<AreaId>,
    /// The expected working time in minutes.
    pub estimate_minutes: Option<u32>,
    /// The day work is planned to start.
    pub start_on: Option<CalendarDate>,
    /// The day the task is due.
    pub due_on: Option<CalendarDate>,
}

impl NewTask {
    /// A task with a title and nothing else decided.
    #[must_use]
    pub fn titled(project_id: ProjectId, title: impl Into<String>) -> Self {
        Self {
            project_id,
            title: title.into(),
            description: String::new(),
            priority: Priority::default(),
            assignee: None,
            status_id: None,
            parent_id: None,
            area_id: None,
            estimate_minutes: None,
            start_on: None,
            due_on: None,
        }
    }
}

/// The use cases that open, change, plan, move, connect and remove tasks.
#[derive(Clone)]
pub struct TaskService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    members: Arc<dyn MemberRepository>,
    conversations: Arc<dyn ConversationRepository>,
    sprints: Arc<dyn SprintRepository>,
    areas: Arc<dyn AreaRepository>,
    clock: Arc<dyn Clock>,
}

impl TaskService {
    /// Creates the service. The conversations are needed because the record
    /// of the work on a task goes with the task, and a task a specialist is
    /// working on is not removed. The sprints are needed to plan a task
    /// into one, and the areas to put a new task into one.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        members: Arc<dyn MemberRepository>,
        conversations: Arc<dyn ConversationRepository>,
        sprints: Arc<dyn SprintRepository>,
        areas: Arc<dyn AreaRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            members,
            conversations,
            sprints,
            areas,
            clock,
        }
    }

    /// Opens a task at the bottom of its status. It starts in the backlog
    /// without points, or, as a subtask, in the sprint of the task it is a
    /// part of. It goes into the area it names, or the first area of the
    /// project; a subtask goes into the area of its task.
    ///
    /// # Errors
    ///
    /// Returns an error if the project, the status, the parent task, the
    /// assignee or the area does not exist, if the parent, the assignee or
    /// the area belongs to another project, if a subtask names another area
    /// than its task's, if the title or the dates are not valid, or if
    /// storage fails.
    pub fn create(&self, new_task: NewTask) -> Result<Task, ApplicationError> {
        self.create_in_view(new_task, &NewTaskDemands::default())
    }

    /// Opens a task made inside a view, as [`create`](Self::create) does,
    /// and gives it what the view asks of it: its tag, and its value of a
    /// custom field, so that the task stays in the view. The demands come
    /// from [`ViewService::demands_of_new_tasks`](crate::ViewService::demands_of_new_tasks).
    /// A subtask is not made inside a view, and takes none of them.
    ///
    /// # Errors
    ///
    /// The same as [`create`](Self::create).
    pub fn create_in_view(
        &self,
        new_task: NewTask,
        demands: &NewTaskDemands,
    ) -> Result<Task, ApplicationError> {
        let mut project = self.require_project(new_task.project_id)?;
        let title = TaskTitle::parse(&new_task.title)?;
        let schedule = Schedule::new(new_task.start_on, new_task.due_on)?;
        let status = match new_task.status_id {
            Some(status_id) => project.workflow().require_status(status_id)?.clone(),
            None => project.workflow().initial_status().clone(),
        };
        let parent = new_task
            .parent_id
            .map(|parent_id| self.require_task(parent_id))
            .transpose()?;
        if parent
            .as_ref()
            .is_some_and(|parent| parent.project_id() != project.id())
        {
            return Err(ApplicationError::DifferentProjects);
        }
        self.check_assignee(new_task.assignee, project.id())?;
        let area_id = area_for_new_task(
            self.areas.as_ref(),
            &project,
            new_task.area_id,
            parent.as_ref(),
        )?;

        let last_rank_in_status = self
            .tasks
            .list_for_project(project.id())?
            .iter()
            .filter(|task| task.status_id() == status.id())
            .map(Task::rank)
            .max();
        let draft = TaskDraft {
            title,
            description: new_task.description,
            priority: new_task.priority,
            assignee: new_task.assignee,
            parent_id: new_task.parent_id,
            estimate_minutes: new_task.estimate_minutes,
            schedule,
        };
        let number = project.allocate_task_number();
        let now = self.clock.now();
        let mut task = Task::create(
            project.id(),
            area_id,
            number,
            &status,
            Rank::between(last_rank_in_status, None),
            draft,
            now,
        );
        // A subtask goes where the task it is a part of goes.
        if let Some(sprint_id) = parent.as_ref().and_then(Task::sprint_id) {
            task.plan_into(Some(sprint_id), now);
        }
        if parent.is_none() {
            if let Some(tag_id) = demands.tag {
                task.put_tag(tag_id, now);
            }
            if let Some((field_id, value)) = &demands.field_value {
                task.set_field_value(*field_id, value.clone(), now);
            }
        }

        // The project is stored first. If storing the task then fails, one
        // number is skipped, which is harmless. In the other order a failure
        // could hand the same number out twice.
        self.projects.save(&project)?;
        self.tasks.save(&task)?;
        Ok(task)
    }

    /// Finds a task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn get(&self, id: TaskId) -> Result<Task, ApplicationError> {
        self.require_task(id)
    }

    /// Lists the tasks of a project in the order a manager arranged them:
    /// by rank, and by number where ranks are equal.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Task>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let mut tasks = self.tasks.list_for_project(project.id())?;
        sort_as_arranged(&mut tasks);
        Ok(tasks)
    }

    /// Tells the history of a task, the oldest change first: that it was
    /// opened, and then everything that changed about it since. A move up
    /// or down within a status is not a change.
    ///
    /// Nothing is kept for the history itself. It is read off the versions
    /// of the task that storage has kept.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn history(&self, id: TaskId) -> Result<Vec<TaskChange>, ApplicationError> {
        let task = self.require_task(id)?;
        let mut versions = self.tasks.versions_of(id)?;
        if versions.is_empty() {
            // Storage that kept no versions still knows the task as it is.
            versions.push(task);
        }
        Ok(history_of(&versions))
    }

    /// Tells the history of a task like [`Self::history`], with the
    /// computer that made each change, where storage knows it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn history_with_computers(
        &self,
        id: TaskId,
    ) -> Result<Vec<HistoryEntry>, ApplicationError> {
        let task = self.require_task(id)?;
        let versions = self.tasks.versions_of(id)?;
        let computers = self.tasks.computers_of_versions(id)?;
        if versions.is_empty() || computers.len() != versions.len() {
            let versions = if versions.is_empty() {
                vec![task]
            } else {
                versions
            };
            return Ok(history_of(&versions)
                .into_iter()
                .map(|change| HistoryEntry {
                    change,
                    computer: None,
                })
                .collect());
        }
        let mut history = Vec::new();
        for (index, (version, computer)) in versions.iter().zip(&computers).enumerate() {
            let changes = match index.checked_sub(1).and_then(|before| versions.get(before)) {
                None => history_of(std::slice::from_ref(version)),
                Some(before) => changes_between(before, version),
            };
            history.extend(changes.into_iter().map(|change| HistoryEntry {
                change,
                computer: Some(*computer),
            }));
        }
        Ok(history)
    }

    /// Tells what really happened to every task of a project, subtasks
    /// included, against its plan: when work on it began (it first went
    /// into a status of a started category) and when it was finished (it
    /// reached the category done), and whether that was late. It is read off
    /// the versions of each task in the log, one read for each task, by
    /// the categories of the statuses and never their names.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn actual_dates(&self, project_id: ProjectId) -> Result<Vec<TaskActual>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let today = self.clock.day_of(self.clock.now());
        let mut tasks = self.tasks.list_for_project(project.id())?;
        tasks.sort_by_key(Task::number);
        tasks
            .into_iter()
            .map(|task| {
                let mut versions = self.tasks.versions_of(task.id())?;
                if versions.is_empty() {
                    versions.push(task.clone());
                }
                let dates = actual_dates_of(&versions, project.workflow());
                let started_on = dates.started_at.map(|moment| self.clock.day_of(moment));
                let finished_on = dates.finished_at.map(|moment| self.clock.day_of(moment));
                Ok(TaskActual {
                    task_id: task.id(),
                    dates,
                    started_on,
                    finished_on,
                    timing: timing_of(task.schedule(), started_on, finished_on, today),
                })
            })
            .collect()
    }

    /// Tells what happened to the tasks of a project, the newest change
    /// first: one page of at most `limit` changes (and never more than
    /// [`MOST_PROJECT_CHANGES`]) that happened before the moment `before`,
    /// or the newest of all without it. Each change is what the history of
    /// its task says ([`Self::history`]), with the task it is about. A task
    /// deleted since keeps the number and the title it had when it was last
    /// stored, and is marked as gone.
    ///
    /// Moments are compared to the millisecond, as the interface sends
    /// them. A page never splits the changes of one millisecond: if the
    /// last ones would not all fit, they are left for the next page, whose
    /// `before` is their moment. Only a page that would then be empty takes
    /// them all, even beyond `limit`.
    ///
    /// Nothing is kept for the history itself: it is read off the versions
    /// of every task the project has had, so a long history costs as much
    /// to read as it is long.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn project_history(
        &self,
        project_id: ProjectId,
        limit: usize,
        before: Option<Timestamp>,
    ) -> Result<Vec<ProjectChange>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let limit = limit.min(MOST_PROJECT_CHANGES);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let before = before.map(|moment| moment.timestamp_millis());

        // Each change with its place in the history of its task, so that
        // the changes of one task at one moment keep their order, reversed.
        let mut changes: Vec<(usize, ProjectChange)> = Vec::new();
        for id in self.tasks.ids_ever_in_project(project.id())? {
            let current = self.tasks.find(id)?;
            let mut versions = self.tasks.versions_of(id)?;
            if versions.is_empty() {
                // Storage that kept no versions still knows the task as it
                // is, if it is there.
                versions.extend(current.clone());
            }
            let Some(latest) = current.as_ref().or_else(|| versions.last()) else {
                continue;
            };
            let (task_number, task_title) = (latest.number(), latest.title().clone());
            let task_gone = current.is_none();
            changes.extend(
                history_of(&versions)
                    .into_iter()
                    .enumerate()
                    .filter(|(_, change)| {
                        before.is_none_or(|before| change.at.timestamp_millis() < before)
                    })
                    .map(|(place, change)| {
                        (
                            place,
                            ProjectChange {
                                change,
                                task_id: id,
                                task_number,
                                task_title: task_title.clone(),
                                task_gone,
                            },
                        )
                    }),
            );
        }
        changes.sort_by(|(one_place, one), (other_place, other)| {
            other
                .change
                .at
                .cmp(&one.change.at)
                .then_with(|| one.task_id.cmp(&other.task_id))
                .then_with(|| other_place.cmp(one_place))
        });

        let mut changes: Vec<ProjectChange> =
            changes.into_iter().map(|(_, change)| change).collect();
        if let Some(first_left_out) = changes.get(limit) {
            let cut = first_left_out.change.at.timestamp_millis();
            let in_cut = |change: &ProjectChange| change.change.at.timestamp_millis() == cut;
            let mut end = limit;
            while end > 0 && changes.get(end - 1).is_some_and(in_cut) {
                end -= 1;
            }
            if end == 0 {
                end = changes.iter().take_while(|change| in_cut(change)).count();
            }
            changes.truncate(end);
        }
        Ok(changes)
    }

    /// Gives a task a new title.
    ///
    /// # Errors
    ///
    /// Returns an error if the title is not valid, if there is no such task,
    /// or if storage fails.
    pub fn rename(&self, id: TaskId, title: &str) -> Result<Task, ApplicationError> {
        let title = TaskTitle::parse(title)?;
        self.change(id, |task, now| {
            task.rename(title, now);
            Ok(())
        })
    }

    /// Replaces the description of a task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn describe(&self, id: TaskId, description: &str) -> Result<Task, ApplicationError> {
        self.change(id, |task, now| {
            task.describe(description.to_owned(), now);
            Ok(())
        })
    }

    /// Changes how urgent a task is.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn prioritize(&self, id: TaskId, priority: Priority) -> Result<Task, ApplicationError> {
        self.change(id, |task, now| {
            task.prioritize(priority, now);
            Ok(())
        })
    }

    /// Makes someone responsible for a task, or nobody.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if the assignee is a team
    /// member that does not exist or is on the team of another project, or
    /// if storage fails.
    pub fn assign(&self, id: TaskId, assignee: Option<Assignee>) -> Result<Task, ApplicationError> {
        self.change(id, |task, now| {
            self.check_assignee(assignee, task.project_id())?;
            task.assign(assignee, now);
            Ok(())
        })
    }

    /// Sets or clears the expected working time of a task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn estimate(&self, id: TaskId, minutes: Option<u32>) -> Result<Task, ApplicationError> {
        self.change(id, |task, now| {
            task.estimate(minutes, now);
            Ok(())
        })
    }

    /// Puts a task into a planned or an active sprint, or with `None` back
    /// into the backlog. Its subtasks go with it. A subtask is not planned
    /// by itself: it goes where the task it is a part of goes.
    ///
    /// # Errors
    ///
    /// Returns an error if the task or the sprint does not exist, if the
    /// task is a subtask, if the sprint belongs to another project or is
    /// completed, or if storage fails.
    pub fn plan(&self, id: TaskId, sprint_id: Option<SprintId>) -> Result<Task, ApplicationError> {
        let task = self.require_task(id)?;
        let project = self.require_project(task.project_id())?;
        if let Some(parent) = task
            .parent_id()
            .map(|parent_id| self.tasks.find(parent_id))
            .transpose()?
            .flatten()
        {
            return Err(ApplicationError::SubtaskPlannedAlone {
                task: reference(&project, &task),
                parent: reference(&project, &parent),
            });
        }
        if let Some(sprint_id) = sprint_id {
            let sprint =
                self.sprints
                    .find(sprint_id)?
                    .ok_or_else(|| ApplicationError::NotFound {
                        kind: "sprint",
                        id: sprint_id.to_string(),
                    })?;
            if sprint.project_id() != task.project_id() {
                return Err(ApplicationError::SprintOfAnotherProject);
            }
            sprint.check_takes_tasks()?;
        }

        let now = self.clock.now();
        let tasks_of_project = self.tasks.list_for_project(task.project_id())?;
        let moving = subtree_of(id, &tasks_of_project);
        let planned: Vec<Task> = tasks_of_project
            .into_iter()
            .filter(|other| moving.contains(&other.id()) && other.sprint_id() != sprint_id)
            .map(|mut other| {
                other.plan_into(sprint_id, now);
                other
            })
            .collect();
        self.tasks.save_all(&planned)?;
        Ok(planned
            .into_iter()
            .find(|planned| planned.id() == id)
            .unwrap_or(task))
    }

    /// Gives a task story points, or with `None` takes them away. The points
    /// come as any number, as the interface sends them, and must be a whole
    /// number from 0 to 999.
    ///
    /// # Errors
    ///
    /// Returns an error if the points are not a whole number from 0 to 999,
    /// if there is no such task, or if storage fails.
    pub fn set_points(&self, id: TaskId, points: Option<f64>) -> Result<Task, ApplicationError> {
        let points = points.map(StoryPoints::from_number).transpose()?;
        self.change(id, |task, now| {
            task.set_points(points, now);
            Ok(())
        })
    }

    /// Changes the planned start and due dates of a task.
    ///
    /// # Errors
    ///
    /// Returns an error if the start is after the due date, if there is no
    /// such task, or if storage fails.
    pub fn reschedule(
        &self,
        id: TaskId,
        start_on: Option<CalendarDate>,
        due_on: Option<CalendarDate>,
    ) -> Result<Task, ApplicationError> {
        let schedule = Schedule::new(start_on, due_on)?;
        self.change(id, |task, now| {
            task.reschedule(schedule, now);
            Ok(())
        })
    }

    /// Moves a task to a status and to a position among the tasks there.
    ///
    /// `index` counts the other tasks of that status that are listed next to
    /// this one, from the top: `0` puts the task first, and a number past
    /// the end puts it last.
    ///
    /// The tasks listed next to a task are the ones with the same parent. A
    /// board shows subtasks under their parent, so a position in one of its
    /// columns counts only the tasks that are not part of another task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if the status is not part
    /// of the task's project, or if storage fails.
    pub fn move_to(
        &self,
        id: TaskId,
        status_id: StatusId,
        index: usize,
    ) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(id)?;
        let project = self.require_project(task.project_id())?;
        let status = project.workflow().require_status(status_id)?;
        let now = self.clock.now();

        let mut neighbours: Vec<Task> = self
            .tasks
            .list_for_project(project.id())?
            .into_iter()
            .filter(|other| {
                other.status_id() == status_id
                    && other.parent_id() == task.parent_id()
                    && other.id() != id
            })
            .collect();
        sort_as_arranged(&mut neighbours);

        let index = index.min(neighbours.len());
        let before = index.checked_sub(1).and_then(|above| neighbours.get(above));
        let after = neighbours.get(index);
        let (before, after) = (before.map(Task::rank), after.map(Task::rank));
        let rank = Rank::between(before, after);
        task.move_to(status, rank, now);

        if rank.fits_between(before, after) {
            self.tasks.save(&task)?;
            return Ok(task);
        }

        // The neighbours are too close together for another rank between
        // them. Spread the whole list out again, with the task in place.
        let mut whole_list = neighbours;
        whole_list.insert(index, task);
        space_out(&mut whole_list, status, now);
        self.tasks.save_all(&whole_list)?;
        self.require_task(id)
    }

    /// Makes a task a part of another task of its project, or, with
    /// `None`, a task of its own. Its own parts go with it. As a subtask is
    /// in the area and the sprint of the task it is a part of, the task and
    /// its parts go into the area and the sprint of their new whole; a task
    /// made one of its own keeps them. Nothing else of the tasks changes,
    /// and all of it is stored in one write.
    ///
    /// # Errors
    ///
    /// Returns an error if either task does not exist, if they belong to
    /// different projects, if the task would be a part of itself or of one
    /// of its own parts (`part_in_circle`), if it or one of its parts waits
    /// for the new whole, directly or through other tasks, which would then
    /// wait for it in turn (`part_in_circle`), or if storage fails.
    pub fn make_part_of(
        &self,
        id: TaskId,
        whole_id: Option<TaskId>,
    ) -> Result<Task, ApplicationError> {
        let task = self.require_task(id)?;
        let whole = whole_id
            .map(|whole_id| self.require_task(whole_id))
            .transpose()?;
        if whole
            .as_ref()
            .is_some_and(|whole| whole.project_id() != task.project_id())
        {
            return Err(ApplicationError::DifferentProjects);
        }
        let project = self.require_project(task.project_id())?;
        let mut tasks_of_project = self.tasks.list_for_project(project.id())?;
        check_regrouping(
            &Outline::of_tasks(&tasks_of_project),
            &[],
            &[PartMove {
                task: id,
                part_of: whole_id.map(TaskInPlan::Existing),
            }],
        )
        .map_err(|problem| {
            regrouping_error(problem, |named| match named {
                TaskInPlan::Existing(other) => tasks_of_project
                    .iter()
                    .find(|known| known.id() == other)
                    .map_or_else(
                        || "a task that is gone".to_owned(),
                        |known| reference(&project, known),
                    ),
                TaskInPlan::Proposed(place) => format!("task {}", place + 1),
            })
        })?;
        if task.parent_id() == whole_id {
            return Ok(task);
        }

        let now = self.clock.now();
        let before = tasks_of_project.clone();
        if let Some(moved) = tasks_of_project.iter_mut().find(|other| other.id() == id) {
            moved.set_parent(whole_id, now)?;
        }
        follow_tops(&mut tasks_of_project, &subtree_of(id, &before), now);
        let changed: Vec<Task> = tasks_of_project
            .into_iter()
            .filter(|after| !before.contains(after))
            .collect();
        self.tasks.save_all(&changed)?;
        changed
            .into_iter()
            .find(|moved| moved.id() == id)
            .map_or_else(|| self.require_task(id), Ok)
    }

    /// Makes a task wait until another task is finished. A part does not
    /// wait for the task it is a part of, directly or through other tasks:
    /// that task waits for its parts.
    ///
    /// # Errors
    ///
    /// Returns an error if either task does not exist, if they belong to
    /// different projects, if the dependency would leave tasks waiting for
    /// each other, or if storage fails.
    pub fn add_dependency(&self, id: TaskId, blocker_id: TaskId) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(id)?;
        let blocker = self.require_task(blocker_id)?;
        if task.project_id() != blocker.project_id() {
            return Err(ApplicationError::DifferentProjects);
        }

        let tasks_of_project = self.tasks.list_for_project(task.project_id())?;
        let dependencies = DependencyGraph::from_tasks_and_parts(&tasks_of_project);
        if id != blocker_id && dependencies.would_create_cycle(id, blocker_id) {
            let project = self.require_project(task.project_id())?;
            return Err(ApplicationError::DependencyCycle {
                task: reference(&project, &task),
                blocker: reference(&project, &blocker),
            });
        }

        task.block_on(blocker_id, self.clock.now())?;
        self.tasks.save(&task)?;
        Ok(task)
    }

    /// Stops a task from waiting for another task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn remove_dependency(
        &self,
        id: TaskId,
        blocker_id: TaskId,
    ) -> Result<Task, ApplicationError> {
        self.change(id, |task, now| {
            task.unblock_from(blocker_id, now);
            Ok(())
        })
    }

    /// Removes a task together with its subtasks. Tasks that waited for a
    /// removed task stop waiting for it. The conversations about the removed
    /// tasks go with them.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if a specialist is working
    /// on it or on one of its subtasks, or if storage fails.
    pub fn delete(&self, id: TaskId) -> Result<(), ApplicationError> {
        let task = self.require_task(id)?;
        let tasks_of_project = self.tasks.list_for_project(task.project_id())?;
        let removed = subtree_of(id, &tasks_of_project);
        let now = self.clock.now();

        let about_removed: Vec<_> = self
            .conversations
            .list_for_project(task.project_id())?
            .into_iter()
            .filter(|conversation| {
                conversation
                    .task_id()
                    .is_some_and(|task_id| removed.contains(&task_id))
            })
            .collect();
        WorkInProgress {
            projects: self.projects.as_ref(),
            tasks: self.tasks.as_ref(),
            members: self.members.as_ref(),
            conversations: self.conversations.as_ref(),
        }
        .refuse_while_worked_on(&about_removed)?;

        let released: Vec<Task> = tasks_of_project
            .into_iter()
            .filter(|other| !removed.contains(&other.id()))
            .filter_map(|mut other| {
                let waited_for: Vec<TaskId> =
                    other.blocked_by().intersection(&removed).copied().collect();
                if waited_for.is_empty() {
                    return None;
                }
                for blocker_id in waited_for {
                    other.unblock_from(blocker_id, now);
                }
                Some(other)
            })
            .collect();

        // The conversations go first. If removing the tasks then fails, the
        // tasks are still there without their record, which is harmless; a
        // conversation about a task that no longer exists could not be
        // shown.
        let conversation_ids: Vec<_> = about_removed
            .iter()
            .map(|conversation| conversation.id())
            .collect();
        self.conversations.delete(&conversation_ids)?;
        self.tasks.save_all(&released)?;
        self.tasks
            .delete(&removed.into_iter().collect::<Vec<_>>())?;
        Ok(())
    }

    /// Loads a task, changes it and stores it.
    fn change(
        &self,
        id: TaskId,
        apply: impl FnOnce(&mut Task, Timestamp) -> Result<(), ApplicationError>,
    ) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(id)?;
        apply(&mut task, self.clock.now())?;
        self.tasks.save(&task)?;
        Ok(task)
    }

    /// Makes sure that a task of a project can be given to an assignee: a
    /// team member must exist and be on the team of that project.
    fn check_assignee(
        &self,
        assignee: Option<Assignee>,
        project_id: ProjectId,
    ) -> Result<(), ApplicationError> {
        let Some(Assignee::Member(member_id)) = assignee else {
            return Ok(());
        };
        let member = self
            .members
            .find(member_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "team member",
                id: member_id.to_string(),
            })?;
        if member.project_id() != project_id {
            return Err(ApplicationError::AssigneeFromAnotherTeam);
        }
        Ok(())
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

    /// Loads a task, or reports that it does not exist.
    fn require_task(&self, id: TaskId) -> Result<Task, ApplicationError> {
        self.tasks
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "task",
                id: id.to_string(),
            })
    }
}

/// Sorts tasks the way a manager arranged them: by rank, and by number where
/// ranks are equal.
fn sort_as_arranged(tasks: &mut [Task]) {
    tasks.sort_by_key(|task| (task.rank(), task.number()));
}

/// Gives the tasks of one status evenly spaced ranks, keeping their order.
fn space_out(tasks_of_status: &mut [Task], status: &Status, now: Timestamp) {
    let mut previous_rank = None;
    for task in tasks_of_status {
        let rank = Rank::between(previous_rank, None);
        task.move_to(status, rank, now);
        previous_rank = Some(rank);
    }
}

/// The task and every task below it in the hierarchy of subtasks.
pub(crate) fn subtree_of(root: TaskId, tasks: &[Task]) -> BTreeSet<TaskId> {
    let mut members = BTreeSet::from([root]);
    loop {
        let size_before = members.len();
        for task in tasks {
            if task
                .parent_id()
                .is_some_and(|parent_id| members.contains(&parent_id))
            {
                members.insert(task.id());
            }
        }
        if members.len() == size_before {
            return members;
        }
    }
}

/// Puts each of the `regrouped` tasks into the area and the sprint of the
/// top of its chain of wholes, as a subtask goes where the task it is a
/// part of goes. A top keeps its own. Only what differs is changed.
pub(crate) fn follow_tops(tasks: &mut [Task], regrouped: &BTreeSet<TaskId>, now: Timestamp) {
    let wholes: std::collections::HashMap<TaskId, Option<TaskId>> = tasks
        .iter()
        .map(|task| (task.id(), task.parent_id()))
        .collect();
    let places: std::collections::HashMap<TaskId, (AreaId, Option<SprintId>)> = tasks
        .iter()
        .map(|task| (task.id(), (task.area_id(), task.sprint_id())))
        .collect();
    for task in tasks
        .iter_mut()
        .filter(|task| regrouped.contains(&task.id()))
    {
        // The chains are free of circles, which was checked; the count only
        // guards against damaged data.
        let mut top = task.id();
        for _ in 0..wholes.len() {
            match wholes.get(&top).copied().flatten() {
                Some(whole) if wholes.contains_key(&whole) => top = whole,
                _ => break,
            }
        }
        let Some(&(area_id, sprint_id)) = places.get(&top) else {
            continue;
        };
        if task.area_id() != area_id {
            task.put_into_area(area_id, now);
        }
        if task.sprint_id() != sprint_id {
            task.plan_into(sprint_id, now);
        }
    }
}

/// The error of a regrouping that breaks a rule of parts, with the tasks
/// named by `name`: a task of the project by its reference, a new task of
/// a proposal by its title.
pub(crate) fn regrouping_error<Key: PartialEq>(
    problem: RegroupingProblem<Key>,
    name: impl Fn(TaskInPlan<Key>) -> String,
) -> ApplicationError {
    match problem {
        RegroupingProblem::NotThere(_) | RegroupingProblem::NoSuchPlace(_) => {
            ApplicationError::ProposedTaskGone
        }
        RegroupingProblem::MovedTwice(_) => {
            ApplicationError::Rule(teamwright_domain::DomainError::TaskMovedTwice)
        }
        RegroupingProblem::PartOfItsOwnPart { task, whole } if task == whole => {
            ApplicationError::PartOfItself { task: name(task) }
        }
        RegroupingProblem::PartOfItsOwnPart { task, whole } => ApplicationError::PartOfItsOwnPart {
            task: name(task),
            whole: name(whole),
        },
        RegroupingProblem::PartWaitsForWhole { task, whole } => {
            ApplicationError::PartWaitsForWhole {
                task: name(task),
                whole: name(whole),
            }
        }
    }
}

/// The name a manager knows a task by, such as `IOT-12`.
pub(crate) fn reference(project: &Project, task: &Task) -> String {
    format!("{}-{}", project.key(), task.number())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};
    use teamwright_domain::{DomainError, Member, MemberId, MemberKind, StatusCategory};

    use super::*;
    use crate::in_memory::InMemorySprintRepository;
    use crate::in_memory::{
        InMemoryConversationRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
        InMemoryProjectRepository, InMemoryTaskRepository, ManualClock,
    };
    use crate::projects::{NewProject, ProjectService};
    use crate::team::{NewMember, TeamService};

    /// The services under test, sharing storage and a clock that the test
    /// controls.
    struct Workspace {
        projects: ProjectService,
        tasks: TaskService,
        team: TeamService,
        sprints: crate::sprints::SprintService,
        areas: crate::areas::AreaService,
        clock: Arc<ManualClock>,
    }

    fn start_of_day() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    fn workspace() -> Workspace {
        let project_repository = Arc::new(InMemoryProjectRepository::default());
        let task_repository = Arc::new(InMemoryTaskRepository::default());
        let member_repository = Arc::new(InMemoryMemberRepository::default());
        let conversation_repository = Arc::new(InMemoryConversationRepository::default());
        let area_store = Arc::new(crate::in_memory::InMemoryAreaRepository::default());
        let clock = Arc::new(ManualClock::starting_at(start_of_day()));
        let sprint_store = Arc::new(InMemorySprintRepository::new(task_repository.clone()));
        Workspace {
            sprints: crate::sprints::SprintService::new(
                project_repository.clone(),
                task_repository.clone(),
                sprint_store.clone(),
                clock.clone(),
            ),
            areas: crate::areas::AreaService::new(
                project_repository.clone(),
                task_repository.clone(),
                area_store.clone(),
                clock.clone(),
            ),
            projects: ProjectService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                Arc::new(InMemoryPlacesRepository::default()),
                sprint_store.clone(),
                Arc::new(crate::in_memory::InMemoryViewRepository::default()),
                area_store.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                sprint_store.clone(),
                area_store.clone(),
                clock.clone(),
            ),
            team: TeamService::new(
                project_repository,
                member_repository,
                task_repository,
                conversation_repository,
                Arc::new(crate::in_memory::InMemoryViewRepository::default()),
                clock.clone(),
            ),
            clock,
        }
    }

    impl Workspace {
        fn project(&self, name: &str) -> Project {
            self.projects
                .create(NewProject {
                    name: name.to_owned(),
                    ..NewProject::default()
                })
                .unwrap()
        }

        fn task(&self, project: &Project, title: &str) -> Task {
            self.tasks
                .create(NewTask::titled(project.id(), title))
                .unwrap()
        }

        fn member(&self, project: &Project, name: &str) -> Member {
            self.team
                .add(NewMember {
                    project_id: project.id(),
                    name: name.to_owned(),
                    kind: MemberKind::Agent,
                    responsibility: String::new(),
                    model: None,
                    effort: None,
                })
                .unwrap()
        }

        fn titles_in(&self, project: &Project, status_id: StatusId) -> Vec<String> {
            self.tasks
                .list_for_project(project.id())
                .unwrap()
                .into_iter()
                .filter(|task| task.status_id() == status_id)
                .map(|task| task.title().to_string())
                .collect()
        }
    }

    fn status_in(project: &Project, category: StatusCategory) -> StatusId {
        project.workflow().first_in(category).unwrap().id()
    }

    #[test]
    fn task_numbers_count_up_within_each_project() {
        let workspace = workspace();
        let sensor = workspace.project("Smart sensor node");
        let display = workspace.project("Display driver");

        let sensor_numbers = [
            workspace.task(&sensor, "First").number(),
            workspace.task(&sensor, "Second").number(),
        ];
        let display_number = workspace.task(&display, "First").number();

        assert_eq!(sensor_numbers, [1, 2]);
        assert_eq!(display_number, 1);
        assert_eq!(
            workspace
                .projects
                .get(sensor.id())
                .unwrap()
                .next_task_number(),
            3
        );
    }

    #[test]
    fn a_new_task_goes_to_the_bottom_of_the_first_todo_status() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        workspace.task(&project, "Choose the microcontroller");
        workspace.task(&project, "Draw the schematic");
        workspace.task(&project, "Route the board");

        assert_eq!(
            workspace.titles_in(&project, status_in(&project, StatusCategory::Todo)),
            [
                "Choose the microcontroller",
                "Draw the schematic",
                "Route the board"
            ]
        );
    }

    #[test]
    fn a_task_can_be_opened_directly_in_another_status() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let in_progress = status_in(&project, StatusCategory::Active);

        let task = workspace
            .tasks
            .create(NewTask {
                status_id: Some(in_progress),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap();

        assert_eq!(task.status_id(), in_progress);
    }

    #[test]
    fn a_task_takes_everything_that_was_given_for_it() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let engineer = Assignee::Member(workspace.member(&project, "Hardware engineer").id());

        let task = workspace
            .tasks
            .create(NewTask {
                description: "Cheap, small, with Wi-Fi.".to_owned(),
                priority: Priority::High,
                assignee: Some(engineer),
                estimate_minutes: Some(120),
                start_on: Some(day(12)),
                due_on: Some(day(14)),
                ..NewTask::titled(project.id(), "Choose the microcontroller")
            })
            .unwrap();

        let stored = workspace.tasks.get(task.id()).unwrap();
        assert_eq!(stored, task);
        assert_eq!(stored.description(), "Cheap, small, with Wi-Fi.");
        assert_eq!(stored.priority(), Priority::High);
        assert_eq!(stored.assignee(), Some(engineer));
        assert_eq!(stored.estimate_minutes(), Some(120));
        assert_eq!(stored.schedule().start_on(), Some(day(12)));
        assert_eq!(stored.schedule().due_on(), Some(day(14)));
    }

    #[test]
    fn a_task_cannot_start_in_a_status_of_another_project() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let other_project = workspace.project("Display driver");
        let foreign_status = status_in(&other_project, StatusCategory::Todo);

        let error = workspace
            .tasks
            .create(NewTask {
                status_id: Some(foreign_status),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::UnknownStatus { status_id })
                if status_id == foreign_status
        ));
    }

    #[test]
    fn a_task_without_a_title_is_refused() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let error = workspace
            .tasks
            .create(NewTask::titled(project.id(), "  "))
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::EmptyText {
                field: "task title"
            })
        ));
    }

    #[test]
    fn a_task_for_a_project_that_does_not_exist_is_refused() {
        let workspace = workspace();

        let error = workspace
            .tasks
            .create(NewTask::titled(ProjectId::generate(), "Draw the schematic"))
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "project",
                ..
            }
        ));
    }

    #[test]
    fn a_task_that_would_start_after_it_is_due_is_refused() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let error = workspace
            .tasks
            .create(NewTask {
                start_on: Some(day(20)),
                due_on: Some(day(12)),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::StartAfterDue { .. })
        ));
    }

    #[test]
    fn a_subtask_belongs_to_a_parent_in_the_same_project() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let parent = workspace.task(&project, "Design the board");

        let subtask = workspace
            .tasks
            .create(NewTask {
                parent_id: Some(parent.id()),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap();

        assert_eq!(subtask.parent_id(), Some(parent.id()));
    }

    #[test]
    fn a_subtask_cannot_have_a_parent_in_another_project() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let other_project = workspace.project("Display driver");
        let foreign_parent = workspace.task(&other_project, "Write the driver");

        let error = workspace
            .tasks
            .create(NewTask {
                parent_id: Some(foreign_parent.id()),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap_err();

        assert!(matches!(error, ApplicationError::DifferentProjects));
    }

    #[test]
    fn a_subtask_of_a_task_that_does_not_exist_is_refused() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let error = workspace
            .tasks
            .create(NewTask {
                parent_id: Some(TaskId::generate()),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "task", .. }
        ));
    }

    #[test]
    fn changes_to_a_task_are_stored_with_the_time_they_were_made() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace.task(&project, "Choose the microcontroler");
        let engineer = Assignee::Member(workspace.member(&project, "Hardware engineer").id());
        workspace.clock.advance(TimeDelta::hours(2));

        workspace
            .tasks
            .rename(task.id(), "Choose the microcontroller")
            .unwrap();
        workspace
            .tasks
            .describe(task.id(), "Cheap, small, with Wi-Fi.")
            .unwrap();
        workspace
            .tasks
            .prioritize(task.id(), Priority::Urgent)
            .unwrap();
        workspace.tasks.assign(task.id(), Some(engineer)).unwrap();
        workspace.tasks.estimate(task.id(), Some(45)).unwrap();
        workspace
            .tasks
            .reschedule(task.id(), Some(day(12)), Some(day(13)))
            .unwrap();

        let stored = workspace.tasks.get(task.id()).unwrap();
        assert_eq!(stored.title().as_str(), "Choose the microcontroller");
        assert_eq!(stored.description(), "Cheap, small, with Wi-Fi.");
        assert_eq!(stored.priority(), Priority::Urgent);
        assert_eq!(stored.assignee(), Some(engineer));
        assert_eq!(stored.estimate_minutes(), Some(45));
        assert_eq!(stored.schedule().due_on(), Some(day(13)));
        assert_eq!(stored.created_at(), start_of_day());
        assert_eq!(stored.updated_at(), start_of_day() + TimeDelta::hours(2));
    }

    #[test]
    fn a_task_can_be_given_to_a_member_of_the_team_and_then_to_the_manager() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let engineer = Assignee::Member(workspace.member(&project, "Hardware engineer").id());
        let task = workspace.task(&project, "Draw the schematic");

        let given = workspace.tasks.assign(task.id(), Some(engineer)).unwrap();
        assert_eq!(given.assignee(), Some(engineer));

        let taken_back = workspace
            .tasks
            .assign(task.id(), Some(Assignee::Manager))
            .unwrap();
        assert_eq!(taken_back.assignee(), Some(Assignee::Manager));
    }

    #[test]
    fn a_task_cannot_be_given_to_a_member_of_the_team_of_another_project() {
        let workspace = workspace();
        let sensor = workspace.project("Smart sensor node");
        let film = workspace.project("Product film");
        let editor = Assignee::Member(workspace.member(&film, "Video editor").id());
        let task = workspace.task(&sensor, "Draw the schematic");

        let on_assigning = workspace.tasks.assign(task.id(), Some(editor)).unwrap_err();
        let on_opening = workspace
            .tasks
            .create(NewTask {
                assignee: Some(editor),
                ..NewTask::titled(sensor.id(), "Route the board")
            })
            .unwrap_err();

        for error in [on_assigning, on_opening] {
            assert_eq!(error.code(), "assignee_from_another_team");
        }
        assert_eq!(workspace.tasks.get(task.id()).unwrap().assignee(), None);
        assert_eq!(
            workspace.tasks.list_for_project(sensor.id()).unwrap().len(),
            1
        );
    }

    #[test]
    fn a_task_cannot_be_given_to_a_member_that_does_not_exist() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace.task(&project, "Draw the schematic");
        let nobody = Assignee::Member(MemberId::generate());

        let error = workspace.tasks.assign(task.id(), Some(nobody)).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "team member",
                ..
            }
        ));
    }

    #[test]
    fn a_task_can_be_unassigned_and_its_estimate_cleared() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace
            .tasks
            .create(NewTask {
                assignee: Some(Assignee::Manager),
                estimate_minutes: Some(30),
                ..NewTask::titled(project.id(), "Order the samples")
            })
            .unwrap();

        workspace.tasks.assign(task.id(), None).unwrap();
        workspace.tasks.estimate(task.id(), None).unwrap();

        let stored = workspace.tasks.get(task.id()).unwrap();
        assert_eq!(stored.assignee(), None);
        assert_eq!(stored.estimate_minutes(), None);
    }

    #[test]
    fn a_task_can_be_moved_to_the_top_the_middle_or_the_bottom_of_a_status() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let in_progress = status_in(&project, StatusCategory::Active);
        for title in [
            "Top",
            "Middle",
            "Bottom",
            "Already there A",
            "Already there B",
        ] {
            workspace.task(&project, title);
        }
        let task_titled = |title: &str| {
            workspace
                .tasks
                .list_for_project(project.id())
                .unwrap()
                .into_iter()
                .find(|task| task.title().as_str() == title)
                .unwrap()
        };
        workspace
            .tasks
            .move_to(task_titled("Already there A").id(), in_progress, 0)
            .unwrap();
        workspace
            .tasks
            .move_to(task_titled("Already there B").id(), in_progress, 1)
            .unwrap();

        workspace
            .tasks
            .move_to(task_titled("Top").id(), in_progress, 0)
            .unwrap();
        workspace
            .tasks
            .move_to(task_titled("Middle").id(), in_progress, 2)
            .unwrap();
        workspace
            .tasks
            .move_to(task_titled("Bottom").id(), in_progress, 4)
            .unwrap();

        assert_eq!(
            workspace.titles_in(&project, in_progress),
            [
                "Top",
                "Already there A",
                "Middle",
                "Already there B",
                "Bottom"
            ]
        );
    }

    #[test]
    fn a_position_past_the_end_means_the_bottom() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let todo = status_in(&project, StatusCategory::Todo);
        let first = workspace.task(&project, "First");
        workspace.task(&project, "Second");

        workspace.tasks.move_to(first.id(), todo, 99).unwrap();

        assert_eq!(workspace.titles_in(&project, todo), ["Second", "First"]);
    }

    #[test]
    fn a_task_can_be_reordered_within_its_status() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let todo = status_in(&project, StatusCategory::Todo);
        workspace.task(&project, "First");
        workspace.task(&project, "Second");
        let third = workspace.task(&project, "Third");

        workspace.tasks.move_to(third.id(), todo, 0).unwrap();

        assert_eq!(
            workspace.titles_in(&project, todo),
            ["Third", "First", "Second"]
        );
    }

    #[test]
    fn a_position_in_a_status_does_not_count_the_subtasks_there() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let todo = status_in(&project, StatusCategory::Todo);
        let first = workspace.task(&project, "First");
        // The subtask is in the same status and ranked between the two
        // tasks, but a board shows it under "First", not in the column.
        workspace
            .tasks
            .create(NewTask {
                parent_id: Some(first.id()),
                ..NewTask::titled(project.id(), "A part of the first")
            })
            .unwrap();
        workspace.task(&project, "Second");
        let third = workspace.task(&project, "Third");

        workspace.tasks.move_to(third.id(), todo, 1).unwrap();

        let on_their_own: Vec<String> = workspace
            .tasks
            .list_for_project(project.id())
            .unwrap()
            .into_iter()
            .filter(|task| task.parent_id().is_none())
            .map(|task| task.title().to_string())
            .collect();
        assert_eq!(on_their_own, ["First", "Third", "Second"]);
    }

    #[test]
    fn a_subtask_is_placed_among_the_other_subtasks_of_its_parent() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let todo = status_in(&project, StatusCategory::Todo);
        let parent = workspace.task(&project, "Draw the schematic");
        workspace.task(&project, "Route the board");
        let part = |title: &str| {
            workspace
                .tasks
                .create(NewTask {
                    parent_id: Some(parent.id()),
                    ..NewTask::titled(project.id(), title)
                })
                .unwrap()
        };
        part("Power supply");
        part("Connectors");
        let last = part("Clock");

        workspace.tasks.move_to(last.id(), todo, 0).unwrap();

        let parts: Vec<String> = workspace
            .tasks
            .list_for_project(project.id())
            .unwrap()
            .into_iter()
            .filter(|task| task.parent_id() == Some(parent.id()))
            .map(|task| task.title().to_string())
            .collect();
        assert_eq!(parts, ["Clock", "Power supply", "Connectors"]);
    }

    #[test]
    fn moving_a_task_to_a_done_status_completes_it_and_moving_it_back_reopens_it() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace.task(&project, "Order the samples");
        workspace.clock.advance(TimeDelta::hours(3));

        let completed = workspace
            .tasks
            .move_to(task.id(), status_in(&project, StatusCategory::Done), 0)
            .unwrap();
        let reopened = workspace
            .tasks
            .move_to(task.id(), status_in(&project, StatusCategory::Active), 0)
            .unwrap();

        assert_eq!(
            completed.completed_at(),
            Some(start_of_day() + TimeDelta::hours(3))
        );
        assert!(!reopened.is_complete());
    }

    #[test]
    fn a_task_cannot_be_moved_to_a_status_of_another_project() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let other_project = workspace.project("Display driver");
        let task = workspace.task(&project, "Order the samples");

        let error = workspace
            .tasks
            .move_to(
                task.id(),
                status_in(&other_project, StatusCategory::Done),
                0,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::UnknownStatus { .. })
        ));
    }

    #[test]
    fn the_order_survives_more_insertions_at_one_spot_than_ranks_can_hold() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let todo = status_in(&project, StatusCategory::Todo);
        for title in ["First", "Second", "Third"] {
            workspace.task(&project, title);
        }
        // Every insertion between "Second" and its lower neighbour halves the
        // distance between their ranks. After about fifty halvings no number
        // is left between them and the status has to be spaced out again.
        let insertions = 80;
        let mut expected = vec!["First".to_owned(), "Second".to_owned()];
        let mut inserted_titles = Vec::new();

        for number in 0..insertions {
            let title = format!("Inserted {number}");
            let task = workspace.task(&project, &title);
            workspace.tasks.move_to(task.id(), todo, 2).unwrap();
            inserted_titles.push(title);
        }

        expected.extend(inserted_titles.into_iter().rev());
        expected.push("Third".to_owned());
        assert_eq!(workspace.titles_in(&project, todo), expected);

        let ranks: Vec<Rank> = workspace
            .tasks
            .list_for_project(project.id())
            .unwrap()
            .iter()
            .map(Task::rank)
            .collect();
        assert!(
            ranks.windows(2).all(|pair| pair[0] < pair[1]),
            "ranks must be distinct"
        );
    }

    #[test]
    fn a_task_can_wait_for_another_task_and_stop_waiting() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let hardware = workspace.task(&project, "Design the board");
        let firmware = workspace.task(&project, "Write the firmware");

        let waiting = workspace
            .tasks
            .add_dependency(firmware.id(), hardware.id())
            .unwrap();
        let released = workspace
            .tasks
            .remove_dependency(firmware.id(), hardware.id())
            .unwrap();

        assert!(waiting.blocked_by().contains(&hardware.id()));
        assert!(released.blocked_by().is_empty());
        assert_eq!(workspace.tasks.get(firmware.id()).unwrap(), released);
    }

    #[test]
    fn a_task_cannot_wait_for_itself() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace.task(&project, "Design the board");

        let error = workspace
            .tasks
            .add_dependency(task.id(), task.id())
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::SelfDependency)
        ));
    }

    #[test]
    fn a_dependency_that_would_close_a_loop_is_refused_by_name() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let hardware = workspace.task(&project, "Design the board");
        let firmware = workspace.task(&project, "Write the firmware");
        let enclosure = workspace.task(&project, "Design the enclosure");
        workspace
            .tasks
            .add_dependency(firmware.id(), hardware.id())
            .unwrap();
        workspace
            .tasks
            .add_dependency(enclosure.id(), firmware.id())
            .unwrap();

        let error = workspace
            .tasks
            .add_dependency(hardware.id(), enclosure.id())
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "SSN-1 cannot wait for SSN-3, because SSN-3 already waits for SSN-1"
        );
        assert_eq!(error.code(), "dependency_cycle");
        assert!(
            workspace
                .tasks
                .get(hardware.id())
                .unwrap()
                .blocked_by()
                .is_empty()
        );
    }

    #[test]
    fn a_part_cannot_wait_for_the_task_it_is_a_part_of() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let board = workspace.task(&project, "Design the board");
        let schematic = workspace
            .tasks
            .create(NewTask {
                parent_id: Some(board.id()),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap();

        let error = workspace
            .tasks
            .add_dependency(schematic.id(), board.id())
            .unwrap_err();

        // The board waits for its part, so the part cannot wait for it.
        assert_eq!(
            error.to_string(),
            "SSN-2 cannot wait for SSN-1, because SSN-1 already waits for SSN-2"
        );
        assert_eq!(error.code(), "dependency_cycle");
    }

    /// A project with a board of two parts and a task of its own after it.
    fn board_with_parts(workspace: &Workspace) -> (Project, Task, Task, Task, Task) {
        let project = workspace.project("Smart sensor node");
        let board = workspace.task(&project, "Design the board");
        let part = |title: &str, whole: &Task| {
            workspace
                .tasks
                .create(NewTask {
                    parent_id: Some(whole.id()),
                    ..NewTask::titled(project.id(), title)
                })
                .unwrap()
        };
        let schematic = part("Draw the schematic", &board);
        let symbols = part("Draw the symbols", &schematic);
        let manual = workspace.task(&project, "Write the manual");
        (project, board, schematic, symbols, manual)
    }

    #[test]
    fn a_task_moved_under_another_takes_its_parts_into_its_area_and_its_sprint() {
        let workspace = workspace();
        let (project, board, schematic, symbols, manual) = board_with_parts(&workspace);
        let documents = workspace.areas.add(project.id(), "Documents").unwrap();
        workspace
            .areas
            .move_task(manual.id(), documents.id())
            .unwrap();
        let sprint = workspace
            .sprints
            .create(crate::sprints::NewSprint::in_project(project.id()))
            .unwrap();
        workspace
            .tasks
            .plan(manual.id(), Some(sprint.id()))
            .unwrap();
        workspace.clock.advance(TimeDelta::hours(1));

        let moved = workspace
            .tasks
            .make_part_of(schematic.id(), Some(manual.id()))
            .unwrap();

        assert_eq!(moved.parent_id(), Some(manual.id()));
        for task in [moved, workspace.tasks.get(symbols.id()).unwrap()] {
            assert_eq!(task.area_id(), documents.id());
            assert_eq!(task.sprint_id(), Some(sprint.id()));
        }
        assert_eq!(
            workspace.tasks.get(symbols.id()).unwrap().parent_id(),
            Some(schematic.id()),
            "its own parts stay its parts"
        );
        let history = workspace.tasks.history(schematic.id()).unwrap();
        assert!(history.iter().any(|change| change.kind
            == teamwright_domain::TaskChangeKind::MadePartOf {
                from: Some(board.id()),
                to: Some(manual.id()),
            }));
    }

    #[test]
    fn a_part_made_a_task_of_its_own_keeps_its_area_and_its_sprint() {
        let workspace = workspace();
        let (project, board, schematic, symbols, _) = board_with_parts(&workspace);
        let sprint = workspace
            .sprints
            .create(crate::sprints::NewSprint::in_project(project.id()))
            .unwrap();
        workspace.tasks.plan(board.id(), Some(sprint.id())).unwrap();

        let alone = workspace.tasks.make_part_of(schematic.id(), None).unwrap();

        assert_eq!(alone.parent_id(), None);
        assert_eq!(alone.sprint_id(), Some(sprint.id()));
        assert_eq!(alone.area_id(), board.area_id());
        assert_eq!(
            workspace.tasks.get(symbols.id()).unwrap().parent_id(),
            Some(schematic.id())
        );
        // It can now be planned by itself.
        workspace.tasks.plan(schematic.id(), None).unwrap();
    }

    #[test]
    fn a_task_is_not_made_a_part_of_itself_or_of_its_own_parts() {
        let workspace = workspace();
        let (_, board, _, symbols, _) = board_with_parts(&workspace);

        let itself = workspace
            .tasks
            .make_part_of(board.id(), Some(board.id()))
            .unwrap_err();
        assert_eq!(itself.to_string(), "SSN-1 cannot be a part of itself");
        assert_eq!(itself.code(), "part_in_circle");

        let below = workspace
            .tasks
            .make_part_of(board.id(), Some(symbols.id()))
            .unwrap_err();
        assert_eq!(
            below.to_string(),
            "SSN-1 cannot be a part of SSN-3, which is a part of SSN-1 already"
        );
        assert_eq!(workspace.tasks.get(board.id()).unwrap().parent_id(), None);
    }

    #[test]
    fn a_task_is_not_made_a_part_of_a_task_it_waits_for() {
        let workspace = workspace();
        let (_, board, schematic, symbols, manual) = board_with_parts(&workspace);
        workspace
            .tasks
            .add_dependency(manual.id(), board.id())
            .unwrap();
        workspace
            .tasks
            .add_dependency(symbols.id(), manual.id())
            .unwrap_err();

        let error = workspace
            .tasks
            .make_part_of(manual.id(), Some(schematic.id()))
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "SSN-4 cannot be a part of SSN-2, because SSN-4 or one of its parts waits for SSN-2, \
             and a task with parts starts only once its parts are finished"
        );
        assert_eq!(error.code(), "part_in_circle");
    }

    #[test]
    fn a_task_is_not_made_a_part_of_a_task_of_another_project() {
        let workspace = workspace();
        let (_, board, ..) = board_with_parts(&workspace);
        let other = workspace.project("Display driver");
        let driver = workspace.task(&other, "Write the driver");

        let error = workspace
            .tasks
            .make_part_of(board.id(), Some(driver.id()))
            .unwrap_err();

        assert!(matches!(error, ApplicationError::DifferentProjects));
    }

    #[test]
    fn moving_a_task_where_it_is_changes_nothing() {
        let workspace = workspace();
        let (_, board, schematic, ..) = board_with_parts(&workspace);
        workspace.clock.advance(TimeDelta::hours(1));

        let same = workspace
            .tasks
            .make_part_of(schematic.id(), Some(board.id()))
            .unwrap();

        assert_eq!(same, schematic);
    }

    #[test]
    fn tasks_of_different_projects_cannot_wait_for_each_other() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let other_project = workspace.project("Display driver");
        let firmware = workspace.task(&project, "Write the firmware");
        let driver = workspace.task(&other_project, "Write the driver");

        let error = workspace
            .tasks
            .add_dependency(firmware.id(), driver.id())
            .unwrap_err();

        assert!(matches!(error, ApplicationError::DifferentProjects));
    }

    #[test]
    fn deleting_a_task_removes_its_subtasks_at_every_depth() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let board = workspace.task(&project, "Design the board");
        let subtask_of = |parent: &Task, title: &str| {
            workspace
                .tasks
                .create(NewTask {
                    parent_id: Some(parent.id()),
                    ..NewTask::titled(project.id(), title)
                })
                .unwrap()
        };
        let schematic = subtask_of(&board, "Draw the schematic");
        subtask_of(&schematic, "Draw the power stage");
        workspace.task(&project, "Write the firmware");

        workspace.tasks.delete(board.id()).unwrap();

        let remaining: Vec<String> = workspace
            .tasks
            .list_for_project(project.id())
            .unwrap()
            .iter()
            .map(|task| task.title().to_string())
            .collect();
        assert_eq!(remaining, ["Write the firmware"]);
    }

    #[test]
    fn deleting_a_task_releases_the_tasks_that_waited_for_it() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let hardware = workspace.task(&project, "Design the board");
        let parts = workspace.task(&project, "Order the parts");
        let firmware = workspace.task(&project, "Write the firmware");
        workspace
            .tasks
            .add_dependency(firmware.id(), hardware.id())
            .unwrap();
        workspace
            .tasks
            .add_dependency(firmware.id(), parts.id())
            .unwrap();

        workspace.tasks.delete(hardware.id()).unwrap();

        let firmware = workspace.tasks.get(firmware.id()).unwrap();
        assert_eq!(
            firmware.blocked_by().iter().copied().collect::<Vec<_>>(),
            [parts.id()]
        );
    }

    #[test]
    fn deleting_a_task_that_does_not_exist_is_reported() {
        let error = workspace().tasks.delete(TaskId::generate()).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "task", .. }
        ));
    }

    #[test]
    fn listing_the_tasks_of_a_project_that_does_not_exist_is_reported() {
        let error = workspace()
            .tasks
            .list_for_project(ProjectId::generate())
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "project",
                ..
            }
        ));
    }

    #[test]
    fn the_history_of_a_task_tells_what_happened_to_it_the_oldest_first() {
        use teamwright_domain::TaskChangeKind;

        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let active = status_in(&project, StatusCategory::Active);
        let task = workspace.task(&project, "Draw the schematic");
        workspace.clock.advance(TimeDelta::hours(1));
        workspace
            .tasks
            .prioritize(task.id(), Priority::Urgent)
            .unwrap();
        workspace.clock.advance(TimeDelta::hours(1));
        workspace.tasks.move_to(task.id(), active, 0).unwrap();

        let history = workspace.tasks.history(task.id()).unwrap();

        assert_eq!(
            history,
            [
                TaskChange {
                    at: start_of_day(),
                    kind: TaskChangeKind::Created
                },
                TaskChange {
                    at: start_of_day() + TimeDelta::hours(1),
                    kind: TaskChangeKind::Prioritized {
                        from: Priority::Normal,
                        to: Priority::Urgent
                    }
                },
                TaskChange {
                    at: start_of_day() + TimeDelta::hours(2),
                    kind: TaskChangeKind::Moved {
                        from: task.status_id(),
                        to: active
                    }
                },
            ]
        );
    }

    #[test]
    fn the_history_lists_what_happens_to_a_task_through_another_task() {
        use teamwright_domain::TaskChangeKind;

        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let schematic = workspace.task(&project, "Draw the schematic");
        let board = workspace.task(&project, "Route the board");
        workspace
            .tasks
            .add_dependency(board.id(), schematic.id())
            .unwrap();

        workspace.tasks.delete(schematic.id()).unwrap();

        let kinds: Vec<_> = workspace
            .tasks
            .history(board.id())
            .unwrap()
            .into_iter()
            .map(|change| change.kind)
            .collect();
        assert_eq!(
            kinds,
            [
                TaskChangeKind::Created,
                TaskChangeKind::NowWaitsFor {
                    blocker: schematic.id()
                },
                TaskChangeKind::NoLongerWaitsFor {
                    blocker: schematic.id()
                },
            ]
        );
    }

    #[test]
    fn the_actual_dates_of_a_task_are_read_from_its_versions_and_compared_with_its_plan() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let active = status_in(&project, StatusCategory::Active);
        let done = status_in(&project, StatusCategory::Done);
        let late = workspace.task(&project, "Draw the schematic");
        let unplanned = workspace.task(&project, "Order the parts");
        workspace
            .tasks
            .reschedule(late.id(), Some(day(10)), Some(day(11)))
            .unwrap();
        workspace.tasks.move_to(late.id(), active, 0).unwrap();
        workspace.clock.advance(TimeDelta::days(2));
        workspace.tasks.move_to(unplanned.id(), done, 0).unwrap();

        let actuals = workspace.tasks.actual_dates(project.id()).unwrap();

        assert_eq!(actuals.len(), 2);
        let (late_one, unplanned_one) = (actuals[0], actuals[1]);
        assert_eq!(late_one.task_id, late.id());
        assert_eq!(late_one.started_on, Some(day(11)));
        assert_eq!(late_one.finished_on, None);
        assert_eq!(late_one.timing.started_late, Some(1));
        assert_eq!(late_one.timing.running_late, Some(2));
        assert_eq!(unplanned_one.started_on, Some(day(13)));
        assert_eq!(unplanned_one.finished_on, Some(day(13)));
        assert_eq!(unplanned_one.timing, Timing::default());
    }

    #[test]
    fn the_history_of_a_task_that_does_not_exist_is_reported() {
        let error = workspace().tasks.history(TaskId::generate()).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "task", .. }
        ));
    }

    /// The kind of each change of a project, with the title of its task.
    fn titled_kinds(changes: &[ProjectChange]) -> Vec<(String, &'static str)> {
        use teamwright_domain::TaskChangeKind;

        changes
            .iter()
            .map(|change| {
                let kind = match change.change.kind {
                    TaskChangeKind::Created => "created",
                    TaskChangeKind::Renamed { .. } => "renamed",
                    TaskChangeKind::Prioritized { .. } => "prioritized",
                    TaskChangeKind::Moved { .. } => "moved",
                    _ => "other",
                };
                (change.task_title.to_string(), kind)
            })
            .collect()
    }

    #[test]
    fn the_history_of_a_project_tells_what_happened_to_its_tasks_the_newest_first() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let other = workspace.project("Weather station");
        let schematic = workspace.task(&project, "Draw the schematic");
        workspace.task(&other, "Pick a sensor");
        workspace.clock.advance(TimeDelta::hours(1));
        let board = workspace.task(&project, "Route the board");
        workspace.clock.advance(TimeDelta::hours(1));
        workspace
            .tasks
            .prioritize(schematic.id(), Priority::Urgent)
            .unwrap();

        let history = workspace
            .tasks
            .project_history(project.id(), 50, None)
            .unwrap();

        assert_eq!(
            titled_kinds(&history),
            [
                ("Draw the schematic".to_owned(), "prioritized"),
                ("Route the board".to_owned(), "created"),
                ("Draw the schematic".to_owned(), "created"),
            ]
        );
        assert_eq!(history[1].task_id, board.id());
        assert_eq!(history[1].task_number, board.number());
        assert!(history.iter().all(|change| !change.task_gone));
    }

    #[test]
    fn a_task_deleted_since_keeps_its_number_and_title_and_is_marked_as_gone() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let schematic = workspace.task(&project, "Draw the schematic");
        workspace.clock.advance(TimeDelta::hours(1));
        workspace
            .tasks
            .rename(schematic.id(), "Draw the schematic again")
            .unwrap();
        workspace.tasks.delete(schematic.id()).unwrap();

        let history = workspace
            .tasks
            .project_history(project.id(), 50, None)
            .unwrap();

        assert_eq!(history.len(), 2);
        assert!(history.iter().all(|change| change.task_gone));
        assert!(history.iter().all(|change| {
            change.task_number == schematic.number()
                && change.task_title.as_str() == "Draw the schematic again"
        }));
    }

    #[test]
    fn the_history_of_a_project_is_read_a_page_at_a_time() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        for title in ["One", "Two", "Three", "Four", "Five"] {
            workspace.task(&project, title);
            workspace.clock.advance(TimeDelta::minutes(1));
        }

        let first = workspace
            .tasks
            .project_history(project.id(), 2, None)
            .unwrap();
        let before = first.last().map(|change| change.change.at);
        let second = workspace
            .tasks
            .project_history(project.id(), 2, before)
            .unwrap();
        let before = second.last().map(|change| change.change.at);
        let third = workspace
            .tasks
            .project_history(project.id(), 2, before)
            .unwrap();

        let titles = |page: &[ProjectChange]| -> Vec<String> {
            page.iter()
                .map(|change| change.task_title.to_string())
                .collect()
        };
        assert_eq!(titles(&first), ["Five", "Four"]);
        assert_eq!(titles(&second), ["Three", "Two"]);
        assert_eq!(titles(&third), ["One"]);
    }

    #[test]
    fn a_page_does_not_split_the_changes_of_one_moment() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        workspace.task(&project, "Early");
        workspace.clock.advance(TimeDelta::minutes(1));
        for title in ["One", "Two", "Three"] {
            workspace.task(&project, title);
        }

        let page = workspace
            .tasks
            .project_history(project.id(), 2, None)
            .unwrap();
        let all_at_once = workspace
            .tasks
            .project_history(project.id(), 1, None)
            .unwrap();

        // The three made at one moment do not fit in a page of two: the page
        // of two would be empty, so it takes all three.
        assert_eq!(page.len(), 3);
        assert_eq!(all_at_once.len(), 3);
        let rest = workspace
            .tasks
            .project_history(project.id(), 2, page.last().map(|change| change.change.at))
            .unwrap();
        assert_eq!(titled_kinds(&rest), [("Early".to_owned(), "created")]);
    }

    #[test]
    fn a_page_has_at_most_two_hundred_changes_and_none_when_none_are_asked_for() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let task = workspace.task(&project, "Draw the schematic");
        for minute in 0..210 {
            workspace.clock.advance(TimeDelta::minutes(1));
            let priority = if minute % 2 == 0 {
                Priority::Urgent
            } else {
                Priority::Normal
            };
            workspace.tasks.prioritize(task.id(), priority).unwrap();
        }

        let page = workspace
            .tasks
            .project_history(project.id(), 1000, None)
            .unwrap();
        let none = workspace
            .tasks
            .project_history(project.id(), 0, None)
            .unwrap();

        assert_eq!(page.len(), MOST_PROJECT_CHANGES);
        assert!(none.is_empty());
    }

    #[test]
    fn the_history_of_a_project_that_does_not_exist_is_reported() {
        let error = workspace()
            .tasks
            .project_history(ProjectId::generate(), 10, None)
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "project",
                ..
            }
        ));
    }
}
