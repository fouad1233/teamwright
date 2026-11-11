# teamwright-domain

The Teamwright model and its rules. This crate knows what a project, a
workflow and a task are, and what may and may not be done with them. It does
no input or output.

## What is in it

| Module | Concepts |
|---|---|
| `ids` | `ProjectId`, `StatusId`, `TaskId`, `MemberId`: one identifier type per kind of thing |
| `text` | `ProjectName`, `StatusName`, `TaskTitle`: names that are never empty, never multi-line, never too long |
| `color` | `Color`: the named colors a status or a team member can have |
| `workflow` | `Workflow`, `Status`, `StatusCategory` |
| `project` | `Project`, `ProjectKey` |
| `task` | `Task`, `TaskDraft`, `Priority`, `Assignee`, `Schedule`, `Rank` |
| `dependencies` | `DependencyGraph`: which task waits for which |
| `error` | `DomainError`: every rule that can be broken |
| `time` | `Timestamp`, `CalendarDate` |

## The rules it enforces

- A name is trimmed, on one line, not empty and within its length limit.
- A project key has 2 to 6 characters, starts with a letter and contains only
  capital letters and digits.
- A workflow has at least one "todo" status and at least one "done" status,
  and no two statuses share a name.
- A task number is handed out once and never reused.
- A task that reaches a "done" status gets a completion time, and loses it
  when it leaves.
- A schedule does not start after it is due.
- A task cannot wait for itself or be its own parent, and a new dependency
  must not close a loop.

## How to work in it

- A type keeps its fields private. Create values through the constructors
  (`parse`, `new`, `create`), which check the rules. Use `restore` only to
  rebuild a value from storage.
- Nothing here asks for the current time. Operations that need it take a
  `Timestamp` argument.
- Statuses carry a `StatusCategory`. Program logic must depend on the
  category, never on the name a manager gave the status.
- Enums that are stored have a stable text form (`as_str`, `parse`). Changing
  that text breaks existing data.

## Tests

```bash
cargo test -p teamwright-domain
```

Every rule above has at least one test with a name that states the rule.
