# teamwright-storage

Stores Teamwright's projects, tasks, sprints, teams, the folders of the projects and
the conversations with specialists in SQLite, and appends every change to an
operation log.

It implements the interfaces `ProjectRepository`, `TaskRepository`,
`MemberRepository`, `ConversationRepository`, `PlanningRepository`,
`PlacesRepository` and `SprintRepository` that the
application crate declares. Nothing outside this crate knows that SQLite is
involved.

## What is in it

| Module | Contents |
|---|---|
| `database` | `Database`: opens the file, runs migrations, hands out read access and logged write transactions |
| `migrations` | The list of schema migrations and the code that applies them |
| `schema/` | One SQL file per migration |
| `projects` | `SqliteProjectRepository` |
| `tasks` | `SqliteTaskRepository` |
| `sprints` | `SqliteSprintRepository`: the sprints of the projects, each saved or deleted together with the tasks that change with it |
| `members` | `SqliteMemberRepository` |
| `conversations` | `SqliteConversationRepository`: conversations, their turns, the steps of the turns and the questions they asked the manager |
| `planning` | The interviews and the proposals of the Advisor, as parts of their turns; `SqlitePlanningRepository`, which stores an approved plan |
| `places` | `SqlitePlacesRepository`: the work folder and the reference folders chosen for each project |
| `operations` | The operation log and the hybrid logical clock |
| `codec` | The text forms of moments, dates, identifiers and enumerations |
| `error` | The crate's internal error type and its conversion to `StorageError` |

## How data is kept

- **Current state** is in ordinary tables: `projects`, `statuses`, `tasks`,
  `task_dependencies`, `members`, `conversations`, `turns`, `turn_steps`,
  `turn_decisions`, `turn_decision_options`, `project_places`,
  `project_reference_folders`, `turn_interviews`, `turn_interview_questions`,
  `turn_interview_options`, `turn_proposals`, `turn_proposal_members`,
  `turn_proposal_tasks`, `turn_proposal_task_waits`, `sprints`.
  The schema with its conventions is in
  [`schema/0001_initial.sql`](src/schema/0001_initial.sql); later files in
  that folder add to it.
- **Every write also appends to `operations`**, in the same transaction. An
  operation records which computer made the change, a clock stamp, and for
  an upsert the complete new state as JSON. State and log therefore cannot
  disagree. The log is what computers will exchange when they sync
  ([decision 0007](../../docs/decisions/0007-local-first-storage.md)).
- **The log is also the history.** Because it keeps the complete state of a
  task after every write, the versions of a task are read straight from it
  (`TaskRepository::versions_of`), in the order of their stamps. The history
  a manager sees needs no table of its own.
- **Each database file has the identity of its computer** (a `ComputerId` of
  the domain), created when the file is first opened and stored in `meta`
  under `device_id`. `Database::device_id` returns it.
- **A conversation knows the computer it runs on** (version 9): the column
  `home_computer` of `conversations`, and the key `home_computer` of its
  logged state. The migration gave every existing conversation, and every
  logged state of one, the identity of this computer, because every
  conversation of an earlier version was started here. A row without it is
  damaged data.
- **Statuses are part of their project.** They have rows of their own in
  `statuses`, but the log keeps them inside the state of the project. Saving
  a project updates the rows of its statuses in place (name, category, color,
  position), adds new ones and deletes the ones the workflow no longer has;
  the logged project then lists the statuses it has now. A status that a
  task is still in cannot be deleted: the database refuses, and nothing is
  written.
- **Deleting a project is logged piece by piece.** In one transaction, every
  task and every member that still belongs to the project is deleted and
  logged as a `delete` of its own, then the statuses and the project, which
  is logged last. The database would otherwise remove those rows by itself,
  through `ON DELETE CASCADE`, without a trace in the log. The conversations
  of the project go first, the same way.
- **A turn is saved often and written sparingly.** The work of a specialist
  saves its turn after every event of the agent program. The row of the turn
  is written and logged (entity `turn`) only when it changed. Its steps have
  a row each in `turn_steps`, keyed by turn and position, and are logged as
  the entity `turn_step` with the identifier `<turn id>/<position>`; only a
  step that is new or changed (a tool use whose outcome became known) is
  written and logged, and a step the turn no longer has is deleted and
  logged as such. A logged turn does not contain its steps.
- **The folders of a project are facts about this computer.** The work
  folder the manager chose is a row in `project_places` (NULL for the
  folder Teamwright gives the project), and the reference folders a row
  each in `project_reference_folders`, keyed by project and position (version
  6). Paths are kept as text, as the manager gave them. Saving updates the
  rows in place and deletes the positions the list no longer has. Every
  write is logged as the entity `project_places` with the identifier of the
  project and the complete state: the work folder and the whole list. A path
  on this computer means nothing on another one, so **these operations must
  not be applied on another computer** when synchronizing is built.
  Deleting a project removes and logs them first, as it does for the
  conversations; the folders on disk are never touched.
- **The question a turn asked the manager** has a row in `turn_decisions`,
  keyed by the turn, and its options a row each in `turn_decision_options`.
  It is written only when it differs from what is stored: when it is asked,
  and when it is answered. It is logged as the entity `decision` with the
  identifier of the turn, and the logged state is the whole question: its
  options and its answer.
- **A conversation is with a specialist or with the Advisor** (version 7):
  the column `with_advisor`, and the model and the effort the Advisor runs
  on in it in `advisor_model` and `advisor_effort`, NULL for the defaults of
  the program. A conversation with a specialist that has either is damaged
  data.
- **The interview and the proposal of a turn of the Advisor** have rows of
  their own, keyed by the turn: `turn_interviews` with its questions (each
  with its answer) and their options, and `turn_proposals` with its members,
  its tasks and what each task waits for. Each is written only when it
  differs from what is stored: when it is made, and when it is answered or
  approved. They are logged as the entities `interview` and `proposal` with
  the identifier of the turn, and the logged state is the whole of it.
  Rows are updated in place by position, and positions a list no longer has
  are deleted.
- **Sprints** (version 8) have a row each in `sprints`, with their summary
  in three columns that are all NULL until the sprint is completed. A sprint
  is logged as the entity `sprint` with its whole state, summary included.
  A task has the columns `sprint_id`, NULL for the backlog, and `points`,
  and its logged state gains `sprint_id` and `points`; a task state logged
  before version 8 has neither and is read as a task in the backlog without
  points. A project counts its sprints in `next_sprint_number`. Saving a
  sprint writes the sprint first and then the tasks that changed with it;
  deleting one writes the tasks that leave it first and then deletes the
  sprint, all in one transaction, so the database never has to take a task
  out of a sprint by itself. Deleting a project deletes and logs its sprints
  after its tasks.
- **An approved plan is one transaction.** `SqlitePlanningRepository`
  writes the project (with its next task number), every new member, every
  new task with what it waits for, and the turn with its approved proposal,
  each logged as usual (`project`, `member`, `task`, `proposal`), or
  nothing. Each turn is deleted
  and logged, which stands for its steps and its question, and then the
  conversation.
  Deleting a task or a member in the database would remove or detach the
  conversations through `ON DELETE CASCADE` and `ON DELETE SET NULL`
  without a trace; the use cases delete or detach them through the
  repository first.

Stored values are checked when they are read. A row that breaks a rule of the
domain is reported as damaged data and never turned into an invalid object.
The model and the effort of a specialist are the columns `model` and
`effort` of `members` (version 4), NULL when they are left to the program; a
person with either is damaged data.

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
