//! The custom fields of a project, as `demoFields.test.ts` describes them
//! for the demo of the interface: the same rules, with the same codes.

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::{
    FieldFilter, FieldNumber, FieldText, Project, TaskChangeKind, Timestamp, ViewFilter, ViewKind,
};

use super::*;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryFieldRepository,
    InMemoryMemberRepository, InMemoryPlacesRepository, InMemoryProjectRepository,
    InMemorySprintRepository, InMemoryTagRepository, InMemoryTaskRepository,
    InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};
use crate::views::ViewService;

/// The services on shared storage, and a project called "Air quality node".
struct Workspace {
    projects: ProjectService,
    tasks: TaskService,
    views: ViewService,
    fields: FieldService,
    project: Project,
}

fn start_of_day() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap()
}

fn workspace() -> Workspace {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations_store = Arc::new(InMemoryConversationRepository::default());
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let areas_store = Arc::new(InMemoryAreaRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let fields_store = Arc::new(InMemoryFieldRepository::default());
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        sprints_store.clone(),
        views_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store,
        sprints_store,
        areas_store,
        clock.clone(),
    );
    let views = ViewService::new(
        projects_store.clone(),
        members_store,
        views_store.clone(),
        Arc::new(InMemoryTagRepository::default()),
        fields_store.clone(),
    );
    let fields = FieldService::new(
        projects_store,
        tasks_store,
        views_store,
        fields_store,
        clock,
    );
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        projects,
        tasks,
        views,
        fields,
        project,
    }
}

impl Workspace {
    fn names(&self) -> Vec<String> {
        self.fields
            .list_for_project(self.project.id())
            .unwrap()
            .iter()
            .map(|field| field.name().to_string())
            .collect()
    }

    fn task(&self, title: &str) -> Task {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
    }

    fn field(&self, name: &str, kind: FieldKind) -> Field {
        self.fields
            .add(self.project.id(), name, kind, None)
            .unwrap()
    }

    fn board(&self) -> View {
        let board = self
            .views
            .list_for_project(self.project.id())
            .unwrap()
            .remove(0);
        assert_eq!(board.kind(), ViewKind::Board);
        board
    }
}

#[test]
fn fields_are_added_at_the_end_and_moved() {
    let workspace = workspace();
    let supplier = workspace.field("Supplier", FieldKind::Choice);
    workspace.field("Unit cost", FieldKind::Number);
    let ordered = workspace.field("Ordered", FieldKind::YesNo);
    assert_eq!(workspace.names(), ["Supplier", "Unit cost", "Ordered"]);

    workspace.fields.move_to(ordered.id(), 0).unwrap();
    assert_eq!(workspace.names(), ["Ordered", "Supplier", "Unit cost"]);
    workspace.fields.move_to(supplier.id(), 99).unwrap();
    assert_eq!(workspace.names(), ["Ordered", "Unit cost", "Supplier"]);
}

