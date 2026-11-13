# teamwright-application

Teamwright's use cases: what the program can do, independent of any database,
window or agent runtime.

## What is in it

| Module | Contents |
|---|---|
| `projects` | `ProjectService`: start, find, list, rename, describe, archive, restore and delete projects; add, change, move and remove the statuses of their workflows |
| `tasks` | `TaskService`: open, change, plan into a sprint, give points, move, connect and remove tasks, and tell the history of one from the versions storage has kept |
| `sprints` | `SprintService`: list, plan, change, delete, start and complete the sprints of a project, and the report for the burndown chart of one (`SprintReport`) |
| `views` | `ViewService`: list, add, rename, filter, move and remove the views of a project |
| `areas` | `AreaService`: list, add, rename, move and remove the areas of a project, and move a task with its subtasks into another area |
| `team` | `TeamService`: put members on the team of a project, change them, take them off |
| `places` | `PlacesService`: the folders of a project: choose its work folder, add and take away reference folders, copy reference files into it |
| `work` | `WorkService`: let AI specialists work: start on a task, ask, reply, answer a specialist's question, list what waits for the manager, stop, follow every event, open a session in the desktop app, recover after a restart; `brief_for` and `instructions_for`. Its part `advisor`: plan a project with the Advisor (`advise`, `answer_interview`, `approve_proposal`); `ApprovedProposal`, `answers_in_words` and `instructions_for_advisor` |
| `sync` | `SyncService`: keep several computers of one manager in step through a folder the manager owns (`round`, `choose_folder`, `stop`, `rename_this_computer`, `accept`, `ignore`, `notices`, `dismiss`, `new_identity`); the ports `SyncFolder` and `SyncLedger`, and the operations that travel between computers |
| `ports` | The interfaces the services need: `ProjectRepository`, `TaskRepository`, `MemberRepository`, `ConversationRepository`, `PlanningRepository`, `PlacesRepository`, `SprintRepository`, `ViewRepository`, `AreaRepository`, `ProjectFolders`, `FileSystem`, `WorkListener`, `Clock` (which also tells the day a moment falls on) |
| `agent_runtime` | `AgentRuntime`: the interface to the program that runs a team member's session, and the data it speaks |
| `error` | `ApplicationError`, with a stable `code()` for interfaces |
| `clock` | `SystemClock`, the real clock |
| `in_memory` | In-memory repositories (`InMemoryPlanningRepository` stores an approved plan into the others, `InMemorySprintRepository` stores the tasks that change with a sprint into the task repository, `InMemoryViewRepository`, `InMemoryAreaRepository`), fixed project folders, a pretended file system, a listener that records, a scripted agent runtime and a settable clock, for tests and demos |

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

- `ProjectService::create` gives a new project six views, in the order
  board, backlog, list, Gantt chart, calendar, dashboard, each named after
  its kind and showing every task. `ProjectService::delete` removes them.
- A project has at most one backlog and one dashboard (`view_kind_taken`).
- A name is tidied and has at most 40 characters (`rule_broken`). Two views
  of a project do not share a name, whatever the capitals
  (`view_name_taken`); a view may change the capitals of its own. A view
  added without a name, or with an empty one, is named after its kind, with
  the first free number from 2 on if that name is taken.
- Only a board, a list, a Gantt chart and a calendar can be filtered
  (`rule_broken`), and a filter can only name a member of the team of the
  project (`member_not_on_team`).
- `move_to` puts a place past either end at that end, stores only the views
  whose place changed, and answers with all the views in their new order.
- The last view of a project cannot be removed (`last_view`).
- `TeamService::remove` makes every view that showed only the tasks of the
  member who leaves show everybody's again, and keeps it hiding finished
  tasks if it did.

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

- `ProjectService::create` gives a new project one area, "General".
  `ProjectService::delete` removes the areas after the tasks.
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
  the manager with the news that the proposal was approved, and that a
  proposal from then on holds only what is to be added. The turn keeps the
  words of the manager alone.
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
decide, `Interview` and `ProposePlan { team }` for the Advisor, where `team`
holds the names of the members the project has. What a session does with
them arrives as `AgentEvent::ManagerAsked` (`ManagerQuestion`),
`InterviewAsked` (`ManagerInterview`) or `PlanProposed` (`PlanProposal`),
with every place counted from 0, and never as a tool use. A request may
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
`PlanUsage` when the program reports them, and are unknown otherwise.

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
