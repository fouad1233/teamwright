//! The repositories and the operation log, tested against a real SQLite
//! database.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::{
    FixedProjectFolders, ManualClock, RecordingWorkListener, ScriptedRuntime,
};
use teamwright_application::{
    AgentEvent, ConversationRepository, ManagerQuestion, MemberRepository, ModelUsage, NewMember,
    NewProject, NewTask, ProjectRepository, ProjectService, QuestionOption, SessionId,
    StatusProperties, TaskRepository, TaskService, TeamService, TurnOutcome, WorkPorts,
    WorkService, WorkflowTemplate,
};
use teamwright_domain::{
    Assignee, CalendarDate, Color, Conversation, ConversationId, DecisionOption, DecisionRequest,
    Effort, Member, MemberId, MemberKind, MemberName, MemberProfile, ModelName, Priority, Project,
    ProjectKey, ProjectName, Rank, Schedule, Status, StatusCategory, StatusId, StatusName,
    StepContent, Task, TaskDraft, TaskId, TaskTitle, Timestamp, TokenCounts, ToolOutcome, Turn,
    TurnOrigin, TurnState, Workflow,
};
use teamwright_storage::{
    Database, Operation, SqliteConversationRepository, SqliteMemberRepository,
    SqliteProjectRepository, SqliteTaskRepository,
};

/// A database with the repositories working on it.
struct Store {
    database: Arc<Database>,
    projects: SqliteProjectRepository,
    tasks: SqliteTaskRepository,
    members: SqliteMemberRepository,
    conversations: SqliteConversationRepository,
}

