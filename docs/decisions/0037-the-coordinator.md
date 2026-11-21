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

**Storage** (migration 24): `coordinators`, `budgets` and
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

## Note of 21 November 2026: the limit of the Claude plan

The limit of the Claude plan ([decision 0040](0040-the-limit-of-the-plan.md))
now reaches the rule. `next_steps` is told whether the plan lets work
start (`may_start_work`), and:

- **While the limit is reached, nothing starts**: no task, no turn to go
  on, no turn to answer a teammate. Whatever could start waits, with the
  reason "the limit of your Claude plan is reached", and takes no place, so
  nothing claims to wait for a busy specialist while nobody works. The record says it once for each
  time the limit is reached, not at every nudge: "Started nothing: the
  limit of your Claude plan is reached. It resets at 22:40.", or, when the
  reset is not known, that work goes on once a turn started by hand goes
  through.
- **A turn the limit ended is not a failed turn.** It did no work, so the
  task is not left to the manager as a failure. Once the limit has lifted,
  the coordinator goes on with it in the same conversation, as the
  manager's "Try again" does: a reply, through the same use case, with its
  own sentence ("The limit of the Claude plan stopped your work, and it has
  reset. Please go on where you stopped."). This is the one exception to
  "a task with a conversation is never started again", and it is no
  start: the conversation goes on. It takes a place and keeps to the
  budgets like any turn, and it comes before new work, whatever the order
  of the board: the work is under way and its specialist knows it. An
  ordinary failure still waits for the manager.
- **No loop.** If the coordinator's own turn to go on ends at the limit
  too, it goes on again only once the coordinator has written that the
  limit held work back after that turn started, that is after a limit that
  was known and has reset again. A limit that ends a turn but is not known
  to be reached (a reset given as already past) would otherwise go on at
  once, for ever; the task then waits for the manager with "it stopped at
  the limit of your Claude plan again as it went on; press Try again". The
  coordinator tells its own turns from the manager's by their sentence, so
  a "Try again" pressed before the reset is still gone on with after it.
- **When the limit lifts, the coordinator acts.** The shell nudges it in
  every project when the limit lifts: at the wake at the known reset, when
  a turn that went through lifts it, and when the program says the plan
  allows work again. It still never polls. The end of a turn now tells
  what it learned about the plan before it wakes the coordinator, so that
  the coordinator never starts more work in the moment between.
- Budgets are not charged in any new way: a turn the limit ended counts
  the tokens it used, as before.

The two new reasons travel as `plan_limit` and `plan_limit_again`. Tested on
the rule and on the in-memory runtime, and followed by the demo; never run
with a real specialist.
