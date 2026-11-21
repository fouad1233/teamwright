# teamwright-application

Teamwright's use cases: what the program can do, independent of any database,
window or agent runtime.

## What is in it

| Module | Contents |
|---|---|
| `projects` | `ProjectService`: start, find, list, rename, describe, archive, restore and delete projects, each new one from a template (`WorkflowTemplate`); add, change, move and remove the statuses of their workflows, set their work-in-progress limits, and count how full each status is (`status_loads`) |
| `tasks` | `TaskService`: open, change, plan into a sprint, give points, move, connect and remove tasks, and tell the history of one, or of a whole project, from the versions storage has kept; tell what really happened to each task against its plan (`TaskActual`) |
| `sprints` | `SprintService`: list, plan, change, delete, start and complete the sprints of a project, and the report for the burndown chart of one (`SprintReport`), read off the versions of every task of the project, so that it follows the tasks that joined or left the sprint and tells which moved on at its completion; and the forecast of a project (`SprintForecast`): the pace, each planned sprint against it, and how many sprints the unfinished work would take, with the tasks without points named |
| `views` | `ViewService`: list, add, rename, filter, give content to, move and remove the views of a project |
| `areas` | `AreaService`: list, add, rename, move and remove the areas of a project, and move a task with its subtasks into another area |
| `tags` | `TagService`: list, add, rename, recolor and remove the tags of a project, and put a tag on a task (by itself, or by its name, making it if need be) or take it off |
| `checklists` | `ChecklistService`: add, rename, tick, move and remove the items of the checklist of a task, each answering with the task |
| `team` | `TeamService`: put members on the team of a project, change them, take them off |
| `places` | `PlacesService`: the folders of a project: choose its work folder, add and take away reference folders, copy reference files into it |
| `files` | `ProjectFilesService`: what lies in the folders of a project, for the files view: `list` a folder at a path inside it, `locate` an entry to open or show, each path checked first |
| `work` | `WorkService`: let AI specialists work: start on a task, ask, reply, answer a specialist's question, list what waits for the manager, stop, follow every event, open a session in the desktop app, recover after a restart; `brief_for` and `instructions_for`. Its part `advisor`: plan a project with the Advisor (`advise`, `answer_interview`, `change_proposal`, `approve_proposal`); `ProposalEdit`, `ApprovedProposal`, `answers_in_words` and `instructions_for_advisor` |
| `digests` | `DigestService`: earlier conversations as digests ([decision 0041](../../docs/decisions/0041-earlier-conversations-as-digests.md)): `find_earlier` (grouped by folder, the folders of the project first), `draft`, `keep` (once per conversation), `digests_of`, `remove`; `group_by_folder`. The Advisor's first turn carries the kept digests (`digests_for_advisor`, at most `ADVISOR_DIGESTS_LENGTH` characters) |
| `teamwork` | `TeamworkService`: what specialists hand each other: every handoff and message of a project (`talk_of_project`, as `TeamTalk`) and the messages that wait for a member (`messages_waiting_for`, for the coordinator); the rules across stored things: `messages_waiting_for` (sent to the member, not delivered, the oldest first), `latest_handoff_of` (the handoff of a task that counts), `handoffs_in_words` (the part of a brief) and `messages_in_words` (what a turn is told) |
| `memory` | `MemoryService`: the memory of a project, as the manager keeps it: every entry (`memory_of_project`), a search (`search`), a fact the manager writes (`add_fact`, `NewFact`), a change, which is a newer entry that replaces the older (`change`), and a removal (`remove`); the rules across stored things: `decision_entry` (an answered decision request as a decision), `contract_entries` (the contract of a handoff, replacing the earlier contract of its task), `facts_remembered_in` and `MOST_FACTS_PER_TURN`, `search` (the entries that fit, newest first, over anything `Searchable`), `memory_in_words` (the part of a brief, within `MEMORY_IN_BRIEF_LENGTH`) and `remembered_entries` (what a search of the team bus reads) ([decision 0036](../../docs/decisions/0036-project-memory.md)) |
| `coordinator` | The coordinator ([decision 0037](../../docs/decisions/0037-the-coordinator.md)): `next_steps`, a pure rule that answers, from a `Situation`, which tasks start with which specialist (`Start`), which specialist gets a turn to answer a teammate's question (`Wake`, at most `MESSAGE_TURNS_PER_HOUR`), which used-up budgets are asked about, and what waits and why (`Waiting`, `WaitReason`); `spending_of` (tokens in all of ended turns, by project, member and task); `words` (the sentences of its record); `CoordinatorService`: switch on and off, the limit, budgets and the answers about them, `budget_questions` for the inbox, and `act`, which does what the rule says through `WorkService` on the computer it runs on and writes down what it did |
| `google_drive` | `GoogleDriveService`: the folder that Google Drive for desktop keeps on this computer (`find_drive_folders`, `look`, `prepare`), behind the port `DriveSurroundings`; see "Syncing through Google Drive" |
| `sync` | `SyncService`: keep several computers of one manager in step through a folder the manager owns (`round`, `choose_folder`, `stop`, `rename_this_computer`, `accept`, `ignore`, `notices`, `dismiss`, `new_identity`); the ports `SyncFolder` and `SyncLedger`, and the operations that travel between computers |
| `workbook` | `WorkbookService`: gather a project into a workbook of four sheets (summary, tasks, sprints, team) as plain data: `Workbook`, `Sheet`, `SheetContent`, `Section`, `Table`, `Column` and `Cell`, whose kind says whether it is text, a whole number, hours, a date or a status. Writing the file is done outside this crate ([decision 0025](../../docs/decisions/0025-the-project-as-a-workbook.md)) |
| `project_report` | `ProjectReportService`: gather the report of a whole project as plain data (`ProjectReport`: the brief, the work done by area and by holder, the plan against what happened, the sprints, the usage by who and by model, the decisions, the handoffs, what is open), and `report_in_markdown`, which writes it as a Markdown file ([decision 0042](../../docs/decisions/0042-the-report-of-a-whole-project.md)) |
| `ports` | The interfaces the services need: `ProjectRepository`, `TaskRepository`, `MemberRepository`, `ConversationRepository`, `PlanningRepository`, `PlacesRepository`, `SprintRepository`, `ViewRepository`, `AreaRepository`, `TagRepository`, `TeamworkRepository`, `DigestRepository`, `EarlierConversations` (with `FoundConversation`), `MemoryRepository`, `ProjectFolders`, `FileSystem`, `FolderReader`, `WorkListener`, `Clock` (which also tells the day a moment falls on) |
| `agent_runtime` | `AgentRuntime`: the interface to the program that runs a team member's session, and the data it speaks |
| `error` | `ApplicationError`, with a stable `code()` for interfaces |
| `clock` | `SystemClock`, the real clock |
| `in_memory` | In-memory repositories (`InMemoryPlanningRepository` stores an approved plan into the others, `InMemorySprintRepository` stores the tasks that change with a sprint into the task repository, `InMemoryViewRepository`, `InMemoryAreaRepository`, `InMemoryTagRepository`, `InMemoryTeamworkRepository`, `InMemoryDigestRepository`, pretended earlier conversations, `InMemoryMemoryRepository`), fixed project folders, a pretended file system, a listener that records, a scripted agent runtime and a settable clock, for tests and demos |

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
- Deleting a project deletes its tasks, its sprints, its views, its areas, its team
  and what it knew about its folders; the folders on disk stay. The project itself goes last, so a deletion that fails half-way can simply be repeated.
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
- `TeamService::set_instructions` gives an AI specialist instructions of its
  own; text that is only whitespace takes them away, a person given any is
  refused (`no_specialist`), and more than 4,000 characters is refused
  (`rule_broken`). `instructions_for` appends them to every turn of the
  specialist, after Teamwright's own instructions, framed so that they add
  to the rules and never replace them, and followed by a line that repeats
  the rules about the folder and about asking the manager
  ([decision 0031](../../docs/decisions/0031-instructions-and-ready-made-specialists.md)).
