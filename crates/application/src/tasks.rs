//! Use cases for tasks.

use std::collections::BTreeSet;
use std::sync::Arc;

use teamwright_domain::{
    Assignee, CalendarDate, DependencyGraph, Priority, Project, ProjectId, Rank, Schedule,
    SprintId, Status, StatusId, StoryPoints, Task, TaskChange, TaskDraft, TaskId, TaskTitle,
    Timestamp, history_of,
};

use crate::error::ApplicationError;
use crate::ports::{
    Clock, ConversationRepository, MemberRepository, ProjectRepository, SprintRepository,
    TaskRepository,
};
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
    clock: Arc<dyn Clock>,
}

impl TaskService {
    /// Creates the service. The conversations are needed because the record
    /// of the work on a task goes with the task, and a task a specialist is
    /// working on is not removed. The sprints are needed to plan a task
    /// into one.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        members: Arc<dyn MemberRepository>,
        conversations: Arc<dyn ConversationRepository>,
        sprints: Arc<dyn SprintRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            members,
            conversations,
            sprints,
            clock,
        }
    }

    /// Opens a task at the bottom of its status. It starts in the backlog
    /// without points, or, as a subtask, in the sprint of the task it is a
    /// part of.
    ///
    /// # Errors
    ///
    /// Returns an error if the project, the status, the parent task or the
    /// assignee does not exist, if the parent or the assignee belongs to
    /// another project, if the title or the dates are not valid, or if
    /// storage fails.
    pub fn create(&self, new_task: NewTask) -> Result<Task, ApplicationError> {
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

    /// Makes a task wait until another task is finished.
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
        let dependencies = DependencyGraph::from_tasks(&tasks_of_project);
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
        let clock = Arc::new(ManualClock::starting_at(start_of_day()));
        Workspace {
            projects: ProjectService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                Arc::new(InMemoryPlacesRepository::default()),
                Arc::new(InMemorySprintRepository::new(task_repository.clone())),
                clock.clone(),
            ),
            tasks: TaskService::new(
                project_repository.clone(),
                task_repository.clone(),
                member_repository.clone(),
                conversation_repository.clone(),
                Arc::new(InMemorySprintRepository::new(task_repository.clone())),
                clock.clone(),
            ),
            team: TeamService::new(
                project_repository,
                member_repository,
                task_repository,
                conversation_repository,
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
    fn the_history_of_a_task_that_does_not_exist_is_reported() {
        let error = workspace().tasks.history(TaskId::generate()).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "task", .. }
        ));
    }
}
