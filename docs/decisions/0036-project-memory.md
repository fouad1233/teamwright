# 0036. Project memory: decisions, facts and contracts

- Status: accepted
- Date: 2026-11-21

## Context

[The architecture](../architecture/overview.md) promises that answered
decisions and published contracts become project memory, which any team
member can search, so that the team does not ask twice and a new task starts
with what it needs. Handoffs and messages ([decision 0029](0029-handoffs-and-messages-between-specialists.md))
carry results from one task to the next, but what was decided or learned once
was still buried in the conversation where it happened.

Four things had to be decided: what an entry is, how entries come about, how
a specialist reads them, and how the team bus, a process of its own that
until now only checked calls ([decision 0012](0012-a-specialist-asks-through-a-tool.md)),
can answer a search with data.

## Decision

- **An entry** belongs to a project and is a **decision**, a **fact** or a
  **contract**, with a title of one line (at most 120 characters), a text (at
  most 20,000, as long as a handoff's contract), the task it belongs to if
  any, when it was recorded, and where it came from: the manager, a
  specialist, an answered decision request, or a handoff, with the
  specialist, conversation and turn unless the manager wrote it.
- **An entry never changes in place.** A change is a newer entry of the same
  kind and task that replaces it; the older one keeps its words and says
  which entry replaced it. Only a current entry can be replaced, and only
  current entries are searched and go into a brief. What the team relied on
  stays readable, and nothing is changed silently. The manager may still
  remove an entry outright.
- **Entries come about four ways.** When the manager answers a decision
  request, the question and the answer become a decision of the task,
  stored once the turn that carries the answer is stored. When a handoff with
  an interface contract is published, the contract becomes the contract of
  its task and replaces the earlier contracts of the task, since the latest
  handoff counts. A specialist records a fact with the tool `remember`: a
  title and at most 2,000 characters, **three in a turn at most**, so that one
  turn cannot flood what every brief and search reads. The manager adds facts
  and changes and removes entries in the place Memory.
- **A search is plain code**: one to twelve words; an entry fits when each
  word begins a word of its title or text, whatever the capitals. No model,
  no embeddings. The specialist and the manager search by the same rule
  (`SearchWords` in the domain). The tool `search_memory` answers with at most
  eight entries, the newest first, each text shortened to 600 characters.
- **The brief carries what plainly belongs to the task**, at most eight
  entries and 3,000 characters, each text shortened to 500: the entries of the
  task, then those of the tasks it waits for (without the contracts their
  handoffs left, which the brief carries already), then every other contract,
  then the entries of the whole project, newest first in each group. It
  always says to search memory before asking the manager; the instructions
  of a specialist say so too.
- **The bus reads memory from a file.** When a turn starts, the runtime
  writes the current entries, in words, to a file of their own in the
  system's folder for temporary files, readable by the user alone, names it
  to the bus with `--memory <file>`, and removes it when the turn is over. A
  fact remembered in the turn is added to what the bus searches. This is the
  smallest honest way: the bus stays a process that never opens the
  database, nothing about SQLite or its locks reaches it, and it searches by
  the same rule the application uses. A file that cannot be read gives a
  search that finds nothing; the other tools still work.
- **Storage** (migration 0025): `memory_entries`, every write logged as
  `memory_entry` and merged by field, with one field, `replaced_by`, the only
  thing that changes. An entry **outlives the task it came from**, because a
  decision or a contract stays true of the project when its task is deleted;
  it goes with its project, each removal logged.
- **The interface** has a fixed place **Memory** between Office and Advisor.
  Memory belongs to the whole project, as the team and the office do, and is
  not a way of looking at tasks, so it is a place, not a view.

## Consequences

- A specialist can find what was decided or agreed without the manager
  retelling it, and every decision the manager makes is kept without anybody
  writing it down.
- The bus searches memory as it was when the turn started, plus what the turn
  itself remembered. What a teammate remembers during the turn is found in
  the next one.
- The file of a turn stays in the temporary folder if Teamwright is ended
  without stopping its turns; the system cleans it.
- A decision whose answer is later reconsidered stays as it was; the manager
  changes the entry or the next answer adds another decision.
- Never run against a real Claude Code: an ignored test asks a real session
  to search and remember through the built bus. The built bus itself was
  driven over standard input, with a memory file, and answered as tested.

## Alternatives considered

- **Let the bus open the database read-only.** Always fresh, but the bus
  would learn SQLite, the place of the data folder and its locks, against
  the rule that it keeps nothing and touches nothing beyond its turn.
- **Pass the entries as arguments.** A command line has a length limit, on
  Windows about 32,000 characters, which a memory passes quickly.
- **Change entries in place.** Simpler, but a fact the team relied on could
  change under it without a trace.
- **Search with a model or embeddings.** Neither predictable nor testable,
  and the manager would not find what the specialist found.
- **Delete the entries of a deleted task.** A decision does not stop being
  true because the task that asked it is gone.
