# Teamwright desktop

The desktop application: an interface written in React and TypeScript
(`src`) inside a Tauri shell written in Rust (`src-tauri`).

## The shell: `src-tauri`

The shell opens the window, builds the services, and exposes one command per
use case. It contains no rules of its own.

| File | Contents |
|---|---|
| `src/main.rs` | Starts the application, lists the commands, and stops the specialists when it closes. Started with `--team-bus`, it serves the team bus instead and opens no window |
| `src/state.rs` | Opens the database, builds the services, and decides what stands behind each of their interfaces |
| `src/commands/` | One function per command: read the arguments, call one use case, answer |
| `src/dto.rs` | The data that travels between the interface and the commands |
| `src/error.rs` | The error a command reports: a stable code and a readable message |
| `build.rs` | The build script; see [building without administrator rights](../../docs/development/windows-without-admin.md) |

To add a command: add the use case to the application crate, add a function
in `src/commands`, list it in `main.rs`, and add the matching method to the
`Backend` interface on the TypeScript side.

**Sprints and the backlog**
([decision 0016](../../docs/decisions/0016-sprints-and-a-backlog.md)) are
the commands in `commands/sprints.rs`: `list_sprints`, `create_sprint`,
`change_sprint`, `delete_sprint`, `start_sprint`, `complete_sprint`,
`plan_task`, `set_task_points` and `sprint_report`. A task on the wire has
its `sprintId` and its `points`. `set_task_points` reads the points as any
number, so that a fraction reaches the use case and is refused there with
the rule it breaks.

**The views of a project**
([decision 0021](../../docs/decisions/0021-views-the-manager-can-add.md))
are the commands in `commands/views.rs`: `list_views`, `add_view`,
`rename_view`, `set_view_filter`, `set_view_content`, `move_view` and
`remove_view`, which the shell serves through `ViewService`. A view on the
wire has its `kind`, its `name`, its `content` (text the interface wrote,
or null;
[decision 0024](../../docs/decisions/0024-a-view-keeps-content-of-its-own.md))
and its `filter`, whose `assignee` is `{"kind": "anyone"}`,
`{"kind": "manager"}`, `{"kind": "member", "memberId": "…"}` or
`{"kind": "nobody"}`. `move_view` reads the place as any number, cut to a
whole number as the interface's arrays would cut it. A database with
projects from before views gives each of them the six views a new project
starts with when it is opened.

**The areas of a project**
([decision 0022](../../docs/decisions/0022-areas-inside-a-project.md))
are the commands in `commands/areas.rs`: `list_areas`, `add_area`,
`rename_area`, `move_area`, `remove_area` and `move_task_to_area`, which
the shell serves through `AreaService`. An area on the wire has its `id`,
its `projectId` and its `name`; a task carries its `areaId`, and a new
task may name one. `move_area` reads the place as any number, cut to a
whole number as the interface's arrays would cut it; `remove_area` takes
`moveTasksTo`, or null. `demoAreas.test.ts` is the specification, and the
application crate mirrors its cases with the same codes and messages. A
database with projects from before areas gives each of them the area
"General", with all its tasks in it, when it is opened.

**Tags and checklists**
([decision 0030](../../docs/decisions/0030-tags-and-checklists.md)) are the
commands in `commands/tags.rs` (`list_tags`, `add_tag`, `rename_tag`,
`recolor_tag`, `remove_tag`, `put_tag_on_task`, `put_named_tag_on_task`,
`take_tag_off_task`), served through `TagService`, and in
`commands/checklists.rs` (`add_checklist_item`, `rename_checklist_item`,
`tick_checklist_item`, `move_checklist_item`, `remove_checklist_item`),
served through `ChecklistService`; each command of a checklist answers
with the task. A tag on the wire has its `id`, `projectId`, `name` and
`color`; `add_tag` takes a color or null for the next in turn, and
`put_named_tag_on_task` answers with `{task, tag}`. A task carries its
`tagIds` and its `checklist` (`id`, `text`, `done`, in order). The filter
of a view gains `tagId`, which may be left out or null for the tasks with
any tag. A change of the history may be `tagged` or `untagged` (with
`tagId`) or `ticked` or `unticked` (with the `item`). `demoTags.test.ts`
is the specification.

**The project as an Excel workbook**
([decision 0025](../../docs/decisions/0025-the-project-as-a-workbook.md))
is two commands in `commands/workbook.rs`. `export_workbook` takes the
project and the path the manager chose in the save window of the dialog
plugin (`dialog:allow-save` in `capabilities/default.json`), gathers the
workbook with `WorkbookService`, has `teamwright-workbook` write it, and
answers with the path. The summary names the computer by the name it has
for syncing, or else the one the operating system gives it. A file that
could not be written is refused with the code `file_not_written` and the
reason in words. `show_exported_workbook` shows the file chosen in its
folder (Explorer, Finder; on Linux the folder is opened), but only a path
`export_workbook` wrote since the program started: the shell keeps them in
`AppState`, so the interface cannot make it open any other place. In the
demo the save window suggests a path and writing is `not_available`.

### The work of the specialists

The shell is where the work of an AI specialist meets the outside world
([decision 0010](../../docs/decisions/0010-one-process-for-each-turn.md),
[decision 0011](../../docs/decisions/0011-what-a-specialist-may-do.md)):

