//! Stores the conversations with specialists, their turns, and the steps of
//! the turns.
//!
//! A turn is saved after every event of the agent program, so saving it is
//! made cheap: its row is written only when it changed, and of its steps
//! only those that are new or changed (a tool use whose outcome is now
//! known) are written. The question the turn asked the manager is written
//! when it is new or has been answered, and so are the interview and the
//! proposal of the Advisor (see [`crate::planning`]). Each written row is
//! logged; an unchanged one is not.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{ConversationRepository, StorageError};
use teamwright_domain::{
    Conversation, ConversationId, ConversationSnapshot, DecisionAnswer, DecisionOption,
    DecisionRequest, DecisionSnapshot, DomainError, MemberProfile, ModelName, ProjectId, Step,
    StepContent, StepKind, TokenCounts, Turn, TurnSnapshot,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};
use crate::planning::{
    InterviewRecord, ProposalRecord, store_planning_of, stored_interview, stored_proposal,
};

/// The columns of the `conversations` table that every query reads.
const CONVERSATION_COLUMNS: &str = "id, project_id, member_id, task_id, started_at, session_id, \
     with_advisor, advisor_model, advisor_effort, home_computer";

/// The columns of the `turns` table that every query reads.
const TURN_COLUMNS: &str = "id, conversation_id, request, origin, state, started_at, ended_at, \
     answer, failure, model, input_tokens, output_tokens, cache_read_tokens, cache_written_tokens";

/// The columns of the `turn_steps` table that every query reads.
const STEP_COLUMNS: &str = "turn_id, position, at, kind, text, tool_use_id, tool, summary, outcome";

// Rows are updated in place, like every row of this database. Replacing a
// conversation or a turn would delete it first, and its turns or steps
// with it.
const UPSERT_CONVERSATION: &str = "
    INSERT INTO conversations (
        id, project_id, member_id, task_id, started_at, session_id, with_advisor,
        advisor_model, advisor_effort, home_computer
    )
    VALUES (
        :id, :project_id, :member_id, :task_id, :started_at, :session_id, :with_advisor,
        :advisor_model, :advisor_effort, :home_computer
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        member_id = excluded.member_id,
        task_id = excluded.task_id,
        started_at = excluded.started_at,
        session_id = excluded.session_id,
        with_advisor = excluded.with_advisor,
        advisor_model = excluded.advisor_model,
        advisor_effort = excluded.advisor_effort,
        home_computer = excluded.home_computer";

const UPSERT_TURN: &str = "
    INSERT INTO turns (
        id, conversation_id, request, origin, state, started_at, ended_at, answer, failure,
        model, input_tokens, output_tokens, cache_read_tokens, cache_written_tokens
    )
    VALUES (
        :id, :conversation_id, :request, :origin, :state, :started_at, :ended_at, :answer,
        :failure, :model, :input_tokens, :output_tokens, :cache_read_tokens,
        :cache_written_tokens
    )
    ON CONFLICT (id) DO UPDATE SET
        conversation_id = excluded.conversation_id,
        request = excluded.request,
        origin = excluded.origin,
        state = excluded.state,
        started_at = excluded.started_at,
        ended_at = excluded.ended_at,
        answer = excluded.answer,
        failure = excluded.failure,
        model = excluded.model,
        input_tokens = excluded.input_tokens,
        output_tokens = excluded.output_tokens,
        cache_read_tokens = excluded.cache_read_tokens,
        cache_written_tokens = excluded.cache_written_tokens";

const UPSERT_STEP: &str = "
    INSERT INTO turn_steps (
        turn_id, position, at, kind, text, tool_use_id, tool, summary, outcome
    )
    VALUES (
        :turn_id, :position, :at, :kind, :text, :tool_use_id, :tool, :summary, :outcome
    )
    ON CONFLICT (turn_id, position) DO UPDATE SET
        at = excluded.at,
        kind = excluded.kind,
        text = excluded.text,
        tool_use_id = excluded.tool_use_id,
        tool = excluded.tool,
        summary = excluded.summary,
        outcome = excluded.outcome";

const UPSERT_DECISION: &str = "
    INSERT INTO turn_decisions (
        turn_id, question, reason, recommended, answered_option, answer_note, answered_at
    )
    VALUES (
        :turn_id, :question, :reason, :recommended, :answered_option, :answer_note,
        :answered_at
    )
    ON CONFLICT (turn_id) DO UPDATE SET
        question = excluded.question,
        reason = excluded.reason,
        recommended = excluded.recommended,
        answered_option = excluded.answered_option,
        answer_note = excluded.answer_note,
        answered_at = excluded.answered_at";

const UPSERT_DECISION_OPTION: &str = "
    INSERT INTO turn_decision_options (turn_id, position, label, consequence)
    VALUES (:turn_id, :position, :label, :consequence)
    ON CONFLICT (turn_id, position) DO UPDATE SET
        label = excluded.label,
        consequence = excluded.consequence";

/// A conversation as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationRecord {
    id: String,
    project_id: String,
    member_id: Option<String>,
    task_id: Option<String>,
    started_at: String,
    session_id: Option<String>,
    with_advisor: bool,
    /// The model of the Advisor; `None` leaves it to the program.
    advisor_model: Option<String>,
    /// The effort of the Advisor; `None` leaves it to the program.
    advisor_effort: Option<String>,
    /// The computer the conversation was started on. A row without one is
    /// damaged: the migration that added it gave every conversation its
    /// computer.
    #[serde(default)]
    home_computer: Option<String>,
}

