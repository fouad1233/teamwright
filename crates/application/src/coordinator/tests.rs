//! The rule of the coordinator, case by case. Every case builds a small
//! project with the use cases in memory and asks [`next_steps`].

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{
    Assignee, Budget, BudgetScope, Color, ComputerId, ConcurrencyLimit, Conversation,
    DecisionOption, DecisionRequest, Member, MemberKind, MessageKind, Priority, Project, SaidIn,
    StatusCategory, Task, TaskId, TeamMessage, Timestamp, TokenBudget, TokenCounts, Turn,
    TurnOrigin,
};

use super::*;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
    InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, StatusProperties, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};

fn moment(minute: i64) -> Timestamp {
    chrono::Utc
        .with_ymd_and_hms(2026, 11, 21, 21, 0, 0)
        .unwrap()
        + TimeDelta::minutes(minute)
}

/// A project with an engineering workflow (to do, in progress, in
/// revision, in test, done), three AI specialists and a person, and what
/// the coordinator knows about it besides.
struct Board {
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
    project: Project,
    hardware: Member,
    firmware: Member,
    mechanical: Member,
    person: Member,
    conversations: Vec<ConversationWithTurns>,
    messages: Vec<TeamMessage>,
    budgets: Vec<Budget>,
    limit: ConcurrencyLimit,
    message_turns: usize,
    plan: WorkPermit,
    plan_limit_told_at: Option<Timestamp>,
}

