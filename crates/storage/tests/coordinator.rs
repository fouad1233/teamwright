//! The coordinator, its budgets and its record, in real SQLite databases:
//! what is stored, what is logged, what goes with a project, and what
//! reaches another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    CoordinatorRepository, NewMember, NewProject, NewTask, ProjectService, TaskService, TeamService,
};
use teamwright_domain::{
    Budget, BudgetAnswer, BudgetScope, ComputerId, ConcurrencyLimit, Coordinator, CoordinatorEntry,
    CoordinatorEntryKind, MemberKind, Project, Timestamp, TokenBudget,
};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteCoordinatorRepository,
    SqliteMemberRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
};

fn moment(minutes: i64) -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 21, 22, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// One computer: its database and the use cases on it.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
    coordinators: SqliteCoordinatorRepository,
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
            coordinators: SqliteCoordinatorRepository::new(database.clone()),
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

    fn logged(&self) -> usize {
        self.database.operations().unwrap().len()
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

fn entry(project: &Project, minute: i64, text: &str) -> CoordinatorEntry {
    CoordinatorEntry::new(
        project.id(),
        ComputerId::generate(),
        CoordinatorEntryKind::Started,
        text,
        moment(minute),
    )
}

#[test]
fn a_coordinator_is_stored_logged_and_read_back_unchanged() {
    let desk = Computer::new();
    let project = project_on(&desk);
    assert_eq!(
        desk.coordinators.coordinator_of(project.id()).unwrap(),
        None
    );

    let mut coordinator = Coordinator::of(project.id());
    coordinator.set_limit(ConcurrencyLimit::new(3).unwrap());
    coordinator.switch_on(ComputerId::generate(), moment(1));
    let before = desk.logged();
    desk.coordinators.save_coordinator(&coordinator).unwrap();
    assert_eq!(
        desk.coordinators.coordinator_of(project.id()).unwrap(),
        Some(coordinator.clone())
    );
    coordinator.switch_off(moment(2));
    desk.coordinators.save_coordinator(&coordinator).unwrap();
    assert_eq!(
        desk.coordinators.coordinator_of(project.id()).unwrap(),
        Some(coordinator)
    );
    assert_eq!(
        desk.logged_since(before),
        vec![
            ("coordinator".to_owned(), "upsert".to_owned()),
            ("coordinator".to_owned(), "upsert".to_owned()),
        ]
    );
}

#[test]
fn a_budget_with_its_question_and_answer_is_stored_and_read_back() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let member = desk
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
    let task = desk
        .tasks
        .create(NewTask::titled(project.id(), "Draw the schematic"))
        .unwrap();

    let mut of_project = Budget::new(
        project.id(),
        BudgetScope::Project,
        TokenBudget::new(1_000_000).unwrap(),
    );
    of_project.ask(1_040_000, moment(1)).unwrap();
    of_project
        .answer(BudgetAnswer::Raised { by: 500_000 }, moment(2))
        .unwrap();
    let mut of_member = Budget::new(
        project.id(),
        BudgetScope::Member(member.id()),
        TokenBudget::new(200_000).unwrap(),
    );
    of_member.ask(210_000, moment(3)).unwrap();
    of_member
        .answer(BudgetAnswer::LeftStopped, moment(4))
        .unwrap();
    let mut of_task = Budget::new(
        project.id(),
        BudgetScope::Task(task.id()),
        TokenBudget::new(50_000).unwrap(),
    );
    of_task.ask(51_000, moment(5)).unwrap();
    let mut taken_away = Budget::new(
        project.id(),
        BudgetScope::Task(task.id()),
        TokenBudget::new(9_000).unwrap(),
    );
    taken_away.set(None);

    for budget in [&of_project, &of_member, &of_task] {
        desk.coordinators.save_budget(budget).unwrap();
    }
    let mut stored = desk.coordinators.budgets_of_project(project.id()).unwrap();
    stored.sort_by_key(|budget| budget.scope());
    let mut expected = vec![of_project, of_member, of_task];
    expected.sort_by_key(|budget| budget.scope());
    assert_eq!(stored, expected);

    // The budget of the task, saved again without tokens, is the same row.
    desk.coordinators.save_budget(&taken_away).unwrap();
    let stored = desk.coordinators.budgets_of_project(project.id()).unwrap();
    assert_eq!(stored.len(), 3);
    assert!(
        stored
            .iter()
            .any(|budget| budget.scope() == BudgetScope::Task(task.id())
                && budget.tokens().is_none())
    );
}

