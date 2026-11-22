//! The use cases of the coordinator: switching it on and off, its limit,
//! the budgets and the answers about them, and acting on what
//! [`next_steps`] says.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::TimeDelta;
use teamwright_domain::{
    Budget, BudgetAnswer, BudgetScope, ComputerId, ConcurrencyLimit, Coordinator, CoordinatorEntry,
    CoordinatorEntryKind, MemberKind, Project, ProjectId, TokenBudget,
};

use super::{
    GO_ON_REQUEST, Plan, Situation, Spending, WaitReason, Waiting, next_steps, spending_of, words,
};
use crate::error::ApplicationError;
use crate::plan_limit::{PlanLimitService, WorkPermit};
use crate::ports::{
    Clock, ConversationRepository, CoordinatorRepository, MemberRepository, ProjectRepository,
    TaskRepository, TeamworkRepository,
};
use crate::work::{AgentProgramState, ConversationWithTurns, WorkService};

/// What a specialist is told in a turn the coordinator gives it to answer
/// its teammates. The messages that wait for it follow, as with every turn.
pub const WAKE_REQUEST: &str = "Teammates asked you something; their messages follow. \
Answer each question with send_message, in a few sentences, and then end your turn. \
Start no other work in this turn.";

/// How many entries of the record the state of a coordinator carries.
const ENTRIES_SHOWN: usize = 30;

/// Everything the coordinator needs from the outside world.
pub struct CoordinatorPorts {
    /// Where projects are stored.
    pub projects: Arc<dyn ProjectRepository>,
    /// Where tasks are stored.
    pub tasks: Arc<dyn TaskRepository>,
    /// Where the members of the teams are stored.
    pub members: Arc<dyn MemberRepository>,
    /// Where conversations and their turns are stored.
    pub conversations: Arc<dyn ConversationRepository>,
    /// Where the messages between specialists are stored.
    pub teamwork: Arc<dyn TeamworkRepository>,
    /// Where the coordinators, their budgets and their records are stored.
    pub coordinators: Arc<dyn CoordinatorRepository>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The identity of this computer. The coordinator of a project acts
    /// only on the computer it was switched on on.
    pub this_computer: ComputerId,
    /// The work of the specialists, through which it starts turns, as the
    /// manager would.
    pub work: WorkService,
    /// What this computer knows about the limit of its Claude plan. While
    /// the limit is reached, the coordinator starts nothing.
    pub plan_limit: PlanLimitService,
}

/// The coordinator of a project as the manager sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinatorState {
    /// Its switch and its limit.
    pub coordinator: Coordinator,
    /// The budgets of the project, also those taken away.
    pub budgets: Vec<Budget>,
    /// What was spent, against which the budgets count.
    pub spending: Spending,
    /// How many specialists are at work now, on every computer.
    pub at_work: usize,
    /// What could start but waits, and why, in the order it would start.
    pub waiting: Vec<Waiting>,
    /// The latest entries of its record, the newest first.
    pub entries: Vec<CoordinatorEntry>,
}

/// A question about a used-up budget that waits for the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetQuestionOfProject {
    /// The project.
    pub project: Project,
    /// The budget, with its open question.
    pub budget: Budget,
    /// What the budget is called: "the budget of the project".
    pub name: String,
    /// What has been spent against it now.
    pub used: u64,
}

/// The use cases of the coordinator.
#[derive(Clone)]
pub struct CoordinatorService {
    inner: Arc<Inner>,
}

struct Inner {
    ports: CoordinatorPorts,
    /// Held while the coordinator acts, so that two acts never start the
    /// same task.
    acting: Mutex<()>,
}

impl CoordinatorService {
    /// Creates the service.
    #[must_use]
    pub fn new(ports: CoordinatorPorts) -> Self {
        Self {
            inner: Arc::new(Inner {
                ports,
                acting: Mutex::new(()),
            }),
        }
    }

    fn ports(&self) -> &CoordinatorPorts {
        &self.inner.ports
    }

