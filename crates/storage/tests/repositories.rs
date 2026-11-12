//! The repositories and the operation log, tested against a real SQLite
//! database.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    MemberRepository, NewMember, NewProject, NewTask, ProjectRepository, ProjectService,
    StatusProperties, TaskRepository, TaskService, TeamService, WorkflowTemplate,
};
use teamwright_domain::{
    Assignee, CalendarDate, Color, Member, MemberId, MemberKind, MemberName, Priority, Project,
    ProjectKey, ProjectName, Rank, Schedule, Status, StatusCategory, StatusId, StatusName, Task,
    TaskDraft, TaskId, TaskTitle, Timestamp, Workflow,
};
use teamwright_storage::{
    Database, SqliteMemberRepository, SqliteProjectRepository, SqliteTaskRepository,
};

/// A database with the repositories working on it.
struct Store {
    database: Arc<Database>,
    projects: SqliteProjectRepository,
    tasks: SqliteTaskRepository,
    members: SqliteMemberRepository,
}

impl Store {
    fn on(database: Database) -> Self {
        let database = Arc::new(database);
        Self {
            projects: SqliteProjectRepository::new(database.clone()),
            tasks: SqliteTaskRepository::new(database.clone()),
            members: SqliteMemberRepository::new(database.clone()),
            database,
        }
    }

    fn in_memory() -> Self {
        Self::on(Database::open_in_memory().unwrap())
    }

    /// Stores a project and returns it.
    fn project(&self, name: &str) -> Project {
        let project = new_project(name);
        self.projects.save(&project).unwrap();
        project
    }

    /// Stores a task in the project's first status and returns it.
    fn task(&self, project: &Project, number: u32, title: &str) -> Task {
        let task = new_task(project, number, title);
        self.tasks.save(&task).unwrap();
        task
    }

    /// Stores an agent on the team of the project and returns it.
    fn member(&self, project: &Project, name: &str) -> Member {
        let member = new_member(project, name);
        self.members.save(&member).unwrap();
        member
    }
}

fn moment(hour: u32) -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 11, hour, 0, 0).unwrap()
}

fn day(day_of_month: u32) -> CalendarDate {
    CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
}

fn new_project(name: &str) -> Project {
    let name = ProjectName::parse(name).unwrap();
    let key = ProjectKey::suggest(&name);
    Project::create(
        name,
        key,
        "A small connected device.".to_owned(),
        Workflow::engineering(),
        moment(9),
    )
}

fn new_member(project: &Project, name: &str) -> Member {
    Member::create(
        project.id(),
        MemberName::parse(name).unwrap(),
        MemberKind::Agent,
        "The schematic, the parts and the board.".to_owned(),
        Color::Teal,
        moment(9),
    )
}

fn new_task(project: &Project, number: u32, title: &str) -> Task {
    Task::create(
        project.id(),
        number,
        project.workflow().initial_status(),
        Rank::from_value(f64::from(number)),
        TaskDraft::titled(TaskTitle::parse(title).unwrap()),
        moment(9),
    )
}

// Projects

#[test]
fn a_saved_project_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");

    let found = store.projects.find(project.id()).unwrap();

    assert_eq!(found, Some(project));
}

#[test]
fn a_project_that_was_never_saved_is_not_found() {
    let store = Store::in_memory();

    let found = store
        .projects
        .find(new_project("Never saved").id())
        .unwrap();

    assert_eq!(found, None);
}

#[test]
fn the_statuses_of_a_project_keep_their_board_order() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");

    let found = store.projects.find(project.id()).unwrap().unwrap();

    let names: Vec<&str> = found
        .workflow()
        .statuses()
        .iter()
        .map(|status| status.name().as_str())
        .collect();
    assert_eq!(
        names,
        ["To do", "In progress", "In revision", "In test", "Done"]
    );
}

