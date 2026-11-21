//! Stores the permission requests of the specialists, with the manager's
//! answers (decision 0038).

use std::sync::Arc;

use rusqlite::{Connection, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{PermissionRepository, StorageError};
use teamwright_domain::{
    ComputerId, DomainError, PermissionAnswer, PermissionDecision, PermissionKind,
    PermissionRequest, PermissionRequestId, PermissionRequestSnapshot, PermissionRule,
    PermissionScope, ProjectId, SaidIn,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::Entity;

/// The columns of the `permission_requests` table that every query reads.
const COLUMNS: &str = "id, project_id, member_id, conversation_id, turn_id, task_id, kind, what, \
                       reason, asked_at, decision, scope, allowed_kind, allowed_what, note, \
                       answered_at, answered_on, next_turn_id, withdrawn_at";

/// The decision of an allowed request, as stored.
const ALLOWED: &str = "allowed";

/// The decision of a refused request, as stored.
const REFUSED: &str = "refused";

/// The kind of a rule that was a prefix of commands, as stored for one day
/// before prefixes were taken out (decision 0038, note of 21 November
/// 2026). A request of that kind is not read back.
const RETIRED_PREFIX: &str = "command_prefix";

// The rows are updated in place, never replaced.
const UPSERT_REQUEST: &str = "
    INSERT INTO permission_requests (
        id, project_id, member_id, conversation_id, turn_id, task_id, kind, what, reason,
        asked_at, decision, scope, allowed_kind, allowed_what, note, answered_at, answered_on,
        next_turn_id, withdrawn_at
    )
    VALUES (
        :id, :project_id, :member_id, :conversation_id, :turn_id, :task_id, :kind, :what,
        :reason, :asked_at, :decision, :scope, :allowed_kind, :allowed_what, :note,
        :answered_at, :answered_on, :next_turn_id, :withdrawn_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        member_id = excluded.member_id,
        conversation_id = excluded.conversation_id,
        turn_id = excluded.turn_id,
        task_id = excluded.task_id,
        kind = excluded.kind,
        what = excluded.what,
        reason = excluded.reason,
        asked_at = excluded.asked_at,
        decision = excluded.decision,
        scope = excluded.scope,
        allowed_kind = excluded.allowed_kind,
        allowed_what = excluded.allowed_what,
        note = excluded.note,
        answered_at = excluded.answered_at,
        answered_on = excluded.answered_on,
        next_turn_id = excluded.next_turn_id,
        withdrawn_at = excluded.withdrawn_at";

/// A request as the database and the operation log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RequestRecord {
    id: String,
    project_id: String,
    member_id: String,
    conversation_id: String,
    turn_id: String,
    task_id: Option<String>,
    kind: String,
    what: String,
    reason: String,
    asked_at: String,
    decision: Option<String>,
    scope: Option<String>,
    allowed_kind: Option<String>,
    allowed_what: Option<String>,
    note: Option<String>,
    answered_at: Option<String>,
    answered_on: Option<String>,
    next_turn_id: Option<String>,
    withdrawn_at: Option<String>,
}

impl RequestRecord {
    /// Takes the stored form of a request.
    fn of(request: &PermissionRequest) -> Self {
        let said_in = request.said_in();
        let answer = request.answer();
        let allowed = answer.and_then(|answer| match &answer.decision {
            PermissionDecision::Allowed { scope, rule } => Some((*scope, rule)),
            PermissionDecision::Refused => None,
        });
        Self {
            id: request.id().to_string(),
            project_id: request.project_id().to_string(),
            member_id: said_in.member_id.to_string(),
            conversation_id: said_in.conversation_id.to_string(),
            turn_id: said_in.turn_id.to_string(),
            task_id: request.task_id().map(|id| id.to_string()),
            kind: request.asked().kind().to_string(),
            what: request.asked().text().to_owned(),
            reason: request.reason().to_owned(),
            asked_at: codec::moment_to_text(request.asked_at()),
            decision: answer.map(|answer| {
                match answer.decision {
                    PermissionDecision::Allowed { .. } => ALLOWED,
                    PermissionDecision::Refused => REFUSED,
                }
                .to_owned()
            }),
            scope: allowed.map(|(scope, _)| scope.to_string()),
            allowed_kind: allowed.map(|(_, rule)| rule.kind().to_string()),
            allowed_what: allowed.map(|(_, rule)| rule.text().to_owned()),
            note: answer.map(|answer| answer.note.clone()),
            answered_at: answer.map(|answer| codec::moment_to_text(answer.at)),
            answered_on: answer.map(|answer| answer.computer.to_string()),
            next_turn_id: answer.map(|answer| answer.next_turn.to_string()),
            withdrawn_at: answer
                .and_then(|answer| answer.withdrawn_at)
                .map(codec::moment_to_text),
        }
    }

    /// Reads the columns of a `permission_requests` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            member_id: row.get("member_id")?,
            conversation_id: row.get("conversation_id")?,
            turn_id: row.get("turn_id")?,
            task_id: row.get("task_id")?,
            kind: row.get("kind")?,
            what: row.get("what")?,
            reason: row.get("reason")?,
            asked_at: row.get("asked_at")?,
            decision: row.get("decision")?,
            scope: row.get("scope")?,
            allowed_kind: row.get("allowed_kind")?,
            allowed_what: row.get("allowed_what")?,
            note: row.get("note")?,
            answered_at: row.get("answered_at")?,
            answered_on: row.get("answered_on")?,
            next_turn_id: row.get("next_turn_id")?,
            withdrawn_at: row.get("withdrawn_at")?,
        })
    }

    /// Rebuilds the request, checking the stored values against the rules:
    /// a rule that could never be allowed is not read back as allowed.
    ///
    /// A request whose rule can no longer be made, because it is a prefix
    /// of commands or a command the stricter rules refuse, is not read back
    /// at all: it grants nothing, and it is not shown. `answered_here` says
    /// whether this computer's own log wrote the answer
    /// ([`answered_here`]).
    fn into_request(self, answered_here: bool) -> Result<Option<PermissionRequest>, DatabaseError> {
        let Some(asked) = self.rule(&self.kind, &self.what)? else {
            return Ok(None);
        };
        let answer = match self.decision.as_deref() {
            None => None,
            Some(decision) => {
                let decision = match decision {
                    ALLOWED => {
                        let (Some(scope), Some(kind), Some(what)) =
                            (&self.scope, &self.allowed_kind, &self.allowed_what)
                        else {
                            return Err(self.damaged("is allowed without saying what"));
                        };
                        let scope: PermissionScope = codec::parse(scope)?;
                        let Some(rule) = self.rule(kind, what)? else {
                            return Ok(None);
                        };
                        PermissionDecision::Allowed { scope, rule }
                    }
                    REFUSED => PermissionDecision::Refused,
                    _ => return Err(self.damaged("has an unknown decision")),
                };
                let (Some(at), Some(computer), Some(next_turn)) =
                    (&self.answered_at, &self.answered_on, &self.next_turn_id)
                else {
                    return Err(self.damaged("is answered without saying when and where"));
                };
                Some(PermissionAnswer {
                    decision,
                    note: self.note.clone().unwrap_or_default(),
                    at: codec::moment_from_text(at)?,
                    computer: codec::parse(computer)?,
                    next_turn: codec::parse(next_turn)?,
                    withdrawn_at: self
                        .withdrawn_at
                        .as_deref()
                        .map(codec::moment_from_text)
                        .transpose()?,
                    answered_here,
                })
            }
        };
        Ok(Some(PermissionRequest::restore(
            PermissionRequestSnapshot {
                id: codec::parse(&self.id)?,
                project_id: codec::parse(&self.project_id)?,
                said_in: SaidIn {
                    member_id: codec::parse(&self.member_id)?,
                    conversation_id: codec::parse(&self.conversation_id)?,
                    turn_id: codec::parse(&self.turn_id)?,
                },
                task_id: self.task_id.as_deref().map(codec::parse).transpose()?,
                asked,
                reason: self.reason.clone(),
                asked_at: codec::moment_from_text(&self.asked_at)?,
                answer,
            },
        )))
    }

    /// A stored rule, or nothing if it can no longer be made.
    fn rule(&self, kind: &str, what: &str) -> Result<Option<PermissionRule>, DatabaseError> {
        if kind == RETIRED_PREFIX {
            return Ok(None);
        }
        let kind: PermissionKind = codec::parse(kind)?;
        match PermissionRule::new(kind, what) {
            Ok(rule) => Ok(Some(rule)),
            Err(DomainError::PermissionNotPossible { .. }) => Ok(None),
            Err(broken) => Err(codec::damaged_record(
                "permission request",
                &self.id,
                &broken,
            )),
        }
    }

    fn damaged(&self, what: &str) -> DatabaseError {
        DatabaseError::Damaged(format!("the permission request {} {what}", self.id))
    }
}

