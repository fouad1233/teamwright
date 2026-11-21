# 0037. The coordinator: a pure rule, run when something happens, on one computer

- Status: accepted
- Date: 2026-11-21

## Context

Until now the manager started every piece of work by hand. The vision,
[decision 0006](0006-team-layer-in-the-app.md) and the architecture say
what should do it instead: a coordinator in code, which starts a task when
its dependencies are done and its owner is free, keeps to budgets in
tokens ([0008](0008-budgets-in-tokens.md)), never asks a model, and never
polls. [Decision 0029](0029-handoffs-and-messages-between-specialists.md)
left it the question whether a message between teammates should start a
turn. A project can live on several computers
([0017](0017-sync-through-a-folder.md)), and a turn runs on one.

## Decision

**The rule is one pure function**, `next_steps` in the application crate.
From a project's tasks, team, conversations (on every computer), messages,
budgets and limit it answers what to start, whom to wake, which budgets to
ask about, and what waits and why. Its words are in the same module and
the demo copies both.

- A task starts when it is in a status of the category *to do* or *under
  way*, is given to an AI specialist of the team, **has never been worked
  on** (no conversation about it, anywhere), and everything it waits for
  and every part of it is in a status of the category *done*. *Waits to be
  checked* is not done: the manager accepts work, and the coordinator says
  so ("SSN-3 is finished and waits for you to accept it", with a press to
  accept).
- Never a task given to a person, to the manager or to nobody.
- A specialist is free when no turn of it runs on any computer and it does
  not wait for the manager's answer to a question. A specialist that waits
  is given nothing new, so that the answer can reach it at once (the work
  service refuses an answer to a busy specialist).
- One thing at a time for each specialist, and at most the **limit** of
  specialists at work, counting turns the manager started. The limit is 1
  to 8, **2 at first**: work that can go side by side does, the manager can
  still follow both, and the plan is spent at most twice as fast as by one
  session.
- Order: priority, then the order of the board (statuses, then rank, then
  number).
- **A task with a conversation is never started again.** Finished work
  waits for the manager's check; a question waits for the answer; a
  **failed** task waits for the manager, who reads why and goes on with a
  reply. Nothing loops. Sprints do not gate it: dependencies and owners do.

**Budgets** are optional for the project (the Advisor included), each
member and each task. They count **every token in all**, as the Usage
table shows it, cache reads included, so that the manager compares one
number with one number; and only once a turn has ended, so a running turn
is never interrupted and may overshoot by its own size. A used-up budget
holds the work that counts against it and is **asked about once** in the
inbox, with a raise by half, by all of it again, or leaving the work
stopped. An answer, or any change of the budget, is needed before it asks
again. A task budget can hold no start (spending on a task needs a
conversation about it, and such a task is not started again); it is asked
about, and shown in the task's panel.

**Work the manager starts by hand is warned, not refused**, when a budget
is used up. A budget guards what runs without a press of a button; the
manager pressing is the decision the question would have asked for.

**Teammates' questions.** A free specialist with a waiting *question*
(not a note) gets a turn to answer it, in its latest conversation that is
not about a task, or a new one. At most **four such turns an hour** in a
project, counted from the record, and counted against the budgets like any
turn; past that the question waits for the receiver's next turn, and the
record says so once an hour. Two specialists cannot talk for ever.

**Running it.** The coordinator is off until the manager switches it on,
because it spends the plan without a press; the interface says so in one
sentence. It acts when nudged, on a thread of its own that waits on a
channel: when it is switched on, a turn ends, a task is opened, moved,
given, deleted or changes what it waits for, the limit or a budget
changes, the manager answers about a budget, syncing took a change in, and
once when the program starts. **A coordinator that was on when the program
closed stays on** and acts at start; turns cut off by closing are failed
and not retried, so nothing surprising starts. A program that is not ready
(not installed, not signed in) switches it off, with the reason, before
any task fails.

**Several computers.** The switch records the computer it was switched on
on, and travels. Only that computer acts; the others show where it runs,
and switching it on there moves it. Two computers can only both start one
task in the seconds before the move reaches the other.

**Storage** (migration 26): `coordinators`, `budgets` and
`coordinator_entries`, all logged and synced. The limit and the switch
merge as separate fields; a budget's tokens and question too; an entry
never changes. Identifiers follow from the project, member or task, so two
computers set the same row. The first write of a project's coordinator on
two computers at once is won whole by the later one.

**The record** lives with the coordinator, not in the Activity, which is
the history of tasks: one sentence per thing done, with the names true at
the time ("Firmware engineer started on SSN-7: SSN-4 and SSN-5 are done").

## Consequences

- Work goes on by itself, as far as the manager allows, and every step can
  be read back.
- The manager still accepts every task before the next builds on it.
- Budgets of different models count the same tokens; a later version may
  weight them.
- Never run with a real specialist: Claude Code was not signed in when this
  was built. Proven with tests on the in-memory runtime and in the demo.

## Alternatives considered

- **Count a task in review as done.** Faster, but work would build on what
  nobody checked.
- **Retry a failed turn.** A missing sign-in would fail every task in turn.
- **Refuse the manager's own start when a budget is used up.** A second
  question for what the press already answered.
- **Keep the switch per computer, unlogged, as integrations are (0027).**
  The other computers could then not see that it runs, nor avoid running
  it twice.
- **A timer that looks every few seconds.** Against "nobody polls", and
  every event that matters is known.
