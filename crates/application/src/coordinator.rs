//! The coordinator: the code that decides what runs next in a project
//! ([decision 0037](../../../docs/decisions/0037-the-coordinator.md)).
//!
//! It never asks a model. From the state of a project (the tasks with the
//! categories of their statuses and what they wait for, who they are given
//! to, which specialists are at work or wait for the manager, what has been
//! spent, the budgets and the limit of specialists at once) the pure
//! function [`next_steps`] answers what should happen: which tasks to
//! start with which specialist, which specialist to give a turn so that it
//! answers its teammates, what the manager is to be asked about a budget,
//! and what waits and why.
//!
//! [`CoordinatorService`] does what [`next_steps`] says when something
//! happens: the manager switched the coordinator on, a turn ended, a task
//! changed, the manager answered, a budget or the limit changed, the limit
//! of the Claude plan lifted, the program started. Nothing polls.
//!
//! While the limit of the Claude plan is reached it starts nothing, and a
//! task whose turn the limit ended is not a failed task: once the limit
//! has lifted, its specialist goes on with it
//! ([decision 0040](../../../docs/decisions/0040-the-limit-of-the-plan.md)).

use std::collections::{BTreeMap, BTreeSet};

use teamwright_domain::{
    Assignee, Budget, BudgetScope, ConcurrencyLimit, Member, MemberId, MemberKind, MessageKind,
    Project, StatusCategory, Task, TaskId, TeamMessage, Timestamp, Turn, TurnState,
};

use crate::plan_limit::WorkPermit;
use crate::work::{ConversationWithTurns, reference_of};

mod service;

pub use service::{
    BudgetQuestionOfProject, CoordinatorPorts, CoordinatorService, CoordinatorState, WAKE_REQUEST,
};

/// How many turns the coordinator gives specialists in an hour of a
/// project so that they answer what teammates asked them. Two specialists
/// that keep asking each other questions would otherwise talk for ever;
/// four answers an hour lets a short exchange through and stops a loop
/// within the hour. Each such turn is counted against the budgets like
/// any other.
pub const MESSAGE_TURNS_PER_HOUR: usize = 4;

/// What a specialist is told in the turn the coordinator gives it to go on
/// with a task whose last turn the limit of the Claude plan ended. It
/// differs from what the manager's "Try again" says, so that the rule can
/// tell the coordinator's own turns from the manager's.
pub const GO_ON_REQUEST: &str = "The limit of the Claude plan stopped your work, and it has reset. \
Please go on where you stopped.";

/// The state of a project, as far as the coordinator needs it.
#[derive(Debug, Clone, Copy)]
pub struct Situation<'state> {
    /// The project, with its workflow.
    pub project: &'state Project,
    /// Its tasks.
    pub tasks: &'state [Task],
    /// Its team.
    pub members: &'state [Member],
    /// Its conversations with their turns, on every computer.
    pub conversations: &'state [ConversationWithTurns],
    /// The messages between its members.
    pub messages: &'state [TeamMessage],
    /// Its budgets.
    pub budgets: &'state [Budget],
    /// How many specialists may work at once.
    pub limit: ConcurrencyLimit,
    /// How many turns the coordinator gave specialists for messages in the
    /// last hour.
    pub message_turns_in_the_last_hour: usize,
    /// Whether the limit of the Claude plan lets work start now.
    pub plan: WorkPermit,
    /// When the coordinator last wrote that the limit of the plan held work
    /// back, if it ever did.
    pub plan_limit_told_at: Option<Timestamp>,
}

/// What has been spent in a project, in tokens of every kind, by the turns
/// that have ended. A running turn counts once it has ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spending {
    /// The whole project, the Advisor included.
    pub project: u64,
    /// Each member that spent something.
    pub members: BTreeMap<MemberId, u64>,
    /// Each task that something was spent on.
    pub tasks: BTreeMap<TaskId, u64>,
}

