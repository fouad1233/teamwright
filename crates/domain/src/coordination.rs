//! The coordinator of a project: whether it runs, how many specialists it
//! lets work at once, the budgets in tokens it keeps to, and the record of
//! what it did.
//!
//! The coordinator is program logic, not a model
//! ([decision 0006](../../../docs/decisions/0006-team-layer-in-the-app.md)).
//! What it decides, from the state of a whole project, is the business of
//! the application. The rules here concern one value: a budget, a limit,
//! the switch, one entry of the record.

use crate::error::DomainError;
use crate::ids::{ComputerId, CoordinatorEntryId, MemberId, ProjectId, TaskId};
use crate::macros::text_enum;
use crate::time::Timestamp;

/// A budget in tokens: how many a project, a member or a task may use
/// before the coordinator starts no more work against it
/// ([decision 0008](../../../docs/decisions/0008-budgets-in-tokens.md)).
///
/// It counts every token of every kind, as the usage of a project does:
/// read, written, read again from the cache and written to the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenBudget(u64);

impl TokenBudget {
    /// The smallest budget: a thousand tokens. Less would stop every turn.
    pub const MIN: u64 = 1_000;
    /// The largest budget: a million million tokens, far beyond any plan.
    pub const MAX: u64 = 1_000_000_000_000;

    /// Checks a number of tokens.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::BudgetOutOfRange`] below [`Self::MIN`] or
    /// above [`Self::MAX`].
    pub const fn new(tokens: u64) -> Result<Self, DomainError> {
        if tokens < Self::MIN || tokens > Self::MAX {
            return Err(DomainError::BudgetOutOfRange {
                min: Self::MIN,
                max: Self::MAX,
            });
        }
        Ok(Self(tokens))
    }

    /// The number of tokens.
    #[must_use]
    pub const fn tokens(self) -> u64 {
        self.0
    }

    /// Whether `used` tokens use the budget up.
    #[must_use]
    pub const fn is_used_up_by(self, used: u64) -> bool {
        used >= self.0
    }

    /// The budget raised by `by` tokens, at most to [`Self::MAX`].
    #[must_use]
    pub const fn raised_by(self, by: u64) -> Self {
        let raised = self.0.saturating_add(by);
        Self(if raised > Self::MAX {
            Self::MAX
        } else {
            raised
        })
    }

    /// The two raises the manager is offered when the budget is used up:
    /// half of it and all of it again, each rounded to two significant
    /// digits so that the numbers read easily.
    #[must_use]
    pub fn raises(self) -> [u64; 2] {
        [round_to_two_digits(self.0 / 2), round_to_two_digits(self.0)]
    }
}

/// A number rounded down to two significant digits, at least one
/// thousand: 123,456 becomes 120,000.
fn round_to_two_digits(number: u64) -> u64 {
    let mut unit = 1_u64;
    while number / unit >= 100 {
        unit = unit.saturating_mul(10);
    }
    (number / unit * unit).max(TokenBudget::MIN)
}

/// How many specialists the coordinator lets work at once in a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConcurrencyLimit(u8);

impl ConcurrencyLimit {
    /// The least: one specialist at a time, the work in a row.
    pub const MIN: u8 = 1;
    /// The most. A team is small (the vision: three well-briefed
    /// specialists beat ten vague ones), and every specialist at work is a
    /// program on this computer and a share of the plan.
    pub const MAX: u8 = 8;
    /// The limit a project starts with: two specialists at once, so that
    /// work that can go side by side does, while the manager can still
    /// follow both and the plan is spent no faster than twice a single
    /// session.
    pub const DEFAULT: Self = Self(2);

    /// Checks a limit.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ConcurrencyOutOfRange`] outside
    /// [`Self::MIN`]..=[`Self::MAX`].
    pub fn new(limit: i64) -> Result<Self, DomainError> {
        u8::try_from(limit)
            .ok()
            .filter(|limit| (Self::MIN..=Self::MAX).contains(limit))
            .map(Self)
            .ok_or(DomainError::ConcurrencyOutOfRange {
                min: Self::MIN,
                max: Self::MAX,
            })
    }

