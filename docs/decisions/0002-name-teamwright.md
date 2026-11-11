# 0002. Name the project Teamwright

- Status: accepted
- Date: 2026-11-11

## Context

The project needed a name that nobody else uses. The founder had already
found that the obvious compounds were taken. Two more constraints applied:

- Anthropic's rules for products built on Claude Code do not allow "Claude" or
  "Claude Code" in a product name. "Powered by Claude" is allowed.
- The name becomes the repository, the crate prefix and the application
  identifier, so changing it later is expensive.

Candidates were checked on 11 November 2026 against the npm registry,
crates.io, GitHub repository and account names, and a web search.

| Candidate | Idea | Result |
|---|---|---|
| Teamwright | A wright is a builder: it builds your team | No product found; registries free |
| Sprintfield | Sprint plus field | Free, but voice typing writes "Springfield" |
| Ganttry | Gantt plus gantry | Free, but easy to misspell and names one feature |
| Crewstead | A homestead for a crew | A GitHub organization with this name appeared ten days earlier |
| Guildry | A guild of specialists | Crowded: Guild.ai and Guildless work in the same field |
| Bullpen, Quarterdeck, Kadro, Jidoka | Various | Already used by other software |

## Decision

The project is named **Teamwright**.

A *wright* is a maker, as in shipwright or playwright. Teamwright builds the
team for a job. Spoken aloud it is also "team right": the right team, the
right size.

## Consequences

- Repository `teamwright`, Rust crates `teamwright-*`, application identifier
  `dev.teamwright.app`.
- A dormant personal GitHub account already holds the handle `teamwright`, so
  a future organization needs a variant.
- These checks are not a trademark clearance. One is needed before any
  commercial use of the name.

## Alternatives considered

See the table above. The founder chose from a shortlist of four.
