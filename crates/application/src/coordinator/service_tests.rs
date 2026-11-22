//! The coordinator at work, on the in-memory stand-ins the tests of the
//! work of the specialists use: the scripted runtime starts no program,
//! and each test ends the turns itself.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{
    Assignee, BudgetAnswer, BudgetScope, ComputerId, CoordinatorEntryKind, Member, MemberKind,
    MessageKind, Project, SaidIn, StatusCategory, Task, TaskId, TeamMessage, Timestamp,
};

use super::*;
use crate::agent_runtime::{
    self, AgentEvent, ModelUsage, PlanLimitHit, PlanReset, PlanWindowKind, SignIn, TurnFailure,
    TurnOutcome,
};
use crate::error::ApplicationError;
use crate::in_memory::{
    FixedProjectFolders, InMemoryAreaRepository, InMemoryConversationRepository,
    InMemoryCoordinatorRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
    InMemoryPlanLimitStore, InMemoryPlanningRepository, InMemoryProjectRepository,
    InMemorySprintRepository, InMemoryTaskRepository, InMemoryTeamworkRepository,
    InMemoryViewRepository, ManualClock, PretendFileSystem, ScriptedRuntime,
};
use crate::plan_limit::{PlanLimitService, PlanReport};
use crate::ports::{
    Clock, CoordinatorRepository, ProjectRepository, TeamworkRepository, WorkListener,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};
use crate::work::{WorkPorts, WorkService};

/// What every finished turn of these tests spends, in all.
const TURN_TOKENS: u64 = 10_000;

/// Takes what the work learns about the plan into what is known, as the
/// shell does.
struct PlanLimitListener {
    plan_limit: PlanLimitService,
}

impl WorkListener for PlanLimitListener {
    fn conversation_changed(
        &self,
        _conversation: &teamwright_domain::Conversation,
        _turns: &[teamwright_domain::Turn],
    ) {
    }

    fn plan_reported(&self, report: &PlanReport) {
        self.plan_limit.take(report).unwrap();
    }
}

fn start_of_evening() -> Timestamp {
    chrono::Utc
        .with_ymd_and_hms(2026, 11, 21, 20, 0, 0)
        .unwrap()
}

struct Office {
    tasks: TaskService,
    team: TeamService,
    work: WorkService,
    coordinator: CoordinatorService,
    store: Arc<InMemoryCoordinatorRepository>,
    projects: Arc<InMemoryProjectRepository>,
    teamwork: Arc<InMemoryTeamworkRepository>,
    permissions: Arc<crate::in_memory::InMemoryPermissionRepository>,
    runtime: Arc<ScriptedRuntime>,
    clock: Arc<ManualClock>,
    this_computer: ComputerId,
    plan_limit: PlanLimitService,
    project: Project,
    hardware: Member,
    firmware: Member,
    /// The turns, counted from 0, that the test has ended.
    ended: Mutex<BTreeSet<usize>>,
}

fn office() -> Office {
    office_on(ComputerId::generate())
}