impl ConversationRecord {
    /// Takes the stored form of a conversation.
    fn of(conversation: &Conversation) -> Self {
        let advisor = conversation.advisor_profile();
        Self {
            id: conversation.id().to_string(),
            project_id: conversation.project_id().to_string(),
            member_id: conversation.member_id().map(|id| id.to_string()),
            task_id: conversation.task_id().map(|id| id.to_string()),
            started_at: codec::moment_to_text(conversation.started_at()),
            session_id: conversation.session_id().map(str::to_owned),
            with_advisor: advisor.is_some(),
            advisor_model: advisor
                .and_then(|profile| profile.model.as_ref())
                .map(ToString::to_string),
            advisor_effort: advisor
                .and_then(|profile| profile.effort)
                .map(|effort| effort.as_str().to_owned()),
            home_computer: Some(conversation.home_computer().to_string()),
        }
    }

    /// Reads the columns of a `conversations` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            member_id: row.get("member_id")?,
            task_id: row.get("task_id")?,
            started_at: row.get("started_at")?,
            session_id: row.get("session_id")?,
            with_advisor: row.get("with_advisor")?,
            advisor_model: row.get("advisor_model")?,
            advisor_effort: row.get("advisor_effort")?,
            home_computer: row.get("home_computer")?,
        })
    }

    /// Rebuilds the conversation, checking the stored values. A
    /// conversation with a specialist that has a model or an effort of the
    /// Advisor is damaged data.
    fn into_conversation(self) -> Result<Conversation, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("conversation", &self.id, &reason);
        let advisor = if self.with_advisor {
            Some(MemberProfile {
                model: self
                    .advisor_model
                    .as_deref()
                    .map(ModelName::parse)
                    .transpose()
                    .map_err(broken)?,
                effort: self
                    .advisor_effort
                    .as_deref()
                    .map(codec::parse)
                    .transpose()?,
            })
        } else if self.advisor_model.is_some() || self.advisor_effort.is_some() {
            return Err(DatabaseError::Damaged(format!(
                "the conversation {} is with a specialist and has a model or an effort of the \
                 Advisor",
                self.id
            )));
        } else {
            None
        };
        let home_computer = self.home_computer.as_deref().ok_or_else(|| {
            DatabaseError::Damaged(format!(
                "the conversation {} does not say which computer it runs on",
                self.id
            ))
        })?;
        Ok(Conversation::restore(ConversationSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            member_id: self.member_id.as_deref().map(codec::parse).transpose()?,
            task_id: self.task_id.as_deref().map(codec::parse).transpose()?,
            started_at: codec::moment_from_text(&self.started_at)?,
            session_id: self.session_id,
            advisor,
            home_computer: codec::parse(home_computer)?,
        }))
    }
}

/// A turn without its steps, as the database and the operation log keep it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TurnRecord {
    id: String,
    conversation_id: String,
    request: String,
    origin: String,
    state: String,
    started_at: String,
    ended_at: Option<String>,
    answer: String,
    failure: Option<String>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_written_tokens: Option<i64>,
}

impl TurnRecord {
    /// Takes the stored form of a turn, without its steps.
    fn of(turn: &Turn) -> Self {
        let tokens = turn.tokens();
        let count = |pick: fn(&TokenCounts) -> u64| tokens.as_ref().map(|t| stored_count(pick(t)));
        Self {
            id: turn.id().to_string(),
            conversation_id: turn.conversation_id().to_string(),
            request: turn.request().to_owned(),
            origin: turn.origin().to_string(),
            state: turn.state().to_string(),
            started_at: codec::moment_to_text(turn.started_at()),
            ended_at: turn.ended_at().map(codec::moment_to_text),
            answer: turn.answer().to_owned(),
            failure: turn.failure().map(str::to_owned),
            model: turn.model().map(str::to_owned),
            input_tokens: count(|tokens| tokens.input),
            output_tokens: count(|tokens| tokens.output),
            cache_read_tokens: count(|tokens| tokens.cache_read),
            cache_written_tokens: count(|tokens| tokens.cache_written),
        }
    }

