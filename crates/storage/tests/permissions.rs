//! Permission requests in real SQLite databases (decision 0038): what is
//! stored, what is logged, what goes with a project, and what reaches
//! another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{NewProject, PermissionRepository, ProjectService};
use teamwright_domain::{
    Answered, ComputerId, ConversationId, MemberId, PermissionKind, PermissionRequest,
    PermissionRule, PermissionScope, Project, SaidIn, TaskId, Timestamp, TurnId,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePermissionRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
};

fn moment(minutes: i64) -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// One computer: its database, its projects and its permission requests.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    permissions: SqlitePermissionRepository,
}

impl Computer {
    fn new() -> Self {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let clock = Arc::new(ManualClock::starting_at(moment(0)));
        Self {
            projects: ProjectService::new(
                Arc::new(SqliteProjectRepository::new(database.clone())),
                Arc::new(SqliteTaskRepository::new(database.clone())),
                Arc::new(SqliteMemberRepository::new(database.clone())),
                Arc::new(SqliteConversationRepository::new(database.clone())),
                Arc::new(SqlitePlacesRepository::new(database.clone())),
                Arc::new(SqliteSprintRepository::new(database.clone())),
                Arc::new(SqliteViewRepository::new(database.clone())),
                Arc::new(SqliteAreaRepository::new(database.clone())),
                clock,
            ),
            permissions: SqlitePermissionRepository::new(database.clone()),
            database,
        }
    }

    fn take_in_from(&self, other: &Self) -> usize {
        let operations = other.database.operations_to_send(0).unwrap();
        self.database.take_in(&operations).unwrap().taken_in
    }

    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn requests(&self, project: &Project) -> Vec<PermissionRequest> {
        self.permissions.requests_of_project(project.id()).unwrap()
    }
}

fn project_on(computer: &Computer) -> Project {
    computer
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap()
}

fn request(project: &Project, minute: i64) -> PermissionRequest {
    PermissionRequest::ask(
        project.id(),
        SaidIn {
            member_id: MemberId::generate(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        },
        Some(TaskId::generate()),
        PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
        "The tests show whether the parser works.",
        moment(minute),
    )
    .unwrap()
}

fn answered(minute: i64) -> Answered {
    Answered {
        at: moment(minute),
        computer: ComputerId::generate(),
        next_turn: TurnId::generate(),
    }
}

#[test]
fn a_request_is_stored_logged_answered_withdrawn_and_read_back_unchanged() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let before = desk.database.operations().unwrap().len();
    let mut later = request(&project, 5);
    let earlier = request(&project, 1);

    desk.permissions.save_request(&later).unwrap();
    desk.permissions.save_request(&earlier).unwrap();
    assert_eq!(desk.requests(&project), [earlier.clone(), later.clone()]);

    later
        .allow(
            PermissionScope::Always,
            PermissionRule::new(PermissionKind::CommandPrefix, "npm run").unwrap(),
            "Only the scripts of the project.",
            answered(6),
        )
        .unwrap();
    desk.permissions.save_request(&later).unwrap();
    assert_eq!(
        desk.permissions.find_request(later.id()).unwrap(),
        Some(later.clone())
    );

    later.withdraw(moment(8)).unwrap();
    desk.permissions.save_request(&later).unwrap();
    assert_eq!(desk.requests(&project), [earlier, later]);
    assert_eq!(
        desk.logged_since(before),
        vec![("permission_request".to_owned(), "upsert".to_owned()); 4]
    );
}

#[test]
fn a_refused_request_is_read_back_refused() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let mut refused = request(&project, 1);
    refused
        .refuse("Not on this computer.", answered(2))
        .unwrap();
    desk.permissions.save_request(&refused).unwrap();
    assert_eq!(desk.requests(&project), [refused]);
}

#[test]
fn requests_go_with_their_project_and_each_removal_is_logged() {
    let desk = Computer::new();
    let project = project_on(&desk);
    desk.permissions
        .save_request(&request(&project, 1))
        .unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.projects.delete(project.id()).unwrap();

    assert!(
        desk.logged_since(before)
            .contains(&("permission_request".to_owned(), "delete".to_owned()))
    );
    assert!(desk.requests(&project).is_empty());
}

#[test]
fn requests_and_their_answers_reach_the_other_computer() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = project_on(&desk);
    let mut asked = request(&project, 1);
    desk.permissions.save_request(&asked).unwrap();

    laptop.take_in_from(&desk);
    assert_eq!(laptop.requests(&project), [asked.clone()]);

    asked
        .allow(
            PermissionScope::Task,
            PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
            "",
            answered(2),
        )
        .unwrap();
    desk.permissions.save_request(&asked).unwrap();
    laptop.take_in_from(&desk);
    assert_eq!(laptop.requests(&project), [asked.clone()]);

    // Taken back on the laptop, it is taken back on the desk too.
    asked.withdraw(moment(3)).unwrap();
    laptop.permissions.save_request(&asked).unwrap();
    desk.take_in_from(&laptop);
    assert_eq!(desk.requests(&project), [asked]);
}