fn board() -> Board {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members = Arc::new(InMemoryMemberRepository::default());
    let conversations = Arc::new(InMemoryConversationRepository::default());
    let areas = Arc::new(InMemoryAreaRepository::default());
    let views = Arc::new(InMemoryViewRepository::default());
    let clock = Arc::new(ManualClock::starting_at(moment(0)));
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
        areas,
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store,
        members,
        tasks_store,
        conversations,
        views,
        clock,
    );
    let project = projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    let add = |name: &str, kind: MemberKind| {
        team.add(NewMember {
            project_id: project.id(),
            name: name.to_owned(),
            kind,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap()
    };
    let hardware = add("Hardware engineer", MemberKind::Agent);
    let firmware = add("Firmware engineer", MemberKind::Agent);
    let mechanical = add("Mechanical engineer", MemberKind::Agent);
    let person = add("Ada", MemberKind::Human);
    Board {
        projects,
        tasks,
        team,
        project,
        hardware,
        firmware,
        mechanical,
        person,
        conversations: Vec::new(),
        messages: Vec::new(),
        budgets: Vec::new(),
        limit: ConcurrencyLimit::DEFAULT,
        message_turns: 0,
        plan: WorkPermit::Allowed,
        plan_limit_told_at: None,
    }
}

/// How a turn ends in a conversation the tests make.
#[derive(Clone, Copy)]
enum Ending {
    Running,
    Finished,
    Failed,
    Asks,
    /// Ended because the limit of the Claude plan was reached.
    PlanLimit,
}

fn counts(all: u64) -> TokenCounts {
    TokenCounts {
        input: 0,
        output: all,
        cache_read: 0,
        cache_written: 0,
    }
}

impl Board {
    fn project(&self) -> Project {
        self.projects.get(self.project.id()).unwrap()
    }

    fn task(&self, title: &str, member: Option<&Member>) -> Task {
        self.tasks
            .create(NewTask {
                assignee: member.map(|member| Assignee::Member(member.id())),
                ..NewTask::titled(self.project.id(), title)
            })
            .unwrap()
    }

    fn subtask(&self, title: &str, member: &Member, parent: &Task) -> Task {
        self.tasks
            .create(NewTask {
                assignee: Some(Assignee::Member(member.id())),
                parent_id: Some(parent.id()),
                ..NewTask::titled(self.project.id(), title)
            })
            .unwrap()
    }

    /// The `place`-th status of the workflow, counted from 0: to do, in
    /// progress, in revision, in test, done.
    fn status(&self, place: usize) -> teamwright_domain::StatusId {
        self.project().workflow().statuses()[place].id()
    }

    fn move_to(&self, task: &Task, category: StatusCategory) {
        let status = self.project().workflow().first_in(category).unwrap().id();
        self.tasks.move_to(task.id(), status, usize::MAX).unwrap();
    }

    fn wait(&self, task: &Task, blocker: &Task) {
        self.tasks.add_dependency(task.id(), blocker.id()).unwrap();
    }

    /// A conversation of `member` about `task`, whose one turn ended as
    /// `ending` and used `spent` tokens.
    fn worked(&mut self, member: &Member, task: Option<&Task>, ending: Ending, spent: u64) {
        let conversation = Conversation::start(
            self.project.id(),
            member.id(),
            task.map(Task::id),
            moment(1),
            ComputerId::generate(),
        );
        self.add_turn(conversation, ending, spent);
    }

    fn add_turn(&mut self, conversation: Conversation, ending: Ending, spent: u64) {
        let mut turn = Turn::start(
            conversation.id(),
            "Work on it",
            TurnOrigin::Brief,
            moment(1),
        )
        .unwrap();
        match ending {
            Ending::Running => {}
            Ending::Finished => turn.finish("Done.", counts(spent), moment(2)).unwrap(),
            Ending::Failed => turn
                .fail("It broke.".to_owned(), Some(counts(spent)), moment(2))
                .unwrap(),
            Ending::PlanLimit => turn
                .end_at_plan_limit(Some(counts(spent)), moment(2))
                .unwrap(),
            Ending::Asks => {
                turn.ask(
                    DecisionRequest::new(
                        "Which way?",
                        "It matters.",
                        vec![
                            DecisionOption {
                                label: "This".to_owned(),
                                consequence: String::new(),
                            },
                            DecisionOption {
                                label: "That".to_owned(),
                                consequence: String::new(),
                            },
                        ],
                        None,
                    )
                    .unwrap(),
                )
                .unwrap();
                turn.finish("I wait for you.", counts(spent), moment(2))
                    .unwrap();
            }
        }
        self.conversations.push(ConversationWithTurns {
            conversation,
            turns: vec![turn],
        });
    }

    /// The coordinator's turn to go on, at `minute`, in the conversation
    /// about `task`, ended at the limit of the plan.
    fn went_on_and_stopped_again(&mut self, task: &Task, minute: i64) {
        let conversation = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.conversation.task_id() == Some(task.id()))
            .unwrap();
        let mut turn = Turn::start(
            conversation.conversation.id(),
            GO_ON_REQUEST,
            TurnOrigin::Message,
            moment(minute),
        )
        .unwrap();
        turn.end_at_plan_limit(None, moment(minute + 1)).unwrap();
        conversation.turns.push(turn);
    }

    fn gone_on(&self) -> Vec<TaskId> {
        self.plan()
            .goes_on
            .iter()
            .map(|going| going.task_id)
            .collect()
    }

    /// A conversation with the Advisor that used `spent` tokens.
    fn advised(&mut self, spent: u64) {
        let conversation = Conversation::start_with_advisor(
            self.project.id(),
            None,
            None,
            moment(1),
            ComputerId::generate(),
        );
        self.add_turn(conversation, Ending::Finished, spent);
    }

    fn message(&mut self, from: &Member, to: &Member, kind: MessageKind) {
        let message = TeamMessage::send(
            self.project.id(),
            SaidIn {
                member_id: from.id(),
                conversation_id: teamwright_domain::ConversationId::generate(),
                turn_id: teamwright_domain::TurnId::generate(),
            },
            to.id(),
            kind,
            "Can the connector move 3 mm to the left?",
            moment(self.messages.len().try_into().unwrap()),
        )
        .unwrap();
        self.messages.push(message);
    }

    fn budget(&mut self, scope: BudgetScope, tokens: u64) {
        self.budgets.push(Budget::new(
            self.project.id(),
            scope,
            TokenBudget::new(tokens).unwrap(),
        ));
    }

    fn plan(&self) -> Plan {
        let project = self.project();
        let tasks = self.tasks.list_for_project(project.id()).unwrap();
        let members = self.team.list_for_project(project.id()).unwrap();
        next_steps(&Situation {
            project: &project,
            tasks: &tasks,
            members: &members,
            conversations: &self.conversations,
            messages: &self.messages,
            budgets: &self.budgets,
            limit: self.limit,
            message_turns_in_the_last_hour: self.message_turns,
            plan: self.plan,
            plan_limit_told_at: self.plan_limit_told_at,
        })
    }

    fn started(&self) -> Vec<TaskId> {
        self.plan()
            .starts
            .iter()
            .map(|start| start.task_id)
            .collect()
    }

    fn reason_for(&self, task: &Task) -> Option<WaitReason> {
        self.plan()
            .waiting
            .iter()
            .find(|waiting| waiting.task_id == Some(task.id()))
            .map(|waiting| waiting.reason)
    }
}

