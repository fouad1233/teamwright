# teamwright-domain

The Teamwright model and its rules. This crate knows what a project, a
workflow, a task, a team member, a conversation with a specialist or with
the Advisor, and a plan the Advisor proposes are, and what may and may not be
done with them. It does no input or output.

## What is in it

| Module | Concepts |
|---|---|
| `ids` | `ProjectId`, `StatusId`, `TaskId`, `SprintId`, `ViewId`, `AreaId`, `TagId`, `ChecklistItemId`, `MemberId`, `ConversationId`, `TurnId`, `HandoffId`, `TeamMessageId`, `MemoryEntryId`: one identifier type per kind of thing |
| `text` | `ProjectName`, `StatusName`, `TaskTitle`, `SprintName`, `ViewName`, `AreaName`, `TagName`, `ChecklistText`, `MemberName`: names that are never empty, never multi-line, never too long; `ModelName`: the name of a model |
| `color` | `Color`: the named colors a status or a team member can have |
| `workflow` | `Workflow`, `Status`, `StatusCategory` |
| `work_in_progress` | `WorkInProgressLimit`: the most tasks that should stand in a status at once; `StatusLoad`, `Fill`: how many tasks stand in each status against its limit ([decision 0033](../../docs/decisions/0033-work-in-progress-limits-and-templates.md)) |
| `template` | `ProjectTemplate`, `TemplateStatus`: what a new project starts from (Standard, Kanban, Scrum, Hardware product, Engineering): statuses with limits, views, areas, a first sprint ([decision 0033](../../docs/decisions/0033-work-in-progress-limits-and-templates.md)) |
| `project` | `Project`, `ProjectKey` |
| `task` | `Task`, `TaskDraft`, `Priority`, `Assignee`, `Schedule`, `Rank` |
| `sprint` | `Sprint`, `SprintState`, `SprintDates`, `SprintSummary`, `SprintUnit`: a stretch of time in which the team works on a part of the backlog; `StoryPoints`: how big a task is; `CountedTask`: a task as a sprint counts its work |
| `view` | `View`, `ViewKind`, `ViewFilter`, `ViewAssignee`: one way of looking at the tasks of a project, with a name, a filter and a place among the views of the project |
| `project_files` | `PathInFolder`: a path inside a folder of a project that cannot leave it by how it is written (no `..`, no root, no drive, no colon); `FolderEntry`, `EntryKind`, `listing_of`: what a folder holds as the files view shows it (noise left out by `is_noise`, folders first, at most `MOST_ENTRIES`); `runs_as_program`: the endings Teamwright does not open |
| `area` | `Area`: a part of a project with a job of its own, such as its firmware, with a name and a place among the areas of the project |
| `sprint_course` | `course_of`: what a sprint held at the end of each of its days, which tasks joined or left it and on which day, and which moved on when it was completed, read off the versions of the tasks (`TaskPast`) |
| `tag` | `Tag`: a word of a project with a color, put on its tasks ([decision 0030](../../docs/decisions/0030-tags-and-checklists.md)) |
| `checklist` | `Checklist` and `ChecklistItem`: the items of the checklist of a task, each a text that is done or not, in the order of their ranks |
| `task_history` | `TaskChange`, `history_of`: what changed from one version of a task to the next |
| `dependencies` | `DependencyGraph`: which task waits for which, for the tasks of a project or of a proposal |
| `integration` | `EventKind` (with stable names such as `task.status_changed`): the kinds of events a Slack channel or a web address can hear; `WebAddress`: an https address whose `Debug` form and `shown()` hide all but the host, since it may be a secret; `RepositoryName`: `owner/name` of a GitHub repository, also read from a git remote (`of_remote`) |
| `member` | `Member`, `MemberKind`: an AI agent or a person on the team of a project; `MemberProfile`, `Effort`: the model and the effort of a specialist; `MemberInstructions`: its instructions, in the manager's words |
| `ready_made` | `ReadyMadeSpecialist`: the twelve roles that ship with Teamwright, each with a description, instructions, a model and an effort ([decision 0031](../../docs/decisions/0031-instructions-and-ready-made-specialists.md)) |
| `conversation` | `Conversation`, `Turn`, `Step`, `TokenCounts`, `ContextUse` (how full the context of a session was after a turn; nearly full from 80 % of the window; `context_of` a conversation is that of its last turn that reported one): the work of an AI specialist or of the Advisor, one request at a time; `DecisionRequest`, `DecisionOption`, `DecisionAnswer`: a question a specialist asks the manager to decide |
| `planning` | `Interview`, `InterviewQuestion`, `InterviewAnswer`: the questions the Advisor asks before it proposes; `Proposal`, `ProposedMember`, `ProposedTask`, `ProposedOwner`: the team and the plan it proposes; `ProposalChange`: one change of the manager's to it before approving |
| `teamwork` | `Handoff`, `HandoffContent`: the result of a task for whoever builds on it (a summary, an interface contract, the files that hold it as `PathInFolder`s); `TeamMessage`, `MessageKind`, `MessageDelivery`: a question or a note from one specialist to another and when it reached its receiver; `SaidIn`: who said it, in which conversation and turn |
| `memory` | `MemoryEntry`, `MemoryKind`, `MemoryContent`: an entry of the memory of a project, a decision, a fact or a contract, with a short title and a text; `MemoryOrigin`, `MemorySource`: where it came from (the manager, a specialist, an answered decision request, a handoff); `SearchWords`: the words of a search and whether an entry fits them ([decision 0036](../../docs/decisions/0036-project-memory.md)) |
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
- A status may carry a work-in-progress limit, a whole number from 1 to
  999. A status of the category done carries none, and loses its limit when
  it becomes one. A limit refuses nothing: `StatusLoad::of_workflow` only
  counts, every task of the project and no subtask apart from its task.
