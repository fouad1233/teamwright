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
            "projects",
            "statuses",
            "task_dependencies",
            "tasks",
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