// What starts.

#[test]
fn a_task_that_waits_for_nothing_starts_with_its_specialist() {
    let board = board();
    let task = board.task("Write the Wi-Fi setup", Some(&board.firmware));
    let plan = board.plan();
    assert_eq!(
        plan.starts,
        vec![Start {
            task_id: task.id(),
            member_id: board.firmware.id(),
            after: Vec::new(),
        }]
    );
    assert!(plan.waiting.is_empty());
}

#[test]
fn a_task_given_to_a_person_the_manager_or_nobody_never_starts() {
    let board = board();
    board.task("Solder the board", Some(&board.person));
    board
        .tasks
        .create(NewTask {
            assignee: Some(Assignee::Manager),
            ..NewTask::titled(board.project.id(), "Order the parts")
        })
        .unwrap();
    board.task("Something nobody has", None);
    assert_eq!(board.plan(), Plan::default());
}

#[test]
fn a_task_starts_once_everything_it_waits_for_is_done() {
    let board = board();
    let pins = board.task("Define the pin map", Some(&board.hardware));
    let parts = board.task("Choose the parts", Some(&board.hardware));
    let drivers = board.task("Write the sensor drivers", Some(&board.firmware));
    board.wait(&drivers, &pins);
    board.wait(&drivers, &parts);
    board.move_to(&pins, StatusCategory::Done);
    board.move_to(&parts, StatusCategory::Active);
    assert!(!board.started().contains(&drivers.id()));
    assert_eq!(board.reason_for(&drivers), None);

    board.move_to(&parts, StatusCategory::Done);
    let plan = board.plan();
    let start = plan
        .starts
        .iter()
        .find(|start| start.task_id == drivers.id())
        .unwrap();
    assert_eq!(start.after, vec![pins.id(), parts.id()]);
}

#[test]
fn work_that_waits_to_be_checked_is_not_done_and_the_manager_is_told() {
    let board = board();
    let pins = board.task("Define the pin map", Some(&board.hardware));
    let drivers = board.task("Write the sensor drivers", Some(&board.firmware));
    board.wait(&drivers, &pins);
    board.move_to(&pins, StatusCategory::Review);
    assert!(board.started().is_empty());
    assert_eq!(
        board.reason_for(&drivers),
        Some(WaitReason::WaitsForCheck(pins.id()))
    );
}

#[test]
fn a_task_waits_for_its_parts() {
    let board = board();
    let firmware = board.task("The firmware", Some(&board.firmware));
    let part = board.subtask("The drivers", &board.mechanical, &firmware);
    assert_eq!(board.started(), vec![part.id()]);
    board.move_to(&part, StatusCategory::Done);
    assert_eq!(board.started(), vec![firmware.id()]);
}

#[test]
fn the_category_of_a_status_counts_never_its_name() {
    let board = board();
    let pins = board.task("Define the pin map", Some(&board.hardware));
    let drivers = board.task("Write the sensor drivers", Some(&board.firmware));
    board.wait(&drivers, &pins);
    // "In test" is renamed "Done", and stays a status for checking.
    board
        .projects
        .change_status(
            board.project.id(),
            board.status(3),
            StatusProperties {
                name: "Done!".to_owned(),
                category: StatusCategory::Review,
                color: Color::Amber,
            },
        )
        .unwrap();
    board.tasks.move_to(pins.id(), board.status(3), 0).unwrap();
    assert!(!board.started().contains(&drivers.id()));
    // "In revision" is renamed "Finished", and is still work under way: a
    // task in it that nobody works on starts.
    board
        .projects
        .change_status(
            board.project.id(),
            board.status(2),
            StatusProperties {
                name: "Finished".to_owned(),
                category: StatusCategory::Active,
                color: Color::Purple,
            },
        )
        .unwrap();
    let revision = board.task("Revise the layout", Some(&board.mechanical));
    board
        .tasks
        .move_to(revision.id(), board.status(2), 0)
        .unwrap();
    assert!(board.started().contains(&revision.id()));
}