#[test]
fn saving_a_project_again_replaces_what_was_stored() {
    let store = Store::in_memory();
    let mut project = store.project("Smart sensor node");

    project.rename(ProjectName::parse("Air quality monitor").unwrap());
    project.describe("Measures temperature and oxygen.".to_owned());
    project.allocate_task_number();
    project.archive(moment(17));
    store.projects.save(&project).unwrap();

    let found = store.projects.find(project.id()).unwrap().unwrap();
    assert_eq!(found, project);
    assert_eq!(found.next_task_number(), 2);
    assert_eq!(found.archived_at(), Some(moment(17)));
    assert_eq!(store.projects.list().unwrap().len(), 1);
}

#[test]
fn projects_are_listed_oldest_first_each_with_its_own_workflow() {
    let store = Store::in_memory();
    let older = Project::create(
        ProjectName::parse("Older").unwrap(),
        ProjectKey::parse("OLD").unwrap(),
        String::new(),
        Workflow::standard(),
        moment(8),
    );
    let newer = new_project("Newer");
    // Stored in the opposite order, to prove that the list sorts.
    store.projects.save(&newer).unwrap();
    store.projects.save(&older).unwrap();

    let listed = store.projects.list().unwrap();

    assert_eq!(listed, [older, newer]);
    assert_eq!(listed[0].workflow().statuses().len(), 4);
    assert_eq!(listed[1].workflow().statuses().len(), 5);
}

#[test]
fn changing_and_reordering_statuses_updates_their_rows_in_place() {
    let store = Store::in_memory();
    let mut project = store.project("Smart sensor node");
    let in_test = project
        .workflow()
        .first_in(StatusCategory::Review)
        .unwrap()
        .id();
    // A task in the status: if its row were deleted and inserted again, the
    // database would refuse, because the task refers to it.
    let mut task = new_task(&project, 1, "Select the sensors");
    let status = project.workflow().status(in_test).unwrap().clone();
    task.move_to(&status, Rank::FIRST, moment(10));
    store.tasks.save(&task).unwrap();

    project
        .change_status(
            in_test,
            StatusName::parse("On the bench").unwrap(),
            StatusCategory::Active,
            Color::Teal,
        )
        .unwrap();
    project.move_status(in_test, 0).unwrap();
    store.projects.save(&project).unwrap();

    let found = store.projects.find(project.id()).unwrap().unwrap();
    assert_eq!(found, project);
    assert_eq!(
        found.workflow().statuses()[0].name().as_str(),
        "On the bench"
    );
    assert_eq!(store.tasks.find(task.id()).unwrap(), Some(task));
}

