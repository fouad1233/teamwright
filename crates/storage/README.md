# teamwright-storage

Stores Teamwright's projects, tasks, sprints, views, teams, the folders of
the projects and the conversations with specialists in SQLite, and appends every change to an
operation log.

It implements the interfaces `ProjectRepository`, `TaskRepository`,
`MemberRepository`, `ConversationRepository`, `PlanningRepository`,
`PlacesRepository`, `SprintRepository` and `ViewRepository` that the
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
| `views` | `SqliteViewRepository`: the views of the projects |
| `members` | `SqliteMemberRepository` |
| `conversations` | `SqliteConversationRepository`: conversations, their turns, the steps of the turns and the questions they asked the manager |
| `planning` | The interviews and the proposals of the Advisor, as parts of their turns; `SqlitePlanningRepository`, which stores an approved plan |
| `places` | `SqlitePlacesRepository`: the work folder and the reference folders chosen for each project |
| `operations` | The operation log, the hybrid logical clock, and the recorder that keeps the stamps of the fields |
| `merge` | How each kind of thing merges field by field, and `MergedState`, a row of `merged_states` |
| `apply` | Writes the rows of a thing from a logged state, and removes them, without logging the thing itself |
| `take_in` | Takes in the operations of another computer: merge, deletions, waiting, repairs |
| `notices` | What syncing tells the manager, kept on this computer |
| `folder` | `FolderOfComputers`: the files in the sync folder, which implements the port `SyncFolder` |
| `ledger` | `SqliteSyncLedger`: the bookkeeping of syncing, which implements the port `SyncLedger` |
| `codec` | The text forms of moments, dates, identifiers and enumerations |
| `error` | The crate's internal error type and its conversion to `StorageError` |

## How data is kept

- **Current state** is in ordinary tables: `projects`, `statuses`, `tasks`,
  `task_dependencies`, `members`, `conversations`, `turns`, `turn_steps`,
  `turn_decisions`, `turn_decision_options`, `project_places`,
  `project_reference_folders`, `turn_interviews`, `turn_interview_questions`,
  `turn_interview_options`, `turn_proposals`, `turn_proposal_members`,
  `turn_proposal_tasks`, `turn_proposal_task_waits`, `sprints`, `views`.
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
- **Views** (version 13) have a row each in `views`: the kind, the name,
  whose tasks are shown (`assignee_kind`, and `assignee_member_id` for a
  member), `hides_finished` and the place (`position`). A view is logged as
  the entity `view` with its whole state. Saving several views, as moving
  one does, is one transaction. Deleting a project deletes and logs its
  views. Migration 13 gives every project of an earlier version the six
  views a new project starts with, and logs them as changes of this
  computer, after every stamp in the log, so that they reach the other
  computers. Their identifiers follow from the project and the kind
  (`View::starting_id`), as those of a new project do: two computers that
  both had a project before version 13 give it the same six views, which
  merge into one set.
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

## Field stamps and merged states

Computers that keep in step merge what they changed field by field
([decision 0017](../../docs/decisions/0017-sync-through-a-folder.md)).
For that, every stored thing keeps the stamp of each of its fields (version
10):

- **Fields.** `merge.rs` says, for every entity, which keys of its logged
  state belong together and merge as one field (a status with the moment of
  completion, a start date with a due date, who a task is given to), and
  which rule a field follows: the later stamp wins, or the larger value
  wins (the next task number, the next sprint number). A key
  no field names is a field of its own, so a key written by a newer version
  is kept and merged too. A turn and its parts merge whole; the folders of
  a project never leave the computer. The list is an exhaustive `match` on
  the entity: a new kind of thing does not compile until it says how it
  merges.
- **`merged_states`** keeps, for every thing ever stored, its merged state
  as JSON (keys this version does not know included), the latest stamp of
  any field as `base_stamp`, the older stamps of the other fields by name,
  and for a deleted thing the stamp of the deletion with, for each
  computer, the latest change of the thing the deleting computer had seen.
  Migration 10 filled it from the log: every field of an existing thing has
  the stamp of its latest upsert.