    /// The coordinator of a project, with its budgets, what waits and its
    /// latest record.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn state(&self, project_id: ProjectId) -> Result<CoordinatorState, ApplicationError> {
        let project = self.require_project(project_id)?;
        let gathered = self.gather(&project)?;
        let plan = gathered.plan(&project);
        let at_work = gathered
            .conversations
            .iter()
            .filter(|conversation| !conversation.conversation.is_with_advisor())
            .filter(|conversation| conversation.turns.iter().any(|turn| turn.is_running()))
            .filter_map(|conversation| conversation.conversation.member_id())
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        Ok(CoordinatorState {
            coordinator: gathered.coordinator,
            budgets: gathered.budgets,
            spending: spending_of(&gathered.conversations),
            at_work,
            waiting: plan.waiting,
            entries: self
                .ports()
                .coordinators
                .latest_entries(project_id, ENTRIES_SHOWN)?,
        })
    }

    /// Switches the coordinator of a project on, on this computer. If it
    /// ran on another computer, it runs here from now on, and the other
    /// computer starts nothing more once it has heard of it. It acts at
    /// the next [`Self::act`].
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn switch_on(&self, project_id: ProjectId) -> Result<CoordinatorState, ApplicationError> {
        let ports = self.ports();
        self.require_project(project_id)?;
        let mut coordinator = self.coordinator_of(project_id)?;
        if !coordinator.runs_on_computer(ports.this_computer) {
            let now = ports.clock.now();
            coordinator.switch_on(ports.this_computer, now);
            ports.coordinators.save_coordinator(&coordinator)?;
            self.record(&[CoordinatorEntry::new(
                project_id,
                ports.this_computer,
                CoordinatorEntryKind::SwitchedOn,
                &format!(
                    "Switched on. Tasks start as soon as they are ready, with at most {} {} at work at once.",
                    coordinator.limit().value(),
                    specialists(coordinator.limit().value())
                ),
                now,
            )])?;
        }
        self.state(project_id)
    }

    /// Switches the coordinator of a project off, wherever it runs. Work
    /// under way goes on; nothing new starts.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn switch_off(&self, project_id: ProjectId) -> Result<CoordinatorState, ApplicationError> {
        self.require_project(project_id)?;
        self.switch_off_because(
            project_id,
            "Switched off. Work under way goes on; nothing new starts.",
        )?;
        self.state(project_id)
    }

    /// Sets how many specialists the coordinator lets work at once.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the limit is out of
    /// its range (`rule_broken`), or if storage fails.
    pub fn set_limit(
        &self,
        project_id: ProjectId,
        limit: i64,
    ) -> Result<CoordinatorState, ApplicationError> {
        self.require_project(project_id)?;
        let limit = ConcurrencyLimit::new(limit)?;
        let mut coordinator = self.coordinator_of(project_id)?;
        coordinator.set_limit(limit);
        self.ports().coordinators.save_coordinator(&coordinator)?;
        self.state(project_id)
    }

    /// Sets the budget of a project, a member or a task, in tokens, or
    /// takes it away with nothing. A question about it that was open is
    /// answered by the change.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the member or the
    /// task is not one of the project (`not_found`), if the number is out of
    /// its range (`rule_broken`), or if storage fails.
    pub fn set_budget(
        &self,
        project_id: ProjectId,
        scope: BudgetScope,
        tokens: Option<u64>,
    ) -> Result<CoordinatorState, ApplicationError> {
        self.require_project(project_id)?;
        self.require_scope(project_id, scope)?;
        let tokens = tokens.map(TokenBudget::new).transpose()?;
        let stored = self
            .ports()
            .coordinators
            .budgets_of_project(project_id)?
            .into_iter()
            .find(|budget| budget.scope() == scope);
        let budget = match (stored, tokens) {
            (Some(mut budget), tokens) => {
                budget.set(tokens);
                Some(budget)
            }
            (None, Some(tokens)) => Some(Budget::new(project_id, scope, tokens)),
            (None, None) => None,
        };
        if let Some(budget) = budget {
            self.ports().coordinators.save_budget(&budget)?;
        }
        self.state(project_id)
    }

    /// Answers the question about a used-up budget: raise it by a number
    /// of tokens, or leave the work against it stopped.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if no question is open
    /// about the budget (`rule_broken`), or if storage fails.
    pub fn answer_budget(
        &self,
        project_id: ProjectId,
        scope: BudgetScope,
        answer: BudgetAnswer,
    ) -> Result<CoordinatorState, ApplicationError> {
        self.require_project(project_id)?;
        let mut budget = self
            .ports()
            .coordinators
            .budgets_of_project(project_id)?
            .into_iter()
            .find(|budget| budget.scope() == scope)
            .ok_or(teamwright_domain::DomainError::NoBudgetQuestion)?;
        budget.answer(answer, self.ports().clock.now())?;
        self.ports().coordinators.save_budget(&budget)?;
        self.state(project_id)
    }

    /// The questions about used-up budgets that wait for the manager, in
    /// every project that is not put away, the one asked first first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn budget_questions(&self) -> Result<Vec<BudgetQuestionOfProject>, ApplicationError> {
        let ports = self.ports();
        let mut questions = Vec::new();
        for project in ports.projects.list()? {
            if project.is_archived() {
                continue;
            }
            let budgets = ports.coordinators.budgets_of_project(project.id())?;
            if budgets
                .iter()
                .all(|budget| budget.open_question().is_none())
            {
                continue;
            }
            let conversations = self.conversations_of(&project)?;
            let spending = spending_of(&conversations);
            let tasks = ports.tasks.list_for_project(project.id())?;
            let members = ports.members.list_for_project(project.id())?;
            for budget in budgets {
                if budget.open_question().is_some() {
                    questions.push(BudgetQuestionOfProject {
                        name: words::budget_name(budget.scope(), &project, &tasks, &members),
                        used: spending.of(budget.scope()),
                        project: project.clone(),
                        budget,
                    });
                }
            }
        }
        questions
            .sort_by_key(|question| question.budget.open_question().map(|asked| asked.asked_at));
        Ok(questions)
    }

    /// The projects whose coordinator runs on this computer.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn running_here(&self) -> Result<Vec<ProjectId>, ApplicationError> {
        let ports = self.ports();
        let mut running = Vec::new();
        for project in ports.projects.list()? {
            if !project.is_archived()
                && self
                    .coordinator_of(project.id())?
                    .runs_on_computer(ports.this_computer)
            {
                running.push(project.id());
            }
        }
        Ok(running)
    }

    /// Does what [`next_steps`] says for a project, if its coordinator runs
    /// on this computer: starts the tasks that are ready, gives specialists
    /// a turn to answer their teammates, asks the manager about a budget
    /// that holds work back, and writes down what it did and why. Answers
    /// with the entries it wrote.
    ///
    /// Call it when something happened that may change what can start: the
    /// coordinator was switched on, a turn ended, a task changed, the
    /// manager answered, a budget or the limit changed, the program
    /// started. Never call it while the work of the specialists reports an
    /// event (from a [`crate::WorkListener`]): starting a turn waits for the
    /// work, which is busy with that event.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails. A turn that cannot be started is
    /// written down instead.
    pub fn act(&self, project_id: ProjectId) -> Result<Vec<CoordinatorEntry>, ApplicationError> {
        let _acting = lock(&self.inner.acting);
        let ports = self.ports();
        let Some(project) = ports.projects.find(project_id)? else {
            return Ok(Vec::new());
        };
        let coordinator = self.coordinator_of(project_id)?;
        if project.is_archived() || !coordinator.runs_on_computer(ports.this_computer) {
            return Ok(Vec::new());
        }
        let gathered = self.gather(&project)?;
        let plan = gathered.plan(&project);
        let now = ports.clock.now();
        let entry = |kind, text: &str| {
            CoordinatorEntry::new(project_id, ports.this_computer, kind, text, now)
        };
        let mut entries = Vec::new();

        if !plan.starts.is_empty() || !plan.goes_on.is_empty() || !plan.wakes.is_empty() {
            let program = ports.work.program();
            let problem = match program.state {
                AgentProgramState::Ready => None,
                AgentProgramState::NotInstalled => {
                    Some("Claude Code was not found on this computer".to_owned())
                }
                AgentProgramState::SignedOut => Some("Claude Code is not signed in".to_owned()),
                AgentProgramState::Unavailable => Some(format!(
                    "Claude Code could not be asked ({})",
                    program.problem.unwrap_or_default()
                )),
            };
            if let Some(problem) = problem {
                return self.switch_off_because(
                    project_id,
                    &format!("Switched itself off: {problem}. Switch it on again once it works."),
                );
            }
        }

        let tasks = &gathered.tasks;
        let members = &gathered.members;
        for start in &plan.starts {
            match ports.work.start_work_by_coordinator(start.task_id) {
                Ok(_) => entries.push(
                    entry(
                        CoordinatorEntryKind::Started,
                        &words::started(
                            &project,
                            tasks,
                            members,
                            start.task_id,
                            start.member_id,
                            &start.after,
                        ),
                    )
                    .about_task(start.task_id)
                    .about_member(start.member_id),
                ),
                Err(ApplicationError::AgentProgram(error)) => {
                    self.record(&entries)?;
                    let mut stopped = entries;
                    stopped.extend(self.switch_off_because(
                        project_id,
                        &format!("Switched itself off: {error}. Switch it on again once it works."),
                    )?);
                    return Ok(stopped);
                }
                Err(error) => entries.push(
                    entry(
                        CoordinatorEntryKind::Held,
                        &format!(
                            "Could not start {}: {error}.",
                            words::task_name(&project, tasks, start.task_id)
                        ),
                    )
                    .about_task(start.task_id),
                ),
            }
        }

        for going in &plan.goes_on {
            // As the manager's "Try again" does: a reply in the conversation
            // whose turn the limit ended. Unlike it, a turn nobody started by
            // hand, so it runs no command (decision 0038).
            let text = match ports
                .work
                .reply_by_coordinator(going.conversation_id, GO_ON_REQUEST)
            {
                Ok(_) => words::went_on(&project, tasks, members, going.task_id, going.member_id),
                Err(error) => format!(
                    "Could not go on with {}: {error}.",
                    words::task_name(&project, tasks, going.task_id)
                ),
            };
            let kind = if text.starts_with("Could not") {
                CoordinatorEntryKind::Held
            } else {
                CoordinatorEntryKind::Started
            };
            entries.push(
                entry(kind, &text)
                    .about_task(going.task_id)
                    .about_member(going.member_id),
            );
        }

        for wake in &plan.wakes {
            let started = match gathered.conversation_for_answers(wake.member_id, ports) {
                Some(conversation) => ports.work.reply_by_coordinator(conversation, WAKE_REQUEST),
                None => ports
                    .work
                    .ask_to_answer_teammates(wake.member_id, WAKE_REQUEST),
            };
            let text = match started {
                Ok(_) => words::woke(members, wake.member_id, &wake.asked_by),
                Err(error) => format!(
                    "Could not give {} a turn to answer its teammates: {error}.",
                    words::member_name(members, wake.member_id)
                ),
            };
            let kind = if text.starts_with("Could not") {
                CoordinatorEntryKind::Held
            } else {
                CoordinatorEntryKind::Woke
            };
            entries.push(entry(kind, &text).about_member(wake.member_id));
        }

        // A budget that held work back is asked about once.
        let mut budgets = gathered.budgets.clone();
        let spending = spending_of(&gathered.conversations);
        for scope in &plan.questions {
            let Some(budget) = budgets.iter_mut().find(|budget| budget.scope() == *scope) else {
                continue;
            };
            let used = spending.of(*scope);
            let limit = budget.tokens().map_or(0, TokenBudget::tokens);
            budget.ask(used, now)?;
            ports.coordinators.save_budget(budget)?;
            let held = plan
                .waiting
                .iter()
                .find(|waiting| waiting.reason == WaitReason::Budget(*scope));
            let name = words::budget_name(*scope, &project, tasks, members);
            let text = match held {
                Some(waiting) => words::held_by_budget(
                    &gathered.action_of(&project, waiting),
                    &name,
                    used,
                    limit,
                ),
                None => words::used_up(&name, used, limit),
            };
            let mut asked = entry(CoordinatorEntryKind::Asked, &text);
            if let Some(waiting) = held {
                asked = asked.about_member(waiting.member_id);
                if let Some(task_id) = waiting.task_id {
                    asked = asked.about_task(task_id);
                }
            }
            entries.push(asked);
        }

        // The limit on turns for messages is told once an hour.
        let an_hour_ago = now - TimeDelta::hours(1);
        let told: Vec<String> = ports
            .coordinators
            .entries_since(project_id, an_hour_ago)?
            .into_iter()
            .filter(|entry| entry.kind == CoordinatorEntryKind::Held)
            .map(|entry| entry.text)
            .collect();
        for waiting in plan
            .waiting
            .iter()
            .filter(|waiting| waiting.reason == WaitReason::MessageLimit)
        {
            let text = format!(
                "Did not {}: {}. The question waits for its next turn.",
                gathered.action_of(&project, waiting),
                words::reason(waiting.reason, &project, tasks, members, waiting.member_id)
            );
            if !told.contains(&text) {
                entries
                    .push(entry(CoordinatorEntryKind::Held, &text).about_member(waiting.member_id));
            }
        }

        // The limit of the plan that holds work back is told once for each
        // time it is reached, not at every nudge.
        let held_by_plan = plan
            .waiting
            .iter()
            .any(|waiting| waiting.reason == WaitReason::PlanLimit);
        if held_by_plan
            && let Some(text) =
                words::held_by_plan_limit(gathered.permit, now, ports.clock.as_ref())
        {
            let reached_at = ports
                .plan_limit
                .standing()?
                .reached
                .map_or(now, |limit| limit.at);
            let told = ports
                .coordinators
                .entries_since(project_id, reached_at)?
                .iter()
                .any(is_plan_limit_told);
            if !told {
                entries.push(entry(CoordinatorEntryKind::Held, &text));
            }
        }

        self.record(&entries)?;
        Ok(entries)
    }

    /// Switches the coordinator off, if it is on, and writes down why.
    fn switch_off_because(
        &self,
        project_id: ProjectId,
        why: &str,
    ) -> Result<Vec<CoordinatorEntry>, ApplicationError> {
        let ports = self.ports();
        let mut coordinator = self.coordinator_of(project_id)?;
        if coordinator.runs_on().is_none() {
            return Ok(Vec::new());
        }
        let now = ports.clock.now();
        coordinator.switch_off(now);
        ports.coordinators.save_coordinator(&coordinator)?;
        let entries = vec![CoordinatorEntry::new(
            project_id,
            ports.this_computer,
            CoordinatorEntryKind::SwitchedOff,
            why,
            now,
        )];
        self.record(&entries)?;
        Ok(entries)
    }

    fn record(&self, entries: &[CoordinatorEntry]) -> Result<(), ApplicationError> {
        if !entries.is_empty() {
            self.ports().coordinators.add_entries(entries)?;
        }
        Ok(())
    }

    fn coordinator_of(&self, project_id: ProjectId) -> Result<Coordinator, ApplicationError> {
        Ok(self
            .ports()
            .coordinators
            .coordinator_of(project_id)?
            .unwrap_or_else(|| Coordinator::of(project_id)))
    }

    fn require_project(&self, project_id: ProjectId) -> Result<Project, ApplicationError> {
        self.ports()
            .projects
            .find(project_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: project_id.to_string(),
            })
    }

    /// Refuses a member or a task that is not one of the project.
    fn require_scope(
        &self,
        project_id: ProjectId,
        scope: BudgetScope,
    ) -> Result<(), ApplicationError> {
        let ports = self.ports();
        let belongs = match scope {
            BudgetScope::Project => true,
            BudgetScope::Member(member_id) => ports
                .members
                .find(member_id)?
                .is_some_and(|member| member.project_id() == project_id),
            BudgetScope::Task(task_id) => ports
                .tasks
                .find(task_id)?
                .is_some_and(|task| task.project_id() == project_id),
        };
        if belongs {
            Ok(())
        } else {
            Err(ApplicationError::NotFound {
                kind: match scope {
                    BudgetScope::Member(_) => "team member",
                    _ => "task",
                },
                id: scope.target(project_id).to_string(),
            })
        }
    }

    fn conversations_of(
        &self,
        project: &Project,
    ) -> Result<Vec<ConversationWithTurns>, ApplicationError> {
        let ports = self.ports();
        ports
            .conversations
            .list_for_project(project.id())?
            .into_iter()
            .map(|conversation| {
                let turns = ports.conversations.turns_of(conversation.id())?;
                Ok(ConversationWithTurns {
                    conversation,
                    turns,
                })
            })
            .collect()
    }

    /// Reads everything [`next_steps`] needs about a project.
    fn gather(&self, project: &Project) -> Result<Gathered, ApplicationError> {
        let ports = self.ports();
        let an_hour_ago = ports.clock.now() - TimeDelta::hours(1);
        let message_turns = ports
            .coordinators
            .entries_since(project.id(), an_hour_ago)?
            .iter()
            .filter(|entry| entry.kind == CoordinatorEntryKind::Woke)
            .count();
        let conversations = self.conversations_of(project)?;
        // When the coordinator last said that the limit held work back,
        // looked for only as far back as a turn the limit ended.
        let plan_limit_told_at = match conversations
            .iter()
            .filter_map(|conversation| conversation.turns.last())
            .filter(|turn| turn.reached_plan_limit())
            .map(teamwright_domain::Turn::started_at)
            .min()
        {
            Some(since) => ports
                .coordinators
                .entries_since(project.id(), since)?
                .iter()
                .filter(|entry| is_plan_limit_told(entry))
                .map(|entry| entry.at)
                .max(),
            None => None,
        };
        Ok(Gathered {
            coordinator: self.coordinator_of(project.id())?,
            tasks: ports.tasks.list_for_project(project.id())?,
            members: ports.members.list_for_project(project.id())?,
            conversations,
            messages: ports.teamwork.messages_of_project(project.id())?,
            budgets: ports.coordinators.budgets_of_project(project.id())?,
            message_turns,
            permit: ports.plan_limit.may_start_work()?,
            plan_limit_told_at,
        })
    }
}

