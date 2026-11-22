# Chapter 12: A night on another computer

*21 and 22 November 2026*

## What happened

### A new computer, and no way in

On the evening of 21 November the founder moved the project from the
computer at work to a computer in the lab: thirty-two cores, a hundred and
twenty-five gigabytes of memory, Ubuntu. He wrote: "which features are
missing and we wanted to implement i will leave you to tomorrow morning 9
am to do them and you can be orchestrator of opus 5.5 agents." Then he
had to go, and said: ask me now.

The lab computer had no Rust, none of the development libraries Tauri
needs, and no `sudo`. Four questions were asked and answered in a minute:
all four groups of the roadmap; a branch and a draft pull request, not
the main branch; set the build up without an administrator; real runs of
Claude Code, sparingly.

Rust went into the home folder. The development packages were fetched
with `apt-get download`, unpacked into a folder, and their links and
`.pc` files turned to where the files really were. Twenty minutes after
the first message the four checks passed on the untouched main branch,
the desktop shell included.

### One agent for each feature

Each feature got one agent, a worktree of its own, a number for its
migration and its decision record given in advance, and one shared page
of rules: every layer or none, pictures looked at by the one who took
them, an honest report. The roadmap and this story were kept away from
the agents, so that they would not collide there.

The first five reported after twenty-six to forty-one minutes each: the
instructions of a specialist and twelve ready-made specialists, the
public demo, handoffs and messages on the team bus, changing a proposed
plan, tags and checklists. The first agent also reported what nobody had
checked: Claude Code on this computer was not signed in. No real
specialist could run that night, though the founder had allowed it.

### The limit, met from the inside

At about a quarter past eight, eight agents stopped in the same minute.
Each ended with the same words: "You've hit your 5-hour limit · resets
10:40pm". The founder came back at the reset and wrote that it had reset,
and later that the easy tasks could go to a smaller model.

Nothing was lost; every agent went on from where it stood. But the line
on the roadmap about the limits of the plan had just been shown from the
inside. The agent that built it was given the exact words of that
evening. A manager whose team runs through the night will meet the same
wall, and should read one plain line with the time it resets, not eight
failed turns.

### Merging

By midnight seventeen features and a user manual were on the branch.
Project memory, the coordinator with its budgets, permission requests,
skills and connectors, the import of earlier conversations, custom
fields, work-in-progress limits and templates, the charts that compare
the plan with what happened, the report of a whole project, grouping, the
notice of a newer version, the limit of the plan.

The merging was the orchestrator's work. Features that landed minutes
apart collided in the same few files: the list of logged things, the two
halves of the wire contract, the lists of imports. An agent was sent
back to rebase when the collision needed its knowledge, and the
orchestrator joined the sides when both had only added a line.

Several agents had run the checks only on their last commit. So every
commit was checked on its own before it was pushed. Four failed: two on
formatting, and two that did not build because they read a style sheet
that a later commit created. Each was folded into the commit that
completed it. The migrations, numbered before the order of merging was known,
stood in the list as 0020, 0018, 0019, 0022; they were renamed in the
order they run.

### The review

None of what a specialist can reach had been run. So one agent was asked
to read it as an attacker would.

A turn without a command permission held. The permission requests did
not. A specialist may write files in its folder, and almost every
command a manager would allow runs files from that folder: a test
script, a build file. One allowed `npm test` was any program, run as the
manager. A prefix constrained nothing after it. A pair of quotes passed
every check of the program's name. A connector was started in the folder
the specialist writes. With a standing permission and the coordinator
answering messages between teammates, it could have happened with nobody
looking. And the words on the card had not said so.

Three agents were set to it the same night, and the limit of the plan
stopped two of them a second time, at midnight, until twenty to four.
By the morning it was in: no prefixes, no standing permission for a
command, none in a turn that only answers a teammate, none next to a
connector's secret, the sandbox of Claude Code switched on and failing
closed, connectors started by Teamwright itself in an empty folder with
only the variables they name, and two sentences on the card that say
what allowing a command means. The sandbox has never been seen working.
The roadmap says so, and says that allowing a command is not ready to be
relied on until a signed-in run has shown it.

