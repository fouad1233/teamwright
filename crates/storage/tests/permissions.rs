//! Permission requests in real SQLite databases (decision 0038): what is
//! stored, what is logged, what goes with a project, and what reaches
//! another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    ExchangedChange, ExchangedOperation, NewProject, PermissionRepository, ProjectService,
};
use teamwright_domain::{
    Answered, ComputerId, ConversationId, MemberId, PermissionKind, PermissionRequest,
    PermissionRule, PermissionScope, Project, SaidIn, StartedBy, TaskId, Timestamp, TurnAbout,
    TurnId, granted_rules,
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
            PermissionScope::Task,
            PermissionRule::new(PermissionKind::Command, "npm run build").unwrap(),
            "Only the build of the project.",
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
    // The laptop sees the answer, and knows it did not write it.
    let on_the_laptop = laptop.requests(&project);
    let seen = on_the_laptop[0].answer().unwrap();
    let given = asked.answer().unwrap();
    assert_eq!(
        (&seen.decision, seen.computer, seen.next_turn),
        (&given.decision, given.computer, given.next_turn)
    );
    assert!(given.answered_here);
    assert!(!seen.answered_here);

    // Taken back on the laptop, it is taken back on the desk too.
    asked.withdraw(moment(3)).unwrap();
    laptop.permissions.save_request(&asked).unwrap();
    desk.take_in_from(&laptop);
    let on_the_desk = desk.requests(&project);
    assert_eq!(
        on_the_desk[0].answer().unwrap().withdrawn_at,
        Some(moment(3))
    );
    assert!(on_the_desk[0].standing().is_none());
}

/// The turn of the specialist who asked, about its task, on a computer.
fn turn_on(computer: &Computer, request: &PermissionRequest) -> TurnAbout {
    TurnAbout {
        project_id: request.project_id(),
        member_id: request.member_id(),
        task_id: request.task_id(),
        turn_id: TurnId::generate(),
        computer: computer.database.device_id(),
        started_by: StartedBy::Manager,
        no_commands: None,
    }
}

#[test]
fn an_answer_grants_only_on_the_computer_whose_log_wrote_it() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = project_on(&desk);
    laptop.take_in_from(&desk);
    let mut asked = request(&project, 1);
    asked
        .allow(
            PermissionScope::Task,
            PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
            "",
            Answered {
                at: moment(2),
                computer: desk.database.device_id(),
                next_turn: TurnId::generate(),
            },
        )
        .unwrap();
    desk.permissions.save_request(&asked).unwrap();
    laptop.take_in_from(&desk);

    let on_the_desk = desk.requests(&project);
    assert!(on_the_desk[0].answer().unwrap().answered_here);
    assert_eq!(
        granted_rules(&on_the_desk, &turn_on(&desk, &asked)).len(),
        1
    );
    let on_the_laptop = laptop.requests(&project);
    assert!(!on_the_laptop[0].answer().unwrap().answered_here);
    assert!(granted_rules(&on_the_laptop, &turn_on(&laptop, &asked)).is_empty());
}

#[test]
fn an_answer_that_names_this_computer_but_came_through_the_sync_folder_grants_nothing() {
    let desk = Computer::new();
    let intruder = Computer::new();
    let project = project_on(&desk);
    intruder.take_in_from(&desk);
    // Whoever can write in the sync folder writes an answer that claims to
    // be the desk's, for a web domain always, which a domain may be.
    let mut forged = PermissionRequest::ask(
        project.id(),
        SaidIn {
            member_id: MemberId::generate(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        },
        None,
        PermissionRule::new(PermissionKind::WebDomain, "example.com").unwrap(),
        "Anything.",
        moment(1),
    )
    .unwrap();
    forged
        .allow(
            PermissionScope::Always,
            PermissionRule::new(PermissionKind::WebDomain, "example.com").unwrap(),
            "",
            Answered {
                at: moment(2),
                computer: desk.database.device_id(),
                next_turn: TurnId::generate(),
            },
        )
        .unwrap();
    intruder.permissions.save_request(&forged).unwrap();
    desk.take_in_from(&intruder);

    let on_the_desk = desk.requests(&project);
    assert_eq!(on_the_desk.len(), 1, "it is shown");
    let answer = on_the_desk[0].answer().unwrap();
    assert_eq!(answer.computer, desk.database.device_id());
    assert!(!answer.answered_here);
    assert!(!answer.given_on(desk.database.device_id()));
    assert!(granted_rules(&on_the_desk, &turn_on(&desk, &forged)).is_empty());
}

#[test]
fn a_prefix_of_commands_from_a_computer_that_still_knew_it_is_not_kept() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let elsewhere = ComputerId::generate();
    let state = serde_json::json!({
        "id": "6f2b3c1e-0000-4000-8000-000000000001",
        "project_id": project.id().to_string(),
        "member_id": MemberId::generate().to_string(),
        "conversation_id": ConversationId::generate().to_string(),
        "turn_id": TurnId::generate().to_string(),
        "task_id": null,
        "kind": "command_prefix",
        "what": "npm run",
        "reason": "The scripts.",
        "asked_at": "2026-11-21T09:01:00.000Z",
        "decision": null, "scope": null, "allowed_kind": null, "allowed_what": null,
        "note": null, "answered_at": null, "answered_on": null, "next_turn_id": null,
        "withdrawn_at": null
    });
    desk.database
        .take_in(&[ExchangedOperation {
            computer: elsewhere,
            sequence: 1,
            stamp: format!("1795770000000-00000-{elsewhere}"),
            entity: "permission_request".to_owned(),
            entity_id: "6f2b3c1e-0000-4000-8000-000000000001".to_owned(),
            change: ExchangedChange::Upsert {
                state: state.to_string(),
                field_stamps: None,
            },
        }])
        .unwrap();
    assert!(desk.requests(&project).is_empty());
}