- **The program.** `state.rs` hands the application crate the user's own
  Claude Code (`teamwright-runtime`) as its agent runtime. The shell never
  reads how the user is signed in; it only asks the program whether they
  are. `TEAMWRIGHT_AGENT_PROGRAM` names another program to use, which is how
  the tools for tests and pictures put a stand-in in its place.
- **The folders of a project.** A specialist works in the work folder of
  its project ([decision 0013](../../docs/decisions/0013-the-folders-of-a-project.md)):
  the one the manager chose, or else `Teamwright/<key of the project>` in the
  user's documents, which is created when it is first needed. When
  `TEAMWRIGHT_DATA_DIR` names a data folder, the folders Teamwright gives the
  projects are in `projects` inside it instead, so a test never writes into
  the user's documents. The reference folders of the project are given to
  each turn to read. `state.rs` reaches the disk for the application through
  `LocalFileSystem`, on the standard library. The commands in
  `commands/places.rs` choose the folders and copy reference files;
  `show_work_folder` opens the work folder in the file manager of the
  operating system (`explorer`, `open` or `xdg-open`) without waiting for
  it, and reports a chosen folder that is gone as `folder_not_found`.
  `commands/files.rs` serves the files view: `list_project_files` reads a
  folder of the project through `LocalFolderReader`, which only reads;
  `open_project_file` opens a file with the program the system has for it,
  and `show_project_file` shows it in the file manager (the same way the
  exported workbook is shown). Each first has the use case check the path:
  it stays inside the folder, links followed, and only its full form is
  handed over, as one argument and without a shell. A folder is not opened
  with another program, and a file that would run as a program (`.exe`,
  `.bat`, `.ps1`, `.lnk` and the like) is not opened at all
  (`invalid_request`): it can be shown in its folder.
- **The team bus.** A specialist asks the manager to decide through a tool,
  `ask_manager`
  ([decision 0012](../../docs/decisions/0012-a-specialist-asks-through-a-tool.md)).
  The Advisor interviews and proposes through two more, `interview` and
  `propose_plan`
  ([decision 0014](../../docs/decisions/0014-the-advisor-plans-through-two-tools.md)).
  The server of these tools is this very program: `state.rs` tells the
  runtime to start it with the argument `--team-bus`, the runtime adds the
  names of the tools the turn is given (and `--member <name>` for each
  member of the team), and `main.rs` then serves those tools on standard
  input and output without a window and without the database. What is asked
  or proposed arrives in the events of the turn, like every other step.
- **Handoffs and messages.** A specialist hands the result of its task over
  with `publish_handoff` and writes to a teammate with `send_message`
  ([decision 0029](../../docs/decisions/0029-handoffs-and-messages-between-specialists.md)).
  The runtime gives the bus the work folder (`--folder <path>`) and the
  names of the teammates (`--teammate <name>`). `team_talk` in
  `commands/teamwork.rs` answers with every handoff and message of a
  project (`TeamTalkDto`: `HandoffDto`, `TeamMessageDto`). The interface
  (`features/teamwork`) asks again whenever a conversation of the project
  changes, and shows the latest handoff of a task in its panel (summary,
  interface contract, files), what a turn handed over and sent, and the
  messages that came with it, as steps of the turn, and the messages
  between teammates, the latest first, in the Team place under the team.
  The demo backend keeps them in `shared/api/demo/teamwork.ts`, starts with
  sample handoffs and messages, delivers a waiting message with the
  receiver's next turn, and its simulated specialist hands its result over
  before it reports.
- **Project memory** ([decision 0036](../../docs/decisions/0036-project-memory.md)).
  `commands/memory.rs` gives every entry of a project (`project_memory`,
  `MemoryEntryDto`), a search by the rule the specialists search by
  (`search_memory`), and adding a fact, changing an entry (a newer entry
  replaces it) and removing one. The fixed place **Memory**, between Office
  and Advisor (`features/memory`), lists the current entries the newest
  first, by kind, with where each came from and its task; it searches, adds,
  changes, removes, and shows the replaced entries on request. A turn shows
  the facts it remembered among its steps, and a search of memory reads as
  "Memory · searched for …". The memory is asked for again whenever a
  conversation of the project changes. The demo backend keeps it in
  `shared/api/demo/memory.ts`, starts with sample entries, turns an answered
  decision and the contract of a handoff into entries, puts memory into the
  brief, and its simulated specialist searches memory and, on every other
  task, remembers a fact.
- **Earlier conversations.** `commands/digests.rs` finds the conversations
  held with Claude Code on this computer (`find_earlier_conversations`),
  makes the digest of one (`draft_digest`), and keeps, lists and removes
  digests (`keep_digest`, `list_digests`, `remove_digest`); the wire types
  are in `dto/digests.rs`
  ([decision 0041](../../docs/decisions/0041-earlier-conversations-as-digests.md)).
  The Advisor place shows the kept digests under the form that starts
  planning, and a dialog lists, filters and imports conversations
  (`features/advisor/EarlierConversations.tsx`). The demo pretends to find
  seven made-up conversations (`shared/api/demo/earlierConversations.ts`);
  the empty demo finds none.