impl Spending {
    /// What was spent against a budget of this scope.
    #[must_use]
    pub fn of(&self, scope: BudgetScope) -> u64 {
        match scope {
            BudgetScope::Project => self.project,
            BudgetScope::Member(member_id) => {
                self.members.get(&member_id).copied().unwrap_or_default()
            }
            BudgetScope::Task(task_id) => self.tasks.get(&task_id).copied().unwrap_or_default(),
        }
    }
}

/// What a project spent, from its conversations. It counts what the usage
/// of a project shows "in all" (`TokenCounts::all`): read, written, read
/// again from the cache and written to the cache.
#[must_use]
pub fn spending_of(conversations: &[ConversationWithTurns]) -> Spending {
    let mut spending = Spending::default();
    for conversation in conversations {
        let spent: u64 = conversation
            .turns
            .iter()
            .filter(|turn| !turn.is_running())
            .filter_map(teamwright_domain::Turn::tokens)
            .map(|tokens| tokens.all())
            .fold(0, u64::saturating_add);
        spending.project = spending.project.saturating_add(spent);
        if conversation.conversation.is_with_advisor() {
            continue;
        }
        if let Some(member_id) = conversation.conversation.member_id() {
            let member = spending.members.entry(member_id).or_default();
            *member = member.saturating_add(spent);
        }
        if let Some(task_id) = conversation.conversation.task_id() {
            let task = spending.tasks.entry(task_id).or_default();
            *task = task.saturating_add(spent);
        }
    }
    spending
}

/// What the coordinator should do now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// The tasks to start, each with its specialist, in this order.
    pub starts: Vec<Start>,
    /// The tasks whose last turn the limit of the plan ended, to go on
    /// with, each in its conversation.
    pub goes_on: Vec<GoOn>,
    /// The specialists to give a turn, to answer what teammates asked.
    pub wakes: Vec<Wake>,
    /// The budgets the manager is to be asked about now: they are used up,
    /// and nothing was asked about them as they stand.
    pub questions: Vec<BudgetScope>,
    /// What could start but waits, and why, in the order it would start.
    pub waiting: Vec<Waiting>,
}

/// A task to start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Start {
    /// The task.
    pub task_id: TaskId,
    /// The AI specialist it is given to.
    pub member_id: MemberId,
    /// The tasks it waited for, all of them done now, by their numbers.
    pub after: Vec<TaskId>,
}

/// A task to go on with after the limit of the plan ended its last turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoOn {
    /// The task.
    pub task_id: TaskId,
    /// The AI specialist it is given to.
    pub member_id: MemberId,
    /// The conversation whose last turn the limit ended.
    pub conversation_id: teamwright_domain::ConversationId,
}

/// A specialist to give a turn so that it answers its teammates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wake {
    /// The specialist.
    pub member_id: MemberId,
    /// Who asked it something, the first to ask first.
    pub asked_by: Vec<MemberId>,
}

/// Something that could start but waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Waiting {
    /// The task, or nothing for a turn to answer teammates.
    pub task_id: Option<TaskId>,
    /// The specialist it is for.
    pub member_id: MemberId,
    /// Why it waits.
    pub reason: WaitReason,
}

/// Why something waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitReason {
    /// A budget it counts against is used up.
    Budget(BudgetScope),
    /// As many specialists as the limit allows are at work.
    Limit,
    /// The specialist is at work on something else.
    SpecialistBusy,
    /// The specialist waits for the manager's answer to a question. It is
    /// given nothing new, so that the answer can reach it at once.
    SpecialistWaitsForManager,
    /// A task it waits for is finished and waits for the manager to check
    /// it: it counts as done only once it is in a status of the category
    /// done.
    WaitsForCheck(TaskId),
    /// The last turn of the task failed. It is not tried again by itself:
    /// the manager reads why and goes on with a reply.
    Failed,
    /// The limit of the Claude plan is reached: nothing starts until it
    /// lifts.
    PlanLimit,
    /// The coordinator went on with the task after the limit of the plan
    /// lifted, and that turn ended at the limit too, while the limit is
    /// not known to be reached again. It is not tried a third time by
    /// itself, so that a limit that is misread cannot make a loop; the
    /// manager presses "Try again".
    PlanLimitAgain,
    /// The specialists had as many turns for messages in the last hour as
    /// [`MESSAGE_TURNS_PER_HOUR`]; the message waits for the next turn of
    /// its receiver.
    MessageLimit,
}

