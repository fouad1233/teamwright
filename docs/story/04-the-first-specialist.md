# Chapter 4: The first specialist goes to work

*12 November 2026, from the morning into the afternoon*

## What happened

### The manager looks, and asks why

The founder opened the program for the first time and looked at what was
there. Then at the list of what was not: no specialist that works, no
Advisor, no chat, no inbox for decisions, no timeline, no office, no
sprints, no sync, and no way to edit statuses, see the history of a task, or
rename or delete a project. The answer was short: "why you didn't add
them!!"

The roadmap was the honest answer. The better one was to start at the top of
the list and work down.

That morning the repository also went public. The founder signed in to
GitHub themself, and from then on every commit was pushed as it was made.

### What the manager saw and wanted changed

A photograph of a board in another program came next: a card, a small row
under it that says "2 subtasks", and the subtasks behind it. Teamwright had
shown subtasks as cards among the others. They now live under their task,
behind a fold that counts them and shows a block for each in the color of
its status. The same message asked for a Gantt chart and a calendar. Both
were built that morning: bars against days with the lines of what waits for
what, a bar that can be dragged to another date, and a month of weeks with
the tasks on their days.

One failure in those hours belonged to a tool and not to the program. A
test dragged the end of a bar and the bar did not move. The program was
right. The test had pressed the mouse button and then moved the mouse
without holding the button down.

The menu of a project followed (rename, put away, restore, delete), and the
editing of statuses. The tool that takes the pictures found a real fault
there: a dialog that needed Escape twice, because the focus had landed on a
button whose tooltip took the first one.

### "Can't you do it with the Claude app?"

The plan was to run the specialists on Claude Code, the command-line
program. The founder asked whether it could be the Claude desktop app
instead, "so also the user can see it from claude app if he want".

The documentation gave a clear answer. The desktop app cannot be driven by
another program. The command-line program can: it runs without a window and
reports every step. And there is a bridge between the two, because a
session of the command-line program can be opened in the desktop app. So
the specialists run on Claude Code, and a conversation gets a button, *Open
in Claude*.

Claude Code had to be installed on a computer where nothing may be
installed with administrator rights. Its installer failed at first for a
reason that had nothing to do with it: this computer is set up so that the
command prompt does not look for programs in the folder it stands in. The
founder signed in themself. Teamwright never sees how.

### One writes the model, another builds it

The founder offered help: other models could be used as assistants, "but
use them efficiently". The computer set the limit. It had less than two
gigabytes of memory free and had crashed once during a build, so only one
assistant worked at a time.

A way of working formed in those hours. The contract between the interface
and the backend was written first, in TypeScript, together with a demo
backend that really behaves: a simulated specialist that takes a task,
shows steps, reports, can be answered and stopped. The tests of that demo
are the specification. An assistant then built the same thing in Rust, in
a copy of the repository of its own, with the same tests by name.

Three assistants worked this way: on the adapter for Claude Code, on the
statuses and the archive, and on the work of the specialists, which came
back as 5,600 lines with 141 tests. The first one checked its work against
the real program and found two things no document says: the program
reports its steps only when asked to be verbose, and a working folder
given in the short form that Windows keeps for old programs makes every
write fail.

The last assistant could not compile the part that opens the window. That
part pulls in too much for the memory that was left. It said so. The part
compiled at the first attempt when it was put together.

What came back was read before it went in. One example in a guide used the
founder's real folder name and was changed. And the commits were put in an
order in which each one builds the whole program, which meant that two of
them had to become one.

### Two o'clock

The test of a real specialist is the only test in the project that costs
something: it uses the founder's own plan. It forms a team of one, a
technical writer, and gives it one task: write a file with a single
sentence that greets the manager.

The first attempt pressed *Start work* and nothing happened. The button was
waiting for the answer to a question, "is Claude Code there?", which takes
no time in the demo and several seconds in the real program. While it
waited it was switched off, and it did not say so. Now the question is
asked when a project opens, and the line next to the button says what it
is waiting for.

The second attempt took 28 seconds. The card of the task got its brass
mark. The panel said WORKING. Two lines appeared, one after the other, each
with a tick at its end: the specialist looked at the folder, then wrote
`greeting.md`. Then the report, and the card moved by itself to the lane
where work is checked. The file was on the disk.

The test went on. It answered the specialist ("add a second sentence"), and
the specialist changed its own file in the same session, in 15 seconds. It
asked a question that was not about a task and got an answer in 13. It
started a long piece of work and stopped it after 6. It closed the program
and opened it again, and the record was still there. Twenty-four checks.
The three finished turns wrote about 950 tokens between them.