#[test]
fn a_removed_status_loses_its_row_and_the_log_shows_the_workflow_without_it() {
    let store = Store::in_memory();
    let mut project = store.project("Smart sensor node");
    let in_revision = project.workflow().statuses()[2].id();

    project.remove_status(in_revision).unwrap();
    store.projects.save(&project).unwrap();

    let found = store.projects.find(project.id()).unwrap().unwrap();
    assert_eq!(found.workflow().statuses().len(), 4);
    assert!(found.workflow().status(in_revision).is_none());
    let log = store.database.operations().unwrap();
    let last = log.last().unwrap();
    assert_eq!(
        (last.entity.as_str(), last.kind.as_str()),
        ("project", "upsert")
    );
    let state: serde_json::Value = serde_json::from_str(last.payload.as_deref().unwrap()).unwrap();
    let logged_ids: Vec<&str> = state["statuses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|status| status["id"].as_str().unwrap())
        .collect();
    assert_eq!(logged_ids.len(), 4);
    assert!(!logged_ids.contains(&in_revision.to_string().as_str()));
}

#[test]
fn a_status_that_a_task_is_still_in_cannot_be_removed_from_storage() {
    let store = Store::in_memory();
    let mut project = store.project("Smart sensor node");
    let task = store.task(&project, 1, "Design the board");
    let log_before = store.database.operations().unwrap().len();
    // The task is in the first status. Another "todo" status lets the
    // domain accept the removal; the database still refuses it.
    project
        .add_status(Status::new(
            StatusName::parse("Backlog").unwrap(),
            StatusCategory::Todo,
            Color::Gray,
        ))
        .unwrap();
    project.remove_status(task.status_id()).unwrap();

    assert!(store.projects.save(&project).is_err());
    assert_eq!(store.database.operations().unwrap().len(), log_before);
    assert_eq!(store.tasks.find(task.id()).unwrap(), Some(task));
}

#[test]
fn deleting_a_project_removes_it_with_everything_that_belongs_to_it_and_logs_each_removal() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let other = store.project("Display driver");
    let board = store.task(&project, 1, "Design the board");
    let mut firmware = new_task(&project, 2, "Write the firmware");
    firmware.set_parent(Some(board.id()), moment(10)).unwrap();
    firmware.block_on(board.id(), moment(10)).unwrap();
    store.tasks.save(&firmware).unwrap();
    let engineer = store.member(&project, "Hardware engineer");
    let kept_task = store.task(&other, 1, "Write the driver");
    let kept_member = store.member(&other, "Firmware engineer");
    let log_before = store.database.operations().unwrap().len();

    store.projects.delete(project.id()).unwrap();

    assert_eq!(store.projects.find(project.id()).unwrap(), None);
    assert_eq!(store.projects.list().unwrap(), std::slice::from_ref(&other));
    assert!(
        store
            .tasks
            .list_for_project(project.id())
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .members
            .list_for_project(project.id())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store.tasks.list_for_project(other.id()).unwrap(),
        [kept_task]
    );
    assert_eq!(
        store.members.list_for_project(other.id()).unwrap(),
        [kept_member]
    );

    let log = store.database.operations().unwrap();
    let mut removals: Vec<(String, String)> = log[log_before..]
        .iter()
        .map(|op| {
            assert_eq!(op.kind, "delete");
            assert_eq!(op.payload, None);
            (op.entity.clone(), op.entity_id.clone())
        })
        .collect();
    removals.sort();
    let mut expected = vec![
        ("member".to_owned(), engineer.id().to_string()),
        ("project".to_owned(), project.id().to_string()),
        ("task".to_owned(), board.id().to_string()),
        ("task".to_owned(), firmware.id().to_string()),
    ];
    expected.sort();
    assert_eq!(removals, expected);
    // The project is logged last, after everything that belonged to it.
    assert_eq!(log.last().unwrap().entity, "project");
}

#[test]
fn deleting_a_project_that_is_not_stored_logs_nothing() {
    let store = Store::in_memory();

    store
        .projects
        .delete(new_project("Never saved").id())
        .unwrap();

    assert!(store.database.operations().unwrap().is_empty());
}

// Tasks

#[test]
fn a_saved_task_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let parent = store.task(&project, 1, "Design the board");
    let blocker = store.task(&project, 2, "Choose the microcontroller");

    let mut task = Task::create(
        project.id(),
        3,
        project.workflow().first_in(StatusCategory::Done).unwrap(),
        Rank::from_value(-512.25),
        TaskDraft {
            description: "Two layers.\nUSB-C power.".to_owned(),
            priority: Priority::Urgent,
            assignee: Some(Assignee::Member(MemberId::generate())),
            parent_id: Some(parent.id()),
            estimate_minutes: Some(240),
            schedule: Schedule::new(Some(day(12)), Some(day(14))).unwrap(),
            ..TaskDraft::titled(TaskTitle::parse("Draw the schematic").unwrap())
        },
        moment(10),
    );
    task.block_on(blocker.id(), moment(11)).unwrap();
    store.tasks.save(&task).unwrap();

    let found = store.tasks.find(task.id()).unwrap();

    assert_eq!(found, Some(task));
}

#[test]
fn a_task_with_nothing_optional_set_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let task = store.task(&project, 1, "Order the samples");

    let found = store.tasks.find(task.id()).unwrap().unwrap();

    assert_eq!(found, task);
    assert_eq!(found.assignee(), None);
    assert_eq!(found.schedule(), Schedule::unscheduled());
    assert!(found.blocked_by().is_empty());
}

#[test]
fn a_task_assigned_to_the_manager_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut task = new_task(&project, 1, "Approve the bill of materials");
    task.assign(Some(Assignee::Manager), moment(10));
    store.tasks.save(&task).unwrap();

    let found = store.tasks.find(task.id()).unwrap().unwrap();

    assert_eq!(found.assignee(), Some(Assignee::Manager));
}

