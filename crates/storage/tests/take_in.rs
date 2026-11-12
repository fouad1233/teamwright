//! Taking in the operations of another computer, tested with two real
//! SQLite databases that hand their operations over in memory.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    ChosenPlaces, ConversationRepository, ExchangedChange, ExchangedOperation, MemberRepository,
    NewMember, NewProject, NoticeKind, PlacesRepository, ProjectRepository, ProjectService,
    SprintRepository, TaskRepository, TaskService, TeamService, WorkflowTemplate,
};
use teamwright_domain::{Assignee, ComputerId, MemberKind, Priority, Project, Task, Timestamp};
use teamwright_storage::{
    Database, SqliteConversationRepository, SqliteMemberRepository, SqlitePlacesRepository,
    SqliteProjectRepository, SqliteSprintRepository, SqliteTaskRepository,
};

/// One computer: its database and the use cases on it.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
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
        let clock = Arc::new(ManualClock::starting_at(moment()));
        Self {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                places.clone(),
                sprints.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                sprints,
                clock.clone(),
            ),
            team: TeamService::new(projects, members, tasks, conversations, clock),
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
    /// every project with its tasks and its team, sorted.
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
        .create(teamwright_application::NewTask {
            description: "Two layers.".to_owned(),
            ..teamwright_application::NewTask::titled(project.id(), "Design the board")
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
        .create(teamwright_application::NewTask::titled(
            project.id(),
            "Write the firmware",
        ))
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
        .create(teamwright_application::NewTask::titled(
            project.id(),
            "Write the firmware",
        ))
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
        .create(teamwright_application::NewTask::titled(
            project.id(),
            "Design the board",
        ))
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