- `TeamService::add_ready_made` puts a ready-made specialist on the team in
  one step, with its name, description, instructions, model and effort; a
  name the team has is numbered ("Researcher 2"), and an unknown key is
  `not_found`. When a proposal of the Advisor is approved, a member whose
  name is exactly that of a ready-made specialist starts with its
  instructions, and the Advisor is told the names.
- A task can be given to the manager or to a member of the team of its own
  project, and to nobody else.
- Work in progress is protected. A member with a running turn cannot leave
  the team (`member_busy`). A task, or the parent of a task, that a running
  turn is about cannot be deleted, and a project in which a turn runs can be
  neither archived nor deleted (`work_in_progress`).
- Deleting a task deletes the conversations about it and its subtasks.
  Deleting a project deletes its conversations. A member that leaves leaves
  its conversations behind, without a member.

## Sprints and the backlog

`SprintService` and two methods of `TaskService` carry out
[decision 0016](../../docs/decisions/0016-sprints-and-a-backlog.md). The
demo backend of the interface (`demoSprints.test.ts`) is the
specification; `sprints/tests.rs` mirrors each of its cases under the same
name, with the same codes and messages.

- A task is in one sprint or in none, which is the backlog. A new task
  starts in the backlog without points; a new subtask starts in the sprint
  of the task it is a part of.
- `TaskService::plan` puts a task with its subtasks into a planned or an
  active sprint of its own project, or back into the backlog. A subtask is
  not planned by itself (`subtask_planned_alone`), a sprint of another
  project is refused (`different_projects`), a completed one too
  (`sprint_completed`).
- `TaskService::set_points` takes the points as any number, as the
  interface sends them, and refuses one that is not whole or not from 0 to
  999 (`rule_broken`). Planning and points are stored as changes to the
  task, but, as in the demo, they are not part of its history.