/// Stores the permission requests in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqlitePermissionRepository {
    database: Arc<Database>,
}

impl SqlitePermissionRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl PermissionRepository for SqlitePermissionRepository {
    fn save_request(&self, request: &PermissionRequest) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let record = RequestRecord::of(request);
            write_request_row(transaction, &record)?;
            operations.record_upsert(Entity::PermissionRequest, &record.id, &record)
        })
    }

    fn requests_of_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<PermissionRequest>, StorageError> {
        let this_computer = self.database.device_id();
        self.database.read(|connection| {
            let query = format!(
                "SELECT {COLUMNS} FROM permission_requests WHERE project_id = :project_id
                 ORDER BY asked_at, id"
            );
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    RequestRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            let mut requests = Vec::with_capacity(records.len());
            for record in records {
                let here = answered_here(connection, &record.id, this_computer)?;
                requests.extend(record.into_request(here)?);
            }
            Ok(requests)
        })
    }

    fn find_request(
        &self,
        id: PermissionRequestId,
    ) -> Result<Option<PermissionRequest>, StorageError> {
        let this_computer = self.database.device_id();
        self.database.read(|connection| {
            let query = format!("SELECT {COLUMNS} FROM permission_requests WHERE id = :id");
            let mut statement = connection.prepare(&query)?;
            let mut records = statement
                .query_map(
                    named_params! { ":id": id.to_string() },
                    RequestRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            let Some(record) = records.pop() else {
                return Ok(None);
            };
            let here = answered_here(connection, &record.id, this_computer)?;
            record.into_request(here)
        })
    }
}

