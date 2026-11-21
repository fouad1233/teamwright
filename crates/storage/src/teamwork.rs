//! Stores what specialists hand each other: the handoffs of tasks, with
//! their files, and the messages between teammates.

use std::sync::Arc;

use rusqlite::{Connection, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{StorageError, TeamworkRepository};
use teamwright_domain::{
    DomainError, Handoff, HandoffContent, HandoffSnapshot, MessageDelivery, PathInFolder,
    ProjectId, SaidIn, TeamMessage, TeamMessageSnapshot,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The columns of the `handoffs` table that every query reads.
const HANDOFF_COLUMNS: &str =
    "id, project_id, task_id, author_id, conversation_id, turn_id, summary, contract, published_at";

/// The columns of the `team_messages` table that every query reads.
const MESSAGE_COLUMNS: &str = "id, project_id, from_member_id, conversation_id, turn_id, \
                               to_member_id, kind, text, sent_at, delivered_turn_id, delivered_at";

// The rows are updated in place, never replaced.
const UPSERT_HANDOFF: &str = "
    INSERT INTO handoffs (
        id, project_id, task_id, author_id, conversation_id, turn_id, summary, contract,
        published_at
    )
    VALUES (
        :id, :project_id, :task_id, :author_id, :conversation_id, :turn_id, :summary, :contract,
        :published_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        task_id = excluded.task_id,
        author_id = excluded.author_id,
        conversation_id = excluded.conversation_id,
        turn_id = excluded.turn_id,
        summary = excluded.summary,
        contract = excluded.contract,
        published_at = excluded.published_at";

const UPSERT_FILE: &str = "
    INSERT INTO handoff_files (handoff_id, position, path)
    VALUES (:handoff_id, :position, :path)
    ON CONFLICT (handoff_id, position) DO UPDATE SET path = excluded.path";

const UPSERT_MESSAGE: &str = "
    INSERT INTO team_messages (
        id, project_id, from_member_id, conversation_id, turn_id, to_member_id, kind, text,
        sent_at, delivered_turn_id, delivered_at
    )
    VALUES (
        :id, :project_id, :from_member_id, :conversation_id, :turn_id, :to_member_id, :kind,
        :text, :sent_at, :delivered_turn_id, :delivered_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        from_member_id = excluded.from_member_id,
        conversation_id = excluded.conversation_id,
        turn_id = excluded.turn_id,
        to_member_id = excluded.to_member_id,
        kind = excluded.kind,
        text = excluded.text,
        sent_at = excluded.sent_at,
        delivered_turn_id = excluded.delivered_turn_id,
        delivered_at = excluded.delivered_at";

/// A handoff as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandoffRecord {
    id: String,
    project_id: String,
    task_id: String,
    author_id: String,
    conversation_id: String,
    turn_id: String,
    summary: String,
    contract: String,
    files: Vec<String>,
    published_at: String,
}

impl HandoffRecord {
    /// Takes the stored form of a handoff.
    fn of(handoff: &Handoff) -> Self {
        let content = handoff.content();
        Self {
            id: handoff.id().to_string(),
            project_id: handoff.project_id().to_string(),
            task_id: handoff.task_id().to_string(),
            author_id: handoff.author_id().to_string(),
            conversation_id: handoff.conversation_id().to_string(),
            turn_id: handoff.turn_id().to_string(),
            summary: content.summary().to_owned(),
            contract: content.contract().to_owned(),
            files: content.files().iter().map(PathInFolder::as_text).collect(),
            published_at: codec::moment_to_text(handoff.published_at()),
        }
    }

    /// Reads the columns of a `handoffs` row, without its files.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            task_id: row.get("task_id")?,
            author_id: row.get("author_id")?,
            conversation_id: row.get("conversation_id")?,
            turn_id: row.get("turn_id")?,
            summary: row.get("summary")?,
            contract: row.get("contract")?,
            files: Vec::new(),
            published_at: row.get("published_at")?,
        })
    }

    /// Rebuilds the handoff, checking the stored values against the rules.
    fn into_handoff(self) -> Result<Handoff, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("handoff", &self.id, &reason);
        let content =
            HandoffContent::new(&self.summary, &self.contract, &self.files).map_err(broken)?;
        Ok(Handoff::restore(HandoffSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            task_id: codec::parse(&self.task_id)?,
            author_id: codec::parse(&self.author_id)?,
            conversation_id: codec::parse(&self.conversation_id)?,
            turn_id: codec::parse(&self.turn_id)?,
            content,
            published_at: codec::moment_from_text(&self.published_at)?,
        }))
    }
}