#[test]
fn a_task_under_way_that_nobody_works_on_starts_but_one_finished_does_not() {
    let board = board();
    let under_way = board.task("Route the board", Some(&board.hardware));
    let checked = board.task("Select the sensors", Some(&board.firmware));
    let done = board.task("Agree on the device", Some(&board.mechanical));
    board.move_to(&under_way, StatusCategory::Active);
    board.move_to(&checked, StatusCategory::Review);
    board.move_to(&done, StatusCategory::Done);
    assert_eq!(board.started(), vec![under_way.id()]);
}

// What never starts again.

#[test]
fn a_task_that_was_worked_on_is_never_started_again() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::Finished, 100);
    assert!(board.started().is_empty());
    assert_eq!(board.reason_for(&task), None);
}

#[test]
fn a_failed_task_is_not_tried_again_and_waits_for_the_manager() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let other = board.task("Route the board", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::Failed, 100);
    let plan = board.plan();
    assert_eq!(
        plan.starts
            .iter()
            .map(|start| start.task_id)
            .collect::<Vec<_>>(),
        vec![other.id()],
        "the specialist is free for other work"
    );
    assert_eq!(board.reason_for(&task), Some(WaitReason::Failed));
}

#[test]
fn a_task_that_waits_for_the_managers_answer_is_not_started_again() {
    let mut board = board();
    let task = board.task("Choose the battery", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::Asks, 100);
    assert!(board.started().is_empty());
}

// The limit of the Claude plan.

#[test]
fn while_the_limit_of_the_plan_is_reached_nothing_starts() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let firmware = board.firmware.clone();
    let hardware = board.hardware.clone();
    board.message(&hardware, &firmware, MessageKind::Question);
    board.plan = WorkPermit::NotUntil(moment(100));
    let plan = board.plan();
    assert!(plan.starts.is_empty());
    assert!(plan.wakes.is_empty());
    assert_eq!(
        plan.waiting,
        vec![
            Waiting {
                task_id: Some(task.id()),
                member_id: board.hardware.id(),
                reason: WaitReason::PlanLimit,
            },
            Waiting {
                task_id: None,
                member_id: board.firmware.id(),
                reason: WaitReason::PlanLimit,
            },
        ]
    );
}

#[test]
fn while_the_limit_is_reached_and_its_reset_is_not_known_nothing_starts() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    board.plan = WorkPermit::NotUntilUnknown;
    assert!(board.started().is_empty());
    assert_eq!(board.reason_for(&task), Some(WaitReason::PlanLimit));
}

#[test]
fn while_the_limit_is_reached_everything_that_could_start_waits_for_it_and_nothing_else() {
    let mut board = board();
    let first = board.task("Draw the schematic", Some(&board.hardware));
    let second = board.task("Route the board", Some(&board.hardware));
    let third = board.task("Write the drivers", Some(&board.firmware));
    board.limit = ConcurrencyLimit::new(1).unwrap();
    board.plan = WorkPermit::NotUntilUnknown;
    for task in [&first, &second, &third] {
        assert_eq!(
            board.reason_for(task),
            Some(WaitReason::PlanLimit),
            "nobody is at work, so nothing waits for a busy specialist or the limit"
        );
    }
}

#[test]
fn work_the_limit_stopped_goes_on_before_new_work_starts() {
    let mut board = board();
    let new = board.task("Draw the schematic", Some(&board.hardware));
    let stopped = board.task("Route the board", Some(&board.hardware));
    board.move_to(&stopped, StatusCategory::Active);
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&stopped), Ending::PlanLimit, 1_000);
    assert_eq!(board.gone_on(), vec![stopped.id()]);
    assert!(board.started().is_empty());
    assert_eq!(board.reason_for(&new), Some(WaitReason::SpecialistBusy));
}

