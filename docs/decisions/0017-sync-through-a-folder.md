# 0017. Keep computers in step through files of operations in a folder the user owns

- Status: proposed
- Date: 2026-11-12
- Changes: [0007](0007-local-first-storage.md), which merged "last writer
  wins per record"; this record merges per field and says how

This record is longer than most, because it is the whole design of one
feature and a reviewer must see it in one place. How it is built, step by
step, is in [the sync plan](../architecture/sync-plan.md).

## Context

[Decision 0007](0007-local-first-storage.md) settled the shape: each computer
keeps its own SQLite database, every write is also an operation in a log with
the identity of the computer and a hybrid logical clock stamp, and computers
exchange operations through a folder the user owns. Each computer writes only
its own files there. The log exists; the exchange does not.

What the code does today, and what is in the way:

- **An operation holds the whole state of one thing** (a project with its
  workflow, a task with what it waits for, a member, a conversation, a turn,
  one step, a question, an interview, a proposal, the folders of a project).
  This is good: a later operation says everything an earlier one said.
- **Merging whole records loses too much.** A project is written every time
  one of its tasks is created, because it counts the task numbers. With "last
  writer wins per record", creating a task on the laptop would undo a rename
  of the project made on the desktop. In the same way, a specialist on the
  desktop that moves a task into review would undo the description the
  manager wrote on the laptop a minute before.
- **Some rules span several records** and cannot hold when two computers work
  apart: a project key is unique, a task number is unique in its project, a
  task never waits for itself through a chain, a task is in a status of its
  workflow, and (with sprints) a project has at most one active sprint.
- **A specialist is a session of Claude Code on one computer**
  ([0010](0010-one-process-for-each-turn.md)). The session lives in the
  files of Claude Code on that computer and works in a folder of that
  computer ([0013](0013-the-folders-of-a-project.md)). Nothing records which
  computer a conversation belongs to.
- **At every start, Teamwright ends every turn that storage calls running**
  (`WorkService::recover`). Once turns arrive from another computer, this
  would end the turns that the other computer is running right now.
- **The folders of a project are logged** as the entity `project_places`,
  and must never be applied on another computer.

This is one manager with a few computers, not a team. Two computers rarely
change the same thing at the same moment, but they will, because the
specialists on one computer keep writing while the manager looks at the
project from another.

## Decision

### 1. What a computer writes into the folder

The manager chooses one folder. Teamwright writes only below
`devices/<identity of this computer>/` in it and never changes or removes
anything else.

```
<chosen folder>/
  devices/
    0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10/
      computer.json
      log/
        000000000001-000000004211.jsonl
        000000004212-000000004390.jsonl
        000000004391-000000004402.jsonl
```

- **`computer.json`** says who the computer is: its identity, the name the
  manager gave it, the version of Teamwright, the format of its files, and
  when it last wrote. Only its owner writes it.
- **A log file** holds operations of this computer, one JSON object per line.
  Its name is the first and the last sequence number it holds, padded to
  twelve digits, so names sort in order and a reader knows from the name
  alone whether it needs the file. The first line is a header (format,
  identity of the computer, first and last number). The last line is a
  footer with the number of operations and the number of bytes before it.
- **An operation line** has the sequence number on its computer, the stamp,
  the entity, its identifier, `upsert` or `delete`, the state (for an
  upsert) and the stamps of the fields of the state that are older than the
  operation itself (see 3).
- **A file never grows and never changes.** New operations go into a new
  file. It is written under a temporary name that starts with a dot, closed,
  and then renamed. A reader takes only names of the exact pattern, and only
  a file whose footer matches what it read. A file that a sync program is
  still copying, a half-copied file, a "conflicted copy" that a sync program
  made, and a file of some other program are all passed over and looked at
  again next time.
- **When.** A computer writes a file a few seconds after its last change,
  and at most every 30 seconds while a specialist streams its work.
- **Long logs.** When a computer has more than 64 small files, it writes one
  file that covers all of them, and then removes the small ones. Both have
  the same operations, so a reader that reads both at once loses nothing.
  The first version keeps every operation forever; a file with only the
  latest state of each thing is designed (section 8) but left out.

Because every file has one writer and no file is ever changed after its
name appears, there is nothing for a sync program to merge, and nothing two
sync programs at once can break.

