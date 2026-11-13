# teamwright-domain

The Teamwright model and its rules. This crate knows what a project, a
workflow, a task, a team member, a conversation with a specialist or with
the Advisor, and a plan the Advisor proposes are, and what may and may not be
done with them. It does no input or output.

## What is in it

| Module | Concepts |
|---|---|
| `ids` | `ProjectId`, `StatusId`, `TaskId`, `SprintId`, `ViewId`, `AreaId`, `MemberId`, `ConversationId`, `TurnId`: one identifier type per kind of thing |
| `text` | `ProjectName`, `StatusName`, `TaskTitle`, `SprintName`, `ViewName`, `AreaName`, `MemberName`: names that are never empty, never multi-line, never too long; `ModelName`: the name of a model |
| `color` | `Color`: the named colors a status or a team member can have |
| `workflow` | `Workflow`, `Status`, `StatusCategory` |
| `project` | `Project`, `ProjectKey` |
| `task` | `Task`, `TaskDraft`, `Priority`, `Assignee`, `Schedule`, `Rank` |
| `sprint` | `Sprint`, `SprintState`, `SprintDates`, `SprintSummary`, `SprintUnit`: a stretch of time in which the team works on a part of the backlog; `StoryPoints`: how big a task is; `CountedTask`: a task as a sprint counts its work |
| `view` | `View`, `ViewKind`, `ViewFilter`, `ViewAssignee`: one way of looking at the tasks of a project, with a name, a filter and a place among the views of the project |
| `area` | `Area`: a part of a project with a job of its own, such as its firmware, with a name and a place among the areas of the project |
| `task_history` | `TaskChange`, `history_of`: what changed from one version of a task to the next |
| `dependencies` | `DependencyGraph`: which task waits for which |
| `member` | `Member`, `MemberKind`: an AI agent or a person on the team of a project; `MemberProfile`, `Effort`: the model and the effort of a specialist |
| `conversation` | `Conversation`, `Turn`, `Step`, `TokenCounts`: the work of an AI specialist or of the Advisor, one request at a time; `DecisionRequest`, `DecisionOption`, `DecisionAnswer`: a question a specialist asks the manager to decide |
| `planning` | `Interview`, `InterviewQuestion`, `InterviewAnswer`: the questions the Advisor asks before it proposes; `Proposal`, `ProposedMember`, `ProposedTask`, `ProposedOwner`: the team and the plan it proposes |
| `error` | `DomainError`: every rule that can be broken |
| `time` | `Timestamp`, `CalendarDate` |

## The rules it enforces

- A name is trimmed, on one line, not empty and within its length limit.
- A project key has 2 to 6 characters, starts with a letter and contains only
  capital letters and digits.
- A workflow has at least one "todo" status and at least one "done" status,
  and no two statuses share a name. Adding, changing, moving and removing a
  status keep it that way; a changed status may keep its own name.
- Categories are ordered by how far along the work is: todo, active, review,
  done. A new status goes behind the last status that is as far along, or
  less far, so a new "active" status lands in front of review and done.
- A task number is handed out once and never reused.
- A task that reaches a "done" status gets a completion time, and loses it
  when it leaves.
- A schedule does not start after it is due.
- A task cannot wait for itself or be its own parent, and a new dependency
  must not close a loop.
- A task is in one sprint or in none, which is the backlog, and has story
  points or none: a whole number from 0 to 999. A new task starts in the
  backlog without points.
- A project counts its sprints like its tasks: a sprint left without a name
  is "Sprint N", and a deleted sprint still counts.
- A sprint name has at most 60 characters. A sprint does not start after it
  ends. A sprint is planned, active or completed, in that order. A planned
  or an active sprint can be changed, but an active one keeps both its
  dates; only a planned one can be deleted or started, and only an active
  one completed. A completed sprint is not touched again: it is the record
  of what was done, with a summary of the work committed and completed,
  counted in points if any of its tasks has points and in tasks otherwise.
  The refusals about the state of a sprint have codes of their own
  (`DomainError::code`): `sprint_completed`, `sprint_active`,
  `sprint_not_active`.
- A view is a board, a backlog, a list, a Gantt chart, a calendar or a
  dashboard. Its name has at most 40 characters; a view named after its
  kind whose name is taken gets the first free number from 2 on, whatever
  the capitals. Only a board, a list, a Gantt chart and a calendar can be
  filtered, by whose tasks they show and whether finished tasks are hidden.
  A new project starts with one view of each kind, in that order, and the
  identifiers of these six follow from the project and the kind, so that
  two computers that each give a project its starting views give it the
  same ones.
- An area's name has at most 40 characters. A new project starts with one
  area, "General", whose identifier follows from the project, so that two
  computers that each give a project its starting area give it the same
  one. A project left with no areas by two computers gets "General" again,
  with an identifier that also follows from the project. Every task is in
  exactly one area; putting it into another keeps its status and its place.
- The history of a task is not recorded as such. It is read off the versions
  the task has had: that it was opened, and then what differs from one
  version to the next. A move up or down within a status is not a change.
- A newcomer to a team gets a color that no other member has, as long as
  there is one left. Gray, amber and red are never handed out.
- Only an AI specialist has a model and an effort; a person's profile is
  always empty. The name of a model is one word of at most 80 characters.
- A turn of a conversation has a request that is not empty. It takes steps
  only while it runs, a tool use is finished only if the turn has it, and the
  turn ends exactly once: finished, failed or stopped.
- A finished turn does not show its report twice: a last text step that is
  the answer is dropped.
- A question to the manager is not empty and has two to four options, each
  with a label; a recommended option is one of them. A turn asks it while it
  runs, and at most once. It is answered after the turn has ended, once,
  with one of its options, with words of the manager's own, or with both,
  never with neither.
- A conversation is with a specialist or with the Advisor. One with the
  Advisor has no member and no task, and keeps the model and the effort the
  Advisor runs on in it.
- A turn carries at most one of a question to decide, an interview and a
  proposal, and records it only while it runs.
- An interview has one to five questions. Each is not empty and has two to
  four options, or none for a question answered in words; a recommended
  option is one of them. It is answered after its turn has ended, once,
  with one answer for each question, each an option of that question, words,
  or both (`every question needs an answer`, `question 2 has no such
  option`, `the answer to question 3 names one of the options or says
  something in words`).
- A proposal has a summary and at least one member or task. No two of its
  members have the same name, whatever the capitals. A task waits only for
  other tasks of the same proposal, by their places, and the tasks do not
  wait for each other in a circle. Names and titles follow the rules of
  members and tasks. It is approved after its turn has ended, once, and
  then keeps what the manager approved, which the same rules check.

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