#[test]
fn two_fields_of_a_project_never_share_a_name() {
    let workspace = workspace();
    let supplier = workspace.field("Supplier", FieldKind::Choice);
    let cost = workspace.field("Unit cost", FieldKind::Number);

    let taken = workspace
        .fields
        .add(workspace.project.id(), " supplier ", FieldKind::Text, None)
        .unwrap_err();
    assert_eq!(taken.code(), "field_name_taken");
    assert_eq!(
        workspace
            .fields
            .rename(cost.id(), "SUPPLIER")
            .unwrap_err()
            .code(),
        "field_name_taken"
    );
    workspace.fields.rename(supplier.id(), "supplier").unwrap();
    let other = workspace
        .projects
        .create(NewProject {
            name: "Weather station".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    assert!(
        workspace
            .fields
            .add(other.id(), "Supplier", FieldKind::Text, None)
            .is_ok()
    );
}

#[test]
fn a_project_has_at_most_thirty_fields() {
    let workspace = workspace();
    for number in 0..Field::MOST_IN_A_PROJECT {
        workspace.field(&format!("Field {number}"), FieldKind::Text);
    }

    let error = workspace
        .fields
        .add(workspace.project.id(), "One more", FieldKind::Text, None)
        .unwrap_err();

    assert_eq!(error.code(), "rule_broken");
}

#[test]
fn only_a_number_field_has_a_unit() {
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
    assert_eq!(cost.unit().unwrap().as_str(), "€");
    let cost = workspace.fields.set_unit(cost.id(), Some(" ")).unwrap();
    assert_eq!(cost.unit(), None);

    let error = workspace
        .fields
        .add(
            workspace.project.id(),
            "Datasheet",
            FieldKind::Link,
            Some("mm"),
        )
        .unwrap_err();
    assert_eq!(error.code(), "rule_broken");
    assert_eq!(workspace.names(), ["Unit cost"]);
}

#[test]
fn a_value_must_suit_its_field_and_belong_to_the_project_of_the_task() {
    let workspace = workspace();
    let task = workspace.task("Order the sensors");
    let mut supplier = workspace.field("Supplier", FieldKind::Choice);
    supplier = workspace
        .fields
        .add_choice(supplier.id(), "Mouser", None)
        .unwrap();
    let mouser = supplier.choices()[0].id();
    let cost = workspace.field("Unit cost", FieldKind::Number);

    let task = workspace
        .fields
        .set_value(task.id(), supplier.id(), FieldValue::Choice(mouser))
        .unwrap();
    let task = workspace
        .fields
        .set_value(
            task.id(),
            cost.id(),
            FieldValue::Number(FieldNumber::new(4.2).unwrap()),
        )
        .unwrap();
    assert_eq!(task.field_values().len(), 2);

    let wrong_kind = workspace
        .fields
        .set_value(task.id(), cost.id(), FieldValue::YesNo(true))
        .unwrap_err();
    assert_eq!(wrong_kind.code(), "rule_broken");
    let no_such_choice = workspace
        .fields
        .set_value(
            task.id(),
            supplier.id(),
            FieldValue::Choice(ChoiceId::generate()),
        )
        .unwrap_err();
    assert_eq!(no_such_choice.code(), "not_found");

    let other = workspace
        .projects
        .create(NewProject {
            name: "Weather station".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    let foreign = workspace
        .fields
        .add(other.id(), "Notes", FieldKind::Text, None)
        .unwrap();
    let error = workspace
        .fields
        .set_value(
            task.id(),
            foreign.id(),
            FieldValue::Text(FieldText::parse("x").unwrap()),
        )
        .unwrap_err();
    assert_eq!(error.code(), "different_projects");

    let cleared = workspace.fields.clear_value(task.id(), cost.id()).unwrap();
    assert_eq!(cleared.field_values().len(), 1);
    assert!(matches!(
        workspace.tasks.history(task.id()).unwrap().last().map(|change| &change.kind),
        Some(TaskChangeKind::FieldValueChanged { field, to: None, .. }) if *field == cost.id()
    ));
}

#[test]
fn removing_a_field_clears_its_values_and_the_filters_of_views() {
    let workspace = workspace();
    let ordered = workspace.field("Ordered", FieldKind::YesNo);
    let kept = workspace.field("Notes", FieldKind::Text);
    let task = workspace.task("Order the sensors");
    workspace
        .fields
        .set_value(task.id(), ordered.id(), FieldValue::YesNo(true))
        .unwrap();
    workspace
        .fields
        .set_value(
            task.id(),
            kept.id(),
            FieldValue::Text(FieldText::parse("Two reels").unwrap()),
        )
        .unwrap();
    let filter = ViewFilter {
        hides_finished: true,
        field: Some(FieldFilter::YesNo {
            field: ordered.id(),
            yes: true,
        }),
        ..ViewFilter::EVERY_TASK
    };
    workspace
        .views
        .set_filter(workspace.board().id(), filter)
        .unwrap();

    workspace.fields.remove(ordered.id()).unwrap();

    let stored = workspace.tasks.get(task.id()).unwrap();
    assert_eq!(
        stored.field_values().keys().copied().collect::<Vec<_>>(),
        [kept.id()]
    );
    assert!(matches!(
        workspace.tasks.history(task.id()).unwrap().last().map(|change| &change.kind),
        Some(TaskChangeKind::FieldValueChanged { field, from: Some(FieldValue::YesNo(true)), to: None })
            if *field == ordered.id()
    ));
    assert_eq!(
        workspace.board().filter(),
        ViewFilter {
            field: None,
            ..filter
        }
    );
    assert_eq!(workspace.names(), ["Notes"]);
}

#[test]
fn removing_a_choice_clears_the_tasks_that_held_it_and_no_others() {
    let workspace = workspace();
    let supplier = workspace.field("Supplier", FieldKind::Choice);
    workspace
        .fields
        .add_choice(supplier.id(), "Mouser", None)
        .unwrap();
    let supplier = workspace
        .fields
        .add_choice(supplier.id(), "LCSC", Some(Color::Teal))
        .unwrap();
    let (mouser, lcsc) = (supplier.choices()[0].id(), supplier.choices()[1].id());
    let from_mouser = workspace.task("Order the sensors");
    let from_lcsc = workspace.task("Order the connectors");
    workspace
        .fields
        .set_value(from_mouser.id(), supplier.id(), FieldValue::Choice(mouser))
        .unwrap();
    workspace
        .fields
        .set_value(from_lcsc.id(), supplier.id(), FieldValue::Choice(lcsc))
        .unwrap();
    let filter = ViewFilter {
        field: Some(FieldFilter::Choice {
            field: supplier.id(),
            choice: mouser,
        }),
        ..ViewFilter::EVERY_TASK
    };
    workspace
        .views
        .set_filter(workspace.board().id(), filter)
        .unwrap();

    let supplier = workspace
        .fields
        .remove_choice(supplier.id(), mouser)
        .unwrap();

    assert_eq!(supplier.choices().len(), 1);
    assert!(
        workspace
            .tasks
            .get(from_mouser.id())
            .unwrap()
            .field_values()
            .is_empty()
    );
    assert_eq!(
        workspace.tasks.get(from_lcsc.id()).unwrap().field_values()[&supplier.id()],
        FieldValue::Choice(lcsc)
    );
    assert_eq!(workspace.board().filter(), ViewFilter::EVERY_TASK);
}

#[test]
fn a_view_is_filtered_only_by_a_choice_or_a_yes_or_no_of_its_own_project() {
    let workspace = workspace();
    let notes = workspace.field("Notes", FieldKind::Text);
    let supplier = workspace.field("Supplier", FieldKind::Choice);
    let board = workspace.board().id();
    let by = |by_field: FieldFilter| ViewFilter {
        field: Some(by_field),
        ..ViewFilter::EVERY_TASK
    };

    let not_filterable = workspace
        .views
        .set_filter(
            board,
            by(FieldFilter::YesNo {
                field: notes.id(),
                yes: true,
            }),
        )
        .unwrap_err();
    assert_eq!(not_filterable.code(), "rule_broken");
    let no_such_choice = workspace
        .views
        .set_filter(
            board,
            by(FieldFilter::Choice {
                field: supplier.id(),
                choice: ChoiceId::generate(),
            }),
        )
        .unwrap_err();
    assert_eq!(no_such_choice.code(), "not_found");
    let no_such_field = workspace
        .views
        .set_filter(
            board,
            by(FieldFilter::YesNo {
                field: FieldId::generate(),
                yes: false,
            }),
        )
        .unwrap_err();
    assert_eq!(no_such_field.code(), "not_found");
    assert_eq!(workspace.board().filter(), ViewFilter::EVERY_TASK);
}

#[test]
fn a_task_made_in_a_view_filtered_by_a_field_holds_the_value_the_view_shows() {
    let workspace = workspace();
    let supplier = workspace.field("Supplier", FieldKind::Choice);
    let supplier = workspace
        .fields
        .add_choice(supplier.id(), "Mouser", None)
        .unwrap();
    let mouser = supplier.choices()[0].id();
    let ordered = workspace.field("Ordered", FieldKind::YesNo);
    let board = workspace.board().id();
    let project = workspace.project.id();
    let make_in_view = |filter: FieldFilter| {
        workspace
            .views
            .set_filter(
                board,
                ViewFilter {
                    field: Some(filter),
                    ..ViewFilter::EVERY_TASK
                },
            )
            .unwrap();
        let demands = workspace
            .views
            .demands_of_new_tasks(board, project)
            .unwrap();
        workspace
            .tasks
            .create_in_view(NewTask::titled(project, "Order"), &demands)
            .unwrap()
    };

    let by_choice = make_in_view(FieldFilter::Choice {
        field: supplier.id(),
        choice: mouser,
    });
    let by_yes = make_in_view(FieldFilter::YesNo {
        field: ordered.id(),
        yes: true,
    });
    let by_no = make_in_view(FieldFilter::YesNo {
        field: ordered.id(),
        yes: false,
    });

    assert_eq!(
        by_choice.field_values().get(&supplier.id()),
        Some(&FieldValue::Choice(mouser))
    );
    assert_eq!(
        by_yes.field_values().get(&ordered.id()),
        Some(&FieldValue::YesNo(true))
    );
    // A task that holds nothing already reads as "no", so nothing is set.
    assert!(by_no.field_values().is_empty());
}
