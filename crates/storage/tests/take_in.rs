//! Taking in the operations of another computer, tested with two real
//! SQLite databases that hand their operations over in memory.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    AreaRepository, AreaService, ChosenPlaces, ConversationRepository, ExchangedChange,
    ExchangedOperation, MemberRepository, NewMember, NewProject, NewSprint, NewTask, NewView,
    NoticeKind, PlacesRepository, ProjectRepository, ProjectService, SprintRepository,
    SprintService, SprintStart, TaskRepository, TaskService, TeamService, ViewRepository,
    ViewService, WorkflowTemplate,
};
use teamwright_domain::{
    Area, AreaId, Assignee, ComputerId, MemberKind, Priority, Project, Task, Timestamp, View,
    ViewAssignee, ViewFilter, ViewKind,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqliteProjectRepository, SqliteSprintRepository, SqliteTaskRepository,
    SqliteViewRepository,
};

/// One computer: its database and the use cases on it.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
    sprints: SprintService,
    views: ViewService,
    areas: AreaService,
    places: SqlitePlacesRepository,
}

fn moment() -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap()
}

impl Computer {
    fn new() -> Self {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
        let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
        let members = Arc::new(SqliteMemberRepository::new(database.clone()));
        let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
        let places = Arc::new(SqlitePlacesRepository::new(database.clone()));
        let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
        let views = Arc::new(SqliteViewRepository::new(database.clone()));
        let area_store = Arc::new(SqliteAreaRepository::new(database.clone()));
        let clock = Arc::new(ManualClock::starting_at(moment()));
        Self {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                places.clone(),
                sprints.clone(),
                views.clone(),
                area_store.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                sprints.clone(),
                area_store.clone(),
                clock.clone(),
            ),
            sprints: SprintService::new(projects.clone(), tasks.clone(), sprints, clock.clone()),
            views: ViewService::new(projects.clone(), members.clone(), views.clone()),
            areas: AreaService::new(projects.clone(), tasks.clone(), area_store, clock.clone()),
            team: TeamService::new(projects, members, tasks, conversations, views, clock),
            places: SqlitePlacesRepository::new(database.clone()),
            database,
        }
    }

    /// Takes in everything the other computer made.
    fn take_in_from(&self, other: &Self) -> usize {
        let operations = other.database.operations_to_send(0).unwrap();
        self.database.take_in(&operations).unwrap().taken_in
    }

    /// Everything the manager works with, in a form that can be compared:
    /// every project with its tasks, its team, its sprints, its
    /// conversations, its views and its areas, sorted.
    fn everything(&self) -> String {
        let projects = SqliteProjectRepository::new(self.database.clone());
        let tasks = SqliteTaskRepository::new(self.database.clone());
        let members = SqliteMemberRepository::new(self.database.clone());
        let sprints = SqliteSprintRepository::new(self.database.clone());
        let conversations = SqliteConversationRepository::new(self.database.clone());
        let mut all = projects.list().unwrap();
        all.sort_by_key(Project::id);
        let mut text = String::new();
        for project in all {
            text.push_str(&format!("{project:?}\n"));
            let mut project_tasks = tasks.list_for_project(project.id()).unwrap();
            project_tasks.sort_by_key(Task::id);
            for task in project_tasks {
                text.push_str(&format!("  {task:?}\n"));
            }
            let mut team = members.list_for_project(project.id()).unwrap();
            team.sort_by_key(teamwright_domain::Member::id);
            for member in team {
                text.push_str(&format!("  {member:?}\n"));
            }
            let mut planned = sprints.list_for_project(project.id()).unwrap();
            planned.sort_by_key(teamwright_domain::Sprint::id);
            for sprint in planned {
                text.push_str(&format!("  {sprint:?}\n"));
            }
            for conversation in conversations.list_for_project(project.id()).unwrap() {
                text.push_str(&format!("  {conversation:?}\n"));
            }
            for view in self.views.list_for_project(project.id()).unwrap() {
                text.push_str(&format!("  {view:?}\n"));
            }
            for area in self.areas.list_for_project(project.id()).unwrap() {
                text.push_str(&format!("  {area:?}\n"));
            }
        }
        text
    }

    fn task(&self, task: &Task) -> Option<Task> {
        SqliteTaskRepository::new(self.database.clone())
            .find(task.id())
            .unwrap()
    }
}

/// Lets two computers take in from each other until neither has anything
/// new: what one repairs goes to the other in the next exchange.
fn keep_in_step(one: &Computer, other: &Computer) {
    for _ in 0..5 {
        let taken = other.take_in_from(one) + one.take_in_from(other);
        if taken == 0 {
            return;
        }
    }
    panic!("the computers did not settle");
}