    /// Reads the columns of a `turns` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            conversation_id: row.get("conversation_id")?,
            request: row.get("request")?,
            origin: row.get("origin")?,
            state: row.get("state")?,
            started_at: row.get("started_at")?,
            ended_at: row.get("ended_at")?,
            answer: row.get("answer")?,
            failure: row.get("failure")?,
            model: row.get("model")?,
            input_tokens: row.get("input_tokens")?,
            output_tokens: row.get("output_tokens")?,
            cache_read_tokens: row.get("cache_read_tokens")?,
            cache_written_tokens: row.get("cache_written_tokens")?,
        })
    }

    /// Rebuilds the turn with its steps, its question, its interview and
    /// its proposal, checking the stored values.
    fn into_turn(
        self,
        steps: Vec<StepRecord>,
        decision: Option<DecisionRecord>,
        interview: Option<InterviewRecord>,
        proposal: Option<ProposalRecord>,
    ) -> Result<Turn, DatabaseError> {
        let tokens = self.tokens()?;
        let steps = steps
            .into_iter()
            .map(StepRecord::into_step)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Turn::restore(TurnSnapshot {
            id: codec::parse(&self.id)?,
            conversation_id: codec::parse(&self.conversation_id)?,
            request: self.request,
            origin: codec::parse(&self.origin)?,
            state: codec::parse(&self.state)?,
            started_at: codec::moment_from_text(&self.started_at)?,
            ended_at: self
                .ended_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
            steps,
            answer: self.answer,
            failure: self.failure,
            model: self.model,
            tokens,
            decision: decision.map(DecisionRecord::into_decision).transpose()?,
            interview: interview.map(InterviewRecord::into_interview).transpose()?,
            proposal: proposal.map(ProposalRecord::into_proposal).transpose()?,
        }))
    }

    /// Reads the tokens from their four columns.
    fn tokens(&self) -> Result<Option<TokenCounts>, DatabaseError> {
        match (
            self.input_tokens,
            self.output_tokens,
            self.cache_read_tokens,
            self.cache_written_tokens,
        ) {
            (None, None, None, None) => Ok(None),
            (Some(input), Some(output), Some(cache_read), Some(cache_written)) => {
                Ok(Some(TokenCounts {
                    input: count_from_store(input)?,
                    output: count_from_store(output)?,
                    cache_read: count_from_store(cache_read)?,
                    cache_written: count_from_store(cache_written)?,
                }))
            }
            _ => Err(DatabaseError::Damaged(format!(
                "the turn {} has some of its counts of tokens but not all",
                self.id
            ))),
        }
    }
}

/// A step of a turn as the database and the operation log keep it. The
/// columns a kind does not use are empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StepRecord {
    turn_id: String,
    position: i64,
    at: String,
    kind: String,
    text: Option<String>,
    tool_use_id: Option<String>,
    tool: Option<String>,
    summary: Option<String>,
    outcome: Option<String>,
}

impl StepRecord {
    /// Takes the stored form of every step of a turn, in order.
    fn all_of(turn: &Turn) -> Vec<Self> {
        let turn_id = turn.id().to_string();
        (0_i64..)
            .zip(turn.steps())
            .map(|(position, step)| Self::of(&turn_id, position, step))
            .collect()
    }

    /// Takes the stored form of one step.
    fn of(turn_id: &str, position: i64, step: &Step) -> Self {
        let mut record = Self {
            turn_id: turn_id.to_owned(),
            position,
            at: codec::moment_to_text(step.at),
            kind: step.content.kind().to_string(),
            text: None,
            tool_use_id: None,
            tool: None,
            summary: None,
            outcome: None,
        };
        match &step.content {
            StepContent::Text { text } => record.text = Some(text.clone()),
            StepContent::Notice { message } => record.text = Some(message.clone()),
            StepContent::Tool {
                use_id,
                tool,
                summary,
                outcome,
            } => {
                record.tool_use_id = Some(use_id.clone());
                record.tool = Some(tool.clone());
                record.summary = Some(summary.clone());
                record.outcome = Some(outcome.to_string());
            }
            StepContent::Denied { tool } => record.tool = Some(tool.clone()),
        }
        record
    }

    /// Reads the columns of a `turn_steps` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            turn_id: row.get("turn_id")?,
            position: row.get("position")?,
            at: row.get("at")?,
            kind: row.get("kind")?,
            text: row.get("text")?,
            tool_use_id: row.get("tool_use_id")?,
            tool: row.get("tool")?,
            summary: row.get("summary")?,
            outcome: row.get("outcome")?,
        })
    }

    /// The identifier of the step in the operation log.
    fn log_id(&self) -> String {
        step_log_id(&self.turn_id, self.position)
    }

    /// Rebuilds the step, checking that the columns of its kind are there.
    fn into_step(self) -> Result<Step, DatabaseError> {
        let missing = |column: &str| {
            DatabaseError::Damaged(format!(
                "the step {} of the turn {} has no {column}",
                self.position, self.turn_id
            ))
        };
        let at = codec::moment_from_text(&self.at)?;
        let content = match codec::parse::<StepKind>(&self.kind)? {
            StepKind::Text => StepContent::Text {
                text: self.text.clone().ok_or_else(|| missing("text"))?,
            },
            StepKind::Notice => StepContent::Notice {
                message: self.text.clone().ok_or_else(|| missing("text"))?,
            },
            StepKind::Tool => StepContent::Tool {
                use_id: self
                    .tool_use_id
                    .clone()
                    .ok_or_else(|| missing("tool_use_id"))?,
                tool: self.tool.clone().ok_or_else(|| missing("tool"))?,
                summary: self.summary.clone().ok_or_else(|| missing("summary"))?,
                outcome: codec::parse(self.outcome.as_deref().ok_or_else(|| missing("outcome"))?)?,
            },
            StepKind::Denied => StepContent::Denied {
                tool: self.tool.clone().ok_or_else(|| missing("tool"))?,
            },
        };
        Ok(Step { at, content })
    }
}

