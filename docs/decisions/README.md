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
| [0010](0010-one-process-for-each-turn.md) | Run one Claude Code process for each turn | Accepted |
| [0011](0011-what-a-specialist-may-do.md) | Let a specialist read and write in the folder of the project, and nothing else | Accepted |
| [0012](0012-a-specialist-asks-through-a-tool.md) | Let a specialist ask the manager through a tool of its own | Accepted |
| [0013](0013-the-folders-of-a-project.md) | Give a project a work folder the manager can choose, and folders that are only read | Accepted |
| [0014](0014-the-advisor-plans-through-two-tools.md) | Let the Advisor interview and propose through two tools | Accepted |
| [0015](0015-light-where-something-happens.md) | Let light fall only where something happens or waits for the manager | Accepted |
| [0016](0016-sprints-and-a-backlog.md) | Plan the work in sprints, with a backlog and story points | Accepted |
| [0017](0017-sync-through-a-folder.md) | Keep computers in step through files of operations in a folder the user owns | Accepted |
| [0018](0018-the-office-as-a-plan-of-a-workshop.md) | Show the team at work as a workshop drawn in plan | Accepted |
| [0019](0019-people-in-the-office.md) | Put people in the office, drawn as pixel characters | Accepted |
| [0020](0020-a-dashboard-worked-out-not-written.md) | Work the dashboard out by rules, and have nothing written for it | Accepted |
| [0021](0021-views-the-manager-can-add.md) | Let the manager add, name, filter, order and remove the views of a project | Accepted |
| [0022](0022-areas-inside-a-project.md) | Cut a project into areas, one for each job inside it | Accepted |
| [0023](0023-a-website-without-a-framework.md) | Make the website one static page, with the film drawn from frames | Accepted |
| [0024](0024-a-view-keeps-content-of-its-own.md) | Let a view keep content of its own, as text the domain does not read | Accepted |
| [0025](0025-the-project-as-a-workbook.md) | Give the manager the project as an Excel workbook, gathered in Rust and written by a crate of its own | Accepted |
| [0026](0026-notifications.md) | Tell the manager through notifications of the system, off until allowed, worded by a rule in the application | Accepted |
| [0027](0027-integrations.md) | Three integrations, set per computer: GitHub through the manager's own gh, a Slack channel, and web addresses that listen | Accepted |
| [0028](0028-themes.md) | Ten themes as sets of the design tokens, chosen per computer, crossed into smoothly | Accepted |
| [0029](0029-handoffs-and-messages-between-specialists.md) | Handoffs and messages between specialists, through the team bus: the latest handoff of a task counts, a message comes with the receiver's next turn | Accepted |
| [0030](0030-tags-and-checklists.md) | Tags that belong to a project, one checklist inside each task, each tag and item merging on its own | Accepted |
| [0031](0031-instructions-and-ready-made-specialists.md) | Instructions of a specialist's own, after Teamwright's and never instead, and twelve ready-made specialists in code | Accepted |
| [0032](0032-a-public-demo-in-the-browser.md) | Publish the interface on the demo backend as a page of the website | Accepted |
| [0033](0033-work-in-progress-limits-and-templates.md) | Limits on the work in a status as a signal, and templates a project starts from | Accepted |
| [0036](0036-project-memory.md) | Project memory: decisions, facts and contracts that are replaced, never changed; a plain search; the bus reads memory from a file of the turn | Accepted |

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