fn office_on(this_computer: ComputerId) -> Office {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members = Arc::new(InMemoryMemberRepository::default());
    let conversations = Arc::new(InMemoryConversationRepository::default());
    let areas = Arc::new(InMemoryAreaRepository::default());
    let views = Arc::new(InMemoryViewRepository::default());
    let teamwork = Arc::new(InMemoryTeamworkRepository::default());
    let store = Arc::new(InMemoryCoordinatorRepository::default());
    let permissions = Arc::new(crate::in_memory::InMemoryPermissionRepository::default());
    let runtime = Arc::new(ScriptedRuntime::new());
    let clock = Arc::new(ManualClock::starting_at(start_of_evening()));
    let plan_limit =
        PlanLimitService::new(Arc::new(InMemoryPlanLimitStore::default()), clock.clone());
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members.clone(),
        conversations.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        Arc::new(InMemorySprintRepository::new(tasks_store.clone())),
        views.clone(),
        areas.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members.clone(),
        conversations.clone(),
        Arc::new(InMemorySprintRepository::new(tasks_store.clone())),
        areas.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store.clone(),
        members.clone(),
        tasks_store.clone(),
        conversations.clone(),
        views,
        clock.clone(),
    );
    let work = WorkService::new(WorkPorts {
        projects: projects_store.clone(),
        tasks: tasks_store.clone(),
        areas,
        members: members.clone(),
        conversations: conversations.clone(),
        planning: Arc::new(InMemoryPlanningRepository::new(
            projects_store.clone(),
            members.clone(),
            tasks_store.clone(),
            conversations.clone(),
        )),
        teamwork: teamwork.clone(),
        digests: Arc::new(crate::in_memory::InMemoryDigestRepository::default()),
        permissions: permissions.clone(),
        command_choice: Arc::new(crate::in_memory::InMemoryCommandChoice::switched_on()),
        memory: Arc::new(crate::in_memory::InMemoryMemoryRepository::default()),
        folders: Arc::new(FixedProjectFolders::inside("projects")),
        places: Arc::new(InMemoryPlacesRepository::default()),
        file_system: Arc::new(PretendFileSystem::default()),
        kits: Arc::new(crate::kits::InMemoryKitRepository::default()),
        shelf: Arc::new(crate::kits::PretendShelf::default()),
        marks: Arc::new(crate::marks::FixedMark::for_tests()),
        trust: Arc::new(crate::kits::InMemoryConnectorTrust::default()),
        runtime: runtime.clone(),
        listener: Arc::new(PlanLimitListener {
            plan_limit: plan_limit.clone(),
        }),
        clock: clock.clone(),
        this_computer,
    });
    let coordinator = CoordinatorService::new(CoordinatorPorts {
        projects: projects_store.clone(),
        tasks: tasks_store,
        members,
        conversations,
        teamwork: teamwork.clone(),
        coordinators: store.clone(),
        clock: clock.clone(),
        this_computer,
        work: work.clone(),
        plan_limit: plan_limit.clone(),
    });
    let project = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    let add = |name: &str| {
        team.add(NewMember {
            project_id: project.id(),
            name: name.to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap()
    };
    let hardware = add("Hardware engineer");
    let firmware = add("Firmware engineer");
    Office {
        tasks,
        team,
        work,
        coordinator,
        store,
        projects: projects_store,
        teamwork,
        permissions,
        runtime,
        clock,
        this_computer,
        plan_limit,
        project,
        hardware,
        firmware,
        ended: Mutex::new(BTreeSet::new()),
    }
}

fn outcome(failure: Option<TurnFailure>) -> AgentEvent {
    AgentEvent::TurnEnded(TurnOutcome {
        session_id: None,
        failure,
        final_text: "Done.".to_owned(),
        usage: vec![ModelUsage {
            model: "claude-opus-model".to_owned(),
            tokens: agent_runtime::TokenCounts {
                input: 0,
                output: TURN_TOKENS,
                cache_read: 0,
                cache_written: 0,
            },
        }],
        estimated_cost_usd: None,
        duration: None,
        model_calls: None,
        permission_denials: Vec::new(),
        context: None,
    })
}

impl Office {
    fn task(&self, title: &str, member: &Member) -> Task {
        self.tasks
            .create(NewTask {
                assignee: Some(Assignee::Member(member.id())),
                ..NewTask::titled(self.project.id(), title)
            })
            .unwrap()
    }

    fn act(&self) -> Vec<String> {
        self.clock.advance(TimeDelta::seconds(1));
        self.coordinator
            .act(self.project.id())
            .unwrap()
            .into_iter()
            .map(|entry| entry.text)
            .collect()
    }

    /// The turns, counted from 0, that were started and not ended.
    fn running(&self) -> Vec<usize> {
        let ended = self.ended.lock().unwrap();
        (0..self.runtime.requests().len())
            .filter(|turn| !ended.contains(turn))
            .collect()
    }

    fn end(&self, turn: usize, failure: Option<TurnFailure>) {
        self.clock.advance(TimeDelta::seconds(1));
        self.runtime.deliver(turn, outcome(failure));
        self.ended.lock().unwrap().insert(turn);
    }

    fn finish_all(&self) {
        for turn in self.running() {
            self.end(turn, None);
        }
    }

    /// The manager accepts every task that waits to be checked.
    fn accept_finished(&self) {
        let project = self.projects.find(self.project.id());
        let workflow = project.unwrap().unwrap().workflow().clone();
        let done = workflow.first_in(StatusCategory::Done).unwrap().id();
        for task in self.tasks.list_for_project(self.project.id()).unwrap() {
            if workflow
                .status(task.status_id())
                .map(|status| status.category())
                == Some(StatusCategory::Review)
            {
                self.tasks.move_to(task.id(), done, usize::MAX).unwrap();
            }
        }
    }

    fn category_of(&self, task: &Task) -> StatusCategory {
        let task = self.tasks.get(task.id()).unwrap();
        let project = self.projects.find(self.project.id());
        project
            .unwrap()
            .unwrap()
            .workflow()
            .status(task.status_id())
            .unwrap()
            .category()
    }

    /// The tasks with a conversation, in the order their work started.
    fn started_tasks(&self) -> Vec<TaskId> {
        self.work
            .list_for_project(self.project.id())
            .unwrap()
            .iter()
            .filter_map(|conversation| conversation.conversation.task_id())
            .collect()
    }

    fn switch_on(&self) {
        self.coordinator.switch_on(self.project.id()).unwrap();
    }

    fn entries(&self) -> Vec<(CoordinatorEntryKind, String)> {
        let mut entries = self.store.latest_entries(self.project.id(), 100).unwrap();
        entries.reverse();
        entries
            .into_iter()
            .map(|entry| (entry.kind, entry.text))
            .collect()
    }
}

#[test]
fn an_office_that_is_off_starts_nothing() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    assert!(office.act().is_empty());
    assert!(office.runtime.requests().is_empty());
}

