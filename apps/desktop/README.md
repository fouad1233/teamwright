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
  its `interview` and its `proposal`, and an owner of a proposed task is
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
  ended the same way at the next start.

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
    work/             The work of AI specialists: starting it, following it, answering
    inbox/            What waits for the manager in every project: questions to decide
    advisor/          Planning a project with the Advisor: its questions, its proposal
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
  `useTeamCommands`, `useConversations`, `useWorkCommands`). Nothing calls
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
- **What belongs to the window stays in the window.** Which view is open
  (`app/navigation.tsx`) and which tasks show their subtasks
  (`features/tasks/subtaskFolds.ts`) say something about the window and
  nothing about the project. They are kept in the storage of the web view
  and never sent to the backend.
- **Two backends.** `TauriBackend` calls the shell. `DemoBackend` runs in the
  browser's memory with a sample project, so the interface can be developed
  and tested without Rust.
- **Logic is tested without rendering.** Pure functions live in `*Model.ts`
  and `shared/lib`, next to their tests. The Gantt chart and the calendar
  only draw what `ganttModel.ts` and `calendarModel.ts` work out.
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