/// Removes the row of a request, without logging it, and says whether
/// there was one.
pub(crate) fn remove_request(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM permission_requests WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Writes the row of a request from a state the log keeps, without logging
/// anything. A state that breaks a rule is refused, and nothing is written.
pub(crate) fn write_request_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: RequestRecord = codec::record_from_state("permission request", state)?;
    // Whether it was answered here is not part of the row.
    match record.into_request(false)? {
        Some(request) => write_request_row(transaction, &RequestRecord::of(&request)),
        // A rule that can no longer be made, written by a computer that
        // still knew it, is not kept.
        None => Ok(()),
    }
}

/// Whether the answer to a request was written by this computer itself, as
/// its own log of operations shows: every state of the request that
/// carries an answer was made here. An answer is written once, by the
/// computer the manager answered on; one that came through the sync folder
/// was made by another computer, whatever computer it names. Anybody who
/// can write in the sync folder can write such an answer, though never in
/// the folder of this computer, which this computer does not read.
fn answered_here(
    connection: &Connection,
    id: &str,
    this_computer: ComputerId,
) -> Result<bool, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT payload, device_id FROM operations
         WHERE entity = :entity AND entity_id = :id AND payload IS NOT NULL
           AND waiting IS NULL",
    )?;
    let states = statement
        .query_map(
            named_params! { ":entity": Entity::PermissionRequest.as_str(), ":id": id },
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?;
    let here = this_computer.to_string();
    let mut answered = false;
    for (payload, computer) in states {
        let state: Value = serde_json::from_str(&payload).map_err(|error| {
            DatabaseError::Damaged(format!(
                "a state of the permission request {id} cannot be read: {error}"
            ))
        })?;
        if state
            .get("decision")
            .is_some_and(|decision| !decision.is_null())
        {
            if computer != here {
                return Ok(false);
            }
            answered = true;
        }
    }
    Ok(answered)
}

/// Writes the row of a request.
fn write_request_row(
    transaction: &Transaction<'_>,
    record: &RequestRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_REQUEST,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":member_id": record.member_id,
            ":conversation_id": record.conversation_id,
            ":turn_id": record.turn_id,
            ":task_id": record.task_id,
            ":kind": record.kind,
            ":what": record.what,
            ":reason": record.reason,
            ":asked_at": record.asked_at,
            ":decision": record.decision,
            ":scope": record.scope,
            ":allowed_kind": record.allowed_kind,
            ":allowed_what": record.allowed_what,
            ":note": record.note,
            ":answered_at": record.answered_at,
            ":answered_on": record.answered_on,
            ":next_turn_id": record.next_turn_id,
            ":withdrawn_at": record.withdrawn_at,
        },
    )?;
    Ok(())
}
