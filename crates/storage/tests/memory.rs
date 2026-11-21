//! The memory of a project, in real SQLite databases: what is stored, what
//! is logged, what outlives a task and goes with a project, and what
//! reaches another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    MemoryRepository, NewMember, NewProject, NewTask, ProjectService, TaskService, TeamService,
};
use teamwright_domain::{
    ConversationId, Member, MemberKind, MemoryContent, MemoryEntry, MemoryKind, MemoryOrigin,
    Project, SaidIn, Task, Timestamp, TurnId,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteMemberRepository,
    SqliteMemoryRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
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
    memory: SqliteMemoryRepository,
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
            memory: SqliteMemoryRepository::new(database.clone()),
            database,
        }
    }

    fn take_in_from(&self, other: &Self) -> usize {
        let operations = other.database.operations_to_send(0).unwrap();
        self.database.take_in(&operations).unwrap().taken_in
    }

    /// The entity and the kind of every operation logged after the first
    /// `from`.
    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn memory(&self, project: &Project) -> Vec<MemoryEntry> {
        self.memory.entries_of_project(project.id()).unwrap()
    }
}

/// A computer with a project, a task and a specialist.
struct Scene {
    desk: Computer,
    project: Project,
    task: Task,
    hardware: Member,
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
    let hardware = desk
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
    Scene {
        desk,
        project,
        task,
        hardware,
    }
}

impl Scene {
    /// A fact the specialist remembered about the task.
    fn fact(&self, minute: i64) -> MemoryEntry {
        MemoryEntry::record(
            self.project.id(),
            MemoryKind::Fact,
            MemoryContent::new("Display bus", "The display is on SPI2.").unwrap(),
            MemoryOrigin::Specialist(SaidIn {
                member_id: self.hardware.id(),
                conversation_id: ConversationId::generate(),
                turn_id: TurnId::generate(),
            }),
            Some(self.task.id()),
            moment(minute),
        )
    }

    /// A decision the manager wrote, for the whole project.
    fn decision(&self, minute: i64) -> MemoryEntry {
        MemoryEntry::record(
            self.project.id(),
            MemoryKind::Decision,
            MemoryContent::new("Power", "Battery only.").unwrap(),
            MemoryOrigin::Manager,
            None,
            moment(minute),
        )
    }
}

#[test]
fn entries_are_stored_logged_and_read_back_unchanged() {
    let scene = scene();
    let desk = &scene.desk;
    let before = desk.database.operations().unwrap().len();
    let later = scene.decision(5);
    let earlier = scene.fact(1);

    desk.memory
        .save_entries(&[later.clone(), earlier.clone()])
        .unwrap();

    assert_eq!(desk.memory(&scene.project), [earlier.clone(), later]);
    assert_eq!(desk.memory.find_entry(earlier.id()).unwrap(), Some(earlier));
    assert_eq!(
        desk.logged_since(before),
        [
            ("memory_entry".to_owned(), "upsert".to_owned()),
            ("memory_entry".to_owned(), "upsert".to_owned()),
        ]
    );
}

#[test]
fn a_replacement_is_stored_with_the_entry_it_replaces_and_a_removal_is_logged() {
    let scene = scene();
    let desk = &scene.desk;
    let mut older = scene.fact(1);
    desk.memory.save_entries(&[older.clone()]).unwrap();

    let newer = MemoryEntry::replace(
        &mut older,
        MemoryContent::new("Display bus", "The display is on SPI1.").unwrap(),
        MemoryOrigin::Manager,
        moment(3),
    )
    .unwrap();
    desk.memory
        .save_entries(&[newer.clone(), older.clone()])
        .unwrap();

    assert_eq!(desk.memory(&scene.project), [older, newer.clone()]);

    let before = desk.database.operations().unwrap().len();
    desk.memory.delete_entry(newer.id()).unwrap();
    desk.memory.delete_entry(newer.id()).unwrap();

    assert_eq!(desk.memory(&scene.project).len(), 1);
    assert_eq!(
        desk.logged_since(before),
        [("memory_entry".to_owned(), "delete".to_owned())]
    );
}

#[test]
fn entries_outlive_the_task_they_came_from() {
    let scene = scene();
    let desk = &scene.desk;
    let fact = scene.fact(1);
    desk.memory
        .save_entries(std::slice::from_ref(&fact))
        .unwrap();

    desk.tasks.delete(scene.task.id()).unwrap();

    assert_eq!(desk.memory(&scene.project), [fact]);
}

#[test]
fn entries_go_with_their_project_and_each_removal_is_logged() {
    let scene = scene();
    let desk = &scene.desk;
    desk.memory
        .save_entries(&[scene.fact(1), scene.decision(2)])
        .unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.projects.delete(scene.project.id()).unwrap();

    let removed = desk
        .logged_since(before)
        .iter()
        .filter(|logged| *logged == &("memory_entry".to_owned(), "delete".to_owned()))
        .count();
    assert_eq!(removed, 2);
    assert!(desk.memory(&scene.project).is_empty());
}

#[test]
fn entries_reach_the_other_computer_and_a_replacement_follows() {
    let scene = scene();
    let laptop = Computer::new();
    let mut fact = scene.fact(1);
    scene.desk.memory.save_entries(&[fact.clone()]).unwrap();

    laptop.take_in_from(&scene.desk);
    assert_eq!(laptop.memory(&scene.project), [fact.clone()]);

    let newer = MemoryEntry::replace(
        &mut fact,
        MemoryContent::new("Display bus", "SPI1 after all.").unwrap(),
        MemoryOrigin::Manager,
        moment(4),
    )
    .unwrap();
    laptop
        .memory
        .save_entries(&[newer.clone(), fact.clone()])
        .unwrap();
    scene.desk.take_in_from(&laptop);

    assert_eq!(scene.desk.memory(&scene.project), [fact, newer]);
}