- A template makes a workflow, views and areas that follow these rules; a
  test passes every template through them. The first area of a template
  takes the identifier of the starting area, and a view of the six first
  kinds the identifier of its kind.
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
- A view is of one of twenty-one kinds (`ViewKind`): board, backlog, list,
  Gantt chart, calendar, dashboard, timeline, table, workload, team,
  activity, mind map, map, whiteboard, chat, doc, embed, form, gallery,
  report and files. A project has at most one backlog and one dashboard
  (`is_single`). Its name has at most 40 characters; a view named after its
  kind whose name is taken gets the first free number from 2 on, whatever
  the capitals. Only the kinds that show some of the tasks can be filtered
  (`can_be_filtered`: board, list, Gantt chart, calendar, timeline, table,
  workload, team, mind map, map and gallery), by whose tasks they show and
  whether finished tasks are hidden. A new project starts with one view of
  each of the first six kinds (`ViewKind::STARTING`), in that order, and
  the identifiers of these six follow from the project and the kind, so
  that two computers that each give a project its starting views give it
  the same ones. A view may keep content of its own (`View::content`):
  text the interface writes and the domain does not read, at most 2 MiB
  (`ViewContentTooLong`;
  [decision 0024](../../docs/decisions/0024-a-view-keeps-content-of-its-own.md)).
- An area's name has at most 40 characters. A new project starts with one
  area, "General", whose identifier follows from the project, so that two
  computers that each give a project its starting area give it the same
  one. A project left with no areas by two computers gets "General" again,
  with an identifier that also follows from the project. Every task is in
  exactly one area; putting it into another keeps its status and its place.
- A tag's name has at most 30 characters; a project has at most 100 tags
  (`Tag::MOST_IN_A_PROJECT`, checked by the use case), and a new tag
  without a color gets the next one of `Tag::COLORS_IN_TURN`. A task
  carries a set of tags (`put_tag`, `take_tag_off`, each saying whether
  anything changed). A task has one checklist: at most 100 items
  (`ChecklistFull`) of at most 200 characters, added at the end, ticked,
  renamed, moved and removed through the task (`NoSuchChecklistItem`,
  code `not_found`, for an item it does not have). Moving an item gives
  that item alone a new rank, unless no rank fits between its neighbours,
  when every item is spaced out again. A view can be filtered by one tag
  (`ViewFilter::tag`) and forgets a tag that was removed (`forget_tag`).
- The history of a task is not recorded as such. It is read off the versions
  the task has had: that it was opened, and then what differs from one
  version to the next. A move up or down within a status is not a change.
  A tag put on or taken off is (`Tagged`, `Untagged`), and so is an item
  of the checklist ticked or unticked (`Ticked`, `Unticked`); adding,
  renaming, moving or removing an item is not.
- A newcomer to a team gets a color that no other member has, as long as
  there is one left. Gray, amber and red are never handed out.
- Only an AI specialist has a model and an effort; a person's profile is
  always empty. The name of a model is one word of at most 80 characters.
- Only an AI specialist has instructions. They keep their lines, lose the
  space at both ends, and have at most 4,000 characters; empty means none.
- Every ready-made specialist has a name, a model and instructions that
  pass the rules of a member, a key and a name no other one has, and
  instructions that say when to ask the manager and what it cannot do.
  A name stands for a ready-made specialist when it is the same, whatever
  the capitals and the space at both ends.
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
- A handoff has a summary that says something (at most 4,000 characters),
  an interface contract that may be empty (at most 20,000), and at most
  fifty files, each a path that stays inside the folder by how it is
  written, none of them the folder itself and none named twice. It never
  changes once it is published.
- A message says something (at most 4,000 characters) and goes to another
  member than its sender. It is delivered once: delivering it again changes
  nothing.
- Before it is approved, the manager changes a proposal one
  `ProposalChange` at a time (`Proposal::change`, `Turn::change_proposal`):
  the title, description, size, priority or owner of a task, a task to wait
  for added or taken out, a task added at the end, a task or a member taken
  out, the model and effort of a member. The whole plan is checked again
  after each change, and a refused change leaves it as it was (`the
  proposal has no task 5`, `"Route the board" cannot wait for "Define the
  outline", which waits for it already`). Taking a task out takes out what
  waited for it and moves the places after it up by one; taking a member
  out leaves its tasks to nobody. Nothing is ever left empty.
- Tasks never wait for each other in a circle. `DependencyGraph` holds the
  rule once, for the tasks of a project (by identifier) and for those of a
  proposal (by place): `would_create_cycle` for one more dependency,
  `has_cycle` for a whole set.
- An entry of memory has a title of one line (at most 120 characters) and
  a text (at most 20,000), both saying something. It names the specialist,
  conversation and turn it came from exactly when it did not come from the
  manager. It is never changed in place: a newer entry replaces it, with
  its kind and its task, and the older one says which entry replaced it.
  Only a current entry can be replaced.
- A search in memory has one to twelve words in at most 200 characters,
  each counted once in small letters. An entry fits when each word begins
  a word of its title or its text, whatever the capitals.

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
