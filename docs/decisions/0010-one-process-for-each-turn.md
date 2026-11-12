# 0010. Run one Claude Code process for each turn

- Status: accepted
- Date: 2026-11-12
- Changes: [0005](0005-drive-the-users-claude-code.md), which had one
  process per active team member, fed turns on standard input

## Context

Decision 0005 described a team member as one long-lived Claude Code process
that reads one turn after another from standard input. Two other decisions
pull the other way. A team member with nothing to do has no process (0005
itself), and a team member that asks the manager something ends its turn and
does not wait ([0006](0006-team-layer-in-the-app.md)). Between two turns of
the same session there can be hours or days.

When the runtime was built against Claude Code 2.1.322, three things were
seen:

- A session is continued by its identifier at any time, from a new process,
  with `--resume`. What it read before comes back from the cache of the
  model, which costs a fraction of reading it anew.
- The result of a turn names the tokens of that turn alone
  (`model_usage_since_last_result`), next to the sum of the whole session.
- A Claude Code process holds a few hundred megabytes while it is alive,
  whether it works or waits.

## Decision

- One turn is one process: `claude -p` with the prompt on standard input and
  the events of the turn as lines of JSON on standard output. When the turn
  ends, the process ends.
- A later turn of the same session is a new process with `--resume` and the
  identifier the first turn reported.
- The usage of a turn is what the program reports for that turn alone.

## Consequences

- Nothing runs between turns. A team of ten specialists that waits for the
  manager costs no memory and needs no supervision.
- Stopping a turn is ending its process. A turn that was running when
  Teamwright was closed is found at the next start and marked as failed;
  the manager can go on with a reply, which resumes the session.
- Every turn pays for starting the program, about a second or two, and for
  reading the session back from the cache.
- A program that Claude Code itself started, such as a local MCP server,
  is not ended with the process on Windows. Ending the whole tree needs the
  job objects of Windows and therefore `unsafe` code. It is left out until
  it is needed.

## Alternatives considered

- **One process per active member, fed on standard input** (0005 as it was
  written). Saves the start of the program for a turn that follows at once,
  at the price of idle processes, a second protocol for input, and state to
  supervise.
