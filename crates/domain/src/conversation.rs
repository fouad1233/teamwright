//! Conversations with AI specialists and with the Advisor, and the turns
//! they are made of.
//!
//! A conversation is a thread of work with one specialist: about a task, or
//! about a question the manager asked. Or it is a thread with the Advisor,
//! in which a project is planned. It is one session of the program that
//! runs the specialist or the Advisor, so the earlier turns are remembered.
//!
//! A turn is one request to the specialist and what came of it: the steps it
//! took while it worked, its answer, and how many tokens it used.

use crate::error::DomainError;
use crate::ids::{ComputerId, ConversationId, MemberId, ProjectId, TaskId, TurnId};
use crate::macros::text_enum;
use crate::member::{Effort, MemberProfile};
use crate::planning::{Interview, InterviewAnswer, Proposal, ProposedMember, ProposedTask};
use crate::text::ModelName;
use crate::time::Timestamp;

/// The plain data of a conversation, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSnapshot {
    /// The identifier of the conversation.
    pub id: ConversationId,
    /// The project the conversation belongs to.
    pub project_id: ProjectId,
    /// The specialist, or nothing if it has left the team since, or if
    /// the conversation is with the Advisor.
    pub member_id: Option<MemberId>,
    /// The task the conversation is about, if it is about one.
    pub task_id: Option<TaskId>,
    /// When the conversation was started.
    pub started_at: Timestamp,
    /// The identifier of the agent program's session, once it is known.
    pub session_id: Option<String>,
    /// The model and the effort of the Advisor, if the conversation is
    /// with the Advisor; nothing for a conversation with a specialist.
    pub advisor: Option<MemberProfile>,
    /// The computer the conversation was started on, where its session of
    /// the agent program lives.
    pub home_computer: ComputerId,
}

/// A thread of work with one AI specialist, or with the Advisor.
///
/// The Advisor is not a member of the team, so a conversation with it has
/// no member and no task. It has the model and the effort the Advisor runs
/// on in it, chosen when it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    id: ConversationId,
    project_id: ProjectId,
    member_id: Option<MemberId>,
    task_id: Option<TaskId>,
    started_at: Timestamp,
    session_id: Option<String>,
    advisor: Option<MemberProfile>,
    home_computer: ComputerId,
}

impl Conversation {
    /// Starts a conversation with a specialist, about a task or not, on
    /// the computer `home_computer`.
    #[must_use]
    pub fn start(
        project_id: ProjectId,
        member_id: MemberId,
        task_id: Option<TaskId>,
        now: Timestamp,
        home_computer: ComputerId,
    ) -> Self {
        Self {
            id: ConversationId::generate(),
            project_id,
            member_id: Some(member_id),
            task_id,
            started_at: now,
            session_id: None,
            advisor: None,
            home_computer,
        }
    }

    /// Starts a conversation with the Advisor, which runs on this model and
    /// at this effort, or on the defaults of the program where they are
    /// absent. It is started on the computer `home_computer`.
    #[must_use]
    pub fn start_with_advisor(
        project_id: ProjectId,
        model: Option<ModelName>,
        effort: Option<Effort>,
        now: Timestamp,
        home_computer: ComputerId,
    ) -> Self {
        Self {
            id: ConversationId::generate(),
            project_id,
            member_id: None,
            task_id: None,
            started_at: now,
            session_id: None,
            advisor: Some(MemberProfile { model, effort }),
            home_computer,
        }
    }

    /// Recreates a conversation from stored data.
    #[must_use]
    pub fn restore(snapshot: ConversationSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            member_id: snapshot.member_id,
            task_id: snapshot.task_id,
            started_at: snapshot.started_at,
            session_id: snapshot.session_id,
            advisor: snapshot.advisor,
            home_computer: snapshot.home_computer,
        }
    }

    /// The identifier of the conversation.
    #[must_use]
    pub const fn id(&self) -> ConversationId {
        self.id
    }

    /// The project the conversation belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The specialist, or nothing if it has left the team since, or if the
    /// conversation is with the Advisor.
    #[must_use]
    pub const fn member_id(&self) -> Option<MemberId> {
        self.member_id
    }

    /// The task the conversation is about, if it is about one.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// When the conversation was started.
    #[must_use]
    pub const fn started_at(&self) -> Timestamp {
        self.started_at
    }

    /// The identifier of the agent program's session, once it is known.
    #[must_use]
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// The computer the conversation was started on. Its session of the
    /// agent program lives there, and it never moves.
    #[must_use]
    pub const fn home_computer(&self) -> ComputerId {
        self.home_computer
    }

    /// Whether the conversation was started on this computer. Only there can
    /// a turn of it be started, answered or stopped.
    #[must_use]
    pub fn is_at_home_on(&self, computer: ComputerId) -> bool {
        self.home_computer == computer
    }

    /// Whether the conversation is with the Advisor.
    #[must_use]
    pub const fn is_with_advisor(&self) -> bool {
        self.advisor.is_some()
    }

    /// The model and the effort the Advisor runs on in this conversation,
    /// if it is with the Advisor. Either may be absent, for the default of
    /// the program.
    #[must_use]
    pub const fn advisor_profile(&self) -> Option<&MemberProfile> {
        self.advisor.as_ref()
    }

    /// Records that the specialist has left the team. What it did stays on
    /// record, without it.
    pub fn forget_member(&mut self) {
        self.member_id = None;
    }

    /// Records the identifier of the agent program's session, so that later
    /// turns continue the same session.
    pub fn remember_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }
}

/// Who wrote the request of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TurnOrigin {
    /// The program wrote it from a task.
    Brief,
    /// The manager wrote it in their own words.
    Message,
    /// It is the manager's answer to a question the specialist asked.
    Decision,
    /// It is the manager's answers to the questions the Advisor asked.
    Interview,
}