/// A message as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MessageRecord {
    id: String,
    project_id: String,
    from_member_id: String,
    conversation_id: String,
    turn_id: String,
    to_member_id: String,
    kind: String,
    text: String,
    sent_at: String,
    delivered_turn_id: Option<String>,
    delivered_at: Option<String>,
}

impl MessageRecord {
    /// Takes the stored form of a message.
    fn of(message: &TeamMessage) -> Self {
        let said_in = message.said_in();
        let delivery = message.delivery();
        Self {
            id: message.id().to_string(),
            project_id: message.project_id().to_string(),
            from_member_id: said_in.member_id.to_string(),
            conversation_id: said_in.conversation_id.to_string(),
            turn_id: said_in.turn_id.to_string(),
            to_member_id: message.to().to_string(),
            kind: message.kind().to_string(),
            text: message.text().to_owned(),
            sent_at: codec::moment_to_text(message.sent_at()),
            delivered_turn_id: delivery.map(|delivery| delivery.turn_id.to_string()),
            delivered_at: delivery.map(|delivery| codec::moment_to_text(delivery.at)),
        }
    }

    /// Reads the columns of a `team_messages` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            from_member_id: row.get("from_member_id")?,
            conversation_id: row.get("conversation_id")?,
            turn_id: row.get("turn_id")?,
            to_member_id: row.get("to_member_id")?,
            kind: row.get("kind")?,
            text: row.get("text")?,
            sent_at: row.get("sent_at")?,
            delivered_turn_id: row.get("delivered_turn_id")?,
            delivered_at: row.get("delivered_at")?,
        })
    }

    /// Rebuilds the message, checking the stored values against the rules.
    fn into_message(self) -> Result<TeamMessage, DatabaseError> {
        let delivery = match (&self.delivered_turn_id, &self.delivered_at) {
            (None, None) => None,
            (Some(turn_id), Some(at)) => Some(MessageDelivery {
                turn_id: codec::parse(turn_id)?,
                at: codec::moment_from_text(at)?,
            }),
            _ => {
                return Err(DatabaseError::Damaged(format!(
                    "the message {} is delivered with a turn or at a moment, not both",
                    self.id
                )));
            }
        };
        TeamMessage::restore(TeamMessageSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            said_in: SaidIn {
                member_id: codec::parse(&self.from_member_id)?,
                conversation_id: codec::parse(&self.conversation_id)?,
                turn_id: codec::parse(&self.turn_id)?,
            },
            to: codec::parse(&self.to_member_id)?,
            kind: codec::parse(&self.kind)?,
            text: self.text.clone(),
            sent_at: codec::moment_from_text(&self.sent_at)?,
            delivery,
        })
        .map_err(|reason| codec::damaged_record("message", &self.id, &reason))
    }
}

/// Stores handoffs and messages in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteTeamworkRepository {
    database: Arc<Database>,
}

impl SqliteTeamworkRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl TeamworkRepository for SqliteTeamworkRepository {
    fn save_handoff(&self, handoff: &Handoff) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| store_handoff(transaction, operations, handoff))
    }

    fn handoffs_of_project(&self, project_id: ProjectId) -> Result<Vec<Handoff>, StorageError> {
        self.database
            .read(|connection| handoffs_where(connection, "project_id", &project_id.to_string()))
    }

    fn save_messages(&self, messages: &[TeamMessage]) -> Result<(), StorageError> {
        if messages.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for message in messages {
                store_message(transaction, operations, message)?;
            }
            Ok(())
        })
    }

    fn messages_of_project(&self, project_id: ProjectId) -> Result<Vec<TeamMessage>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {MESSAGE_COLUMNS} FROM team_messages WHERE project_id = :project_id
                 ORDER BY sent_at, id"
            );
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    MessageRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records
                .into_iter()
                .map(MessageRecord::into_message)
                .collect()
        })
    }
}