/// A desk with a project of the engineering workflow and a task, and a
/// laptop that has taken all of it in.
fn desk_and_laptop() -> (Computer, Computer, Project, Task) {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = desk
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    let task = desk
        .tasks
        .create(NewTask {
            description: "Two layers.".to_owned(),
            ..NewTask::titled(project.id(), "Design the board")
        })
        .unwrap();
    laptop.take_in_from(&desk);
    assert_eq!(laptop.everything(), desk.everything());
    (desk, laptop, project, task)
}

#[test]
fn a_new_computer_takes_in_everything_another_one_has() {
    let (desk, laptop, project, task) = desk_and_laptop();

    assert_eq!(laptop.task(&task), desk.task(&task));
    assert_eq!(
        laptop.projects.get(project.id()).unwrap(),
        desk.projects.get(project.id()).unwrap()
    );
}

#[test]
fn changes_to_different_fields_of_one_task_both_survive() {
    let (desk, laptop, _, task) = desk_and_laptop();
    desk.tasks.rename(task.id(), "Route the board").unwrap();
    laptop
        .tasks
        .prioritize(task.id(), Priority::Urgent)
        .unwrap();

    keep_in_step(&desk, &laptop);

    let merged = desk.task(&task).unwrap();
    assert_eq!(merged.title().as_str(), "Route the board");
    assert_eq!(merged.priority(), Priority::Urgent);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn the_later_change_of_one_field_wins_in_either_order() {
    for desk_first in [true, false] {
        let (desk, laptop, _, task) = desk_and_laptop();
        desk.tasks.rename(task.id(), "Earlier title").unwrap();
        laptop.tasks.rename(task.id(), "Later title").unwrap();

        if desk_first {
            laptop.take_in_from(&desk);
            desk.take_in_from(&laptop);
        } else {
            desk.take_in_from(&laptop);
            laptop.take_in_from(&desk);
        }

        assert_eq!(desk.task(&task).unwrap().title().as_str(), "Later title");
        assert_eq!(laptop.everything(), desk.everything());
    }
}

#[test]
fn taking_in_the_same_operations_twice_changes_nothing_and_logs_nothing() {
    let (desk, laptop, _, task) = desk_and_laptop();
    desk.tasks.rename(task.id(), "Route the board").unwrap();
    laptop.take_in_from(&desk);
    let log = laptop.database.operations().unwrap().len();
    let state = laptop.everything();

    let taken = laptop.take_in_from(&desk);

    assert_eq!(taken, 0);
    assert_eq!(laptop.database.operations().unwrap().len(), log);
    assert_eq!(laptop.everything(), state);
}

#[test]
fn creating_a_task_on_one_computer_keeps_a_rename_of_the_project_on_the_other() {
    let (desk, laptop, project, _) = desk_and_laptop();
    desk.projects
        .rename(project.id(), "Sensor node, second board")
        .unwrap();
    laptop
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();

    keep_in_step(&desk, &laptop);

    let merged = laptop.projects.get(project.id()).unwrap();
    assert_eq!(merged.name().as_str(), "Sensor node, second board");
    assert_eq!(merged.next_task_number(), 3);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_task_deleted_on_one_computer_stays_deleted_and_the_dropped_change_is_told() {
    let (desk, laptop, _, task) = desk_and_laptop();
    laptop.tasks.rename(task.id(), "Route the board").unwrap();
    desk.tasks.delete(task.id()).unwrap();

    keep_in_step(&desk, &laptop);

    assert!(desk.task(&task).is_none());
    assert!(laptop.task(&task).is_none());
    assert_eq!(laptop.everything(), desk.everything());
    let notices = laptop.database.notices().unwrap();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].kind, NoticeKind::ChangeDropped);
    assert_eq!(notices[0].task_id, Some(task.id()));
    assert!(notices[0].message.contains("Route the board"));
    assert!(desk.database.notices().unwrap().is_empty());
}

#[test]
fn a_project_deleted_on_one_computer_takes_the_task_added_on_the_other_with_it() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let added = laptop
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    desk.projects.delete(project.id()).unwrap();

    keep_in_step(&desk, &laptop);

    assert!(laptop.projects.list().unwrap().is_empty());
    assert!(laptop.task(&added).is_none());
    assert!(desk.task(&added).is_none());
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_task_that_arrives_before_its_project_waits_for_it() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = desk
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let task = desk
        .tasks
        .create(NewTask::titled(project.id(), "Design the board"))
        .unwrap();
    let operations = desk.database.operations_to_send(0).unwrap();
    let (of_the_project, of_the_task): (Vec<_>, Vec<_>) = operations
        .into_iter()
        .partition(|operation| operation.entity == "project");

    laptop.database.take_in(&of_the_task).unwrap();
    assert!(laptop.task(&task).is_none());
    laptop.database.take_in(&of_the_project).unwrap();

    assert_eq!(laptop.task(&task), desk.task(&task));
    assert_eq!(laptop.everything(), desk.everything());
}

