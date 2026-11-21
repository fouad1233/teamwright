//! Use cases for projects.

use std::cmp::Reverse;
use std::sync::Arc;

use teamwright_domain::{
    Area, Color, Project, ProjectId, ProjectKey, ProjectName, ProjectTemplate, Rank, Sprint,
    SprintDates, SprintName, Status, StatusCategory, StatusId, StatusLoad, StatusName, Task, View,
    WorkInProgressLimit, template,
};

use crate::error::ApplicationError;
use crate::ports::{
    AreaRepository, Clock, ConversationRepository, MemberRepository, PlacesRepository,
    ProjectRepository, SprintRepository, TaskRepository, ViewRepository,
};
use crate::work::WorkInProgress;

/// The templates a new project can start from (see [`ProjectTemplate`]).
/// The name is older than templates: a template decides more than the
/// workflow now, also the views, the areas and a first sprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkflowTemplate {
    /// To do, in progress, in review, done, and the six usual views.
    #[default]
    Standard,
    /// To do, in progress, in revision, in test, done.
    Engineering,
    /// A flow with work-in-progress limits and no sprints.
    Kanban,
    /// Sprints from a backlog, the backlog first, a first sprint planned.
    Scrum,
    /// Design, review, order, build and test, in areas for the parts of a
    /// device.
    HardwareProduct,
}

impl WorkflowTemplate {
    /// What the template decides.
    #[must_use]
    pub const fn template(self) -> &'static ProjectTemplate {
        match self {
            Self::Standard => &template::STANDARD,
            Self::Engineering => &template::ENGINEERING,
            Self::Kanban => &template::KANBAN,
            Self::Scrum => &template::SCRUM,
            Self::HardwareProduct => &template::HARDWARE_PRODUCT,
        }
    }

    /// Every template, in the order of [`ProjectTemplate::ALL`].
    pub const ALL: [Self; 5] = [
        Self::Standard,
        Self::Kanban,
        Self::Scrum,
        Self::HardwareProduct,
        Self::Engineering,
    ];
}

/// What is needed to start a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewProject {
    /// The name of the project.
    pub name: String,
    /// The prefix for task numbers. If it is missing or blank, one is
    /// derived from the name.
    pub key: Option<String>,
    /// A free-form description. May be empty.
    pub description: String,
    /// The workflow the project starts with.
    pub workflow: WorkflowTemplate,
}

/// What a manager decides about a status: its name, what it means for the
/// progress of a task, and its color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProperties {
    /// The name, before it is tidied.
    pub name: String,
    /// What the status means for the progress of a task.
    pub category: StatusCategory,
    /// The color it is shown in.
    pub color: Color,
}

/// The use cases that start, find, change and remove projects and their
/// workflows.
#[derive(Clone)]
pub struct ProjectService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    members: Arc<dyn MemberRepository>,
    conversations: Arc<dyn ConversationRepository>,
    places: Arc<dyn PlacesRepository>,
    sprints: Arc<dyn SprintRepository>,
    views: Arc<dyn ViewRepository>,
    areas: Arc<dyn AreaRepository>,
    clock: Arc<dyn Clock>,
}

