# teamwright-application

Teamwright's use cases: what the program can do, independent of any database,
window or agent runtime.

## What is in it

| Module | Contents |
|---|---|
| `projects` | `ProjectService`: start, find, list, rename, describe, archive, restore and delete projects; add, change, move and remove the statuses of their workflows |
| `tasks` | `TaskService`: open, change, move, connect and remove tasks, and tell the history of one from the versions storage has kept |
| `team` | `TeamService`: put members on the team of a project, change them, take them off |
| `places` | `PlacesService`: the folders of a project: choose its work folder, add and take away reference folders, copy reference files into it |
| `work` | `WorkService`: let AI specialists work: start on a task, ask, reply, answer a specialist's question, list what waits for the manager, stop, follow every event, open a session in the desktop app, recover after a restart; `brief_for` and `instructions_for` |
| `ports` | The interfaces the services need: `ProjectRepository`, `TaskRepository`, `MemberRepository`, `ConversationRepository`, `PlacesRepository`, `ProjectFolders`, `FileSystem`, `WorkListener`, `Clock` |
| `agent_runtime` | `AgentRuntime`: the interface to the program that runs a team member's session, and the data it speaks |
| `error` | `ApplicationError`, with a stable `code()` for interfaces |
| `clock` | `SystemClock`, the real clock |
| `in_memory` | In-memory repositories, fixed project folders, a pretended file system, a listener that records, a scripted agent runtime and a settable clock, for tests and demos |

## How a use case is built

Every use case follows the same three steps:

1. Load what is needed through the repositories.
2. Let the domain types apply their rules.
3. Store the result and return it.

```rust
pub fn rename(&self, id: TaskId, title: &str) -> Result<Task, ApplicationError> {
    let title = TaskTitle::parse(title)?;      // the domain checks the title
    self.change(id, |task, now| {              // load, change, store
        task.rename(title, now);
        Ok(())
    })
}
```

Rules that concern a single value live in the domain. Rules that need to look
at several stored things live here: a project key must be unique, a
dependency must not close a loop, a subtask and its parent must share a
project, two members of one team cannot have the same name.

## What the services guarantee

- Project keys are unique, including the keys of archived projects. The key
  of a deleted project is free again.
- Archived projects are listed with the one put away last first. Restoring
  one brings it back with its tasks and its team.
- Deleting a project deletes its tasks, its team and what it knew about its
  folders; the folders on disk stay. The project itself goes last, so a deletion that fails half-way can simply be repeated.
- Changing a status between "done" and not done completes or reopens every
  task in it, subtasks included.
- A status that still has tasks is removed only when another status is
  named for them (`status_in_use` otherwise). They go to the bottom of that
  status in their order, and are completed or reopened by its category.
- Task numbers count up per project and are never reused.
- A new task goes to the bottom of its status.
- Moving a task changes only that task, except in the rare case where its new
  neighbours have no rank left between them. Then the whole status is spaced
  out again in one atomic write.
- A dependency that would leave tasks waiting for each other is refused, and
  the error names both tasks the way the manager knows them (`IOT-3`).
- Deleting a task deletes its subtasks and releases the tasks that waited for
  any of them.
- Every project has a team of its own. No two members of a team share a
  name, whatever the capitals.
- A newcomer to a team gets a color that sets it apart from the others.
- Taking a member off a team gives the member's tasks to nobody.
- `TeamService::set_profile` (and `add`) give an AI specialist a model and
  an effort; an empty model means none. A person given either is refused
  (`no_specialist`). A turn runs on the specialist's model and effort, or on
  the program's defaults when it has none.
- A task can be given to the manager or to a member of the team of its own
  project, and to nobody else.
- Work in progress is protected. A member with a running turn cannot leave
  the team (`member_busy`). A task, or the parent of a task, that a running
  turn is about cannot be deleted, and a project in which a turn runs can be
  neither archived nor deleted (`work_in_progress`).
- Deleting a task deletes the conversations about it and its subtasks.
  Deleting a project deletes its conversations. A member that leaves leaves
  its conversations behind, without a member.

## The folders of a project

`PlacesService` carries out
[decision 0013](../../docs/decisions/0013-the-folders-of-a-project.md). A
project has a work folder, in which its specialists read and write, and
reference folders, which they may only read. `ProjectPlaces` is what the
manager sees; `ChosenPlaces` is what `PlacesRepository` stores.

- The work folder is the one the manager chose, or else the one Teamwright
  gives the project (`ProjectFolders`). `work_folder_of` says which.
- `set_work_folder` takes a folder, or `None` to go back to the one
  Teamwright gives. It is refused while a specialist works in the project
  (`work_in_progress`).
- A folder must be named (`rule_broken`) and must exist
  (`folder_not_found`). The path is kept as the manager gave it, trimmed.
- Two folders are compared by their full forms (`FileSystem::resolve`),
  part by part, so that `/work` does not contain `/workshop`. A reference
  folder cannot be the work folder, lie inside it, or contain it
  (`rule_broken`). A folder that no longer exists is compared as it was
  named.
- Adding a reference folder twice keeps it once. Taking away one the project
  does not have changes nothing. Neither touches the folder itself.
