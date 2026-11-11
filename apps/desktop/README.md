# Teamwright desktop

The desktop application: an interface written in React and TypeScript
(`src`) inside a Tauri shell written in Rust (`src-tauri`).

## The shell: `src-tauri`

The shell opens the window, builds the services, and exposes one command per
use case. It contains no rules of its own.

| File | Contents |
|---|---|
| `src/main.rs` | Starts the application and lists the commands |
| `src/state.rs` | Opens the database and builds the services |
| `src/commands/` | One function per command: read the arguments, call one use case, answer |
| `src/dto.rs` | The data that travels between the interface and the commands |
| `src/error.rs` | The error a command reports: a stable code and a readable message |
| `build.rs` | The build script; see [building without administrator rights](../../docs/development/windows-without-admin.md) |

To add a command: add the use case to the application crate, add a function
in `src/commands`, list it in `main.rs`, and add the matching method to the
`Backend` interface on the TypeScript side.

## The interface: `src`

```
src/
  main.tsx            Starts the interface and provides what every part needs
  app/                The window as a whole, and where the user is in it
  features/
    projects/         Sidebar, project header, creating a project, first run
    board/            The board: lanes, cards, drag and drop
    tasks/            The task panel, the list, badges, and task logic
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
  built on it (`useProjects`, `useTasks`, `useTaskCommands`). Nothing calls
  Tauri directly except `shared/api/tauriBackend.ts`.
- **Two backends.** `TauriBackend` calls the shell. `DemoBackend` runs in the
  browser's memory with a sample project, so the interface can be developed
  and tested without Rust.
- **Logic is tested without rendering.** Pure functions live in `*Model.ts`
  and `shared/lib`, next to their tests.
- **The contract is mirrored by hand.** `shared/api/types.ts` and
  `src-tauri/src/dto.rs` describe the same data and change together.

## The look

The interface follows the design philosophy in
[`docs/design`](../../docs/design/README.md).

- Every color, size and radius is a token in `shared/styles/tokens.css`.
  Components never use raw values. The dark and the light theme are two sets
  of values for the same tokens and follow the operating system.
- A status, a priority or a person gets its color through a `data-tone`
  attribute, which sets `--tone`, `--tone-text` and `--tone-wash`.
- Styles are CSS Modules, one file next to each component.
- Menus, dialogs, popovers and tooltips are built on Radix primitives, which
  handle focus and the keyboard. The calendar is `react-day-picker`.
- Labels use the global class `label`: small capitals in the monospace.

## Commands

Run from the repository root.

| What | Command |
|---|---|
| The application with live reload | `npm run app` |
| The interface in a browser | `npm run dev` |
| Types, lints, formatting and tests | `npm run check` |

See [development](../../docs/development/README.md) for the rest.