#[test]
fn a_small_project_runs_to_its_end_by_itself_with_the_manager_accepting_the_work() {
    let office = office();
    let parts = office.task("Choose the parts", &office.hardware);
    let schematic = office.task("Draw the schematic", &office.hardware);
    let drivers = office.task("Write the drivers", &office.firmware);
    let board = office.task("Route the board", &office.hardware);
    office
        .tasks
        .add_dependency(schematic.id(), parts.id())
        .unwrap();
    office
        .tasks
        .add_dependency(board.id(), schematic.id())
        .unwrap();
    office
        .tasks
        .add_dependency(drivers.id(), parts.id())
        .unwrap();
    office.switch_on();

    let mut rounds = 0;
    loop {
        office.act();
        if office.running().is_empty() {
            break;
        }
        office.finish_all();
        office.accept_finished();
        rounds += 1;
        assert!(rounds < 10, "the project should be finished by now");
    }

    for task in [&parts, &schematic, &drivers, &board] {
        assert_eq!(office.category_of(task), StatusCategory::Done);
    }
    assert_eq!(
        office.started_tasks(),
        vec![parts.id(), schematic.id(), drivers.id(), board.id()]
    );
    assert_eq!(
        rounds, 3,
        "parts, then schematic and drivers at once, then the board"
    );
    let started: Vec<String> = office
        .entries()
        .into_iter()
        .filter(|(kind, _)| *kind == CoordinatorEntryKind::Started)
        .map(|(_, text)| text)
        .collect();
    assert_eq!(
        started,
        vec![
            "Hardware engineer started on SSN-1: it waits for nothing.",
            "Hardware engineer started on SSN-2: SSN-1 is done.",
            "Firmware engineer started on SSN-3: SSN-1 is done.",
            "Hardware engineer started on SSN-4: SSN-2 is done.",
        ]
    );
}

