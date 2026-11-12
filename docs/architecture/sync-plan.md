# Plan: keeping computers in step through a folder

This is how [decision 0017](../decisions/0017-sync-through-a-folder.md) is
built. Read the decision first: it says what the design is and why. This
document says in which order it is built, what each crate gains, and which
tests prove each step. It is a plan for review; nothing in it is built yet.

## The order of the steps

| Step | Crate | What it adds | Needs |
|---|---|---|---|
| 1 | domain, application, storage, desktop | A conversation knows its home computer; only turns of this computer are ended | nothing |
| 2 | storage | Field stamps: every write keeps a stamp for each field it wrote | nothing |
| 3 | storage | Every repository can write its rows from a stored state without logging | 2 |
| 4 | storage | Taking in operations from another computer: merge, deletions, waiting, repairs | 2, 3 |
| 5 | storage | The files in the sync folder: writing, reading, joining small files | nothing |
| 6 | application, storage | `SyncService` and its ports; the tests with two databases and a real folder | 1, 4, 5 |
| 7 | desktop shell | The commands, the background round, the events, the copy before the first take-in | 6 |
| 8 | desktop interface | The Computers place, the state line, conversations that run elsewhere | 7 |
| 9 | docs | READMEs, overview, manual, roadmap, story; decision 0017 accepted | 8 |

Each step is one or a few commits, and every commit passes the four checks of
[AGENTS.md](../../AGENTS.md). Steps 1, 2 and 5 do not depend on each other and
can be built in any order. Nothing is visible to the manager before step 8,
and nothing writes into a folder before step 7, so a half-built feature
never touches anyone's data. Steps 1 to 4 change only how data is kept on one
computer, and each keeps today's behavior.

**Sprints.** Sprints are being built at the same time. Whichever lands
second adds what the other needs: if sprints land first, step 2 lists their
fields; if this lands first, the sprint work adds the entity `sprint` to the
table of fields in `merge.rs` (step 2) and the repairs of step 4 (a deleted
sprint, two active sprints). The table is an exhaustive `match` on `Entity`,
so a new entity without a merge rule does not compile.

## Step 1: a conversation has a home computer

Why first: it fixes the one thing that would do harm the moment turns arrive
from elsewhere, and it is useful on its own.

**Domain**

- `ComputerId`, an identifier like the others (a UUID with a text form).
  `DeviceId` in storage becomes a thin wrapper of it, or is replaced by it.
- `Conversation` gains `home_computer: ComputerId`, set when it is started
  and never changed; `ConversationSnapshot` gains the field.

**Application**

- `WorkPorts` gains `this_computer: ComputerId`.
- A new conversation is started with `home_computer = this_computer`.
- `end_turns_nothing_runs` (used by `recover` and `close`) ends only the
  running turns whose conversation is at home here.
- Replying, answering a question or an interview, approving a proposal,
  stopping a turn and opening the session in the desktop app refuse a
  conversation whose home is elsewhere, with the new error
  `ApplicationError::ConversationRunsElsewhere { computer }`.
- The guards of work in progress keep counting every running turn, so they
  protect the work of other computers too.

**Storage**

- Migration `0008_home_computer.sql`: `ALTER TABLE conversations ADD COLUMN
  home_computer TEXT`, then fill it with the identity in `meta` (every
  conversation in an existing database was started here). A new database
  has no conversations, so nothing is left empty. A conversation read with
  no home computer is damaged data.
- `ConversationRecord` gains `home_computer`; the logged state carries it.
- `Database::device_id()` is handed to the work service by the shell.

**Shell and interface**

- `ConversationDto` gains `homeComputer` (identity) and `isHere`
  (`dto.rs` and `types.ts` together). Until step 8 every conversation is at
  home, and the interface needs no change beyond the type.

**Tests**

- Application: `recover` leaves a running turn of another computer running;
  `close` too; replying in a conversation of another computer is refused;
  the guard that refuses removing a working member counts a turn of another
  computer.
- Storage: the migration fills the home computer from `meta` and keeps the
  conversations; a conversation round-trips with its home.

## Step 2: field stamps