### 2. How a computer takes in what others wrote

- **Which computers.** A computer reads the folders of the computers the
  manager has accepted. A folder of a computer it does not know is shown to
  the manager first ("Desk is in the sync folder. Take in its projects?").
- **What it has read.** For each accepted computer it keeps the highest
  sequence number taken in. It reads only files whose last number is
  higher, and takes only the operations above that number.
- **In which order.** All new operations of one round, from all computers,
  are taken in by stamp. A computer that saw a change always stamps later
  than that change, so a project comes before its tasks and a task before
  the change that depends on it.
- **Twice changes nothing.** An operation is known by the computer that
  made it and its sequence number there. It is stored in the local log once,
  and the merge (section 3) gives the same result however often it runs.
- **Missing parents.** An operation for a thing whose parent is not known
  yet (a task whose project has not arrived) is kept and marked as waiting,
  and tried again in every round.
- **Unknown kinds.** An operation for an entity this version does not know
  is kept, marked as waiting, and applied after an update (section 5).
- **The clock.** For every operation taken in, the clock moves on to at
  least its stamp, so the next change made here is stamped later than
  everything seen. A stamp more than five minutes ahead of this computer's
  own time is still taken in, and the manager is told that the other
  computer's clock is ahead.
- **One transaction for each batch.** Applying a batch, storing its
  operations and moving the "read up to" numbers happen in one transaction,
  so a crash leaves either all of it or none.
- **Never the folders.** Operations of `project_places` are never written
  into the folder and are passed over if they are found there.

### 3. When two computers changed the same thing

**Last writer wins per field, not per record.** Each kind of thing is split
into fields that merge on their own. Each field keeps the stamp of the
operation that last set it. When an operation is written, the fields that
changed get its stamp and the others keep theirs; the operation carries all
of them. When an operation is taken in, each field takes the incoming value
if its stamp is later than the stamp it has. This gives the same result in
any order, any number of times, and an operation that is missing a field
(written by an older version) simply says nothing about it.

| Thing | Who changes it | Fields that merge on their own | Special |
|---|---|---|---|
| Project | any computer | key; name; description; archived; the workflow (all statuses together) | the next task number takes the larger of the two |
| Task | any computer | title; description; status with completion; priority; who is responsible; parent; what it waits for; estimate; dates (start and due together); rank; sprint; points; number | last update takes the later moment |
| Member | any computer | name; responsibility; model and effort together; color | last update takes the later moment |
| Sprint | any computer | name; goal; dates; state; position | |
| Conversation | the computer it runs on | member; task; session; the Advisor's model and effort | home computer never changes |
| Turn, step, question, interview, proposal | the computer it runs on | the whole record as one field | |
| Folders of a project | never leaves the computer | | |

Fields that belong together (a start and a due date, a status and the moment
of completion) merge as one, so a rule between them cannot break.

**A deletion is final.** A deleted project, task, member, sprint,
conversation or turn stays deleted, and a later change to it, from anywhere,
is dropped. Identifiers are never used twice, so this cannot hide a new
thing. The one exception is a step of a turn, whose identifier is its
position: a step can be written again after it was removed, and the later
stamp wins. A deletion carries, for each computer, the latest change of that
thing it had seen. A change it had not seen is a change the deletion threw
away, and the manager is told.

**Rules across records are repaired, never refused.** After a batch is
applied, the computer checks the rules that span records and repairs what
broke, as an ordinary logged change of its own:

| What broke | Repair | Who repairs |
|---|---|---|
| A task is in a status the workflow no longer has | the first status of the same category, else the first status | any computer; all reach the same value |
| A task points to a deleted member, sprint, parent or task it waited for | that reference is removed | any computer |
| A conversation is about a deleted task, or its project is gone | the conversation is deleted, as it is today | any computer |
| Two tasks of a project have the same number | the task whose number was set later gets the next free number | only the computer that created that task |
| Two projects have the same key | the project whose key was set later gets the key with a number, as for a new project | only the computer that created it |
| A chain of waiting or of parents closes on itself | the link set last is removed | any computer |
| Two sprints of a project are active | the one started later goes back to planned | any computer |