#[test]
fn a_limit_of_one_keeps_the_work_in_a_row() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.task("Write the drivers", &office.firmware);
    office
        .coordinator
        .set_limit(office.project.id(), 1)
        .unwrap();
    office.switch_on();
    office.act();
    assert_eq!(office.running().len(), 1);
    office.act();
    assert_eq!(
        office.running().len(),
        1,
        "acting again starts nothing more"
    );
    office.finish_all();
    office.act();
    assert_eq!(office.running().len(), 1);
    assert_eq!(office.runtime.requests().len(), 2);
}

#[test]
fn a_used_up_budget_stops_the_work_and_asks_the_manager_once() {
    let office = office();
    let first = office.task("Draw the schematic", &office.hardware);
    let second = office.task("Write the drivers", &office.firmware);
    office
        .coordinator
        .set_limit(office.project.id(), 1)
        .unwrap();
    office
        .coordinator
        .set_budget(office.project.id(), BudgetScope::Project, Some(TURN_TOKENS))
        .unwrap();
    office.switch_on();
    office.act();
    office.finish_all();
    assert_eq!(office.started_tasks(), vec![first.id()]);

    let said = office.act();
    assert_eq!(
        said,
        vec![
            "Did not start SSN-2: the budget of the project is used up (10,000 of 10,000 tokens). You are asked in the inbox."
        ]
    );
    assert!(office.running().is_empty());
    assert!(office.act().is_empty(), "asked once");
    let questions = office.coordinator.budget_questions().unwrap();
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].name, "the budget of the project");
    assert_eq!(questions[0].used, TURN_TOKENS);

    office
        .coordinator
        .answer_budget(
            office.project.id(),
            BudgetScope::Project,
            BudgetAnswer::Raised { by: 5_000 },
        )
        .unwrap();
    assert!(office.coordinator.budget_questions().unwrap().is_empty());
    office.act();
    assert_eq!(office.started_tasks(), vec![first.id(), second.id()]);
}

#[test]
fn work_left_stopped_stays_stopped_until_the_budget_changes() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.task("Write the drivers", &office.firmware);
    office
        .coordinator
        .set_limit(office.project.id(), 1)
        .unwrap();
    office
        .coordinator
        .set_budget(office.project.id(), BudgetScope::Project, Some(TURN_TOKENS))
        .unwrap();
    office.switch_on();
    office.act();
    office.finish_all();
    office.act();
    office
        .coordinator
        .answer_budget(
            office.project.id(),
            BudgetScope::Project,
            BudgetAnswer::LeftStopped,
        )
        .unwrap();
    assert!(office.act().is_empty());
    assert!(office.running().is_empty());
    office
        .coordinator
        .set_budget(office.project.id(), BudgetScope::Project, None)
        .unwrap();
    office.act();
    assert_eq!(office.running().len(), 1);
}

#[test]
fn a_running_turn_is_never_stopped_by_a_budget() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.switch_on();
    office.act();
    office
        .coordinator
        .set_budget(office.project.id(), BudgetScope::Project, Some(1_000))
        .unwrap();
    office.act();
    assert!(!office.runtime.was_stopped(0));
    assert_eq!(office.running(), vec![0]);
}

#[test]
fn a_failed_turn_is_not_tried_again() {
    let office = office();
    let task = office.task("Draw the schematic", &office.hardware);
    office.switch_on();
    office.act();
    office.end(
        0,
        Some(TurnFailure::Reported {
            kind: Some("error_during_execution".to_owned()),
        }),
    );
    for _ in 0..3 {
        office.act();
    }
    assert_eq!(office.runtime.requests().len(), 1);
    let state = office.coordinator.state(office.project.id()).unwrap();
    assert_eq!(
        state.waiting,
        vec![Waiting {
            task_id: Some(task.id()),
            member_id: office.hardware.id(),
            reason: WaitReason::Failed,
        }]
    );
}