- **What the agents spent.** `project_usage` answers with what every member,
  the Advisor and those who left have spent (`ProjectUsageDto`: the tokens
  by kind and in all, the turns that ended, the model of the latest turn,
  and the context of the agent's latest conversation), worked out by
  `teamwright_application::usage_of`. A turn and a conversation carry their
  `context` (`ContextDto`: `held`, `window` or null, `isNearlyFull`). The
  interface asks for the usage again when a turn starts or ends; the demo
  backend stands in with `shared/api/demo/usage.ts`, which follows the
  same rules.
- **Integrations**
  ([decision 0027](../../docs/decisions/0027-integrations.md)). The entry
  "Integrations…" of the project's menu opens `IntegrationsDialog`
  (`features/projects`), with a part for GitHub, Slack and web addresses,
  each saying in one line whether it is on. The commands in
  `commands/integrations.rs` (`project_integrations`, `set_repository`,
  `set_slack_address`, `set_slack_event`, `add_web_address`,
  `remove_web_address`, `set_web_address_event`) answer with
  `ProjectIntegrationsDto`, in which an address is only its host: the
  interface never gets a secret back. The demo backend keeps the settings
  in memory (`shared/api/demo/integrations.ts`) and sends nothing.
  GitHub: the commands in `commands/github.rs` (`github_sign_in`,
  `set_github_program`, `github_overview`, `refresh_github`,
  `task_on_github`, `issue_draft`, `open_issue`) call `GitHubService`,
  which runs the manager's `gh` through `teamwright-integrations`. The
  part "GitHub" of a task (`features/github/TaskGitHub`) lists its pull
  requests and issues (`GitHubItems`: a status mark and a word for the
  state, the number, the title; a line opens the item in the browser
  through `open_web_address`), reads again on "Refresh", and opens an
  issue only after a dialog has shown what will be sent and where. The
  demo pretends `gh` is signed in and that a few pull requests and issues
  name the first tasks (`shared/api/demo/gitHub.ts`).
  Slack and web addresses: `events::tell` hands every event to
  `DeliveryService::deliver` on a thread of its own, beside the
  notifications. `integration_deliveries`, `test_channel` and
  `test_web_address` answer with `DeliveriesDto` and `DeliveryDto`; the
  dialog asks for the deliveries every five seconds while it is open,
  shows "The last message could not be sent: …" under the channel and the
  last ten deliveries under each address. The demo pretends that every
  test reached its address.
- **What happened, and notifications**
  ([decision 0026](../../docs/decisions/0026-notifications.md)). The work
  of the specialists hands every event of a project
  (`teamwright_application::ProjectEvent`) to `EventToInterface::happened`
  in `state.rs`; the commands `start_sprint` and `complete_sprint` hand
  theirs over after they succeeded. Both go to `events::tell` in
  `src-tauri/src/events.rs`, the one place where events leave the window:
  today it sends a notification of the system through
  `tauri-plugin-notification`, if `NotificationService` says the event is
  worth one with the choice of this computer and the window is not in
  front. Pressing one brings the window to the front and sends the event
  `notification-pressed` (`NotificationPressedDto`), which
  `features/notifications` turns into a place to open. The commands in
  `commands/notifications.rs` (`notification_choice`,
  `allow_notifications`, `decline_notifications`, `set_notification_kind`,
  `send_test_notification`) answer with the choice
  (`NotificationChoiceDto`). The interface asks once, in the inbox
  (`AskToNotify`), and keeps the switches on the page Computers
  (`NotificationSettings`). The demo backend keeps the choice in memory and
  refuses the test notification with `not_available`, as it does other
  things only the desktop has.
- **The theme of this computer.** The commands `theme` and `choose_theme`
  in `commands/appearance.rs` read and keep the name of the theme of the
  window (`AppearanceService`, decision 0028); `choose_theme` refuses a
  name it does not know with `invalid_request`. The name is a plain string
  on the wire, `ThemeName` in `types.ts`. The demo backend keeps it in
  memory, starting from the theme the browser remembered.
- **Opening a web address.** `open_web_address` in `commands/web.rs` opens
  an address in the manager's own browser. The pure `checked_web_address`
  refuses anything that is not `https` with a host (and an address with a
  name and a password); what is handed on is the address as the parser
  wrote it again, as one argument to the system's own opener (`explorer`,
  `open`, `xdg-open`), without a shell. `Backend.openWebAddress` calls it;
  the demo does what a link does (`window.open` with `noopener`). The Embed
  view's "Open in the browser" uses it.
- **Planning with the Advisor.** The commands `advise`, `answer_interview`,
  `change_proposal` and `approve_proposal` in `commands/work.rs` carry it
  out; `reply` and
  `list_conversations_waiting` serve conversations with the Advisor too. In
  `dto.rs`, a conversation says whether it `isWithAdvisor`, a turn carries
  its `interview` and its `proposal`, a conversation names the computer it
  runs on (`homeComputer`) and says whether that is this one (`isHere`),
  and an owner of a proposed task is
  `{"kind": "manager"}` or `{"kind": "member", "name": "…"}`. A place sent
  as a number that is not a whole number from 0, such as `-1` or `1.5`, is
  read as a place that is not there, so that the use case refuses it with
  the rule it breaks. A change of a proposal (`ProposalChange` in
  `types.ts`, `ProposalEditInput` in `dto.rs`) is tagged by its `kind`
  (`retitle`, `describe`, `estimate`, `prioritize`, `giveTo`, `addWait`,
  `removeWait`, `addTask`, `removeTask`, `removeMember`, `setProfile`) and
  names tasks and members by their places.