/// What the coordinator should do now in a project. A pure function: it
/// reads nothing and changes nothing.
///
/// - A task starts when it is in a status of the category to do or under
///   way, it is given to an AI specialist of the team, nobody has worked on
///   it yet (it has no conversation, on any computer), every task it waits
///   for and every part of it is in a status of the category done, its
///   specialist is free, no budget it counts against is used up, and fewer
///   specialists than the limit are at work.
/// - A task given to a person, or to nobody, is never started.
/// - A specialist is free when no turn of it runs, on any computer, and it
///   does not wait for the manager's answer to a question.
/// - A specialist is given one thing at a time, and at most `limit`
///   specialists are at work at once, counting the turns the manager
///   started.
/// - A task that has a conversation is never started again: its work is
///   under way, finished, waits for the manager, or failed. A failed task
///   waits for the manager.
/// - A task whose last turn ended at the limit of the Claude plan did not
///   fail: its specialist goes on with it in the same conversation, as the
///   manager's "Try again" does. If that turn of the coordinator's ends at
///   the limit too, it goes on again only once the coordinator has seen
///   the limit hold work back since that turn started, that is after the
///   next reset; otherwise the task waits for the manager.
/// - While the limit of the plan is reached, nothing starts, goes on or is
///   woken: whatever could, waits for the limit, and takes no place.
/// - Work the limit stopped goes on before new work starts.
/// - The tasks are taken by priority, the most urgent first, then in the
///   order of the board: the order of the statuses, then the order within
///   a status, then the number.
/// - When the tasks are served, a free specialist to whom a teammate asked
///   a question that waits is given a turn to answer it, at most
///   [`MESSAGE_TURNS_PER_HOUR`] an hour. A note waits for the next turn.
/// - A budget that is used up is to be asked about, once.
///
/// Logic follows the category of a status, never its name.
#[must_use]
pub fn next_steps(situation: &Situation<'_>) -> Plan {
    let project = situation.project;
    let workflow = project.workflow();
    let spending = spending_of(situation.conversations);
    let category_of = |task: &Task| {
        workflow
            .status(task.status_id())
            .map(teamwright_domain::Status::category)
    };
    let tasks_by_id: BTreeMap<TaskId, &Task> = situation
        .tasks
        .iter()
        .map(|task| (task.id(), task))
        .collect();
    let specialists: BTreeMap<MemberId, &Member> = situation
        .members
        .iter()
        .filter(|member| member.kind() == MemberKind::Agent)
        .map(|member| (member.id(), member))
        .collect();

    let mut at_work: BTreeSet<MemberId> = BTreeSet::new();
    let mut waiting_for_manager: BTreeSet<MemberId> = BTreeSet::new();
    let mut worked_on: BTreeSet<TaskId> = BTreeSet::new();
    let mut failed: BTreeSet<TaskId> = BTreeSet::new();
    // The tasks whose latest turn ended at the limit of the plan, with that
    // conversation and turn. Such a turn did not fail.
    let mut stopped_by_limit: BTreeMap<TaskId, (&ConversationWithTurns, &Turn)> = BTreeMap::new();
    for conversation in situation.conversations {
        let turns = &conversation.turns;
        if let Some(task_id) = conversation.conversation.task_id() {
            worked_on.insert(task_id);
            match turns.last() {
                Some(turn) if turn.reached_plan_limit() => {
                    let later = stopped_by_limit
                        .get(&task_id)
                        .is_none_or(|(_, kept)| kept.started_at() < turn.started_at());
                    if later {
                        stopped_by_limit.insert(task_id, (conversation, turn));
                    }
                }
                Some(turn) if turn.state() == TurnState::Failed => {
                    failed.insert(task_id);
                }
                _ => {}
            }
        }
        let Some(member_id) = conversation.conversation.member_id() else {
            continue;
        };
        if conversation.conversation.is_with_advisor() {
            continue;
        }
        if turns.iter().any(teamwright_domain::Turn::is_running) {
            at_work.insert(member_id);
        }
        let waits = turns
            .last()
            .filter(|turn| !turn.is_running())
            .and_then(|turn| turn.decision())
            .is_some_and(teamwright_domain::DecisionRequest::is_open);
        if waits {
            waiting_for_manager.insert(member_id);
        }
    }

    let used_up = |scope: BudgetScope| {
        situation
            .budgets
            .iter()
            .any(|budget| budget.scope() == scope && budget.is_used_up_by(spending.of(scope)))
    };
    let mut free_places = usize::from(situation.limit.value()).saturating_sub(at_work.len());
    let mut given: BTreeSet<MemberId> = BTreeSet::new();
    let mut plan = Plan::default();
    // While the limit of the plan is reached, whatever could start now waits
    // for it instead, and takes no place.
    let held_by_plan = situation.plan != WorkPermit::Allowed;

    // Why a specialist cannot be given something now, whatever it is.
    let reason_for = |member_id: MemberId,
                      scopes: &[BudgetScope],
                      free_places: usize,
                      given: &BTreeSet<MemberId>|
     -> Option<WaitReason> {
        if let Some(scope) = scopes.iter().copied().find(|scope| used_up(*scope)) {
            return Some(WaitReason::Budget(scope));
        }
        if at_work.contains(&member_id) || given.contains(&member_id) {
            return Some(WaitReason::SpecialistBusy);
        }
        if waiting_for_manager.contains(&member_id) {
            return Some(WaitReason::SpecialistWaitsForManager);
        }
        (free_places == 0).then_some(WaitReason::Limit)
    };

    let mut candidates: Vec<(&Task, MemberId)> = situation
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                category_of(task),
                Some(StatusCategory::Todo | StatusCategory::Active)
            )
        })
        .filter_map(|task| match task.assignee() {
            Some(Assignee::Member(member_id)) if specialists.contains_key(&member_id) => {
                Some((task, member_id))
            }
            _ => None,
        })
        .collect();
    let place_of_status = |task: &Task| {
        workflow
            .statuses()
            .iter()
            .position(|status| status.id() == task.status_id())
            .unwrap_or(usize::MAX)
    };
    // Work the limit of the plan stopped goes on before new work starts:
    // it is under way, and its specialist knows it.
    let stopped = |task: &Task| stopped_by_limit.contains_key(&task.id());
    candidates.sort_by(|(first, _), (second, _)| {
        stopped(second)
            .cmp(&stopped(first))
            .then(second.priority().cmp(&first.priority()))
            .then(place_of_status(first).cmp(&place_of_status(second)))
            .then(first.rank().cmp(&second.rank()))
            .then(first.number().cmp(&second.number()))
    });

    let scopes_of = |task: &Task, member_id: MemberId| {
        [
            BudgetScope::Project,
            BudgetScope::Member(member_id),
            BudgetScope::Task(task.id()),
        ]
    };
    for (task, member_id) in candidates {
        if worked_on.contains(&task.id()) {
            if failed.contains(&task.id()) {
                plan.waiting.push(Waiting {
                    task_id: Some(task.id()),
                    member_id,
                    reason: WaitReason::Failed,
                });
            } else if let Some((conversation, turn)) =
                stopped_by_limit
                    .get(&task.id())
                    .filter(|(conversation, _)| {
                        conversation.conversation.member_id() == Some(member_id)
                    })
            {
                // The coordinator's own turn to go on ended at the limit
                // too: it goes on again only once the limit was seen to
                // hold work back since, so that a misread limit makes no
                // loop.
                let again = turn.request() == GO_ON_REQUEST
                    && situation.plan == WorkPermit::Allowed
                    && situation
                        .plan_limit_told_at
                        .is_none_or(|told| told < turn.started_at());
                let reason = if again {
                    Some(WaitReason::PlanLimitAgain)
                } else {
                    reason_for(member_id, &scopes_of(task, member_id), free_places, &given)
                };
                if let Some(reason) = reason {
                    plan.waiting.push(Waiting {
                        task_id: Some(task.id()),
                        member_id,
                        reason,
                    });
                    continue;
                }
                if held_by_plan {
                    plan.waiting.push(Waiting {
                        task_id: Some(task.id()),
                        member_id,
                        reason: WaitReason::PlanLimit,
                    });
                    continue;
                }
                free_places -= 1;
                given.insert(member_id);
                plan.goes_on.push(GoOn {
                    task_id: task.id(),
                    member_id,
                    conversation_id: conversation.conversation.id(),
                });
            }
            continue;
        }
        // What it waits for: the tasks it is blocked by and its parts.
        let mut waits_for: Vec<&Task> = task
            .blocked_by()
            .iter()
            .filter_map(|id| tasks_by_id.get(id).copied())
            .chain(
                situation
                    .tasks
                    .iter()
                    .filter(|other| other.parent_id() == Some(task.id())),
            )
            .collect();
        waits_for.sort_by_key(|other| other.number());
        let not_done: Vec<&&Task> = waits_for
            .iter()
            .filter(|other| category_of(other) != Some(StatusCategory::Done))
            .collect();
        if !not_done.is_empty() {
            // Only a wait that the manager can end is told.
            if let Some(checked) = not_done
                .iter()
                .find(|other| category_of(other) == Some(StatusCategory::Review))
                .filter(|_| {
                    not_done
                        .iter()
                        .all(|other| category_of(other) == Some(StatusCategory::Review))
                })
            {
                plan.waiting.push(Waiting {
                    task_id: Some(task.id()),
                    member_id,
                    reason: WaitReason::WaitsForCheck(checked.id()),
                });
            }
            continue;
        }
        if let Some(reason) =
            reason_for(member_id, &scopes_of(task, member_id), free_places, &given)
        {
            plan.waiting.push(Waiting {
                task_id: Some(task.id()),
                member_id,
                reason,
            });
            continue;
        }
        if held_by_plan {
            plan.waiting.push(Waiting {
                task_id: Some(task.id()),
                member_id,
                reason: WaitReason::PlanLimit,
            });
            continue;
        }
        free_places -= 1;
        given.insert(member_id);
        plan.starts.push(Start {
            task_id: task.id(),
            member_id,
            after: waits_for.iter().map(|other| other.id()).collect(),
        });
    }

    // The specialists to whom teammates asked a question, the one asked
    // first first.
    let mut asked: BTreeMap<MemberId, Vec<&TeamMessage>> = BTreeMap::new();
    for message in situation.messages {
        if message.is_waiting()
            && message.kind() == MessageKind::Question
            && specialists.contains_key(&message.to())
        {
            asked.entry(message.to()).or_default().push(message);
        }
    }
    let mut asked: Vec<(MemberId, Vec<&TeamMessage>)> = asked.into_iter().collect();
    for (_, messages) in &mut asked {
        messages.sort_by_key(|message| (message.sent_at(), message.id()));
    }
    asked.sort_by_key(|(_, messages)| {
        messages
            .first()
            .map(|message| (message.sent_at(), message.id()))
    });
    let mut message_turns = situation.message_turns_in_the_last_hour;
    for (member_id, messages) in asked {
        if given.contains(&member_id) {
            // The task it starts on brings the messages with it.
            continue;
        }
        let scopes = [BudgetScope::Project, BudgetScope::Member(member_id)];
        let reason = reason_for(member_id, &scopes, free_places, &given).or_else(|| {
            (message_turns >= MESSAGE_TURNS_PER_HOUR).then_some(WaitReason::MessageLimit)
        });
        if let Some(reason) = reason {
            if !matches!(
                reason,
                WaitReason::SpecialistBusy | WaitReason::SpecialistWaitsForManager
            ) {
                plan.waiting.push(Waiting {
                    task_id: None,
                    member_id,
                    reason,
                });
            }
            continue;
        }
        if held_by_plan {
            plan.waiting.push(Waiting {
                task_id: None,
                member_id,
                reason: WaitReason::PlanLimit,
            });
            continue;
        }
        free_places -= 1;
        message_turns += 1;
        given.insert(member_id);
        let mut asked_by: Vec<MemberId> = Vec::new();
        for message in messages {
            if !asked_by.contains(&message.from()) {
                asked_by.push(message.from());
            }
        }
        plan.wakes.push(Wake {
            member_id,
            asked_by,
        });
    }

    // Every budget that is used up is asked about, once, whether or not
    // it holds work back now.
    let mut questions: Vec<BudgetScope> = situation
        .budgets
        .iter()
        .filter(|budget| budget.needs_question(spending.of(budget.scope())))
        .map(Budget::scope)
        .collect();
    questions.sort();
    plan.questions = questions;
    plan
}