/// How a turn ends when the limit of the plan is reached, resetting at
/// `resets_at`.
fn at_the_limit(resets_at: Timestamp) -> Option<TurnFailure> {
    Some(TurnFailure::PlanLimitReached(PlanLimitHit {
        window: Some(PlanWindowKind::FiveHours),
        resets: PlanReset::At(resets_at),
    }))
}

#[test]
fn a_team_stopped_by_the_limit_of_the_plan_goes_on_by_itself_after_the_reset() {
    let office = office();
    let schematic = office.task("Draw the schematic", &office.hardware);
    let drivers = office.task("Write the drivers", &office.firmware);
    let board = office.task("Route the board", &office.hardware);
    office
        .tasks
        .add_dependency(board.id(), schematic.id())
        .unwrap();
    office.switch_on();
    office.act();
    assert_eq!(office.running(), vec![0, 1]);

    // At 20:15 the limit stops both turns. It resets at 22:40.
    let resets_at = chrono::Utc
        .with_ymd_and_hms(2026, 11, 21, 22, 40, 0)
        .unwrap();
    office.clock.advance(TimeDelta::minutes(15));
    office.end(0, at_the_limit(resets_at));
    office.end(1, at_the_limit(resets_at));
    assert_eq!(
        office.plan_limit.may_start_work().unwrap(),
        WorkPermit::NotUntil(resets_at)
    );
    for task in [&schematic, &drivers] {
        assert_eq!(
            office.category_of(task),
            StatusCategory::Active,
            "the work stays under way"
        );
    }

    // Every nudge while the limit is reached starts nothing, and the
    // record says why once.
    assert_eq!(
        office.act(),
        vec!["Started nothing: the limit of your Claude plan is reached. It resets at 22:40."]
    );
    for _ in 0..3 {
        assert!(office.act().is_empty());
    }
    assert_eq!(office.runtime.requests().len(), 2);
    let state = office.coordinator.state(office.project.id()).unwrap();
    assert!(
        state
            .waiting
            .iter()
            .all(|waiting| waiting.reason == WaitReason::PlanLimit)
    );
    assert_eq!(state.waiting.len(), 2);

    // The shell wakes at the reset, looks, and nudges the coordinator.
    office.clock.advance(TimeDelta::hours(3));
    assert_eq!(
        office.plan_limit.look_at_reset().unwrap(),
        crate::plan_limit::PlanLimitChange::Lifted
    );
    assert_eq!(
        office.act(),
        vec![
            "Hardware engineer went on with SSN-1: the limit of your Claude plan stopped its last turn, and it has reset.",
            "Firmware engineer went on with SSN-2: the limit of your Claude plan stopped its last turn, and it has reset.",
        ]
    );
    let requests = office.runtime.requests();
    assert_eq!(requests.len(), 4);
    assert!(requests[2].prompt.starts_with(GO_ON_REQUEST));
    assert_eq!(
        office.started_tasks(),
        vec![schematic.id(), drivers.id()],
        "it went on in the same conversations"
    );

    // The work goes on to the end, with the manager accepting it.
    office.finish_all();
    office.accept_finished();
    office.act();
    office.finish_all();
    office.accept_finished();
    for task in [&schematic, &drivers, &board] {
        assert_eq!(office.category_of(task), StatusCategory::Done);
    }
    assert_eq!(office.runtime.requests().len(), 5);
}

#[test]
fn a_turn_that_goes_on_and_meets_the_limit_again_waits_for_the_next_reset() {
    let office = office();
    let task = office.task("Draw the schematic", &office.hardware);
    office.switch_on();
    office.act();
    let first_reset = start_of_evening() + TimeDelta::hours(1);
    office.end(0, at_the_limit(first_reset));
    office.act();
    office.clock.advance(TimeDelta::hours(1));
    office.plan_limit.look_at_reset().unwrap();
    office.act();
    assert_eq!(office.runtime.requests().len(), 2, "it went on once");

    // The turn that went on meets a new limit at once.
    let second_reset = first_reset + TimeDelta::hours(5);
    office.end(1, at_the_limit(second_reset));
    assert_eq!(
        office.act(),
        vec![
            "Started nothing: the limit of your Claude plan is reached. It resets tomorrow at 02:00."
        ]
    );
    for _ in 0..3 {
        assert!(office.act().is_empty());
    }
    assert_eq!(office.runtime.requests().len(), 2, "no loop");

    // After the next reset it goes on again.
    office.clock.advance(TimeDelta::hours(5));
    office.plan_limit.look_at_reset().unwrap();
    office.act();
    assert_eq!(office.runtime.requests().len(), 3);
    assert_eq!(office.started_tasks(), vec![task.id()]);
}

