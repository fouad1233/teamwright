//! Brings a database file to the format this program expects.
//!
//! The format version is the number of migrations that have been applied. It
//! is kept in SQLite's `user_version`. A released migration is never edited:
//! a change to the format is a new migration at the end of the list.

use rusqlite::Connection;

use crate::error::DatabaseError;

/// Every migration, in the order it is applied.
const MIGRATIONS: &[&str] = &[
    include_str!("schema/0001_initial.sql"),
    include_str!("schema/0002_team_members.sql"),
    include_str!("schema/0003_conversations.sql"),
    include_str!("schema/0004_member_profiles.sql"),
    include_str!("schema/0005_decisions.sql"),
    include_str!("schema/0006_project_places.sql"),
    include_str!("schema/0007_planning.sql"),
    include_str!("schema/0008_sprints.sql"),
    include_str!("schema/0009_home_computer.sql"),
];

/// Applies the migrations a database has not seen yet. Each one runs in its
/// own transaction, so a failure leaves the database at the last complete
/// version.
pub(crate) fn migrate(connection: &mut Connection) -> Result<(), DatabaseError> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(found)
        .ok()
        .filter(|applied| *applied <= MIGRATIONS.len())
        .ok_or(DatabaseError::TooNew {
            found,
            supported: MIGRATIONS.len(),
        })?;

    // The first migration produces version 1, the second version 2, and so on.
    for (version, migration) in (1_i64..).zip(MIGRATIONS).skip(applied) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(migration)?;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version_of(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn table_names(connection: &Connection) -> Vec<String> {
        connection
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn a_new_database_gets_every_table_and_the_latest_version() {
        let mut connection = Connection::open_in_memory().unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        let tables = table_names(&connection);
        for expected in [
            "conversations",
            "members",
            "meta",
            "operations",
            "project_places",
            "project_reference_folders",
            "projects",
            "sprints",
            "statuses",
            "task_dependencies",
            "tasks",
            "turn_decision_options",
            "turn_decisions",
            "turn_interview_options",
            "turn_interview_questions",
            "turn_interviews",
            "turn_proposal_members",
            "turn_proposal_task_waits",
            "turn_proposal_tasks",
            "turn_proposals",
            "turn_steps",
            "turns",
        ] {
            assert!(
                tables.iter().any(|table| table == expected),
                "missing table {expected}"
            );
        }
    }

    #[test]
    fn a_database_of_an_older_version_is_brought_up_to_date_with_its_data_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATIONS[0]).unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();
        connection
            .execute(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z')",
                [],
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        assert!(table_names(&connection).contains(&"members".to_owned()));
        assert!(table_names(&connection).contains(&"conversations".to_owned()));
        let kept: String = connection
            .query_row("SELECT name FROM projects WHERE id = 'p1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kept, "Smart sensor node");
    }

    #[test]
    fn a_database_without_profiles_is_brought_up_to_date_with_its_members_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..3] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 3).unwrap();
        connection
            .execute_batch(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z');
                 INSERT INTO members (
                     id, project_id, name, kind, responsibility, color, joined_at, updated_at
                 )
                 VALUES (
                     'm1', 'p1', 'Hardware engineer', 'agent', '', 'teal',
                     '2026-11-11T09:00:00.000Z', '2026-11-11T09:00:00.000Z'
                 );",
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        let (name, model, effort): (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT name, model, effort FROM members WHERE id = 'm1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(name, "Hardware engineer");
        assert_eq!(model, None);
        assert_eq!(effort, None);
    }

    #[test]
    fn a_database_without_decisions_is_brought_up_to_date_with_its_turns_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..4] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 4).unwrap();
        connection
            .execute_batch(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z');
                 INSERT INTO conversations (id, project_id, member_id, task_id, started_at)
                 VALUES ('c1', 'p1', NULL, NULL, '2026-11-11T09:00:00.000Z');
                 INSERT INTO turns (
                     id, conversation_id, request, origin, state, started_at, answer
                 )
                 VALUES (
                     't1', 'c1', 'Which regulator?', 'message', 'running',
                     '2026-11-11T09:00:00.000Z', ''
                 );",
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        let request: String = connection
            .query_row("SELECT request FROM turns WHERE id = 't1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(request, "Which regulator?");
        let questions: i64 = connection
            .query_row("SELECT count(*) FROM turn_decisions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(questions, 0);
    }

    #[test]
    fn a_database_without_the_folders_of_projects_is_brought_up_to_date_with_its_projects_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..5] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 5).unwrap();
        connection
            .execute(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z')",
                [],
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        let kept: String = connection
            .query_row("SELECT name FROM projects WHERE id = 'p1'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kept, "Smart sensor node");
        let chosen: i64 = connection
            .query_row("SELECT count(*) FROM project_places", [], |row| row.get(0))
            .unwrap();
        assert_eq!(chosen, 0);
    }

    #[test]
    fn a_database_without_planning_is_brought_up_to_date_with_its_conversations_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..6] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 6).unwrap();
        connection
            .execute_batch(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z');
                 INSERT INTO conversations (id, project_id, member_id, task_id, started_at)
                 VALUES ('c1', 'p1', NULL, NULL, '2026-11-11T09:00:00.000Z');",
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        let (with_advisor, model, effort): (i64, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT with_advisor, advisor_model, advisor_effort FROM conversations
                 WHERE id = 'c1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!((with_advisor, model, effort), (0, None, None));
        let proposals: i64 = connection
            .query_row("SELECT count(*) FROM turn_proposals", [], |row| row.get(0))
            .unwrap();
        assert_eq!(proposals, 0);
    }

    #[test]
    fn a_database_without_sprints_is_brought_up_to_date_with_its_tasks_kept() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..7] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 7).unwrap();
        connection
            .execute_batch(
                "INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 3, '2026-11-11T09:00:00.000Z');
                 INSERT INTO statuses (id, project_id, position, name, category, color)
                 VALUES ('s1', 'p1', 0, 'To do', 'todo', 'slate');
                 INSERT INTO tasks (
                     id, project_id, number, title, description, status_id, priority,
                     assignee_kind, assignee_member_id, parent_id, estimate_minutes,
                     start_on, due_on, rank, created_at, updated_at, completed_at
                 )
                 VALUES (
                     't1', 'p1', 1, 'Design the board', 'Two layers.', 's1', 'high',
                     'manager', NULL, NULL, 240, '2026-11-12', '2026-11-14', 1024.0,
                     '2026-11-11T09:00:00.000Z', '2026-11-11T10:00:00.000Z', NULL
                 ),
                 (
                     't2', 'p1', 2, 'Write the firmware', '', 's1', 'normal',
                     NULL, NULL, 't1', NULL, NULL, NULL, 2048.0,
                     '2026-11-11T09:00:00.000Z', '2026-11-11T09:00:00.000Z', NULL
                 );",
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            version_of(&connection),
            i64::try_from(MIGRATIONS.len()).unwrap()
        );
        type Kept = (String, String, Option<i64>, Option<String>, Option<String>);
        let kept: Kept = connection
            .query_row(
                "SELECT title, priority, estimate_minutes, due_on, updated_at FROM tasks
                 WHERE id = 't1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            kept,
            (
                "Design the board".to_owned(),
                "high".to_owned(),
                Some(240),
                Some("2026-11-14".to_owned()),
                Some("2026-11-11T10:00:00.000Z".to_owned())
            )
        );
        let parent: Option<String> = connection
            .query_row("SELECT parent_id FROM tasks WHERE id = 't2'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(parent.as_deref(), Some("t1"));
        let in_backlog_without_points: i64 = connection
            .query_row(
                "SELECT count(*) FROM tasks WHERE sprint_id IS NULL AND points IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(in_backlog_without_points, 2);
        let (next_task, next_sprint): (i64, i64) = connection
            .query_row(
                "SELECT next_task_number, next_sprint_number FROM projects WHERE id = 'p1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((next_task, next_sprint), (3, 1));
        let sprints: i64 = connection
            .query_row("SELECT count(*) FROM sprints", [], |row| row.get(0))
            .unwrap();
        assert_eq!(sprints, 0);
    }

    #[test]
    fn a_database_without_home_computers_gives_every_conversation_this_computer() {
        let mut connection = Connection::open_in_memory().unwrap();
        for migration in &MIGRATIONS[..8] {
            connection.execute_batch(migration).unwrap();
        }
        connection.pragma_update(None, "user_version", 8).unwrap();
        connection
            .execute_batch(
                "INSERT INTO meta (key, value)
                 VALUES ('device_id', '0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10');
                 INSERT INTO projects (id, key, name, description, next_task_number, created_at)
                 VALUES ('p1', 'SSN', 'Smart sensor node', '', 1, '2026-11-11T09:00:00.000Z');
                 INSERT INTO conversations (id, project_id, member_id, task_id, started_at)
                 VALUES ('c1', 'p1', NULL, NULL, '2026-11-11T09:00:00.000Z');
                 INSERT INTO operations (device_id, stamp, entity, entity_id, kind, payload)
                 VALUES (
                     '0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10',
                     '1762851600000-00000-0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10',
                     'conversation', 'c1', 'upsert', '{\"id\":\"c1\",\"project_id\":\"p1\"}'
                 );",
            )
            .unwrap();

        migrate(&mut connection).unwrap();

        let home: String = connection
            .query_row(
                "SELECT home_computer FROM conversations WHERE id = 'c1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(home, "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10");
        let logged: String = connection
            .query_row(
                "SELECT json_extract(payload, '$.home_computer') FROM operations",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(logged, home);
    }

    #[test]
    fn migrating_an_up_to_date_database_changes_nothing() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .execute("INSERT INTO meta (key, value) VALUES ('kept', 'yes')", [])
            .unwrap();

        migrate(&mut connection).unwrap();

        let kept: String = connection
            .query_row("SELECT value FROM meta WHERE key = 'kept'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(kept, "yes");
    }

    #[test]
    fn a_database_from_a_newer_program_is_refused() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "user_version", 9_999)
            .unwrap();

        let error = migrate(&mut connection).unwrap_err();

        assert!(matches!(error, DatabaseError::TooNew { found: 9_999, .. }));
    }
}