- A sprint left without a name is "Sprint N", N being one more than the
  number of sprints the project has had, deleted ones included.
- Only a planned sprint is deleted (`sprint_active`, `sprint_completed`
  otherwise); its tasks go back to the backlog in the same write.
- A sprint starts only with at least one task (`sprint_empty`) and while no
  other sprint of its project is active (`another_sprint_active`).
- Completing the active sprint sends its unfinished tasks, with their
  subtasks, to the backlog or to another sprint of the same project that is
  not completed, in the same write. Finished means a status of the category
  done, whatever its name. The summary counts the tasks without their
  subtasks, in points if any has points.
- The report gives, for each day from the start to the end date, the work
  that remained at the end of the day, from the moments the tasks of the
  sprint were finished; days that have not come, or came after the sprint
  was completed, have none. Days are counted where the program runs
  (`Clock::day_of`).

## The views of a project

`ViewService` carries out
[decision 0021](../../docs/decisions/0021-views-the-manager-can-add.md). The
demo backend of the interface (`demoViews.test.ts`) is the specification;
`views/tests.rs` mirrors each of its cases under the same name, with the
same codes and messages. The one about a project kept before it had views
is a migration of the database, and is tested with the migrations of the
storage crate.

- `ProjectService::create` gives a new project the views of its template,
  in the template's order, each named after its kind and showing every
  task. The standard template gives six, in the order board, backlog,
  list, Gantt chart, calendar, dashboard. `ProjectService::delete` removes
  them.
- A project has at most one backlog and one dashboard (`view_kind_taken`).
- A name is tidied and has at most 40 characters (`rule_broken`). Two views
  of a project do not share a name, whatever the capitals
  (`view_name_taken`); a view may change the capitals of its own. A view
  added without a name, or with an empty one, is named after its kind, with
  the first free number from 2 on if that name is taken.
- A view of any other kind can be added as often as the manager likes.
  None of the fourteen newer kinds (timeline to report) is given to a new
  project.
- Only the kinds that show some of the tasks can be filtered
  (`ViewKind::can_be_filtered`; `rule_broken` for the others), and a
  filter can only name a member of the team of the project
  (`member_not_on_team`).
- `set_content` replaces what a view keeps of its own, or clears it with
  nothing ([decision 0024](../../docs/decisions/0024-a-view-keeps-content-of-its-own.md)).
  Any kind may keep content; more than 2 MiB is `rule_broken`.
- `move_to` puts a place past either end at that end, stores only the views
  whose place changed, and answers with all the views in their new order.
- The last view of a project cannot be removed (`last_view`).
- `TeamService::remove` makes every view that showed only the tasks of the
  member who leaves show everybody's again, and keeps it hiding finished
  tasks if it did.

## Tags and checklists

`TagService` and `ChecklistService` carry out
[decision 0030](../../docs/decisions/0030-tags-and-checklists.md). The
demo backend of the interface (`demoTags.test.ts`) is the specification;
`tags/tests.rs` and the tests of `checklists.rs` follow its cases, with
the same codes and messages.

- A tag name is tidied and has at most 30 characters (`rule_broken`). Two
  tags of a project do not share a name, whatever the capitals
  (`tag_name_taken`); two projects may. A project has at most 100 tags
  (`rule_broken`). Tags are listed by name, whatever the capitals.
- A tag of another project is not put on a task, nor named by the filter
  of a view (`different_projects`); a tag that does not exist is
  `not_found`. `ViewService::new` takes the tags for that check.
- `put_on_by_name` finds the tag of the project with that name, whatever
  the capitals, or makes it with the next color in turn.
- `remove` takes the tag off every task that carries it, then out of the
  filter of every view that showed only the tasks with it, and then
  removes it. Deleting a project leaves its tags to storage, which removes
  and logs them with the project.
- An item of a checklist is tidied and has at most 200 characters; a
  checklist has at most 100 items (`rule_broken`). An item the task does
  not have is `not_found`. A change that changes nothing, such as ticking
  an item that is done, stores nothing.
- `WorkbookService` names the tags of each task, by name, in the column
  "Tags" of the Tasks sheet; its ports gain `tags`.

## Templates and work-in-progress limits

[Decision 0033](../../docs/decisions/0033-work-in-progress-limits-and-templates.md).
The tests are in `projects.rs`; the demo backend (`demoTemplates.test.ts`)
follows them case by case.

- `ProjectService::create` builds the project from `NewProject::workflow`,
  a `WorkflowTemplate` that names a `ProjectTemplate` of the domain: its
  workflow with its limits, its views, its areas, and for Scrum a planned,
  empty "Sprint 1", which counts as the first sprint of the project. No
  template gives a team. The project does not remember its template.
- `set_status_limit` gives a status a limit from 1 to 999, another one, or
  none. Out of range, or on a status of the category done, is
  `rule_broken`. A limit below the tasks already in the status is allowed,
  and nothing moves. `change_status` to the category done drops the limit.