#[test]
fn once_the_limit_has_lifted_work_starts_again() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    board.plan = WorkPermit::NotUntil(moment(100));
    assert!(board.started().is_empty());
    board.plan = WorkPermit::Allowed;
    assert_eq!(board.started(), vec![task.id()]);
}

#[test]
fn a_task_the_limit_stopped_did_not_fail_and_goes_on_once_the_limit_lifts() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::PlanLimit, 100);

    board.plan = WorkPermit::NotUntil(moment(100));
    assert!(board.gone_on().is_empty());
    assert_eq!(board.reason_for(&task), Some(WaitReason::PlanLimit));

    board.plan = WorkPermit::Allowed;
    let plan = board.plan();
    assert!(plan.starts.is_empty(), "it is not started anew");
    assert_eq!(
        plan.goes_on,
        vec![GoOn {
            task_id: task.id(),
            member_id: board.hardware.id(),
            conversation_id: board.conversations[0].conversation.id(),
        }]
    );
    assert_eq!(board.reason_for(&task), None);
}

#[test]
fn going_on_takes_a_place_and_keeps_to_the_budgets() {
    let mut board = board();
    let stopped = board.task("Draw the schematic", Some(&board.hardware));
    let other = board.task("Route the board", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&stopped), Ending::PlanLimit, 1_000);
    assert_eq!(board.gone_on(), vec![stopped.id()]);
    assert_eq!(board.reason_for(&other), Some(WaitReason::SpecialistBusy));

    board.budget(BudgetScope::Task(stopped.id()), 1_000);
    assert!(board.gone_on().is_empty());
    assert_eq!(
        board.reason_for(&stopped),
        Some(WaitReason::Budget(BudgetScope::Task(stopped.id())))
    );
}

#[test]
fn a_task_the_limit_stopped_again_as_it_went_on_does_not_loop() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::PlanLimit, 100);
    // The coordinator went on at minute 10, and the limit ended that turn
    // too, but the limit is not known to be reached: it is not tried again.
    board.went_on_and_stopped_again(&task, 10);
    board.plan_limit_told_at = Some(moment(5));
    assert!(board.gone_on().is_empty());
    assert_eq!(board.reason_for(&task), Some(WaitReason::PlanLimitAgain));

    // While the limit is reached it waits for the limit, as everything does.
    board.plan = WorkPermit::NotUntil(moment(100));
    assert_eq!(board.reason_for(&task), Some(WaitReason::PlanLimit));

    // Once the coordinator has said that the limit held work back since,
    // the next reset lets it go on once more.
    board.plan_limit_told_at = Some(moment(12));
    board.plan = WorkPermit::Allowed;
    assert_eq!(board.gone_on(), vec![task.id()]);
}

#[test]
fn an_ordinary_failed_task_still_waits_for_the_manager_after_the_limit_lifts() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&task), Ending::Failed, 100);
    board.plan_limit_told_at = Some(moment(5));
    assert!(board.gone_on().is_empty());
    assert_eq!(board.reason_for(&task), Some(WaitReason::Failed));
}

// Who is free.

#[test]
fn a_specialist_is_given_one_task_at_a_time() {
    let board = board();
    let first = board.task("Draw the schematic", Some(&board.hardware));
    let second = board.task("Route the board", Some(&board.hardware));
    assert_eq!(board.started(), vec![first.id()]);
    assert_eq!(board.reason_for(&second), Some(WaitReason::SpecialistBusy));
}

#[test]
fn a_specialist_at_work_is_given_nothing_new() {
    let mut board = board();
    let question = board.task("A question", None);
    let next = board.task("Route the board", Some(&board.hardware));
    let hardware = board.hardware.clone();
    // A turn the manager started, about no task, on another computer.
    board.worked(&hardware, None, Ending::Running, 0);
    let _ = question;
    assert!(board.started().is_empty());
    assert_eq!(board.reason_for(&next), Some(WaitReason::SpecialistBusy));
}

#[test]
fn a_specialist_that_waits_for_the_managers_answer_is_given_nothing_new() {
    let mut board = board();
    let asked = board.task("Choose the battery", Some(&board.hardware));
    let next = board.task("Route the board", Some(&board.hardware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&asked), Ending::Asks, 100);
    assert!(board.started().is_empty());
    assert_eq!(
        board.reason_for(&next),
        Some(WaitReason::SpecialistWaitsForManager)
    );
}

