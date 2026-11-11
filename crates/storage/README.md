# teamwright-storage

Stores Teamwright's projects, tasks and teams in SQLite, and appends every
change to an operation log.

It implements the interfaces `ProjectRepository`, `TaskRepository` and
`MemberRepository` that the application crate declares. Nothing outside this crate knows that SQLite is
involved.

## What is in it

| Module | Contents |
|---|---|
| `database` | `Database`: opens the file, runs migrations, hands out read access and logged write transactions |
| `migrations` | The list of schema migrations and the code that applies them |
| `schema/` | One SQL file per migration |
| `projects` | `SqliteProjectRepository` |
| `tasks` | `SqliteTaskRepository` |
| `members` | `SqliteMemberRepository` |
| `operations` | The operation log, the hybrid logical clock, `DeviceId` |
| `codec` | The text forms of moments, dates, identifiers and enumerations |
| `error` | The crate's internal error type and its conversion to `StorageError` |

## How data is kept

- **Current state** is in ordinary tables: `projects`, `statuses`, `tasks`,
  `task_dependencies`, `members`. The schema with its conventions is in
  [`schema/0001_initial.sql`](src/schema/0001_initial.sql); later files in
  that folder add to it.
- **Every write also appends to `operations`**, in the same transaction. An
  operation records which computer made the change, a clock stamp, and for
  an upsert the complete new state as JSON. State and log therefore cannot
  disagree. The log is what computers will exchange when they sync
  ([decision 0007](../../docs/decisions/0007-local-first-storage.md)).
- **Each database file has a device identity**, created when the file is
  first opened and stored in `meta`.

Stored values are checked when they are read. A row that breaks a rule of the
domain is reported as damaged data and never turned into an invalid object.

## The clock stamp

Stamps come from a hybrid logical clock: the time of day in milliseconds, a
counter for changes within the same millisecond, and the device identity as a
tie-breaker. As text they look like

```
1762851600000-00042-0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10
```

and sort in the order the changes happened, even if the computer's clock
stands still or is set back.

## Changing the schema

A released migration is never edited.

1. Add a new file `src/schema/NNNN_what_changes.sql`.
2. Append it to `MIGRATIONS` in `src/migrations.rs`.
3. Update the record types and queries in the repository it affects.
4. Add tests. `a_new_database_gets_every_table_and_the_latest_version` keeps
   the table list honest, and
   `a_database_of_an_older_version_is_brought_up_to_date_with_its_data_kept`
   proves that people who already use the program keep their data.

A database written by a newer program is refused with a clear message instead
of being read wrongly.

## Rules for code in this crate

- All access goes through `Database::read` and `Database::write`. A write
  runs in one transaction and receives an `OperationRecorder`; every change
  must be recorded through it.
- Rows are updated in place with `ON CONFLICT … DO UPDATE`. Replacing a row
  would delete it first and cascade to the rows that refer to it.
- The domain types are rebuilt through their checking constructors, never
  assembled by hand.

## Tests

```bash
cargo test -p teamwright-storage
```

Unit tests cover the codecs, the migrations, the clock and the database
wrapper. `tests/repositories.rs` runs the repositories, the operation log and
the application's use cases against a real SQLite database.

## Building

SQLite is compiled from source into the program (`rusqlite` with the
`bundled` feature), so a C compiler is needed to build this crate, but users
need no database installed.