- `status_loads` counts, for every status in board order, the tasks of the
  whole project in it (no subtask apart from its task) against its limit.
  Nothing anywhere refuses a task a full status: a limit is a signal.

## The areas of a project

`AreaService` carries out
[decision 0022](../../docs/decisions/0022-areas-inside-a-project.md). The
demo backend of the interface (`demoAreas.test.ts`) is the specification;
`areas/tests.rs` mirrors its cases under the same names, with the same
codes and messages. Two of them are tested in the storage crate: a project
kept before it had areas gets "General" from a migration of the database,
and a task whose area the project does not have can only come from another
computer, so that case is tested where storage takes in what another
computer did.

- `ProjectService::create` gives a new project the areas of its template:
  one, "General", for most, and Electronics, Firmware and Enclosure for a
  hardware product. `ProjectService::delete` removes the areas after the
  tasks.
- A name is tidied and has at most 40 characters (`rule_broken`). Two areas
  of a project do not share a name, whatever the capitals
  (`area_name_taken`); an area may change the capitals of its own.
- Every task is in exactly one area. `TaskService::create` puts a new task
  into the area it names, or into the first area of the project; a subtask
  goes into the area of its task, and naming another is refused
  (`rule_broken`). An area of another project is refused
  (`different_projects`). The tasks of an approved plan of the Advisor go
  into the first area.
- `move_to` puts a place past either end at that end, stores only the areas
  whose place changed, and answers with all the areas in their new order.
- `remove` refuses the last area of a project (`last_area`), checked first.
  An area that holds tasks, subtasks counted, is removed only when another
  area of the same project is named for them (`area_in_use`,
  `different_projects`); they keep their status and their place. An area
  without tasks is removed whatever is named.
- `move_task` moves a task with its subtasks and answers with the task; a
  subtask is not moved by itself (`subtask_moved_alone`), and a task that is
  in the area already is not changed. Moving a task is a change of the task,
  but, as in the demo, it is not part of its history.

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

`ProjectFilesService` shows what lies in these folders, for the view of
the kind `files`. It only reads, through `FolderReader`.

- The folder must be the work folder or a reference folder of the project,
  as `places` names it (`rule_broken` otherwise). The path inside it is a
  `PathInFolder` (`rule_broken` if it goes up or starts elsewhere).
- What the path names, with every link followed, must lie inside the full
  form of the folder (`rule_broken`). A link in a listing that leads
  outside, or nowhere, is listed as a link and not followed.
- Nothing at the path, or a file where a folder is listed, is `not_found`.
- `.git`, `node_modules` and `target` are never listed; other names that
  begin with a dot only when asked. At most 2000 entries are answered, and
  `has_more` says when there were more.

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
- Every turn of a specialist may ask the manager to decide (`team_tools:
  [TeamTool::AskManager]`). The
  question it asks is recorded on the turn, and a turn that finishes with a
  question leaves its task where it is: it is not ready to be checked.
- **Handoffs and messages**
  ([decision 0029](../../docs/decisions/0029-handoffs-and-messages-between-specialists.md)).
  A turn about a task is also given `TeamTool::PublishHandoff` with the work
  folder, and a turn of a specialist whose team has other AI specialists
  `TeamTool::SendMessage` with their names, in the order they joined. A
  handoff that arrives while its turn runs is stored (`TeamworkRepository`)
  for the task of the conversation and its specialist; a message is stored
  for the teammate of that name, whatever the capitals, and one to
  somebody who is not an AI specialist of the team becomes a notice of the
  turn instead. Neither ends the turn, and a turn that hands over still
  moves its task on to be checked.
- **The brief** of a task (`brief_for`) adds, for each task it waits for
  that has a handoff, the latest one: its summary, its interface contract
  and its files.
- **A message is delivered** with the next turn of its receiver, whatever
  starts it: the messages that wait (`messages_waiting_for`) are put after
  the request in what the program is told (`messages_in_words`), the
  stored request stays as it was written, and each is marked delivered with
  the turn once the turn is stored. A turn the program could not start
  delivers nothing. An arriving message does not start a turn by itself:
  that is for the coordinator, which asks `TeamworkService::messages_waiting_for`.
- The instructions of a specialist (`instructions_for`) say to hand the
  result over with `publish_handoff` before the report, and to write to a
  teammate with `send_message` without waiting for an answer.
- **Project memory**
  ([decision 0036](../../docs/decisions/0036-project-memory.md)). Every
  turn of a specialist is also given `TeamTool::Remember` and
  `TeamTool::SearchMemory` with the current entries of the memory of its
  project, newest first, each with where it came from and its task in
  words (`remembered_entries`). Memory grows by itself in three places:
  `answer_decision` stores the question and its answer as a decision
  (`decision_entry`) once the turn that carries the answer is stored; a
  handoff with an interface contract stores the contract of its task and
  marks the earlier contracts of the task as replaced by it
  (`contract_entries`); a fact a specialist remembers in a running turn is
  stored with the task of the conversation, at most three in a turn, and
  a fourth becomes a notice of the turn. `start_work` writes the brief
  with `brief_with_memory`, which adds what memory says about the task
  (`memory_in_words`): the entries of the task, then those of the tasks it
  waits for (without the contracts their handoffs left, which the brief
  carries already), then every other contract, then the entries of the
  whole project, newest first in each group, at most eight and 3,000
  characters, and in any case a sentence that says to search memory
  before asking the manager. The instructions say the same, and how to
  remember.