#[test]
fn a_limit_misread_as_lifted_does_not_make_a_loop() {
    let office = office();
    let task = office.task("Draw the schematic", &office.hardware);
    office.switch_on();
    office.act();
    // The program gives a reset that has already passed: the limit is
    // never known to be reached.
    let passed = start_of_evening() - TimeDelta::hours(1);
    office.end(0, at_the_limit(passed));
    assert_eq!(
        office.plan_limit.may_start_work().unwrap(),
        WorkPermit::Allowed
    );
    office.act();
    assert_eq!(office.runtime.requests().len(), 2, "it went on once");
    office.end(1, at_the_limit(passed));
    for _ in 0..3 {
        office.act();
    }
    assert_eq!(office.runtime.requests().len(), 2, "and not again");
    let state = office.coordinator.state(office.project.id()).unwrap();
    assert_eq!(
        state.waiting,
        vec![Waiting {
            task_id: Some(task.id()),
            member_id: office.hardware.id(),
            reason: WaitReason::PlanLimitAgain,
        }]
    );
}

#[test]
fn a_limit_whose_reset_is_not_known_holds_work_until_a_turn_goes_through() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.switch_on();
    office.act();
    office.end(
        0,
        Some(TurnFailure::PlanLimitReached(PlanLimitHit {
            window: None,
            resets: PlanReset::Unknown,
        })),
    );
    assert_eq!(
        office.act(),
        vec![
            "Started nothing: the limit of your Claude plan is reached. When it resets is not known: work goes on once a turn you start by hand goes through."
        ]
    );
    assert!(office.act().is_empty());
    assert_eq!(office.runtime.requests().len(), 1);

    // The manager starts a turn by hand, and it goes through.
    office.coordinator.switch_off(office.project.id()).unwrap();
    office
        .work
        .ask(office.firmware.id(), "Is the bus free?")
        .unwrap();
    office.end(1, None);
    assert_eq!(
        office.plan_limit.may_start_work().unwrap(),
        WorkPermit::Allowed
    );
    office.switch_on();
    office.act();
    assert_eq!(office.runtime.requests().len(), 3);
    let prompt = &office.runtime.requests()[2].prompt;
    assert!(prompt.starts_with(GO_ON_REQUEST), "{prompt}");
}

#[test]
fn a_program_that_is_not_signed_in_switches_the_coordinator_off() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.runtime.set_sign_in(Ok(SignIn::SignedOut));
    office.switch_on();
    assert_eq!(
        office.act(),
        vec![
            "Switched itself off: Claude Code is not signed in. Switch it on again once it works."
        ]
    );
    assert!(office.runtime.requests().is_empty());
    let state = office.coordinator.state(office.project.id()).unwrap();
    assert_eq!(state.coordinator.runs_on(), None);
}

#[test]
fn a_coordinator_switched_on_on_another_computer_acts_only_there() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    let mut elsewhere = teamwright_domain::Coordinator::of(office.project.id());
    elsewhere.switch_on(ComputerId::generate(), start_of_evening());
    office.store.save_coordinator(&elsewhere).unwrap();
    assert!(office.act().is_empty());
    assert!(office.runtime.requests().is_empty());
    assert!(office.coordinator.running_here().unwrap().is_empty());
    // Switching it on here moves it here.
    office.switch_on();
    assert_eq!(
        office.coordinator.running_here().unwrap(),
        vec![office.project.id()]
    );
    office.act();
    assert_eq!(office.runtime.requests().len(), 1);
}

