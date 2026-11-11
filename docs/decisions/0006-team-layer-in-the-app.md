# 0006. Keep the team layer in Teamwright and coordinate in code

- Status: accepted
- Date: 2026-11-11

## Context

A team needs a shared task list, a way to send messages, a way to ask the
manager, and something that decides who works next. Claude Code has an
experimental feature for this, agent teams, but it does not run in headless
mode, which is the only mode an app can drive.

There is also a cost question. When a lead agent coordinates the others, every
status check and every routing step is a model call. Claude Code's
documentation reports roughly seven times the token use of a normal session
for agent teams in plan mode.

## Decision

- **Teamwright owns the team's state:** tasks and their dependencies,
  messages, decision requests, handoffs, artifacts and project memory.
- **The coordinator is ordinary code.** It starts a task when its
  dependencies are finished and its owner is free, delivers messages, wakes a
  team member when an answer arrives, and enforces limits. It makes no model
  calls.
- **Team members reach the team through tools.** Teamwright runs a local MCP
  server, the *team bus*, and gives every session access to it. The tools
  cover asking the manager, messaging a teammate, publishing a handoff,
  updating a task, reporting progress, and reading or writing project memory.
- **The Advisor returns structure through tools too.** The brief, the
  staffing proposal and the plan are submitted as tool calls with a defined
  shape, so the app can show them as editable forms instead of parsing prose.
- **Asking never blocks a process.** A team member that asks the manager
  finishes its turn. When the answer arrives, the coordinator starts a new
  turn with it.

## Consequences

- Coordination costs no tokens and behaves the same way every time.
- A request for the manager survives restarts and can wait for days.
- The team bus is a local server and must be closed to everything else: it
  listens on the loopback address only and every session gets its own token,
  which also tells the bus who is calling.
- The quality of a team depends on how well its members use the tools, so the
  tool descriptions and each member's instructions are part of the product
  and need tests of their own.

## Alternatives considered

- **Claude Code agent teams.** Not available in headless mode, and the
  coordination is paid for in tokens.
- **Files in a shared folder as the message channel.** Simple, but team
  members would have to poll, and nothing would enforce structure.
- **A lead agent that coordinates.** Flexible, expensive, and hard to make
  repeatable. The Advisor is called for judgment only: planning, replanning,
  reviewing.
