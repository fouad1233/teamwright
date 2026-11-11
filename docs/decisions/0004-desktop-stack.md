# 0004. Build the desktop app with Tauri, Rust and React

- Status: accepted
- Date: 2026-11-11

## Context

The founder asked for a Rust backend, a React frontend, and a real executable
that opens like a native application while rendering its interface with a
browser engine. The backend has to manage long-lived child processes, a local
database and a local server, which is the kind of work Rust does safely.

## Decision

- **Shell:** Tauri 2. The core is a Rust process; the interface runs in the
  operating system's own web view (WebView2 on Windows).
- **Backend:** a Cargo workspace of small crates with one responsibility
  each, described in the [architecture overview](../architecture/overview.md).
- **Frontend:** React with TypeScript in strict mode, built with Vite.
- **Boundary:** the interface calls typed commands and listens to typed
  events. It holds no business rules.

## Consequences

- Installers are small because no browser is bundled.
- Building from source needs the Rust toolchain and Node.js.
- The interface must be tested on each platform's web view.
- Because the interface depends only on a typed backend interface, a second
  implementation of that interface can run it in a plain browser with demo
  data. This is used for development and for a public demo.

## Alternatives considered

- **Electron.** Mature, but the backend would be Node.js rather than Rust and
  every install would carry its own browser.
- **A local web server opened in the user's browser.** No window of its own,
  no tray, no native notifications.
- **A native Rust interface toolkit.** No React and a far smaller pool of
  contributors.