/// What the coordinator read about a project.
struct Gathered {
    coordinator: Coordinator,
    tasks: Vec<teamwright_domain::Task>,
    members: Vec<teamwright_domain::Member>,
    conversations: Vec<ConversationWithTurns>,
    messages: Vec<teamwright_domain::TeamMessage>,
    budgets: Vec<Budget>,
    message_turns: usize,
    permit: WorkPermit,
    plan_limit_told_at: Option<teamwright_domain::Timestamp>,
}

impl Gathered {
    fn plan(&self, project: &Project) -> Plan {
        next_steps(&Situation {
            project,
            tasks: &self.tasks,
            members: &self.members,
            conversations: &self.conversations,
            messages: &self.messages,
            budgets: &self.budgets,
            limit: self.coordinator.limit(),
            message_turns_in_the_last_hour: self.message_turns,
            plan: self.permit,
            plan_limit_told_at: self.plan_limit_told_at,
        })
    }

    /// What was not done, after "Did not": "start SSN-9", or "give
    /// Firmware engineer a turn to answer its teammates".
    fn action_of(&self, project: &Project, waiting: &Waiting) -> String {
        match waiting.task_id {
            Some(task_id) => format!("start {}", words::task_name(project, &self.tasks, task_id)),
            None => format!(
                "give {} a turn to answer its teammates",
                words::member_name(&self.members, waiting.member_id)
            ),
        }
    }