/// An operation of a computer that is not one of the two.
fn from_elsewhere(sequence: u64, entity: &str, entity_id: &str, state: &str) -> ExchangedOperation {
    let elsewhere = ComputerId::generate();
    ExchangedOperation {
        computer: elsewhere,
        sequence,
        stamp: format!("1762938000000-00000-{elsewhere}"),
        entity: entity.to_owned(),
        entity_id: entity_id.to_owned(),
        change: ExchangedChange::Upsert {
            state: state.to_owned(),
            field_stamps: None,
        },
    }
}

#[test]
fn an_operation_of_a_kind_this_version_does_not_know_is_kept_and_waits() {
    let laptop = Computer::new();
    let state = laptop.everything();

    let report = laptop
        .database
        .take_in(&[from_elsewhere(1, "whiteboard", "w1", r#"{"id":"w1"}"#)])
        .unwrap();

    assert_eq!(report.taken_in, 1);
    assert_eq!(laptop.everything(), state);
    assert!(
        laptop
            .database
            .operations()
            .unwrap()
            .iter()
            .any(|operation| operation.entity == "whiteboard")
    );
}

#[test]
fn the_folders_of_a_project_never_come_from_another_computer() {
    let (desk, laptop, project, _) = desk_and_laptop();
    desk.places
        .save(
            project.id(),
            &ChosenPlaces {
                work_folder: Some("D:/sample/board".into()),
                reference_folders: Vec::new(),
            },
        )
        .unwrap();
    let sent = desk.database.operations_to_send(0).unwrap();
    assert!(
        sent.iter()
            .all(|operation| operation.entity != "project_places")
    );

    let report = laptop
        .database
        .take_in(&[from_elsewhere(
            1,
            "project_places",
            &project.id().to_string(),
            &format!(
                r#"{{"project_id":"{}","work_folder":"D:/sample/elsewhere","reference_folders":[]}}"#,
                project.id()
            ),
        )])
        .unwrap();

    assert_eq!(report.taken_in, 1);
    assert_eq!(
        laptop.places.find(project.id()).unwrap(),
        ChosenPlaces::default()
    );
}

#[test]
fn a_task_moved_into_a_status_removed_elsewhere_lands_in_a_status_of_the_workflow() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let in_revision = project
        .workflow()
        .statuses()
        .iter()
        .find(|status| status.name().as_str() == "In revision")
        .unwrap()
        .clone();
    laptop
        .tasks
        .move_to(task.id(), in_revision.id(), 0)
        .unwrap();
    desk.projects
        .remove_status(project.id(), in_revision.id(), None)
        .unwrap();

    keep_in_step(&desk, &laptop);

    let workflow = desk.projects.get(project.id()).unwrap().workflow().clone();
    let moved = desk.task(&task).unwrap();
    let status = workflow.status(moved.status_id()).unwrap();
    assert_eq!(status.category(), in_revision.category());
    assert_eq!(laptop.everything(), desk.everything());
    assert!(
        laptop
            .database
            .notices()
            .unwrap()
            .iter()
            .any(|notice| notice.kind == NoticeKind::TaskMoved)
    );
}

#[test]
fn a_task_given_to_a_member_who_left_elsewhere_has_nobody() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let engineer = desk
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
    laptop.take_in_from(&desk);
    laptop
        .tasks
        .assign(task.id(), Some(Assignee::Member(engineer.id())))
        .unwrap();
    desk.team.remove(engineer.id()).unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(desk.task(&task).unwrap().assignee(), None);
    assert!(
        SqliteMemberRepository::new(laptop.database.clone())
            .find(engineer.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn after_a_stamp_from_the_future_the_next_change_here_is_later_still() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let tomorrow = Utc::now().timestamp_millis() + 86_400_000;
    let mut operation = desk.database.operations_to_send(0).unwrap().remove(0);
    operation.sequence = 1;
    operation.computer = ComputerId::generate();
    operation.stamp = format!("{tomorrow:013}-00000-{}", operation.computer);

    let report = laptop.database.take_in(&[operation.clone()]).unwrap();
    laptop
        .projects
        .rename(project.id(), "Renamed later")
        .unwrap();

    assert!(report.clock_ahead_by_millis.unwrap() > 86_000_000);
    let latest = laptop
        .database
        .operations_to_send(0)
        .unwrap()
        .pop()
        .unwrap();
    assert!(latest.stamp > operation.stamp);
}

#[test]
fn two_tasks_numbered_alike_are_told_apart_by_the_computer_that_made_the_later_one() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let earlier = desk
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    let later = laptop
        .tasks
        .create(NewTask::titled(project.id(), "Test the radio"))
        .unwrap();
    assert_eq!((earlier.number(), later.number()), (2, 2));

    keep_in_step(&desk, &laptop);

    assert_eq!(desk.task(&earlier).unwrap().number(), 2);
    assert_eq!(desk.task(&later).unwrap().number(), 3);
    assert_eq!(
        laptop
            .projects
            .get(project.id())
            .unwrap()
            .next_task_number(),
        4
    );
    assert_eq!(laptop.everything(), desk.everything());
    let told = laptop.database.notices().unwrap();
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].kind, NoticeKind::TaskRenumbered);
    assert!(told[0].message.starts_with("SSN-2 is now SSN-3"));
    assert!(desk.database.notices().unwrap().is_empty());
}