- **Instructions and ready-made specialists**
  ([decision 0031](../../docs/decisions/0031-instructions-and-ready-made-specialists.md)).
  A member on the wire carries its `instructions`. The commands
  `set_member_instructions`, `list_ready_made_specialists` and
  `add_ready_made_member` in `commands/team.rs` answer with a `MemberDto` or
  a list of `ReadyMadeSpecialistDto`. `src/shared/api/readyMade.json` is the
  interface's copy of the set and of the limit of length of instructions:
  the demo offers the set from it and the Team place counts against the
  limit. A test in `commands/team.rs` fails when the copy differs from the
  domain; run the tests of this crate with `TEAMWRIGHT_WRITE_READY_MADE=1`
  to write it anew. In the Team place, `ReadyMadeDialog` is the chooser and
  `InstructionsField` the growing field with its count in the dialog that
  changes a specialist.
- **Limits and templates**
  ([decision 0033](../../docs/decisions/0033-work-in-progress-limits-and-templates.md)).
  A status on the wire carries its `limit`, or `null`. The commands
  `set_status_limit`, `list_status_loads` and `list_project_templates` in
  `commands/projects.rs` answer with a `ProjectDto`, a list of
  `StatusLoadDto` (`count`, `limit` and `fill`: `under`, `at`, `over` or
  `null`), and a list of `ProjectTemplateDto`. `NewProjectInput.workflow`
  names a template: `standard`, `kanban`, `scrum`, `hardware` or
  `engineering`. `src/shared/api/projectTemplates.json` is the interface's
  copy of the templates and of the range of a limit: the demo makes
  projects from it. A test in `commands/projects.rs` fails when the copy
  differs from the domain; run the tests of this crate with
  `TEAMWRIGHT_WRITE_TEMPLATES=1` to write it anew. The formatter passes
  over it, so that it keeps the layout of `serde_json`.
- **Slow commands.** Asking the program whether it is there, and starting a
  turn (also `advise` and `answer_interview`), take seconds. Those commands are declared `async`, so they run on a
  thread of their own and the window stays alive.
- **One event.** Every change to a conversation is sent to the interface as
  the event `conversation-changed`, with the whole conversation. The
  interface replaces what it shows; it never has to put steps together.
- **Closing.** When the program closes, the specialists that are still
  working are stopped and their turns end as failed, with a text that says
  why. A turn that was cut off harder than that, for example by a crash, is
  ended the same way at the next start. A turn of a conversation that runs
  on another computer is never ended here.

### Keeping computers in step

The shell connects `SyncService` to the disk
([decision 0017](../../docs/decisions/0017-sync-through-a-folder.md)):
`state.rs` builds it with `SqliteSyncLedger` and `FolderOfComputers`, the
version of the program, and the data folder, inside which the sync folder
may not lie.

- **The background round.** `start_syncing_in_the_background` starts one
  thread when the program starts. Every five seconds it looks: this
  computer's changes are written once the latest is five seconds old, or at
  the latest thirty seconds after the last round, and the other computers
  are read every twenty seconds. Nothing happens while no folder is chosen.
  A round started by a command and one started by the thread never run at
  once: the service runs one at a time.
- **After a round** the turns that another computer deleted while they ran
  here are stopped, and the interface is told: `data-changed` with
  `{ "projectIds": [...] }` when something changed in projects,
  `conversation-changed` with the whole conversation for each conversation
  that changed, and `sync-changed` with the whole `SyncStateDto`, after
  every round, also one that failed.
- **The copy before the first take-in** is made by the ledger, as
  `teamwright-before-sync-<date>.db` in the data folder.
- **The name of this computer** is asked for when the folder is chosen.
  `SyncStateDto.suggestedName` offers the name the operating system gives
  the computer (`COMPUTERNAME` or `HOSTNAME`), only as a suggestion.
- **A new identity** takes effect when Teamwright starts again; until then
  the state is `stopped` and every round is refused with
  `identity_in_use`.

The commands are in `commands/sync.rs`:

| Command | Arguments | Answer |
|---|---|---|
| `sync_state` | | `SyncStateDto` |
| `choose_sync_folder` | `folder`, `computerName` | `SyncStateDto`, after a first round |
| `stop_syncing` | | `SyncStateDto` |
| `sync_now` | | `SyncStateDto`, after the round |
| `rename_this_computer` | `name` | `SyncStateDto` |
| `accept_computer` | `computerId` | `SyncStateDto`, after a round |
| `ignore_computer` | `computerId` | `SyncStateDto` |
| `sync_notices` | | `SyncNoticeDto[]`, newest first |
| `dismiss_sync_notice` | `noticeId` | nothing |
| `give_this_computer_a_new_identity` | | `SyncStateDto` |
| `google_drive` | | `GoogleDriveDto` |
| `prepare_google_drive_folder` | `folder` | the path of the folder `Teamwright` inside it |

**Through Google Drive.** `commands/google_drive.rs` offers the folder that
Google Drive for desktop keeps on this computer, so that the manager can
sync through it in one press. `state.rs` builds `GoogleDriveService` with
`LocalDriveSurroundings`, which reads the drive letters (`C:` to `Z:`), the
home folder (`USERPROFILE`, or `HOME`) and the disk. `GoogleDriveDto` has
`finding` (`found`, `not_installed`, `not_for_this_system`), `folders` and
`syncsThrough`, each a folder with `path` and `account` (the address, when
the path names it). `prepare_google_drive_folder` makes `Teamwright` inside
a folder that was found and answers with it; the folder is then chosen with
`choose_sync_folder`, as any other. There is no sign-in to Google: Google's
own program does the syncing.