#[test]
fn a_task_that_was_never_saved_is_not_found() {
    let store = Store::in_memory();

    assert_eq!(store.tasks.find(TaskId::generate()).unwrap(), None);
}

#[test]
fn saving_a_task_again_keeps_the_tasks_that_refer_to_it_attached() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut board = store.task(&project, 1, "Design the board");
    let mut schematic = new_task(&project, 2, "Draw the schematic");
    schematic.set_parent(Some(board.id()), moment(10)).unwrap();
    let mut firmware = new_task(&project, 3, "Write the firmware");
    firmware.block_on(board.id(), moment(10)).unwrap();
    store
        .tasks
        .save_all(&[schematic.clone(), firmware.clone()])
        .unwrap();

    board.rename(
        TaskTitle::parse("Design the main board").unwrap(),
        moment(12),
    );
    store.tasks.save(&board).unwrap();

    assert_eq!(store.tasks.find(schematic.id()).unwrap(), Some(schematic));
    assert_eq!(store.tasks.find(firmware.id()).unwrap(), Some(firmware));
}

#[test]
fn removing_a_dependency_and_saving_removes_it_from_storage() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let board = store.task(&project, 1, "Design the board");
    let mut firmware = new_task(&project, 2, "Write the firmware");
    firmware.block_on(board.id(), moment(10)).unwrap();
    store.tasks.save(&firmware).unwrap();

    firmware.unblock_from(board.id(), moment(11));
    store.tasks.save(&firmware).unwrap();

    assert!(
        store
            .tasks
            .find(firmware.id())
            .unwrap()
            .unwrap()
            .blocked_by()
            .is_empty()
    );
}

#[test]
fn tasks_are_listed_for_their_own_project_only() {
    let store = Store::in_memory();
    let sensor = store.project("Smart sensor node");
    let display = store.project("Display driver");
    let board = store.task(&sensor, 1, "Design the board");
    let mut firmware = new_task(&sensor, 2, "Write the firmware");
    firmware.block_on(board.id(), moment(10)).unwrap();
    store.tasks.save(&firmware).unwrap();
    store.task(&display, 1, "Write the driver");

    let mut listed = store.tasks.list_for_project(sensor.id()).unwrap();
    listed.sort_by_key(Task::number);

    assert_eq!(listed, [board, firmware]);
}

#[test]
fn several_tasks_are_saved_together_or_not_at_all() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let valid = new_task(&project, 1, "Design the board");
    // This status belongs to no stored project, so the database refuses it.
    let unknown_status = Status::new(
        StatusName::parse("Nowhere").unwrap(),
        StatusCategory::Todo,
        Color::Gray,
    );
    let invalid = Task::create(
        project.id(),
        2,
        &unknown_status,
        Rank::FIRST,
        TaskDraft::titled(TaskTitle::parse("Write the firmware").unwrap()),
        moment(9),
    );

    let result = store.tasks.save_all(&[valid.clone(), invalid]);

    assert!(result.is_err());
    assert_eq!(store.tasks.find(valid.id()).unwrap(), None);
    assert!(
        store
            .database
            .operations()
            .unwrap()
            .iter()
            .all(|op| op.entity != "task")
    );
}

#[test]
fn deleting_tasks_removes_them_and_the_dependencies_that_mention_them() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let board = store.task(&project, 1, "Design the board");
    let mut firmware = new_task(&project, 2, "Write the firmware");
    firmware.block_on(board.id(), moment(10)).unwrap();
    store.tasks.save(&firmware).unwrap();

    store
        .tasks
        .delete(&[board.id(), TaskId::generate()])
        .unwrap();

    assert_eq!(store.tasks.find(board.id()).unwrap(), None);
    let firmware = store.tasks.find(firmware.id()).unwrap().unwrap();
    assert!(firmware.blocked_by().is_empty());
}

// Team members

#[test]
fn a_saved_member_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let member = store.member(&project, "Hardware engineer");

    let found = store.members.find(member.id()).unwrap();

    assert_eq!(found, Some(member));
}