impl ProjectService {
    /// Creates the service. The tasks, the members, the conversations, the
    /// folders, the sprints and the views are needed because removing a
    /// project removes them too, changing the workflow can move or complete
    /// tasks, and a project in which a specialist works is neither put away
    /// nor removed. The views and the areas are also needed because a new
    /// project starts with six views and the area "General".
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "a project owns this many kinds of stored things"
    )]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        members: Arc<dyn MemberRepository>,
        conversations: Arc<dyn ConversationRepository>,
        places: Arc<dyn PlacesRepository>,
        sprints: Arc<dyn SprintRepository>,
        views: Arc<dyn ViewRepository>,
        areas: Arc<dyn AreaRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            members,
            conversations,
            places,
            sprints,
            views,
            areas,
            clock,
        }
    }

    /// Starts a project from its template: the statuses of its workflow
    /// with their limits, its views in their order, its areas, and a first
    /// sprint if the template plans one. The standard template gives the six
    /// views every project started with before templates, and the one area
    /// "General". No template gives a team: the Advisor proposes one.
    ///
    /// The project does not remember its template. Everything the template
    /// made can be changed like anything else.
    ///
    /// # Errors
    ///
    /// Returns an error if the name or the key is not valid, if the requested
    /// key belongs to another project, or if storage fails.
    pub fn create(&self, new_project: NewProject) -> Result<Project, ApplicationError> {
        let name = ProjectName::parse(&new_project.name)?;
        let existing = self.projects.list()?;

        let requested_key = new_project
            .key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty());
        let key = match requested_key {
            Some(requested) => {
                let key = ProjectKey::parse(requested)?;
                if is_key_in_use(&existing, &key) {
                    return Err(ApplicationError::ProjectKeyTaken {
                        key: key.to_string(),
                    });
                }
                key
            }
            None => Self::unused_key_for(&name, &existing),
        };

        let template = new_project.workflow.template();
        let now = self.clock.now();
        let mut project = Project::create(
            name,
            key,
            new_project.description.trim().to_owned(),
            template.workflow(),
            now,
        );
        let first_sprint = if template.plans_first_sprint {
            let number = project.allocate_sprint_number();
            Some(Sprint::plan(
                project.id(),
                SprintName::numbered(number),
                "",
                SprintDates::new(None, None)?,
                now,
            ))
        } else {
            None
        };
        // The project is stored first: its views, its areas and its sprint
        // refer to it.
        self.projects.save(&project)?;
        self.views
            .save_all(&View::starting_with(project.id(), template.views))?;
        self.areas
            .save_all(&Area::starting_with(project.id(), &template.area_names()))?;
        if let Some(sprint) = first_sprint {
            self.sprints.save(&sprint, &[])?;
        }
        Ok(project)
    }

    /// Lists the projects that are not archived, oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn list(&self) -> Result<Vec<Project>, ApplicationError> {
        let projects = self.projects.list()?;
        Ok(projects
            .into_iter()
            .filter(|project| !project.is_archived())
            .collect())
    }

    /// Finds a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn get(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Gives a project a new name.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not valid, if there is no such
    /// project, or if storage fails.
    pub fn rename(&self, id: ProjectId, name: &str) -> Result<Project, ApplicationError> {
        let name = ProjectName::parse(name)?;
        self.change(id, |project| {
            project.rename(name);
            Ok(())
        })
    }

    /// Replaces the description of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn describe(&self, id: ProjectId, description: &str) -> Result<Project, ApplicationError> {
        self.change(id, |project| {
            project.describe(description.trim().to_owned());
            Ok(())
        })
    }

    /// Puts a project away. Its tasks are kept.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if a specialist is
    /// working in it, or if storage fails.
    pub fn archive(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        let project = self.get(id)?;
        self.refuse_while_worked_on(&project)?;
        let now = self.clock.now();
        self.change(id, |project| {
            project.archive(now);
            Ok(())
        })
    }

    /// Lists the projects that were put away, the one put away last first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn list_archived(&self) -> Result<Vec<Project>, ApplicationError> {
        let mut archived: Vec<Project> = self
            .projects
            .list()?
            .into_iter()
            .filter(Project::is_archived)
            .collect();
        // The sort is stable, so projects archived at the same moment stay
        // oldest first.
        archived.sort_by_key(|project| Reverse(project.archived_at()));
        Ok(archived)
    }

    /// Brings an archived project back. Its tasks and its team were kept.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn restore(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.change(id, |project| {
            project.unarchive();
            Ok(())
        })
    }

    /// Removes a project for good, with its tasks, its sprints, its views,
    /// its areas, its team, the conversations with its specialists and what it knew
    /// about its folders. The folders themselves stay where they are. Its
    /// key is free for a new project afterwards.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if a specialist is
    /// working in it, or if storage fails.
    pub fn delete(&self, id: ProjectId) -> Result<(), ApplicationError> {
        let project = self.get(id)?;
        let conversations = self.refuse_while_worked_on(&project)?;
        let conversation_ids: Vec<_> = conversations
            .iter()
            .map(|conversation| conversation.id())
            .collect();
        self.conversations.delete(&conversation_ids)?;
        let task_ids: Vec<_> = self
            .tasks
            .list_for_project(project.id())?
            .iter()
            .map(Task::id)
            .collect();
        self.tasks.delete(&task_ids)?;
        // The sprints go after their tasks, so that no task is left in a
        // sprint that is gone.
        for sprint in self.sprints.list_for_project(project.id())? {
            self.sprints.delete(sprint.id(), &[])?;
        }
        for view in self.views.list_for_project(project.id())? {
            self.views.delete(view.id())?;
        }
        // The areas go after the tasks that are in them.
        for area in self.areas.list_for_project(project.id())? {
            self.areas.delete(area.id())?;
        }
        for member in self.members.list_for_project(project.id())? {
            self.members.delete(member.id())?;
        }
        self.places.delete(project.id())?;
        // The project goes last. If anything fails before, it is still
        // there, and removing it again finishes the job.
        self.projects.delete(project.id())?;
        Ok(())
    }

    /// Adds a status to the workflow of a project. It goes behind the last
    /// status that is as far along in the course of the work, or less far.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name is not
    /// valid or another status has it, or if storage fails.
    pub fn add_status(
        &self,
        id: ProjectId,
        properties: StatusProperties,
    ) -> Result<Project, ApplicationError> {
        self.change(id, |project| {
            let name = StatusName::parse(&properties.name)?;
            project.add_status(Status::new(name, properties.category, properties.color))?;
            Ok(())
        })
    }

    /// Gives a status a new name, category and color in one step.
    ///
    /// When the status starts or stops being a "done" status, the tasks in
    /// it are completed or reopened, because whether a task is complete
    /// follows from the category of its status.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or status, if the name
    /// is not valid or another status has it, if the workflow would be left
    /// without a "todo" or a "done" status, or if storage fails.
    pub fn change_status(
        &self,
        id: ProjectId,
        status_id: StatusId,
        properties: StatusProperties,
    ) -> Result<Project, ApplicationError> {
        let mut project = self.get(id)?;
        let was_done =
            project.workflow().require_status(status_id)?.category() == StatusCategory::Done;
        let name = StatusName::parse(&properties.name)?;
        project.change_status(status_id, name, properties.category, properties.color)?;
        let changed = project.workflow().require_status(status_id)?.clone();

        if was_done != (changed.category() == StatusCategory::Done) {
            let now = self.clock.now();
            let mut affected = self.tasks_in_status(&project, status_id)?;
            for task in &mut affected {
                // Moving a task to the status it is in, at the rank it has,
                // sets its completion from the new category.
                let rank = task.rank();
                task.move_to(&changed, rank, now);
            }
            // The tasks are stored first. Should storing the project fail,
            // repeating the change finds the category still changing and
            // brings the tasks in line again.
            self.tasks.save_all(&affected)?;
        }

        self.projects.save(&project)?;
        Ok(project)
    }

    /// Moves a status of a workflow to another place. A place past the end
    /// means the last place.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or status, or if storage
    /// fails.
    pub fn move_status(
        &self,
        id: ProjectId,
        status_id: StatusId,
        index: usize,
    ) -> Result<Project, ApplicationError> {
        self.change(id, |project| {
            project.move_status(status_id, index)?;
            Ok(())
        })
    }

    /// Gives a status a work-in-progress limit, another one, or none.
    ///
    /// A limit is a signal, never a wall: setting one below the number of
    /// tasks already in the status is allowed, and nothing is moved.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or status, if the limit
    /// is not a whole number from 1 to 999, if the status is of the category
    /// done, or if storage fails.
    pub fn set_status_limit(
        &self,
        id: ProjectId,
        status_id: StatusId,
        limit: Option<i64>,
    ) -> Result<Project, ApplicationError> {
        let limit = limit.map(WorkInProgressLimit::new).transpose()?;
        self.change(id, |project| {
            project.set_status_limit(status_id, limit)?;
            Ok(())
        })
    }

    /// How many tasks stand in each status of a project, against its limit,
    /// in board order. Every task of the project counts, in every area and
    /// sprint; a subtask does not count apart from its task (see
    /// [`StatusLoad::of_workflow`]).
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn status_loads(&self, id: ProjectId) -> Result<Vec<StatusLoad>, ApplicationError> {
        let project = self.get(id)?;
        let tasks = self.tasks.list_for_project(project.id())?;
        Ok(StatusLoad::of_workflow(project.workflow(), &tasks))
    }

    /// Removes a status from a workflow.
    ///
    /// The tasks in it, subtasks included, go to the bottom of the status
    /// named by `move_tasks_to`, in their order, and are completed or
    /// reopened by its category. A status without tasks needs no
    /// destination.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or status, if the
    /// workflow would be left without a "todo" or a "done" status, if the
    /// status has tasks and no other status is named for them, if the named
    /// status is not part of the workflow, or if storage fails.
    pub fn remove_status(
        &self,
        id: ProjectId,
        status_id: StatusId,
        move_tasks_to: Option<StatusId>,
    ) -> Result<Project, ApplicationError> {
        let mut project = self.get(id)?;
        let mut moving = self.tasks_in_status(&project, status_id)?;
        let removed = project.remove_status(status_id)?;

        if !moving.is_empty() {
            let destination_id = move_tasks_to
                .filter(|&destination_id| destination_id != status_id)
                .ok_or_else(|| ApplicationError::StatusInUse {
                    name: removed.name().to_string(),
                    count: moving.len(),
                })?;
            let destination = project.workflow().require_status(destination_id)?.clone();
            let mut previous_rank = self
                .tasks_in_status(&project, destination_id)?
                .last()
                .map(Task::rank);
            let now = self.clock.now();
            for task in &mut moving {
                let rank = Rank::between(previous_rank, None);
                task.move_to(&destination, rank, now);
                previous_rank = Some(rank);
            }
            // The tasks leave before the status goes, so that no stored task
            // ever refers to a status that no longer exists.
            self.tasks.save_all(&moving)?;
        }

        self.projects.save(&project)?;
        Ok(project)
    }

    /// Refuses to go on while a specialist works in a project. Returns the
    /// conversations of the project otherwise.
    fn refuse_while_worked_on(
        &self,
        project: &Project,
    ) -> Result<Vec<teamwright_domain::Conversation>, ApplicationError> {
        let conversations = self.conversations.list_for_project(project.id())?;
        WorkInProgress {
            projects: self.projects.as_ref(),
            tasks: self.tasks.as_ref(),
            members: self.members.as_ref(),
            conversations: self.conversations.as_ref(),
        }
        .refuse_while_worked_on(&conversations)?;
        Ok(conversations)
    }

    /// The tasks of a project that are in one status, subtasks included, in
    /// the order a manager arranged them: by rank, and by number where ranks
    /// are equal.
    fn tasks_in_status(
        &self,
        project: &Project,
        status_id: StatusId,
    ) -> Result<Vec<Task>, ApplicationError> {
        let mut tasks: Vec<Task> = self
            .tasks
            .list_for_project(project.id())?
            .into_iter()
            .filter(|task| task.status_id() == status_id)
            .collect();
        tasks.sort_by_key(|task| (task.rank(), task.number()));
        Ok(tasks)
    }

    /// Loads a project, changes it and stores it. Nothing is stored if the
    /// change is refused.
    fn change(
        &self,
        id: ProjectId,
        apply: impl FnOnce(&mut Project) -> Result<(), ApplicationError>,
    ) -> Result<Project, ApplicationError> {
        let mut project = self.get(id)?;
        apply(&mut project)?;
        self.projects.save(&project)?;
        Ok(project)
    }

    /// Derives a key from a project name that no existing project uses.
    ///
    /// If the natural key is taken, a number is appended, and letters are
    /// dropped from the end as needed to stay within the length limit:
    /// `IOT`, `IOT2`, `IOT3`, …
    fn unused_key_for(name: &ProjectName, existing: &[Project]) -> ProjectKey {
        // With thousands of projects that share a key stem, the suggestion
        // comes back as it is. Task numbers stay usable even with a
        // repeated key.
        ProjectKey::suggest(name).first_free(|candidate| is_key_in_use(existing, candidate))
    }
}