text_enum!(TurnOrigin, kind = "origin of a turn", {
    Brief => "brief",
    Message => "message",
    Decision => "decision",
    Interview => "interview",
});

/// One of the answers a specialist offers the manager for a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionOption {
    /// The option in a few words. Never empty in a [`DecisionRequest`].
    pub label: String,
    /// What choosing it leads to. May be empty.
    pub consequence: String,
}

/// What the manager decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionAnswer {
    /// The option the manager chose, by its place among the options,
    /// counted from 0. Nothing if the manager answered in their own words.
    pub option: Option<usize>,
    /// What the manager added to the option, or the whole answer if no
    /// option was chosen. Empty if an option was chosen and nothing added.
    pub note: String,
    /// When the manager answered.
    pub at: Timestamp,
}

/// The plain data of a decision request, used to recreate one that was
/// stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSnapshot {
    /// The question.
    pub question: String,
    /// Why it matters.
    pub reason: String,
    /// The options.
    pub options: Vec<DecisionOption>,
    /// The recommended option, by its place from 0.
    pub recommended: Option<usize>,
    /// What the manager decided, if the manager has.
    pub answer: Option<DecisionAnswer>,
}

/// A question that is the manager's to answer. A specialist asks it
/// instead of guessing, and ends its turn. The answer starts the next turn
/// of the same conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRequest {
    question: String,
    reason: String,
    options: Vec<DecisionOption>,
    recommended: Option<usize>,
    answer: Option<DecisionAnswer>,
}

impl DecisionRequest {
    /// The fewest options a question has.
    pub const FEWEST_OPTIONS: usize = 2;
    /// The most options a question has.
    pub const MOST_OPTIONS: usize = 4;

    /// Makes a question that has not been answered. Whitespace around the
    /// texts is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the question is empty, if there are fewer than
    /// two or more than four options, if an option has no label, or if the
    /// recommended option is not one of them.
    pub fn new(
        question: &str,
        reason: &str,
        options: Vec<DecisionOption>,
        recommended: Option<usize>,
    ) -> Result<Self, DomainError> {
        let question = question.trim();
        if question.is_empty() {
            return Err(DomainError::EmptyText { field: "question" });
        }
        if !(Self::FEWEST_OPTIONS..=Self::MOST_OPTIONS).contains(&options.len()) {
            return Err(DomainError::OptionCount {
                count: options.len(),
            });
        }
        let options = options
            .into_iter()
            .map(|option| {
                let label = option.label.trim();
                if label.is_empty() {
                    return Err(DomainError::EmptyText {
                        field: "label of an option",
                    });
                }
                Ok(DecisionOption {
                    label: label.to_owned(),
                    consequence: option.consequence.trim().to_owned(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if recommended.is_some_and(|place| place >= options.len()) {
            return Err(DomainError::NoSuchOption);
        }
        Ok(Self {
            question: question.to_owned(),
            reason: reason.trim().to_owned(),
            options,
            recommended,
            answer: None,
        })
    }

    /// Recreates a question from stored data.
    #[must_use]
    pub fn restore(snapshot: DecisionSnapshot) -> Self {
        Self {
            question: snapshot.question,
            reason: snapshot.reason,
            options: snapshot.options,
            recommended: snapshot.recommended,
            answer: snapshot.answer,
        }
    }

    /// The question.
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Why it matters. May be empty.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The options, two to four.
    #[must_use]
    pub fn options(&self) -> &[DecisionOption] {
        &self.options
    }

    /// The option the specialist recommends, by its place from 0, if it
    /// recommends one.
    #[must_use]
    pub const fn recommended(&self) -> Option<usize> {
        self.recommended
    }

    /// What the manager decided. Nothing while the question is open.
    #[must_use]
    pub const fn answer(&self) -> Option<&DecisionAnswer> {
        self.answer.as_ref()
    }

    /// Whether the manager has not answered yet.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.answer.is_none()
    }
}

/// How a turn stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TurnState {
    /// The specialist is working on it.
    Running,
    /// The specialist finished it and answered.
    Finished,
    /// It ended without an answer, for the reason in its failure text.
    Failed,
    /// The manager stopped it.
    Stopped,
}

text_enum!(TurnState, kind = "state of a turn", {
    Running => "running",
    Finished => "finished",
    Failed => "failed",
    Stopped => "stopped",
});

/// How a use of a tool came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolOutcome {
    /// The tool has not finished yet.
    Running,
    /// The tool finished.
    Done,
    /// The tool failed or was refused.
    Failed,
}

text_enum!(ToolOutcome, kind = "outcome of a tool use", {
    Running => "running",
    Done => "done",
    Failed => "failed",
});

/// The kinds of steps, by their stable names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepKind {
    /// The specialist wrote something.
    Text,
    /// The specialist used a tool.
    Tool,
    /// A tool use was refused.
    Denied,
    /// The program said something about the run itself.
    Notice,
}

text_enum!(StepKind, kind = "kind of step", {
    Text => "text",
    Tool => "tool",
    Denied => "denied",
    Notice => "notice",
});

/// What happened in one step of a turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepContent {
    /// The specialist wrote something.
    Text {
        /// What it wrote.
        text: String,
    },
    /// The specialist used a tool, for example to read or to write a file.
    Tool {
        /// The identifier of this use. It stays the same while the outcome
        /// changes.
        use_id: String,
        /// The name of the tool, such as `Read`.
        tool: String,
        /// What the tool was used on, short: usually a file. May be empty.
        summary: String,
        /// How the use came out, so far.
        outcome: ToolOutcome,
    },
    /// A tool was refused, because the specialist may not use it that way.
    Denied {
        /// The name of the tool.
        tool: String,
    },
    /// The program said something about the run itself: a retry, a warning.
    Notice {
        /// What it said.
        message: String,
    },
}