impl Store {
    fn on(database: Database) -> Self {
        let database = Arc::new(database);
        Self {
            projects: SqliteProjectRepository::new(database.clone()),
            tasks: SqliteTaskRepository::new(database.clone()),
            members: SqliteMemberRepository::new(database.clone()),
            conversations: SqliteConversationRepository::new(database.clone()),
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

#[test]
fn every_version_a_task_was_stored_in_can_be_read_back_the_oldest_first() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let blocker = store.task(&project, 1, "Choose the microcontroller");
    let opened = store.task(&project, 2, "Draw the schematic");
    let mut waiting = opened.clone();
    waiting.block_on(blocker.id(), moment(10)).unwrap();
    store.tasks.save(&waiting).unwrap();
    let mut done = waiting.clone();
    done.move_to(
        project.workflow().first_in(StatusCategory::Done).unwrap(),
        Rank::FIRST,
        moment(11),
    );
    done.assign(Some(Assignee::Manager), moment(11));
    store.tasks.save(&done).unwrap();

    let versions = store.tasks.versions_of(opened.id()).unwrap();

    assert_eq!(versions, [opened, waiting, done.clone()]);
    assert_eq!(
        versions.last(),
        store.tasks.find(done.id()).unwrap().as_ref(),
        "the last version is the task as it is stored"
    );
    assert_eq!(
        store.tasks.versions_of(blocker.id()).unwrap().len(),
        1,
        "the versions of another task are not mixed in"
    );
}

#[test]
fn a_task_that_was_never_stored_has_no_versions() {
    let store = Store::in_memory();

    assert_eq!(store.tasks.versions_of(TaskId::generate()).unwrap(), []);
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

fn sonnet_at_extra_high_effort() -> MemberProfile {
    MemberProfile {
        model: Some(ModelName::parse("sonnet").unwrap()),
        effort: Some(Effort::ExtraHigh),
    }
}

#[test]
fn a_member_with_a_model_and_an_effort_is_found_again_unchanged() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut member = new_member(&project, "Hardware engineer");
    member
        .set_profile(sonnet_at_extra_high_effort(), moment(10))
        .unwrap();
    store.members.save(&member).unwrap();

    let found = store.members.find(member.id()).unwrap().unwrap();

    assert_eq!(found.model().map(ModelName::as_str), Some("sonnet"));
    assert_eq!(found.effort(), Some(Effort::ExtraHigh));
    assert_eq!(found, member);
}

#[test]
fn a_change_of_the_profile_is_logged_with_the_complete_new_state() {
    let store = Store::in_memory();
    let project = store.project("Smart sensor node");
    let mut member = store.member(&project, "Hardware engineer");

    member
        .set_profile(sonnet_at_extra_high_effort(), moment(10))
        .unwrap();
    store.members.save(&member).unwrap();
    member
        .set_profile(MemberProfile::default(), moment(11))
        .unwrap();
    store.members.save(&member).unwrap();

    let log = store.database.operations().unwrap();
    let states: Vec<serde_json::Value> = log
        .iter()
        .filter(|op| op.entity.as_str() == "member")
        .map(|op| serde_json::from_str(op.payload.as_deref().unwrap()).unwrap())
        .collect();
    assert_eq!(states.len(), 3);
    assert_eq!(states[0]["model"], serde_json::Value::Null);
    assert_eq!(states[1]["model"], "sonnet");
    assert_eq!(states[1]["effort"], "xhigh");
    assert_eq!(states[1]["name"], "Hardware engineer");
    assert_eq!(states[1]["updated_at"], "2026-11-11T10:00:00.000Z");
    assert_eq!(states[2]["model"], serde_json::Value::Null);
    assert_eq!(states[2]["effort"], serde_json::Value::Null);
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

// Conversations

/// Stores a project with a specialist and a task given to it, and a
/// conversation about the task.
fn stored_conversation(store: &Store) -> (Project, Member, Task, Conversation) {
    let project = store.project("Smart sensor node");
    let member = store.member(&project, "Hardware engineer");
    let task = store.task(&project, 1, "Draw the schematic");
    let conversation = Conversation::start(project.id(), member.id(), Some(task.id()), moment(9));
    store
        .conversations
        .save_conversation(&conversation)
        .unwrap();
    (project, member, task, conversation)
}

fn running_turn(conversation: &Conversation, request: &str, at: Timestamp) -> Turn {
    Turn::start(conversation.id(), request, TurnOrigin::Brief, at).unwrap()
}

fn tokens() -> TokenCounts {
    TokenCounts {
        input: 42,
        output: 880,
        cache_read: 21_500,
        cache_written: 6400,
    }
}

/// The entity and kind of the operations logged after the first `skip`.
fn logged_after(store: &Store, skip: usize) -> Vec<(String, String)> {
    store
        .database
        .operations()
        .unwrap()
        .into_iter()
        .skip(skip)
        .map(|op: Operation| (op.entity, op.kind))
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(entity, kind)| ((*entity).to_owned(), (*kind).to_owned()))
        .collect()
}

#[test]
fn a_saved_conversation_and_its_turns_are_found_again_unchanged() {
    let store = Store::in_memory();
    let (project, _, _, mut conversation) = stored_conversation(&store);
    conversation.remember_session("0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10".to_owned());
    store
        .conversations
        .save_conversation(&conversation)
        .unwrap();

    let mut first = running_turn(&conversation, "Draw the schematic", moment(9));
    first.record_model("claude-opus-model".to_owned()).unwrap();
    first
        .notice("A plugin could not be loaded.".to_owned(), moment(9))
        .unwrap();
    first
        .say("I will read the notes.".to_owned(), moment(9))
        .unwrap();
    first
        .use_tool(
            "read".to_owned(),
            "Read".to_owned(),
            "notes.md".to_owned(),
            moment(9),
        )
        .unwrap();
    first
        .use_tool(
            "write".to_owned(),
            "Write".to_owned(),
            String::new(),
            moment(9),
        )
        .unwrap();
    first.finish_tool("read", false).unwrap();
    first.finish_tool("write", true).unwrap();
    first.record_denial("Bash".to_owned(), moment(9)).unwrap();
    first.finish("The report.", tokens(), moment(10)).unwrap();
    let mut second = Turn::start(
        conversation.id(),
        "Use the smaller regulator.",
        TurnOrigin::Message,
        moment(11),
    )
    .unwrap();
    second.say("Working on it.".to_owned(), moment(11)).unwrap();
    // Saved in the other order, to show that turns are listed by age.
    store.conversations.save_turn(&second).unwrap();
    store.conversations.save_turn(&first).unwrap();

    assert_eq!(
        store.conversations.find(conversation.id()).unwrap(),
        Some(conversation.clone())
    );
    assert_eq!(
        store.conversations.turns_of(conversation.id()).unwrap(),
        [first, second.clone()]
    );
    assert_eq!(
        store.conversations.list_for_project(project.id()).unwrap(),
        [conversation]
    );
    assert_eq!(store.conversations.running_turns().unwrap(), [second]);
}

#[test]
fn a_conversation_that_was_never_saved_is_not_found() {
    let store = Store::in_memory();

    assert_eq!(
        store
            .conversations
            .find(ConversationId::generate())
            .unwrap(),
        None
    );
    assert!(
        store
            .conversations
            .turns_of(ConversationId::generate())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn conversations_are_listed_for_their_own_project_oldest_first() {
    let store = Store::in_memory();
    let (project, member, _, about_the_task) = stored_conversation(&store);
    let question = Conversation::start(project.id(), member.id(), None, moment(8));
    store.conversations.save_conversation(&question).unwrap();
    let other = store.project("Thermostat");

    assert_eq!(
        store.conversations.list_for_project(project.id()).unwrap(),
        [question, about_the_task]
    );
    assert!(
        store
            .conversations
            .list_for_project(other.id())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn saving_a_turn_again_writes_and_logs_only_what_changed() {
    let store = Store::in_memory();
    let (_, _, _, conversation) = stored_conversation(&store);
    let mut turn = running_turn(&conversation, "Draw the schematic", moment(9));
    store.conversations.save_turn(&turn).unwrap();
    let start = store.database.operations().unwrap().len();

    turn.say("I will read the notes.".to_owned(), moment(9))
        .unwrap();
    turn.use_tool(
        "read".to_owned(),
        "Read".to_owned(),
        "notes.md".to_owned(),
        moment(9),
    )
    .unwrap();
    store.conversations.save_turn(&turn).unwrap();
    let after_two_steps = logged_after(&store, start);

    turn.finish_tool("read", false).unwrap();
    store.conversations.save_turn(&turn).unwrap();
    let after_the_tool = logged_after(&store, start + 2);

    store.conversations.save_turn(&turn).unwrap();
    let after_nothing = logged_after(&store, start + 3);

    turn.say("The report.".to_owned(), moment(10)).unwrap();
    store.conversations.save_turn(&turn).unwrap();
    turn.finish("The report.", tokens(), moment(10)).unwrap();
    store.conversations.save_turn(&turn).unwrap();
    let after_the_end = logged_after(&store, start + 3);

    assert_eq!(
        after_two_steps,
        pairs(&[("turn_step", "upsert"), ("turn_step", "upsert")])
    );
    assert_eq!(after_the_tool, pairs(&[("turn_step", "upsert")]));
    assert!(after_nothing.is_empty());
    assert_eq!(
        after_the_end,
        pairs(&[
            ("turn_step", "upsert"),
            ("turn", "upsert"),
            ("turn_step", "delete"),
        ])
    );
    let log = store.database.operations().unwrap();
    let tool_state: serde_json::Value =
        serde_json::from_str(log[start + 2].payload.as_deref().unwrap()).unwrap();
    assert_eq!(log[start + 2].entity_id, format!("{}/1", turn.id()));
    assert_eq!(tool_state["outcome"], "done");
    assert_eq!(
        store.conversations.turns_of(conversation.id()).unwrap(),
        [turn]
    );
}

#[test]
fn a_finished_turn_keeps_its_tokens_and_a_tool_its_outcome() {
    let store = Store::in_memory();
    let (_, _, _, conversation) = stored_conversation(&store);
    let mut turn = running_turn(&conversation, "Draw the schematic", moment(9));
    turn.use_tool(
        "write".to_owned(),
        "Write".to_owned(),
        String::new(),
        moment(9),
    )
    .unwrap();
    turn.finish_tool("write", true).unwrap();
    turn.fail("The file is locked.".to_owned(), Some(tokens()), moment(10))
        .unwrap();

    store.conversations.save_turn(&turn).unwrap();

    let found = store
        .conversations
        .turns_of(conversation.id())
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(found.state(), TurnState::Failed);
    assert_eq!(found.tokens(), Some(tokens()));
    assert!(matches!(
        &found.steps()[0].content,
        StepContent::Tool {
            outcome: ToolOutcome::Failed,
            ..
        }
    ));
    assert!(store.conversations.running_turns().unwrap().is_empty());
}

/// A finished turn that asked the manager which battery to use.
fn turn_that_asked(conversation: &Conversation) -> Turn {
    let mut turn = running_turn(conversation, "Choose the battery", moment(9));
    turn.ask(
        DecisionRequest::new(
            "Which battery?",
            "It sets the size of the board.",
            vec![
                DecisionOption {
                    label: "The proven way".to_owned(),
                    consequence: "A larger board.".to_owned(),
                },
                DecisionOption {
                    label: "The compact way".to_owned(),
                    consequence: String::new(),
                },
                DecisionOption {
                    label: "Ask the supplier".to_owned(),
                    consequence: "A week of waiting.".to_owned(),
                },
            ],
            Some(2),
        )
        .unwrap(),
    )
    .unwrap();
    turn.finish("I am waiting for your decision.", tokens(), moment(10))
        .unwrap();
    turn
}

#[test]
fn a_turn_with_a_question_is_found_again_unchanged() {
    let store = Store::in_memory();
    let (_, _, _, conversation) = stored_conversation(&store);
    let turn = turn_that_asked(&conversation);

    store.conversations.save_turn(&turn).unwrap();

    assert_eq!(
        store.conversations.turns_of(conversation.id()).unwrap(),
        [turn]
    );
}

#[test]
fn a_turn_with_an_answered_question_is_found_again_unchanged() {
    let store = Store::in_memory();
    let (_, _, _, conversation) = stored_conversation(&store);
    let mut by_option = turn_that_asked(&conversation);
    by_option
        .answer_decision(Some(0), "But keep the connector.", moment(11))
        .unwrap();
    let mut in_words = turn_that_asked(&conversation);
    in_words
        .answer_decision(None, "Neither: use the one we have.", moment(12))
        .unwrap();

    store.conversations.save_turn(&by_option).unwrap();
    store.conversations.save_turn(&in_words).unwrap();

    let found = store.conversations.turns_of(conversation.id()).unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.contains(&by_option));
    assert!(found.contains(&in_words));
}

#[test]
fn asking_and_answering_are_logged_with_the_whole_question() {
    let store = Store::in_memory();
    let (_, _, _, conversation) = stored_conversation(&store);
    let mut turn = running_turn(&conversation, "Choose the battery", moment(9));
    store.conversations.save_turn(&turn).unwrap();
    let asked = turn_that_asked(&conversation);
    // The same turn, now with the question and its end.
    turn = Turn::restore(teamwright_domain::TurnSnapshot {
        id: turn.id(),
        conversation_id: conversation.id(),
        request: asked.request().to_owned(),
        origin: asked.origin(),
        state: asked.state(),
        started_at: asked.started_at(),
        ended_at: asked.ended_at(),
        steps: asked.steps().to_vec(),
        answer: asked.answer().to_owned(),
        failure: None,
        model: None,
        tokens: asked.tokens(),
        decision: asked.decision().cloned(),
    });
    let start = store.database.operations().unwrap().len();

    store.conversations.save_turn(&turn).unwrap();
    let after_asking = logged_after(&store, start);
    store.conversations.save_turn(&turn).unwrap();
    let after_nothing = logged_after(&store, start + after_asking.len());
    turn.answer_decision(Some(1), "", moment(11)).unwrap();
    store.conversations.save_turn(&turn).unwrap();
    let after_answering = logged_after(&store, start + after_asking.len());

    assert_eq!(
        after_asking,
        pairs(&[("turn", "upsert"), ("decision", "upsert")])
    );
    assert!(after_nothing.is_empty());
    assert_eq!(after_answering, pairs(&[("decision", "upsert")]));
    let log = store.database.operations().unwrap();
    let last = log.last().unwrap();
    assert_eq!(last.entity_id, turn.id().to_string());
    let state: serde_json::Value = serde_json::from_str(last.payload.as_deref().unwrap()).unwrap();
    assert_eq!(state["question"], "Which battery?");
    assert_eq!(state["recommended"], 2);
    assert_eq!(state["options"][1]["label"], "The compact way");
    assert_eq!(state["answered_option"], 1);
    assert_eq!(state["answer_note"], "");
    assert_eq!(
        store.conversations.turns_of(conversation.id()).unwrap(),
        [turn]
    );
}

#[test]
fn a_conversation_without_its_member_is_kept_and_logged() {
    let store = Store::in_memory();
    let (_, member, _, mut conversation) = stored_conversation(&store);

    conversation.forget_member();
    store
        .conversations
        .save_conversation(&conversation)
        .unwrap();
    store.members.delete(member.id()).unwrap();

    let found = store
        .conversations
        .find(conversation.id())
        .unwrap()
        .unwrap();
    assert_eq!(found.member_id(), None);
    let log = store.database.operations().unwrap();
    let conversation_state: serde_json::Value = log
        .iter()
        .rev()
        .find(|op| op.entity == "conversation")
        .and_then(|op| op.payload.as_deref())
        .map(|payload| serde_json::from_str(payload).unwrap())
        .unwrap();
    assert_eq!(conversation_state["member_id"], serde_json::Value::Null);
}

#[test]
fn deleting_conversations_removes_their_turns_and_logs_each_removal() {
    let store = Store::in_memory();
    let (project, member, _, conversation) = stored_conversation(&store);
    let mut turn = running_turn(&conversation, "Draw the schematic", moment(9));
    turn.say("Hello.".to_owned(), moment(9)).unwrap();
    store.conversations.save_turn(&turn).unwrap();
    let kept = Conversation::start(project.id(), member.id(), None, moment(10));
    store.conversations.save_conversation(&kept).unwrap();
    let start = store.database.operations().unwrap().len();

    store
        .conversations
        .delete(&[conversation.id(), ConversationId::generate()])
        .unwrap();

    assert_eq!(store.conversations.find(conversation.id()).unwrap(), None);
    assert!(
        store
            .conversations
            .turns_of(conversation.id())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store.conversations.list_for_project(project.id()).unwrap(),
        [kept]
    );
    assert_eq!(
        logged_after(&store, start),
        pairs(&[("turn", "delete"), ("conversation", "delete")])
    );
}

#[test]
fn deleting_a_project_removes_its_conversations_with_a_trace_in_the_log() {
    let store = Store::in_memory();
    let (project, _, _, conversation) = stored_conversation(&store);
    store
        .conversations
        .save_turn(&running_turn(
            &conversation,
            "Draw the schematic",
            moment(9),
        ))
        .unwrap();

    store.projects.delete(project.id()).unwrap();

    assert_eq!(store.conversations.find(conversation.id()).unwrap(), None);
    assert!(store.conversations.running_turns().unwrap().is_empty());
    let log = store.database.operations().unwrap();
    for entity in ["turn", "conversation", "task", "member", "project"] {
        assert!(
            log.iter()
                .any(|op| op.entity == entity && op.kind == "delete"),
            "the removal of the {entity} is not logged"
        );
    }
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
    let member_repository = Arc::new(SqliteMemberRepository::new(database.clone()));
    let conversation_repository = Arc::new(SqliteConversationRepository::new(database));
    let clock = Arc::new(ManualClock::starting_at(moment(9)));
    let projects = ProjectService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        project_repository,
        member_repository,
        task_repository,
        conversation_repository,
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
            model: None,
            effort: None,
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
    let conversation_repository = Arc::new(SqliteConversationRepository::new(database.clone()));
    let clock = Arc::new(ManualClock::starting_at(moment(9)));
    let projects = ProjectService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        project_repository,
        member_repository,
        task_repository,
        conversation_repository,
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
        model: None,
        effort: None,
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

#[test]
fn the_work_of_a_specialist_is_kept_on_the_real_database() {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let project_repository = Arc::new(SqliteProjectRepository::new(database.clone()));
    let task_repository = Arc::new(SqliteTaskRepository::new(database.clone()));
    let member_repository = Arc::new(SqliteMemberRepository::new(database.clone()));
    let conversation_repository = Arc::new(SqliteConversationRepository::new(database.clone()));
    let clock = Arc::new(ManualClock::starting_at(moment(9)));
    let runtime = Arc::new(ScriptedRuntime::new());
    let work_service = || {
        WorkService::new(WorkPorts {
            projects: project_repository.clone(),
            tasks: task_repository.clone(),
            members: member_repository.clone(),
            conversations: conversation_repository.clone(),
            folders: Arc::new(FixedProjectFolders::inside("projects")),
            runtime: runtime.clone(),
            listener: Arc::new(RecordingWorkListener::default()),
            clock: clock.clone(),
        })
    };
    let projects = ProjectService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        project_repository.clone(),
        task_repository.clone(),
        member_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        project_repository.clone(),
        member_repository.clone(),
        task_repository.clone(),
        conversation_repository.clone(),
        clock.clone(),
    );
    let work = work_service();
    let project = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let engineer = team
        .add(NewMember {
            project_id: project.id(),
            name: "Hardware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    let schematic = tasks
        .create(NewTask {
            assignee: Some(Assignee::Member(engineer.id())),
            ..NewTask::titled(project.id(), "Draw the schematic")
        })
        .unwrap();

    // A turn works and reports.
    let started = work.start_work(schematic.id()).unwrap();
    runtime.deliver(
        0,
        AgentEvent::ToolUse {
            id: "read".to_owned(),
            tool: "Read".to_owned(),
            summary: "notes.md".to_owned(),
        },
    );
    runtime.deliver(
        0,
        AgentEvent::ToolResult {
            tool_use_id: "read".to_owned(),
            tool: None,
            failed: false,
        },
    );
    runtime.deliver(
        0,
        AgentEvent::Text {
            text: "The report.".to_owned(),
        },
    );
    runtime.deliver(
        0,
        AgentEvent::TurnEnded(TurnOutcome {
            session_id: Some(SessionId::parse("abc-123").unwrap()),
            failure: None,
            final_text: "The report.".to_owned(),
            usage: vec![ModelUsage {
                model: "claude-opus-model".to_owned(),
                tokens: teamwright_application::TokenCounts {
                    input: 1,
                    output: 2,
                    cache_read: 3,
                    cache_written: 4,
                },
            }],
            estimated_cost_usd: None,
            duration: None,
            model_calls: None,
            permission_denials: Vec::new(),
        }),
    );

    let kept = work.list_for_project(project.id()).unwrap();
    assert_eq!(kept.len(), 1);
    let turn = &kept[0].turns[0];
    assert_eq!(turn.state(), TurnState::Finished);
    assert_eq!(turn.answer(), "The report.");
    assert_eq!(turn.steps().len(), 1);
    let in_review = project
        .workflow()
        .first_in(StatusCategory::Review)
        .unwrap()
        .id();
    assert_eq!(tasks.get(schematic.id()).unwrap().status_id(), in_review);

    // A second turn is cut off by closing the program.
    work.reply(started.conversation.id(), "Use the smaller regulator.")
        .unwrap();
    assert_eq!(
        team.remove(engineer.id()).unwrap_err().code(),
        "member_busy"
    );
    work_service().recover().unwrap();
    let turns = conversation_repository
        .turns_of(started.conversation.id())
        .unwrap();
    assert_eq!(turns[1].state(), TurnState::Failed);

    // A third turn asks the manager to decide, and the manager answers.
    let id = started.conversation.id();
    work.reply(id, "Choose the regulator.").unwrap();
    runtime.deliver(
        2,
        AgentEvent::ManagerAsked(ManagerQuestion {
            question: "Which regulator?".to_owned(),
            reason: String::new(),
            options: ["The smaller one", "The cheaper one"]
                .map(|label| QuestionOption {
                    label: label.to_owned(),
                    consequence: String::new(),
                })
                .to_vec(),
            recommended: Some(0),
        }),
    );
    runtime.deliver(
        2,
        AgentEvent::TurnEnded(TurnOutcome {
            session_id: Some(SessionId::parse("abc-123").unwrap()),
            failure: None,
            final_text: "I am waiting for your decision.".to_owned(),
            usage: Vec::new(),
            estimated_cost_usd: None,
            duration: None,
            model_calls: None,
            permission_denials: Vec::new(),
        }),
    );
    assert_eq!(work.waiting_for_manager().unwrap().len(), 1);
    work.answer_decision(id, Some(1), "").unwrap();
    work.stop(id).unwrap();
    let turns = conversation_repository.turns_of(id).unwrap();
    let answer = turns[2]
        .decision()
        .and_then(DecisionRequest::answer)
        .unwrap();
    assert_eq!(answer.option, Some(1));
    assert_eq!(turns[3].origin(), TurnOrigin::Decision);
    assert_eq!(turns[3].request(), "Decision: The cheaper one");
    assert!(work.waiting_for_manager().unwrap().is_empty());

    // The specialist leaves, the task goes, and the log shows each step.
    team.remove(engineer.id()).unwrap();
    assert_eq!(
        conversation_repository
            .find(started.conversation.id())
            .unwrap()
            .unwrap()
            .member_id(),
        None
    );
    tasks.delete(schematic.id()).unwrap();
    assert!(work.list_for_project(project.id()).unwrap().is_empty());
    let log = database.operations().unwrap();
    assert!(
        log.iter()
            .any(|op| op.entity == "conversation" && op.kind == "delete")
    );
    assert!(
        log.iter()
            .any(|op| op.entity == "turn" && op.kind == "delete")
    );
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