// The limit.

#[test]
fn no_more_specialists_work_at_once_than_the_limit() {
    let mut board = board();
    let first = board.task("Draw the schematic", Some(&board.hardware));
    let second = board.task("Write the drivers", Some(&board.firmware));
    let third = board.task("Design the enclosure", Some(&board.mechanical));
    assert_eq!(board.started(), vec![first.id(), second.id()]);
    assert_eq!(board.reason_for(&third), Some(WaitReason::Limit));

    board.limit = ConcurrencyLimit::new(1).unwrap();
    assert_eq!(board.started(), vec![first.id()]);
    board.limit = ConcurrencyLimit::new(3).unwrap();
    assert_eq!(board.started().len(), 3);
}

#[test]
fn turns_the_manager_started_count_against_the_limit() {
    let mut board = board();
    board.task("Draw the schematic", Some(&board.hardware));
    let drivers = board.task("Write the drivers", Some(&board.firmware));
    let enclosure = board.task("Design the enclosure", Some(&board.mechanical));
    let hardware = board.hardware.clone();
    board.worked(&hardware, None, Ending::Running, 0);
    assert_eq!(board.started(), vec![drivers.id()]);
    assert_eq!(board.reason_for(&enclosure), Some(WaitReason::Limit));
}

#[test]
fn the_advisor_at_work_is_not_a_specialist_at_work() {
    let mut board = board();
    board.limit = ConcurrencyLimit::new(1).unwrap();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    let conversation = Conversation::start_with_advisor(
        board.project.id(),
        None,
        None,
        moment(1),
        ComputerId::generate(),
    );
    board.add_turn(conversation, Ending::Running, 0);
    assert_eq!(board.started(), vec![task.id()]);
}

// The order.

#[test]
fn the_most_urgent_task_starts_first() {
    let mut board = board();
    board.limit = ConcurrencyLimit::new(1).unwrap();
    let normal = board.task("Write the drivers", Some(&board.firmware));
    let urgent = board.task("Fix the power", Some(&board.hardware));
    board
        .tasks
        .prioritize(urgent.id(), Priority::Urgent)
        .unwrap();
    assert_eq!(board.started(), vec![urgent.id()]);
    assert_eq!(board.reason_for(&normal), Some(WaitReason::Limit));
}

#[test]
fn of_equal_priority_the_order_of_the_board_counts() {
    let mut board = board();
    board.limit = ConcurrencyLimit::new(1).unwrap();
    let first = board.task("Write the drivers", Some(&board.firmware));
    let second = board.task("Fix the power", Some(&board.hardware));
    assert_eq!(board.started(), vec![first.id()]);
    // Moved to the top of its column, the second goes first.
    board
        .tasks
        .move_to(second.id(), board.status(0), 0)
        .unwrap();
    assert_eq!(board.started(), vec![second.id()]);
    // A status further along the workflow comes after the earlier ones.
    board.move_to(&second, StatusCategory::Active);
    assert_eq!(board.started(), vec![first.id()]);
}

// Budgets.

#[test]
fn a_used_up_budget_of_the_project_stops_all_new_work_and_asks_once() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    board.advised(5_000);
    board.budget(BudgetScope::Project, 5_000);
    let plan = board.plan();
    assert!(plan.starts.is_empty());
    assert_eq!(
        board.reason_for(&task),
        Some(WaitReason::Budget(BudgetScope::Project))
    );
    assert_eq!(plan.questions, vec![BudgetScope::Project]);

    board.budgets[0].ask(5_000, moment(3)).unwrap();
    let plan = board.plan();
    assert!(plan.starts.is_empty());
    assert!(plan.questions.is_empty(), "asked once");
}

#[test]
fn a_budget_that_is_not_used_up_holds_nothing() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    board.advised(4_999);
    board.budget(BudgetScope::Project, 5_000);
    assert_eq!(board.started(), vec![task.id()]);
    assert!(board.plan().questions.is_empty());
}