/// Takes in, on `computer`, one state of a request written by another
/// computer, later than anything else, with only the fields given.
fn take_in_foreign_state(computer: &Computer, id: &str, state: &serde_json::Value) {
    let elsewhere = ComputerId::generate();
    computer
        .database
        .take_in(&[ExchangedOperation {
            computer: elsewhere,
            sequence: 1,
            stamp: format!("1999999999999-00000-{elsewhere}"),
            entity: "permission_request".to_owned(),
            entity_id: id.to_owned(),
            change: ExchangedChange::Upsert {
                state: state.to_string(),
                field_stamps: None,
            },
        }])
        .unwrap();
}

/// A permission allowed on the desk with this rule and scope, asked in a
/// conversation about a task, and taken back.
fn taken_back_on_the_desk(
    desk: &Computer,
    project: &Project,
    rule: PermissionRule,
    scope: PermissionScope,
) -> PermissionRequest {
    let mut asked = PermissionRequest::ask(
        project.id(),
        SaidIn {
            member_id: MemberId::generate(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        },
        Some(TaskId::generate()),
        rule.clone(),
        "To finish the task.",
        moment(1),
    )
    .unwrap();
    asked
        .allow(
            scope,
            rule,
            "",
            Answered {
                at: moment(2),
                computer: desk.database.device_id(),
                next_turn: TurnId::generate(),
            },
        )
        .unwrap();
    desk.permissions.save_request(&asked).unwrap();
    asked.withdraw(moment(3)).unwrap();
    desk.permissions.save_request(&asked).unwrap();
    asked
}

/// The second review (22 November 2026): a state without an answer, from
/// the sync folder, that brings a taken-back permission back and moves it
/// to another specialist and task, must not make it grant anything here.
fn a_foreign_state_without_an_answer_cannot_revive_or_move(
    rule: PermissionRule,
    scope: PermissionScope,
) {
    let desk = Computer::new();
    let project = project_on(&desk);
    let asked = taken_back_on_the_desk(&desk, &project, rule, scope);
    assert!(granted_rules(&desk.requests(&project), &turn_on(&desk, &asked)).is_empty());

    let other_member = MemberId::generate();
    let other_task = TaskId::generate();
    take_in_foreign_state(
        &desk,
        &asked.id().to_string(),
        &serde_json::json!({
            "withdrawn_at": null,
            "member_id": other_member.to_string(),
            "task_id": other_task.to_string(),
        }),
    );

    let on_the_desk = desk.requests(&project);
    assert_eq!(on_the_desk.len(), 1, "it is still shown");
    let moved = &on_the_desk[0];
    // The state was taken in: the request now names the other specialist
    // and task, and is no longer taken back.
    assert_eq!(moved.member_id(), other_member);
    assert_eq!(moved.task_id(), Some(other_task));
    assert!(moved.answer().unwrap().withdrawn_at.is_none());
    // But not all of it was written here, so it grants nothing to anybody.
    assert!(!moved.answer().unwrap().answered_here);
    let for_the_other = TurnAbout {
        member_id: other_member,
        task_id: Some(other_task),
        ..turn_on(&desk, &asked)
    };
    assert!(granted_rules(&on_the_desk, &for_the_other).is_empty());
    assert!(granted_rules(&on_the_desk, &turn_on(&desk, &asked)).is_empty());
}

#[test]
fn a_foreign_state_cannot_revive_or_move_a_command_allowed_for_a_task() {
    a_foreign_state_without_an_answer_cannot_revive_or_move(
        PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
        PermissionScope::Task,
    );
}

#[test]
fn a_foreign_state_cannot_revive_or_move_a_web_domain_allowed_always() {
    a_foreign_state_without_an_answer_cannot_revive_or_move(
        PermissionRule::new(PermissionKind::WebDomain, "docs.rs").unwrap(),
        PermissionScope::Always,
    );
}

#[test]
fn a_foreign_state_that_only_moves_a_standing_permission_ends_it_here() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let mut asked = request(&project, 1);
    asked
        .allow(
            PermissionScope::Task,
            PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
            "",
            Answered {
                at: moment(2),
                computer: desk.database.device_id(),
                next_turn: TurnId::generate(),
            },
        )
        .unwrap();
    desk.permissions.save_request(&asked).unwrap();
    assert_eq!(
        granted_rules(&desk.requests(&project), &turn_on(&desk, &asked)).len(),
        1
    );

    let other_member = MemberId::generate();
    take_in_foreign_state(
        &desk,
        &asked.id().to_string(),
        &serde_json::json!({ "member_id": other_member.to_string() }),
    );
    let on_the_desk = desk.requests(&project);
    let for_the_other = TurnAbout {
        member_id: other_member,
        ..turn_on(&desk, &asked)
    };
    assert!(granted_rules(&on_the_desk, &for_the_other).is_empty());
    assert!(granted_rules(&on_the_desk, &turn_on(&desk, &asked)).is_empty());
}