- **The recorder does it.** `OperationRecorder::record_upsert` keeps its
  signature: it reads the merged state, gives every field whose value
  changed the stamp of the new operation, keeps the stamps of the others,
  writes the merged state, and logs the operation with the stamps of its
  fields that are older than itself (`field_stamps`). Writing the same state
  again changes no stamp. `record_delete` records the deletion and what it
  had seen (`seen`).
- **A deletion is final.** Writing a deleted thing again is refused
  (`DatabaseError::Deleted`), because identifiers are never used twice. A
  step of a turn is the exception: its identifier is its position, and it
  can be written again after it was removed.
- **Numbers.** Every operation has `device_sequence`, its number on the
  computer that made it; for an operation of this computer it equals
  `sequence`.

The table `merged_states` is bookkeeping of the log and is not logged
itself.

**Writing rows from a stored state.** Every record type can be read back
from the JSON the log keeps, and every repository's store function is split
in two: the half that writes the rows of a record, and the half that logs.
`apply::write_state` calls the first half for any entity, after rebuilding
the domain value through its checking constructor, so a damaged state is
refused and nothing is written. `apply::remove` removes the rows of a thing
without logging its removal; what still belongs to it and has an entry of
its own in the log (the tasks of a project, the turns of a conversation) is
removed with a logged deletion of this computer. This is how an operation
taken in from another computer, which is in the log already, reaches the
tables.

## Taking in the operations of another computer

`Database::operations_to_send(after)` hands out the operations this
computer made after a number, without the folders of the projects, as
`ExchangedOperation`s of the application crate.
`Database::take_in(operations)` takes in operations of other computers, all
in one transaction:

1. The operations are sorted by stamp. Each is stored in the log once, with
   its computer and its number there; one the log has already is passed
   over, so taking in twice changes nothing and logs nothing. The clock
   moves on to at least its stamp.
2. An operation of an entity this version does not know waits (`waiting =
   'unknown entity'`). One of `project_places` is kept and never applied
   (`'never'`).
3. An operation for a thing whose parent was deleted (a task of a deleted
   project) is dropped. One whose parent, or something its merged state
   refers to (a task it waits for, its member, its sprint), has not arrived
   yet waits (`'parent'`) and is tried again until a pass makes no progress.
4. An upsert is merged into the merged state field by field: a field takes
   the incoming value if its stamp is later, and a field whose larger value
   wins takes the larger one. A deleted thing stays deleted, except a step
   of a turn that a later change writes again. If the merged state changed,
   its rows are written. A reference to something deleted, or to a status
   the workflow no longer has, is repaired as the rows are written, with a
   logged change of this computer: the task moves to the first status of
   the same category (else the first status), and loses its deleted parent,
   blocker, member or sprint; a conversation about a deleted task is
   deleted, and one with a deleted member forgets it, as does a view that
   showed only the tasks of a deleted member: it shows everybody's again
   and keeps hiding finished tasks if it did. A view of a deleted project is
   dropped like a task of one.
5. A deletion first repairs, with logged changes of this computer, what
   still refers to the thing (the subtasks and the tasks that wait for a
   task, the conversations about it, the tasks given to a member and the
   views that show only theirs, the tasks of a sprint), then removes its
   rows. If it throws away a change this
   computer made that it had not seen, the manager gets a notice. Two
   deletions of one thing keep the earlier stamp.
6. An operation whose state breaks a rule, or that the database refuses, is
   kept in the log as `'damaged'` and never applied; the batch goes on.
7. The rules that span records are checked in the projects that changed,
   and what broke is repaired with ordinary logged writes of this computer,
   which go out in the next round:

   | What broke | Repair | Who repairs |
   |---|---|---|
   | Two sprints of a project are active | the one started later goes back to planned | any computer |
   | Two tasks of a project have one number | the task whose number was set later gets the next free number | only the computer that created that task |
   | Two projects have one key | the project whose key was set later gets the key with the next number (`SSN2` gives `SSN3`) | only the computer that created it |
   | A chain of waiting or of parts closes on itself | the link whose field was set last is removed | any computer |
   | A project has two backlogs, or two dashboards | the younger is removed | any computer |
   | Two views of a project have one name, whatever the capitals | the younger gets the name with the first free number from 2 on (`Board` gives `Board 2`) | any computer |
   | A project has no view left, because two computers each removed others | it gets a board again, with the identifier of its starting board, or, if that was deleted, the next one `View::board_again` gives | any computer |

   "Set later" is decided by the stamp of the field, and the creator of a
   thing is the computer of its first operation. A view is as old as its
   first operation. A renumbered task and a project with a new key leave a
   notice; a repaired view does not, because no kind of notice is about
   views.

