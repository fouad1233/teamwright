# Chapter 7: Many hands

*12 November 2026, the night*

## What happened

### One who hands out the work

The lamp of the last chapter was built by an assistant in a checkout of its
own. Then the founder wrote one more sentence: to save tokens, the session
should give pieces of the work to several assistants and "be the
orchestrator".

From then on the work had a different shape. One session decided what was
to be built, wrote a brief for each piece, read what came back, brought it
onto the main line commit by commit, built the program and ran it. The
assistants each had a checkout, a port for their own preview and a copy of
the tool that takes pictures. Only one of them was allowed to compile Rust
at a time, because the computer has sixteen gigabytes of memory and a
build needs a good part of them.

Three pieces went out that night: the screens of sprints, the Rust side of
sprints, and the office.

### The Advisor hands in its plan twice

Before that, one mark on the roadmap was still open. A later round with
the Advisor, in which the manager asks for more after a plan was approved,
was built and had never been run with the real program.

The test was made longer. After the approval it said: one more thing, the
device should also run from a battery for a day; add what that needs; the
team stays as it is. All twenty-nine checks passed. The output underneath
them did not look right. The Advisor had proposed the same three members
again, and fourteen tasks, most of which repeated the thirteen that were
already on the board. Approving that added nobody twice, because a rule
stops it. It doubled the tasks, because no rule can tell a task from the
same task in other words.

The cause was not in the model. The session of the Advisor had never seen
the approval happen. The manager presses a button in a window, the program
adds the members and the tasks, and nothing of that reaches the session.
To the Advisor its proposal was still open, and "add this to the plan"
meant "hand in the plan again with this in it". It did what it was asked.

Two things changed. A message of the manager's that follows an approval
now reaches the Advisor with the news that its proposal was approved, and
with what a proposal may hold from then on. And the tool with which it
proposes refuses a member who is on the team already, and says what to
leave out, so that the Advisor tries again in the same turn.

The second run asked for the battery again. The Advisor proposed no member
and seven tasks: a power budget, the manager's approval of the battery,
the changes to the schematic, the layout, the firmware and the case, and a
test of one day. That turn wrote 1,804 tokens. The one before the fix had
written 3,427.

### A specification two can read

Sprints had been studied before they were built: two boards the founder
had pointed to, one a primer on Scrum and one a worked example with a
backlog ordered by feature, and the way Jira lets a team plan. The
decision was short. A project has a backlog. A sprint is planned, runs,
and is completed for good; one runs at a time. A task has story points or
none. What a sprint did not finish moves on.

The way of working that had served all day was used once more, and this
time it carried two assistants at once. First one assistant wrote the
contract and the demo, with forty-six cases that say what a sprint does
and what it refuses, word for word. Then it went on to the screens, and
another wrote the Rust side against the same cases. They never spoke to
each other. They did not need to.

### An office without a clock

The founder had asked for a view like a game: the team as characters at
work, with what each is doing, how long it will still take and what it has
used. The assistant that got this piece wrote the decision first. The
office is a workshop drawn in plan, as an architect draws a room: walls of
two lines, a door, a bench for every member, the manager and the Advisor
at the front. A lamp hangs over every bench. It is lit only where work is
going on, which is what the lamp of the last chapter had promised.

One thing that was asked for is not there. The office shows no time left,
because Teamwright does not know how long a task will still take. It shows
how long the work has gone on, counted every second, and the size the
manager gave the task. A sentence in the corner of the drawing says so.

### The limit

In the middle of bringing the office onto the main line, the founder's
plan reached its limit. A short allowance was left. The session
stopped the assistant that was writing Rust, finished the piece it had in
its hands, pushed it, and wrote down what was done and what was not.

When the limit had reset, the founder wrote: "Yes, resume sprints." The
assistant was shown its desk again. It had been closer to the end than
anyone knew, and reported four minutes later: six commits, every case of
the demo ported under the same name.

### Bringing it together

Its fourth commit did not pass the linter on the main line. It built a
service in the shell that nothing used until the fifth commit, which adds
the commands. The assistant had been told never to compile the shell, so
it could not have seen that. The line was moved to the commit in which it
is used.

The two interface pieces had both added a tab in the same two files. Both
tabs were kept: Backlog second, Office before the Advisor. In the manual
the two new sections had landed in the same place and were put in order.

Then the program was built and used. The test of the built program plans
a sprint, fills it, refuses to start it empty, starts it, refuses a second
beside it, completes it, and finds everything again after a restart. The
tool that takes the pictures plans the sample project in three sprints,
one over, one running, one to come.

For the office a real specialist was let to work once more. The picture
had to be taken in the seven seconds its turn lasts. It shows one bench
under a lit lamp: at work, "Write a greeting", starting, for one second.
The run failed right after. The new step had gone back to the board and
opened the first card to go on, and the first card was no longer the task:
it had moved into work. The step was told to look for the task by its
title, and the next run passed.

## Why it went this way

**The one who hands out the work keeps three things.** Deciding what is
built, reading what comes back, and running the real program. An assistant
reports what it believes, from its own checkout. Twice that night what
came back was right there and did not hold on the main line: a test of the
office did not know the fields that sprints had added to a task, and a
commit of the Rust side did not pass a linter its author had been told not
to run.

**Write the specification where both sides can run it.** The demo and its
tests are code. Whoever builds on them finds out at once when they
misread them. That is why two assistants could build two halves of one
feature in the same hours.

**A session knows what it is told.** The Advisor repeated its plan because
nobody had told it that the plan was approved. What happens in the window
does not happen in the conversation unless the program says so.

**Say it, and also refuse it.** Words to a model work most of the time. A
tool that refuses, and says what to change, works the rest of the time.
Where the program cannot tell, as with a task that repeats another in
other words, the roadmap says that it rests on words.

**Do not show what is not known.** A time left would have looked good in
the office. It would have been a guess.

**Stop in order.** When the limit came, the piece in hand was finished and
the rest was written down. Work that was stopped half-way was left where
it could be picked up.

## Decisions

- [0015: Let light fall only where something happens or waits for the manager](../decisions/0015-light-where-something-happens.md)
- [0016: Plan the work in sprints, with a backlog and story points](../decisions/0016-sprints-and-a-backlog.md)
- [0017: Keep computers in step through files of operations in a folder the user owns](../decisions/0017-sync-through-a-folder.md),
  proposed and waiting for the founder's answer
- [0018: Show the team at work as a workshop drawn in plan](../decisions/0018-the-office-as-a-plan-of-a-workshop.md)

## Storyboard beats

1. One figure at a table with three envelopes. It writes a page into each
   and slides them under three doors. Behind each door a lamp goes on.
2. The figure with the notebook from the last chapter is asked for one
   more thing. It hands over the whole wall of cards a second time, every
   card twice. It looks pleased.
3. A stamp, "Approved", comes down in one room. In the next room the
   figure with the notebook hears nothing: the wall between them has no
   door. A hand cuts a slot in the wall and passes a note through.
4. Asked again, the figure pins seven new cards next to the old ones, and
   no new chair.
5. Two people build two halves of a bridge from both banks without
   looking up. Between them hangs a sheet with forty-six lines. The halves
   meet.
6. A room drawn from above, in thin lines. Benches. Over one of them a
   lamp is lit, and a counter under it goes from one second to two. On the
   wall, where a clock would hang, a note: "not known".
7. A gauge reaches its end and every lamp but one goes out. Under the last
   one, a hand finishes a line, closes the book, and leaves a list on the
   table. Later the lamps come back on, and the list is the first thing
   that is read.
