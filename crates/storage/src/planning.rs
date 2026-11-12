//! Stores what the Advisor asked and proposed in a turn: its interview,
//! with the questions, their options and the answers, and its proposal,
//! with the members, the tasks and what each task waits for.
//!
//! Both are kept in rows of their own, keyed by the turn, and are written
//! only when they differ from what is stored. Each is logged as one entity
//! (`interview`, `proposal`) whose state is the whole of it.

use rusqlite::{Connection, OptionalExtension, Transaction, named_params};
use serde::Serialize;
use teamwright_domain::{
    DecisionOption, DomainError, Interview, InterviewAnswer, InterviewQuestion, InterviewSnapshot,
    MemberName, ModelName, Proposal, ProposalSnapshot, ProposedMember, ProposedOwner, ProposedTask,
    TaskTitle, Turn,
};

use crate::codec;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

/// The text form of an owner who is the manager.
const OWNER_MANAGER: &str = "manager";
/// The text form of an owner who is a member, named in `owner_name`.
const OWNER_MEMBER: &str = "member";

/// The interview of a turn, as the database and the operation log keep it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct InterviewRecord {
    turn_id: String,
    questions: Vec<QuestionRecord>,
    answered_at: Option<String>,
}

/// One question of an interview, with its answer once there is one.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct QuestionRecord {
    question: String,
    reason: String,
    recommended: Option<i64>,
    options: Vec<OptionRecord>,
    answered_option: Option<i64>,
    answer_note: Option<String>,
}

/// One option of a question.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct OptionRecord {
    label: String,
    consequence: String,
}

impl InterviewRecord {
    /// Takes the stored form of the interview of a turn, if it asked one.
    pub(crate) fn of(turn: &Turn) -> Option<Self> {
        let interview = turn.interview()?;
        let answers = interview.answers();
        Some(Self {
            turn_id: turn.id().to_string(),
            questions: interview
                .questions()
                .iter()
                .enumerate()
                .map(|(place, question)| {
                    let answer = answers.and_then(|answers| answers.get(place));
                    QuestionRecord {
                        question: question.question.clone(),
                        reason: question.reason.clone(),
                        recommended: question.recommended.map(stored_position),
                        options: question
                            .options
                            .iter()
                            .map(|option| OptionRecord {
                                label: option.label.clone(),
                                consequence: option.consequence.clone(),
                            })
                            .collect(),
                        answered_option: answer
                            .and_then(|answer| answer.option)
                            .map(stored_position),
                        answer_note: answer.map(|answer| answer.note.clone()),
                    }
                })
                .collect(),
            answered_at: interview.answered_at().map(codec::moment_to_text),
        })
    }

    /// Rebuilds the interview, checking that the answers are whole.
    pub(crate) fn into_interview(self) -> Result<Interview, DatabaseError> {
        let damaged = || {
            DatabaseError::Damaged(format!(
                "the interview of the turn {} has some of its answers but not all",
                self.turn_id
            ))
        };
        let answered = self.answered_at.is_some();
        let mut questions = Vec::new();
        let mut answers = Vec::new();
        for record in &self.questions {
            match (&record.answer_note, answered) {
                (Some(note), true) => answers.push(InterviewAnswer {
                    option: record
                        .answered_option
                        .map(position_from_store)
                        .transpose()?,
                    note: note.clone(),
                }),
                (None, false) if record.answered_option.is_none() => {}
                _ => return Err(damaged()),
            }
            questions.push(InterviewQuestion {
                question: record.question.clone(),
                reason: record.reason.clone(),
                options: record
                    .options
                    .iter()
                    .map(|option| DecisionOption {
                        label: option.label.clone(),
                        consequence: option.consequence.clone(),
                    })
                    .collect(),
                recommended: record.recommended.map(position_from_store).transpose()?,
            });
        }
        Ok(Interview::restore(InterviewSnapshot {
            questions,
            answers: answered.then_some(answers),
            answered_at: self
                .answered_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        }))
    }
}

