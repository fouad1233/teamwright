//! Stores the coordinator of each project, its budgets in tokens and the
//! record of what it did
//! ([decision 0037](../../../docs/decisions/0037-the-coordinator.md)).
//!
//! All three are logged and travel to the other computers. The switch of a
//! coordinator names the computer it runs on, so the others see that it
//! runs there and start nothing themselves.

use std::sync::Arc;

use rusqlite::{OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{CoordinatorRepository, StorageError};
use teamwright_domain::{
    Budget, BudgetAnswer, BudgetQuestion, BudgetScope, BudgetScopeKind, ConcurrencyLimit,
    Coordinator, CoordinatorEntry, DomainError, ProjectId, Timestamp, TokenBudget,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::Entity;

// The rows are updated in place, never replaced.
const UPSERT_COORDINATOR: &str = "
    INSERT INTO coordinators (id, project_id, concurrency_limit, runs_on, switched_at)
    VALUES (:id, :project_id, :concurrency_limit, :runs_on, :switched_at)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        concurrency_limit = excluded.concurrency_limit,
        runs_on = excluded.runs_on,
        switched_at = excluded.switched_at";

const UPSERT_BUDGET: &str = "
    INSERT INTO budgets (id, project_id, scope, tokens, question)
    VALUES (:id, :project_id, :scope, :tokens, :question)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        scope = excluded.scope,
        tokens = excluded.tokens,
        question = excluded.question";

const UPSERT_ENTRY: &str = "
    INSERT INTO coordinator_entries (
        id, project_id, at, kind, text, task_id, member_id, computer
    )
    VALUES (:id, :project_id, :at, :kind, :text, :task_id, :member_id, :computer)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        at = excluded.at,
        kind = excluded.kind,
        text = excluded.text,
        task_id = excluded.task_id,
        member_id = excluded.member_id,
        computer = excluded.computer";

const ENTRY_COLUMNS: &str = "id, project_id, at, kind, text, task_id, member_id, computer";

/// A coordinator as the database and the operation log keep it. Its
/// identifier is that of its project.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoordinatorRecord {
    id: String,
    project_id: String,
    concurrency_limit: i64,
    runs_on: Option<String>,
    switched_at: Option<String>,
}

impl CoordinatorRecord {
    fn of(coordinator: &Coordinator) -> Self {
        Self {
            id: coordinator.project_id().to_string(),
            project_id: coordinator.project_id().to_string(),
            concurrency_limit: i64::from(coordinator.limit().value()),
            runs_on: coordinator.runs_on().map(|computer| computer.to_string()),
            switched_at: coordinator.switched_at().map(codec::moment_to_text),
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            concurrency_limit: row.get("concurrency_limit")?,
            runs_on: row.get("runs_on")?,
            switched_at: row.get("switched_at")?,
        })
    }

    fn into_coordinator(self) -> Result<Coordinator, DatabaseError> {
        let limit = ConcurrencyLimit::new(self.concurrency_limit)
            .map_err(|reason| codec::damaged_record("coordinator", &self.id, &reason))?;
        Ok(Coordinator::restore(
            codec::parse(&self.project_id)?,
            limit,
            self.runs_on.as_deref().map(codec::parse).transpose()?,
            self.switched_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        ))
    }
}

/// The question about a budget, as the log keeps it inside the budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionRecord {
    asked_at: String,
    limit: u64,
    used: u64,
    /// `raised` or `left_stopped`, once the manager answered.
    answer: Option<String>,
    raised_by: Option<u64>,
    answered_at: Option<String>,
}

impl QuestionRecord {
    fn of(question: &BudgetQuestion) -> Self {
        let (answer, raised_by, answered_at) = match question.answer {
            None => (None, None, None),
            Some((BudgetAnswer::Raised { by }, at)) => (
                Some("raised".to_owned()),
                Some(by),
                Some(codec::moment_to_text(at)),
            ),
            Some((BudgetAnswer::LeftStopped, at)) => (
                Some("left_stopped".to_owned()),
                None,
                Some(codec::moment_to_text(at)),
            ),
        };
        Self {
            asked_at: codec::moment_to_text(question.asked_at),
            limit: question.limit.tokens(),
            used: question.used,
            answer,
            raised_by,
            answered_at,
        }
    }