/// Tells whether any project, archived or not, already has the key.
fn is_key_in_use(projects: &[Project], key: &ProjectKey) -> bool {
    projects.iter().any(|project| project.key() == key)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};
    use teamwright_domain::{DomainError, MemberKind, TaskId, Timestamp};

    use super::*;
    use crate::in_memory::InMemorySprintRepository;
    use crate::in_memory::{
        InMemoryConversationRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
        InMemoryProjectRepository, InMemoryTaskRepository, ManualClock,
    };
    use crate::tasks::{NewTask, TaskService};
    use crate::team::{NewMember, TeamService};

    /// The services under test, sharing storage and a clock that the test
    /// controls.
    struct Workspace {
        projects: ProjectService,
        tasks: TaskService,
        team: TeamService,
        views: Arc<crate::in_memory::InMemoryViewRepository>,
        areas: Arc<crate::in_memory::InMemoryAreaRepository>,
        sprints: Arc<InMemorySprintRepository>,
        clock: Arc<ManualClock>,
    }

    fn start_of_day() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn workspace() -> Workspace {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        let members = Arc::new(InMemoryMemberRepository::default());
        let conversations = Arc::new(InMemoryConversationRepository::default());
        let area_store = Arc::new(crate::in_memory::InMemoryAreaRepository::default());
        let view_store = Arc::new(crate::in_memory::InMemoryViewRepository::default());
        let sprint_store = Arc::new(InMemorySprintRepository::new(tasks.clone()));
        let clock = Arc::new(ManualClock::starting_at(start_of_day()));
        Workspace {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(InMemoryPlacesRepository::default()),
                sprint_store.clone(),
                view_store.clone(),
                area_store.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(InMemorySprintRepository::new(tasks.clone())),
                area_store.clone(),
                clock.clone(),
            ),
            team: TeamService::new(
                projects,
                members,
                tasks,
                conversations,
                Arc::new(crate::in_memory::InMemoryViewRepository::default()),
                clock.clone(),
            ),
            views: view_store,
            areas: area_store,
            sprints: sprint_store,
            clock,
        }
    }

    fn service() -> ProjectService {
        workspace().projects
    }

    impl Workspace {
        /// A project with the engineering workflow: to do, in progress, in
        /// revision, in test, done.
        fn engineering_project(&self, name: &str) -> Project {
            self.projects
                .create(NewProject {
                    workflow: WorkflowTemplate::Engineering,
                    ..named(name)
                })
                .unwrap()
        }

        fn task_in(&self, project: &Project, title: &str, status: &Status) -> Task {
            self.tasks
                .create(NewTask {
                    status_id: Some(status.id()),
                    ..NewTask::titled(project.id(), title)
                })
                .unwrap()
        }

        fn titles_in(&self, project: &Project, status: &Status) -> Vec<String> {
            self.tasks
                .list_for_project(project.id())
                .unwrap()
                .into_iter()
                .filter(|task| task.status_id() == status.id())
                .map(|task| task.title().to_string())
                .collect()
        }

        fn stored(&self, id: TaskId) -> Task {
            self.tasks.get(id).unwrap()
        }
    }

    fn status_named(project: &Project, name: &str) -> Status {
        project
            .workflow()
            .statuses()
            .iter()
            .find(|status| status.name().as_str() == name)
            .unwrap()
            .clone()
    }

    fn status_names(project: &Project) -> Vec<String> {
        project
            .workflow()
            .statuses()
            .iter()
            .map(|status| status.name().to_string())
            .collect()
    }

    fn properties(name: &str, category: StatusCategory, color: Color) -> StatusProperties {
        StatusProperties {
            name: name.to_owned(),
            category,
            color,
        }
    }

    fn named(name: &str) -> NewProject {
        NewProject {
            name: name.to_owned(),
            ..NewProject::default()
        }
    }

    #[test]
    fn a_new_project_is_stored_and_can_be_found_again() {
        let service = service();

        let created = service.create(named("Smart sensor node")).unwrap();

        assert_eq!(service.get(created.id()).unwrap(), created);
        assert_eq!(created.name().as_str(), "Smart sensor node");
        assert_eq!(created.created_at(), start_of_day());
    }

    #[test]
    fn a_project_without_a_requested_key_gets_one_from_its_name() {
        let created = service().create(named("Smart sensor node")).unwrap();

        assert_eq!(created.key().as_str(), "SSN");
    }

    #[test]
    fn a_blank_requested_key_counts_as_no_key() {
        let new_project = NewProject {
            key: Some("   ".to_owned()),
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.key().as_str(), "SSN");
    }

    #[test]
    fn a_requested_key_is_used() {
        let new_project = NewProject {
            key: Some("iot".to_owned()),
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.key().as_str(), "IOT");
    }

    #[test]
    fn a_requested_key_that_another_project_uses_is_refused() {
        let service = service();
        service
            .create(NewProject {
                key: Some("IOT".to_owned()),
                ..named("First")
            })
            .unwrap();

        let error = service
            .create(NewProject {
                key: Some("iot".to_owned()),
                ..named("Second")
            })
            .unwrap_err();

        assert!(matches!(error, ApplicationError::ProjectKeyTaken { key } if key == "IOT"));
    }

    #[test]
    fn a_derived_key_that_is_taken_gets_a_number() {
        let service = service();

        let keys: Vec<String> = (0..3)
            .map(|_| {
                service
                    .create(named("Smart sensor node"))
                    .unwrap()
                    .key()
                    .to_string()
            })
            .collect();

        assert_eq!(keys, ["SSN", "SSN2", "SSN3"]);
    }

    #[test]
    fn a_numbered_key_stays_within_the_length_limit() {
        let service = service();
        let long_name = "A very long name with a lot of words in it";
        service.create(named(long_name)).unwrap();

        let second = service.create(named(long_name)).unwrap();

        assert_eq!(second.key().as_str(), "AVLNW2");
    }

    #[test]
    fn the_key_of_an_archived_project_stays_reserved() {
        let service = service();
        let first = service.create(named("Smart sensor node")).unwrap();
        service.archive(first.id()).unwrap();

        let second = service.create(named("Smart sensor node")).unwrap();

        assert_eq!(second.key().as_str(), "SSN2");
    }

    #[test]
    fn a_project_without_a_name_is_refused() {
        let error = service().create(named("   ")).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::EmptyText {
                field: "project name"
            })
        ));
    }

    #[test]
    fn a_project_starts_with_the_workflow_that_was_chosen() {
        let new_project = NewProject {
            workflow: WorkflowTemplate::Engineering,
            ..named("Smart sensor node")
        };

        let created = service().create(new_project).unwrap();

        let names: Vec<&str> = created
            .workflow()
            .statuses()
            .iter()
            .map(|status| status.name().as_str())
            .collect();
        assert_eq!(
            names,
            ["To do", "In progress", "In revision", "In test", "Done"]
        );
        assert!(
            created
                .workflow()
                .first_in(StatusCategory::Review)
                .is_some()
        );
    }

    #[test]
    fn the_description_is_stored_without_surrounding_whitespace() {
        let new_project = NewProject {
            description: "  A connected sensor.\n".to_owned(),
            ..named("Node")
        };

        let created = service().create(new_project).unwrap();

        assert_eq!(created.description(), "A connected sensor.");
    }

    #[test]
    fn projects_are_listed_oldest_first() {
        let service = service();
        let first = service.create(named("First")).unwrap();
        let second = service.create(named("Second")).unwrap();

        let listed = service.list().unwrap();

        assert_eq!(listed, [first, second]);
    }

    #[test]
    fn archived_projects_are_not_listed_but_can_still_be_found() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let archived = service.archive(project.id()).unwrap();

        assert!(service.list().unwrap().is_empty());
        assert_eq!(service.get(project.id()).unwrap(), archived);
        assert_eq!(archived.archived_at(), Some(start_of_day()));
    }

    #[test]
    fn renaming_a_project_changes_its_name_and_keeps_its_key() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let renamed = service.rename(project.id(), "Air quality monitor").unwrap();

        assert_eq!(renamed.name().as_str(), "Air quality monitor");
        assert_eq!(renamed.key().as_str(), "SSN");
        assert_eq!(service.get(project.id()).unwrap(), renamed);
    }

    #[test]
    fn describing_a_project_replaces_its_description() {
        let service = service();
        let project = service.create(named("Smart sensor node")).unwrap();

        let described = service
            .describe(project.id(), "Measures temperature.")
            .unwrap();

        assert_eq!(described.description(), "Measures temperature.");
    }

    #[test]
    fn a_project_that_does_not_exist_is_reported_as_not_found() {
        let missing = ProjectId::generate();

        let error = service().get(missing).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound { kind: "project", id } if id == missing.to_string()
        ));
    }

    // Putting projects away and removing them

    #[test]
    fn a_project_that_was_put_away_is_listed_among_the_archived_ones_only() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        workspace.projects.archive(project.id()).unwrap();

        assert!(workspace.projects.list().unwrap().is_empty());
        let archived: Vec<ProjectId> = workspace
            .projects
            .list_archived()
            .unwrap()
            .iter()
            .map(Project::id)
            .collect();
        assert_eq!(archived, [project.id()]);
    }

    #[test]
    fn the_project_that_was_put_away_last_is_listed_first() {
        let workspace = workspace();
        let first = workspace.engineering_project("Smart sensor node");
        let second = workspace.engineering_project("Display driver");
        workspace.projects.archive(first.id()).unwrap();
        workspace.clock.advance(TimeDelta::hours(1));
        workspace.projects.archive(second.id()).unwrap();

        let archived = workspace.projects.list_archived().unwrap();

        let names: Vec<&str> = archived
            .iter()
            .map(|project| project.name().as_str())
            .collect();
        assert_eq!(names, ["Display driver", "Smart sensor node"]);
    }

    #[test]
    fn a_restored_project_comes_back_with_everything_it_had() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        workspace
            .tasks
            .create(NewTask::titled(project.id(), "Draw the schematic"))
            .unwrap();
        workspace.projects.archive(project.id()).unwrap();

        let restored = workspace.projects.restore(project.id()).unwrap();

        assert_eq!(restored.archived_at(), None);
        assert_eq!(workspace.projects.list().unwrap(), [restored]);
        assert_eq!(
            workspace
                .tasks
                .list_for_project(project.id())
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_removed_project_takes_its_tasks_and_its_team_with_it() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let other = workspace.engineering_project("Display driver");
        let removed_task = workspace
            .tasks
            .create(NewTask::titled(project.id(), "Draw the schematic"))
            .unwrap();
        let removed_member = workspace
            .team
            .add(NewMember {
                project_id: project.id(),
                name: "Hardware engineer".to_owned(),
                kind: MemberKind::Agent,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap();
        let kept = workspace
            .tasks
            .create(NewTask::titled(other.id(), "Write the driver"))
            .unwrap();

        workspace.projects.delete(project.id()).unwrap();

        let listed: Vec<ProjectId> = workspace
            .projects
            .list()
            .unwrap()
            .iter()
            .map(Project::id)
            .collect();
        assert_eq!(listed, [other.id()]);
        assert!(matches!(
            workspace.tasks.list_for_project(project.id()),
            Err(ApplicationError::NotFound { .. })
        ));
        assert!(matches!(
            workspace.team.list_for_project(project.id()),
            Err(ApplicationError::NotFound { .. })
        ));
        assert!(matches!(
            workspace.tasks.get(removed_task.id()),
            Err(ApplicationError::NotFound { .. })
        ));
        assert!(matches!(
            workspace.team.get(removed_member.id()),
            Err(ApplicationError::NotFound { .. })
        ));
        assert_eq!(
            workspace.tasks.list_for_project(other.id()).unwrap(),
            [kept]
        );
    }

    #[test]
    fn a_new_project_may_take_the_key_of_one_that_was_removed() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        workspace.projects.delete(project.id()).unwrap();
        let again = workspace.engineering_project("Smart sensor node");

        assert_eq!(again.key().as_str(), "SSN");
    }

    #[test]
    fn removing_or_restoring_a_project_that_does_not_exist_is_reported_as_not_found() {
        let service = service();
        let missing = ProjectId::generate();

        assert_eq!(service.delete(missing).unwrap_err().code(), "not_found");
        assert_eq!(service.restore(missing).unwrap_err().code(), "not_found");
    }

    // Adding a status

    #[test]
    fn a_new_step_of_the_work_goes_behind_the_other_steps_in_front_of_review_and_done() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let changed = workspace
            .projects
            .add_status(
                project.id(),
                properties("Waiting for parts", StatusCategory::Active, Color::Pink),
            )
            .unwrap();

        assert_eq!(
            status_names(&changed),
            [
                "To do",
                "In progress",
                "In revision",
                "Waiting for parts",
                "In test",
                "Done"
            ]
        );
        assert_eq!(workspace.projects.get(project.id()).unwrap(), changed);
    }

    #[test]
    fn a_new_todo_status_goes_behind_the_other_todo_statuses() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let changed = workspace
            .projects
            .add_status(
                project.id(),
                properties("Backlog", StatusCategory::Todo, Color::Gray),
            )
            .unwrap();

        assert_eq!(
            status_names(&changed)[..3],
            ["To do", "Backlog", "In progress"]
        );
    }

    #[test]
    fn a_second_done_status_goes_to_the_end() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let changed = workspace
            .projects
            .add_status(
                project.id(),
                properties("Cancelled", StatusCategory::Done, Color::Red),
            )
            .unwrap();

        assert_eq!(
            status_names(&changed).last().map(String::as_str),
            Some("Cancelled")
        );
    }

    #[test]
    fn a_new_status_gets_a_tidied_name_and_keeps_its_color_and_category() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let changed = workspace
            .projects
            .add_status(
                project.id(),
                properties(
                    "  Waiting   for parts ",
                    StatusCategory::Active,
                    Color::Pink,
                ),
            )
            .unwrap();

        let added = status_named(&changed, "Waiting for parts");
        assert_eq!(added.category(), StatusCategory::Active);
        assert_eq!(added.color(), Color::Pink);
    }

    #[test]
    fn a_new_status_may_not_take_a_name_in_use_whatever_the_capitals() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let error = workspace
            .projects
            .add_status(
                project.id(),
                properties("in TEST", StatusCategory::Review, Color::Amber),
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the workflow already has a status named \"in TEST\""
        );
        assert_eq!(workspace.projects.get(project.id()).unwrap(), project);
    }

    #[test]
    fn a_new_status_without_a_name_is_refused() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let error = workspace
            .projects
            .add_status(
                project.id(),
                properties("  ", StatusCategory::Active, Color::Blue),
            )
            .unwrap_err();

        assert_eq!(error.to_string(), "the status name must not be empty");
    }

    // Changing a status

    #[test]
    fn a_status_gets_another_name_and_color() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");

        let changed = workspace
            .projects
            .change_status(
                project.id(),
                in_test.id(),
                properties("On the bench", StatusCategory::Review, Color::Teal),
            )
            .unwrap();

        let status = changed.workflow().status(in_test.id()).unwrap();
        assert_eq!(status.name().as_str(), "On the bench");
        assert_eq!(status.category(), StatusCategory::Review);
        assert_eq!(status.color(), Color::Teal);
        assert_eq!(workspace.projects.get(project.id()).unwrap(), changed);
    }

    #[test]
    fn a_changed_status_may_keep_its_own_name() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");

        let changed = workspace
            .projects
            .change_status(
                project.id(),
                in_test.id(),
                properties("In test", StatusCategory::Review, Color::Red),
            )
            .unwrap();

        assert_eq!(status_named(&changed, "In test").color(), Color::Red);
    }

    #[test]
    fn a_changed_status_may_not_take_the_name_of_another() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");

        let error = workspace
            .projects
            .change_status(
                project.id(),
                in_test.id(),
                properties("Done", StatusCategory::Review, Color::Amber),
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
    }

    #[test]
    fn the_tasks_of_a_status_that_becomes_a_done_status_are_completed() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");
        let tested = workspace.task_in(&project, "Select the sensors", &in_test);
        workspace.clock.advance(TimeDelta::hours(2));

        workspace
            .projects
            .change_status(
                project.id(),
                in_test.id(),
                properties("Accepted", StatusCategory::Done, Color::Green),
            )
            .unwrap();

        let stored = workspace.stored(tested.id());
        assert_eq!(
            stored.completed_at(),
            Some(start_of_day() + TimeDelta::hours(2))
        );
        assert_eq!(stored.rank(), tested.rank());
    }

    #[test]
    fn the_tasks_of_a_done_status_that_becomes_something_else_are_reopened() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let with_cancelled = workspace
            .projects
            .add_status(
                project.id(),
                properties("Cancelled", StatusCategory::Done, Color::Red),
            )
            .unwrap();
        let cancelled = status_named(&with_cancelled, "Cancelled");
        let dropped = workspace.task_in(&project, "Add a second display", &cancelled);

        workspace
            .projects
            .change_status(
                project.id(),
                cancelled.id(),
                properties("On hold", StatusCategory::Active, Color::Red),
            )
            .unwrap();

        assert!(dropped.completed_at().is_some());
        assert_eq!(workspace.stored(dropped.id()).completed_at(), None);
    }

    #[test]
    fn the_only_todo_status_keeps_its_category() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let todo = status_named(&project, "To do");

        let error = workspace
            .projects
            .change_status(
                project.id(),
                todo.id(),
                properties("To do", StatusCategory::Active, Color::Gray),
            )
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "a workflow needs at least one status in the \"todo\" category"
        );
    }

    #[test]
    fn the_only_done_status_keeps_its_category() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let done = status_named(&project, "Done");

        let error = workspace
            .projects
            .change_status(
                project.id(),
                done.id(),
                properties("Done", StatusCategory::Review, Color::Green),
            )
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "a workflow needs at least one status in the \"done\" category"
        );
    }

    // Moving a status

    #[test]
    fn a_status_moves_to_another_place() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");

        let changed = workspace
            .projects
            .move_status(project.id(), in_test.id(), 1)
            .unwrap();

        assert_eq!(
            status_names(&changed),
            ["To do", "In test", "In progress", "In revision", "Done"]
        );
        assert_eq!(workspace.projects.get(project.id()).unwrap(), changed);
    }

    #[test]
    fn a_place_past_the_end_means_the_last_place() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let todo = status_named(&project, "To do");

        let changed = workspace
            .projects
            .move_status(project.id(), todo.id(), 99)
            .unwrap();

        assert_eq!(
            status_names(&changed).last().map(String::as_str),
            Some("To do")
        );
    }

    #[test]
    fn moving_a_status_the_project_does_not_have_is_refused() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let error = workspace
            .projects
            .move_status(project.id(), StatusId::generate(), 0)
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
    }

    // Removing a status

    #[test]
    fn a_status_without_tasks_is_removed() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_revision = status_named(&project, "In revision");

        let changed = workspace
            .projects
            .remove_status(project.id(), in_revision.id(), None)
            .unwrap();

        assert_eq!(
            status_names(&changed),
            ["To do", "In progress", "In test", "Done"]
        );
        assert_eq!(workspace.projects.get(project.id()).unwrap(), changed);
    }

    #[test]
    fn a_status_with_tasks_is_not_removed_while_they_have_nowhere_to_go() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_revision = status_named(&project, "In revision");
        workspace.task_in(&project, "Define the pin map", &in_revision);

        for destination in [None, Some(in_revision.id())] {
            let error = workspace
                .projects
                .remove_status(project.id(), in_revision.id(), destination)
                .unwrap_err();

            assert_eq!(error.code(), "status_in_use");
            assert_eq!(
                error.to_string(),
                "the status \"In revision\" still has 1 task, which need another status to go to"
            );
        }
        let stored = workspace.projects.get(project.id()).unwrap();
        assert!(stored.workflow().status(in_revision.id()).is_some());
    }

    #[test]
    fn the_refusal_counts_every_task_in_the_status_subtasks_included() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_revision = status_named(&project, "In revision");
        let parent = workspace.task_in(&project, "Define the pin map", &in_revision);
        workspace
            .tasks
            .create(NewTask {
                status_id: Some(in_revision.id()),
                parent_id: Some(parent.id()),
                ..NewTask::titled(project.id(), "List the pins")
            })
            .unwrap();
        workspace.task_in(&project, "Check the footprints", &in_revision);

        let error = workspace
            .projects
            .remove_status(project.id(), in_revision.id(), None)
            .unwrap_err();

        assert!(error.to_string().contains("still has 3 tasks,"));
    }

    #[test]
    fn the_tasks_of_a_removed_status_go_to_the_bottom_of_another_in_their_order() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_progress = status_named(&project, "In progress");
        let in_revision = status_named(&project, "In revision");
        workspace.task_in(&project, "Draw the schematic", &in_progress);
        workspace.task_in(&project, "Define the pin map", &in_revision);
        workspace.task_in(&project, "Check the footprints", &in_revision);

        workspace
            .projects
            .remove_status(project.id(), in_revision.id(), Some(in_progress.id()))
            .unwrap();

        assert_eq!(
            workspace.titles_in(&project, &in_progress),
            [
                "Draw the schematic",
                "Define the pin map",
                "Check the footprints"
            ]
        );
    }

    #[test]
    fn tasks_that_go_to_a_done_status_are_completed() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");
        let tested = workspace.task_in(&project, "Select the sensors", &in_test);

        workspace
            .projects
            .remove_status(
                project.id(),
                in_test.id(),
                Some(status_named(&project, "Done").id()),
            )
            .unwrap();

        assert_eq!(
            workspace.stored(tested.id()).completed_at(),
            Some(start_of_day())
        );
    }

    #[test]
    fn the_only_todo_status_cannot_be_removed() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");

        let error = workspace
            .projects
            .remove_status(project.id(), status_named(&project, "To do").id(), None)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "a workflow needs at least one status in the \"todo\" category"
        );
    }

    #[test]
    fn tasks_cannot_be_sent_to_a_status_the_project_does_not_have() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_revision = status_named(&project, "In revision");
        let task = workspace.task_in(&project, "Define the pin map", &in_revision);

        let error = workspace
            .projects
            .remove_status(project.id(), in_revision.id(), Some(StatusId::generate()))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(workspace.stored(task.id()), task);
    }

    // Templates

    fn from_template(workspace: &Workspace, template: WorkflowTemplate) -> Project {
        workspace
            .projects
            .create(NewProject {
                workflow: template,
                ..named("Smart sensor node")
            })
            .unwrap()
    }

    fn view_kinds(workspace: &Workspace, project: &Project) -> Vec<teamwright_domain::ViewKind> {
        let mut views = workspace.views.list_for_project(project.id()).unwrap();
        views.sort_by_key(View::position);
        views.iter().map(View::kind).collect()
    }

    fn area_names(workspace: &Workspace, project: &Project) -> Vec<String> {
        let mut areas = workspace.areas.list_for_project(project.id()).unwrap();
        areas.sort_by_key(Area::position);
        areas.iter().map(|area| area.name().to_string()).collect()
    }

    fn limits(project: &Project) -> Vec<Option<u16>> {
        project
            .workflow()
            .statuses()
            .iter()
            .map(|status| status.limit().map(WorkInProgressLimit::value))
            .collect()
    }

    #[test]
    fn the_standard_template_makes_a_project_as_before_templates() {
        let workspace = workspace();

        let project = from_template(&workspace, WorkflowTemplate::Standard);

        assert_eq!(
            status_names(&project),
            ["To do", "In progress", "In review", "Done"]
        );
        assert_eq!(limits(&project), [None, None, None, None]);
        let mut views = workspace.views.list_for_project(project.id()).unwrap();
        views.sort_by_key(View::position);
        assert_eq!(views, View::starting(project.id()));
        assert_eq!(
            workspace.areas.list_for_project(project.id()).unwrap(),
            [Area::starting(project.id())]
        );
        assert!(
            workspace
                .sprints
                .list_for_project(project.id())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_kanban_project_starts_with_limits_and_without_a_backlog_or_a_sprint() {
        use teamwright_domain::ViewKind;
        let workspace = workspace();

        let project = from_template(&workspace, WorkflowTemplate::Kanban);

        assert_eq!(
            status_names(&project),
            ["Backlog", "Ready", "In progress", "Review", "Done"]
        );
        assert_eq!(limits(&project), [None, Some(5), Some(3), Some(2), None]);
        assert_eq!(
            view_kinds(&workspace, &project),
            [
                ViewKind::Board,
                ViewKind::List,
                ViewKind::Calendar,
                ViewKind::Dashboard
            ]
        );
        assert!(
            workspace
                .sprints
                .list_for_project(project.id())
                .unwrap()
                .is_empty()
        );
        assert_eq!(project.next_sprint_number(), 1);
    }

    #[test]
    fn a_scrum_project_starts_with_the_backlog_first_and_a_first_sprint_planned() {
        use teamwright_domain::{SprintState, ViewKind};
        let workspace = workspace();

        let project = from_template(&workspace, WorkflowTemplate::Scrum);

        assert_eq!(
            view_kinds(&workspace, &project)[..2],
            [ViewKind::Backlog, ViewKind::Board]
        );
        let sprints = workspace.sprints.list_for_project(project.id()).unwrap();
        assert_eq!(sprints.len(), 1);
        assert_eq!(sprints[0].name().as_str(), "Sprint 1");
        assert_eq!(sprints[0].state(), SprintState::Planned);
        assert_eq!(sprints[0].dates().start_on(), None);
        let stored = workspace.projects.get(project.id()).unwrap();
        assert_eq!(stored.next_sprint_number(), 2);
    }

    #[test]
    fn a_hardware_product_starts_with_the_steps_of_a_device_and_an_area_for_each_part() {
        let workspace = workspace();

        let project = from_template(&workspace, WorkflowTemplate::HardwareProduct);

        assert_eq!(
            status_names(&project),
            [
                "To do",
                "Design",
                "Design review",
                "Ordered",
                "Build",
                "Test",
                "Done"
            ]
        );
        assert_eq!(
            area_names(&workspace, &project),
            ["Electronics", "Firmware", "Enclosure"]
        );
        // A task opened without an area goes into the first one.
        let task = workspace
            .tasks
            .create(NewTask::titled(project.id(), "Draw the schematic"))
            .unwrap();
        assert_eq!(task.area_id(), Area::starting_id(project.id()));
    }

    #[test]
    fn no_template_gives_a_team() {
        let workspace = workspace();
        for template in WorkflowTemplate::ALL {
            let project = from_template(&workspace, template);

            assert!(
                workspace
                    .team
                    .list_for_project(project.id())
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn everything_a_template_made_can_be_changed_afterwards() {
        let workspace = workspace();
        let project = from_template(&workspace, WorkflowTemplate::Kanban);
        let in_progress = status_named(&project, "In progress");

        workspace
            .projects
            .set_status_limit(project.id(), in_progress.id(), None)
            .unwrap();
        let changed = workspace
            .projects
            .remove_status(project.id(), status_named(&project, "Ready").id(), None)
            .unwrap();

        assert_eq!(limits(&changed), [None, None, Some(2), None]);
    }

    // Work-in-progress limits

    #[test]
    fn a_status_gets_a_limit_another_one_and_none() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");

        let limited = workspace
            .projects
            .set_status_limit(project.id(), in_test.id(), Some(2))
            .unwrap();
        assert_eq!(limits(&limited)[3], Some(2));
        assert_eq!(workspace.projects.get(project.id()).unwrap(), limited);

        let cleared = workspace
            .projects
            .set_status_limit(project.id(), in_test.id(), None)
            .unwrap();
        assert_eq!(limits(&cleared)[3], None);
    }

    #[test]
    fn a_limit_out_of_range_or_on_a_done_status_is_refused() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");
        let done = status_named(&project, "Done");

        let zero = workspace
            .projects
            .set_status_limit(project.id(), in_test.id(), Some(0))
            .unwrap_err();
        let on_done = workspace
            .projects
            .set_status_limit(project.id(), done.id(), Some(3))
            .unwrap_err();

        assert_eq!(zero.code(), "rule_broken");
        assert_eq!(
            zero.to_string(),
            "a limit is a whole number of tasks from 1 to 999"
        );
        assert_eq!(on_done.code(), "rule_broken");
        assert_eq!(workspace.projects.get(project.id()).unwrap(), project);
    }

    #[test]
    fn a_limit_below_the_tasks_already_in_the_status_is_allowed_and_moves_nothing() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_progress = status_named(&project, "In progress");
        workspace.task_in(&project, "Draw the schematic", &in_progress);
        workspace.task_in(&project, "Define the pin map", &in_progress);

        workspace
            .projects
            .set_status_limit(project.id(), in_progress.id(), Some(1))
            .unwrap();

        assert_eq!(workspace.titles_in(&project, &in_progress).len(), 2);
    }

    #[test]
    fn a_task_may_be_moved_into_a_full_status() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let todo = status_named(&project, "To do");
        let in_progress = status_named(&project, "In progress");
        workspace
            .projects
            .set_status_limit(project.id(), in_progress.id(), Some(1))
            .unwrap();
        workspace.task_in(&project, "Draw the schematic", &in_progress);
        let waiting = workspace.task_in(&project, "Define the pin map", &todo);

        workspace
            .tasks
            .move_to(waiting.id(), in_progress.id(), 1)
            .unwrap();

        let loads = workspace.projects.status_loads(project.id()).unwrap();
        let load = loads
            .iter()
            .find(|load| load.status_id() == in_progress.id())
            .unwrap();
        assert_eq!(load.count(), 2);
        assert_eq!(load.fill(), Some(teamwright_domain::Fill::Over));
    }

    #[test]
    fn the_loads_count_every_task_of_the_project_but_not_its_subtasks() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_progress = status_named(&project, "In progress");
        workspace
            .projects
            .set_status_limit(project.id(), in_progress.id(), Some(2))
            .unwrap();
        let parent = workspace.task_in(&project, "Draw the schematic", &in_progress);
        workspace
            .tasks
            .create(NewTask {
                status_id: Some(in_progress.id()),
                parent_id: Some(parent.id()),
                ..NewTask::titled(project.id(), "Draw the power supply")
            })
            .unwrap();
        workspace.task_in(&project, "Define the pin map", &in_progress);

        let loads = workspace.projects.status_loads(project.id()).unwrap();

        let counts: Vec<usize> = loads.iter().map(StatusLoad::count).collect();
        assert_eq!(counts, [0, 2, 0, 0, 0]);
        assert_eq!(loads[1].fill(), Some(teamwright_domain::Fill::At));
        assert_eq!(loads[0].fill(), None);
    }

    #[test]
    fn a_limit_goes_when_its_status_becomes_a_done_status() {
        let workspace = workspace();
        let project = workspace.engineering_project("Smart sensor node");
        let in_test = status_named(&project, "In test");
        workspace
            .projects
            .set_status_limit(project.id(), in_test.id(), Some(2))
            .unwrap();

        let changed = workspace
            .projects
            .change_status(
                project.id(),
                in_test.id(),
                properties("Accepted", StatusCategory::Done, Color::Green),
            )
            .unwrap();

        assert_eq!(limits(&changed)[3], None);
    }
}