#[test]
fn a_used_up_budget_of_a_member_holds_only_its_work() {
    let mut board = board();
    let earlier = board.task("Choose the parts", Some(&board.hardware));
    let schematic = board.task("Draw the schematic", Some(&board.hardware));
    let drivers = board.task("Write the drivers", Some(&board.firmware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&earlier), Ending::Finished, 2_000);
    board.budget(BudgetScope::Member(hardware.id()), 2_000);
    assert_eq!(board.started(), vec![drivers.id()]);
    assert_eq!(
        board.reason_for(&schematic),
        Some(WaitReason::Budget(BudgetScope::Member(hardware.id())))
    );
    assert_eq!(
        board.plan().questions,
        vec![BudgetScope::Member(hardware.id())]
    );
}

#[test]
fn a_used_up_budget_of_a_task_is_asked_about_and_holds_nothing_else() {
    // Something is spent on a task only in a conversation about it, and a
    // task with a conversation is never started again. So the budget of a
    // task holds no start; it is asked about once it is used up, and the
    // rest of the work goes on.
    let mut board = board();
    let schematic = board.task("Draw the schematic", Some(&board.hardware));
    let drivers = board.task("Write the drivers", Some(&board.firmware));
    let hardware = board.hardware.clone();
    board.worked(&hardware, Some(&schematic), Ending::Finished, 1_000);
    board.budget(BudgetScope::Task(schematic.id()), 1_000);
    let plan = board.plan();
    assert_eq!(board.started(), vec![drivers.id()]);
    assert_eq!(plan.questions, vec![BudgetScope::Task(schematic.id())]);
}

#[test]
fn a_used_up_budget_is_asked_about_even_when_nothing_waits() {
    let mut board = board();
    board.advised(6_000);
    board.budget(BudgetScope::Project, 5_000);
    assert_eq!(board.plan().questions, vec![BudgetScope::Project]);
}

#[test]
fn a_budget_without_tokens_holds_nothing() {
    let mut board = board();
    let task = board.task("Draw the schematic", Some(&board.hardware));
    board.advised(9_000);
    board.budget(BudgetScope::Project, 5_000);
    board.budgets[0].set(None);
    assert_eq!(board.started(), vec![task.id()]);
}

#[test]
fn the_tokens_of_a_running_turn_count_once_it_has_ended() {
    let mut board = board();
    let hardware = board.hardware.clone();
    board.worked(&hardware, None, Ending::Running, 0);
    board.worked(&hardware, None, Ending::Finished, 300);
    board.advised(700);
    let spending = spending_of(&board.conversations);
    assert_eq!(spending.project, 1_000);
    assert_eq!(spending.of(BudgetScope::Member(hardware.id())), 300);
}

// Messages between teammates.

#[test]
fn a_free_specialist_that_was_asked_a_question_gets_a_turn_to_answer() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    board.message(&hardware, &firmware, MessageKind::Question);
    assert_eq!(
        board.plan().wakes,
        vec![Wake {
            member_id: firmware.id(),
            asked_by: vec![hardware.id()],
        }]
    );
}

#[test]
fn a_note_waits_for_the_next_turn_of_its_receiver() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    board.message(&hardware, &firmware, MessageKind::Note);
    assert!(board.plan().wakes.is_empty());
}

#[test]
fn a_specialist_that_starts_a_task_gets_its_messages_with_it() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    let task = board.task("Write the drivers", Some(&firmware));
    board.message(&hardware, &firmware, MessageKind::Question);
    let plan = board.plan();
    assert_eq!(plan.starts.len(), 1);
    assert_eq!(plan.starts[0].task_id, task.id());
    assert!(plan.wakes.is_empty());
}

#[test]
fn a_busy_specialist_is_not_woken_and_tasks_come_before_messages() {
    let mut board = board();
    board.limit = ConcurrencyLimit::new(1).unwrap();
    let (hardware, firmware, mechanical) = (
        board.hardware.clone(),
        board.firmware.clone(),
        board.mechanical.clone(),
    );
    board.message(&hardware, &firmware, MessageKind::Question);
    let task = board.task("Design the enclosure", Some(&mechanical));
    let plan = board.plan();
    assert_eq!(plan.starts[0].task_id, task.id());
    assert!(plan.wakes.is_empty());
    assert!(plan.waiting.iter().any(|waiting| waiting.task_id.is_none()
        && waiting.member_id == firmware.id()
        && waiting.reason == WaitReason::Limit));

    board.limit = ConcurrencyLimit::DEFAULT;
    board.worked(&firmware, None, Ending::Running, 0);
    assert!(board.plan().wakes.is_empty());
}