    /// The number of specialists.
    #[must_use]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for ConcurrencyLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The coordinator of one project: its limit, and where it runs.
///
/// It runs on one computer at most: the one where the manager switched it
/// on. Another computer that has the project sees that it runs there and
/// starts nothing, so that two computers never start the same task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coordinator {
    project_id: ProjectId,
    limit: ConcurrencyLimit,
    runs_on: Option<ComputerId>,
    switched_at: Option<Timestamp>,
}

impl Coordinator {
    /// The coordinator a project has until the manager changes it: off,
    /// with the default limit.
    #[must_use]
    pub fn of(project_id: ProjectId) -> Self {
        Self {
            project_id,
            limit: ConcurrencyLimit::DEFAULT,
            runs_on: None,
            switched_at: None,
        }
    }

    /// Rebuilds a coordinator from storage.
    #[must_use]
    pub const fn restore(
        project_id: ProjectId,
        limit: ConcurrencyLimit,
        runs_on: Option<ComputerId>,
        switched_at: Option<Timestamp>,
    ) -> Self {
        Self {
            project_id,
            limit,
            runs_on,
            switched_at,
        }
    }

    /// The project.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// How many specialists may work at once.
    #[must_use]
    pub const fn limit(&self) -> ConcurrencyLimit {
        self.limit
    }

    /// The computer it runs on, or nothing while it is off.
    #[must_use]
    pub const fn runs_on(&self) -> Option<ComputerId> {
        self.runs_on
    }

    /// When it was last switched on or off.
    #[must_use]
    pub const fn switched_at(&self) -> Option<Timestamp> {
        self.switched_at
    }

    /// Whether it runs on `computer`.
    #[must_use]
    pub fn runs_on_computer(&self, computer: ComputerId) -> bool {
        self.runs_on == Some(computer)
    }

    /// Switches it on, on `computer`. Switching it on where it runs on
    /// another computer moves it here: the other computer sees that and
    /// starts nothing more.
    pub fn switch_on(&mut self, computer: ComputerId, now: Timestamp) {
        if self.runs_on != Some(computer) {
            self.runs_on = Some(computer);
            self.switched_at = Some(now);
        }
    }

    /// Switches it off, wherever it runs.
    pub fn switch_off(&mut self, now: Timestamp) {
        if self.runs_on.is_some() {
            self.runs_on = None;
            self.switched_at = Some(now);
        }
    }

    /// Sets how many specialists may work at once.
    pub const fn set_limit(&mut self, limit: ConcurrencyLimit) {
        self.limit = limit;
    }
}

/// What a budget is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BudgetScope {
    /// The whole project, the Advisor included.
    Project,
    /// One member of the team, in all its conversations.
    Member(MemberId),
    /// One task, in all the conversations about it.
    Task(TaskId),
}

/// The kind of a budget's scope, as storage and the interface name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BudgetScopeKind {
    /// The whole project.
    Project,
    /// One member of the team.
    Member,
    /// One task.
    Task,
}

text_enum!(BudgetScopeKind, kind = "kind of budget", {
    Project => "project",
    Member => "member",
    Task => "task",
});

impl BudgetScope {
    /// The kind of the scope.
    #[must_use]
    pub const fn kind(self) -> BudgetScopeKind {
        match self {
            Self::Project => BudgetScopeKind::Project,
            Self::Member(_) => BudgetScopeKind::Member,
            Self::Task(_) => BudgetScopeKind::Task,
        }
    }