### The second look, and the program at last

The fixes were read again by an agent that had not written them. It
found one more way in, and proved it with a small program of its own: a
permission that the manager had taken back could be brought back through
the sync folder and moved to another specialist, because the check
looked only at the part of the record that carried the answer. It found
a worse one that nobody had asked it to look for: a connector that
arrived through the sync folder was started as it came.

So there was a second round before the morning. A permission counts only
if every state of it was written on this computer. A connector is
started only as the manager confirmed it on this computer. A command
runs only in a turn the manager starts by hand, and with a cleared
environment. And because the sandbox had still never been seen working,
the whole of it was put behind a switch that is off: a specialist cannot
even ask for a command until the manager turns that on.

Meanwhile another agent did what had not been done once all night. It
built the program and started it, on a display of its own, and drove the
real window through seventy-six checks: a project from each template, a
task with a tag and a checklist and a field of each kind, the views, the
memory, the coordinator switched on against a stand-in that fails every
turn. It started the task once, said why it failed, and waited. A
database made by the program of the main branch opened and went from
format 17 to 29. No command and no part of the wire contract was wrong.
One sentence on the first screen was: it still said that a team that
works together was to come.

### The first real session

The founder came back in the morning, looked, and signed in to Claude
Code. Thirteen tests that had waited all night ran for the first time.

Ten passed at once or after a wrong expectation of the test was put
right: the instructions reached a session, a handoff came through the
bus, the memory was searched, a session got its own skill and connector
and nothing of the founder's, the launcher handed a connector one
variable and no other.

Two could not be answered. The sandbox for commands, which two reviews
and a switch had been built around, did not start: Ubuntu as installed
refuses one user namespace inside another. Nothing ran outside it. The
tests of it had passed in the first run, because they checked that
nothing leaked and nothing had run at all; they now fail when the
command did not run.

One failed, and it was not about anything built that night. A session
wrote a file into a folder it was given to read only, and Claude Code
answered "File created successfully". The rule that forbids it had
begun with one slash since the first week of the project, and one slash
means a path inside the working folder. On Windows, where a folder
begins with a letter, it had always held, and Windows was where it had
been tried. A reviewer had written the night before: this predates
tonight, a real run once passed with it, verify before changing. It was
in the released version. It took two slashes.

## Why it went this way

**A draft, not the main branch.** The lab computer cannot build for
Windows. A draft pull request let the build server check Linux and
Windows after every push while the main branch stayed as it was.

**In progress, not done.** Project memory, the coordinator, skills and
connectors and permission requests are marked in progress on the roadmap,
though each is built through every layer. None has run with a real
specialist, and a mark that says done would say more than is known.

**The review before the morning.** A feature that opens what an earlier
decision closed is not finished when its tests pass. Its tests were
written by the one who built it.

## Decisions

- [0029](../decisions/0029-handoffs-and-messages-between-specialists.md)
  to [0043](../decisions/0043-a-notice-of-newer-versions.md): one for
  each feature of the night, and dated notes of 21 November in 0014,
  0016, 0021, 0037, 0038, 0039 and 0040.
- The page [Building on Linux without administrator rights](../development/linux-without-admin.md).

## Storyboard beats

1. A lab at night. One screen. A message: "I have to go, ask me
   questions now." Four questions, four answers, a door.
2. Benches in a row, each with its own lamp and its own copy of the same
   drawing. Five lamps, then eight.
3. A quarter past eight: every lamp goes out in the same minute. On each
   bench the same slip of paper with a time on it.
4. The lamps come back. One bench begins to draw the slip of paper
   itself.
5. Near midnight a visitor walks the benches, reading as a thief would,
   and leaves a list on the last one.
6. Before dawn the visitor comes back and finds a window left open that
   the first list had closed a door beside. A switch is fitted on the
   wall, and left off.
7. First light. For the first time that night somebody opens the real
   door of the workshop and walks through every room.
8. Morning. The founder turns a key. The oldest lock in the building,
   the one nobody had looked at all night, turns out to have been
   fitted for another country's doors.