- A question is open while it is the question of the latest turn of its
  conversation, that turn has ended, and the manager has not answered.
  `answer_decision` answers it with an option (counted from 0), with words,
  or with both, and starts a turn of origin `Decision` whose request is
  `Decision: <option>`, followed by the words if there are any, or only the
  words (`no_decision_open`, `no_specialist`, `rule_broken`,
  `member_busy`). The answer is stored only if the new turn starts.
  `reply` is refused while a question is open (`decision_open`), after the
  checks it had before.
- `waiting_for_manager` lists the conversations with an open question, an
  open interview or an open proposal in every project that is not archived,
  the one that has waited longest first. Those whose home is another
  computer are listed too, to be read: answering them is refused here.
- `stop` records the running turn as stopped by the manager and then stops
  the program. Stopping a conversation in which nothing runs changes nothing.
- `recover` ends, as failed, the turns that storage still calls running when
  the program starts: closing Teamwright cut them off.
- **A conversation has a home computer** (`WorkPorts::this_computer`): the
  one it was started on, where its session of the agent program lives.
  `reply`, `answer_decision`, `answer_interview`, `approve_proposal`, `stop`
  and `open_in_desktop_app` refuse a conversation whose home is another
  computer (`runs_elsewhere`); approving there would create the same tasks
  twice, and a reply could not resume a session that is not there.
  `recover` and `close` end only the turns of conversations at home here,
  and never a turn another computer runs. The guards of work in progress
  count the running turns of every computer, so nothing is removed from
  under a specialist that works elsewhere.

## Planning with the Advisor

`WorkService` also carries out
[decision 0014](../../docs/decisions/0014-the-advisor-plans-through-two-tools.md),
in `work/advisor.rs`. The Advisor is not a member of the team: a
conversation with it has no member and no task, and keeps the model and
the effort it runs on, chosen when it starts. No task moves when its turns
start or end.

- `advise(project, brief, model, effort)` opens a conversation with the
  Advisor whose first turn is the brief (origin `message`). An empty brief
  or a model that is not one word is refused (`rule_broken`); an empty model
  means none. One conversation with the Advisor runs in a project at a time
  (`advisor_busy`, "the Advisor is still working on this project").
- A turn of the Advisor runs in the work folder of the project with the
  reference folders that exist as readable folders, with
  `PermissionProfile::advisor()`, with the team tools `Interview` and
  `ProposePlan { team }` (the names of the members the project has), on the
  model and at the effort of the conversation, and with the instructions of
  `instructions_for_advisor`: who the Advisor is and how it plans, the
  reference folders, what the project has already when it has members or
  tasks (it then proposes only what is to be added), and the rules not to
  ask what a specialist works out itself and not to write "recommended"
  into an option.
- What the Advisor asks (`AgentEvent::InterviewAsked`) and proposes
  (`PlanProposed`) is recorded on its turn. An interview or a proposal is
  open while it belongs to the latest turn, that turn has ended, and the
  manager has not answered or approved it.
- `answer_interview` takes one answer for each question (`no_interview_open`
  where none is open; `every question needs an answer`, `question 2 has no
  such option`, `the answer to question 3 names one of the options or says
  something in words`, all `rule_broken`; `advisor_busy`) and starts a turn
  of origin `interview` whose request is `answers_in_words`. The answers are
  stored only if the new turn starts.
