//! What specialists hand each other, in real SQLite databases: what is
//! stored, what is logged, what goes with a task or a project, and what
//! reaches another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    NewMember, NewProject, NewTask, ProjectService, TaskService, TeamService, TeamworkRepository,
};
use teamwright_domain::{
    ConversationId, Handoff, HandoffContent, Member, MemberKind, MessageKind, Project, SaidIn,
    Task, TeamMessage, Timestamp, TurnId,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqliteProjectRepository, SqliteSprintRepository, SqliteTaskRepository,
    SqliteTeamworkRepository, SqliteViewRepository,
};

fn moment(minutes: i64) -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// One computer: its database and the use cases on it.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
    teamwork: SqliteTeamworkRepository,
}

impl Computer {
    fn new() -> Self {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
        let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
        let members = Arc::new(SqliteMemberRepository::new(database.clone()));
        let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
        let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
        let views = Arc::new(SqliteViewRepository::new(database.clone()));
        let areas = Arc::new(SqliteAreaRepository::new(database.clone()));
        let clock = Arc::new(ManualClock::starting_at(moment(0)));
        Self {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(SqlitePlacesRepository::new(database.clone())),
                sprints.clone(),
                views.clone(),
                areas.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                sprints,
                areas,
                clock.clone(),
            ),
            team: TeamService::new(projects, members, tasks, conversations, views, clock),
            teamwork: SqliteTeamworkRepository::new(database.clone()),
            database,
        }
    }

    fn take_in_from(&self, other: &Self) -> usize {
        let operations = other.database.operations_to_send(0).unwrap();
        self.database.take_in(&operations).unwrap().taken_in
    }

    fn specialist(&self, project: &Project, name: &str) -> Member {
        self.team
            .add(NewMember {
                project_id: project.id(),
                name: name.to_owned(),
                kind: MemberKind::Agent,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap()
    }

    /// The entity and the kind of every operation logged after the first
    /// `from`.
    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn handoffs(&self, project: &Project) -> Vec<Handoff> {
        self.teamwork.handoffs_of_project(project.id()).unwrap()
    }

    fn messages(&self, project: &Project) -> Vec<TeamMessage> {
        self.teamwork.messages_of_project(project.id()).unwrap()
    }
}

/// A computer with a project, a task and two specialists.
struct Scene {
    desk: Computer,
    project: Project,
    task: Task,
    hardware: Member,
    firmware: Member,
}

fn scene() -> Scene {
    let desk = Computer::new();
    let project = desk
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let task = desk
        .tasks
        .create(NewTask::titled(project.id(), "Draw the pin map"))
        .unwrap();
    let hardware = desk.specialist(&project, "Hardware engineer");
    let firmware = desk.specialist(&project, "Firmware engineer");
    Scene {
        desk,
        project,
        task,
        hardware,
        firmware,
    }
}

fn said_by(member: &Member) -> SaidIn {
    SaidIn {
        member_id: member.id(),
        conversation_id: ConversationId::generate(),
        turn_id: TurnId::generate(),
    }
}

impl Scene {
    fn handoff(&self, minute: i64) -> Handoff {
        Handoff::publish(
            self.project.id(),
            self.task.id(),
            said_by(&self.hardware),
            HandoffContent::new(
                "Pin map v1.",
                "SDA on pin 4.",
                &["hardware/pinmap.md".to_owned(), "notes.md".to_owned()],
            )
            .unwrap(),
            moment(minute),
        )
    }

    fn message(&self, minute: i64) -> TeamMessage {
        TeamMessage::send(
            self.project.id(),
            said_by(&self.hardware),
            self.firmware.id(),
            MessageKind::Question,
            "Which pins are free?",
            moment(minute),
        )
        .unwrap()
    }
}

#[test]
fn a_handoff_and_a_message_are_stored_logged_and_read_back_unchanged() {
    let scene = scene();
    let desk = &scene.desk;
    let before = desk.database.operations().unwrap().len();
    let later = scene.handoff(5);
    let earlier = scene.handoff(1);
    let mut message = scene.message(2);

    desk.teamwork.save_handoff(&later).unwrap();
    desk.teamwork.save_handoff(&earlier).unwrap();
    desk.teamwork.save_messages(&[message.clone()]).unwrap();

    assert_eq!(desk.handoffs(&scene.project), [earlier, later]);
    assert_eq!(desk.messages(&scene.project), [message.clone()]);
    assert_eq!(
        desk.logged_since(before),
        [
            ("handoff".to_owned(), "upsert".to_owned()),
            ("handoff".to_owned(), "upsert".to_owned()),
            ("team_message".to_owned(), "upsert".to_owned()),
        ]
    );

    let before = desk.database.operations().unwrap().len();
    message.deliver(TurnId::generate(), moment(9));
    desk.teamwork.save_messages(&[message.clone()]).unwrap();

    assert_eq!(desk.messages(&scene.project), [message]);
    assert_eq!(
        desk.logged_since(before),
        [("team_message".to_owned(), "upsert".to_owned())]
    );
}

#[test]
fn the_handoffs_of_a_task_go_with_it_and_each_removal_is_logged() {
    let scene = scene();
    let desk = &scene.desk;
    desk.teamwork.save_handoff(&scene.handoff(1)).unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.tasks.delete(scene.task.id()).unwrap();

    assert!(desk.handoffs(&scene.project).is_empty());
    let logged = desk.logged_since(before);
    assert_eq!(logged[0], ("handoff".to_owned(), "delete".to_owned()));
    assert!(logged.contains(&("task".to_owned(), "delete".to_owned())));
}

#[test]
fn handoffs_and_messages_go_with_their_project_and_each_removal_is_logged() {
    let scene = scene();
    let desk = &scene.desk;
    desk.teamwork.save_handoff(&scene.handoff(1)).unwrap();
    desk.teamwork.save_messages(&[scene.message(2)]).unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.projects.delete(scene.project.id()).unwrap();

    let logged = desk.logged_since(before);
    assert!(logged.contains(&("handoff".to_owned(), "delete".to_owned())));
    assert!(logged.contains(&("team_message".to_owned(), "delete".to_owned())));
    assert!(desk.handoffs(&scene.project).is_empty());
    assert!(desk.messages(&scene.project).is_empty());
}

#[test]
fn handoffs_and_messages_reach_the_other_computer_and_a_delivery_follows() {
    let scene = scene();
    let laptop = Computer::new();
    let handoff = scene.handoff(1);
    let mut message = scene.message(2);
    scene.desk.teamwork.save_handoff(&handoff).unwrap();
    scene
        .desk
        .teamwork
        .save_messages(&[message.clone()])
        .unwrap();

    laptop.take_in_from(&scene.desk);

    assert_eq!(laptop.handoffs(&scene.project), [handoff]);
    assert_eq!(laptop.messages(&scene.project), [message.clone()]);

    message.deliver(TurnId::generate(), moment(3));
    laptop.teamwork.save_messages(&[message.clone()]).unwrap();
    scene.desk.take_in_from(&laptop);

    assert_eq!(scene.desk.messages(&scene.project), [message]);
}

#[test]
fn a_handoff_of_a_task_deleted_elsewhere_is_dropped_and_its_task_takes_the_others_with_it() {
    let scene = scene();
    let laptop = Computer::new();
    laptop.take_in_from(&scene.desk);
    let kept_elsewhere = scene.handoff(1);
    laptop.teamwork.save_handoff(&kept_elsewhere).unwrap();
    let published_here = scene.handoff(2);
    scene.desk.teamwork.save_handoff(&published_here).unwrap();
    scene.desk.tasks.delete(scene.task.id()).unwrap();

    scene.desk.take_in_from(&laptop);
    laptop.take_in_from(&scene.desk);

    assert!(scene.desk.handoffs(&scene.project).is_empty());
    assert!(laptop.handoffs(&scene.project).is_empty());
}
