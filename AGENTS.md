# Guide for AI agents working on Teamwright

Read this before you change anything. It is short on purpose. The documents
it points to are the source of truth.

## What this is

Teamwright is a desktop application in which a human manager runs a project
with a team of AI specialists. It is a Tauri app: Rust in the back, React and
TypeScript in the front. Start with [README.md](README.md) and
[the vision](docs/product/vision.md), then
[the architecture](docs/architecture/overview.md).

## Where things are

| Path | Contents | Its guide |
|---|---|---|
| `crates/domain` | The model and its rules. No input or output. | [README](crates/domain/README.md) |
| `crates/application` | Use cases, and the interfaces they need. | [README](crates/application/README.md) |
| `crates/storage` | SQLite storage and the operation log. | [README](crates/storage/README.md) |
| `apps/desktop` | The Tauri shell and the interface. | [README](apps/desktop/README.md) |
| `docs/decisions` | Why things are the way they are. | [README](docs/decisions/README.md) |
| `docs/design` | The visual language. | [README](docs/design/README.md) |
| `docs/story` | The history of the project. | [README](docs/story/README.md) |

## How to check your work

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
npm run check
```

All four must pass before a commit. For a change to the interface, also look
at it: `npm run dev` serves it in a browser on demo data. See
[development](docs/development/README.md).

## Rules that are easy to break

1. **Small commits.** One logical change per commit, with a
   [Conventional Commits](https://www.conventionalcommits.org/) message whose
   body says what changed and why. Every commit passes the checks.
2. **Dependencies point inwards.** `domain` knows nothing of the others.
   `application` knows `domain`. Everything else may know both.
3. **No rules in the interface.** A rule belongs in `domain` (about one
   value) or `application` (about several stored things).
4. **Logic follows the category of a status, never its name.** Managers name
   statuses freely.
5. **Every database write is logged.** Use `Database::write` and its
   `OperationRecorder`. Update rows in place; never replace them.
6. **The wire contract has two halves.** `apps/desktop/src-tauri/src/dto.rs`
   and `apps/desktop/src/shared/api/types.ts` change together.
7. **The interface uses design tokens and its own components.** No raw
   colors or sizes, no browser drop-downs or date fields. Follow
   [the design philosophy](docs/design/philosophy.md).
8. **No `unwrap` or `expect` outside tests.** `unsafe` needs
   `#[expect(unsafe_code, reason = "…")]`.
9. **Write it down as you go.** Update the README of the part you changed and
   the marks in [the roadmap](docs/product/roadmap.md). A lasting decision
   gets a [decision record](docs/decisions/README.md). A working session gets
   a chapter in [the story](docs/story/README.md).
10. **Honest status.** Do not call something done until it has been run.
    Say what was not verified.
11. **`References/` stays private.** It is ignored by git. Never add it or
    copy from it.
12. **The name.** The product is Teamwright. "Claude" and "Claude Code" may
    appear in text that says what Teamwright runs on, never in the name of
    the product or of a feature.

## Writing

Code, comments and documents are written in plain English: full words, short
sentences, no jargon that a newcomer would have to look up. A name says what
a thing is. A comment says why.