#[test]
fn two_projects_with_one_key_are_told_apart_by_the_computer_that_made_the_later_one() {
    let (desk, laptop, _, _) = desk_and_laptop();
    let earlier = desk
        .projects
        .create(NewProject {
            name: "Signal strength node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let later = laptop
        .projects
        .create(NewProject {
            name: "Soil sensor net".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    assert_eq!(earlier.key(), later.key());

    keep_in_step(&desk, &laptop);

    let earlier = desk.projects.get(earlier.id()).unwrap();
    let later = desk.projects.get(later.id()).unwrap();
    assert_eq!(earlier.key().as_str(), "SSN2");
    assert_eq!(later.key().as_str(), "SSN3");
    assert_eq!(laptop.everything(), desk.everything());
    assert_eq!(
        laptop.database.notices().unwrap()[0].kind,
        NoticeKind::ProjectRekeyed
    );
}

#[test]
fn a_chain_of_waiting_closed_by_two_computers_is_opened_again() {
    let (desk, laptop, project, board) = desk_and_laptop();
    let firmware = desk
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    laptop.take_in_from(&desk);
    desk.tasks
        .add_dependency(firmware.id(), board.id())
        .unwrap();
    laptop
        .tasks
        .add_dependency(board.id(), firmware.id())
        .unwrap();

    keep_in_step(&desk, &laptop);

    let firmware = desk.task(&firmware).unwrap();
    let board = desk.task(&board).unwrap();
    assert!(firmware.blocked_by().contains(&board.id()));
    assert!(
        board.blocked_by().is_empty(),
        "the link set last is removed"
    );
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn of_two_sprints_started_apart_the_one_started_later_goes_back_to_planned() {
    let (desk, laptop, project, board) = desk_and_laptop();
    let firmware = desk
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    let sprint = |name: &str| {
        desk.sprints
            .create(NewSprint {
                project_id: project.id(),
                name: name.to_owned(),
                goal: String::new(),
                start_on: None,
                end_on: None,
            })
            .unwrap()
    };
    let first = sprint("Sprint 1");
    let second = sprint("Sprint 2");
    desk.tasks.plan(board.id(), Some(first.id())).unwrap();
    desk.tasks.plan(firmware.id(), Some(second.id())).unwrap();
    laptop.take_in_from(&desk);
    let start = |name: &str| SprintStart {
        name: name.to_owned(),
        goal: String::new(),
        start_on: chrono::NaiveDate::from_ymd_opt(2026, 11, 16).unwrap(),
        end_on: chrono::NaiveDate::from_ymd_opt(2026, 11, 27).unwrap(),
    };
    desk.sprints.start(first.id(), start("Sprint 1")).unwrap();
    laptop
        .sprints
        .start(second.id(), start("Sprint 2"))
        .unwrap();

    keep_in_step(&desk, &laptop);

    assert!(desk.sprints.get(first.id()).unwrap().is_active());
    let second = desk.sprints.get(second.id()).unwrap();
    assert!(!second.is_active());
    assert_eq!(second.started_at(), None);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_conversation_about_a_task_deleted_elsewhere_goes_with_it() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let engineer = laptop
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
    let conversation = teamwright_domain::Conversation::start(
        project.id(),
        engineer.id(),
        Some(task.id()),
        moment(),
        laptop.database.device_id(),
    );
    let conversations = SqliteConversationRepository::new(laptop.database.clone());
    conversations.save_conversation(&conversation).unwrap();
    let turn = teamwright_domain::Turn::start(
        conversation.id(),
        "Draw it.",
        teamwright_domain::TurnOrigin::Brief,
        moment(),
    )
    .unwrap();
    conversations.save_turn(&turn).unwrap();
    desk.tasks.delete(task.id()).unwrap();

    let report = laptop
        .database
        .take_in(&desk.database.operations_to_send(0).unwrap())
        .unwrap();
    keep_in_step(&desk, &laptop);

    assert_eq!(report.turns_to_stop, [turn.id()]);
    assert!(conversations.find(conversation.id()).unwrap().is_none());
    assert_eq!(laptop.everything(), desk.everything());
}

/// The names of the views of a project on a computer, in their order.
fn view_names(computer: &Computer, project: &Project) -> Vec<String> {
    computer
        .views
        .list_for_project(project.id())
        .unwrap()
        .iter()
        .map(|view| view.name().to_string())
        .collect()
}

/// The view of a project on a computer that has a name.
fn view_named(computer: &Computer, project: &Project, name: &str) -> View {
    computer
        .views
        .list_for_project(project.id())
        .unwrap()
        .into_iter()
        .find(|view| view.name().as_str() == name)
        .unwrap()
}

/// Waits a little, so that what the other computer does next has a later
/// stamp than what this one did: two computers that have not taken in from
/// each other may otherwise stamp two changes in the same millisecond.
fn let_the_clock_move_on() {
    std::thread::sleep(std::time::Duration::from_millis(5));
}

fn add_view(computer: &Computer, project: &Project, kind: ViewKind, name: Option<&str>) -> View {
    computer
        .views
        .add(NewView {
            project_id: project.id(),
            kind,
            name: name.map(str::to_owned),
        })
        .unwrap()
}

#[test]
fn the_views_of_a_new_project_reach_the_other_computer() {
    let (desk, laptop, project, _) = desk_and_laptop();

    assert_eq!(
        view_names(&laptop, &project),
        ["Board", "Backlog", "List", "Gantt", "Calendar", "Dashboard"]
    );
    assert_eq!(
        laptop.views.list_for_project(project.id()).unwrap(),
        desk.views.list_for_project(project.id()).unwrap()
    );
}

#[test]
fn a_view_renamed_on_one_computer_and_moved_on_the_other_keeps_both_changes() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let gantt = view_named(&desk, &project, "Gantt");
    desk.views.rename(gantt.id(), "Schedule").unwrap();
    laptop.views.move_to(gantt.id(), 0).unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(
        view_names(&desk, &project),
        [
            "Schedule",
            "Board",
            "Backlog",
            "List",
            "Calendar",
            "Dashboard"
        ]
    );
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_view_added_to_a_project_deleted_elsewhere_goes_with_it() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let added = add_view(&laptop, &project, ViewKind::Board, Some("Firmware only"));
    desk.projects.delete(project.id()).unwrap();

    keep_in_step(&desk, &laptop);

    let views = SqliteViewRepository::new(laptop.database.clone());
    assert!(views.find(added.id()).unwrap().is_none());
    assert!(views.list_for_project(project.id()).unwrap().is_empty());
    assert!(
        SqliteViewRepository::new(desk.database.clone())
            .find(added.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_view_that_showed_a_member_who_left_elsewhere_shows_everybody_again() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let engineer = desk
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
    laptop.take_in_from(&desk);
    let board = view_named(&laptop, &project, "Board");
    laptop
        .views
        .set_filter(
            board.id(),
            ViewFilter {
                assignee: ViewAssignee::Member(engineer.id()),
                hides_finished: true,
            },
        )
        .unwrap();
    desk.team.remove(engineer.id()).unwrap();

    keep_in_step(&desk, &laptop);

    let shown = ViewFilter {
        assignee: ViewAssignee::Anyone,
        hides_finished: true,
    };
    assert_eq!(view_named(&desk, &project, "Board").filter(), shown);
    assert_eq!(view_named(&laptop, &project, "Board").filter(), shown);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn of_two_views_named_alike_the_older_keeps_the_name_and_the_younger_is_numbered() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let older = add_view(&desk, &project, ViewKind::List, Some("Firmware"));
    let_the_clock_move_on();
    let younger = add_view(&laptop, &project, ViewKind::Board, Some("FIRMWARE"));

    keep_in_step(&desk, &laptop);

    let views = desk.views.list_for_project(project.id()).unwrap();
    let name_of = |id| {
        views
            .iter()
            .find(|view| view.id() == id)
            .map(|view| view.name().to_string())
            .unwrap()
    };
    assert_eq!(name_of(older.id()), "Firmware");
    assert_eq!(name_of(younger.id()), "FIRMWARE 2");
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn of_two_backlogs_added_apart_the_younger_is_removed() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let backlog = view_named(&desk, &project, "Backlog");
    desk.views.remove(backlog.id()).unwrap();
    let older = add_view(&desk, &project, ViewKind::Backlog, None);
    let_the_clock_move_on();
    laptop.views.remove(backlog.id()).unwrap();
    let younger = add_view(&laptop, &project, ViewKind::Backlog, None);

    keep_in_step(&desk, &laptop);

    let backlogs: Vec<View> = desk
        .views
        .list_for_project(project.id())
        .unwrap()
        .into_iter()
        .filter(|view| view.kind() == ViewKind::Backlog)
        .collect();
    assert_eq!(backlogs.len(), 1);
    assert_eq!(backlogs[0].id(), older.id());
    assert_eq!(backlogs[0].name().as_str(), "Backlog");
    assert!(
        SqliteViewRepository::new(laptop.database.clone())
            .find(younger.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(laptop.everything(), desk.everything());
}

/// A small generator of numbers that are random enough for a test and the
/// same in every run.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from(self.0 >> 33).unwrap() % sides
    }
}

/// One random change on a computer. Changes that the rules refuse, or that
/// meet something the other computer deleted, are simply not made.
fn random_change(computer: &Computer, project: &Project, dice: &mut Dice) {
    let tasks = computer
        .tasks
        .list_for_project(project.id())
        .unwrap_or_default();
    let pick = |dice: &mut Dice| -> Option<&Task> {
        if tasks.is_empty() {
            None
        } else {
            tasks.get(dice.roll(tasks.len()))
        }
    };
    let Ok(current) = computer.projects.get(project.id()) else {
        return;
    };
    let statuses = current.workflow().statuses().to_vec();
    let members = computer
        .team
        .list_for_project(project.id())
        .unwrap_or_default();
    let areas = computer
        .areas
        .list_for_project(project.id())
        .unwrap_or_default();
    let area_of = |dice: &mut Dice| areas.get(dice.roll(areas.len().max(1))).map(Area::id);
    let _ = match dice.roll(12) {
        9 => computer
            .areas
            .add(project.id(), &format!("Area {}", dice.roll(1000)))
            .map(|_| ()),
        10 => match (pick(dice), area_of(dice)) {
            (Some(task), Some(area)) => computer.areas.move_task(task.id(), area).map(|_| ()),
            _ => Ok(()),
        },
        11 => match (area_of(dice), area_of(dice)) {
            (Some(removed), Some(target)) if dice.roll(3) == 0 => {
                computer.areas.remove(removed, Some(target))
            }
            _ => Ok(()),
        },
        0 | 1 => computer
            .tasks
            .create(NewTask::titled(
                project.id(),
                format!("Task {}", dice.roll(1000)),
            ))
            .map(|_| ()),
        2 => match pick(dice) {
            Some(task) => computer
                .tasks
                .rename(task.id(), &format!("Renamed {}", dice.roll(1000)))
                .map(|_| ()),
            None => Ok(()),
        },
        3 => match pick(dice) {
            Some(task) => {
                let status = &statuses[dice.roll(statuses.len())];
                computer
                    .tasks
                    .move_to(task.id(), status.id(), 0)
                    .map(|_| ())
            }
            None => Ok(()),
        },
        4 => match (pick(dice), pick(dice)) {
            (Some(task), Some(blocker)) => computer
                .tasks
                .add_dependency(task.id(), blocker.id())
                .map(|_| ()),
            _ => Ok(()),
        },
        5 => match pick(dice) {
            Some(task) if dice.roll(3) == 0 => computer.tasks.delete(task.id()),
            _ => Ok(()),
        },
        6 => computer
            .projects
            .rename(project.id(), &format!("Project {}", dice.roll(1000)))
            .map(|_| ()),
        7 => match (pick(dice), members.first()) {
            (Some(task), Some(member)) => computer
                .tasks
                .assign(task.id(), Some(Assignee::Member(member.id())))
                .map(|_| ()),
            _ => Ok(()),
        },
        _ => {
            if members.is_empty() || dice.roll(4) == 0 {
                computer
                    .team
                    .add(NewMember {
                        project_id: project.id(),
                        name: format!("Member {}", dice.roll(1000)),
                        kind: MemberKind::Human,
                        responsibility: String::new(),
                        model: None,
                        effort: None,
                    })
                    .map(|_| ())
            } else {
                computer.team.remove(members[dice.roll(members.len())].id())
            }
        }
    };
}

/// Hands a random part of what one computer made to the other.
fn exchange_some(from: &Computer, to: &Computer, dice: &mut Dice) {
    let operations = from.database.operations_to_send(0).unwrap();
    let chosen: Vec<_> = operations
        .into_iter()
        .filter(|_| dice.roll(3) != 0)
        .collect();
    to.database.take_in(&chosen).unwrap();
}

#[test]
fn two_computers_that_change_things_at_random_end_up_equal() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let mut dice = Dice(20_261_112);

    for _ in 0..300 {
        let computer = if dice.roll(2) == 0 { &desk } else { &laptop };
        random_change(computer, &project, &mut dice);
        if dice.roll(10) == 0 {
            if dice.roll(2) == 0 {
                exchange_some(&desk, &laptop, &mut dice);
            } else {
                exchange_some(&laptop, &desk, &mut dice);
            }
        }
    }
    keep_in_step(&desk, &laptop);

    assert_eq!(laptop.everything(), desk.everything());
    let tasks = desk.tasks.list_for_project(project.id()).unwrap();
    let mut numbers: Vec<u32> = tasks.iter().map(Task::number).collect();
    numbers.sort_unstable();
    numbers.dedup();
    assert_eq!(
        numbers.len(),
        tasks.len(),
        "every task has a number of its own"
    );
}

#[test]
fn a_project_whose_views_were_all_removed_on_two_computers_gets_a_board_again() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let views = desk.views.list_for_project(project.id()).unwrap();
    assert!(views.len() >= 2);
    let (desk_removes, laptop_removes) = views.split_at(views.len() / 2);
    let starting_board = View::starting_id(project.id(), ViewKind::Board);
    assert!(desk_removes.iter().any(|view| view.id() == starting_board));
    for view in desk_removes {
        desk.views.remove(view.id()).unwrap();
    }
    for view in laptop_removes {
        laptop.views.remove(view.id()).unwrap();
    }

    keep_in_step(&desk, &laptop);

    let left = desk.views.list_for_project(project.id()).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].kind(), ViewKind::Board);
    assert_eq!(left[0].name().as_str(), "Board");
    assert_ne!(
        left[0].id(),
        starting_board,
        "a deleted identifier is never used again"
    );
    assert_eq!(
        Some(left[0].clone()),
        View::board_again(project.id(), 1),
        "every computer gives the same board"
    );
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_project_saved_from_before_a_take_in_does_not_take_its_counters_back() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let repository = SqliteProjectRepository::new(laptop.database.clone());
    let mut loaded_before = repository.find(project.id()).unwrap().unwrap();
    assert_eq!(loaded_before.next_task_number(), 2);
    desk.tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    laptop.take_in_from(&desk);

    // A use case that loaded the project before the take-in stores it.
    loaded_before.rename(teamwright_domain::ProjectName::parse("Renamed").unwrap());
    repository.save(&loaded_before).unwrap();
    let created = laptop
        .tasks
        .create(NewTask::titled(project.id(), "Test the radio"))
        .unwrap();

    assert_eq!(created.number(), 3);
    let stored = repository.find(project.id()).unwrap().unwrap();
    assert_eq!(stored.name().as_str(), "Renamed");
    assert_eq!(stored.next_task_number(), 4);
    keep_in_step(&desk, &laptop);
    assert_eq!(laptop.everything(), desk.everything());
    assert!(laptop.database.notices().unwrap().is_empty());
}

#[test]
fn the_history_of_a_task_names_the_computer_of_each_change() {
    let (desk, laptop, _, task) = desk_and_laptop();
    laptop.tasks.rename(task.id(), "Route the board").unwrap();
    desk.take_in_from(&laptop);

    let history = desk.tasks.history_with_computers(task.id()).unwrap();

    let made: Vec<_> = history
        .iter()
        .map(|entry| (entry.change.kind.clone(), entry.computer))
        .collect();
    assert_eq!(made.len(), 2);
    assert_eq!(
        made[0],
        (
            teamwright_domain::TaskChangeKind::Created,
            Some(desk.database.device_id())
        )
    );
    assert!(matches!(
        made[1],
        (
            teamwright_domain::TaskChangeKind::Renamed { .. },
            Some(computer)
        ) if computer == laptop.database.device_id()
    ));
}

/// The names of the areas of a project on a computer, in their order.
fn area_names(computer: &Computer, project: &Project) -> Vec<String> {
    computer
        .areas
        .list_for_project(project.id())
        .unwrap()
        .iter()
        .map(|area| area.name().to_string())
        .collect()
}

#[test]
fn the_area_of_a_new_project_reaches_the_other_computer() {
    let (desk, laptop, project, task) = desk_and_laptop();

    assert_eq!(area_names(&laptop, &project), ["General"]);
    assert_eq!(
        laptop.areas.list_for_project(project.id()).unwrap(),
        desk.areas.list_for_project(project.id()).unwrap()
    );
    assert_eq!(
        laptop.task(&task).unwrap().area_id(),
        Area::starting_id(project.id())
    );
}

#[test]
fn an_area_renamed_on_one_computer_and_moved_on_the_other_keeps_both_changes() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let firmware = desk.areas.add(project.id(), "Firmware").unwrap();
    laptop.take_in_from(&desk);
    desk.areas.rename(firmware.id(), "Software").unwrap();
    laptop.areas.move_to(firmware.id(), 0).unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(area_names(&desk, &project), ["Software", "General"]);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_task_moved_into_another_area_on_one_computer_is_moved_on_the_other() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let firmware = desk.areas.add(project.id(), "Firmware").unwrap();
    desk.areas.move_task(task.id(), firmware.id()).unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(laptop.task(&task).unwrap().area_id(), firmware.id());
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn an_area_added_to_a_project_deleted_elsewhere_goes_with_it() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let added = laptop.areas.add(project.id(), "Firmware").unwrap();
    desk.projects.delete(project.id()).unwrap();

    keep_in_step(&desk, &laptop);

    let areas = SqliteAreaRepository::new(laptop.database.clone());
    assert!(areas.find(added.id()).unwrap().is_none());
    assert!(areas.list_for_project(project.id()).unwrap().is_empty());
    assert!(
        SqliteAreaRepository::new(desk.database.clone())
            .find(added.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(laptop.everything(), desk.everything());
}

/// The case `demoAreas.test.ts` calls "puts a kept task whose area the
/// project does not have into the first area". Here such a task comes from
/// another computer: it was put into an area that was removed elsewhere.
#[test]
fn puts_a_kept_task_whose_area_the_project_does_not_have_into_the_first_area() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let firmware = desk.areas.add(project.id(), "Firmware").unwrap();
    let board = desk.areas.add(project.id(), "Board").unwrap();
    desk.areas.move_to(firmware.id(), 0).unwrap();
    laptop.take_in_from(&desk);
    let lost = laptop
        .tasks
        .create(NewTask {
            area_id: Some(board.id()),
            ..NewTask::titled(project.id(), "Route the board")
        })
        .unwrap();
    desk.areas.remove(board.id(), None).unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(desk.task(&lost).unwrap().area_id(), firmware.id());
    assert_eq!(laptop.task(&lost).unwrap().area_id(), firmware.id());
    assert_eq!(area_names(&laptop, &project), ["Firmware", "General"]);
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn of_two_areas_named_alike_the_older_keeps_the_name_and_the_younger_is_numbered() {
    let (desk, laptop, project, _) = desk_and_laptop();
    let older = desk.areas.add(project.id(), "Firmware").unwrap();
    let_the_clock_move_on();
    let younger = laptop.areas.add(project.id(), "FIRMWARE").unwrap();

    keep_in_step(&desk, &laptop);

    let name_of = |id| area_named_by_id(&desk, &project, id);
    assert_eq!(name_of(older.id()), "Firmware");
    assert_eq!(name_of(younger.id()), "FIRMWARE 2");
    assert_eq!(laptop.everything(), desk.everything());
}

/// The name of an area of a project on a computer.
fn area_named_by_id(computer: &Computer, project: &Project, id: AreaId) -> String {
    computer
        .areas
        .list_for_project(project.id())
        .unwrap()
        .into_iter()
        .find(|area| area.id() == id)
        .map(|area| area.name().to_string())
        .unwrap()
}

#[test]
fn a_project_whose_areas_were_all_removed_on_two_computers_gets_general_again() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let firmware = desk.areas.add(project.id(), "Firmware").unwrap();
    laptop.take_in_from(&desk);
    let general = Area::starting_id(project.id());
    desk.areas.remove(general, Some(firmware.id())).unwrap();
    laptop.areas.remove(firmware.id(), None).unwrap();

    keep_in_step(&desk, &laptop);

    let left = desk.areas.list_for_project(project.id()).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].name().as_str(), "General");
    assert_ne!(
        left[0].id(),
        general,
        "a deleted identifier is never used again"
    );
    assert_eq!(
        Some(left[0].clone()),
        Area::general_again(project.id(), 1),
        "every computer gives the same area"
    );
    assert_eq!(desk.task(&task).unwrap().area_id(), left[0].id());
    assert_eq!(laptop.everything(), desk.everything());
}

#[test]
fn a_subtask_made_on_one_computer_follows_its_task_into_the_area_it_was_moved_to_on_the_other() {
    let (desk, laptop, project, task) = desk_and_laptop();
    let firmware = desk.areas.add(project.id(), "Firmware").unwrap();
    laptop.take_in_from(&desk);
    desk.areas.move_task(task.id(), firmware.id()).unwrap();
    let part = laptop
        .tasks
        .create(NewTask {
            parent_id: Some(task.id()),
            ..NewTask::titled(project.id(), "Power supply")
        })
        .unwrap();

    keep_in_step(&desk, &laptop);

    assert_eq!(desk.task(&part).unwrap().area_id(), firmware.id());
    assert_eq!(laptop.task(&part).unwrap().area_id(), firmware.id());
    assert_eq!(laptop.everything(), desk.everything());
}
