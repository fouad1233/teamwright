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
`rename_view`, `set_view_filter`, `move_view` and `remove_view`, which the
shell serves through `ViewService`. A view on the wire has its `kind`, its
`name` and its `filter`, whose `assignee` is `{"kind": "anyone"}`,
`{"kind": "manager"}`, `{"kind": "member", "memberId": "…"}` or
`{"kind": "nobody"}`. `move_view` reads the place as any number, cut to a
whole number as the interface's arrays would cut it. A database with
projects from before views gives each of them the six views a new project
starts with when it is opened.

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
- **Planning with the Advisor.** The commands `advise`, `answer_interview`
  and `approve_proposal` in `commands/work.rs` carry it out; `reply` and
  `list_conversations_waiting` serve conversations with the Advisor too. In
  `dto.rs`, a conversation says whether it `isWithAdvisor`, a turn carries
  its `interview` and its `proposal`, a conversation names the computer it
  runs on (`homeComputer`) and says whether that is this one (`isHere`),
  and an owner of a proposed task is
  `{"kind": "manager"}` or `{"kind": "member", "name": "…"}`. A place sent
  as a number that is not a whole number from 0, such as `-1` or `1.5`, is
  read as a place that is not there, so that the use case refuses it with
  the rule it breaks.
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

`SyncStateDto` has `folder`, `thisComputer`, `computerName`,
`suggestedName`, `phase` (`off`, `in_step`, `writing`, `unreachable`,
`stopped`), `lastRound`, `problem` and `computers`, each a `ComputerDto`
with `id`, `name`, `version`, `acceptance` (`undecided`, `accepted`,
`ignored`), `lastHeard`, `clockAhead` and `newerFormat`. A
`SyncNoticeDto` has `id`, `at`, `kind` (`task_renumbered`,
`project_rekeyed`, `change_dropped`, `task_moved`, `clock_ahead`),
`projectId`, `taskId` and `message`. The error codes are those of the
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
    projects/         Sidebar, project header, creating a project, first run
    board/            The board: lanes, cards, drag and drop
    gantt/            The Gantt chart: bars against days, lines of dependencies
    calendar/         The calendar: a month of weeks with the tasks on their days
    tasks/            The task panel, the list, badges, the fold of subtasks, task logic
    team/             The team: avatars, the Team view, adding and changing a member
    office/           The office: the team at work, drawn as a workshop in plan
      characters/     The people of the office: pixel figures drawn in code, and portraits
    dashboard/        How a project is going: a summary in words, numbers, small charts
    work/             The work of AI specialists: starting it, following it, answering
    inbox/            What waits for the manager in every project: questions to decide
    advisor/          Planning a project with the Advisor: its questions, its proposal
    sprints/          The Backlog view, sprints and points, the burndown, the board of a sprint, completed sprints opened
    sync/             Computers: syncing through a folder, the line at the foot of the sidebar, conversations elsewhere
  shared/
    api/              The Backend interface and its two implementations
    ui/               Buttons, menus, dialogs, the calendar field, tooltips
    lib/              Formatting and ranking helpers
    styles/           Design tokens and document-wide styles
```

Rules of the interface:

- **No business rules.** The interface asks the backend and shows the answer.
  Where it shows a change before the backend has answered (moving a card), it
  repeats the backend's arithmetic and corrects itself if the answer differs.
- **One way to the backend.** Components use `useBackend()` or the hooks
  built on it (`useProjects`, `useTasks`, `useTaskCommands`, `useTeam`,
  `useTeamCommands`, `useConversations`, `useWorkCommands`, `useSprints`,
  `useSprintCommands`). Nothing calls
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
- **A conversation is continued only where it runs.** A conversation whose
  `isHere` is false is read, never continued: `ConversationView`, the
  Advisor's page and the inbox leave out the reply box, Stop and Open in
  Claude, show a question with its options but nothing to pick
  (`DecisionCard` with `answeredOn`), and put `RunsElsewhere` in their
  place, with the name of the computer from the state of syncing.
- **What belongs to the window stays in the window.** Which view is open
  (`app/navigation.tsx`) and which tasks show their subtasks
  (`features/tasks/subtaskFolds.ts`) say something about the window and
  nothing about the project. They are kept in the storage of the web view
  and never sent to the backend.
- **Two backends.** `TauriBackend` calls the shell. `DemoBackend` runs in the
  browser's memory with a sample project, so the interface can be developed
  and tested without Rust.
- **Logic is tested without rendering.** Pure functions live in `*Model.ts`
  and `shared/lib`, next to their tests. The Gantt chart, the calendar and
  the burndown chart only draw what `ganttModel.ts`, `calendarModel.ts` and
  `burndownModel.ts` work out.
- **The dashboard is worked out, not written**
  ([decision 0020](../../docs/decisions/0020-a-dashboard-worked-out-not-written.md)).
  Its sentences, numbers and bars come from `dashboardModel.ts`, from the
  tasks, the team, the conversations and the sprints the interface already
  has. It asks no model and adds no command. It reuses the burndown and the
  completed sprints of `sprints/` and the count of what waits and the words
  for tokens of `office/officeModel.ts`.
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
  Components never use raw values. The dark and the light theme are two sets
  of values for the same tokens and follow the operating system.
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
| Types, lints, formatting and tests | `npm run check` |

See [development](../../docs/development/README.md) for the rest.