/// The proposal of a turn, as the database and the operation log keep it.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ProposalRecord {
    turn_id: String,
    summary: String,
    members: Vec<ProposedMemberRecord>,
    tasks: Vec<ProposedTaskRecord>,
    approved_at: Option<String>,
}

/// One member of a proposal.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProposedMemberRecord {
    name: String,
    responsibility: String,
    reason: String,
    model: Option<String>,
    effort: Option<String>,
}

/// One task of a proposal.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct ProposedTaskRecord {
    title: String,
    description: String,
    owner_kind: Option<String>,
    owner_name: Option<String>,
    estimate_minutes: Option<i64>,
    priority: String,
    waits_for: Vec<i64>,
}

impl ProposalRecord {
    /// Takes the stored form of the proposal of a turn, if it made one.
    pub(crate) fn of(turn: &Turn) -> Option<Self> {
        let proposal = turn.proposal()?;
        Some(Self {
            turn_id: turn.id().to_string(),
            summary: proposal.summary().to_owned(),
            members: proposal
                .members()
                .iter()
                .map(|member| ProposedMemberRecord {
                    name: member.name.to_string(),
                    responsibility: member.responsibility.clone(),
                    reason: member.reason.clone(),
                    model: member.model.as_ref().map(ToString::to_string),
                    effort: member.effort.map(|effort| effort.as_str().to_owned()),
                })
                .collect(),
            tasks: proposal
                .tasks()
                .iter()
                .map(|task| {
                    let (owner_kind, owner_name) = match &task.owner {
                        None => (None, None),
                        Some(ProposedOwner::Manager) => (Some(OWNER_MANAGER.to_owned()), None),
                        Some(ProposedOwner::Member(name)) => {
                            (Some(OWNER_MEMBER.to_owned()), Some(name.to_string()))
                        }
                    };
                    ProposedTaskRecord {
                        title: task.title.to_string(),
                        description: task.description.clone(),
                        owner_kind,
                        owner_name,
                        estimate_minutes: task.estimate_minutes.map(i64::from),
                        priority: task.priority.as_str().to_owned(),
                        waits_for: task
                            .waits_for
                            .iter()
                            .copied()
                            .map(stored_position)
                            .collect(),
                    }
                })
                .collect(),
            approved_at: proposal.approved_at().map(codec::moment_to_text),
        })
    }