#[test]
fn a_person_on_a_team_is_found_again_as_a_person() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let person = Member::create(
        project.id(),
        MemberName::parse("Dana").unwrap(),
        MemberKind::Human,
        String::new(),
        Color::Pink,
        moment(9),
    );
    store.members.save(&person).unwrap();

    let found = store.members.find(person.id()).unwrap().unwrap();

    assert_eq!(found.kind(), MemberKind::Human);
    assert_eq!(found.color(), Color::Pink);
    assert_eq!(found.responsibility(), "");
}

#[test]
fn a_member_that_was_never_saved_is_not_found() {
    let store = Store::in_memory();

    assert_eq!(store.members.find(MemberId::generate()).unwrap(), None);
}

#[test]
fn saving_a_member_again_replaces_what_was_stored() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut member = store.member(&project, "Hardware engineer");

    member.rename(
        MemberName::parse("Electronics engineer").unwrap(),
        moment(10),
    );
    member.set_responsibility("Everything on the board.".to_owned(), moment(11));
    store.members.save(&member).unwrap();

    let team = store.members.list_for_project(project.id()).unwrap();
    assert_eq!(team, [member]);
}

#[test]
fn members_are_listed_for_the_team_of_their_own_project_only() {
    let store = Store::in_memory();
    let sensor = store.project("Smart sensor node");
    let film = store.project("Product film");
    let engineer = store.member(&sensor, "Hardware engineer");
    let editor = store.member(&film, "Video editor");

    assert_eq!(
        store.members.list_for_project(sensor.id()).unwrap(),
        [engineer]
    );
    assert_eq!(store.members.list_for_project(film.id()).unwrap(), [editor]);
}

#[test]
fn a_deleted_member_is_gone_and_the_others_stay() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let hardware = store.member(&project, "Hardware engineer");
    let firmware = store.member(&project, "Firmware engineer");

    store.members.delete(hardware.id()).unwrap();

    assert_eq!(store.members.find(hardware.id()).unwrap(), None);
    assert_eq!(
        store.members.list_for_project(project.id()).unwrap(),
        [firmware]
    );
}

#[test]
fn changes_to_a_member_are_logged_with_the_complete_new_state() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let member = store.member(&project, "Hardware engineer");
    store.members.delete(member.id()).unwrap();
    store.members.delete(MemberId::generate()).unwrap();

    let log = store.database.operations().unwrap();

    let summary: Vec<(&str, &str)> = log
        .iter()
        .map(|op| (op.entity.as_str(), op.kind.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("project", "upsert"),
            ("member", "upsert"),
            ("member", "delete")
        ]
    );
    let state: serde_json::Value =
        serde_json::from_str(log[1].payload.as_deref().unwrap()).unwrap();
    assert_eq!(state["name"], "Hardware engineer");
    assert_eq!(state["kind"], "agent");
    assert_eq!(state["color"], "teal");
    assert_eq!(state["project_id"], project.id().to_string());
    assert_eq!(state["joined_at"], "2026-11-11T09:00:00.000Z");
    assert_eq!(log[2].entity_id, member.id().to_string());
    assert_eq!(log[2].payload, None);
}

// The operation log

#[test]
fn every_write_is_logged_in_order_with_the_identity_of_the_computer() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut task = store.task(&project, 1, "Design the board");
    task.rename(
        TaskTitle::parse("Design the main board").unwrap(),
        moment(10),
    );
    store.tasks.save(&task).unwrap();
    store.tasks.delete(&[task.id()]).unwrap();

    let log = store.database.operations().unwrap();

    let summary: Vec<(&str, &str)> = log
        .iter()
        .map(|op| (op.entity.as_str(), op.kind.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("project", "upsert"),
            ("task", "upsert"),
            ("task", "upsert"),
            ("task", "delete")
        ]
    );
    assert!(
        log.iter()
            .all(|op| op.device_id == store.database.device_id())
    );
    assert!(
        log.windows(2).all(|pair| pair[0].stamp < pair[1].stamp),
        "stamps must increase"
    );
    assert!(
        log[1..]
            .iter()
            .all(|op| op.entity_id == task.id().to_string())
    );
}