- `add_reference_files` copies files into the folder `references` inside the
  work folder and answers with the names they got. A name that is taken
  there, by a file on disk or an earlier file of the same call, gets a
  number in front of its ending, as in `datasheet (2).pdf`; capitals do not
  make a name different. A path without a file name is refused before
  anything is copied (`rule_broken`); a file that cannot be read fails as
  storage (`storage_failed`) with its path in the message.
- Teamwright never removes a file or folder of the manager.

## The work of the specialists

`WorkService` lets the AI specialists of a team work. A conversation is one
session of the agent program with one specialist, about a task or about a
question; every request to the specialist is a turn of it.

- `start_work` needs a task given to a member of the kind agent
  (`no_specialist`), on which work has not started before
  (`work_already_started`). It opens a conversation and starts its first turn
  with the brief of the task (`brief_for`).
- `reply` starts a new turn in the same session with the manager's words,
  trimmed; `ask` opens a conversation without a task.
- A specialist works on one thing at a time (`member_busy`). Whether it is
  busy is read from storage: a turn in the state running.
- A turn runs with `PermissionProfile::specialist()`, in the work folder of
  the project (`PlacesService::work_folder_of`), with the reference folders
  that still exist as `readable_folders`, and with instructions that say who
  the specialist is and how the team works (`instructions_for`). The
  instructions list the reference folders, if there are any, as readable
  and not to be changed, and always say that single files the manager
  handed over are copies in the folder `references`. A reference folder is
  given to the turn, and named in the instructions, in its full form
  (`FileSystem::resolve`), whatever way the manager named it: the agent
  program compares paths as they are written, and refuses a folder that a
  specialist names in another way than the program was given it. Opening a session in the
  desktop app uses the same work folder. The first turn starts a new
  session; later turns resume it by the identifier the program reported.
- Every event of the runtime is recorded in the turn, the turn is stored,
  and the `WorkListener` is told with the whole conversation. Plan usage is
  not shown yet.
- When a turn starts on a task in a todo or review status, the task moves to
  the bottom of the first active status. When a turn finishes without
  failure, a task in an active status moves to the bottom of the first
  review status, if the workflow has one. Nothing here moves a task to done,
  and a failed or stopped turn leaves the task where it is.
- Every turn may ask the manager to decide (`may_ask_manager`). The
  question it asks is recorded on the turn, and a turn that finishes with a
  question leaves its task where it is: it is not ready to be checked.
- A question is open while it is the question of the latest turn of its
  conversation, that turn has ended, and the manager has not answered.
  `answer_decision` answers it with an option (counted from 0), with words,
  or with both, and starts a turn of origin `Decision` whose request is
  `Decision: <option>`, followed by the words if there are any, or only the
  words (`no_decision_open`, `no_specialist`, `rule_broken`,
  `member_busy`). The answer is stored only if the new turn starts.
  `reply` is refused while a question is open (`decision_open`), after the
  checks it had before.
- `waiting_for_manager` lists the conversations with an open question in
  every project that is not archived, the one that has waited longest first.
- `stop` records the running turn as stopped by the manager and then stops
  the program. Stopping a conversation in which nothing runs changes nothing.
- `recover` ends, as failed, the turns that storage still calls running when
  the program starts: closing Teamwright cut them off.

The runtime calls the listener of a turn on a thread of its own. The service
holds one lock while a use case starts or stops a turn and while an event is
recorded, so each sees the stored turns whole. The `WorkListener` is called
under that lock: it must return quickly and must not call the service.

## The agent runtime

A team member's session runs on the user's own Claude Code
([decision 0005](../../docs/decisions/0005-drive-the-users-claude-code.md)).
The application only knows the trait `AgentRuntime`, implemented by the
`teamwright-runtime` crate. Through it, the application can:

- find the installed program and its version (`installation`),
- ask the program whether the user is signed in (`sign_in`); no credential
  is ever read or passed on,
- run one turn of a session (`start_turn`). The turn runs on its own thread
  and reports `AgentEvent`s to a listener: the session started, text, tool
  use and results, denied permissions, retries, plan usage, and last of all
  `TurnEnded` with the tokens per model. The returned `RunningTurn` can be
  stopped,
- open a session in the Claude desktop app (`open_in_desktop_app`).

A turn request may give the session the tool to ask the manager to decide
(`may_ask_manager`); a question it asks arrives as
`AgentEvent::ManagerAsked`, with the recommended option counted from 0. It
may also name folders the session may read but not write
(`readable_folders`).

A turn request carries its permissions as data, a `PermissionProfile`.
`PermissionProfile::specialist()` is the profile of a specialist: read,
search, and edit inside the working folder; no shell; everything else is
denied without waiting. The modes that approve risky tool use on the
manager's behalf are not offered.

Usage is counted in tokens
([decision 0008](../../docs/decisions/0008-budgets-in-tokens.md)). The dollar
figure is kept as `estimated_cost_usd`. Plan limits are reported as
`PlanUsage` when the program reports them, and are unknown otherwise.

## Adding a use case

1. Add a method to the service it belongs to, or a new service for a new
   area. Document its errors.
2. If it needs something new from the outside world, add a method to a port
   and implement it in `in_memory` and in the storage crate.
3. Write tests against the in-memory implementations. Name each test after
   the behavior it proves.

## Tests

```bash
cargo test -p teamwright-application
```
