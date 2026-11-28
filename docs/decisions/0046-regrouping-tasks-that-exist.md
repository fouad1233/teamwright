# 0046. Regroup tasks that exist: by hand, and in a proposal of the Advisor

- Status: accepted
- Date: 2026-11-28

## Context

The founder runs Teamwright 0.3.1 on a Mac. His project had 26 tasks. He
asked the Advisor to regroup them into 8 new module tasks, with the tasks
that exist as their subtasks. The Advisor answered that it could not, and
told him to do it by hand on the board.

Both answers were half true. Version 0.3.0 let a task of a proposal be a
part of another task of the same proposal only
([decision 0014](0014-the-advisor-plans-through-two-tools.md), note of 27
November 2026); a proposal could not name a task that exists, and the
instructions said so ("a task that exists cannot"). And nothing at all,
not even the board, could move a task that exists under another task or
out from under one: the parent of a task was set only when it was created.
So the advice to do it by hand was wrong.

The Advisor also said that its tool "has no field for subtasks", which was
false in 0.3.1: `propose_plan` had `part_of`. Whether a conversation that
began in an older version gets the new tool was looked up in the code: it
does. The tools of a turn come from the team bus, which is started anew for
every turn by the program that is installed, with `--mcp-config`, also when
the session is resumed; the instructions go with every turn too
(`--append-system-prompt`). What stays from the older version is what the
conversation itself holds: the earlier messages, among them what the
Advisor was told after an approval ("propose only what is to be added …
no task that exists"). The likely reading is that the Advisor took "never a
task that exists" for "no subtasks". It was not a defect of delivery; see
the decision about the instructions below.

## Decision

**One rule of regrouping, in the domain** (`parts.rs`,
`check_regrouping`). It takes the outline of a project (what each task is
a part of and what it waits for), new tasks (which may be parts of tasks
that exist or of each other) and moves of tasks that exist (each with its
own parts), and answers with the first problem:

- a task that is not there, or moved twice;
- a task that would be a part of itself or of one of its own parts, at any
  depth;
- a part that would wait for the task it is a part of, directly or through
  other tasks. The coordinator starts a task with parts only once its parts
  are finished, so that task waits for its parts; a circle with a part in
  it could never start.

Only what the regrouping changes is looked at: each new link from a whole
to its part is added one at a time and may not close a circle. A circle the
board had before (a merge leaves one until the next take-in repairs it) is
not held against the regrouping. The rule works on any kind of key, as the
dependency graph does: identifiers on the board, the places of the
references in the team bus. It is used by the board, by every change of a
proposal, by approving, and by the team bus before it takes the call.

**By hand.** In the details of a task, a row "Part of" shows the task it is
a part of and opens the project's task picker, which leaves out the task's
own parts; a cross makes it a task of its own. The move is one write,
logged, the task updated in place. Its parts go with it. A subtask is in
the area and the sprint of the task it is a part of, so the moved task and
its parts take the area and the sprint of their new whole; a task made one
of its own keeps the ones it has. The history of the task says it
(`made_part_of`), and the log of the project counts it as a change of the
plan.

The same rule closed a gap of the board: a subtask could be made to wait
for its own task. That is now refused as a circle, as it was refused in a
proposal.

**In a proposal.** A new task may be a part of a task that exists, and a
proposal may carry moves: each names a task that exists and what it goes
under, a task of the proposal by its place, another task that exists, or
nothing (a task of its own). A proposal may hold only moves and new tasks;
it is empty only with no members, no tasks and no moves.

- The model names tasks that exist by the references of its instructions
  (`IOT-3`, whatever the capitals). The bus is told the board as arguments
  (`--task`, `--part-of`, `--waits-for`) and refuses what breaks the rule
  in words the model can act on. When the call is recorded, a reference is
  turned into the identifier of the task, so that a new number or a new key
  after a merge does not change what the proposal means.
- The manager can send a move elsewhere or take it out (`move`,
  `removeMove`), and make a new task a part of a task that exists
  (`makePartOf` with `partOfTask`). Every change is checked against the
  board as it is.
- Approving checks again against the board as it is then, because a task
  may have been deleted or moved since. If anything no longer holds, all of
  the approval is refused with the reason, and nothing is created or moved.
  Otherwise the new tasks are created, then the moves are made, in one
  write.
- **Where the moved tasks go.** A part goes into the area and the sprint of
  the top of its chain, as on the board. A new task of its own goes into
  the first area and the backlog, as before, unless the tasks moved under
  it are all in one area and one sprint: it then goes there, so that a
  regrouping does not take them out of the sprint the manager put them in.
- **The instructions** no longer say that a task that exists cannot have
  parts. They say that the Advisor regroups only when the manager asks for
  it, never on its own initiative, and how; that it never proposes a task
  again in order to regroup it, and never tells the manager to regroup by
  hand. They also say that what they and the tool offer may have grown
  since earlier messages of the conversation, and that they hold over those
  messages. What the Advisor is told after an approval lists the moves, and
  says to move a task rather than propose it again.

**Storage.** Migration 0032: a column for the task of the project a new
task of a proposal is a part of, and a table of the moves. The proposal is
still logged whole; a state logged before has neither and reads as a plan
that moves nothing.

**Merging.** The parent of a task is a field of its own that merges by the
later change, as before. Two kinds of circle can come from two computers:

- A circle of parts (one computer puts A under B, the other B under A). It
  was repaired already: of the links of the circle, the part set last gives
  way and becomes a task of its own.
- A circle that shows only once a task waits for its parts: one computer
  makes A a part of B while the other lets A wait for B. Taking in now
  repairs it the same way: of the links of the circle, waits and parts
  alike, the one set last gives way. The area of a part was already put
  right after a merge (it takes the area of the top of its chain); the
  sprint is not, see below.

## Consequences

- The founder's request arrives as one proposal: the modules as new tasks,
  the 26 tasks moved under them, which he reviews, changes and approves.
  The real Advisor on the cheapest model did it for a project of four tasks
  at the first attempt.
- Moving a task can change its sprint, and a regrouping into new modules
  can take tasks out of a sprint when the tasks of one module were in
  different sprints. That is the rule of subtasks; the sentence before
  Approve says so.
- After a merge, a subtask may stand in another sprint than its task, if
  one computer moved it while the other planned the task. Nothing repairs
  that yet; the same could happen before, with a subtask made on one
  computer while its task was planned on the other.
- The command line of an Advisor's turn grows with the board, by a few
  words for each task. The instructions already listed every task.

## Alternatives considered

- **Let the Advisor propose the tasks that exist again, as new ones, and
  delete the old ones.** Their history, their conversations and their
  numbers would be lost. Rejected.
- **Moves as ordinary changes the Advisor makes through a tool of its
  own.** Nothing would wait for the manager's approval, against decision
  0014.
- **Keep a reference such as `IOT-3` in the proposal, resolved only on
  approval.** A merge can renumber a task or change the key of a project;
  the proposal would then mean another task.
- **Refuse a regrouping whose tasks are in different sprints.** The
  manager would have to plan first and regroup after, which is what he
  asked the Advisor to spare him. The sprint follows the rule of subtasks
  instead, and the manager sees it before approving.
- **Put a new module where most of its moved tasks are.** It guesses, and a
  guess about a sprint is hard to see in a review. Only a place they all
  share is taken.