/// The sentences of the coordinator's record. They carry the names that
/// are true when they are written, like the events of a project.
pub mod words {
    use teamwright_domain::{BudgetScope, Member, MemberId, Project, Task, TaskId, Timestamp};

    use super::{WaitReason, WorkPermit, reference_of};
    use crate::plan_limit::reset_in_words;
    use crate::ports::Clock;

    /// How the line begins that says the limit of the plan held work back.
    /// The coordinator finds its earlier lines by it.
    pub const HELD_BY_PLAN_LIMIT: &str =
        "Started nothing: the limit of your Claude plan is reached.";

    /// What the coordinator's record calls a member.
    #[must_use]
    pub fn member_name(members: &[Member], member_id: MemberId) -> String {
        members
            .iter()
            .find(|member| member.id() == member_id)
            .map_or_else(
                || "A specialist who left the team".to_owned(),
                |member| member.name().to_string(),
            )
    }

    /// What the coordinator's record calls a task: its reference.
    #[must_use]
    pub fn task_name(project: &Project, tasks: &[Task], task_id: TaskId) -> String {
        tasks
            .iter()
            .find(|task| task.id() == task_id)
            .map_or_else(|| "a task".to_owned(), |task| reference_of(project, task))
    }

    /// A number of tokens with a comma between each three digits:
    /// 1,040,000.
    #[must_use]
    pub fn tokens(count: u64) -> String {
        let digits = count.to_string();
        let mut grouped = String::new();
        for (place, digit) in digits.chars().enumerate() {
            if place > 0 && (digits.len() - place).is_multiple_of(3) {
                grouped.push(',');
            }
            grouped.push(digit);
        }
        grouped
    }