    /// Rebuilds the proposal, checking names, titles and the other values
    /// against the rules.
    pub(crate) fn into_proposal(self) -> Result<Proposal, DatabaseError> {
        let broken = |reason: DomainError| {
            codec::damaged_record("proposal of the turn", &self.turn_id, &reason)
        };
        let members = self
            .members
            .iter()
            .map(|member| {
                Ok(ProposedMember {
                    name: MemberName::parse(&member.name).map_err(broken)?,
                    responsibility: member.responsibility.clone(),
                    reason: member.reason.clone(),
                    model: member
                        .model
                        .as_deref()
                        .map(ModelName::parse)
                        .transpose()
                        .map_err(broken)?,
                    effort: member.effort.as_deref().map(codec::parse).transpose()?,
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;
        let tasks = self
            .tasks
            .iter()
            .map(|task| {
                let owner = match (task.owner_kind.as_deref(), task.owner_name.as_deref()) {
                    (None, None) => None,
                    (Some(OWNER_MANAGER), None) => Some(ProposedOwner::Manager),
                    (Some(OWNER_MEMBER), Some(name)) => Some(ProposedOwner::Member(
                        MemberName::parse(name).map_err(broken)?,
                    )),
                    _ => {
                        return Err(DatabaseError::Damaged(format!(
                            "a task of the proposal of the turn {} has an owner that cannot be read",
                            self.turn_id
                        )));
                    }
                };
                Ok(ProposedTask {
                    title: TaskTitle::parse(&task.title).map_err(broken)?,
                    description: task.description.clone(),
                    owner,
                    estimate_minutes: task
                        .estimate_minutes
                        .map(|minutes| {
                            u32::try_from(minutes).map_err(|reason| {
                                DatabaseError::damaged("the estimate", &minutes.to_string(), reason)
                            })
                        })
                        .transpose()?,
                    priority: codec::parse(&task.priority)?,
                    waits_for: task
                        .waits_for
                        .iter()
                        .map(|&place| position_from_store(place))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;
        Ok(Proposal::restore(ProposalSnapshot {
            summary: self.summary,
            members,
            tasks,
            approved_at: self
                .approved_at
                .as_deref()
                .map(codec::moment_from_text)
                .transpose()?,
        }))
    }
}

/// Writes and logs the interview and the proposal of a turn, each only if
/// it differs from what is stored. A turn never loses either, so there is
/// nothing to remove.
pub(crate) fn store_planning_of(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    turn: &Turn,
) -> Result<(), DatabaseError> {
    let turn_id = turn.id().to_string();
    if let Some(interview) = InterviewRecord::of(turn)
        && stored_interview(transaction, &turn_id)?.as_ref() != Some(&interview)
    {
        write_interview(transaction, &interview)?;
        operations.record_upsert(Entity::Interview, &interview.turn_id, &interview)?;
    }
    if let Some(proposal) = ProposalRecord::of(turn)
        && stored_proposal(transaction, &turn_id)?.as_ref() != Some(&proposal)
    {
        write_proposal(transaction, &proposal)?;
        operations.record_upsert(Entity::Proposal, &proposal.turn_id, &proposal)?;
    }
    Ok(())
}

/// Writes an interview with its questions and their options.
fn write_interview(
    transaction: &Transaction<'_>,
    record: &InterviewRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO turn_interviews (turn_id, answered_at) VALUES (:turn_id, :answered_at)
         ON CONFLICT (turn_id) DO UPDATE SET answered_at = excluded.answered_at",
        named_params! { ":turn_id": record.turn_id, ":answered_at": record.answered_at },
    )?;
    for (position, question) in (0_i64..).zip(&record.questions) {
        transaction.execute(
            "INSERT INTO turn_interview_questions (
                 turn_id, position, question, reason, recommended, answered_option, answer_note
             )
             VALUES (
                 :turn_id, :position, :question, :reason, :recommended, :answered_option,
                 :answer_note
             )
             ON CONFLICT (turn_id, position) DO UPDATE SET
                 question = excluded.question,
                 reason = excluded.reason,
                 recommended = excluded.recommended,
                 answered_option = excluded.answered_option,
                 answer_note = excluded.answer_note",
            named_params! {
                ":turn_id": record.turn_id,
                ":position": position,
                ":question": question.question,
                ":reason": question.reason,
                ":recommended": question.recommended,
                ":answered_option": question.answered_option,
                ":answer_note": question.answer_note,
            },
        )?;
        for (option_position, option) in (0_i64..).zip(&question.options) {
            transaction.execute(
                "INSERT INTO turn_interview_options (
                     turn_id, question_position, position, label, consequence
                 )
                 VALUES (:turn_id, :question_position, :position, :label, :consequence)
                 ON CONFLICT (turn_id, question_position, position) DO UPDATE SET
                     label = excluded.label,
                     consequence = excluded.consequence",
                named_params! {
                    ":turn_id": record.turn_id,
                    ":question_position": position,
                    ":position": option_position,
                    ":label": option.label,
                    ":consequence": option.consequence,
                },
            )?;
        }
        transaction.execute(
            "DELETE FROM turn_interview_options
             WHERE turn_id = :turn_id AND question_position = :question_position
               AND position >= :count",
            named_params! {
                ":turn_id": record.turn_id,
                ":question_position": position,
                ":count": stored_position(question.options.len()),
            },
        )?;
    }
    transaction.execute(
        "DELETE FROM turn_interview_questions WHERE turn_id = :turn_id AND position >= :count",
        named_params! {
            ":turn_id": record.turn_id,
            ":count": stored_position(record.questions.len()),
        },
    )?;
    Ok(())
}

/// Writes a proposal with its members, its tasks and what they wait for.
fn write_proposal(
    transaction: &Transaction<'_>,
    record: &ProposalRecord,
) -> Result<(), DatabaseError> {
    transaction.execute(
        "INSERT INTO turn_proposals (turn_id, summary, approved_at)
         VALUES (:turn_id, :summary, :approved_at)
         ON CONFLICT (turn_id) DO UPDATE SET
             summary = excluded.summary,
             approved_at = excluded.approved_at",
        named_params! {
            ":turn_id": record.turn_id,
            ":summary": record.summary,
            ":approved_at": record.approved_at,
        },
    )?;
    for (position, member) in (0_i64..).zip(&record.members) {
        transaction.execute(
            "INSERT INTO turn_proposal_members (
                 turn_id, position, name, responsibility, reason, model, effort
             )
             VALUES (:turn_id, :position, :name, :responsibility, :reason, :model, :effort)
             ON CONFLICT (turn_id, position) DO UPDATE SET
                 name = excluded.name,
                 responsibility = excluded.responsibility,
                 reason = excluded.reason,
                 model = excluded.model,
                 effort = excluded.effort",
            named_params! {
                ":turn_id": record.turn_id,
                ":position": position,
                ":name": member.name,
                ":responsibility": member.responsibility,
                ":reason": member.reason,
                ":model": member.model,
                ":effort": member.effort,
            },
        )?;
    }
    transaction.execute(
        "DELETE FROM turn_proposal_members WHERE turn_id = :turn_id AND position >= :count",
        named_params! {
            ":turn_id": record.turn_id,
            ":count": stored_position(record.members.len()),
        },
    )?;
    for (position, task) in (0_i64..).zip(&record.tasks) {
        transaction.execute(
            "INSERT INTO turn_proposal_tasks (
                 turn_id, position, title, description, owner_kind, owner_name,
                 estimate_minutes, priority
             )
             VALUES (
                 :turn_id, :position, :title, :description, :owner_kind, :owner_name,
                 :estimate_minutes, :priority
             )
             ON CONFLICT (turn_id, position) DO UPDATE SET
                 title = excluded.title,
                 description = excluded.description,
                 owner_kind = excluded.owner_kind,
                 owner_name = excluded.owner_name,
                 estimate_minutes = excluded.estimate_minutes,
                 priority = excluded.priority",
            named_params! {
                ":turn_id": record.turn_id,
                ":position": position,
                ":title": task.title,
                ":description": task.description,
                ":owner_kind": task.owner_kind,
                ":owner_name": task.owner_name,
                ":estimate_minutes": task.estimate_minutes,
                ":priority": task.priority,
            },
        )?;
        // The rows of what a task waits for are only keys, so they are
        // written anew, as the dependencies of a task are.
        transaction.execute(
            "DELETE FROM turn_proposal_task_waits
             WHERE turn_id = :turn_id AND task_position = :task_position",
            named_params! { ":turn_id": record.turn_id, ":task_position": position },
        )?;
        for (wait_position, waits_for) in (0_i64..).zip(&task.waits_for) {
            transaction.execute(
                "INSERT INTO turn_proposal_task_waits (turn_id, task_position, position, waits_for)
                 VALUES (:turn_id, :task_position, :position, :waits_for)",
                named_params! {
                    ":turn_id": record.turn_id,
                    ":task_position": position,
                    ":position": wait_position,
                    ":waits_for": waits_for,
                },
            )?;
        }
    }
    transaction.execute(
        "DELETE FROM turn_proposal_tasks WHERE turn_id = :turn_id AND position >= :count",
        named_params! {
            ":turn_id": record.turn_id,
            ":count": stored_position(record.tasks.len()),
        },
    )?;
    Ok(())
}

/// Reads the stored interview of one turn, if it asked one.
pub(crate) fn stored_interview(
    connection: &Connection,
    turn_id: &str,
) -> Result<Option<InterviewRecord>, DatabaseError> {
    let Some(answered_at) = connection
        .query_row(
            "SELECT answered_at FROM turn_interviews WHERE turn_id = :turn_id",
            named_params! { ":turn_id": turn_id },
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
    else {
        return Ok(None);
    };
    let mut questions: Vec<QuestionRecord> = connection
        .prepare(
            "SELECT question, reason, recommended, answered_option, answer_note
             FROM turn_interview_questions WHERE turn_id = :turn_id ORDER BY position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok(QuestionRecord {
                question: row.get("question")?,
                reason: row.get("reason")?,
                recommended: row.get("recommended")?,
                options: Vec::new(),
                answered_option: row.get("answered_option")?,
                answer_note: row.get("answer_note")?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let options: Vec<(i64, OptionRecord)> = connection
        .prepare(
            "SELECT question_position, label, consequence FROM turn_interview_options
             WHERE turn_id = :turn_id ORDER BY question_position, position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok((
                row.get("question_position")?,
                OptionRecord {
                    label: row.get("label")?,
                    consequence: row.get("consequence")?,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    for (question_position, option) in options {
        let question = usize::try_from(question_position)
            .ok()
            .and_then(|place| questions.get_mut(place))
            .ok_or_else(|| {
                DatabaseError::Damaged(format!(
                    "an option of the interview of the turn {turn_id} belongs to no question"
                ))
            })?;
        question.options.push(option);
    }
    Ok(Some(InterviewRecord {
        turn_id: turn_id.to_owned(),
        questions,
        answered_at,
    }))
}

/// Reads the stored proposal of one turn, if it made one.
pub(crate) fn stored_proposal(
    connection: &Connection,
    turn_id: &str,
) -> Result<Option<ProposalRecord>, DatabaseError> {
    let Some((summary, approved_at)) = connection
        .query_row(
            "SELECT summary, approved_at FROM turn_proposals WHERE turn_id = :turn_id",
            named_params! { ":turn_id": turn_id },
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?
    else {
        return Ok(None);
    };
    let members = connection
        .prepare(
            "SELECT name, responsibility, reason, model, effort FROM turn_proposal_members
             WHERE turn_id = :turn_id ORDER BY position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok(ProposedMemberRecord {
                name: row.get("name")?,
                responsibility: row.get("responsibility")?,
                reason: row.get("reason")?,
                model: row.get("model")?,
                effort: row.get("effort")?,
            })
        })?
        .collect::<Result<_, _>>()?;
    let mut tasks: Vec<ProposedTaskRecord> = connection
        .prepare(
            "SELECT title, description, owner_kind, owner_name, estimate_minutes, priority
             FROM turn_proposal_tasks WHERE turn_id = :turn_id ORDER BY position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok(ProposedTaskRecord {
                title: row.get("title")?,
                description: row.get("description")?,
                owner_kind: row.get("owner_kind")?,
                owner_name: row.get("owner_name")?,
                estimate_minutes: row.get("estimate_minutes")?,
                priority: row.get("priority")?,
                waits_for: Vec::new(),
            })
        })?
        .collect::<Result<_, _>>()?;
    let waits: Vec<(i64, i64)> = connection
        .prepare(
            "SELECT task_position, waits_for FROM turn_proposal_task_waits
             WHERE turn_id = :turn_id ORDER BY task_position, position",
        )?
        .query_map(named_params! { ":turn_id": turn_id }, |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<_, _>>()?;
    for (task_position, waits_for) in waits {
        let task = usize::try_from(task_position)
            .ok()
            .and_then(|place| tasks.get_mut(place))
            .ok_or_else(|| {
                DatabaseError::Damaged(format!(
                    "a task of the proposal of the turn {turn_id} that waits is not there"
                ))
            })?;
        task.waits_for.push(waits_for);
    }
    Ok(Some(ProposalRecord {
        turn_id: turn_id.to_owned(),
        summary,
        members,
        tasks,
        approved_at,
    }))
}

/// A place in a list as SQLite keeps it. Lists here are short, so the
/// place always fits.
fn stored_position(position: usize) -> i64 {
    i64::try_from(position).unwrap_or(i64::MAX)
}

/// Reads a place in a list that SQLite keeps.
fn position_from_store(position: i64) -> Result<usize, DatabaseError> {
    usize::try_from(position)
        .map_err(|reason| DatabaseError::damaged("the place", &position.to_string(), reason))
}
