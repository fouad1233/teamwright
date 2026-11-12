# teamwright-application

Teamwright's use cases: what the program can do, independent of any database,
window or agent runtime.

## What is in it

| Module | Contents |
|---|---|
| `projects` | `ProjectService`: start, find, list, rename, describe, archive, restore and delete projects; add, change, move and remove the statuses of their workflows |
| `tasks` | `TaskService`: open, change, move, connect and remove tasks |
| `team` | `TeamService`: put members on the team of a project, change them, take them off |
| `ports` | The interfaces the services need: `ProjectRepository`, `TaskRepository`, `MemberRepository`, `Clock` |
| `agent_runtime` | `AgentRuntime`: the interface to the program that runs a team member's session, and the data it speaks |
| `error` | `ApplicationError`, with a stable `code()` for interfaces |
| `clock` | `SystemClock`, the real clock |
| `in_memory` | In-memory repositories and a settable clock, for tests and demos |

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
- Deleting a project deletes its tasks and its team. The project itself goes
  last, so a deletion that fails half-way can simply be repeated.
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
- A task can be given to the manager or to a member of the team of its own
  project, and to nobody else.

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
