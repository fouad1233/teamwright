# Development

How to build, run and check Teamwright from source.

## What you need

| Tool | Version | Notes |
|---|---|---|
| Rust | stable | `rustup` reads the channel from `rust-toolchain.toml` |
| A C compiler | | SQLite is compiled from source. On Windows this is the Microsoft C++ Build Tools, or [the setup without administrator rights](windows-without-admin.md) |
| Node.js | 22 or later | |
| WebView2 | | Windows only, and part of Windows 11 |

On Linux and macOS the [Tauri prerequisites](https://tauri.app/start/prerequisites/)
apply.

## First time

```bash
npm install
```

## Run

| What | Command |
|---|---|
| The desktop application, with live reload | `npm run app` |
| Only the interface, in a browser, on demo data | `npm run dev` then open http://localhost:1420 |
| The interface on an empty demo | http://localhost:1420/?demo=empty |

The interface runs without Rust: in a browser it uses a demo backend that
lives in memory. This is the fastest way to work on the look and feel.

## Check

Run all of these before a commit. The build server runs the same.

| What | Command |
|---|---|
| Rust formatting | `cargo fmt --all --check` |
| Rust lints | `cargo clippy --all-targets -- -D warnings` |
| Rust tests | `cargo test` |
| Interface: types, lints, formatting, tests | `npm run check` |

## Build

```bash
npm run build --workspace @teamwright/desktop        # the interface only
npm run tauri --workspace @teamwright/desktop -- build --debug --no-bundle
```

The second command builds the application as
`target/debug/teamwright-desktop.exe`.

## Smoke test of the real application

```bash
node tools/smoke/desktop-smoke.mjs target/debug/teamwright-desktop.exe screenshot.png
```

This starts the built application on an empty temporary data folder, drives
its window, and checks that the interface, the commands and the database work
together, including after a restart. Windows only for now.

## Where the data lives

The application keeps its database in the folder the operating system assigns
to it (`%APPDATA%\dev.teamwright.app` on Windows). Set `TEAMWRIGHT_DATA_DIR`
to use another folder.

## The layout of the repository

| Path | Contents |
|---|---|
| `crates/domain` | The model and its rules. No input or output. |
| `crates/application` | The use cases, and the interfaces they need. |
| `crates/storage` | SQLite storage and the operation log. |
| `apps/desktop/src-tauri` | The desktop shell: commands that wrap the use cases. |
| `apps/desktop/src` | The interface: React and TypeScript. |
| `docs` | Product, architecture, decisions, design, story. |
| `tools` | Scripts for development: the key visual, the smoke test. |

Each crate and the interface have a README of their own. The
[architecture overview](../architecture/overview.md) explains how they fit
together.