    /// The identifier of what the budget is for: the project, the member
    /// or the task. It is also the identifier of the budget, so that two
    /// computers that each set the budget of one thing set the same one.
    #[must_use]
    pub fn target(self, project_id: ProjectId) -> uuid::Uuid {
        match self {
            Self::Project => project_id.as_uuid(),
            Self::Member(member_id) => member_id.as_uuid(),
            Self::Task(task_id) => task_id.as_uuid(),
        }
    }

    /// The scope of a kind and the identifier of its target.
    #[must_use]
    pub const fn from_parts(kind: BudgetScopeKind, target: uuid::Uuid) -> Self {
        match kind {
            BudgetScopeKind::Project => Self::Project,
            BudgetScopeKind::Member => Self::Member(MemberId::from_uuid(target)),
            BudgetScopeKind::Task => Self::Task(TaskId::from_uuid(target)),
        }
    }
}

/// What the manager answered when a budget was used up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetAnswer {
    /// The budget was raised by this many tokens.
    Raised {
        /// How many tokens were added.
        by: u64,
    },
    /// The work against the budget stays stopped.
    LeftStopped,
}

/// The question the coordinator asked the manager when a budget was used
/// up, and the answer, once there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetQuestion {
    /// When it was asked.
    pub asked_at: Timestamp,
    /// The budget when it was asked.
    pub limit: TokenBudget,
    /// How many tokens had been used when it was asked.
    pub used: u64,
    /// The answer, and when it was given.
    pub answer: Option<(BudgetAnswer, Timestamp)>,
}

/// A budget in tokens for a project, a member or a task, and the question
/// the coordinator asked about it.
///
/// A budget is never removed while its project lives: taking it away
/// leaves it without tokens, so that the manager can set it again (a
/// removal would be final on every computer, decision 0017).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    project_id: ProjectId,
    scope: BudgetScope,
    tokens: Option<TokenBudget>,
    question: Option<BudgetQuestion>,
}

impl Budget {
    /// A budget the manager sets for the first time.
    #[must_use]
    pub const fn new(project_id: ProjectId, scope: BudgetScope, tokens: TokenBudget) -> Self {
        Self {
            project_id,
            scope,
            tokens: Some(tokens),
            question: None,
        }
    }

    /// Rebuilds a budget from storage.
    #[must_use]
    pub const fn restore(
        project_id: ProjectId,
        scope: BudgetScope,
        tokens: Option<TokenBudget>,
        question: Option<BudgetQuestion>,
    ) -> Self {
        Self {
            project_id,
            scope,
            tokens,
            question,
        }
    }

    /// The project it belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// What it is for.
    #[must_use]
    pub const fn scope(&self) -> BudgetScope {
        self.scope
    }

    /// Its identifier: that of what it is for.
    #[must_use]
    pub fn id(&self) -> uuid::Uuid {
        self.scope.target(self.project_id)
    }

    /// The tokens it allows, or nothing if the manager took it away.
    #[must_use]
    pub const fn tokens(&self) -> Option<TokenBudget> {
        self.tokens
    }

    /// The question asked about it, answered or not.
    #[must_use]
    pub const fn question(&self) -> Option<&BudgetQuestion> {
        self.question.as_ref()
    }

    /// Whether `used` tokens use it up. A budget without tokens never is.
    #[must_use]
    pub fn is_used_up_by(&self, used: u64) -> bool {
        self.tokens.is_some_and(|tokens| tokens.is_used_up_by(used))
    }

    /// The question that waits for the manager: asked about the budget as
    /// it stands, and not answered. A question about a budget that was
    /// changed since was answered by the change.
    #[must_use]
    pub fn open_question(&self) -> Option<&BudgetQuestion> {
        self.question
            .as_ref()
            .filter(|question| question.answer.is_none() && Some(question.limit) == self.tokens)
    }

    /// Whether the manager is to be asked about it now: `used` tokens use
    /// it up, and nothing was asked about it as it stands. The manager is
    /// asked once for each budget: an answer, or a change of the budget,
    /// is needed before the question comes again.
    #[must_use]
    pub fn needs_question(&self, used: u64) -> bool {
        let Some(tokens) = self.tokens else {
            return false;
        };
        tokens.is_used_up_by(used)
            && self
                .question
                .is_none_or(|question| question.limit != tokens)
    }

