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

The test does not need Claude Code and does not use it. It points the
application at a stand-in (`tools/shared/stand-in-agent.cmd`) that says it is
installed and signed in and cannot do any work. That is enough to check how
work is started and how a turn that fails is shown.

## A real specialist at work

```bash
node tools/smoke/specialist-smoke.mjs target/debug/teamwright-desktop.exe
```

This is the one test that is not free. It forms a team of one AI specialist
in the built application and lets it do a real, small piece of work with the
Claude Code that is installed on this computer, signed in as whoever is
signed in there: write a file, change it after an answer, answer a question,
and be stopped half-way. It checks the steps, the report, the file on disk,
where the task moves, and that the record survives a restart. It prints what
every turn used.

Because it uses a part of that account's plan, it is not part of any
automatic check. Run it by hand after a change to the work of the
specialists. It ends with code 2, without a failure, if Claude Code is not
installed or not signed in. Windows only for now.

The tests of the runtime crate that need the real program are ignored by
default for the same reason:

```bash
cargo test -p teamwright-runtime -- --ignored
```

## Pictures for the documentation

```bash
node tools/screenshots/capture-screenshots.mjs target/debug/teamwright-desktop.exe
```

This takes the pictures in [`docs/screenshots`](../screenshots/README.md)
again: the built application on a sample project, in the dark and in the
light theme. Run it after a change to the interface that the pictures would
show, and commit the new pictures with the change. Windows only for now.

The two pictures of a specialist at work are not taken by this tool, because
nobody works in the sample project. The test of a real specialist takes
them when it is given the folder:

```bash
node tools/smoke/specialist-smoke.mjs target/debug/teamwright-desktop.exe docs/screenshots
```

## Where the data lives

The application keeps its database in the folder the operating system assigns
to it (`%APPDATA%\dev.teamwright.app` on Windows). Set `TEAMWRIGHT_DATA_DIR`
to use another folder.

What the specialists produce is not in the database either: it is files in
the folder of their project, `Teamwright\<key of the project>` in the user's
documents. With `TEAMWRIGHT_DATA_DIR` set, those folders are in `projects`
inside the data folder instead, so the tools above never write into the
user's documents.

Set `TEAMWRIGHT_AGENT_PROGRAM` to the path of a program, and Teamwright runs
the specialists with it instead of looking for Claude Code. This is for a
Claude Code that is installed where nobody would look, and for the tools
above: they name the stand-in, so that a test or a picture does not depend
on the Claude Code of the computer it runs on, and never shows how its user
is signed in.

What the interface remembers about itself, such as the project and the view
that were open, is not in the database. The web view keeps it in a folder of
its own. The tools above give every run a fresh one through
`WEBVIEW2_USER_DATA_FOLDER`, so that a run never sees what an earlier run or
the installed application left behind.

## The layout of the repository

| Path | Contents |
|---|---|
| `crates/domain` | The model and its rules. No input or output. |
| `crates/application` | The use cases, and the interfaces they need. |
| `crates/storage` | SQLite storage and the operation log. |
| `crates/runtime` | The adapter for Claude Code, the program that runs the specialists. |
| `apps/desktop/src-tauri` | The desktop shell: commands that wrap the use cases. |
| `apps/desktop/src` | The interface: React and TypeScript. |
| `docs` | Product, architecture, decisions, design, story, screenshots. |
| `tools` | Scripts for development: the key visual, the smoke test, the screenshots. |

Each crate and the interface have a README of their own. The
[architecture overview](../architecture/overview.md) explains how they fit
together.