#[test]
fn an_upsert_carries_the_complete_new_state_and_a_delete_carries_nothing() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let task = store.task(&project, 1, "Design the board");
    store.tasks.delete(&[task.id()]).unwrap();

    let log = store.database.operations().unwrap();

    let project_state: serde_json::Value =
        serde_json::from_str(log[0].payload.as_deref().unwrap()).unwrap();
    assert_eq!(project_state["name"], "Smart sensor node");
    assert_eq!(project_state["key"], "SSN");
    assert_eq!(project_state["statuses"].as_array().unwrap().len(), 5);

    let task_state: serde_json::Value =
        serde_json::from_str(log[1].payload.as_deref().unwrap()).unwrap();
    assert_eq!(task_state["title"], "Design the board");
    assert_eq!(task_state["number"], 1);
    assert_eq!(task_state["created_at"], "2026-11-11T09:00:00.000Z");

    assert_eq!(log[2].payload, None);
}

#[test]
fn deleting_a_task_that_is_not_stored_logs_nothing() {
    let store = Store::in_memory();

    store.tasks.delete(&[TaskId::generate()]).unwrap();

    assert!(store.database.operations().unwrap().is_empty());
}

// A database file

#[test]
fn everything_survives_closing_and_reopening_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("teamwright.db");

    let (project, task, member, stamp_before) = {
        let store = Store::on(Database::open(&path).unwrap());
        let project = store.project("Smart sensor node");
        let task = store.task(&project, 1, "Design the board");
        let member = store.member(&project, "Hardware engineer");
        let last_stamp = store.database.operations().unwrap().pop().unwrap().stamp;
        (project, task, member, last_stamp)
    };

    let reopened = Store::on(Database::open(&path).unwrap());
    assert_eq!(
        reopened.projects.find(project.id()).unwrap(),
        Some(project.clone())
    );
    assert_eq!(reopened.tasks.find(task.id()).unwrap(), Some(task));
    assert_eq!(reopened.members.find(member.id()).unwrap(), Some(member));

    // The clock carries on from where the log ends.
    reopened.projects.save(&project).unwrap();
    let stamp_after = reopened.database.operations().unwrap().pop().unwrap().stamp;
    assert!(stamp_after > stamp_before);
}

// The use cases on the real database

#[test]
fn the_use_cases_work_on_the_real_database() {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
    let task_repository = Arc::new(SqliteTaskRepository::new(database.clone()));
    let member_repository = Arc::new(SqliteMemberRepository::new(database));
    let clock = Arc::new(ManualClock::starting_at(moment(9)));
    let projects = ProjectService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        project_repository,
        member_repository,
        task_repository,
        clock.clone(),
    );

    let project = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    let hardware = tasks
        .create(NewTask::titled(project.id(), "Design the board"))
        .unwrap();
    let firmware = tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    let enclosure = tasks
        .create(NewTask::titled(project.id(), "Design the enclosure"))
        .unwrap();
    tasks.add_dependency(firmware.id(), hardware.id()).unwrap();
    tasks.add_dependency(enclosure.id(), hardware.id()).unwrap();

    clock.advance(TimeDelta::hours(4));
    let done = project
        .workflow()
        .first_in(StatusCategory::Done)
        .unwrap()
        .id();
    let finished = tasks.move_to(hardware.id(), done, 0).unwrap();
    tasks.delete(enclosure.id()).unwrap();

    assert_eq!(finished.completed_at(), Some(moment(13)));
    let remaining = tasks.list_for_project(project.id()).unwrap();
    let titles: Vec<&str> = remaining.iter().map(|task| task.title().as_str()).collect();
    assert_eq!(titles, ["Design the board", "Write the firmware"]);
    assert_eq!(projects.get(project.id()).unwrap().next_task_number(), 4);
    assert!(tasks.add_dependency(hardware.id(), firmware.id()).is_err());

    // A team: a member is given a task, and leaves.
    let engineer = team
        .add(NewMember {
            project_id: project.id(),
            name: "Firmware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: "Everything that runs on the device.".to_owned(),
        })
        .unwrap();
    let given = tasks
        .assign(firmware.id(), Some(Assignee::Member(engineer.id())))
        .unwrap();
    assert_eq!(given.assignee(), Some(Assignee::Member(engineer.id())));
    assert_eq!(
        team.list_for_project(project.id()).unwrap(),
        std::slice::from_ref(&engineer)
    );

    team.remove(engineer.id()).unwrap();

    assert_eq!(tasks.get(firmware.id()).unwrap().assignee(), None);
    assert!(team.list_for_project(project.id()).unwrap().is_empty());
    let nobody = Some(Assignee::Member(engineer.id()));
    assert!(tasks.assign(firmware.id(), nobody).is_err());
}