#[test]
fn switching_off_lets_work_under_way_go_on_and_starts_nothing_new() {
    let office = office();
    office.task("Draw the schematic", &office.hardware);
    office.task("Route the board", &office.hardware);
    office.switch_on();
    office.act();
    office.coordinator.switch_off(office.project.id()).unwrap();
    assert!(!office.runtime.was_stopped(0));
    office.finish_all();
    office.act();
    assert_eq!(office.runtime.requests().len(), 1);
    let kinds: Vec<CoordinatorEntryKind> =
        office.entries().into_iter().map(|(kind, _)| kind).collect();
    assert_eq!(
        kinds,
        vec![
            CoordinatorEntryKind::SwitchedOn,
            CoordinatorEntryKind::Started,
            CoordinatorEntryKind::SwitchedOff,
        ]
    );
}

fn question(office: &Office, from: &Member, to: &Member) {
    let message = TeamMessage::send(
        office.project.id(),
        SaidIn {
            member_id: from.id(),
            conversation_id: teamwright_domain::ConversationId::generate(),
            turn_id: teamwright_domain::TurnId::generate(),
        },
        to.id(),
        MessageKind::Question,
        "Which pin is the display on?",
        office.clock.now(),
    )
    .unwrap();
    office.teamwork.save_messages(&[message]).unwrap();
}

#[test]
fn a_question_between_teammates_is_answered_without_the_manager() {
    let office = office();
    question(&office, &office.hardware, &office.firmware);
    office.switch_on();
    assert_eq!(
        office.act(),
        vec!["Firmware engineer was given a turn to answer Hardware engineer."]
    );
    let request = &office.runtime.requests()[0];
    assert!(request.prompt.starts_with(WAKE_REQUEST));
    assert!(request.prompt.contains("Which pin is the display on?"));
    assert!(
        office
            .teamwork
            .messages_of_project(office.project.id())
            .unwrap()
            .iter()
            .all(|message| !message.is_waiting()),
        "the message went with the turn"
    );
    // The next question goes on in the same conversation.
    office.finish_all();
    question(&office, &office.hardware, &office.firmware);
    office.act();
    let conversations = office.work.list_for_project(office.project.id()).unwrap();
    assert_eq!(conversations.len(), 1);
    assert_eq!(conversations[0].turns.len(), 2);
}

#[test]
fn two_specialists_cannot_ask_each_other_for_ever() {
    let office = office();
    office.switch_on();
    let mut turns = 0;
    for round in 0..10 {
        let (from, to) = if round % 2 == 0 {
            (&office.hardware, &office.firmware)
        } else {
            (&office.firmware, &office.hardware)
        };
        question(&office, from, to);
        office.act();
        turns = office.runtime.requests().len();
        office.finish_all();
    }
    assert_eq!(turns, MESSAGE_TURNS_PER_HOUR);
    let held: Vec<String> = office
        .entries()
        .into_iter()
        .filter(|(kind, _)| *kind == CoordinatorEntryKind::Held)
        .map(|(_, text)| text)
        .collect();
    assert_eq!(
        held.len(),
        2,
        "told once an hour for each specialist: {held:?}"
    );
    // An hour later the questions that waited are answered: both
    // specialists have some.
    office.clock.advance(TimeDelta::hours(1));
    office.act();
    assert_eq!(office.runtime.requests().len(), MESSAGE_TURNS_PER_HOUR + 2);
}

#[test]
fn a_budget_set_for_a_member_of_another_project_is_refused() {
    let office = office();
    let result = office.coordinator.set_budget(
        office.project.id(),
        BudgetScope::Member(teamwright_domain::MemberId::generate()),
        Some(5_000),
    );
    assert!(matches!(result, Err(ApplicationError::NotFound { .. })));
    let result = office
        .coordinator
        .set_budget(office.project.id(), BudgetScope::Project, Some(5));
    assert!(matches!(result, Err(ApplicationError::Rule(_))));
    let _ = &office.team;
}