`SyncStateDto` has `folder`, `thisComputer`, `computerName`,
`suggestedName`, `phase` (`off`, `in_step`, `writing`, `unreachable`,
`stopped`), `lastRound`, `problem` (in words, for the manager),
`problemCode` (to decide by: `sync_folder_unreachable`, `identity_in_use`,
`restart_needed`, `storage_failed`, `sync_off`, or null) and `computers`,
each a `ComputerDto`
with `id`, `name`, `version`, `acceptance` (`undecided`, `accepted`,
`ignored`), `lastHeard`, `clockAhead` and `newerFormat`. A
`SyncNoticeDto` has `id`, `at`, `kind` (`task_renumbered`,
`project_rekeyed`, `change_dropped`, `task_moved`, `clock_ahead`),
`projectId`, `taskId` and `message`. An entry of the history of a task
(`list_task_history`) has `computer`, the name of the computer that made
the change, when it was another computer whose name is known; it is left
out otherwise. The history of a project (`list_project_history`, with
`projectId`, `limit` and an optional `before` moment) answers with such
entries, newest first and without `computer`, each with `task`: its
`id`, `number`, `title` and `gone`. The interface reads it with
`useProjectHistory` (`features/tasks/taskQueries.ts`), a page of 50 at a
time. The error codes are those of the
application: `sync_off`, `sync_folder_unreachable`, `sync_folder_in_data`,
`identity_in_use`, `folder_not_found`, `rule_broken` (also for an
identifier that cannot be read, or an empty name), `not_found` and
`storage_failed`. `types.ts` mirrors these types (`SyncState`, `Computer`,
`SyncNotice`, `DataChanged`), and `Backend` has one method for each command
and two listeners, `onSyncChanged` and `onDataChanged`.

The demo backend pretends syncing (`shared/api/demo/sync.ts`, tested in
`demoSync.test.ts`): it is off at first; once a folder and a name are
chosen it is in step, and a second computer, "Laptop", shows up undecided
a moment later. Taking it in brings a task, a notice of a renumbered task,
and a conversation that runs on Laptop. Its refusals are those of the shell.
`pretendSync` and `pretendComputer` let a scenario show a round under way,
a folder that cannot be reached, a copied identity, or another computer;
in the browser the demo is `window.teamwrightDemo`.

## The interface: `src`

```
src/
  main.tsx            Starts the interface and provides what every part needs
  app/                The window as a whole, and where the user is in it
  features/
    projects/         Sidebar, project header, its menu, creating a project, first run, export to Excel
    board/            The board: lanes, cards, drag and drop
    gantt/            The Gantt chart: bars against days, lines of dependencies
    calendar/         The calendar: a month of weeks with the tasks on their days
    timeline/         The timeline: a row for every person, with their tasks along the days
    table/            The table: one task a row, every field a column changed in place, sums at its foot
    workload/         The workload: a row for every person, a column for every week, the hours that fall into each
    people/           The People view (kind `team`): a column for every person, what they do now, their tasks
    mindmap/          The Mind map view: the project, its areas, tasks and subtasks as a tree that folds
    activity/         The Activity view: the log of the project, newest first, by day
    doc/              The Doc view: a page of Markdown the view keeps, read like a page of a book
    chat/             The Chat view: the talk with the specialists and the Advisor that is not about a task
    report/           The Report view: one page about the project, set like a printed sheet, to print or save as PDF
    whiteboard/       The Whiteboard view: notes, texts, boxes and ellipses joined by lines and arrows, kept in the view
    embed/            The Embed view: one https address shown in a sandboxed frame, under a thin bar
    form/             The Form view: a form set like a printed sheet, which makes a task when it is sent
    gallery/          The Gallery view: the tasks as cards with a picture the view keeps, in three sizes
    files/            The Files view: what lies in the folders of the project, a quiet table to walk, open and show
    tasks/            The task panel, the list, badges, the fold of subtasks, task logic
    team/             The team: avatars, the Team view, adding and changing a member
    office/           The office: the team at work, drawn as a workshop in plan
      characters/     The people of the office: pixel figures drawn in code, and portraits
    dashboard/        How a project is going: a summary in words, numbers, small charts
    work/             The work of AI specialists: starting it, following it, answering
    teamwork/         What specialists hand each other: the handoff of a task, steps of a turn, messages between teammates
    inbox/            What waits for the manager in every project: questions to decide
    advisor/          Planning a project with the Advisor: its questions, its proposal
    areas/            The areas of a project: at the side, the open area, the dialog, the field of a task
    tags/             The tags of a project: the chips, the line of tags of a task and its picker, the dialog
    checklists/       The checklist of a task in its panel, and its progress on cards and rows
    sprints/          The Backlog view, sprints and points, the burndown with the scope that joined or left, the pace of the team, the board of a sprint, completed sprints opened
    sync/             Computers: syncing through a folder or Google Drive, the line at the foot of the sidebar, conversations elsewhere
    views/            The row of views and places, the chooser, the menu and filter of a view
  shared/
    api/              The Backend interface and its two implementations
    ui/               Buttons, menus, dialogs, the calendar field, tooltips, a ground that is moved and zoomed, the marks of other services
    logos/            The official marks of GitHub and Google Drive, unaltered (their README says the rules)
    lib/              Formatting and ranking helpers
    styles/           Design tokens and document-wide styles
```