#[test]
fn the_workflow_and_removal_use_cases_work_on_the_real_database() {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
    let task_repository = Arc::new(SqliteTaskRepository::new(database.clone()));
    let member_repository = Arc::new(SqliteMemberRepository::new(database.clone()));
    let clock = Arc::new(ManualClock::starting_at(moment(9)));
    let projects = ProjectService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        project_repository,
        member_repository,
        task_repository,
        clock.clone(),
    );
    let project = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    let in_revision = status_named(&project, "In revision");
    let in_test = status_named(&project, "In test");
    let pin_map = tasks
        .create(NewTask {
            status_id: Some(in_revision),
            ..NewTask::titled(project.id(), "Define the pin map")
        })
        .unwrap();

    // A new status, changed into a done status, completes the task in it.
    let project = projects
        .add_status(
            project.id(),
            StatusProperties {
                name: "Waiting for parts".to_owned(),
                category: StatusCategory::Active,
                color: Color::Pink,
            },
        )
        .unwrap();
    let waiting = status_named(&project, "Waiting for parts");
    let parts = tasks
        .create(NewTask {
            status_id: Some(waiting),
            ..NewTask::titled(project.id(), "Order the parts")
        })
        .unwrap();
    clock.advance(TimeDelta::hours(1));
    projects
        .change_status(
            project.id(),
            waiting,
            StatusProperties {
                name: "Parts arrived".to_owned(),
                category: StatusCategory::Done,
                color: Color::Green,
            },
        )
        .unwrap();
    assert_eq!(
        tasks.get(parts.id()).unwrap().completed_at(),
        Some(moment(10))
    );

    // Removing a status with a task needs a destination, then moves it.
    let refused = projects.remove_status(project.id(), in_revision, None);
    assert_eq!(refused.unwrap_err().code(), "status_in_use");
    projects
        .remove_status(project.id(), in_revision, Some(in_test))
        .unwrap();
    projects.move_status(project.id(), in_test, 0).unwrap();
    let stored = projects.get(project.id()).unwrap();
    assert_eq!(stored.workflow().statuses()[0].id(), in_test);
    assert!(stored.workflow().status(in_revision).is_none());
    assert_eq!(tasks.get(pin_map.id()).unwrap().status_id(), in_test);

    // Archived, restored, and finally removed with its team.
    team.add(NewMember {
        project_id: project.id(),
        name: "Hardware engineer".to_owned(),
        kind: MemberKind::Agent,
        responsibility: String::new(),
    })
    .unwrap();
    projects.archive(project.id()).unwrap();
    assert_eq!(projects.list_archived().unwrap().len(), 1);
    projects.restore(project.id()).unwrap();
    assert!(projects.list_archived().unwrap().is_empty());

    projects.delete(project.id()).unwrap();

    assert!(projects.list().unwrap().is_empty());
    assert!(tasks.get(pin_map.id()).is_err());
    let log = database.operations().unwrap();
    assert!(
        log.iter()
            .any(|op| op.entity == "member" && op.kind == "delete")
    );
    let again = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    assert_eq!(again.key().as_str(), "SSN");
}

fn status_named(project: &Project, name: &str) -> StatusId {
    project
        .workflow()
        .statuses()
        .iter()
        .find(|status| status.name().as_str() == name)
        .unwrap()
        .id()
}