impl StepContent {
    /// The kind of this step.
    #[must_use]
    pub const fn kind(&self) -> StepKind {
        match self {
            Self::Text { .. } => StepKind::Text,
            Self::Tool { .. } => StepKind::Tool,
            Self::Denied { .. } => StepKind::Denied,
            Self::Notice { .. } => StepKind::Notice,
        }
    }
}

/// One thing that happened while a specialist worked on a request.
///
/// A step is plain data. A turn hands its steps out only to be read, so a
/// step can only change through the turn it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// When it happened.
    pub at: Timestamp,
    /// What happened.
    pub content: StepContent,
}

/// Counts of tokens, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TokenCounts {
    /// Tokens the model read that were not in its cache.
    pub input: u64,
    /// Tokens the model wrote, thinking included.
    pub output: u64,
    /// Tokens read from the cache.
    pub cache_read: u64,
    /// Tokens written to the cache.
    pub cache_written: u64,
}

/// The plain data of a turn, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSnapshot {
    /// The identifier of the turn.
    pub id: TurnId,
    /// The conversation the turn belongs to.
    pub conversation_id: ConversationId,
    /// What the specialist was asked to do.
    pub request: String,
    /// Who wrote the request.
    pub origin: TurnOrigin,
    /// How the turn stands.
    pub state: TurnState,
    /// When the turn started.
    pub started_at: Timestamp,
    /// When the turn ended, if it has.
    pub ended_at: Option<Timestamp>,
    /// What happened, in order.
    pub steps: Vec<Step>,
    /// The specialist's answer. Empty until it has one.
    pub answer: String,
    /// Why the turn failed or was stopped, in words.
    pub failure: Option<String>,
    /// The model the turn ran on, once that is known.
    pub model: Option<String>,
    /// The tokens the turn used, once it has ended.
    pub tokens: Option<TokenCounts>,
    /// What the specialist asked the manager to decide in this turn, if
    /// anything.
    pub decision: Option<DecisionRequest>,
    /// What the Advisor asked the manager in this turn, if anything.
    pub interview: Option<Interview>,
    /// What the Advisor proposed in this turn, if anything.
    pub proposal: Option<Proposal>,
}

/// One request to a specialist or to the Advisor, and what came of it.
///
/// A turn runs until it ends, and it ends exactly once: finished, failed or
/// stopped. Steps are only added while it runs. It carries at most one of a
/// question to decide, an interview and a proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    id: TurnId,
    conversation_id: ConversationId,
    request: String,
    origin: TurnOrigin,
    state: TurnState,
    started_at: Timestamp,
    ended_at: Option<Timestamp>,
    steps: Vec<Step>,
    answer: String,
    failure: Option<String>,
    model: Option<String>,
    tokens: Option<TokenCounts>,
    decision: Option<DecisionRequest>,
    interview: Option<Interview>,
    proposal: Option<Proposal>,
}

impl Turn {
    /// The failure text of a turn the manager stopped.
    pub const STOPPED_BY_THE_MANAGER: &'static str = "Stopped by the manager.";

    /// Starts a turn with a request. Whitespace around the request is
    /// removed.
    ///
    /// # Errors
    ///
    /// Returns an error if nothing is left of the request.
    pub fn start(
        conversation_id: ConversationId,
        request: &str,
        origin: TurnOrigin,
        now: Timestamp,
    ) -> Result<Self, DomainError> {
        let request = request.trim();
        if request.is_empty() {
            return Err(DomainError::EmptyText { field: "message" });
        }
        Ok(Self {
            id: TurnId::generate(),
            conversation_id,
            request: request.to_owned(),
            origin,
            state: TurnState::Running,
            started_at: now,
            ended_at: None,
            steps: Vec::new(),
            answer: String::new(),
            failure: None,
            model: None,
            tokens: None,
            decision: None,
            interview: None,
            proposal: None,
        })
    }

    /// Recreates a turn from stored data.
    #[must_use]
    pub fn restore(snapshot: TurnSnapshot) -> Self {
        Self {
            id: snapshot.id,
            conversation_id: snapshot.conversation_id,
            request: snapshot.request,
            origin: snapshot.origin,
            state: snapshot.state,
            started_at: snapshot.started_at,
            ended_at: snapshot.ended_at,
            steps: snapshot.steps,
            answer: snapshot.answer,
            failure: snapshot.failure,
            model: snapshot.model,
            tokens: snapshot.tokens,
            decision: snapshot.decision,
            interview: snapshot.interview,
            proposal: snapshot.proposal,
        }
    }

    /// The identifier of the turn.
    #[must_use]
    pub const fn id(&self) -> TurnId {
        self.id
    }

    /// The conversation the turn belongs to.
    #[must_use]
    pub const fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    /// What the specialist was asked to do.
    #[must_use]
    pub fn request(&self) -> &str {
        &self.request
    }

    /// Who wrote the request.
    #[must_use]
    pub const fn origin(&self) -> TurnOrigin {
        self.origin
    }

    /// How the turn stands.
    #[must_use]
    pub const fn state(&self) -> TurnState {
        self.state
    }

