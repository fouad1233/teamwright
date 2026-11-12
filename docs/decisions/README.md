# Decision records

Every choice that shapes Teamwright for a long time gets a short record here:
what the situation was, what was decided, what follows from it, and what else
was considered.

## Rules

- One decision per file, numbered in the order they were made.
- A record is never rewritten to match later thinking. When a decision
  changes, a new record supersedes the old one and both stay.
- Keep it short. A record that needs more than a page is usually two
  decisions.

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-decisions.md) | Record significant decisions | Accepted |
| [0002](0002-name-teamwright.md) | Name the project Teamwright | Accepted |
| [0003](0003-apache-2-license.md) | License under Apache-2.0 | Accepted |
| [0004](0004-desktop-stack.md) | Build the desktop app with Tauri, Rust and React | Accepted |
| [0005](0005-drive-the-users-claude-code.md) | Drive the user's own Claude Code in headless mode | Accepted |
| [0006](0006-team-layer-in-the-app.md) | Keep the team layer in Teamwright and coordinate in code | Accepted |
| [0007](0007-local-first-storage.md) | Store locally and sync through a user-owned folder | Accepted |
| [0008](0008-budgets-in-tokens.md) | Measure usage and budgets in tokens | Accepted |
| [0009](0009-a-team-per-project.md) | Give every project a team of its own, with agents and people as one kind of member | Accepted |

## Template

```markdown
# NNNN. Title

- Status: proposed | accepted | superseded by NNNN
- Date: YYYY-MM-DD

## Context
What is true and what forces are at play.

## Decision
What we will do.

## Consequences
What becomes easier, what becomes harder, what we must now do.

## Alternatives considered
What else was on the table and why it lost.
```