/// The question a turn asked the manager, with its options and its answer,
/// as the database and the operation log keep it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DecisionRecord {
    turn_id: String,
    question: String,
    reason: String,
    recommended: Option<i64>,
    options: Vec<DecisionOptionRecord>,
    answered_option: Option<i64>,
    answer_note: Option<String>,
    answered_at: Option<String>,
}

/// One option of a question, in its place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DecisionOptionRecord {
    label: String,
    consequence: String,
}

impl DecisionRecord {
    /// Takes the stored form of the question of a turn, if it asked one.
    fn of(turn: &Turn) -> Option<Self> {
        let decision = turn.decision()?;
        let answer = decision.answer();
        Some(Self {
            turn_id: turn.id().to_string(),
            question: decision.question().to_owned(),
            reason: decision.reason().to_owned(),
            recommended: decision.recommended().map(stored_position),
            options: decision
                .options()
                .iter()
                .map(|option| DecisionOptionRecord {
                    label: option.label.clone(),
                    consequence: option.consequence.clone(),
                })
                .collect(),
            answered_option: answer.and_then(|answer| answer.option).map(stored_position),
            answer_note: answer.map(|answer| answer.note.clone()),
            answered_at: answer.map(|answer| codec::moment_to_text(answer.at)),
        })
    }

    /// Rebuilds the question, checking the stored values.
    fn into_decision(self) -> Result<DecisionRequest, DatabaseError> {
        let answer = match (self.answered_at, self.answer_note) {
            (None, None) if self.answered_option.is_none() => None,
            (Some(at), Some(note)) => Some(DecisionAnswer {
                option: self.answered_option.map(position_from_store).transpose()?,
                note,
                at: codec::moment_from_text(&at)?,
            }),
            _ => {
                return Err(DatabaseError::Damaged(format!(
                    "the question of the turn {} has some of its answer but not all",
                    self.turn_id
                )));
            }
        };
        Ok(DecisionRequest::restore(DecisionSnapshot {
            question: self.question,
            reason: self.reason,
            options: self
                .options
                .into_iter()
                .map(|option| DecisionOption {
                    label: option.label,
                    consequence: option.consequence,
                })
                .collect(),
            recommended: self.recommended.map(position_from_store).transpose()?,
            answer,
        }))
    }
}

/// Stores the conversations in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteConversationRepository {
    database: Arc<Database>,
}

impl SqliteConversationRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl ConversationRepository for SqliteConversationRepository {
    fn save_conversation(&self, conversation: &Conversation) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            store_conversation(transaction, operations, conversation)
        })
    }

    fn save_turn(&self, turn: &Turn) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| store_turn(transaction, operations, turn))
    }

    fn find(&self, id: ConversationId) -> Result<Option<Conversation>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {CONVERSATION_COLUMNS} FROM conversations WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    ConversationRecord::from_row,
                )
                .optional()?
                .map(ConversationRecord::into_conversation)
                .transpose()
        })
    }

    fn turns_of(&self, id: ConversationId) -> Result<Vec<Turn>, StorageError> {
        self.database.read(|connection| {
            let id = id.to_string();
            let mut steps_by_turn = group_by_turn(read_steps(
                connection,
                &format!(
                    "SELECT {STEP_COLUMNS} FROM turn_steps
                     WHERE turn_id IN (SELECT id FROM turns WHERE conversation_id = :id)
                     ORDER BY turn_id, position"
                ),
                &id,
            )?);
            let query = format!(
                "SELECT {TURN_COLUMNS} FROM turns
                 WHERE conversation_id = :id
                 ORDER BY started_at, id"
            );
            read_turns(connection, &query, Some(&id))?
                .into_iter()
                .map(|record| {
                    let steps = steps_by_turn.remove(&record.id).unwrap_or_default();
                    read_turn(connection, record, steps)
                })
                .collect()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Conversation>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {CONVERSATION_COLUMNS} FROM conversations
                 WHERE project_id = :project_id
                 ORDER BY started_at, id"
            );
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    ConversationRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records
                .into_iter()
                .map(ConversationRecord::into_conversation)
                .collect()
        })
    }

    fn running_turns(&self) -> Result<Vec<Turn>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {TURN_COLUMNS} FROM turns WHERE state = 'running' ORDER BY started_at, id"
            );
            read_turns(connection, &query, None)?
                .into_iter()
                .map(|record| {
                    let steps = steps_of(connection, &record.id)?;
                    read_turn(connection, record, steps)
                })
                .collect()
        })
    }

    fn delete(&self, ids: &[ConversationId]) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            for id in ids {
                delete_conversation(transaction, operations, &id.to_string())?;
            }
            Ok(())
        })
    }
}

