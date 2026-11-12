//! Writes the rows of a thing from a state the log keeps, and removes them,
//! without logging the thing itself.
//!
//! This is how an operation taken in from another computer reaches the
//! tables: the operation is already in the log, so writing its rows must not
//! log it a second time. Each repository's store function is split into the
//! half that writes the rows and the half that logs; this module calls the
//! first half. A state is checked against the rules of the domain before
//! anything is written, so a damaged state from elsewhere is refused like
//! damaged data read from disk.

use rusqlite::Transaction;
use serde_json::{Map, Value};

use crate::conversations;
use crate::error::DatabaseError;
use crate::members;
use crate::operations::{Entity, OperationRecorder};
use crate::planning;
use crate::projects;
use crate::sprints;
use crate::tasks;

/// Writes the rows of a thing from its state, without logging anything.
///
/// # Errors
///
/// Returns an error, and writes nothing, if the state cannot be read or
/// breaks a rule of the domain, or if the database refuses the rows (a
/// reference to something that is not there). The folders of a project are
/// never written from elsewhere.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    entity: Entity,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    match entity {
        Entity::Project => projects::write_state(transaction, state),
        Entity::Task => tasks::write_state(transaction, state),
        Entity::Member => members::write_state(transaction, state),
        Entity::Sprint => sprints::write_state(transaction, state),
        Entity::Conversation => conversations::write_conversation_state(transaction, state),
        Entity::Turn => conversations::write_turn_state(transaction, state),
        Entity::TurnStep => conversations::write_step_state(transaction, state),
        Entity::Decision => conversations::write_decision_state(transaction, state),
        Entity::Interview => planning::write_interview_state(transaction, state),
        Entity::Proposal => planning::write_proposal_state(transaction, state),
        Entity::ProjectPlaces => Err(DatabaseError::Damaged(
            "the folders of a project are never taken from another computer".to_owned(),
        )),
    }
}

