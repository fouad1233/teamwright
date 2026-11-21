//! The tags of the projects and the tags and checklists of the tasks in a
//! real SQLite database: what is stored, what is logged, and what goes with
//! a tag or a project.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use serde_json::Value;
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    ChecklistService, NewProject, NewTask, ProjectService, TagRepository, TagService,
    TaskRepository, TaskService,
};
use teamwright_domain::{Color, Project, Task};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqliteProjectRepository, SqliteSprintRepository, SqliteTagRepository,
    SqliteTaskRepository, SqliteViewRepository,
};

/// The use cases on one database, a project in it and a task.
struct Workspace {
    database: Arc<Database>,
    projects: ProjectService,
    tags: TagService,
    checklists: ChecklistService,
    project: Project,
    task: Task,
}

fn workspace() -> Workspace {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
    let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
    let members = Arc::new(SqliteMemberRepository::new(database.clone()));
    let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
    let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
    let views = Arc::new(SqliteViewRepository::new(database.clone()));
    let areas = Arc::new(SqliteAreaRepository::new(database.clone()));
    let tag_store = Arc::new(SqliteTagRepository::new(database.clone()));
    let clock = Arc::new(ManualClock::starting_at(
        Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap(),
    ));
    let project_service = ProjectService::new(
        projects.clone(),
        tasks.clone(),
        members.clone(),
        conversations.clone(),
        Arc::new(SqlitePlacesRepository::new(database.clone())),
        sprints.clone(),
        views.clone(),
        areas.clone(),
        clock.clone(),
    );
    let task_service = TaskService::new(
        projects.clone(),
        tasks.clone(),
        members,
        conversations,
        sprints,
        areas,
        clock.clone(),
    );
    let project = project_service
        .create(NewProject {
            name: "Air quality node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let task = task_service
        .create(NewTask::titled(project.id(), "Shield the radio"))
        .unwrap();
    Workspace {
        tags: TagService::new(projects, tasks.clone(), views, tag_store, clock.clone()),
        checklists: ChecklistService::new(tasks, clock),
        projects: project_service,
        project,
        task,
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

    fn operation_count(&self) -> usize {
        self.database.operations().unwrap().len()
    }

    /// The state of the task the log kept last.
    fn last_logged_task(&self) -> serde_json::Map<String, Value> {
        let operations = self.database.operations().unwrap();
        let payload = operations
            .iter()
            .rev()
            .find(|operation| operation.entity == "task" && operation.kind == "upsert")
            .and_then(|operation| operation.payload.clone())
            .unwrap();
        match serde_json::from_str(&payload).unwrap() {
            Value::Object(state) => state,
            other => panic!("a task is logged as an object, not {other}"),
        }
    }

    fn stored_task(&self) -> Task {
        SqliteTaskRepository::new(self.database.clone())
            .find(self.task.id())
            .unwrap()
            .unwrap()
    }
}

#[test]
fn every_change_to_a_tag_is_stored_and_logged() {
    let workspace = workspace();
    let before = workspace.operation_count();

    let tag = workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();
    workspace.tags.rename(tag.id(), "Radio tests").unwrap();
    workspace.tags.recolor(tag.id(), Color::Pink).unwrap();

    let found = SqliteTagRepository::new(workspace.database.clone())
        .find(tag.id())
        .unwrap()
        .unwrap();
    assert_eq!(found.name().as_str(), "Radio tests");
    assert_eq!(found.color(), Color::Pink);

    workspace.tags.remove(tag.id()).unwrap();
    let logged = workspace.logged_since(before);
    assert_eq!(
        logged,
        [
            ("tag".to_owned(), "upsert".to_owned()),
            ("tag".to_owned(), "upsert".to_owned()),
            ("tag".to_owned(), "upsert".to_owned()),
            ("tag".to_owned(), "delete".to_owned()),
        ]
    );
    assert!(
        SqliteTagRepository::new(workspace.database.clone())
            .list_for_project(workspace.project.id())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_tags_of_a_task_are_stored_and_each_is_a_key_of_its_logged_state() {
    let workspace = workspace();
    let task = workspace.task.id();
    let (_, emc) = workspace.tags.put_on_by_name(task, "EMC").unwrap();
    let (_, power) = workspace.tags.put_on_by_name(task, "Power").unwrap();

    assert_eq!(workspace.stored_task().tags().len(), 2);
    workspace.tags.take_off(task, emc.id()).unwrap();

    let stored = workspace.stored_task();
    assert_eq!(
        stored.tags().iter().copied().collect::<Vec<_>>(),
        [power.id()]
    );
    let state = workspace.last_logged_task();
    assert_eq!(state[&format!("tag:{}", emc.id())], Value::Bool(false));
    assert_eq!(state[&format!("tag:{}", power.id())], Value::Bool(true));
}

#[test]
fn removing_a_tag_takes_it_off_its_tasks_with_a_logged_change_first() {
    let workspace = workspace();
    let (_, emc) = workspace
        .tags
        .put_on_by_name(workspace.task.id(), "EMC")
        .unwrap();
    let before = workspace.operation_count();

    workspace.tags.remove(emc.id()).unwrap();

    assert!(workspace.stored_task().tags().is_empty());
    assert_eq!(
        workspace.logged_since(before),
        [
            ("task".to_owned(), "upsert".to_owned()),
            ("tag".to_owned(), "delete".to_owned()),
        ]
    );
}

#[test]
fn the_checklist_of_a_task_is_stored_in_its_order_and_a_removed_item_is_logged_as_null() {
    let workspace = workspace();
    let task = workspace.task.id();
    workspace
        .checklists
        .add_item(task, "Order the stencil")
        .unwrap();
    let added = workspace
        .checklists
        .add_item(task, "Bake the board")
        .unwrap();
    let [stencil, bake] = [0, 1].map(|index| added.checklist().items()[index].id());
    workspace.checklists.tick_item(task, stencil, true).unwrap();
    workspace.checklists.move_item(task, bake, 0).unwrap();

    let stored = workspace.stored_task();
    let items: Vec<(&str, bool)> = stored
        .checklist()
        .items()
        .iter()
        .map(|item| (item.text().as_str(), item.is_done()))
        .collect();
    assert_eq!(
        items,
        [("Bake the board", false), ("Order the stencil", true)]
    );
    let listed = SqliteTaskRepository::new(workspace.database.clone())
        .list_for_project(workspace.project.id())
        .unwrap();
    assert_eq!(listed[0].checklist(), stored.checklist());

    workspace.checklists.remove_item(task, stencil).unwrap();
    let state = workspace.last_logged_task();
    assert_eq!(state[&format!("item:{stencil}")], Value::Null);
    assert_eq!(state[&format!("item:{bake}")]["text"], "Bake the board");
    assert_eq!(workspace.stored_task().checklist().items().len(), 1);
}

#[test]
fn deleting_a_project_deletes_and_logs_its_tags() {
    let workspace = workspace();
    workspace
        .tags
        .put_on_by_name(workspace.task.id(), "EMC")
        .unwrap();
    let before = workspace.operation_count();

    workspace.projects.delete(workspace.project.id()).unwrap();

    let logged = workspace.logged_since(before);
    assert!(logged.contains(&("tag".to_owned(), "delete".to_owned())));
    assert_eq!(
        logged.last(),
        Some(&("project".to_owned(), "delete".to_owned()))
    );
}