    fn into_question(self, id: &str) -> Result<BudgetQuestion, DatabaseError> {
        let damaged = |reason: &str| {
            DatabaseError::Damaged(format!("the question about the budget {id} {reason}"))
        };
        let answer = match (self.answer.as_deref(), self.answered_at.as_deref()) {
            (None, None) => None,
            (Some("raised"), Some(at)) => Some((
                BudgetAnswer::Raised {
                    by: self.raised_by.ok_or_else(|| damaged("raises by nothing"))?,
                },
                codec::moment_from_text(at)?,
            )),
            (Some("left_stopped"), Some(at)) => {
                Some((BudgetAnswer::LeftStopped, codec::moment_from_text(at)?))
            }
            _ => return Err(damaged("has an answer that is not understood")),
        };
        Ok(BudgetQuestion {
            asked_at: codec::moment_from_text(&self.asked_at)?,
            limit: TokenBudget::new(self.limit)
                .map_err(|reason| codec::damaged_record("budget", id, &reason))?,
            used: self.used,
            answer,
        })
    }
}

/// A budget as the database and the operation log keep it. Its identifier
/// is that of what it is for.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BudgetRecord {
    id: String,
    project_id: String,
    scope: String,
    tokens: Option<u64>,
    question: Option<QuestionRecord>,
}

impl BudgetRecord {
    fn of(budget: &Budget) -> Self {
        Self {
            id: budget.id().to_string(),
            project_id: budget.project_id().to_string(),
            scope: budget.scope().kind().to_string(),
            tokens: budget.tokens().map(TokenBudget::tokens),
            question: budget.question().map(QuestionRecord::of),
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        let tokens: Option<i64> = row.get("tokens")?;
        let question: Option<String> = row.get("question")?;
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            scope: row.get("scope")?,
            tokens: tokens.and_then(|tokens| u64::try_from(tokens).ok()),
            // A question that cannot be read is left out here and refused
            // when the budget is rebuilt.
            question: question.and_then(|text| serde_json::from_str(&text).ok()),
        })
    }

    fn into_budget(self) -> Result<Budget, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("budget", &self.id, &reason);
        let kind: BudgetScopeKind = codec::parse(&self.scope)?;
        let target: uuid::Uuid = self
            .id
            .parse()
            .map_err(|_| DatabaseError::Damaged(format!("{} is not an identifier", self.id)))?;
        let tokens = self
            .tokens
            .map(TokenBudget::new)
            .transpose()
            .map_err(broken)?;
        let question = self
            .question
            .clone()
            .map(|question| question.into_question(&self.id))
            .transpose()?;
        Ok(Budget::restore(
            codec::parse(&self.project_id)?,
            BudgetScope::from_parts(kind, target),
            tokens,
            question,
        ))
    }
}

/// An entry of the record as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryRecord {
    id: String,
    project_id: String,
    at: String,
    kind: String,
    text: String,
    task_id: Option<String>,
    member_id: Option<String>,
    computer: String,
}

impl EntryRecord {
    fn of(entry: &CoordinatorEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            project_id: entry.project_id.to_string(),
            at: codec::moment_to_text(entry.at),
            kind: entry.kind.to_string(),
            text: entry.text.clone(),
            task_id: entry.task_id.map(|id| id.to_string()),
            member_id: entry.member_id.map(|id| id.to_string()),
            computer: entry.computer.to_string(),
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            at: row.get("at")?,
            kind: row.get("kind")?,
            text: row.get("text")?,
            task_id: row.get("task_id")?,
            member_id: row.get("member_id")?,
            computer: row.get("computer")?,
        })
    }

    fn into_entry(self) -> Result<CoordinatorEntry, DatabaseError> {
        Ok(CoordinatorEntry {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            at: codec::moment_from_text(&self.at)?,
            kind: codec::parse(&self.kind)?,
            text: self.text,
            task_id: self.task_id.as_deref().map(codec::parse).transpose()?,
            member_id: self.member_id.as_deref().map(codec::parse).transpose()?,
            computer: codec::parse(&self.computer)?,
        })
    }
}

/// Stores the coordinators, their budgets and their records in the SQLite
/// database.
#[derive(Debug, Clone)]
pub struct SqliteCoordinatorRepository {
    database: Arc<Database>,
}

impl SqliteCoordinatorRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    fn entries_where(
        &self,
        project_id: ProjectId,
        since: Option<Timestamp>,
        most: Option<usize>,
    ) -> Result<Vec<CoordinatorEntry>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {ENTRY_COLUMNS} FROM coordinator_entries
                 WHERE project_id = :project_id AND at >= :since
                 ORDER BY at DESC, id DESC LIMIT :most"
            );
            let records = connection
                .prepare(&query)?
                .query_map(
                    named_params! {
                        ":project_id": project_id.to_string(),
                        ":since": since.map(codec::moment_to_text).unwrap_or_default(),
                        ":most": most.map_or(-1, |most| i64::try_from(most).unwrap_or(i64::MAX)),
                    },
                    EntryRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(EntryRecord::into_entry).collect()
        })
    }
}