`TakeInReport` says how many operations were new, which projects and
conversations changed, which turns that ran here were deleted elsewhere
(their programs are to be stopped), how many notices were added, and how
far the latest stamp is ahead of this computer's clock. The notices
(`notices` table, version 11) are bookkeeping of syncing and are not logged:
`Database::notices` lists them, `Database::dismiss_notice` forgets one. The
history of a task passes over operations that wait.

## The files in the sync folder

`FolderOfComputers` implements the port `SyncFolder` of the application
crate. It knows nothing of SQLite. A computer writes only below its own
folder:

```
<chosen folder>/devices/<identity>/computer.json
<chosen folder>/devices/<identity>/log/000000000001-000000004211.jsonl
```

- **The card**, `computer.json`, says who the computer is: its identity,
  its name, the version of Teamwright and the format of its files, and when
  it last wrote. It is written under a temporary name and renamed over the
  old one, so a reader sees the old card or the new one.
- **A log file** is named by the first and the last number it holds, padded
  to twelve digits. Its first line is a header (`format`, `computer`,
  `first`, `last`), every line after it one operation (`sequence`, `stamp`,
  `entity`, `id`, `kind`, and `state` with `field_stamps` for an upsert or
  `seen` for a deletion), and its last line a footer (`count`, `bytes`
  before it). It is written as `.writing-<random>`, flushed to the disk and
  renamed; it never changes afterwards.
- **Reading** takes only names of the exact pattern, so a temporary file, a
  "conflicted copy" a sync program made and a file of another program are
  passed over. A file whose footer does not match what was read, that does
  not end in a newline, or that has a line that is not an operation, is
  `NotYet` and looked at again next time. A header of a newer format is
  reported as `NewerFormat`.
- **Joining.** When a computer has more than 64 files smaller than 256 KiB,
  `join_small_files` writes one file that covers all of its operations and
  then removes the others. A reader that read some of the small files takes
  from the joined one only the numbers it has not read.

`tests/folder.rs` runs all of this in a temporary folder of its own.

## The bookkeeping of syncing

`SqliteSyncLedger` implements the port `SyncLedger` (version 12). The table
`computers` keeps the other computers: what their cards say, whether the
manager accepted them (NULL while undecided), the number of the last of
their operations taken in, how far their clock is ahead, and a newer
format they wrote. `written_files` keeps the names of the files this
computer wrote; a file in its own folder that is not there means another
computer uses its identity. The settings are keys of `meta`:
`sync_folder`, `files_folder`, `computer_name`, `written_through` and
`copied_before_sync`. Taking in a batch and moving the number read up to
happen in one transaction. Before anything is taken in for the first time,
the database is copied with `VACUUM INTO` to
`teamwright-before-sync-<date>.db` next to it. A new identity changes
`device_id` in `meta` and in the open database at once; the operations of
the old identity that were never written into the folder are renumbered
under the new one. None of this is logged.

`tests/sync.rs` proves the feature: real databases in temporary folders of
their own, a `SyncService` for each, and one temporary sync folder. Changes
made apart meet in either order, a second exchange changes nothing, a
computer that joins late ends up equal, and a database copied file for file
is found out and goes on under a new identity without losing an operation.

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

Unit tests cover the codecs, the migrations, the clock, the field stamps,
writing rows from a stored state, and the database wrapper.
`tests/repositories.rs` runs the repositories, the operation log and the
application's use cases against a real SQLite database. `tests/take_in.rs`
runs two real databases that take in each other's operations, handed over
in memory.

## Building

SQLite is compiled from source into the program (`rusqlite` with the
`bundled` feature), so a C compiler is needed to build this crate, but users
need no database installed.
