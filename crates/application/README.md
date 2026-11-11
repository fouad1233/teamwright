# teamwright-application

Teamwright's use cases: what the program can do, independent of any database,
window or agent runtime.

## What is in it

| Module | Contents |
|---|---|
| `projects` | `ProjectService`: start, find, list, rename, describe and archive projects |
| `tasks` | `TaskService`: open, change, move, connect and remove tasks |
| `team` | `TeamService`: put members on the team of a project, change them, take them off |
| `ports` | The interfaces the services need: `ProjectRepository`, `TaskRepository`, `MemberRepository`, `Clock` |
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

- Project keys are unique, including the keys of archived projects.
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