impl CoordinatorRepository for SqliteCoordinatorRepository {
    fn coordinator_of(&self, project_id: ProjectId) -> Result<Option<Coordinator>, StorageError> {
        self.database.read(|connection| {
            connection
                .query_row(
                    "SELECT id, project_id, concurrency_limit, runs_on, switched_at
                     FROM coordinators WHERE id = :id",
                    named_params! { ":id": project_id.to_string() },
                    CoordinatorRecord::from_row,
                )
                .optional()?
                .map(CoordinatorRecord::into_coordinator)
                .transpose()
        })
    }

    fn save_coordinator(&self, coordinator: &Coordinator) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let record = CoordinatorRecord::of(coordinator);
            write_coordinator_row(transaction, &record)?;
            operations.record_upsert(Entity::Coordinator, &record.id, &record)
        })
    }

    fn budgets_of_project(&self, project_id: ProjectId) -> Result<Vec<Budget>, StorageError> {
        self.database.read(|connection| {
            let records = connection
                .prepare(
                    "SELECT id, project_id, scope, tokens, question FROM budgets
                     WHERE project_id = :project_id ORDER BY id",
                )?
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    BudgetRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(BudgetRecord::into_budget).collect()
        })
    }

    fn save_budget(&self, budget: &Budget) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let record = BudgetRecord::of(budget);
            write_budget_row(transaction, &record)?;
            operations.record_upsert(Entity::Budget, &record.id, &record)
        })
    }

    fn latest_entries(
        &self,
        project_id: ProjectId,
        most: usize,
    ) -> Result<Vec<CoordinatorEntry>, StorageError> {
        self.entries_where(project_id, None, Some(most))
    }

    fn entries_since(
        &self,
        project_id: ProjectId,
        since: Timestamp,
    ) -> Result<Vec<CoordinatorEntry>, StorageError> {
        self.entries_where(project_id, Some(since), None)
    }

    fn add_entries(&self, entries: &[CoordinatorEntry]) -> Result<(), StorageError> {
        if entries.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for entry in entries {
                let record = EntryRecord::of(entry);
                write_entry_row(transaction, &record)?;
                operations.record_upsert(Entity::CoordinatorEntry, &record.id, &record)?;
            }
            Ok(())
        })
    }
}

fn write_coordinator_row(
    transaction: &Transaction<'_>,
    record: &CoordinatorRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_COORDINATOR,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":concurrency_limit": record.concurrency_limit,
            ":runs_on": record.runs_on,
            ":switched_at": record.switched_at,
        },
    )?;
    Ok(())
}

fn write_budget_row(
    transaction: &Transaction<'_>,
    record: &BudgetRecord,
) -> Result<(), DatabaseError> {
    let question = record
        .question
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| DatabaseError::Damaged(error.to_string()))?;
    transaction.execute(
        UPSERT_BUDGET,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":scope": record.scope,
            ":tokens": record.tokens.map(|tokens| i64::try_from(tokens).unwrap_or(i64::MAX)),
            ":question": question,
        },
    )?;
    Ok(())
}

fn write_entry_row(
    transaction: &Transaction<'_>,
    record: &EntryRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_ENTRY,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":at": record.at,
            ":kind": record.kind,
            ":text": record.text,
            ":task_id": record.task_id,
            ":member_id": record.member_id,
            ":computer": record.computer,
        },
    )?;
    Ok(())
}

/// Writes the row of a coordinator from a state the log keeps, without
/// logging anything. A state that breaks a rule is refused.
pub(crate) fn write_coordinator_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: CoordinatorRecord = codec::record_from_state("coordinator", state)?;
    let record = CoordinatorRecord::of(&record.into_coordinator()?);
    write_coordinator_row(transaction, &record)
}

/// Writes the row of a budget from a state the log keeps, without logging
/// anything. A state that breaks a rule is refused.
pub(crate) fn write_budget_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: BudgetRecord = codec::record_from_state("budget", state)?;
    let record = BudgetRecord::of(&record.into_budget()?);
    write_budget_row(transaction, &record)
}

/// Writes the row of an entry from a state the log keeps, without logging
/// anything.
pub(crate) fn write_entry_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: EntryRecord = codec::record_from_state("coordinator entry", state)?;
    let record = EntryRecord::of(&record.into_entry()?);
    write_entry_row(transaction, &record)
}

/// Removes the row of one of the three, without logging it.
pub(crate) fn remove_row(
    transaction: &Transaction<'_>,
    entity: Entity,
    id: &str,
) -> Result<(), DatabaseError> {
    let table = match entity {
        Entity::Coordinator => "coordinators",
        Entity::Budget => "budgets",
        Entity::CoordinatorEntry => "coordinator_entries",
        _ => return Ok(()),
    };
    transaction.execute(
        &format!("DELETE FROM {table} WHERE id = :id"),
        named_params! { ":id": id },
    )?;
    Ok(())
}