Why: per-field merging needs to know, for each field of a stored thing, the
stamp of the write that set it.

**Storage**

- New module `merge.rs`. For each `Entity`, the fields that merge on their
  own (decision 0017, section 3), as a list of `Field { name, keys, rule }`.
  `keys` are the keys of the logged JSON that belong to the field (for
  example `status` is `status_id` and `completed_at`; `schedule` is
  `start_on` and `due_on`; `assignee` is `assignee_kind` and
  `assignee_member_id`). A key that no field names is a field of its own.
  `rule` is `Later` (the later stamp wins) or `Larger` (the larger value
  wins: `next_task_number`, `updated_at`). The turn and its parts have one
  field, `whole`. `project_places` is marked as never leaving the computer.
- Migration `0009_merged_states.sql`:
  - `operations` gains `device_sequence INTEGER` (the number of the
    operation on the computer that made it; filled with `sequence` for
    every existing row), `field_stamps TEXT` (JSON: the stamps of fields
    older than the operation; NULL when there are none), and `waiting TEXT`
    (NULL once applied; otherwise why it waits: `parent`, `unknown entity`,
    `newer format`). A unique index on `(device_id, device_sequence)`, and an
    index on `stamp`.
  - A new table `merged_states (entity, entity_id, state, base_stamp,
    field_stamps, deleted_stamp, deleted_seen)`, keyed by entity and
    identifier. `state` is the merged JSON, including keys this version
    does not know. A field without a stamp in `field_stamps` has the
    `base_stamp`. `deleted_stamp` is set for a deleted thing, and
    `deleted_seen` keeps, for each computer, the latest change of the thing
    the deleting computer had seen.
  - It is filled from the log in SQL: for each entity and identifier, the
    latest operation by stamp; an upsert gives the state and the base stamp,
    a delete gives the deletion.
- `OperationRecorder::record_upsert` keeps its signature. It reads the
  merged state, gives the fields whose value changed the stamp of the new
  operation, keeps the stamps of the others, writes the merged state, and
  stores the operation with the older stamps in `field_stamps`.
  `record_delete` sets the deletion and its `seen`.
- `HybridLogicalClock` gains `witness(stamp)`, which moves `last` to at
  least the stamp; `Stamp` gains the device part when read from text.

**Tests**

- Writing a task twice with only the title changed gives the title the new
  stamp and every other field the old one.
- Writing the same state again changes no stamp (the turn already skips
  unchanged writes; the others are checked).
- A deletion records what was seen.
- The migration fills `merged_states` from an existing log, with deletions,
  and `device_sequence` equals `sequence`.
- `witness` never moves the clock back, and the next tick is later than the
  witnessed stamp.