    /// Several names joined into one phrase: "A", "A and B", "A, B and C".
    #[must_use]
    pub fn joined(names: &[String]) -> String {
        match names {
            [] => String::new(),
            [only] => only.clone(),
            [first @ .., last] => format!("{} and {last}", first.join(", ")),
        }
    }

    /// What a budget is called in a sentence.
    #[must_use]
    pub fn budget_name(
        scope: BudgetScope,
        project: &Project,
        tasks: &[Task],
        members: &[Member],
    ) -> String {
        match scope {
            BudgetScope::Project => "the budget of the project".to_owned(),
            BudgetScope::Member(member_id) => {
                format!("the budget of {}", member_name(members, member_id))
            }
            BudgetScope::Task(task_id) => {
                format!("the budget of {}", task_name(project, tasks, task_id))
            }
        }
    }

    /// "Firmware engineer started on SSN-7: SSN-4 and SSN-5 are done."
    #[must_use]
    pub fn started(
        project: &Project,
        tasks: &[Task],
        members: &[Member],
        task_id: TaskId,
        member_id: MemberId,
        after: &[TaskId],
    ) -> String {
        let who = member_name(members, member_id);
        let what = task_name(project, tasks, task_id);
        let names: Vec<String> = after
            .iter()
            .map(|id| task_name(project, tasks, *id))
            .collect();
        match names.len() {
            0 => format!("{who} started on {what}: it waits for nothing."),
            1 => format!("{who} started on {what}: {} is done.", names[0]),
            _ => format!("{who} started on {what}: {} are done.", joined(&names)),
        }
    }

