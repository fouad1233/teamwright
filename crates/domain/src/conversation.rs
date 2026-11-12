//! Conversations with AI specialists, and the turns they are made of.
//!
//! A conversation is a thread of work with one specialist: about a task, or
//! about a question the manager asked. It is one session of the program that
//! runs the specialist, so the specialist remembers the earlier turns.
//!
//! A turn is one request to the specialist and what came of it: the steps it
//! took while it worked, its answer, and how many tokens it used.

use crate::error::DomainError;
use crate::ids::{ConversationId, MemberId, ProjectId, TaskId, TurnId};
use crate::macros::text_enum;
use crate::time::Timestamp;

/// The plain data of a conversation, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSnapshot {
    /// The identifier of the conversation.
    pub id: ConversationId,
    /// The project the conversation belongs to.
    pub project_id: ProjectId,
    /// The specialist, or nothing if it has left the team since.
    pub member_id: Option<MemberId>,
    /// The task the conversation is about, if it is about one.
    pub task_id: Option<TaskId>,
    /// When the conversation was started.
    pub started_at: Timestamp,
    /// The identifier of the agent program's session, once it is known.
    pub session_id: Option<String>,
}

/// A thread of work with one AI specialist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    id: ConversationId,
    project_id: ProjectId,
    member_id: Option<MemberId>,
    task_id: Option<TaskId>,
    started_at: Timestamp,
    session_id: Option<String>,
}

impl Conversation {
    /// Starts a conversation with a specialist, about a task or not.
    #[must_use]
    pub fn start(
        project_id: ProjectId,
        member_id: MemberId,
        task_id: Option<TaskId>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: ConversationId::generate(),
            project_id,
            member_id: Some(member_id),
            task_id,
            started_at: now,
            session_id: None,
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

    /// The specialist, or nothing if it has left the team since.
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
}

text_enum!(TurnOrigin, kind = "origin of a turn", {
    Brief => "brief",
    Message => "message",
});

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
}

/// One request to a specialist, and what came of it.
///
/// A turn runs until it ends, and it ends exactly once: finished, failed or
/// stopped. Steps are only added while it runs.
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

        let conversation = Conversation::start(project_id, member_id, Some(task_id), moment(0));

        assert_eq!(conversation.project_id(), project_id);
        assert_eq!(conversation.member_id(), Some(member_id));
        assert_eq!(conversation.task_id(), Some(task_id));
        assert_eq!(conversation.started_at(), moment(0));
        assert_eq!(conversation.session_id(), None);
    }

    #[test]
    fn a_conversation_remembers_its_session_and_can_lose_its_specialist() {
        let mut conversation =
            Conversation::start(ProjectId::generate(), MemberId::generate(), None, moment(0));

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
        };

        let conversation = Conversation::restore(snapshot.clone());

        assert_eq!(conversation.id(), snapshot.id);
        assert_eq!(conversation.member_id(), None);
        assert_eq!(conversation.task_id(), snapshot.task_id);
        assert_eq!(conversation.session_id(), Some("abc"));
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
        };

        assert_eq!(Turn::restore(snapshot), turn);
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