/// Removes the rows of a thing, without logging its removal.
///
/// What still belongs to the thing and has an entry of its own in the log
/// (the tasks of a project, the turns of a conversation) is removed with a
/// logged deletion of this computer. The parts of a thing that the log keeps
/// inside its state (the statuses of a project) or whose deletion its
/// deletion stands for (the steps of a turn, its question, its interview,
/// its proposal) go with it.
pub(crate) fn remove(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    entity: Entity,
    entity_id: &str,
) -> Result<(), DatabaseError> {
    match entity {
        Entity::Project => {
            projects::remove_project(transaction, operations, entity_id)?;
        }
        Entity::Task => {
            tasks::remove_task(transaction, entity_id)?;
        }
        Entity::Member => {
            members::remove_member(transaction, entity_id)?;
        }
        Entity::Sprint => {
            sprints::remove_sprint(transaction, entity_id)?;
        }
        Entity::Conversation => {
            conversations::remove_conversation(transaction, operations, entity_id)?;
        }
        Entity::Turn => {
            conversations::remove_turn(transaction, entity_id)?;
        }
        Entity::TurnStep => conversations::remove_step(transaction, entity_id)?,
        // A question, an interview and a proposal are never removed on
        // their own: they go with their turn.
        Entity::Decision | Entity::Interview | Entity::Proposal | Entity::ProjectPlaces => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;
    use rusqlite::Connection;
    use rusqlite::types::Value as Cell;
    use teamwright_application::{
        ConversationRepository, MemberRepository, ProjectRepository, SprintRepository,
        TaskRepository,
    };
    use teamwright_domain::{
        Assignee, Color, Conversation, DecisionOption, DecisionRequest, Interview,
        InterviewQuestion, Member, MemberKind, MemberName, Priority, Project, ProjectKey,
        ProjectName, Proposal, ProposedTask, Rank, Schedule, Sprint, SprintName, StoryPoints, Task,
        TaskDraft, TaskTitle, Timestamp, TokenCounts, Turn, TurnOrigin, Workflow,
    };

    use super::*;
    use crate::conversations::SqliteConversationRepository;
    use crate::database::Database;
    use crate::members::SqliteMemberRepository;
    use crate::merge::object_from_text;
    use crate::projects::SqliteProjectRepository;
    use crate::sprints::SqliteSprintRepository;
    use crate::tasks::SqliteTaskRepository;

    /// The tables that hold the data the manager works with.
    const TABLES: &[&str] = &[
        "projects",
        "statuses",
        "tasks",
        "task_dependencies",
        "members",
        "sprints",
        "conversations",
        "turns",
        "turn_steps",
        "turn_decisions",
        "turn_decision_options",
        "turn_interviews",
        "turn_interview_questions",
        "turn_interview_options",
        "turn_proposals",
        "turn_proposal_members",
        "turn_proposal_tasks",
        "turn_proposal_task_waits",
    ];

    fn moment(minutes: i64) -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
            + chrono::TimeDelta::minutes(minutes)
    }

    /// Every row of the tables of the data, as text, sorted.
    fn rows_of(database: &Database) -> Vec<String> {
        database
            .read(|connection: &Connection| {
                let mut rows = Vec::new();
                for table in TABLES {
                    let mut statement = connection.prepare(&format!("SELECT * FROM {table}"))?;
                    let columns = statement.column_count();
                    let mut query = statement.query([])?;
                    while let Some(row) = query.next()? {
                        let cells: Vec<String> = (0..columns)
                            .map(|column| format!("{:?}", row.get::<_, Cell>(column).unwrap()))
                            .collect();
                        rows.push(format!("{table}: {}", cells.join(" | ")));
                    }
                }
                rows.sort();
                Ok(rows)
            })
            .unwrap()
    }

    /// A database with one of every thing the manager works with, written
    /// through the repositories: a project, a member, a sprint, two tasks
    /// (one waits for the other, is a part of it, is planned into the
    /// sprint and given to the member), a conversation, and turns with
    /// steps, a question, an interview and a proposal. One step was written
    /// and removed again.
    fn database_with_everything() -> Arc<Database> {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let name = ProjectName::parse("Smart sensor node").unwrap();
        let mut project = Project::create(
            name.clone(),
            ProjectKey::suggest(&name),
            String::new(),
            Workflow::standard(),
            moment(0),
        );
        let first = project.allocate_task_number();
        let second = project.allocate_task_number();
        SqliteProjectRepository::new(database.clone())
            .save(&project)
            .unwrap();
        let member = Member::create(
            project.id(),
            MemberName::parse("Hardware engineer").unwrap(),
            MemberKind::Agent,
            "The board.".to_owned(),
            Color::Teal,
            moment(1),
        );
        SqliteMemberRepository::new(database.clone())
            .save(&member)
            .unwrap();
        let sprint = Sprint::plan(
            project.id(),
            SprintName::parse("Sprint 1").unwrap(),
            "",
            teamwright_domain::SprintDates::new(None, None).unwrap(),
            moment(2),
        );
        SqliteSprintRepository::new(database.clone())
            .save(&sprint, &[])
            .unwrap();
        let status = project.workflow().initial_status().clone();
        let board = Task::create(
            project.id(),
            first,
            &status,
            Rank::between(None, None),
            TaskDraft {
                title: TaskTitle::parse("Design the board").unwrap(),
                description: "Two layers.".to_owned(),
                priority: Priority::High,
                assignee: Some(Assignee::Member(member.id())),
                parent_id: None,
                estimate_minutes: Some(240),
                schedule: Schedule::unscheduled(),
            },
            moment(3),
        );
        let mut power = Task::create(
            project.id(),
            second,
            &status,
            Rank::between(Some(board.rank()), None),
            TaskDraft {
                title: TaskTitle::parse("Power supply").unwrap(),
                description: String::new(),
                priority: Priority::Normal,
                assignee: Some(Assignee::Manager),
                parent_id: Some(board.id()),
                estimate_minutes: None,
                schedule: Schedule::unscheduled(),
            },
            moment(3),
        );
        power.block_on(board.id(), moment(4)).unwrap();
        power.plan_into(Some(sprint.id()), moment(4));
        power.set_points(Some(StoryPoints::new(3).unwrap()), moment(4));
        SqliteTaskRepository::new(database.clone())
            .save_all(&[board.clone(), power])
            .unwrap();

        let conversations = SqliteConversationRepository::new(database.clone());
        let conversation = Conversation::start(
            project.id(),
            member.id(),
            Some(board.id()),
            moment(5),
            database.device_id(),
        );
        conversations.save_conversation(&conversation).unwrap();
        let mut asking =
            Turn::start(conversation.id(), "Draw it.", TurnOrigin::Brief, moment(5)).unwrap();
        asking.notice("Starting.".to_owned(), moment(5)).unwrap();
        asking
            .say("Which regulator?".to_owned(), moment(6))
            .unwrap();
        asking
            .ask(
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
                    Some(0),
                )
                .unwrap(),
            )
            .unwrap();
        conversations.save_turn(&asking).unwrap();
        // The last text of a finished turn becomes its answer: its step is
        // removed again.
        asking
            .finish("Which regulator?", TokenCounts::default(), moment(7))
            .unwrap();
        conversations.save_turn(&asking).unwrap();

        let advisor = Conversation::start_with_advisor(
            project.id(),
            None,
            None,
            moment(8),
            database.device_id(),
        );
        conversations.save_conversation(&advisor).unwrap();
        let mut planning =
            Turn::start(advisor.id(), "Plan it.", TurnOrigin::Message, moment(8)).unwrap();
        planning
            .ask_interview(
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
        conversations.save_turn(&planning).unwrap();
        let mut proposing =
            Turn::start(advisor.id(), "Propose.", TurnOrigin::Message, moment(9)).unwrap();
        proposing
            .propose(
                Proposal::new(
                    "A device.",
                    Vec::new(),
                    vec![ProposedTask {
                        title: TaskTitle::parse("Route the board").unwrap(),
                        description: String::new(),
                        owner: None,
                        estimate_minutes: None,
                        priority: Priority::Normal,
                        waits_for: Vec::new(),
                    }],
                )
                .unwrap(),
            )
            .unwrap();
        conversations.save_turn(&proposing).unwrap();
        database
    }

    #[test]
    fn the_states_of_the_log_written_again_elsewhere_give_the_same_rows() {
        let original = database_with_everything();
        let copy = Database::open_in_memory().unwrap();

        for operation in original.operations().unwrap() {
            let entity = Entity::from_name(&operation.entity).unwrap();
            if entity == Entity::ProjectPlaces {
                continue;
            }
            copy.write(|transaction, operations| match &operation.payload {
                Some(payload) => {
                    let state = object_from_text(payload, "a state")?;
                    write_state(transaction, entity, &state)
                }
                None => remove(transaction, operations, entity, &operation.entity_id),
            })
            .unwrap();
        }

        assert_eq!(rows_of(&copy), rows_of(&original));
        assert!(rows_of(&copy).len() > TABLES.len());
    }

    #[test]
    fn a_state_that_breaks_a_rule_is_refused_and_leaves_nothing() {
        let original = database_with_everything();
        let copy = Database::open_in_memory().unwrap();
        let project = original
            .operations()
            .unwrap()
            .into_iter()
            .find(|operation| operation.entity == "project")
            .unwrap();
        let mut state = object_from_text(project.payload.as_deref().unwrap(), "a state").unwrap();
        state.insert("name".to_owned(), Value::String("   ".to_owned()));

        let result = copy.write(|transaction, _| write_state(transaction, Entity::Project, &state));

        assert!(result.is_err());
        assert!(rows_of(&copy).is_empty());
    }
}