    /// Whether the specialist is still working on the turn.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.state == TurnState::Running
    }

    /// When the turn started.
    #[must_use]
    pub const fn started_at(&self) -> Timestamp {
        self.started_at
    }

    /// When the turn ended, if it has.
    #[must_use]
    pub const fn ended_at(&self) -> Option<Timestamp> {
        self.ended_at
    }

    /// What happened, in order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The specialist's answer: its report. Empty until it has one.
    #[must_use]
    pub fn answer(&self) -> &str {
        &self.answer
    }

    /// Why the turn failed or was stopped, in words.
    #[must_use]
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// The model the turn ran on, once that is known.
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// The tokens the turn used, once it has ended.
    #[must_use]
    pub const fn tokens(&self) -> Option<TokenCounts> {
        self.tokens
    }

    /// What the specialist asked the manager to decide in this turn, if
    /// anything.
    #[must_use]
    pub const fn decision(&self) -> Option<&DecisionRequest> {
        self.decision.as_ref()
    }

    /// Records that the specialist asked the manager to decide something.
    /// A turn asks at most once. The question counts as asked when the turn
    /// ends, because only then does the specialist wait for the answer.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended or has asked already.
    pub fn ask(&mut self, request: DecisionRequest) -> Result<(), DomainError> {
        self.require_nothing_asked()?;
        self.decision = Some(request);
        Ok(())
    }

    /// What the Advisor asked the manager in this turn, if anything.
    #[must_use]
    pub const fn interview(&self) -> Option<&Interview> {
        self.interview.as_ref()
    }

    /// What the Advisor proposed in this turn, if anything.
    #[must_use]
    pub const fn proposal(&self) -> Option<&Proposal> {
        self.proposal.as_ref()
    }

    /// Records that the Advisor asked the manager the questions of an
    /// interview. Like a question to decide, it counts as asked when the
    /// turn ends.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended, or if it has asked or
    /// proposed already.
    pub fn ask_interview(&mut self, interview: Interview) -> Result<(), DomainError> {
        self.require_nothing_asked()?;
        self.interview = Some(interview);
        Ok(())
    }

    /// Records that the Advisor proposed a team and a plan.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended, or if it has asked or
    /// proposed already.
    pub fn propose(&mut self, proposal: Proposal) -> Result<(), DomainError> {
        self.require_nothing_asked()?;
        self.proposal = Some(proposal);
        Ok(())
    }

    /// Records the manager's answers to the interview of the turn, one for
    /// each question: an option, words, or both. Whitespace around the
    /// words is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn is still running, if it asked no
    /// interview, if the interview has been answered already, if there is
    /// not one answer for each question, or if an answer names an option
    /// its question does not have, or no option and no words.
    pub fn answer_interview(
        &mut self,
        answers: Vec<InterviewAnswer>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if self.is_running() {
            return Err(DomainError::TurnStillRunning);
        }
        self.interview
            .as_mut()
            .ok_or(DomainError::NothingAsked)?
            .answer(answers, now)
    }

    /// Records that the manager approved the proposal of the turn, with
    /// the members and the tasks the manager left in it. The proposal
    /// keeps them in place of what was proposed.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn is still running, if it proposed
    /// nothing, if the proposal has been approved already, or if what is
    /// approved breaks a rule of a plan.
    pub fn approve_proposal(
        &mut self,
        members: Vec<ProposedMember>,
        tasks: Vec<ProposedTask>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if self.is_running() {
            return Err(DomainError::TurnStillRunning);
        }
        self.proposal
            .as_mut()
            .ok_or(DomainError::NothingProposed)?
            .approve(members, tasks, now)
    }

    /// Records the manager's answer to the question of the turn: one of its
    /// options, words of the manager's own, or both. Whitespace around the
    /// words is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn is still running, if it asked nothing,
    /// if its question has been answered already, if the option is not one
    /// of the question's, or if the answer names no option and says
    /// nothing.
    pub fn answer_decision(
        &mut self,
        option: Option<usize>,
        note: &str,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if self.is_running() {
            return Err(DomainError::TurnStillRunning);
        }
        let decision = self.decision.as_mut().ok_or(DomainError::NothingAsked)?;
        if decision.answer.is_some() {
            return Err(DomainError::AlreadyAnswered);
        }
        if option.is_some_and(|place| place >= decision.options.len()) {
            return Err(DomainError::NoSuchOption);
        }
        let note = note.trim();
        if option.is_none() && note.is_empty() {
            return Err(DomainError::EmptyAnswer);
        }
        decision.answer = Some(DecisionAnswer {
            option,
            note: note.to_owned(),
            at: now,
        });
        Ok(())
    }

    /// Records the model the turn runs on.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended.
    pub fn record_model(&mut self, model: String) -> Result<(), DomainError> {
        self.require_running()?;
        self.model = Some(model);
        Ok(())
    }

    /// Records text the specialist wrote. Text that is only whitespace says
    /// nothing and is left out.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended.
    pub fn say(&mut self, text: String, at: Timestamp) -> Result<(), DomainError> {
        self.require_running()?;
        if !text.trim().is_empty() {
            self.steps.push(Step {
                at,
                content: StepContent::Text { text },
            });
        }
        Ok(())
    }

    /// Records that the specialist started to use a tool.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended.
    pub fn use_tool(
        &mut self,
        use_id: String,
        tool: String,
        summary: String,
        at: Timestamp,
    ) -> Result<(), DomainError> {
        self.require_running()?;
        self.steps.push(Step {
            at,
            content: StepContent::Tool {
                use_id,
                tool,
                summary,
                outcome: ToolOutcome::Running,
            },
        });
        Ok(())
    }

    /// Records that a tool use has finished, or failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended, or if it has no tool use with
    /// this identifier.
    pub fn finish_tool(&mut self, use_id: &str, failed: bool) -> Result<(), DomainError> {
        self.require_running()?;
        let outcome = self
            .steps
            .iter_mut()
            .find_map(|step| match &mut step.content {
                StepContent::Tool {
                    use_id: id,
                    outcome,
                    ..
                } if id == use_id => Some(outcome),
                _ => None,
            })
            .ok_or_else(|| DomainError::UnknownToolUse {
                id: use_id.to_owned(),
            })?;
        *outcome = if failed {
            ToolOutcome::Failed
        } else {
            ToolOutcome::Done
        };
        Ok(())
    }

    /// Records that a tool use was refused.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended.
    pub fn record_denial(&mut self, tool: String, at: Timestamp) -> Result<(), DomainError> {
        self.require_running()?;
        self.steps.push(Step {
            at,
            content: StepContent::Denied { tool },
        });
        Ok(())
    }

    /// Records something the program said about the run itself.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has ended.
    pub fn notice(&mut self, message: String, at: Timestamp) -> Result<(), DomainError> {
        self.require_running()?;
        self.steps.push(Step {
            at,
            content: StepContent::Notice { message },
        });
        Ok(())
    }

    /// Ends the turn with the specialist's answer.
    ///
    /// The answer is the specialist's last text. If the last step is that
    /// same text, the step is removed: it is the report, and would otherwise
    /// be shown twice.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has already ended.
    pub fn finish(
        &mut self,
        answer: &str,
        tokens: TokenCounts,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        self.require_running()?;
        let answer = answer.trim();
        let last_step_is_the_answer = matches!(
            self.steps.last(),
            Some(Step { content: StepContent::Text { text }, .. }) if text.trim() == answer
        );
        if last_step_is_the_answer {
            self.steps.pop();
        }
        self.answer = answer.to_owned();
        self.end(TurnState::Finished, None, Some(tokens), now);
        Ok(())
    }

    /// Ends the turn without an answer, for a reason given in words. The
    /// tokens are those it used before it failed, if they are known.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has already ended.
    pub fn fail(
        &mut self,
        failure: String,
        tokens: Option<TokenCounts>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        self.require_running()?;
        self.end(TurnState::Failed, Some(failure), tokens, now);
        Ok(())
    }

    /// Ends the turn because the manager stopped it. The tokens are those
    /// it used before it was stopped, if they are known.
    ///
    /// # Errors
    ///
    /// Returns an error if the turn has already ended.
    pub fn stop(&mut self, tokens: Option<TokenCounts>, now: Timestamp) -> Result<(), DomainError> {
        self.require_running()?;
        self.end(
            TurnState::Stopped,
            Some(Self::STOPPED_BY_THE_MANAGER.to_owned()),
            tokens,
            now,
        );
        Ok(())
    }

    /// Puts the turn into its final state.
    fn end(
        &mut self,
        state: TurnState,
        failure: Option<String>,
        tokens: Option<TokenCounts>,
        now: Timestamp,
    ) {
        self.state = state;
        self.failure = failure;
        self.tokens = tokens;
        self.ended_at = Some(now);
    }

    /// Refuses to ask or to propose in a turn that has ended, or that has
    /// asked or proposed already.
    fn require_nothing_asked(&self) -> Result<(), DomainError> {
        self.require_running()?;
        if self.decision.is_some() || self.interview.is_some() || self.proposal.is_some() {
            return Err(DomainError::AlreadyAsked);
        }
        Ok(())
    }

    /// Refuses a change to a turn that has ended.
    fn require_running(&self) -> Result<(), DomainError> {
        if self.is_running() {
            Ok(())
        } else {
            Err(DomainError::TurnEnded)
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::planning::InterviewQuestion;

    fn moment(minute: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 12, 9, minute, 0)
            .unwrap()
    }

    fn tokens() -> TokenCounts {
        TokenCounts {
            input: 42,
            output: 880,
            cache_read: 21_500,
            cache_written: 6400,
        }
    }

    fn running_turn() -> Turn {
        Turn::start(
            ConversationId::generate(),
            "Draw the schematic",
            TurnOrigin::Brief,
            moment(0),
        )
        .unwrap()
    }

    #[test]
    fn a_conversation_starts_with_its_specialist_and_without_a_session() {
        let project_id = ProjectId::generate();
        let member_id = MemberId::generate();
        let task_id = TaskId::generate();

        let conversation = Conversation::start(
            project_id,
            member_id,
            Some(task_id),
            moment(0),
            ComputerId::generate(),
        );

        assert_eq!(conversation.project_id(), project_id);
        assert_eq!(conversation.member_id(), Some(member_id));
        assert_eq!(conversation.task_id(), Some(task_id));
        assert_eq!(conversation.started_at(), moment(0));
        assert_eq!(conversation.session_id(), None);
    }

    #[test]
    fn a_conversation_remembers_its_session_and_can_lose_its_specialist() {
        let mut conversation = Conversation::start(
            ProjectId::generate(),
            MemberId::generate(),
            None,
            moment(0),
            ComputerId::generate(),
        );

        conversation.remember_session("0196f3a2".to_owned());
        conversation.forget_member();

        assert_eq!(conversation.session_id(), Some("0196f3a2"));
        assert_eq!(conversation.member_id(), None);
    }

    #[test]
    fn a_stored_conversation_is_recreated_as_it_was() {
        let snapshot = ConversationSnapshot {
            id: ConversationId::generate(),
            project_id: ProjectId::generate(),
            member_id: None,
            task_id: Some(TaskId::generate()),
            started_at: moment(3),
            session_id: Some("abc".to_owned()),
            advisor: None,
            home_computer: ComputerId::generate(),
        };

        let conversation = Conversation::restore(snapshot.clone());

        assert_eq!(conversation.id(), snapshot.id);
        assert_eq!(conversation.member_id(), None);
        assert_eq!(conversation.task_id(), snapshot.task_id);
        assert_eq!(conversation.session_id(), Some("abc"));
        assert_eq!(conversation.home_computer(), snapshot.home_computer);
    }

    #[test]
    fn a_conversation_is_at_home_on_the_computer_it_was_started_on() {
        let desk = ComputerId::generate();
        let laptop = ComputerId::generate();

        let conversation = Conversation::start(
            ProjectId::generate(),
            MemberId::generate(),
            None,
            moment(0),
            desk,
        );

        assert!(conversation.is_at_home_on(desk));
        assert!(!conversation.is_at_home_on(laptop));
    }

    #[test]
    fn a_turn_starts_running_with_its_request_tidied() {
        let turn = Turn::start(
            ConversationId::generate(),
            "  Use the smaller regulator. \n",
            TurnOrigin::Message,
            moment(0),
        )
        .unwrap();

        assert_eq!(turn.request(), "Use the smaller regulator.");
        assert_eq!(turn.origin(), TurnOrigin::Message);
        assert_eq!(turn.state(), TurnState::Running);
        assert!(turn.steps().is_empty());
        assert_eq!(turn.answer(), "");
        assert_eq!(turn.ended_at(), None);
        assert_eq!(turn.tokens(), None);
    }

    #[test]
    fn a_turn_without_a_request_is_refused() {
        let error = Turn::start(
            ConversationId::generate(),
            "  \n ",
            TurnOrigin::Message,
            moment(0),
        )
        .unwrap_err();

        assert_eq!(error, DomainError::EmptyText { field: "message" });
        assert_eq!(error.to_string(), "the message must not be empty");
    }

    #[test]
    fn steps_are_recorded_in_order_and_empty_text_is_left_out() {
        let mut turn = running_turn();

        turn.notice("Starting.".to_owned(), moment(1)).unwrap();
        turn.say("I will read the notes.".to_owned(), moment(2))
            .unwrap();
        turn.say("   ".to_owned(), moment(2)).unwrap();
        turn.use_tool(
            "read".to_owned(),
            "Read".to_owned(),
            "notes.md".to_owned(),
            moment(3),
        )
        .unwrap();
        turn.record_denial("Bash".to_owned(), moment(4)).unwrap();

        let kinds: Vec<StepKind> = turn
            .steps()
            .iter()
            .map(|step| step.content.kind())
            .collect();
        assert_eq!(
            kinds,
            [
                StepKind::Notice,
                StepKind::Text,
                StepKind::Tool,
                StepKind::Denied
            ]
        );
        assert_eq!(turn.steps()[2].at, moment(3));
    }

    #[test]
    fn a_tool_runs_until_it_is_finished() {
        let mut turn = running_turn();
        turn.use_tool(
            "read".to_owned(),
            "Read".to_owned(),
            "notes.md".to_owned(),
            moment(1),
        )
        .unwrap();
        turn.use_tool(
            "write".to_owned(),
            "Write".to_owned(),
            "schematic.md".to_owned(),
            moment(2),
        )
        .unwrap();

        turn.finish_tool("read", false).unwrap();
        turn.finish_tool("write", true).unwrap();

        let outcomes: Vec<ToolOutcome> = turn
            .steps()
            .iter()
            .filter_map(|step| match &step.content {
                StepContent::Tool { outcome, .. } => Some(*outcome),
                _ => None,
            })
            .collect();
        assert_eq!(outcomes, [ToolOutcome::Done, ToolOutcome::Failed]);
    }

    #[test]
    fn finishing_a_tool_use_the_turn_does_not_have_is_refused() {
        let mut turn = running_turn();

        let error = turn.finish_tool("missing", false).unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownToolUse {
                id: "missing".to_owned()
            }
        );
    }

    #[test]
    fn a_finished_turn_keeps_its_answer_once() {
        let mut turn = running_turn();
        turn.say("Reading first.".to_owned(), moment(1)).unwrap();
        turn.say("Done: Draw the schematic.\n".to_owned(), moment(2))
            .unwrap();

        turn.finish("Done: Draw the schematic.", tokens(), moment(5))
            .unwrap();

        assert_eq!(turn.state(), TurnState::Finished);
        assert_eq!(turn.answer(), "Done: Draw the schematic.");
        assert_eq!(turn.steps().len(), 1);
        assert_eq!(turn.tokens(), Some(tokens()));
        assert_eq!(turn.ended_at(), Some(moment(5)));
        assert_eq!(turn.failure(), None);
    }

    #[test]
    fn a_last_text_that_is_not_the_answer_stays() {
        let mut turn = running_turn();
        turn.say("Something else.".to_owned(), moment(1)).unwrap();

        turn.finish("The report.", tokens(), moment(5)).unwrap();

        assert_eq!(turn.steps().len(), 1);
    }

    #[test]
    fn a_stopped_turn_says_who_stopped_it() {
        let mut turn = running_turn();

        turn.stop(None, moment(2)).unwrap();

        assert_eq!(turn.state(), TurnState::Stopped);
        assert_eq!(turn.failure(), Some("Stopped by the manager."));
        assert_eq!(turn.answer(), "");
        assert_eq!(turn.ended_at(), Some(moment(2)));
    }

    #[test]
    fn a_failed_turn_says_why() {
        let mut turn = running_turn();

        turn.fail("The program ended.".to_owned(), Some(tokens()), moment(2))
            .unwrap();

        assert_eq!(turn.state(), TurnState::Failed);
        assert_eq!(turn.failure(), Some("The program ended."));
        assert_eq!(turn.tokens(), Some(tokens()));
    }

    #[test]
    fn a_turn_ends_exactly_once_and_takes_no_steps_afterwards() {
        let mut turn = running_turn();
        turn.use_tool(
            "read".to_owned(),
            "Read".to_owned(),
            String::new(),
            moment(1),
        )
        .unwrap();
        turn.stop(None, moment(2)).unwrap();

        assert_eq!(
            turn.finish("Late.", tokens(), moment(3)),
            Err(DomainError::TurnEnded)
        );
        assert_eq!(
            turn.fail("Late.".to_owned(), None, moment(3)),
            Err(DomainError::TurnEnded)
        );
        assert_eq!(turn.stop(None, moment(3)), Err(DomainError::TurnEnded));
        assert_eq!(
            turn.say("Late.".to_owned(), moment(3)),
            Err(DomainError::TurnEnded)
        );
        assert_eq!(turn.finish_tool("read", false), Err(DomainError::TurnEnded));
        assert_eq!(
            turn.record_model("a model".to_owned()),
            Err(DomainError::TurnEnded)
        );
        assert_eq!(turn.state(), TurnState::Stopped);
        assert_eq!(turn.ended_at(), Some(moment(2)));
    }

    #[test]
    fn a_stored_turn_is_recreated_as_it_was() {
        let mut turn = running_turn();
        turn.record_model("claude-model".to_owned()).unwrap();
        turn.say("Hello.".to_owned(), moment(1)).unwrap();
        turn.finish("Report.", tokens(), moment(2)).unwrap();
        let snapshot = TurnSnapshot {
            id: turn.id(),
            conversation_id: turn.conversation_id(),
            request: turn.request().to_owned(),
            origin: turn.origin(),
            state: turn.state(),
            started_at: turn.started_at(),
            ended_at: turn.ended_at(),
            steps: turn.steps().to_vec(),
            answer: turn.answer().to_owned(),
            failure: None,
            model: Some("claude-model".to_owned()),
            tokens: turn.tokens(),
            decision: None,
            interview: None,
            proposal: None,
        };

        assert_eq!(Turn::restore(snapshot), turn);
    }

    fn option(label: &str, consequence: &str) -> DecisionOption {
        DecisionOption {
            label: label.to_owned(),
            consequence: consequence.to_owned(),
        }
    }

    fn battery_question() -> DecisionRequest {
        DecisionRequest::new(
            "Which battery?",
            "It sets the size of the board.",
            vec![
                option("The proven way", "A larger board."),
                option("The compact way", "A new part to test."),
            ],
            Some(0),
        )
        .unwrap()
    }

    /// A turn that asked the battery question and has ended.
    fn turn_that_asked() -> Turn {
        let mut turn = running_turn();
        turn.ask(battery_question()).unwrap();
        turn.finish("I wait for your decision.", tokens(), moment(5))
            .unwrap();
        turn
    }

    #[test]
    fn a_question_is_tidied_and_starts_without_an_answer() {
        let question = DecisionRequest::new(
            "  Which battery? ",
            " It sets the size. ",
            vec![option(" Proven ", " Larger. "), option("Compact", "")],
            None,
        )
        .unwrap();

        assert_eq!(question.question(), "Which battery?");
        assert_eq!(question.reason(), "It sets the size.");
        assert_eq!(question.options()[0], option("Proven", "Larger."));
        assert_eq!(question.recommended(), None);
        assert_eq!(question.answer(), None);
        assert!(question.is_open());
    }

    #[test]
    fn a_question_that_cannot_be_answered_is_refused() {
        let two = || vec![option("A", ""), option("B", "")];

        assert_eq!(
            DecisionRequest::new(" ", "", two(), None),
            Err(DomainError::EmptyText { field: "question" })
        );
        assert_eq!(
            DecisionRequest::new("Which?", "", vec![option("A", "")], None),
            Err(DomainError::OptionCount { count: 1 })
        );
        assert_eq!(
            DecisionRequest::new("Which?", "", vec![option("A", ""); 5], None)
                .unwrap_err()
                .to_string(),
            "a question has two to four options, but this one has 5"
        );
        assert_eq!(
            DecisionRequest::new("Which?", "", vec![option("A", ""), option(" ", "")], None)
                .unwrap_err()
                .to_string(),
            "the label of an option must not be empty"
        );
        assert_eq!(
            DecisionRequest::new("Which?", "", two(), Some(2)),
            Err(DomainError::NoSuchOption)
        );
    }

    #[test]
    fn a_turn_asks_only_while_it_runs_and_only_once() {
        let mut turn = running_turn();

        turn.ask(battery_question()).unwrap();

        assert_eq!(turn.decision(), Some(&battery_question()));
        assert_eq!(turn.ask(battery_question()), Err(DomainError::AlreadyAsked));
        let mut ended = running_turn();
        ended.stop(None, moment(2)).unwrap();
        assert_eq!(ended.ask(battery_question()), Err(DomainError::TurnEnded));
    }

    #[test]
    fn an_answer_takes_an_option_words_or_both() {
        let mut by_option = turn_that_asked();
        by_option.answer_decision(Some(1), "", moment(7)).unwrap();
        let mut in_words = turn_that_asked();
        in_words
            .answer_decision(None, "  Neither. Use the gray one.  ", moment(7))
            .unwrap();

        assert_eq!(
            by_option.decision().and_then(DecisionRequest::answer),
            Some(&DecisionAnswer {
                option: Some(1),
                note: String::new(),
                at: moment(7),
            })
        );
        let answer = in_words.decision().and_then(DecisionRequest::answer);
        assert_eq!(answer.map(|answer| answer.option), Some(None));
        assert_eq!(
            answer.map(|answer| answer.note.as_str()),
            Some("Neither. Use the gray one.")
        );
        assert!(!in_words.decision().unwrap().is_open());
    }

    #[test]
    fn an_answer_must_fit_the_question_and_comes_once_after_the_turn() {
        let mut running = running_turn();
        running.ask(battery_question()).unwrap();
        assert_eq!(
            running.answer_decision(Some(0), "", moment(3)),
            Err(DomainError::TurnStillRunning)
        );

        let mut nothing_asked = running_turn();
        nothing_asked
            .finish("Report.", tokens(), moment(3))
            .unwrap();
        assert_eq!(
            nothing_asked.answer_decision(Some(0), "", moment(4)),
            Err(DomainError::NothingAsked)
        );

        let mut turn = turn_that_asked();
        let no_such_option = turn.answer_decision(Some(2), "", moment(6)).unwrap_err();
        assert_eq!(
            no_such_option.to_string(),
            "the question has no such option"
        );
        let empty = turn.answer_decision(None, "   ", moment(6)).unwrap_err();
        assert_eq!(
            empty.to_string(),
            "an answer names one of the options or says something in words"
        );
        assert!(turn.decision().unwrap().is_open());

        turn.answer_decision(Some(0), "", moment(6)).unwrap();
        assert_eq!(
            turn.answer_decision(Some(1), "", moment(7)),
            Err(DomainError::AlreadyAnswered)
        );
    }

    #[test]
    fn a_stored_question_is_recreated_as_it_was() {
        let mut turn = turn_that_asked();
        turn.answer_decision(Some(0), "Keep the connector.", moment(8))
            .unwrap();
        let decision = turn.decision().unwrap().clone();

        let restored = DecisionRequest::restore(DecisionSnapshot {
            question: decision.question().to_owned(),
            reason: decision.reason().to_owned(),
            options: decision.options().to_vec(),
            recommended: decision.recommended(),
            answer: decision.answer().cloned(),
        });

        assert_eq!(restored, decision);
    }

    #[test]
    fn a_conversation_with_the_advisor_has_no_member_and_no_task_but_a_profile() {
        let project_id = ProjectId::generate();
        let model = ModelName::parse("opus").unwrap();

        let conversation = Conversation::start_with_advisor(
            project_id,
            Some(model.clone()),
            None,
            moment(0),
            ComputerId::generate(),
        );

        assert!(conversation.is_with_advisor());
        assert_eq!(conversation.member_id(), None);
        assert_eq!(conversation.task_id(), None);
        assert_eq!(
            conversation.advisor_profile(),
            Some(&MemberProfile {
                model: Some(model),
                effort: None
            })
        );
        let with_specialist = Conversation::start(
            project_id,
            MemberId::generate(),
            None,
            moment(0),
            ComputerId::generate(),
        );
        assert!(!with_specialist.is_with_advisor());
        assert_eq!(with_specialist.advisor_profile(), None);
    }

    fn interview() -> Interview {
        Interview::new(vec![InterviewQuestion {
            question: "What is fixed?".to_owned(),
            reason: String::new(),
            options: Vec::new(),
            recommended: None,
        }])
        .unwrap()
    }

    fn proposal() -> Proposal {
        Proposal::new(
            "A device.",
            vec![ProposedMember {
                name: crate::text::MemberName::parse("Hardware engineer").unwrap(),
                responsibility: String::new(),
                reason: String::new(),
                model: None,
                effort: None,
            }],
            Vec::new(),
        )
        .unwrap()
    }

    #[test]
    fn a_turn_carries_at_most_one_of_a_question_an_interview_and_a_proposal() {
        let mut interviewing = running_turn();
        interviewing.ask_interview(interview()).unwrap();
        assert_eq!(interviewing.interview(), Some(&interview()));
        assert_eq!(
            interviewing.propose(proposal()),
            Err(DomainError::AlreadyAsked)
        );
        assert_eq!(
            interviewing.ask(battery_question()),
            Err(DomainError::AlreadyAsked)
        );

        let mut proposing = running_turn();
        proposing.propose(proposal()).unwrap();
        assert_eq!(proposing.proposal(), Some(&proposal()));
        assert_eq!(
            proposing.ask_interview(interview()),
            Err(DomainError::AlreadyAsked)
        );

        let mut asking = running_turn();
        asking.ask(battery_question()).unwrap();
        assert_eq!(asking.propose(proposal()), Err(DomainError::AlreadyAsked));

        let mut ended = running_turn();
        ended.stop(None, moment(2)).unwrap();
        assert_eq!(
            ended.ask_interview(interview()),
            Err(DomainError::TurnEnded)
        );
    }

    #[test]
    fn an_interview_is_answered_after_its_turn_has_ended() {
        let mut turn = running_turn();
        turn.ask_interview(interview()).unwrap();
        let answers = || {
            vec![InterviewAnswer {
                option: None,
                note: " Nothing. ".to_owned(),
            }]
        };
        assert_eq!(
            turn.answer_interview(answers(), moment(3)),
            Err(DomainError::TurnStillRunning)
        );
        turn.finish("I wait for your answers.", tokens(), moment(4))
            .unwrap();

        turn.answer_interview(answers(), moment(5)).unwrap();

        let interview = turn.interview().unwrap();
        assert_eq!(interview.answers().unwrap()[0].note, "Nothing.");
        assert_eq!(interview.answered_at(), Some(moment(5)));
        let mut nothing_asked = running_turn();
        nothing_asked
            .finish("Report.", tokens(), moment(3))
            .unwrap();
        assert_eq!(
            nothing_asked.answer_interview(answers(), moment(4)),
            Err(DomainError::NothingAsked)
        );
    }

    #[test]
    fn a_proposal_is_approved_after_its_turn_has_ended() {
        let mut turn = running_turn();
        turn.propose(proposal()).unwrap();
        let members = || proposal().members().to_vec();
        assert_eq!(
            turn.approve_proposal(members(), Vec::new(), moment(3)),
            Err(DomainError::TurnStillRunning)
        );
        turn.finish("I propose one specialist.", tokens(), moment(4))
            .unwrap();

        turn.approve_proposal(members(), Vec::new(), moment(5))
            .unwrap();

        assert_eq!(turn.proposal().unwrap().approved_at(), Some(moment(5)));
        let mut nothing_proposed = running_turn();
        nothing_proposed
            .finish("Report.", tokens(), moment(3))
            .unwrap();
        assert_eq!(
            nothing_proposed.approve_proposal(members(), Vec::new(), moment(4)),
            Err(DomainError::NothingProposed)
        );
    }

    #[test]
    fn every_stable_name_survives_being_written_as_text_and_read_back() {
        for origin in TurnOrigin::ALL {
            assert_eq!(origin.as_str().parse::<TurnOrigin>().unwrap(), *origin);
        }
        for state in TurnState::ALL {
            assert_eq!(state.as_str().parse::<TurnState>().unwrap(), *state);
        }
        for outcome in ToolOutcome::ALL {
            assert_eq!(outcome.as_str().parse::<ToolOutcome>().unwrap(), *outcome);
        }
        for kind in StepKind::ALL {
            assert_eq!(kind.as_str().parse::<StepKind>().unwrap(), *kind);
        }
    }
}