- `reply` in a conversation with the Advisor is a wish for a change or
  another round. It is refused while the questions are open
  (`interview_open`, "the Advisor waits for your answers: answer its
  questions first") and while the Advisor works (`advisor_busy`). The
  session of the Advisor does not see an approval happen. So a reply that
  follows one reaches the Advisor as `message_after_approval`: the words of
  the manager with the news that the proposal was approved, what was
  approved as it was approved (members, and tasks with owners, sizes and
  what they wait for, so that a task the manager renamed or took out is
  not proposed again), and that a proposal from then on holds only what is
  to be added. The turn keeps the words of the manager alone.
- `change_proposal` makes one change of the manager's (`ProposalEdit`) to
  the open proposal, checked by the domain (`ProposalChange`) and against
  the team: an owner must be on the team or in the proposal, and a
  proposed member whose name the team has cannot be taken out ("Firmware
  engineer is on the team already: approving does not add them again").
  The turn is saved, so the change is logged with the whole proposal,
  kept over a restart and taken in by the other computers. Refusals:
  `no_proposal_open`, `runs_elsewhere`, `rule_broken` with the reason in
  words. A refused change changes nothing.
- `approve_proposal` takes what the manager left of the proposal
  (`ApprovedProposal`) and checks all of it first (`no_proposal_open`; the
  rules of names, titles and plans; a task given to somebody who is neither
  on the team nor in the proposal, `rule_broken`). Then the members join the
  team as AI specialists with the colors newcomers get, except a member
  whose name the team has, whatever the capitals; tasks go into the first
  status of the workflow, after the tasks there, numbered on, each waiting
  for the tasks the proposal says. The project, the members, the tasks and
  the turn with the approved proposal go to `PlanningRepository` in one
  write, or nothing does. The proposal keeps what was approved, with the
  moment.
- While the Advisor works, a project can be neither archived nor deleted
  (`work_in_progress`, "The Advisor is still working on a plan; stop the
  work first").

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

A turn request names the tools of the team bus the session is given
(`team_tools`, each a `TeamTool`): `AskManager` to ask the manager to
decide, `PublishHandoff { folder }` to hand the result of a task over and
`SendMessage { teammates }` to write to a teammate, `Remember` and
`SearchMemory { entries }` for the memory of the project, and `Interview` and
`ProposePlan { team }` for the Advisor, where `team` holds the names of the
members the project has. What a session does with them arrives as
`AgentEvent::ManagerAsked` (`ManagerQuestion`), `HandoffPublished`
(`PublishedHandoff`), `MessageSent` (`SentMessage`), `FactRemembered`
(`RememberedFact`), `InterviewAsked`
(`ManagerInterview`) or `PlanProposed` (`PlanProposal`), with every place
counted from 0, and never as a tool use. A request may
also name folders the session may read but not write (`readable_folders`).

A turn request carries its permissions as data, a `PermissionProfile`.
`PermissionProfile::specialist()` is the profile of a specialist: read,
search, and edit inside the working folder; no shell; everything else is
denied without waiting. `PermissionProfile::advisor()` reads and searches
and has no tool to write. The modes that approve risky tool use on the
manager's behalf are not offered.

Usage is counted in tokens
([decision 0008](../../docs/decisions/0008-budgets-in-tokens.md)). The dollar
figure is kept as `estimated_cost_usd`. Plan limits are reported as
`PlanUsage` when the program reports them, and are unknown otherwise. How
full the context of the session is after a turn arrives as
`TurnOutcome::context` (`SessionContext`: the tokens the last request
held, and the window of its model if the program named it), and is kept on
the turn (`Turn::record_context`, the domain's `ContextUse`).

`WorkService::usage` (the pure `usage_of` in `usage.rs`) says what a
project has spent: for every member that had a conversation, the one that
spent the most first, for the Advisor and for those who left (together),
and for the whole project, the tokens of the turns that ended by kind, how
many turns ended, and the model of the latest turn. The context of an agent
is that of its latest conversation (the one started last), as its last turn
that reported one says: nothing while that conversation has not reported
one, even if an older one did, because the latest is the session that goes
on. Groups have no context.

## What happened, and notifications

`project_events.rs` says in plain data what happened in a project
(`ProjectEvent`: the project, its name, when, and a `Happening`):
`TaskFinished` (a specialist finished its work on a task), `WaitsForManager`
(a decision, the Advisor's questions or its proposal; `WaitsFor`),
`AgentStopped` (a turn failed), `TaskCreated`, `TaskStatusChanged` (with the
status it left), `SprintStarted` and `SprintCompleted`. An `Agent` is a
specialist with its name, or the Advisor; a `TaskOfEvent` has the task's
reference, title, status (`StatusOfEvent`: name and category) and the name
of its assignee where the team that was handed in knows it. The pure
`event_of_ended_turn` finds the event of a turn that has just ended, or
nothing (a turn the manager stopped, an answer to a question the manager
asked); `event_of_started_sprint` and `event_of_completed_sprint` those of
a sprint; `event_of_created_task` and `event_of_moved_task` those of a task
the manager opened or moved (a move within its status is none). The shell
finds the events of tasks in its commands `create_task` and `move_task`;
tasks opened from an approved plan or moved by the work of a specialist
are not told as these events. `WorkService` calls
`WorkListener::happened` once for each turn that ends with an event, after
it told `conversation_changed`; a listener that does not care keeps the
empty default.

`notifications.rs` decides which events are worth a notification of the
system and words them (`notification_of`, `notification_for`): someone
waits ("Hardware engineer asks you to decide", with the question's first
words), work is ready ("SH-4 is ready", with the task's title), someone
stopped ("Firmware engineer stopped", with why). A sprint is worth none.
`NotificationService` keeps the choice of this computer through the port
`NotificationChoiceStore`: not asked, allowed or declined, and a switch for
each `NotificationKind`. Nothing is sent until allowed
([decision 0026](../../docs/decisions/0026-notifications.md)).

`appearance.rs` keeps the `Theme` of this computer through the port
`ThemeStore`, with `AppearanceService`: Lamplight until the manager
chooses another of the ten, and a name it does not know reads as
Lamplight. Like the choice about notifications it is a fact about one
computer, not logged and never synced
([decision 0028](../../docs/decisions/0028-themes.md)).

## Integrations

`integrations.rs` keeps what the manager set for the integrations of a
project on this computer
([decision 0027](../../docs/decisions/0027-integrations.md)):
`ProjectIntegrations` is the GitHub repository the manager named (or none,
to read it from the work folder), the Slack channel (`SlackChannel`: its
address and the kinds of events told in it) and up to five web addresses
(`ListeningAddress`: an address, an optional secret, the kinds of events
sent). `IntegrationSettings` changes them through the port
`IntegrationStore`, which also keeps the GitHub program the manager named.
A new channel is told what a person wants to hear of
(`events_told_in_a_channel`); a new address hears everything
(`events_sent_to_an_address`). The settings are not logged and not synced.

`integrations/github.rs` is GitHub through the manager's own `gh`, behind
the port `GitHubProgram`. The repository of a project is the one named, or
the one its work folder is a clone of (`RepositoryName::of_remote`). A
pull request or an issue is about a task when the pure `mentions` finds
the task's reference, whole and in any case, in its title or branch (for
a pull request also in its body): `SH-4` is not in `SH-41`.
`GitHubService` keeps one reading per project in memory with its age, and
reads only when asked (`refresh`), or once for a task when nothing was
read since the program started (`of_task`). `overview` counts what is
about tasks and lists the rest; `issue_for_task` words an issue (the
reference first in the title) and `open_issue` sends it, which the
interface does only after the manager confirmed. `sign_in_problem` says
what is missing: not installed (with where to get it), or not signed in
(`gh auth login`).

`integrations/messages.rs` says what the listeners outside the computer
get of an event, in pure functions: `kind_of` (the `EventKind` the manager
chooses by), `channel_message_of` (one plain line for a chat channel, the
project's name first: "Smart sensor node: Hardware engineer finished SH-4
Draw the schematic.") and `payload_of` (an `EventPayload`, whose JSON form,
version `PAYLOAD_VERSION`, is written by the integrations crate and
documented in [webhooks](../../docs/integrations/webhooks.md)).
`integrations/delivery.rs` passes events on: `targets_of` is the rule of
which event goes where (the channel if it tells that kind, every address
that hears it); `DeliveryService::deliver` sends through the port
`WebSender` and keeps in memory the last failure of the channel and the
last ten deliveries of each address (`deliveries`); `test_channel` and
`test_address` try them. It waits for answers, so the shell calls it on a
thread of its own.

## Syncing through Google Drive

`GoogleDriveService` finds the folder that Google Drive for desktop keeps
on this computer, so that the manager can sync through it in one press. It
reads the disk and the environment only through `DriveSurroundings`, so the
rule is tested on pretended computers.

- **On Windows** a drive whose root holds `My Drive`, in one of the
  languages Drive is most often set to; or, when the root carries Drive's
  own hidden folder `.shortcut-targets-by-id`, the one folder there that is
  neither hidden nor one of Drive's others (`Shared drives`, `Other
  computers`, ...). Then `My Drive` and `Google Drive` in the home folder.
- **On macOS** `~/Library/CloudStorage/GoogleDrive-<address>/My Drive` for
  each account, with the address as `account`, then `~/Google Drive`.
- **On Linux** nothing is looked for (`NotForThisSystem`).
- `look` says what was found (`DriveFinding`) and which folder of Drive
  holds the sync folder, compared in full form. `prepare` makes the folder
  `Teamwright` inside a folder that was found, and refuses any other
  (`folder_not_found`); choosing it is left to `SyncService`.

## Keeping computers in step

`SyncService` keeps the computers of one manager in step through a folder
the manager owns
([decision 0017](../../docs/decisions/0017-sync-through-a-folder.md)). It
never looks inside the state an operation carries; storage merges.

- **Two ports.** `SyncFolder` writes and reads the files in the folder: a
  card per computer (`ComputerCard`), files of operations
  (`ExchangedOperation`) that are `Complete`, `NotYet` or in a
  `NewerFormat`, and joining small files. `SyncLedger` is the log and the
  bookkeeping: this computer's operations after a number, taking in
  operations of another computer together with the number read up to, the
  other computers (`KnownComputer`, `Acceptance`), the files this computer
  wrote, the settings, the notices, the copy before the first take-in, and
  a new identity.
- **A round** (`round`) refuses without a folder (`sync_off`) or with a
  folder that cannot be reached (`sync_folder_unreachable`). It checks that
  every file in this computer's own folder is one it wrote, and otherwise
  stops (`identity_in_use`): the data folder was copied to another
  computer. It writes the operations not written yet (at most 2000 a file),
  the card, and joins small files. Then, for every accepted computer, it
  reads the new complete files in order, up to the first one that is not
  whole, and takes them in in batches of 500. A computer found for the
  first time is remembered as undecided and reported; it is not read until
  the manager accepts it. A file in a newer format stops reading that
  computer and is reported. A clock more than five minutes ahead leaves a
  notice. Two rounds never run at once.
- **Choosing the folder** (`choose_folder`) asks for the name of this
  computer (`ComputerName`, a rule of the domain), refuses a folder that is
  not there (`folder_not_found`) or lies inside Teamwright's own data
  folder (`sync_folder_in_data`). A folder other than the one written to
  before starts anew, and the first round writes the whole log.
- **A new identity** (`new_identity`) is for a computer whose data folder
  was copied. Its old identity becomes another computer, accepted, read
  from after the last operation this computer wrote into the folder; the
  operations it never wrote go on under the new identity. Syncing waits for
  Teamwright to start again with the new identity.
- `state` says where syncing stands (`SyncPhase`: `off`, `in_step`,
  `writing`, `unreachable`, `stopped`) with the other computers and what
  went wrong last.
- `WorkService::stop_turns` stops the programs of turns that a deletion
  from another computer removed while they ran here, and
  `WorkService::conversation` finds one conversation with its turns, to
  tell the interface what changed.
- `TaskService::history_with_computers` tells the history of a task with
  the computer that made each change (`HistoryEntry`), from
  `TaskRepository::computers_of_versions`. Storage that does not know the
  computers answers with none, and the entries then name none.
- `TaskService::project_history` tells what happened to the tasks of a
  project, the newest change first, a page at a time (`ProjectChange`: the
  change, and the task's id, number, title and whether it is gone). A page
  has at most `limit` changes, never more than `MOST_PROJECT_CHANGES`
  (200), older than `before`, compared to the millisecond, and never
  splits the changes of one millisecond. It reads the versions of every
  task the project has had, deleted ones too
  (`TaskRepository::ids_ever_in_project`). Only tasks: that a task was
  deleted, and what happened to sprints, views and members, is not in it.

## The project as a workbook

`WorkbookService::workbook_of` carries out
[decision 0025](../../docs/decisions/0025-the-project-as-a-workbook.md).
It reads a project and returns what its workbook holds; it writes nothing.
`workbook/tests.rs` checks every sheet on a project in memory.

- Four sheets, in this order: Summary, Tasks, Sprints, Team.
- The summary is an overview: the name of the project as its title, then
  sections of facts (the key, the description, the day of the export and
  the computer it was made on; the tasks open, finished and in all; the
  tasks of every status in the order of the workflow; the running sprint).
  A project without sprints has no sprint section; one with sprints and
  none running says so.
- Finished means a status of the category done, the rule sprints use. The
  figures the dashboard works out in the interface alone (in work, late,
  given to nobody) are left out, so that no rule is written twice.
- The days a sprint has left are its end date less today; past its end
  they are below zero. Its work is counted as a sprint counts it: its main
  tasks, in points if any has points, and otherwise in tasks. A completed
  sprint keeps the summary it was completed with.
- The tasks come in the order of an outline: the main tasks by number, each
  followed by its subtasks, set in one level deeper, which name it under
  "Part of". Dates are days where the program runs (`Clock::day_of`).
- The team is the manager ("You"), the members in the order they joined,
  and the Advisor if the project has spoken with it. Tokens are counted for
  AI specialists and the Advisor, every kind together (`TokenCounts::all`).

## The report of a whole project

`ProjectReportService::report_of` carries out
[decision 0042](../../docs/decisions/0042-the-report-of-a-whole-project.md).
It reads a project and returns its report; it writes nothing.
`project_report/tests.rs` checks every part on a project in memory, and
`project_report/markdown.rs` tests the Markdown it is written as.

- A project is complete when it has tasks and every one of them is in a
  status of the category done. There is no state "completed" to set. The
  report is called "Report of the project" then, and "Report of the
  project so far" before.
- The brief is the summary of the latest plan of the Advisor the manager
  approved. A project planned without the Advisor has none.
- The work is shared by area only when the project has more than one
  area, and by holder in the order of the team, then the manager, then
  the tasks given to nobody. A part that holds no task is left out.
- The plan against what happened says only what is stored: the
  estimates of the finished tasks, how long the turns of the specialists
  on them ran (from the start to the end of every turn that ended), and
  the due dates met and missed, where a day is the day where the program
  runs (`Clock::day_of`). An estimate is the size of the work for a
  skilled person, not a forecast of a specialist's time, and the report
  says so beside the comparison. The work of people is not timed, and
  the report says that too when people finished tasks. Nothing is
  extrapolated: the sprint charts work out the course of the work.
- A sprint keeps the summary it was completed with; any other is counted
  from its main tasks now, as its report counts it.
- What the team used is `usage_of` by who, and the turns that ended by
  the model they ran on.
- The decisions are the questions of the specialists and of the Advisor's
  interviews that the manager answered, in the order they were answered.
  A question that waits is counted under what is still open.
- The results handed over are the latest handoff of every task, in the
  order of the tasks (`latest_handoff_of`).
- The summary and the plan are sentences worked out here, so that the
  page on the screen and the Markdown file say the same.

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
