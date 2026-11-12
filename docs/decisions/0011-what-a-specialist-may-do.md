# 0011. Let a specialist read and write in the folder of the project, and nothing else

- Status: accepted
- Date: 2026-11-12

## Context

A specialist is a Claude Code session on the manager's own computer, with
the manager's files, and with whatever the manager connected to Claude
Code: skills, plugins, MCP servers, connectors to mail and calendars.

Claude Code asks before it uses a tool that could do harm. In a session
that Teamwright runs there is nobody to ask: the session has no window. The
program can be told to deny instead of asking, to approve changes to files
in its working folder, to approve everything, or to let a model judge what
is safe.

The architecture says that risky tool use is never approved by Teamwright on
the manager's behalf, but forwarded to the manager. The part that forwards
it, the team bus with its permission requests, is a later milestone.

## Decision

Until a request for permission can reach the manager, a specialist works
with this profile:

- It may read and search, and create and change files, inside the folder of
  its project. That folder is `Teamwright/<key of the project>` in the
  manager's documents.
- It has no shell. The tools that run commands are removed from the
  session.
- Of the built-in tools it has only the ones for reading, searching and
  editing files, for skills, and for finding the tools of MCP servers. Tools
  that schedule work, start other sessions or publish something are not
  there at all.
- Everything else that would need approval is denied at once, so a turn
  never waits for an answer that nobody can give. What was denied is shown
  to the manager with the turn.
- The modes of Claude Code that approve everything, or that let a model
  decide what is safe, cannot be chosen. The type that describes the
  permissions of a turn does not contain them.

The profile is data that the application hands to the runtime. The runtime
only translates it into options of the program.

## Consequences

- A specialist can produce what is made of files: documents, source code,
  schematics as text, calculations, plans. It cannot compile, run tests,
  fetch from the web or order parts. When a task needs one of these, the
  specialist has to say so in its report.
- The folder of the project is where the specialists hand work to each
  other and to the manager. Nothing of theirs is hidden in the database.
- A specialist cannot be told from the interface to be less careful. That
  is on purpose.
- The session still knows the manager's own MCP servers and connectors.
  Their tools are denied unless the manager's own Claude Code settings
  allow them for every session. When the team bus is built, a specialist
  should get only the servers Teamwright gives it.

## Alternatives considered

- **Approve everything.** Fast, and it puts the manager's computer and
  accounts in the hands of whatever a document tells the model to do.
- **Let a model judge what is safe.** That is an approval on the manager's
  behalf, by another model.
- **Wait until permission requests can be shown.** No specialist could work
  before the team bus exists, although most of their work is writing files.