A repair that several computers make is an ordinary change. Most give the
same value everywhere; where two differ (one computer knew the category of
the removed status, the other did not), the later repair wins like any other
change, so the computers still end up equal. A repair that would hand out
something unique (a new number, a new key) is made by one computer only: the
one that created the thing, which is the computer of its first operation.

**What the manager is told.** A short list of notices, kept on this computer:
a task or a project that got a new number or key, a change that was dropped
because the thing was deleted elsewhere, a task that moved because its
status was removed, another computer's clock. A field that simply lost to a
later change is not announced. Both values stay in the history of the task,
with the computer that wrote each.

### 4. What is carried over

| Kind | Carried | Why |
|---|---|---|
| Projects with their workflows | yes | they are the work |
| Tasks, what they wait for, their history | yes | the history is the log itself |
| Members of the teams | yes | |
| Sprints, a task's sprint and its points | yes | the same rules as tasks |
| Conversations, turns, steps, questions | yes, to be read | the manager sees the work wherever they are |
| Interviews and proposals of the Advisor | yes, to be read | |
| The identifier of a Claude Code session | yes, as data | it is useless elsewhere and harmless; it lets a later version hand a conversation over |
| The folders of a project | no | a path on one computer means nothing on another ([0013](0013-the-folders-of-a-project.md)) |
| The files in the work folder | no | they are the manager's files; git, or the sync program itself, carries them |
| Notices, the sync settings, what was read | no | they describe this computer's view |

**A conversation has a home computer**: the one it was started on, where its
Claude Code session lives. Only there can a turn be started, a question or
an interview answered, a proposal approved, or a turn stopped. On another
computer the conversation can be read, with a plain sentence that says where
it runs. Approving a proposal on two computers would otherwise create the
same tasks twice, and a reply could not resume a session that is not there.

**A turn running on another computer** is shown as running there, with the
time of its last news. It moves as the files arrive, which is as fast as the
sync program is. Teamwright never ends a turn whose home is another
computer, at start, at close, or ever. A guard that protects work in
progress (a member who is working cannot be removed) counts the turns of
every computer, so the laptop refuses what would break the desktop's work.

**Going on with a conversation on another computer** is not possible in the
first version. Claude Code keeps a session in its own files on its computer,
and resuming needs those files and the same work folder. A later version can
hand over: start a new conversation on this computer with a brief made from
the turns that were carried over.

### 5. Versions

- **The format of the files** has a number, in each header and in
  `computer.json`. It changes only when the meaning of an existing field
  changes or a field is removed. Adding a field or an entity does not
  change it.
- **An older version meets a new field**: it merges it with its stamp like
  any other field and keeps it in the merged state, but has no column for
  it. When the version that knows the field is installed, its migration
  fills the column from the kept state.
- **An older version meets a new entity**: it keeps the operations,
  waiting, and the newer version applies them at its first start.
- **An older version meets a newer format**: it stops reading that computer
  at the first such file and tells the manager to update Teamwright on this
  computer. Its own changes are still written; the newer computer reads
  every older format.
- **A newer version meets old operations**: a missing field says nothing;
  operations written before syncing existed have no field stamps, so each
  field counts as set by the operation itself.

### 6. Privacy and safety

- **What leaves the computer** is everything in section 4: project names,
  tasks, members, sprints, and the full text of conversations, which can
  quote files the specialists read. Also the computer's name, the version
  of Teamwright and the identity of the computer.
- **Where it goes**: into the chosen folder, nowhere else. Teamwright makes
  no network connection for syncing. Whatever the sync program does with
  the folder is the sync program's business and the manager's choice.
- **What is never written**: paths of folders, the work folder's files, the
  credentials of Claude Code, anything outside `devices/<own identity>/`.
- **What an operation taken in can do**: change data in Teamwright, nothing
  else. It never starts a turn, never runs a program, never reads or writes
  a file outside the database, and never answers for the manager.
- **A person who can read the folder** can read all of the above. Nothing is
  encrypted in the first version.
- **A person who can write into the folder** can change or delete any
  project on every computer that takes in, including by writing into the
  folder of an accepted computer. Accepting computers prevents mixing up two
  libraries by mistake, not an attack. Before it takes in anything for the
  first time, Teamwright copies its database file next to itself, and the
  local log keeps every state a thing ever had, so data can be recovered by
  hand.