- Every entity has a merge rule (the exhaustive match, plus a test over
  `Entity`'s list).

## Step 3: writing rows from a stored state

Why: taking in writes rows from a merged state, without a new operation and
without the checks of the use cases.

**Storage**

- Every record type (`ProjectRecord`, `StatusRecord`, `TaskRecord`,
  `MemberRecord`, `ConversationRecord`, `TurnRecord`, `StepRecord`,
  `DecisionRecord`, `InterviewRecord`, `ProposalRecord` and their parts)
  derives `Deserialize` as well as `Serialize`. Only `TaskRecord` does
  today.
- Each repository's store function is split in two: one that writes the
  rows of a record, and the existing one, which calls it and then logs. The
  writing half is `pub(crate)` and is what step 4 calls. Deletion is split
  the same way.
- Each writing half still checks the record by rebuilding the domain value
  through its checking constructor, so a damaged state from elsewhere is
  refused like damaged data read from disk, and never stored.

**Tests**

- For every entity: a state logged by a write, written again through the
  writing half into a second database, gives the same rows. This is the
  first test with two databases, without any folder.
- A state that breaks a rule of the domain is refused and leaves nothing.

## Step 4: taking in operations

Why: this is the merge. It is the largest step and is built in three
commits: plain fields and deletions, waiting operations, repairs.

**Storage**

- `Database::take_in(operations: &[IncomingOperation]) -> TakeInReport`,
  where an incoming operation is what a line of a file holds plus the
  computer it came from. In one transaction:
  1. Sort by stamp. For each operation, insert it into `operations` with
     its own computer, number, stamp and field stamps; if it was there
     already, go to the next one.
  2. Witness its stamp.
  3. `project_places`: mark it as not applied and go on.
     An unknown entity: mark it `waiting = 'unknown entity'`.
  4. A parent that is deleted (a task of a deleted project, a step of a
     deleted turn): drop it and note it if it was a change the deletion had
     not seen. A parent that is not known yet: `waiting = 'parent'`.
  5. Merge it into `merged_states`, field by field, with the rule of each
     field. A deleted thing stays deleted (a step of a turn excepted: the
     later stamp wins).
  6. If the merged state changed, write its rows through step 3. A
     reference the database enforces (a task's status, its parent) is
     repaired as the row is written, with a logged change of this computer.
  7. Retry the waiting operations until a pass makes no progress.
  8. Check the rules across records for the projects that changed, and
     repair them with ordinary logged writes (decision 0017, section 3):
     dangling references, chains that close on themselves, two tasks with
     one number or two projects with one key (only if this computer created
     the later one), two active sprints.
- `TakeInReport`: the projects and conversations that changed, the turns
  that were deleted while they run here, and the notices.
- A notice table, `notices`, keeps what the manager is told. It is
  bookkeeping of this computer and is not logged (see the open questions).
- `versions_of` keeps reading the states of the log in stamp order and
  passes over operations that wait.
- The repositories' `delete` paths log the deletion of every child as they
  do today, so a deletion that arrives brings the deletions of the children
  its computer knew; children it did not know are removed by step 8.

**Tests** (two in-memory databases, operations handed over in memory)

- A task changed on both: different fields both survive; the same field
  keeps the later value; the result is the same in both orders.
- Taking in the same operations twice changes nothing, and logs nothing.
- Creating a task on one computer does not undo a rename of the project on
  the other (the reason for per-field merging).
- A task deleted on one and changed on the other is deleted on both, and the
  computer whose change was dropped has a notice.
- A project deleted on one while the other added a task: the task is gone
  on both.
- A task arriving before its project waits and is applied once the project
  arrives.
- An operation of an unknown entity waits and is kept.
- An operation of `project_places` is stored and never applied.
- A status removed on one while the other moved a task into it: the task
  ends up in a status of the workflow on both.
- Two tasks numbered 12: the one created later is renumbered, only by its
  creator, and both databases agree after the renumbering is taken in.
- Two projects with the key `SSN`: the same.
- A chain of waiting closed by two computers is opened again.
- A member deleted while a task was given to them elsewhere: the task has
  nobody.
- The clock: after taking in a stamp from the future, the next local stamp
  is later than it.
- A long random run: two databases make a few hundred random changes from a
  fixed seed (a small generator in the test, no new dependency), exchange
  in random batches and orders, and end up with the same state. The state
  is compared through the repositories, every project with its tasks,
  members and conversations, sorted.

## Step 5: the files in the sync folder

Why: the folder format is independent of the merge and is tested on its own
against a real temporary folder.

**Storage**: a module `folder` with `FolderOfComputers`, which implements
the port `SyncFolder` of step 6. It knows nothing of SQLite.

- `write_card(card)`: writes `devices/<id>/computer.json` under a temporary
  name and renames it over the old one.
- `write_file(computer, operations) -> FileName`: writes
  `devices/<id>/log/.writing-<random>`, with a header, one line per
  operation and a footer with the count and the bytes; flushes it to disk;
  renames it to `<first>-<last>.jsonl`. The numbers are padded to twelve
  digits.
- `computers()`: the identities of the folders under `devices/` whose name
  is an identity, and their cards. A card that cannot be read is reported
  as such, not as an error of the whole folder.
- `files_of(computer, after)`: the names of the exact pattern whose last
  number is above `after`, in order.
- `read_file(computer, name) -> Complete(operations) | NotYet`: `NotYet`
  for a missing or wrong footer, a wrong count, or a line that is not JSON.
- `join_small_files(computer)`: when there are more than 64 files below a
  size, writes one file that covers them, then removes them.
- A newer format in a header is reported, and nothing after it is read.

**Tests** (in a temporary folder)

- Written and read back, a file gives the same operations.
- A file cut off at any byte is `NotYet`; with its end added, it is read.
- A temporary file, a file named like `… (conflicted copy).jsonl`, and a
  file of another pattern are passed over.
- Joining small files: a reader that read some of the small files and then
  the joined one gets every operation once (the numbers decide).
- Two writers for two computers in the same folder never touch each other's
  files.
- A card replaced while it is read is either the old or the new one.

## Step 6: the use cases

**Application**: a module `sync` with its types and ports.

- `ComputerCard { id, name, version, format, written_at }`.
- `ExchangedOperation`: the computer, its number there, the stamp, the
  entity, its identifier, `Upsert` or `Delete`, the state and the field
  stamps as JSON text. The application does not look inside the JSON.
- Port `SyncFolder`: the operations of step 5.
- Port `SyncLedger`, implemented by storage: this computer's operations
  after a number (without `project_places`), `take_in`, the computers known
  with their names, acceptance and "read up to" numbers, the files this
  computer wrote, the settings (the folder, this computer's name, what was
  written), the notices, and `new_identity()`.
- `SyncService`:
  - `round()`: refuses if no folder is chosen; checks that every file in
    this computer's folder is one it wrote (else stops with
    `IdentityInUse`); writes the operations not yet written; writes the
    card; joins small files; for every accepted computer, reads the new
    complete files and takes them in, in batches of a few hundred
    operations; reports what changed, new computers and problems.
  - `choose_folder(path)`: refuses a folder inside Teamwright's data
    folder; remembers it; the first round writes the whole log.
  - `stop()`, `rename_this_computer(name)`, `accept(computer)`,
    `ignore(computer)`, `notices()`, `dismiss(notice)`,
    `new_identity()`.
- `WorkService::stop_turns(ids)` stops turns that a deletion from another
  computer removed while they ran here.

**Storage**: `SqliteSyncLedger`, and migration `0010_sync.sql` with the
bookkeeping: `computers` (identity, name, accepted, read up to, format,
version, last heard, how far its clock is ahead), `written_files` (name,
first, last, when), `notices`, and the keys `sync_folder`, `computer_name`
and `written_through` in `meta`.

**Tests**

- Application, with in-memory fakes of both ports: a round writes only new
  operations; a computer that is not accepted is not read; an identity in
  use stops the round; a newer format stops reading that computer and is
  reported.
- `crates/storage/tests/sync.rs`, the test that proves the feature: two
  real databases, two `SyncService`s, one temporary folder. Each scenario of
  step 4 is run with the exchange in both orders (A writes, B reads, B
  writes, A reads; and B first), and then a second time, and after each
  exchange the two databases are compared. The second exchange must change
  nothing; the only operations it may carry are the repairs that the first
  exchange made.
- A third database that joins late reads everything from both and ends up
  equal.
- A database copied file for file to a second computer is found out, and
  after `new_identity()` both go on without losing an operation.

## Step 7: the shell

- `state.rs` builds `SyncService` with `FolderOfComputers` and
  `SqliteSyncLedger`, and starts one background thread. Every 5 seconds it
  asks whether there is something to write; it writes when the last change
  is 5 seconds old, or 30 seconds have passed since the last file. Every 20
  seconds it reads the others. It never runs two rounds at once.
- Before the first round that takes in anything, the database is copied
  with `VACUUM INTO` to `teamwright-before-sync-<date>.db` in the data
  folder.
- After a round: the event `data-changed` with the identifiers of the
  projects that changed; `conversation-changed` for each conversation that
  changed, through the existing listener; `sync-changed` with the state.
  Turns deleted while running here are stopped.
- Commands: `sync_state`, `choose_sync_folder`, `stop_syncing`,
  `sync_now`, `rename_this_computer`, `accept_computer`,
  `ignore_computer`, `sync_notices`, `dismiss_sync_notice`,
  `give_this_computer_a_new_identity`.
- The contract, `dto.rs` and `types.ts` together: `SyncStateDto` (the
  folder, this computer, the other computers with name, version, last heard
  and whether they are accepted or use a newer format, the phase:
  `off`, `in_step`, `writing`, `unreachable`, `stopped`, and the last round),
  `SyncNoticeDto`, and `TaskVersionDto` gains the computer's name.
- The demo backend in the browser pretends a second computer, "Desk", with
  one conversation that runs there.

**Tests**: the commands map errors to codes; a round started by the thread
and one started by `sync_now` do not run at once.

## Step 8: the interface

Following [the design philosophy](../design/philosophy.md), with the
components of the interface and its tokens.

- **Computers**: a place reached from the bottom of the navigation. This
  computer's name, editable. The sync folder, chosen with the system's
  folder dialog, with "Stop syncing". The other computers in a list: name,
  version, last heard. A new one has "Take in" and "Ignore". The notices,
  newest first, each with a link to its task or project.
- **The state line** at the bottom of the navigation, in words.
- **Conversations of another computer**: "Runs on Desk" and the time of the
  last news where the reply box is; the buttons that act on the turn are
  not shown. A running turn shows its steps as they arrive.
- **History of a task**: the computer next to each version.
- The project views reload on `data-changed`.

**Tests**: the component tests of the features (the state line for each
phase, a conversation of another computer without a reply box), and the
interface is looked at in the browser on the demo data, in both themes.

## Step 9: writing it down

The storage README (field stamps, taking in, the folder), the application
and desktop READMEs, the overview's "Data and sync", the manual, the marks of
M7 in the roadmap, a chapter in the story, and decision 0017 set to accepted
with what changed while it was built.

## What changes in the existing code

- `WorkService::recover` and `close` end every running turn. They must end
  only those of this computer (step 1).
- Nothing says which computer a conversation belongs to (step 1).
- Only `TaskRecord` can be read back from JSON; the other records only
  write (step 3).
- The store functions write rows and log in one piece; taking in needs the
  writing alone (step 3).
- `OperationRecorder` stamps a whole operation, not its fields; the log has
  no number of the operation on its own computer and no way to mark an
  operation as waiting (step 2).
- `ProjectRecord` carries `next_task_number`, so the project is written with
  every new task. Merged per record, this would lose renames; merged per
  field with "larger wins", it is harmless (step 2).
- Rules that span records (unique key, unique number, no cycles, a task in a
  status of its workflow) are only checked by the use cases. Taking in needs
  repairs for them (step 4).
- Decision 0007 says "last writer wins per record". Decision 0017 changes
  that to per field.
- AGENTS.md rule 5, "every database write is logged", needs a sentence for
  the bookkeeping of syncing (see the open questions).

## Open questions for the review

1. **Merge per field instead of per record.** Recommended: yes; per record
   undoes the manager's edits in ordinary work.
2. **A deletion is final.** Recommended: yes, with a notice for the change
   that was dropped.
3. **Act on a conversation only on its home computer.** Recommended: yes for
   the first version; a reply from another computer, carried out by the
   home computer, comes with signed operations.
4. **No encryption and no signatures in the first version.** Recommended:
   yes, with accepting computers and a copy of the database before the first
   take-in; both come before projects are shared with people.
5. **All projects or none.** Recommended: all.
6. **The bookkeeping of syncing is not logged** (what was read, the
   computers, the notices), like `meta` today. Recommended: yes, and say so
   in AGENTS.md rule 5: the data the manager works with is logged, the
   bookkeeping of the log is not.
7. **The name of this computer.** Recommended: ask for it when the folder is
   chosen, with the system's name of the computer as a suggestion, because
   that name goes into the folder and often contains a person's name.
8. **The whole history goes out on the first sync**, not only the latest
   state. Recommended: yes; it is small today, and it gives every computer
   the history of every task.
9. **The folder code as a module of storage**, not a crate of its own.
   Recommended: a module; the files are another form of the operation log,
   which storage owns.
10. **Looking at the folder every 20 seconds** instead of watching it.
    Recommended: yes; it is simple, and fast enough for sync programs that
    take seconds themselves.