#[test]
fn turns_for_messages_stop_at_the_limit_of_an_hour() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    board.message(&hardware, &firmware, MessageKind::Question);
    board.message_turns = MESSAGE_TURNS_PER_HOUR;
    let plan = board.plan();
    assert!(plan.wakes.is_empty());
    assert_eq!(
        plan.waiting,
        vec![Waiting {
            task_id: None,
            member_id: firmware.id(),
            reason: WaitReason::MessageLimit,
        }]
    );
    board.message_turns = MESSAGE_TURNS_PER_HOUR - 1;
    assert_eq!(board.plan().wakes.len(), 1);
}

#[test]
fn two_specialists_asking_each_other_are_both_counted_against_the_hour() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    board.message(&hardware, &firmware, MessageKind::Question);
    board.message(&firmware, &hardware, MessageKind::Question);
    board.message_turns = MESSAGE_TURNS_PER_HOUR - 1;
    let plan = board.plan();
    assert_eq!(plan.wakes.len(), 1, "the last turn of the hour goes to one");
    assert_eq!(plan.wakes[0].member_id, firmware.id(), "asked first");
}

#[test]
fn a_used_up_budget_holds_a_turn_for_messages_too() {
    let mut board = board();
    let (hardware, firmware) = (board.hardware.clone(), board.firmware.clone());
    board.message(&hardware, &firmware, MessageKind::Question);
    board.worked(&firmware, None, Ending::Finished, 1_000);
    board.budget(BudgetScope::Member(firmware.id()), 1_000);
    let plan = board.plan();
    assert!(plan.wakes.is_empty());
    assert_eq!(plan.questions, vec![BudgetScope::Member(firmware.id())]);
}

#[test]
fn a_message_to_a_person_or_to_a_specialist_who_left_wakes_nobody() {
    let mut board = board();
    let (hardware, firmware, person) = (
        board.hardware.clone(),
        board.firmware.clone(),
        board.person.clone(),
    );
    board.message(&hardware, &person, MessageKind::Question);
    board.message(&hardware, &firmware, MessageKind::Question);
    board.team.remove(firmware.id()).unwrap();
    assert!(board.plan().wakes.is_empty());
}

// Words.

#[test]
fn the_record_says_what_started_and_why() {
    let board = board();
    let project = board.project();
    let pins = board.task("Define the pin map", Some(&board.hardware));
    let parts = board.task("Choose the parts", Some(&board.hardware));
    let drivers = board.task("Write the sensor drivers", Some(&board.firmware));
    let tasks = board.tasks.list_for_project(project.id()).unwrap();
    let members = board.team.list_for_project(project.id()).unwrap();
    assert_eq!(
        words::started(
            &project,
            &tasks,
            &members,
            drivers.id(),
            board.firmware.id(),
            &[pins.id(), parts.id()]
        ),
        "Firmware engineer started on SSN-3: SSN-1 and SSN-2 are done."
    );
    assert_eq!(
        words::started(
            &project,
            &tasks,
            &members,
            pins.id(),
            board.hardware.id(),
            &[]
        ),
        "Hardware engineer started on SSN-1: it waits for nothing."
    );
    assert_eq!(
        words::held_by_budget(
            "start SSN-3",
            "the budget of the project",
            1_040_000,
            1_000_000
        ),
        "Did not start SSN-3: the budget of the project is used up (1,040,000 of 1,000,000 tokens). You are asked in the inbox."
    );
    assert_eq!(
        words::woke(
            &members,
            board.firmware.id(),
            &[board.hardware.id(), board.mechanical.id()]
        ),
        "Firmware engineer was given a turn to answer Hardware engineer and Mechanical engineer."
    );
    assert_eq!(words::tokens(999), "999");
    assert_eq!(words::tokens(1_000), "1,000");
    assert_eq!(
        words::joined(&["A".into(), "B".into(), "C".into()]),
        "A, B and C"
    );
}