/// Reads the handoffs whose `column` is `value`, the oldest first, with
/// their files. `column` is one of the fixed column names above, never
/// input.
fn handoffs_where(
    connection: &Connection,
    column: &str,
    value: &str,
) -> Result<Vec<Handoff>, DatabaseError> {
    let query = format!(
        "SELECT {HANDOFF_COLUMNS} FROM handoffs WHERE {column} = :value ORDER BY published_at, id"
    );
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(named_params! { ":value": value }, HandoffRecord::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    let mut files = connection
        .prepare("SELECT path FROM handoff_files WHERE handoff_id = :id ORDER BY position")?;
    records
        .into_iter()
        .map(|mut record| {
            record.files = files
                .query_map(named_params! { ":id": record.id }, |row| row.get(0))?
                .collect::<Result<Vec<String>, _>>()?;
            record.into_handoff()
        })
        .collect()
}

/// Writes a handoff with its files and logs the change.
pub(crate) fn store_handoff(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    handoff: &Handoff,
) -> Result<(), DatabaseError> {
    let record = HandoffRecord::of(handoff);
    write_handoff_rows(transaction, &record)?;
    operations.record_upsert(Entity::Handoff, &record.id, &record)
}

/// Writes a message and logs the change.
pub(crate) fn store_message(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    message: &TeamMessage,
) -> Result<(), DatabaseError> {
    let record = MessageRecord::of(message);
    write_message_row(transaction, &record)?;
    operations.record_upsert(Entity::TeamMessage, &record.id, &record)
}

/// Removes and logs the handoffs of a task, before the task goes: the
/// database would otherwise remove them by itself, without a trace in the
/// log.
pub(crate) fn delete_handoffs_of_task(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    task_id: &str,
) -> Result<(), DatabaseError> {
    let ids: Vec<String> = transaction
        .prepare("SELECT id FROM handoffs WHERE task_id = :task_id")?
        .query_map(named_params! { ":task_id": task_id }, |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for id in ids {
        if remove_handoff(transaction, &id)? {
            operations.record_delete(Entity::Handoff, &id)?;
        }
    }
    Ok(())
}

/// Removes the rows of a handoff, without logging it, and says whether
/// there was one. Its files go with it.
pub(crate) fn remove_handoff(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM handoffs WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Removes the row of a message, without logging it, and says whether
/// there was one.
pub(crate) fn remove_message(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM team_messages WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the rows of a handoff from a state the log keeps, without
/// logging anything. A state that breaks a rule is refused, and nothing is
/// written.
pub(crate) fn write_handoff_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: HandoffRecord = codec::record_from_state("handoff", state)?;
    let record = HandoffRecord::of(&record.into_handoff()?);
    write_handoff_rows(transaction, &record)
}

/// Writes the row of a message from a state the log keeps, without logging
/// anything. A state that breaks a rule is refused, and nothing is written.
pub(crate) fn write_message_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: MessageRecord = codec::record_from_state("message", state)?;
    let record = MessageRecord::of(&record.into_message()?);
    write_message_row(transaction, &record)
}

/// Writes the rows of a handoff: the handoff, its files by position, and
/// the removal of positions it no longer has.
fn write_handoff_rows(
    transaction: &Transaction<'_>,
    record: &HandoffRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_HANDOFF,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":task_id": record.task_id,
            ":author_id": record.author_id,
            ":conversation_id": record.conversation_id,
            ":turn_id": record.turn_id,
            ":summary": record.summary,
            ":contract": record.contract,
            ":published_at": record.published_at,
        },
    )?;
    for (position, path) in (0_i64..).zip(&record.files) {
        transaction.execute(
            UPSERT_FILE,
            named_params! { ":handoff_id": record.id, ":position": position, ":path": path },
        )?;
    }
    transaction.execute(
        "DELETE FROM handoff_files WHERE handoff_id = :handoff_id AND position >= :count",
        named_params! {
            ":handoff_id": record.id,
            ":count": i64::try_from(record.files.len()).unwrap_or(i64::MAX),
        },
    )?;
    Ok(())
}

/// Writes the row of a message.
fn write_message_row(
    transaction: &Transaction<'_>,
    record: &MessageRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_MESSAGE,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":from_member_id": record.from_member_id,
            ":conversation_id": record.conversation_id,
            ":turn_id": record.turn_id,
            ":to_member_id": record.to_member_id,
            ":kind": record.kind,
            ":text": record.text,
            ":sent_at": record.sent_at,
            ":delivered_turn_id": record.delivered_turn_id,
            ":delivered_at": record.delivered_at,
        },
    )?;
    Ok(())
}