### What only the real thing showed

The first real report began like this: `**What I did:**`. The asterisks
were on the screen. A model writes Markdown, and the simulated specialist
of the demo had written plain sentences, so nobody had seen it. The text
of a specialist is now set as it was meant, by a part that trusts nothing
in it: it cannot bring HTML, a link in it cannot be followed, and a
picture in it is not fetched.

Every turn also began with a warning of the program about the founder's
own setup. It is true and it is not about the work, so it now takes one
line until somebody asks for the rest.

And a specialist is a process of its own. When Teamwright was closed while
one worked, the process went on working in the dark. Closing the program
now stops the specialists and writes into their turns why.

### A picture that said too much

The tool took the pictures for the README again. One of them, the team, had
gained a line under the table: which Claude Code was found on the computer,
and how its user is signed in. On this computer that user was the founder,
and the line named the kind of plan. It was noticed before anything was
pushed.

A picture of a sample project should not depend on who takes it. The tools
now point the program at a stand-in for Claude Code, a few lines that say
"I am installed, and somebody is signed in" and can do nothing else. The
stand-in also gave the free test something it never had: a turn that
fails, and what the window says then. Writing that check showed that a
failed start said "Claude Code ended" twice in one sentence.

### The history that was already written

The founder had asked for the history of a task. On the first day a log
had been built that keeps the whole state of a task after every change, so
that two computers can one day be brought in line. That log already was
the history. Nothing had to be added to the database: the program reads
the versions of a task from the log and says what differs from one to the
next.

## Why it went this way

**The demo is the specification.** A contract that only exists as types
says what the data looks like. A demo that behaves says what the program
does, and its tests can be carried over word for word. The assistant that
builds the other half has something to be right against.

**The real run is the test that counts.** Every fault of this chapter that
mattered to a user was found by running the real program with a real
specialist: the button that waited in silence, the asterisks, the warning,
the process that outlived its window. None of them could show in the demo.

**A specialist may write files in its folder, and nothing else**
([decision 0011](../decisions/0011-what-a-specialist-may-do.md)). There is
nobody to ask for permission in a session without a window, so everything
that would need asking is refused, and shown.

**One process for each turn**
([decision 0010](../decisions/0010-one-process-for-each-turn.md)). It is
slower than a process that stays, and it cannot hang forever, be left
behind half alive, or hold a file open.

**Not everything was run.** *Open in Claude* would have opened a test
session in the founder's own Claude app, in the middle of their work. It
was left for the founder to press.

## Decisions

- [0010: Start one process for each turn of a specialist](../decisions/0010-one-process-for-each-turn.md)
- [0011: Let a specialist read and write in the folder of the project, and
  nothing else](../decisions/0011-what-a-specialist-may-do.md)

## Storyboard beats

1. The manager opens the door of the workshop for the first time. On the
   wall hangs a list of what is missing. A finger taps it, hard.
2. A photograph is pinned to the board: a card with a small drawer under
   it. The drawer of a real card slides open, and two smaller cards are
   inside.
3. Bars lie along a ruler of days. A thread runs from the end of one to the
   start of the next. A hand drags a bar, and the thread follows.
4. A glass door with a name on it, locked. Next to it a plain side door
   stands open. Somebody hangs a sign on the side door: an arrow that
   points to the glass one.
5. Two desks. On one, a model of a machine made of paper, with a checklist
   beside it. On the other, behind a partition, a builder makes the same
   machine in steel and passes the parts through a hatch. Only one lamp
   burns over the partitions: the meter on the wall is almost at its end.
6. A hand presses a button and nothing happens. Behind the panel a small
   figure stands with a telephone and waits for somebody to pick up. A
   sign lights up beside the button: "Looking".
7. A block stamped TW turns brass. Two lines write themselves, each ending
   in a tick. A sheet of paper slides into a folder on the desk. On the
   board, a card moves one lane to the right by itself.
8. The first report under glass. The words have asterisks around them. A
   lens is set in front of the glass, and the asterisks become bold type.
9. The lights of the workshop go out. One machine keeps running in the
   dark. A hand comes back, switches it off, and ties a tag to it: "was
   closed while working".
10. The camera on its tripod again. In a corner of the frame, on the desk,
    lies a membership card. A hand lays a blank card over it before the
    shutter clicks.
11. A ledger that has stood on the shelf since the first day. Somebody
    turns its pages backward. On every page is the same card, a little
    earlier each time.