- **Deliberately not protected against in the first version**: someone with
  write access to the folder, a sync provider that reads the files, and a
  stolen copy of the folder. The answer to those is encryption and signed
  operations, which belong with sharing a project between people.
- **A copied database.** If the data folder of one computer is copied to
  another, both have the same identity. A computer remembers every file it
  wrote; a file in its own folder that it did not write means that another
  computer uses its identity. Syncing then stops, and the manager can give
  this computer a new identity. Its old identity becomes another computer
  whose files are read like any other, so nothing is lost. A database
  restored from a backup is found the same way.

### 7. The interface, in outline

- **Computers** is a new place, reached from the bottom of the navigation:
  the name of this computer, the sync folder (chosen with the system's
  folder dialog, as the folders of a project are), the other computers with
  their names, versions and when they were last heard from, a button to
  accept a new one, and the notices.
- **The state of syncing** is one quiet line at the bottom of the
  navigation: "In step, 2 min ago", "Sending", "Folder not reachable",
  "Desk uses a newer Teamwright", "Stopped: this computer's identity is in
  use elsewhere". Nothing blinks; a problem is shown in words.
- **Viewing a project that runs on another computer** means: the board, the
  tasks, the team and the conversations are all there and can be changed
  like any other, except the conversations, which show "Runs on Desk" with
  the time of the last news instead of a reply box. A turn running there
  shows its steps as they arrive. The history of a task names the computer
  of each version.

### 8. Left out of the first version

- Replying, answering, approving or stopping on a computer that is not the
  conversation's home; handing a conversation over to another computer.
- Carrying the files of the work folder, and the folders of a project.
- Encryption, signed operations, and protection against a person who can
  write into the folder.
- Files with only the latest state of each thing, removing old operations,
  and forgetting deleted things. The log and the folder only grow, slowly.
- Choosing which projects are synced: all are, or none.
- Merging two edits of the same text. One whole description wins.
- Showing two conflicting values side by side. The history has both.
- Several people. The design is for one manager.
- Watching the folder for changes. It is looked at every 20 seconds.

## Consequences

- A manager can work on any of their computers and see the work of the
  specialists on all of them, with no account and no server.
- Every write keeps a stamp for each field of what it wrote. Storage gains a
  table of merged states and the operation log gains a few columns; the
  repositories keep calling the same recorder.
- Applying an operation from elsewhere is a new way of writing rows. Each
  repository must be able to write its rows from a stored state without
  logging a new operation, and its record types must be readable from JSON.
- Repairs mean that taking in can itself produce changes, which go out in
  the next round. They settle, because a repair only removes a broken
  reference or hands out a free value, and never breaks another rule.
- A conversation now belongs to a computer, and the program never ends a
  turn that belongs to another one. This changes `recover` and `close`.
- Every new kind of thing, sprints first, must say how it merges and which
  computer may change it. The list in section 3 is the place.
- A task number can change after it was shown, in the rare case that two
  computers created a task in the same project before they met.

## Alternatives considered

- **Last writer wins per record, as 0007 said.** Simplest, and wrong for
  this program: projects are written with every new task, and tasks are
  written by the specialists, so ordinary work on one computer would undo
  the manager's edits on the other.
- **Each operation lists the fields it changed, without stamps.** A reader
  could merge per field too, but only by reading every operation; a state
  that carries its stamps can be merged alone, which is what a later file of
  latest states needs.
- **A deletion that a later change undoes.** Kinder to a late edit, but a
  deleted project would come back as a shell without its tasks, and a
  deleted task without its conversations. Final deletion with a notice is
  easier to understand.
- **One growing file per computer.** Fewer files, but a sync program copies
  it while it grows, and the other side reads a torn last line; some sync
  programs upload the whole file again after every change.
- **Ranges of task numbers for each computer, or numbers with the computer's
  letter.** No renumbering, but keys like `SSN-1012` or `SSN-12b` for the
  common case of one computer, to save the rare case of two.
- **Running a reply from another computer by an order in the folder** that
  the home computer carries out. Useful later; it makes the folder a way to
  start programs, and needs the protection the first version leaves out.
- **The database in the folder, the Google Drive API, a server, a CRDT
  library**: rejected in 0007 for reasons that still hold.