    /// The conversation in which a specialist answers its teammates: its
    /// latest conversation that is not about a task, runs on this computer
    /// and waits for nothing, so that its session knows what it said
    /// before. Nothing if it has none; a new one is then opened.
    fn conversation_for_answers(
        &self,
        member_id: teamwright_domain::MemberId,
        ports: &CoordinatorPorts,
    ) -> Option<teamwright_domain::ConversationId> {
        self.conversations
            .iter()
            .filter(|conversation| {
                let talk = &conversation.conversation;
                talk.member_id() == Some(member_id)
                    && talk.task_id().is_none()
                    && !talk.is_with_advisor()
                    && talk.is_at_home_on(ports.this_computer)
                    && conversation.turns.last().is_none_or(|turn| {
                        !turn.is_running() && turn.decision().is_none_or(|asked| !asked.is_open())
                    })
            })
            .max_by_key(|conversation| conversation.conversation.started_at())
            .map(|conversation| conversation.conversation.id())
            .filter(|_| {
                self.members
                    .iter()
                    .any(|member| member.id() == member_id && member.kind() == MemberKind::Agent)
            })
    }
}

/// Whether an entry of the record is the line that said the limit of the
/// plan held work back.
fn is_plan_limit_told(entry: &CoordinatorEntry) -> bool {
    entry.kind == CoordinatorEntryKind::Held && entry.text.starts_with(words::HELD_BY_PLAN_LIMIT)
}

/// "specialist" or "specialists".
const fn specialists(count: u8) -> &'static str {
    if count == 1 {
        "specialist"
    } else {
        "specialists"
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