/// A permission the manager allowed on this computer, for a task of a
/// specialist (decision 0038).
fn allowed_for_task(
    office: &Office,
    member: &Member,
    task: &Task,
    kind: teamwright_domain::PermissionKind,
    what: &str,
) -> teamwright_domain::PermissionRule {
    use teamwright_domain::{
        Answered, ConversationId, PermissionRequest, PermissionRule, PermissionScope, TurnId,
    };
    let rule = PermissionRule::new(kind, what).unwrap();
    office.clock.advance(TimeDelta::seconds(1));
    let mut request = PermissionRequest::ask(
        office.project.id(),
        SaidIn {
            member_id: member.id(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        },
        Some(task.id()),
        rule.clone(),
        "The task needs it.",
        office.clock.now(),
    )
    .unwrap();
    request
        .allow(
            PermissionScope::Task,
            rule.clone(),
            "",
            Answered {
                at: office.clock.now(),
                computer: office.this_computer,
                next_turn: TurnId::generate(),
            },
        )
        .unwrap();
    crate::ports::PermissionRepository::save_request(office.permissions.as_ref(), &request)
        .unwrap();
    rule
}

#[test]
fn work_the_coordinator_starts_runs_no_command_allowed_for_its_task() {
    use crate::agent_runtime::PermissionProfile;
    use teamwright_domain::PermissionKind;

    let office = office();
    let schematic = office.task("Draw the schematic", &office.hardware);
    let check = allowed_for_task(
        &office,
        &office.hardware,
        &schematic,
        PermissionKind::Command,
        "make check",
    );
    let docs = allowed_for_task(
        &office,
        &office.hardware,
        &schematic,
        PermissionKind::WebDomain,
        "docs.rs",
    );
    office.switch_on();
    office.act();

    // Started by the coordinator: the web domain, and no command.
    let started = &office.runtime.requests()[0];
    assert_eq!(
        started.permissions,
        PermissionProfile::specialist().with_granted(std::slice::from_ref(&docs))
    );
    assert!(
        !started
            .extra_instructions
            .as_deref()
            .unwrap_or_default()
            .contains("make check")
    );

    // The manager's own reply in the same conversation has the command.
    office.end(0, None);
    let conversation = office.work.list_for_project(office.project.id()).unwrap()[0]
        .conversation
        .id();
    office.work.reply(conversation, "Run the checks.").unwrap();
    assert_eq!(
        office.runtime.requests()[1].permissions,
        PermissionProfile::specialist().with_granted(&[check, docs])
    );
}

#[test]
fn work_the_coordinator_goes_on_with_after_the_limit_runs_no_command() {
    use crate::agent_runtime::PermissionProfile;
    use teamwright_domain::PermissionKind;

    let office = office();
    let schematic = office.task("Draw the schematic", &office.hardware);
    let check = allowed_for_task(
        &office,
        &office.hardware,
        &schematic,
        PermissionKind::Command,
        "make check",
    );
    // The manager starts the work by hand: the command is there.
    office.work.start_work(schematic.id()).unwrap();
    assert_eq!(
        office.runtime.requests()[0].permissions,
        PermissionProfile::specialist().with_granted(std::slice::from_ref(&check))
    );

    let resets_at = chrono::Utc
        .with_ymd_and_hms(2026, 11, 21, 22, 40, 0)
        .unwrap();
    office.clock.advance(TimeDelta::minutes(15));
    office.end(0, at_the_limit(resets_at));
    office.switch_on();
    office.clock.advance(TimeDelta::hours(3));
    office.plan_limit.look_at_reset().unwrap();
    office.act();

    // The coordinator goes on with it: no command.
    let went_on = &office.runtime.requests()[1];
    assert!(went_on.prompt.starts_with(GO_ON_REQUEST));
    assert_eq!(went_on.permissions, PermissionProfile::specialist());
}
