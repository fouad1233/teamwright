//! The custom fields of the projects and the values of the tasks in a real
//! SQLite database: what is stored, what is logged, and what goes with a
//! field, a choice or a project.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use serde_json::{Value, json};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    FieldRepository, FieldService, NewProject, NewTask, ProjectService, TaskRepository, TaskService,
};
use teamwright_domain::{
    CalendarDate, FieldKind, FieldNumber, FieldText, FieldValue, Project, Task,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteFieldRepository,
    SqliteMemberRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
};

/// The use cases on one database, a project in it and a task.
struct Workspace {
    database: Arc<Database>,
    projects: ProjectService,
    fields: FieldService,
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
    let field_store = Arc::new(SqliteFieldRepository::new(database.clone()));
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
        .create(NewTask::titled(project.id(), "Order the sensors"))
        .unwrap();
    Workspace {
        fields: FieldService::new(projects, tasks, views, field_store, clock),
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

    /// The state the log kept last of a kind of thing.
    fn last_logged(&self, entity: &str) -> serde_json::Map<String, Value> {
        let operations = self.database.operations().unwrap();
        let payload = operations
            .iter()
            .rev()
            .find(|operation| operation.entity == entity && operation.kind == "upsert")
            .and_then(|operation| operation.payload.clone())
            .unwrap();
        match serde_json::from_str(&payload).unwrap() {
            Value::Object(state) => state,
            other => panic!("a {entity} is logged as an object, not {other}"),
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
fn a_field_and_its_choices_are_stored_and_a_removed_choice_is_logged_as_null() {
    let workspace = workspace();
    let before = workspace.operation_count();
    let supplier = workspace
        .fields
        .add(workspace.project.id(), "Supplier", FieldKind::Choice, None)
        .unwrap();
    workspace
        .fields
        .add_choice(supplier.id(), "Mouser", None)
        .unwrap();
    let supplier = workspace
        .fields
        .add_choice(supplier.id(), "LCSC", None)
        .unwrap();
    let mouser = supplier.choices()[0].id();

    let supplier = workspace
        .fields
        .remove_choice(supplier.id(), mouser)
        .unwrap();

    let found = SqliteFieldRepository::new(workspace.database.clone())
        .find(supplier.id())
        .unwrap()
        .unwrap();
    assert_eq!(found, supplier);
    let state = workspace.last_logged("field");
    assert_eq!(state[&format!("choice:{mouser}")], Value::Null);
    assert_eq!(state["kind"], "choice");
    assert_eq!(
        workspace.logged_since(before),
        vec![("field".to_owned(), "upsert".to_owned()); 4]
    );
}

#[test]
fn every_kind_of_value_is_stored_and_each_is_a_key_of_the_logged_task() {
    let workspace = workspace();
    let project = workspace.project.id();
    let task = workspace.task.id();
    let add =
        |name: &str, kind: FieldKind| workspace.fields.add(project, name, kind, None).unwrap();
    let notes = add("Notes", FieldKind::Text);
    let cost = add("Unit cost", FieldKind::Number);
    let arrives = add("Arrives", FieldKind::Date);
    let ordered = add("Ordered", FieldKind::YesNo);
    let datasheet = add("Datasheet", FieldKind::Link);
    let values = [
        (
            notes.id(),
            FieldValue::Text(FieldText::parse("Two reels").unwrap()),
        ),
        (
            cost.id(),
            FieldValue::Number(FieldNumber::new(0.42).unwrap()),
        ),
        (
            arrives.id(),
            FieldValue::Date(CalendarDate::from_ymd_opt(2026, 11, 30).unwrap()),
        ),
        (ordered.id(), FieldValue::YesNo(true)),
        (
            datasheet.id(),
            FieldValue::link("https://www.sensirion.com/sen5x.pdf").unwrap(),
        ),
    ];
    for (field, value) in values.clone() {
        workspace.fields.set_value(task, field, value).unwrap();
    }

    let stored = workspace.stored_task();
    assert_eq!(stored.field_values().len(), 5);
    for (field, value) in &values {
        assert_eq!(&stored.field_values()[field], value);
    }
    let listed = SqliteTaskRepository::new(workspace.database.clone())
        .list_for_project(project)
        .unwrap();
    assert_eq!(listed[0].field_values(), stored.field_values());

    workspace.fields.clear_value(task, notes.id()).unwrap();
    let state = workspace.last_logged("task");
    assert_eq!(state[&format!("field:{}", notes.id())], Value::Null);
    assert_eq!(
        state[&format!("field:{}", cost.id())],
        json!({ "number": 0.42 })
    );
    assert_eq!(
        state[&format!("field:{}", arrives.id())],
        json!({ "date": "2026-11-30" })
    );
    assert_eq!(workspace.stored_task().field_values().len(), 4);
}

#[test]
fn removing_a_field_clears_its_values_with_a_logged_change_first() {
    let workspace = workspace();
    let ordered = workspace
        .fields
        .add(workspace.project.id(), "Ordered", FieldKind::YesNo, None)
        .unwrap();
    workspace
        .fields
        .set_value(workspace.task.id(), ordered.id(), FieldValue::YesNo(false))
        .unwrap();
    let before = workspace.operation_count();

    workspace.fields.remove(ordered.id()).unwrap();

    assert!(workspace.stored_task().field_values().is_empty());
    assert_eq!(
        workspace.logged_since(before),
        [
            ("task".to_owned(), "upsert".to_owned()),
            ("field".to_owned(), "delete".to_owned()),
        ]
    );
}

#[test]
fn deleting_a_project_deletes_and_logs_its_fields() {
    let workspace = workspace();
    let cost = workspace
        .fields
        .add(
            workspace.project.id(),
            "Unit cost",
            FieldKind::Number,
            Some("€"),
        )
        .unwrap();
    workspace
        .fields
        .set_value(
            workspace.task.id(),
            cost.id(),
            FieldValue::Number(FieldNumber::new(3.0).unwrap()),
        )
        .unwrap();
    let before = workspace.operation_count();

    workspace.projects.delete(workspace.project.id()).unwrap();

    let logged = workspace.logged_since(before);
    assert!(logged.contains(&("field".to_owned(), "delete".to_owned())));
    assert_eq!(
        logged.last(),
        Some(&("project".to_owned(), "delete".to_owned()))
    );
}