    /// "Firmware engineer went on with SSN-7: the limit of your Claude plan
    /// stopped its last turn, and it has reset."
    #[must_use]
    pub fn went_on(
        project: &Project,
        tasks: &[Task],
        members: &[Member],
        task_id: TaskId,
        member_id: MemberId,
    ) -> String {
        format!(
            "{} went on with {}: the limit of your Claude plan stopped its last turn, and it has reset.",
            member_name(members, member_id),
            task_name(project, tasks, task_id)
        )
    }

    /// "Started nothing: the limit of your Claude plan is reached. It
    /// resets at 22:40." Nothing if work may start.
    #[must_use]
    pub fn held_by_plan_limit(
        permit: WorkPermit,
        now: Timestamp,
        clock: &dyn Clock,
    ) -> Option<String> {
        match permit {
            WorkPermit::Allowed => None,
            WorkPermit::NotUntil(resets_at) => Some(format!(
                "{HELD_BY_PLAN_LIMIT} It resets {}.",
                reset_in_words(resets_at, now, clock)
            )),
            WorkPermit::NotUntilUnknown => Some(format!(
                "{HELD_BY_PLAN_LIMIT} When it resets is not known: work goes on once a turn you start by hand goes through."
            )),
        }
    }

    /// "Firmware engineer was given a turn to answer Hardware engineer."
    #[must_use]
    pub fn woke(members: &[Member], member_id: MemberId, asked_by: &[MemberId]) -> String {
        let names: Vec<String> = asked_by
            .iter()
            .map(|id| member_name(members, *id))
            .collect();
        format!(
            "{} was given a turn to answer {}.",
            member_name(members, member_id),
            joined(&names)
        )
    }