/// Writes a conversation without its turns and logs the change.
pub(crate) fn store_conversation(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    conversation: &Conversation,
) -> Result<(), DatabaseError> {
    let record = ConversationRecord::of(conversation);
    write_conversation(transaction, &record)?;
    operations.record_upsert(Entity::Conversation, &record.id, &record)
}

/// Writes the row of a conversation.
fn write_conversation(
    transaction: &Transaction<'_>,
    record: &ConversationRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_CONVERSATION,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":member_id": record.member_id,
            ":task_id": record.task_id,
            ":started_at": record.started_at,
            ":session_id": record.session_id,
            ":with_advisor": record.with_advisor,
            ":advisor_model": record.advisor_model,
            ":advisor_effort": record.advisor_effort,
            ":home_computer": record.home_computer,
        },
    )?;
    Ok(())
}

/// Writes the row of a conversation from a state the log keeps, without
/// logging anything. The state is first checked against the rules: a state
/// that breaks one is refused, and nothing is written.
pub(crate) fn write_conversation_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: ConversationRecord = codec::record_from_state("conversation", state)?;
    write_conversation(
        transaction,
        &ConversationRecord::of(&record.into_conversation()?),
    )
}

/// Writes the row of a turn, without its steps, from a state the log keeps,
/// without logging anything. The state is checked first.
pub(crate) fn write_turn_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: TurnRecord = codec::record_from_state("turn", state)?;
    record.clone().into_turn(Vec::new(), None, None, None)?;
    write_turn(transaction, &record)
}

/// Writes the row of a step of a turn from a state the log keeps, without
/// logging anything. The state is checked first.
pub(crate) fn write_step_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: StepRecord = codec::record_from_state("step of a turn", state)?;
    record.clone().into_step()?;
    write_step(transaction, &record)
}

/// Writes the question of a turn from a state the log keeps, without
/// logging anything. The state is checked first.
pub(crate) fn write_decision_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: DecisionRecord = codec::record_from_state("question of a turn", state)?;
    record.clone().into_decision()?;
    write_decision(transaction, &record)
}