    /// Records that the manager was asked about it.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NoBudget`] if it has no tokens.
    pub fn ask(&mut self, used: u64, now: Timestamp) -> Result<(), DomainError> {
        let limit = self.tokens.ok_or(DomainError::NoBudget)?;
        self.question = Some(BudgetQuestion {
            asked_at: now,
            limit,
            used,
            answer: None,
        });
        Ok(())
    }

    /// Sets the tokens it allows, or takes them away with nothing. A
    /// question that was open is answered by the change.
    pub const fn set(&mut self, tokens: Option<TokenBudget>) {
        self.tokens = tokens;
    }

    /// Answers the open question.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NoBudgetQuestion`] if no question is open, or
    /// [`DomainError::BudgetOutOfRange`] for a raise of nothing.
    pub fn answer(&mut self, answer: BudgetAnswer, now: Timestamp) -> Result<(), DomainError> {
        if self.open_question().is_none() {
            return Err(DomainError::NoBudgetQuestion);
        }
        let Some(question) = self.question.as_mut() else {
            return Err(DomainError::NoBudgetQuestion);
        };
        if let BudgetAnswer::Raised { by } = answer {
            if by == 0 {
                return Err(DomainError::BudgetOutOfRange {
                    min: TokenBudget::MIN,
                    max: TokenBudget::MAX,
                });
            }
            self.tokens = Some(question.limit.raised_by(by));
        }
        question.answer = Some((answer, now));
        Ok(())
    }
}

/// What the coordinator did, for its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoordinatorEntryKind {
    /// It started a specialist on a task.
    Started,
    /// It gave a specialist a turn to answer what teammates asked.
    Woke,
    /// It did not start work it could have started, and says why.
    Held,
    /// It asked the manager about a budget that is used up.
    Asked,
    /// The manager switched it on.
    SwitchedOn,
    /// The manager switched it off, or it switched itself off and says why.
    SwitchedOff,
}

text_enum!(CoordinatorEntryKind, kind = "kind of coordinator entry", {
    Started => "started",
    Woke => "woke",
    Held => "held",
    Asked => "asked",
    SwitchedOn => "switched_on",
    SwitchedOff => "switched_off",
});

/// One entry of the record of a coordinator: what it did and why, in one
/// sentence the manager can read back. It never changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoordinatorEntry {
    /// Its identifier.
    pub id: CoordinatorEntryId,
    /// The project.
    pub project_id: ProjectId,
    /// When it happened.
    pub at: Timestamp,
    /// What kind of thing it was.
    pub kind: CoordinatorEntryKind,
    /// What happened and why, with the names true at the moment.
    pub text: String,
    /// The task it is about, if one.
    pub task_id: Option<TaskId>,
    /// The member it is about, if one.
    pub member_id: Option<MemberId>,
    /// The computer the coordinator ran on.
    pub computer: ComputerId,
}

impl CoordinatorEntry {
    /// The most characters of the text of an entry.
    pub const MAX_TEXT_LENGTH: usize = 500;

    /// A new entry. The text is trimmed and cut to its length.
    #[must_use]
    pub fn new(
        project_id: ProjectId,
        computer: ComputerId,
        kind: CoordinatorEntryKind,
        text: &str,
        at: Timestamp,
    ) -> Self {
        Self {
            id: CoordinatorEntryId::generate(),
            project_id,
            at,
            kind,
            text: text.trim().chars().take(Self::MAX_TEXT_LENGTH).collect(),
            task_id: None,
            member_id: None,
            computer,
        }
    }