    /// Why something waits, after a colon.
    #[must_use]
    pub fn reason(
        reason: WaitReason,
        project: &Project,
        tasks: &[Task],
        members: &[Member],
        member_id: MemberId,
    ) -> String {
        let who = member_name(members, member_id);
        match reason {
            WaitReason::Budget(scope) => {
                format!("{} is used up", budget_name(scope, project, tasks, members))
            }
            WaitReason::Limit => "as many specialists as the limit allows are at work".to_owned(),
            WaitReason::SpecialistBusy => format!("{who} is at work on something else"),
            WaitReason::SpecialistWaitsForManager => {
                format!("{who} waits for your answer to a question")
            }
            WaitReason::WaitsForCheck(task_id) => format!(
                "{} is finished and waits for you to accept it",
                task_name(project, tasks, task_id)
            ),
            WaitReason::Failed => "its last turn failed; reply to go on".to_owned(),
            WaitReason::PlanLimit => "the limit of your Claude plan is reached".to_owned(),
            WaitReason::PlanLimitAgain => {
                "it stopped at the limit of your Claude plan again as it went on; press Try again"
                    .to_owned()
            }
            WaitReason::MessageLimit => format!(
                "the specialists had {} turns for messages in the last hour",
                super::MESSAGE_TURNS_PER_HOUR
            ),
        }
    }

    /// "The budget of SSN-9 is used up (1,040,000 of 1,000,000 tokens).
    /// You are asked in the inbox."
    #[must_use]
    pub fn used_up(budget: &str, used: u64, limit: u64) -> String {
        let mut letters = budget.chars();
        let budget = letters.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(letters).collect()
        });
        format!(
            "{budget} is used up ({} of {} tokens). You are asked in the inbox.",
            tokens(used),
            tokens(limit)
        )
    }

    /// "Did not start SSN-9: the budget of the project is used up
    /// (1,040,000 of 1,000,000 tokens). You are asked in the inbox."
    /// `action` is what was not done: "start SSN-9".
    #[must_use]
    pub fn held_by_budget(action: &str, budget: &str, used: u64, limit: u64) -> String {
        format!(
            "Did not {action}: {budget} is used up ({} of {} tokens). You are asked in the inbox.",
            tokens(used),
            tokens(limit)
        )
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod service_tests;
