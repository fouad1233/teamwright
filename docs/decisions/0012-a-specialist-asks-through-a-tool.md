# 0012. Let a specialist ask the manager through a tool of its own

- Status: accepted
- Date: 2026-11-12

## Context

A specialist that meets a choice which is the manager's to make must not
guess. Until now it could only say so in its report, as a sentence among
others, and the manager had to find it. The product promises more: a
question in an inbox, with why it matters, the options with what each leads
to, and a recommendation ([vision](../product/vision.md)).

The architecture has planned a team bus from the start: a small tool server
that every session of a specialist is given, through which it talks to the
project. Three things were not known until they were tried against Claude
Code 2.1.322:

- A session in restricted mode can be given one tool server of its own,
  with `--mcp-config` and `--strict-mcp-config`, and sees no other.
- A tool of that server can be used without asking when it is named in the
  rules of the session.
- The call appears in the stream of events that Teamwright already reads,
  with everything the specialist passed to the tool.

A specialist on the lightest model, asked to choose how a device is powered,
called the tool with a question, a reason, four options with their
consequences and a recommendation, and then ended its turn in one sentence.

## Decision

- Every session of a specialist is given one tool server, `teamwright`, with
  one tool, `ask_manager`. Its arguments are the question, why it matters,
  two to four options with what each leads to, and the option the
  specialist recommends.
- The tool server is the Teamwright program itself, started by Claude Code
  with the argument `--team-bus`. It speaks over standard input and output,
  opens no window and no database, and keeps nothing.
- The question is recorded from the stream of events, not by the tool
  server. The server only checks the arguments and answers that the
  question is with the manager. So there is one way in which a turn reaches
  the record, and the server needs no way to reach the running program.
- Asking does not block. The specialist ends its turn. The question is open
  until the manager answers, and the answer is the request of the next turn
  of the same session.
- While a question is open in a conversation, answering it is the only way
  to go on with that conversation. A task whose work ended in a question
  stays in work.
- A specialist that waits is not busy. It can be given something else.

## Consequences

- A decision request has a fixed shape, which the interface can show as
  what it is: a question with options that can be picked, not a paragraph.
- The same tool server is the place for what comes next: handing work to
  another specialist, messages between specialists, asking for permission
  to do something risky.
- The arguments of the tool are checked in one place that both the server
  and the reader of the stream use, so the two cannot disagree about what a
  valid question is.
- A specialist can still write a question into its report instead of
  using the tool. The instructions tell it not to, and the interface cannot
  tell.
- Starting the program a second time as a tool server costs a moment at the
  start of every turn.

## Alternatives considered

- **A marked block in the report.** The specialist writes its question in
  an agreed form at the end of its report, and Teamwright reads it out. It
  needs no server, and it breaks as soon as a model formats the block a
  little differently.
- **Let the tool server write the question into the database.** Two
  programs would write to one database at once, and a question could be
  recorded for a turn that the manager has just stopped.
- **Keep the session waiting until the manager answers.** A process would
  wait for hours, holding memory, and die with the program.
  [Decision 0010](0010-one-process-for-each-turn.md) is against it.