Rules of the interface:

- **No business rules.** The interface asks the backend and shows the answer.
  Where it shows a change before the backend has answered (moving a card), it
  repeats the backend's arithmetic and corrects itself if the answer differs.
- **Limits are counted by the backend.** The head of a lane with a limit
  shows the count of the whole project against it, from
  `useStatusLoads`, which asks again whenever the tasks or the limits
  change; `board/boardModel.ts` only turns that answer into a figure, a
  gauge and words. A lane over its limit takes the red of trouble and the
  word "over", and is never lit. The limit is set in a column of the
  dialog of the statuses. The dialog of a new project draws its chooser
  (`projects/TemplateChoice.tsx`) from `useProjectTemplates`.
- **News and failures come in the corner.** `useNotifier()` shows a short
  message at the bottom right that goes by itself: `showError` with a rule
  in the colour of danger, `showNotice` with a plain rule and at most one
  thing to do about it. Exporting a project to Excel
  (`projects/workbookExport.ts`, from the menu of the project) says where
  the file went and offers "Show in folder".
- **One way to the backend.** Components use `useBackend()` or the hooks
  built on it (`useProjects`, `useTasks`, `useTaskCommands`, `useTeam`,
  `useTeamCommands`, `useConversations`, `useWorkCommands`, `useSprints`,
  `useProjectHistory`, `useSprintCommands`, `useViews`, `useViewCommands`,
  `useSetViewContent`,
  `useAreas`,
  `useAreaCommands`, `useTags`, `useTagCommands`, `useChecklistCommands`). Nothing calls
  Tauri directly except `shared/api/tauriBackend.ts`.
- **The team of the open project is known everywhere.** `TeamProvider` puts
  it in a context, so a card, a row or a menu can show who a task is given
  to without the team being handed down to it. `WorkProvider` does the same
  for the conversations with the specialists, so a card can show that
  somebody is working on its task, or waits for a decision about it.
- **What waits for the manager is asked for across projects.** The inbox
  and the count next to it in the sidebar come from one question to the
  backend (`useConversationsWaiting`), which is asked again when a turn
  ends or starts. The open project does not matter to it.
- **A conversation is replaced, never patched.** While a specialist works,
  the backend sends the whole conversation with every step
  (`useConversations`). The interface puts it in the place of the one it
  had, and loads the tasks again when a turn starts or ends, because that
  moves a task.
- **What another computer did is asked for again.** Syncing
  ([decision 0017](../../docs/decisions/0017-sync-through-a-folder.md))
  sends `data-changed` with the projects a round changed;
  `useFreshDataFromOtherComputers`, mounted once in `App`, asks again for
  every query of those projects and for those that are not of one project
  (`isStaleAfterChanges` in `sync/syncModel.ts`). `sync-changed` replaces
  the state of syncing (`useSyncState`), which the line at the foot of the
  sidebar and Computers show. The words of that line, and of "heard 2 min
  ago", are worked out in `syncModel.ts`. Whether another computer uses this
  one's identity is read from the words of the state's `problem`, because
  the state carries no code for it.
- **Google Drive in one press.** On Computers, before a folder is chosen,
  "Sync with Google Drive" stands beside "Choose a folder" when
  `useGoogleDrive` found a folder of Drive; with several accounts the
  manager picks one. It asks the backend to make `Teamwright` inside it and
  goes on to the name of this computer as with any folder. Without Google
  Drive for desktop a sentence and a link to Google's page for it stand
  there instead. Once the folder lies in Drive, the line of the last round
  says so beside Drive's mark (`roundInWords`). The part "Google Drive" of
  the dialog "Integrations…" says how it works and leads to Computers;
  nothing is set per project.
- **Marks of other services.** `ServiceMark` (`shared/ui`) shows the
  official mark of GitHub or Google Drive from `shared/logos/`, small,
  before the name of the part it heads, never recoloured. GitHub's black or
  white form is chosen by the tokens `--mark-black` and `--mark-white`,
  which every theme sets after its ground. Slack has no mark: it is named
  in words.
- **A conversation is continued only where it runs.** A conversation whose
  `isHere` is false is read, never continued: `ConversationView`, the
  Advisor's page and the inbox leave out the reply box, Stop and Open in
  Claude, show a question with its options but nothing to pick
  (`DecisionCard` with `answeredOn`), and put `RunsElsewhere` in their
  place, with the name of the computer from the state of syncing.
- **A proposal is changed where it stands.** `ProposalCard` keeps no draft
  of its own: every change (a title or a size typed in its row, an owner,
  a description, a priority, a task to wait for picked with `Picker` from
  `features/tasks/TaskPicker.tsx`, a task added or taken out, a member
  taken out or given another model or effort) goes to the backend at
  once through `changeProposal`, and the card shows the proposal as it
  comes back. A refused change shows its reason under the row it
  concerns, not in a notice. Approve sends the proposal as it reads. The
  demo backend applies the same changes with the same refusals
  (`proposalAfter` in `shared/api/demo/planning.ts`).
- **What belongs to the window stays in the window.** Which view or place
  is open (`app/navigation.tsx`) and which tasks show their subtasks
  (`features/tasks/subtaskFolds.ts`) say something about the window and
  nothing about the project. They are kept in the storage of the web view
  and never sent to the backend. The views themselves belong to the
  project and are asked of the backend. A view is remembered by its
  identifier; when it is gone, the project opens on its first view.