#[test]
fn the_record_is_read_the_newest_first_and_since_a_moment() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let entries: Vec<CoordinatorEntry> = (0..5)
        .map(|minute| entry(&project, minute, &format!("Entry {minute}")))
        .collect();
    desk.coordinators.add_entries(&entries).unwrap();
    let latest = desk.coordinators.latest_entries(project.id(), 2).unwrap();
    assert_eq!(
        latest
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Entry 4", "Entry 3"]
    );
    let since = desk
        .coordinators
        .entries_since(project.id(), moment(3))
        .unwrap();
    assert_eq!(since.len(), 2);
    assert_eq!(since[1], entries[3]);
}

#[test]
fn the_coordinator_its_budgets_and_its_record_go_with_the_project() {
    let desk = Computer::new();
    let project = project_on(&desk);
    let mut coordinator = Coordinator::of(project.id());
    coordinator.switch_on(ComputerId::generate(), moment(1));
    desk.coordinators.save_coordinator(&coordinator).unwrap();
    desk.coordinators
        .save_budget(&Budget::new(
            project.id(),
            BudgetScope::Project,
            TokenBudget::new(5_000).unwrap(),
        ))
        .unwrap();
    desk.coordinators
        .add_entries(&[entry(&project, 1, "Started.")])
        .unwrap();
    let before = desk.logged();
    desk.projects.delete(project.id()).unwrap();
    let removed: Vec<String> = desk
        .logged_since(before)
        .into_iter()
        .filter(|(_, kind)| kind == "delete")
        .map(|(entity, _)| entity)
        .collect();
    for entity in ["coordinator", "budget", "coordinator_entry"] {
        assert!(removed.contains(&entity.to_owned()), "{entity} was logged");
    }
    assert_eq!(
        desk.coordinators.coordinator_of(project.id()).unwrap(),
        None
    );
}

#[test]
fn the_switch_and_the_limit_merge_on_their_own_between_computers() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = project_on(&desk);
    let desk_id = desk.database.device_id();
    // Both know the coordinator. (Two computers that write it for the
    // first time at once each write every field, and the later write wins
    // whole, like any thing two computers create under one identifier.)
    desk.coordinators
        .save_coordinator(&Coordinator::of(project.id()))
        .unwrap();
    laptop.take_in_from(&desk);

    // The desk switches it on, while the laptop changes the limit.
    let mut on_desk = desk
        .coordinators
        .coordinator_of(project.id())
        .unwrap()
        .unwrap();
    on_desk.switch_on(desk_id, moment(1));
    desk.coordinators.save_coordinator(&on_desk).unwrap();
    let mut on_laptop = laptop
        .coordinators
        .coordinator_of(project.id())
        .unwrap()
        .unwrap();
    on_laptop.set_limit(ConcurrencyLimit::new(4).unwrap());
    laptop.coordinators.save_coordinator(&on_laptop).unwrap();

    laptop.take_in_from(&desk);
    desk.take_in_from(&laptop);
    for computer in [&desk, &laptop] {
        let merged = computer
            .coordinators
            .coordinator_of(project.id())
            .unwrap()
            .unwrap();
        assert_eq!(merged.runs_on(), Some(desk_id), "the switch of the desk");
        assert_eq!(merged.limit().value(), 4, "the limit of the laptop");
    }
}

#[test]
fn a_budget_and_the_record_reach_the_other_computer() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = project_on(&desk);
    let mut budget = Budget::new(
        project.id(),
        BudgetScope::Project,
        TokenBudget::new(5_000).unwrap(),
    );
    budget.ask(6_000, moment(2)).unwrap();
    desk.coordinators.save_budget(&budget).unwrap();
    desk.coordinators
        .add_entries(&[entry(&project, 1, "Started.")])
        .unwrap();
    laptop.take_in_from(&desk);
    assert_eq!(
        laptop
            .coordinators
            .budgets_of_project(project.id())
            .unwrap(),
        vec![budget.clone()]
    );
    assert_eq!(
        laptop
            .coordinators
            .latest_entries(project.id(), 10)
            .unwrap()[0]
            .text,
        "Started."
    );

    // The manager answers on the laptop; the desk takes the answer in.
    budget
        .answer(BudgetAnswer::Raised { by: 5_000 }, moment(3))
        .unwrap();
    laptop.coordinators.save_budget(&budget).unwrap();
    desk.take_in_from(&laptop);
    let on_desk = desk.coordinators.budgets_of_project(project.id()).unwrap();
    assert_eq!(on_desk, vec![budget]);
    assert_eq!(on_desk[0].tokens().unwrap().tokens(), 10_000);
}