/// Removes a conversation with its turns, and says whether there was one.
/// The removal of each turn that is left is logged; the removal of the
/// conversation itself is left to the caller.
pub(crate) fn remove_conversation(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let turn_ids: Vec<String> = transaction
        .prepare("SELECT id FROM turns WHERE conversation_id = :id ORDER BY started_at, id")?
        .query_map(named_params! { ":id": id }, |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for turn_id in turn_ids {
        remove_turn(transaction, &turn_id)?;
        operations.record_delete(Entity::Turn, &turn_id)?;
    }
    let rows_removed = transaction.execute(
        "DELETE FROM conversations WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Removes a turn with its steps, its question, its interview and its
/// proposal, without logging anything, and says whether there was one.
pub(crate) fn remove_turn(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    transaction.execute(
        "DELETE FROM turn_steps WHERE turn_id = :turn_id",
        named_params! { ":turn_id": id },
    )?;
    let rows_removed = transaction.execute(
        "DELETE FROM turns WHERE id = :turn_id",
        named_params! { ":turn_id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Removes a step of a turn by its identifier in the log, without logging
/// anything.
pub(crate) fn remove_step(
    transaction: &Transaction<'_>,
    log_id: &str,
) -> Result<(), DatabaseError> {
    let Some((turn_id, position)) = log_id.rsplit_once('/') else {
        return Err(DatabaseError::damaged(
            "the identifier of a step",
            log_id,
            "it has no position",
        ));
    };
    let position: i64 = position
        .parse()
        .map_err(|reason| DatabaseError::damaged("the identifier of a step", log_id, reason))?;
    transaction.execute(
        "DELETE FROM turn_steps WHERE turn_id = :turn_id AND position = :position",
        named_params! { ":turn_id": turn_id, ":position": position },
    )?;
    Ok(())
}

/// Writes a turn: its row, its steps, its question, its interview and its
/// proposal, each only if it changed, and logs what was written.
pub(crate) fn store_turn(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    turn: &Turn,
) -> Result<(), DatabaseError> {
    let record = TurnRecord::of(turn);
    if stored_turn(transaction, &record.id)?.as_ref() != Some(&record) {
        write_turn(transaction, &record)?;
        operations.record_upsert(Entity::Turn, &record.id, &record)?;
    }

    let mut stored_steps: BTreeMap<i64, StepRecord> = steps_of(transaction, &record.id)?
        .into_iter()
        .map(|step| (step.position, step))
        .collect();
    for step in &StepRecord::all_of(turn) {
        if stored_steps.remove(&step.position).as_ref() != Some(step) {
            write_step(transaction, step)?;
            operations.record_upsert(Entity::TurnStep, &step.log_id(), step)?;
        }
    }
    // What is left was stored once and is no longer part of the turn, like
    // the last text of a finished turn, which is its answer.
    for obsolete in stored_steps.values() {
        transaction.execute(
            "DELETE FROM turn_steps WHERE turn_id = :turn_id AND position = :position",
            named_params! {
                ":turn_id": obsolete.turn_id,
                ":position": obsolete.position,
            },
        )?;
        operations.record_delete(Entity::TurnStep, &obsolete.log_id())?;
    }

    // A turn never loses its question, so there is nothing to remove: only
    // a new or a newly answered question is written.
    if let Some(decision) = &DecisionRecord::of(turn)
        && stored_decision(transaction, &record.id)?.as_ref() != Some(decision)
    {
        write_decision(transaction, decision)?;
        operations.record_upsert(Entity::Decision, &decision.turn_id, decision)?;
    }
    store_planning_of(transaction, operations, turn)
}

/// Rebuilds a turn from its row and its steps, with what it asked or
/// proposed.
fn read_turn(
    connection: &Connection,
    record: TurnRecord,
    steps: Vec<StepRecord>,
) -> Result<Turn, DatabaseError> {
    let decision = stored_decision(connection, &record.id)?;
    let interview = stored_interview(connection, &record.id)?;
    let proposal = stored_proposal(connection, &record.id)?;
    record.into_turn(steps, decision, interview, proposal)
}

/// Removes the conversations of a project with their turns, and logs each
/// removal. Used when the project itself is removed.
pub(crate) fn delete_conversations_of_project(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    project_id: &str,
) -> Result<(), DatabaseError> {
    let ids: Vec<String> = transaction
        .prepare("SELECT id FROM conversations WHERE project_id = :project_id")?
        .query_map(named_params! { ":project_id": project_id }, |row| {
            row.get(0)
        })?
        .collect::<Result<_, _>>()?;
    for id in ids {
        delete_conversation(transaction, operations, &id)?;
    }
    Ok(())
}

/// Removes one conversation with its turns and their steps. The removal of
/// each turn is logged, and stands for the removal of its steps; then the
/// removal of the conversation is logged.
pub(crate) fn delete_conversation(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    id: &str,
) -> Result<(), DatabaseError> {
    if remove_conversation(transaction, operations, id)? {
        operations.record_delete(Entity::Conversation, id)?;
    }
    Ok(())
}

/// Writes the row of a turn.
fn write_turn(transaction: &Transaction<'_>, record: &TurnRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_TURN,
        named_params! {
            ":id": record.id,
            ":conversation_id": record.conversation_id,
            ":request": record.request,
            ":origin": record.origin,
            ":state": record.state,
            ":started_at": record.started_at,
            ":ended_at": record.ended_at,
            ":answer": record.answer,
            ":failure": record.failure,
            ":model": record.model,
            ":input_tokens": record.input_tokens,
            ":output_tokens": record.output_tokens,
            ":cache_read_tokens": record.cache_read_tokens,
            ":cache_written_tokens": record.cache_written_tokens,
        },
    )?;
    Ok(())
}

/// Writes the row of a step.
fn write_step(transaction: &Transaction<'_>, record: &StepRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_STEP,
        named_params! {
            ":turn_id": record.turn_id,
            ":position": record.position,
            ":at": record.at,
            ":kind": record.kind,
            ":text": record.text,
            ":tool_use_id": record.tool_use_id,
            ":tool": record.tool,
            ":summary": record.summary,
            ":outcome": record.outcome,
        },
    )?;
    Ok(())
}

/// Writes the question of a turn with its options.
fn write_decision(
    transaction: &Transaction<'_>,
    record: &DecisionRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_DECISION,
        named_params! {
            ":turn_id": record.turn_id,
            ":question": record.question,
            ":reason": record.reason,
            ":recommended": record.recommended,
            ":answered_option": record.answered_option,
            ":answer_note": record.answer_note,
            ":answered_at": record.answered_at,
        },
    )?;
    for (position, option) in (0_i64..).zip(&record.options) {
        transaction.execute(
            UPSERT_DECISION_OPTION,
            named_params! {
                ":turn_id": record.turn_id,
                ":position": position,
                ":label": option.label,
                ":consequence": option.consequence,
            },
        )?;
    }
    transaction.execute(
        "DELETE FROM turn_decision_options WHERE turn_id = :turn_id AND position >= :count",
        named_params! {
            ":turn_id": record.turn_id,
            ":count": stored_position(record.options.len()),
        },
    )?;
    Ok(())
}

/// Reads the stored question of one turn with its options, if it asked
/// one.
fn stored_decision(
    connection: &Connection,
    turn_id: &str,
) -> Result<Option<DecisionRecord>, DatabaseError> {
    let Some(mut record) = connection
        .query_row(
            "SELECT turn_id, question, reason, recommended, answered_option, answer_note,
                    answered_at
             FROM turn_decisions WHERE turn_id = :turn_id",
            named_params! { ":turn_id": turn_id },
            |row| {
                Ok(DecisionRecord {
                    turn_id: row.get("turn_id")?,
                    question: row.get("question")?,
                    reason: row.get("reason")?,
                    recommended: row.get("recommended")?,
                    options: Vec::new(),
                    answered_option: row.get("answered_option")?,
                    answer_note: row.get("answer_note")?,
                    answered_at: row.get("answered_at")?,
                })
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    record.options = connection
        .prepare(
            "SELECT label, consequence FROM turn_decision_options
             WHERE turn_id = :turn_id ORDER BY position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok(DecisionOptionRecord {
                label: row.get("label")?,
                consequence: row.get("consequence")?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(Some(record))
}

/// Reads the stored row of one turn, if there is one.
fn stored_turn(connection: &Connection, id: &str) -> Result<Option<TurnRecord>, DatabaseError> {
    let query = format!("SELECT {TURN_COLUMNS} FROM turns WHERE id = :id");
    Ok(connection
        .query_row(&query, named_params! { ":id": id }, TurnRecord::from_row)
        .optional()?)
}

/// Reads the rows of turns. The query has the parameter `:id` if `id` is
/// given, and none otherwise.
fn read_turns(
    connection: &Connection,
    query: &str,
    id: Option<&str>,
) -> Result<Vec<TurnRecord>, DatabaseError> {
    let mut statement = connection.prepare(query)?;
    let rows = match id {
        Some(id) => statement.query_map(named_params! { ":id": id }, TurnRecord::from_row)?,
        None => statement.query_map([], TurnRecord::from_row)?,
    };
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Reads the steps of one turn in order.
fn steps_of(connection: &Connection, turn_id: &str) -> Result<Vec<StepRecord>, DatabaseError> {
    read_steps(
        connection,
        &format!("SELECT {STEP_COLUMNS} FROM turn_steps WHERE turn_id = :id ORDER BY position"),
        turn_id,
    )
}

/// Reads the rows of steps with a query whose one parameter is `:id`.
fn read_steps(
    connection: &Connection,
    query: &str,
    id: &str,
) -> Result<Vec<StepRecord>, DatabaseError> {
    let mut statement = connection.prepare(query)?;
    let steps = statement
        .query_map(named_params! { ":id": id }, StepRecord::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(steps)
}

/// Groups steps by their turn, keeping their order.
fn group_by_turn(steps: Vec<StepRecord>) -> HashMap<String, Vec<StepRecord>> {
    let mut steps_by_turn: HashMap<String, Vec<StepRecord>> = HashMap::new();
    for step in steps {
        steps_by_turn
            .entry(step.turn_id.clone())
            .or_default()
            .push(step);
    }
    steps_by_turn
}

/// The identifier of a step in the operation log: its turn and its position.
fn step_log_id(turn_id: &str, position: i64) -> String {
    format!("{turn_id}/{position}")
}

/// A count of tokens as SQLite keeps it. SQLite has no unsigned integers; a
/// count beyond the largest signed one cannot occur in practice and is kept
/// as that largest one.
fn stored_count(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// The place of an option as SQLite keeps it. A question has at most four
/// options, so the place always fits.
fn stored_position(position: usize) -> i64 {
    i64::try_from(position).unwrap_or(i64::MAX)
}

/// Reads the place of an option that SQLite keeps.
fn position_from_store(position: i64) -> Result<usize, DatabaseError> {
    usize::try_from(position).map_err(|reason| {
        DatabaseError::damaged("the place of an option", &position.to_string(), reason)
    })
}

/// Reads a count of tokens that SQLite keeps.
fn count_from_store(count: i64) -> Result<u64, DatabaseError> {
    u64::try_from(count)
        .map_err(|reason| DatabaseError::damaged("the count of tokens", &count.to_string(), reason))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_application::{MemberRepository, ProjectRepository};
    use teamwright_domain::{
        Color, MemberKind, MemberName, ProjectKey, ProjectName, TurnOrigin, Workflow,
    };

    use super::*;
    use crate::members::SqliteMemberRepository;
    use crate::projects::SqliteProjectRepository;

    fn moment() -> teamwright_domain::Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn count_of_steps(database: &Database) -> i64 {
        database
            .read(|connection| {
                Ok(
                    connection
                        .query_row("SELECT count(*) FROM turn_steps", [], |row| row.get(0))?,
                )
            })
            .unwrap()
    }

    /// A database with a project, a member, a conversation and a turn of
    /// two steps.
    fn database_with_a_turn() -> (Arc<Database>, Conversation, Turn) {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let name = ProjectName::parse("Smart sensor node").unwrap();
        let project = teamwright_domain::Project::create(
            name.clone(),
            ProjectKey::suggest(&name),
            String::new(),
            Workflow::standard(),
            moment(),
        );
        SqliteProjectRepository::new(database.clone())
            .save(&project)
            .unwrap();
        let member = teamwright_domain::Member::create(
            project.id(),
            MemberName::parse("Hardware engineer").unwrap(),
            MemberKind::Agent,
            String::new(),
            Color::Teal,
            moment(),
        );
        SqliteMemberRepository::new(database.clone())
            .save(&member)
            .unwrap();
        let conversation = Conversation::start(
            project.id(),
            member.id(),
            None,
            moment(),
            database.device_id(),
        );
        let mut turn = Turn::start(
            conversation.id(),
            "Which regulator?",
            TurnOrigin::Message,
            moment(),
        )
        .unwrap();
        turn.notice("Starting.".to_owned(), moment()).unwrap();
        turn.say("The smaller one.".to_owned(), moment()).unwrap();
        let repository = SqliteConversationRepository::new(database.clone());
        repository.save_conversation(&conversation).unwrap();
        repository.save_turn(&turn).unwrap();
        (database, conversation, turn)
    }

    #[test]
    fn a_step_that_is_no_longer_part_of_its_turn_loses_its_row() {
        let (database, _, mut turn) = database_with_a_turn();
        assert_eq!(count_of_steps(&database), 2);

        turn.finish("The smaller one.", TokenCounts::default(), moment())
            .unwrap();
        SqliteConversationRepository::new(database.clone())
            .save_turn(&turn)
            .unwrap();

        assert_eq!(count_of_steps(&database), 1);
    }

    #[test]
    fn the_steps_go_with_their_conversation() {
        let (database, conversation, _) = database_with_a_turn();

        SqliteConversationRepository::new(database.clone())
            .delete(&[conversation.id()])
            .unwrap();

        assert_eq!(count_of_steps(&database), 0);
    }

    #[test]
    fn the_question_of_a_turn_goes_with_its_conversation() {
        let (database, conversation, mut turn) = database_with_a_turn();
        turn.ask(
            DecisionRequest::new(
                "Which regulator?",
                "",
                vec![
                    DecisionOption {
                        label: "The smaller one".to_owned(),
                        consequence: String::new(),
                    },
                    DecisionOption {
                        label: "The cheaper one".to_owned(),
                        consequence: String::new(),
                    },
                ],
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let repository = SqliteConversationRepository::new(database.clone());
        repository.save_turn(&turn).unwrap();
        let rows_of_questions = || -> i64 {
            database
                .read(|connection| {
                    Ok(connection.query_row(
                        "SELECT (SELECT count(*) FROM turn_decisions)
                              + (SELECT count(*) FROM turn_decision_options)",
                        [],
                        |row| row.get(0),
                    )?)
                })
                .unwrap()
        };
        assert_eq!(rows_of_questions(), 3);

        repository.delete(&[conversation.id()]).unwrap();

        assert_eq!(rows_of_questions(), 0);
    }

    #[test]
    fn the_interview_and_the_proposal_of_a_turn_go_with_its_conversation() {
        use teamwright_domain::{
            Interview, InterviewQuestion, MemberName, Priority, Proposal, ProposedTask, TaskTitle,
        };

        let (database, conversation, mut turn) = database_with_a_turn();
        turn.ask_interview(
            Interview::new(vec![InterviewQuestion {
                question: "What matters most?".to_owned(),
                reason: String::new(),
                options: vec![
                    DecisionOption {
                        label: "Soon".to_owned(),
                        consequence: String::new(),
                    },
                    DecisionOption {
                        label: "Cheap".to_owned(),
                        consequence: String::new(),
                    },
                ],
                recommended: None,
            }])
            .unwrap(),
        )
        .unwrap();
        let mut proposing =
            Turn::start(conversation.id(), "Propose.", TurnOrigin::Message, moment()).unwrap();
        proposing
            .propose(
                Proposal::new(
                    "A device.",
                    Vec::new(),
                    vec![
                        ProposedTask {
                            title: TaskTitle::parse("Draw the schematic").unwrap(),
                            description: String::new(),
                            owner: Some(teamwright_domain::ProposedOwner::Member(
                                MemberName::parse("Hardware engineer").unwrap(),
                            )),
                            estimate_minutes: None,
                            priority: Priority::Normal,
                            waits_for: Vec::new(),
                        },
                        ProposedTask {
                            title: TaskTitle::parse("Route the board").unwrap(),
                            description: String::new(),
                            owner: None,
                            estimate_minutes: None,
                            priority: Priority::Normal,
                            waits_for: vec![0],
                        },
                    ],
                )
                .unwrap(),
            )
            .unwrap();
        let repository = SqliteConversationRepository::new(database.clone());
        repository.save_turn(&turn).unwrap();
        repository.save_turn(&proposing).unwrap();
        let rows = || -> i64 {
            database
                .read(|connection| {
                    Ok(connection.query_row(
                        "SELECT (SELECT count(*) FROM turn_interviews)
                              + (SELECT count(*) FROM turn_interview_questions)
                              + (SELECT count(*) FROM turn_interview_options)
                              + (SELECT count(*) FROM turn_proposals)
                              + (SELECT count(*) FROM turn_proposal_tasks)
                              + (SELECT count(*) FROM turn_proposal_task_waits)",
                        [],
                        |row| row.get(0),
                    )?)
                })
                .unwrap()
        };
        assert_eq!(rows(), 8);

        repository.delete(&[conversation.id()]).unwrap();

        assert_eq!(rows(), 0);
    }

    #[test]
    fn counts_of_tokens_survive_the_trip_through_signed_numbers() {
        assert_eq!(count_from_store(stored_count(21_500)).unwrap(), 21_500);
        assert_eq!(stored_count(u64::MAX), i64::MAX);
        assert!(count_from_store(-1).is_err());
    }
}