- **Views are drawn through one table of kinds.** `views/viewKinds.tsx`
  gives each kind its label, what it is for, its mark and the component
  that draws it; a new kind is one entry. The wire knows twenty kinds; the
  chooser offers only those with an entry, and a view of a kind without
  one, which another computer or a newer program may have added, is drawn
  as a board. Each kind's component gets the view itself (`KindProps.view`);
  a kind that is a page reads `view.content` and writes it with
  `useSetViewContent` (`views/viewQueries.ts`), which shows the new content
  at once and writes the content of one view in order, at most about once
  a second. The filter of a view is applied
  in the workspace, where the tasks are handed to the view
  (`views/viewModel.ts`), so a view draws fewer tasks without knowing why.
  The board alone keeps every task and is told which ones get a card,
  because it places a dropped card among all the tasks of its status and
  counts the tasks a card waits for.
- **The open area is applied the same way.** The workspace hands a view
  only the tasks of the open area (`areas/areaModel.ts`), and the filter
  of the view on top. `OpenAreaProvider` makes the open area known to the
  task commands, so that a task made anywhere in a view goes into it, and
  to the dashboard, which names it. Which area is open is part of
  `app/navigation.tsx`: remembered like the open view, kept while the
  inbox or the computers are open, and every area when it is gone.
- **Two backends.** `TauriBackend` calls the shell. `DemoBackend` runs in the
  browser's memory with a sample project, so the interface can be developed
  and tested without Rust.
- **What a browser cannot do, the demo does not offer.** Every demo is a
  browser, so an action that needs the shell is shown as not available
  ("In the desktop program, not in the demo") or left out, through
  `useIsDemo`: export to Excel, opening a file with its program or showing
  it in its folder, the test notification, the question whether to allow
  notifications, "Show folder" and "Open in Claude Code". The demo backend
  answers them with `not_available` all the same, so a gesture that still
  reaches one (a double click on a file) says so in a notice.
