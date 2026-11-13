//! The areas of the projects in a real SQLite database: what is stored,
//! what is logged, and what goes with a project.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    AreaRepository, AreaService, NewProject, NewTask, ProjectService, TaskRepository, TaskService,
};
use teamwright_domain::{Area, Project, Task};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqliteProjectRepository, SqliteSprintRepository, SqliteTaskRepository,
    SqliteViewRepository,
};

/// The use cases on one database, and a project in it.
struct Workspace {
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    areas: AreaService,
    project: Project,
}

fn workspace() -> Workspace {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
    let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
    let members = Arc::new(SqliteMemberRepository::new(database.clone()));
    let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
    let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
    let areas = Arc::new(SqliteAreaRepository::new(database.clone()));
    let clock = Arc::new(ManualClock::starting_at(
        Utc.with_ymd_and_hms(2026, 11, 13, 9, 0, 0).unwrap(),
    ));
    let project_service = ProjectService::new(
        projects.clone(),
        tasks.clone(),
        members.clone(),
        conversations.clone(),
        Arc::new(SqlitePlacesRepository::new(database.clone())),
        sprints.clone(),
        Arc::new(SqliteViewRepository::new(database.clone())),
        areas.clone(),
        clock.clone(),
    );
    let project = project_service
        .create(NewProject {
            name: "Air quality node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        tasks: TaskService::new(
            projects.clone(),
            tasks.clone(),
            members,
            conversations,
            sprints,
            areas.clone(),
            clock.clone(),
        ),
        areas: AreaService::new(projects, tasks, areas, clock),
        projects: project_service,
        project,
        database,
    }
}

impl Workspace {
    /// The entity and the kind of every operation logged after the first
    /// `from`.
    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn operations(&self) -> usize {
        self.database.operations().unwrap().len()
    }

    fn stored(&self) -> Vec<Area> {
        let mut areas = SqliteAreaRepository::new(self.database.clone())
            .list_for_project(self.project.id())
            .unwrap();
        areas.sort_by_key(Area::position);
        areas
    }

    fn stored_task(&self, task: &Task) -> Task {
        SqliteTaskRepository::new(self.database.clone())
            .find(task.id())
            .unwrap()
            .unwrap()
    }
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(entity, kind)| ((*entity).to_owned(), (*kind).to_owned()))
        .collect()
}

#[test]
fn a_new_project_stores_and_logs_its_area_general() {
    let workspace = workspace();

    assert_eq!(workspace.stored(), [Area::starting(workspace.project.id())]);
    let areas_logged = workspace
        .logged_since(0)
        .into_iter()
        .filter(|(entity, kind)| entity == "area" && kind == "upsert")
        .count();
    assert_eq!(areas_logged, 1);
}

#[test]
fn every_change_to_an_area_is_stored_and_logged() {
    let workspace = workspace();
    let before = workspace.operations();

    let added = workspace
        .areas
        .add(workspace.project.id(), "Firmware")
        .unwrap();
    workspace.areas.rename(added.id(), "Software").unwrap();
    workspace.areas.move_to(added.id(), 0).unwrap();

    let found = SqliteAreaRepository::new(workspace.database.clone())
        .find(added.id())
        .unwrap()
        .unwrap();
    assert_eq!(found.name().as_str(), "Software");
    assert_eq!(found.position(), 0);
    // Added, renamed, and moved first, which moves "General" one place on.
    assert_eq!(
        workspace.logged_since(before),
        pairs(&[("area", "upsert"); 4])
    );

    let before = workspace.operations();
    workspace.areas.remove(added.id(), None).unwrap();

    assert_eq!(workspace.logged_since(before), pairs(&[("area", "delete")]));
    assert_eq!(workspace.stored().len(), 1);
}

#[test]
fn the_tasks_of_a_removed_area_are_stored_in_the_other_one_and_logged_first() {
    let workspace = workspace();
    let firmware = workspace
        .areas
        .add(workspace.project.id(), "Firmware")
        .unwrap();
    let drivers = workspace
        .tasks
        .create(NewTask {
            area_id: Some(firmware.id()),
            ..NewTask::titled(workspace.project.id(), "Write the drivers")
        })
        .unwrap();
    let general = Area::starting_id(workspace.project.id());
    let before = workspace.operations();

    workspace
        .areas
        .remove(firmware.id(), Some(general))
        .unwrap();

    assert_eq!(workspace.stored_task(&drivers).area_id(), general);
    assert_eq!(
        workspace.logged_since(before),
        pairs(&[("task", "upsert"), ("area", "delete")])
    );
}

#[test]
fn a_task_moved_into_another_area_is_stored_with_its_subtasks() {
    let workspace = workspace();
    let firmware = workspace
        .areas
        .add(workspace.project.id(), "Firmware")
        .unwrap();
    let drivers = workspace
        .tasks
        .create(NewTask::titled(workspace.project.id(), "Write the drivers"))
        .unwrap();
    let part = workspace
        .tasks
        .create(NewTask {
            parent_id: Some(drivers.id()),
            ..NewTask::titled(workspace.project.id(), "Driver for the oxygen sensor")
        })
        .unwrap();
    let before = workspace.operations();

    workspace
        .areas
        .move_task(drivers.id(), firmware.id())
        .unwrap();

    assert_eq!(workspace.stored_task(&drivers).area_id(), firmware.id());
    assert_eq!(workspace.stored_task(&part).area_id(), firmware.id());
    assert_eq!(
        workspace.logged_since(before),
        pairs(&[("task", "upsert"), ("task", "upsert")])
    );
}

#[test]
fn the_areas_go_with_their_project_and_each_removal_is_logged() {
    let workspace = workspace();
    workspace
        .areas
        .add(workspace.project.id(), "Firmware")
        .unwrap();
    let before = workspace.operations();

    workspace.projects.delete(workspace.project.id()).unwrap();

    assert!(workspace.stored().is_empty());
    let removed_areas = workspace
        .logged_since(before)
        .into_iter()
        .filter(|(entity, kind)| entity == "area" && kind == "delete")
        .count();
    assert_eq!(removed_areas, 2);
}
