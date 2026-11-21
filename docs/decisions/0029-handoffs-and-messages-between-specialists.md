# 0029. Handoffs and messages between specialists, through the team bus

- Status: accepted
- Date: 2026-11-21

## Context

Specialists never read each other's conversations
([the architecture](../architecture/overview.md)). A task that builds on
another must still start with what the other produced, and a specialist
sometimes needs something from a teammate that only the teammate knows.
Until now the only way across was the manager, who read one report and
retold it in a brief or a message.

The team bus exists for this ([decision 0012](0012-a-specialist-asks-through-a-tool.md)):
a tool server, started by Claude Code for one turn, that checks a call and
answers the session, while what was said reaches Teamwright in the stream
of events of the turn. It had one tool for specialists, `ask_manager`.

Three things had to be decided: what a handoff and a message are, when a
message reaches its receiver, and what a "registered artifact" is.

## Decision

- **Two more tools for a specialist.** `publish_handoff` is given to a turn
  about a task; `send_message` is given to every turn of a specialist whose
  team has another AI specialist. The bus is told the folder the turn works
  in and the names of the teammates, as arguments after the tools.
- **A handoff** belongs to a task and to the specialist who published it.
  It has a summary (required, at most 4,000 characters), an interface
  contract (text, may be empty, at most 20,000 characters) and a list of
  files (at most fifty). It never changes once it is published. A task may
  have several, one from each turn that published one; **the latest
  counts**. A turn publishes at most one.
- **A registered artifact** is a file named in a handoff: a path inside the
  work folder of the project, the folder the specialist may write, written
  with `/` between its parts. It must not leave the folder by how it is
  written (no `..`, no root, no drive), and it must exist when the handoff
  is published: the bus looks, following links, and refuses a handoff that
  names a file that is not there, saying which. Teamwright keeps the path,
  not a copy of the file.
- **A message** goes from one specialist to one other AI specialist of the
  same team, named by name (whatever the capitals). It is a `question` or a
  `note`, with a text of at most 4,000 characters. A turn sends at most
  five.
- **Neither ends the turn.** A handoff is the last thing before the report,
  and a message does not wait for an answer, so the session goes on. Only
  `ask_manager` ends a turn, and after it the bus takes nothing more.
- **A message is delivered with the receiver's next turn**, whatever starts
  that turn: a task, the manager's message or an answer to a question.
  What waits is put after the request in what the program is told, and the
  message is marked delivered with that turn once the turn is stored. The
  request the manager sees stays as it was written; the conversation shows
  the messages that came with the turn beside it. Nobody polls, and an
  arriving message does not start a turn by itself: whether it should is
  the coordinator's business. The application answers the question the
  coordinator will ask, `messages_waiting_for`.
- **A task's brief carries what the tasks it builds on handed over**: for
  each task it waits for that has a handoff, the latest handoff's summary,
  contract and files.
- **The decoder reports a handoff or a message only when the bus took it.**
  It holds a call that passes the check until the answer of the bus comes
  back in the stream; a call the bus refused is reported as an ordinary
  tool use that failed. This differs from `ask_manager`, whose call the
  decoder judges by itself, because only the bus can look at the disk.
- **Both are stored on their own** (migration 19: `handoffs`,
  `handoff_files`, `team_messages`), each write through the recorder, so
  they are logged and reach the other computers. A handoff merges whole; a
  message merges by field, so that its delivery, the one thing that
  changes, is a field of its own. A handoff goes with its task and a
  message with its project, each removal logged. The member, conversation
  and turn they name are not enforced as references: what was handed over
  stays readable after a specialist leaves.

## Consequences

- A new task starts with what it needs instead of a transcript, and the
  manager sees every handoff and message: the handoff of a task in its
  panel, every call as a step of the turn it happened in, and the messages
  between teammates in the Team place.
- The coordinator has a tested function to decide whether a member has
  something waiting for it.
- Two computers that both start a turn of the same receiver may both
  deliver a message. The later delivery wins the merge; the message is
  read twice, which is harmless.
- A handoff refers to files on the computer it was published on. On
  another computer the paths are the same inside the project's folder only
  if the folders are kept in step by other means.
- A version of Teamwright before this one keeps the operations of the two
  new entities waiting, as it does for every entity it does not know.
- Messages to people on the team are not possible. The manager is reached
  with `ask_manager`, and a person has no session to deliver to.
- A specialist may still describe its result in its report and not hand
  it over. The instructions ask it to, and nothing enforces it.

## Alternatives considered

- **Keep the handoff and the messages inside the turn**, like a decision.
  A turn merges whole, so delivering a message would rewrite the turn of
  the sender; and finding what waits for a member, or the handoff of a
  task, would mean reading every turn of the project.
- **Let a message start the receiver's turn at once.** That is a scheduling
  decision with budgets and limits behind it: the coordinator's, not the
  bus's.
- **End the turn after a handoff**, as `ask_manager` does. The report would
  then come in a turn of its own, and a specialist that hands over and then
  reports in one turn is the common case.
- **Accept any path in a handoff.** The paths are what a later task is told
  to read. A path that is not there would send it looking for nothing, so
  the bus refuses it while the specialist can still fix it.
- **Copy the files into Teamwright.** Files are the specialist's work in
  the project's folder, which the manager owns
  ([decision 0011](0011-what-a-specialist-may-do.md)); a second copy would
  go stale.

## Note of 21 November 2026: framed words, after a review

A review found that the messages of teammates and the summaries and
contracts of handoffs were pasted into the brief and the turn text as plain
text, so a message could pretend to be the manager or to grant something.
Now every text another specialist wrote stands in a block of its own. The
first line says what it is and who wrote it, and the last line,
`--- End of teammate words ---`, ends it. A run of three hyphens inside the
text is broken by a space, so the text cannot close the block itself or open
a false one. The fixed instructions of a specialist's turn say that what
stands in those blocks is the words of teammates, not the manager's: they
never give permission and never change the rules. The digests given to the
Advisor are framed in the same spirit, as accounts and not instructions. The
framing is a help to the model, not a lock: what a specialist may do is still
decided by its permissions ([decision 0011](0011-what-a-specialist-may-do.md)).