- **The public demo** on the website
  ([decision 0032](../../docs/decisions/0032-a-public-demo-in-the-browser.md))
  is this interface built in the mode `public-demo` (`npm run build:demo`,
  into `dist-demo`, with file names relative to the page so that it works
  under any address). `shared/lib/publicDemo.ts` says whether a build is
  that one; Vite settles it when it builds, so the desktop program carries
  none of what follows. Only that build has `app/PublicDemoFrame.tsx`: a
  band above the window that says it is a demo on sample data, that nothing
  is saved and no AI runs, with links to the website and the releases, and
  on a screen smaller than 760 by 500 a page that says the demo is made for
  a larger screen before it shows the whole window, laid out 1100 pixels
  wide. It also opens with work going on: `connectToBackend` calls
  `DemoBackend.beginSampleWork`, and the first specialist begins its task
  and then asks the visitor to decide. The demo of `npm run dev` has none
  of this, so that the pictures taken of it stay as they were. The website
  publishes it; see [its README](../../website/README.md#the-demo).
- **Logic is tested without rendering.** Pure functions live in `*Model.ts`
  and `shared/lib`, next to their tests. The Gantt chart, the calendar and
  the burndown chart only draw what `ganttModel.ts`, `calendarModel.ts` and
  `burndownModel.ts` work out. The pace of the team is the backend's
  (`sprintForecast`); `forecastModel.ts` only puts it into words. The
  actual dates of the Gantt chart and how late they were are the backend's
  too (`listTaskActuals`); `ganttModel.ts` only places them.
- **The dashboard is worked out, not written**
  ([decision 0020](../../docs/decisions/0020-a-dashboard-worked-out-not-written.md)).
  Its sentences, numbers and bars come from `dashboardModel.ts`, from the
  tasks, the team, the conversations and the sprints the interface already
  has. It asks no model and adds no command. It reuses the burndown and the
  completed sprints of `sprints/` and the count of what waits and the words
  for tokens of `office/officeModel.ts`. Its part Usage is the exception:
  a table of what every specialist, the Advisor and those who left spent,
  by kind, in all, in turns, and the model they ran on last, with the
  project at the foot. Those figures are the backend's (`projectUsage`);
  `usageTable` only names them and puts them in order.
- **The context of an agent is drawn, not worked out.** `work/ContextMeter`
  draws how full a session is ("Context 38k of 200k" over a hairline meter,
  in the warning colour from 80 %, its title saying the session will soon
  have to make room) from the `context` the backend sends. The office shows
  it on the place of every specialist and of the Advisor, the conversation
  panel of a task at its head, and the Team view beside what each
  specialist used. `ProjectWorkspace` asks for the usage and hands it down
  through `WorkContext` (`useUsageOfProject`). The "Used" figures of the
  office are still added up from the turns (`tokensOfTurns`).
- **Sprints were in the contract before they were in the shell.** The
  demo backend and `demoSprints.test.ts` came first and said what the
  shell was to do; the Rust side was written against them, case by case
  ([decision 0016](../../docs/decisions/0016-sprints-and-a-backlog.md)).
  A change to sprints goes the same way: the demo and its tests first.
- **Dates are counted, not clocked.** A calendar date is a whole number of
  days (`shared/lib/days.ts`), so adding days never meets a time zone or a
  change of the clocks.
- **The contract is mirrored by hand.** `shared/api/types.ts` and
  `src-tauri/src/dto.rs` describe the same data and change together.

## The look

The interface follows the design philosophy in
[`docs/design`](../../docs/design/README.md).

- Every color, size and radius is a token in `shared/styles/tokens.css`.
  Components never use raw values. Lamplight, the default theme, is those
  tokens: a dark and a light set that follow the operating system.
- **Themes** (decision 0028). `shared/styles/themes.css` gives the nine
  other themes, one fixed mode each, as sets of the same tokens under
  `data-theme` on the root; `shared/theme/themeContrast.test.ts` measures
  their contrast with the pure functions of `shared/theme/contrast.ts`.
  The four special palettes draw a pattern (`--pattern`, from
  `tools/design/theme-patterns.mjs`) on the ground of the window, a little
  stronger above the foot of the sidebar and in the band of an empty state
  (the global class `pattern-band`). `features/appearance` is the chooser,
  in the section Theme of the page Computers, and `useKeepTheme`, which
  turns the window to the theme the backend keeps. The window remembers
  its theme in `localStorage` (`teamwright.theme`), and
  `public/remembered-theme.js` sets it on the root before anything is
  drawn, so the window does not open in Lamplight and then turn. What
  draws with its own colours listens for `subscribeToWindowLook`
  (`shared/theme/applyTheme.ts`): the office and the portraits. A sheet
  marked `data-paper` stays paper in every theme. A theme the manager
  chooses is crossed into in 400 ms on an eased curve
  (`--duration-crossing`, `--ease-crossing`): a view transition of the
  whole window where the web view has one (WebView2 does), an eased
  transition of every colour where it does not, and at once with motion
  turned down. In the demo,
  `window.teamwrightTheme('tide')` shows a theme as the chooser does.
- The gold has two tokens: `--brass` for marks (a rule, a block, a ring)
  and `--brass-text` for words. On paper the gold of a mark is too light to
  be read, so words in gold always use `--brass-text`.
- A status, a priority or a person gets its color through a `data-tone`
  attribute, which sets `--tone`, `--tone-text` and `--tone-wash`.
- Styles are CSS Modules, one file next to each component.
- Menus, dialogs, popovers and tooltips are built on Radix primitives, which
  handle focus and the keyboard. The calendar is `react-day-picker`.
- Labels use the global class `label`: small capitals in the monospace.
- Who a task is given to looks the same everywhere. The manager is a brass
  circle. An AI specialist is a block with its initials in its own color,
  like a part of the logo. A person is a circle with initials. Nobody is a
  dashed outline.
- Light falls only where something happens or waits for the manager
  ([decision 0015](../../docs/decisions/0015-light-where-something-happens.md)).
  A card at rest is flat. Under the pointer a lamp follows it
  (`shared/ui/lamplight.ts`). A card a specialist works on has a point of
  light that travels round its edge and a sun in its corner, next to the
  gold block. A card that waits for the manager is held by four gold
  corners and says "Waits for you". Every light stands beside a mark or a
  word that says the same, because a picture or reduced motion shows no
  light. The office draws the same rule as a room
  ([decision 0018](../../docs/decisions/0018-the-office-as-a-plan-of-a-workshop.md)):
  a lamp is lit over a bench where work goes on, and a bench that waits
  for the manager borrows the corners of a waiting card. What each bench
  shows, and how many benches stand in a row, is worked out in
  `officeModel.ts`.
- The office has people in it
  ([decision 0019](../../docs/decisions/0019-people-in-the-office.md)).
  Each is a pixel figure written by hand as a grid of letters in
  `office/characters/sprites.ts`; `palette.ts` names every color a figure
  may have, the only place where colors are data. `characterModel.ts`
  reads a member's role from the words of its name and responsibility,
  and its skin and hair from its identifier. `spriteModel.ts` puts a
  figure together in a pose and adds its outline. `paint.ts` paints it at
  a whole number of device pixels per pixel, so it is never blurred.
  `office/roomModel.ts` works out where everybody sits, the aisles, the
  shared places and who walks when; `roomPainter.ts` runs the one loop
  that draws the room on a canvas over it, and stops it when the window
  is hidden or motion is turned down. The scale of the figures comes from
  the token `--office-scale`, three in a wide window and two in a narrow
  one, so that the painter and the depth of the aisles always agree. A member's portrait (`Portrait`)
  stands on its bench and in the Team view. To look at every figure in
  every pose at once, a contact sheet is easy to make: draw
  `standingFigure` and `benchScene` with `paintSprite` on a page of their
  own, at a large scale, on both grounds.
- Movement comes from the motion tokens: named curves and durations, with
  a reduced form for each under `prefers-reduced-motion`, declared with
  `!important` so that it wins over the rule in `base.css` that ends every
  other animation at once. What is played from code, such as the drop of a
  dragged card, reads the same tokens through `shared/ui/motion.ts`. A side
  panel plays its closing before it goes, through `useLeaving`.
- A subtask is shown under its task, never next to it: as a smaller card on
  the board and as a row set in on the list, each with the mark of its own
  status. The task ends in a fold (`SubtaskFold`) that counts its subtasks
  and shows one block for each in the pigment of its status.

## Commands

Run from the repository root.

| What | Command |
|---|---|
| The application with live reload | `npm run app` |
| The interface in a browser | `npm run dev` |
| The public demo, built into `apps/desktop/dist-demo` | `npm run build:demo --workspace @teamwright/desktop` |
| Types, lints, formatting and tests | `npm run check` |

See [development](../../docs/development/README.md) for the rest.