    /// The entry, about a task.
    #[must_use]
    pub const fn about_task(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// The entry, about a member.
    #[must_use]
    pub const fn about_member(mut self, member_id: MemberId) -> Self {
        self.member_id = Some(member_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn moment(minute: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 21, 22, minute, 0)
            .unwrap()
    }

    fn budget(tokens: u64) -> Budget {
        Budget::new(
            ProjectId::generate(),
            BudgetScope::Project,
            TokenBudget::new(tokens).unwrap(),
        )
    }

    #[test]
    fn a_budget_is_a_thousand_tokens_or_more() {
        assert!(TokenBudget::new(999).is_err());
        assert_eq!(TokenBudget::new(1_000).unwrap().tokens(), 1_000);
        assert!(TokenBudget::new(TokenBudget::MAX).is_ok());
        assert!(TokenBudget::new(TokenBudget::MAX + 1).is_err());
    }

    #[test]
    fn a_budget_is_used_up_when_what_was_used_reaches_it() {
        let tokens = TokenBudget::new(10_000).unwrap();
        assert!(!tokens.is_used_up_by(9_999));
        assert!(tokens.is_used_up_by(10_000));
        assert!(tokens.is_used_up_by(25_000));
    }

    #[test]
    fn the_raises_offered_are_half_and_all_of_the_budget_in_round_numbers() {
        assert_eq!(
            TokenBudget::new(2_000_000).unwrap().raises(),
            [1_000_000, 2_000_000]
        );
        assert_eq!(
            TokenBudget::new(1_234_567).unwrap().raises(),
            [610_000, 1_200_000]
        );
        assert_eq!(TokenBudget::new(1_500).unwrap().raises(), [1_000, 1_500]);
    }

    #[test]
    fn a_raise_never_passes_the_largest_budget() {
        let tokens = TokenBudget::new(TokenBudget::MAX).unwrap();
        assert_eq!(tokens.raised_by(10).tokens(), TokenBudget::MAX);
    }

    #[test]
    fn the_limit_of_specialists_at_once_is_one_to_eight_and_two_at_first() {
        assert!(ConcurrencyLimit::new(0).is_err());
        assert_eq!(ConcurrencyLimit::new(1).unwrap().value(), 1);
        assert_eq!(ConcurrencyLimit::new(8).unwrap().value(), 8);
        assert!(ConcurrencyLimit::new(9).is_err());
        assert!(ConcurrencyLimit::new(-1).is_err());
        assert_eq!(ConcurrencyLimit::default().value(), 2);
    }

    #[test]
    fn a_coordinator_is_off_until_it_is_switched_on_on_one_computer() {
        let mut coordinator = Coordinator::of(ProjectId::generate());
        let desk = ComputerId::generate();
        let laptop = ComputerId::generate();
        assert_eq!(coordinator.runs_on(), None);
        coordinator.switch_on(desk, moment(1));
        assert!(coordinator.runs_on_computer(desk));
        assert!(!coordinator.runs_on_computer(laptop));
        coordinator.switch_on(laptop, moment(2));
        assert!(coordinator.runs_on_computer(laptop));
        assert!(!coordinator.runs_on_computer(desk));
        coordinator.switch_off(moment(3));
        assert_eq!(coordinator.runs_on(), None);
        assert_eq!(coordinator.switched_at(), Some(moment(3)));
    }

    #[test]
    fn switching_on_where_it_runs_already_changes_nothing() {
        let mut coordinator = Coordinator::of(ProjectId::generate());
        let desk = ComputerId::generate();
        coordinator.switch_on(desk, moment(1));
        coordinator.switch_on(desk, moment(5));
        assert_eq!(coordinator.switched_at(), Some(moment(1)));
    }

    #[test]
    fn the_manager_is_asked_once_about_a_budget_that_is_used_up() {
        let mut budget = budget(10_000);
        assert!(!budget.needs_question(9_000));
        assert!(budget.needs_question(10_000));
        budget.ask(10_400, moment(1)).unwrap();
        assert!(!budget.needs_question(12_000));
        assert_eq!(budget.open_question().unwrap().used, 10_400);
    }

    #[test]
    fn leaving_the_work_stopped_asks_nothing_more_until_the_budget_changes() {
        let mut budget = budget(10_000);
        budget.ask(10_400, moment(1)).unwrap();
        budget.answer(BudgetAnswer::LeftStopped, moment(2)).unwrap();
        assert!(budget.open_question().is_none());
        assert!(budget.is_used_up_by(10_400));
        assert!(!budget.needs_question(20_000));
        budget.set(Some(TokenBudget::new(15_000).unwrap()));
        assert!(budget.needs_question(20_000));
    }

    #[test]
    fn raising_a_budget_answers_the_question_and_frees_the_work() {
        let mut budget = budget(10_000);
        budget.ask(10_400, moment(1)).unwrap();
        budget
            .answer(BudgetAnswer::Raised { by: 5_000 }, moment(2))
            .unwrap();
        assert_eq!(budget.tokens().unwrap().tokens(), 15_000);
        assert!(!budget.is_used_up_by(10_400));
        assert!(budget.open_question().is_none());
        assert!(matches!(
            budget.question().unwrap().answer,
            Some((BudgetAnswer::Raised { by: 5_000 }, _))
        ));
    }

    #[test]
    fn a_question_is_answered_once() {
        let mut budget = budget(10_000);
        assert_eq!(
            budget.answer(BudgetAnswer::LeftStopped, moment(1)),
            Err(DomainError::NoBudgetQuestion)
        );
        budget.ask(10_000, moment(1)).unwrap();
        budget.answer(BudgetAnswer::LeftStopped, moment(2)).unwrap();
        assert_eq!(
            budget.answer(BudgetAnswer::Raised { by: 1_000 }, moment(3)),
            Err(DomainError::NoBudgetQuestion)
        );
    }

    #[test]
    fn changing_the_budget_answers_an_open_question() {
        let mut budget = budget(10_000);
        budget.ask(10_000, moment(1)).unwrap();
        budget.set(Some(TokenBudget::new(30_000).unwrap()));
        assert!(budget.open_question().is_none());
        assert!(!budget.is_used_up_by(10_000));
    }

    #[test]
    fn a_budget_taken_away_stops_nothing_and_asks_nothing() {
        let mut budget = budget(10_000);
        budget.set(None);
        assert!(!budget.is_used_up_by(u64::MAX));
        assert!(!budget.needs_question(u64::MAX));
        assert_eq!(budget.ask(1, moment(1)), Err(DomainError::NoBudget));
    }

    #[test]
    fn a_raise_of_nothing_is_refused() {
        let mut budget = budget(10_000);
        budget.ask(10_000, moment(1)).unwrap();
        assert!(
            budget
                .answer(BudgetAnswer::Raised { by: 0 }, moment(2))
                .is_err()
        );
    }

    #[test]
    fn the_identifier_of_a_budget_is_that_of_what_it_is_for() {
        let project_id = ProjectId::generate();
        let task_id = TaskId::generate();
        assert_eq!(
            BudgetScope::Project.target(project_id),
            project_id.as_uuid()
        );
        assert_eq!(
            BudgetScope::Task(task_id).target(project_id),
            task_id.as_uuid()
        );
        assert_eq!(
            BudgetScope::from_parts(BudgetScopeKind::Task, task_id.as_uuid()),
            BudgetScope::Task(task_id)
        );
    }

    #[test]
    fn an_entry_is_cut_to_its_length() {
        let entry = CoordinatorEntry::new(
            ProjectId::generate(),
            ComputerId::generate(),
            CoordinatorEntryKind::Held,
            &"x".repeat(900),
            moment(1),
        );
        assert_eq!(
            entry.text.chars().count(),
            CoordinatorEntry::MAX_TEXT_LENGTH
        );
    }
}
