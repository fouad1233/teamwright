# Chapter 3: A team with faces

*12 November 2026, the hours after midnight*

## What happened

### Pictures of the real thing

The README described the product in words, next to a drawing. A reader could
not see what existed.

A tool was written that starts the built application on an empty data folder,
creates a sample project through the same commands the interface uses, and
takes pictures of it: the first screen, the board, a task, the list, each in
the dark and in the light theme, all at one size. Nobody edits the pictures.
When the interface changes, the tool takes them again.

Looking at the first pictures at full size found two things that no test
looks for. The scrollbars had arrows and a track, like a part of some other
program. And in the list, the name of an assignee sat about two pixels
higher than the title and the date next to it. Both were fixed before the
pictures went into the README.

### A check that had been red

While these changes were committed, the format check failed, for a file that
had not been touched: the README of the desktop application. It had failed
since the commit that added that README. Nine more commits had gone in after
it without that check being run. Most of them changed only documents, and
for the others only the Rust checks had been run.

The project's own rule says that every commit passes the checks. It had been
broken by the one who wrote it down. The cause was small (the formatter pads
tables, and the documents are written with compact ones), and the fix was to
leave Markdown to the people who write it. The lesson was not small: the
checks run before every commit, also when "only a document" changed.

### The team

On the first day the founder had asked for something plain: to open a task
and give it to a certain agent, "like the software designer or hardware
designer", and one day to a real person. The model had an identifier for a
team member and nothing behind it. A task could be given to the manager or
to nobody.

The team was built from the inside out, in small commits:

- **The model.** A member has a name, a kind (an AI agent or a person), a
  sentence about what it is on the team for, and a color.
- **The use cases.** Put a member on the team of a project, rename it,
  change what it is there for, take it off. No two members of a team share a
  name. A newcomer gets a color nobody on the team has. A member who leaves
  gives its tasks to nobody. A task can only be given to a member of the
  team of its own project.
- **The storage.** A second migration of the database, tested by bringing a
  database of the first version up to date with its data kept.
- **The interface.** A third view, *Team*, lists the manager and every
  member with a tally of its tasks. The assignee menu lists the team. On
  cards and rows a member appears as two initials in its own color: an AI
  specialist as a block, like the parts of the logo, a person as a circle.
  The manager stays brass.

The sample project got the team the founder had described on the first day:
a hardware engineer, a firmware engineer and a mechanical engineer.

None of them can work yet. The dialog that adds an AI specialist says so in
its second sentence, and the Team view repeats it under the list.

When it was done, the Rust crates had 189 tests where they had had 150, the
interface 88 where it had had 67, and the smoke test of the real application
23 checks where it had had 15.

### The crash

In the middle of the interface work the computer stopped.

When it came back, the history was whole: 64 commits, every object in
place. The interface work that had not been committed yet, twelve new files
and nine changed ones, was on the disk as it had been left. Every check passed. The
work went on from the line where it had stopped.

### The test that was right about the wrong thing

The smoke test then reported that the board did not show who a task was
given to. The pictures, taken minutes earlier from the same program, showed
that it did.

The test was not wrong and neither was the program. The interface remembers
which view was open. The web view keeps that memory in a folder of its own,
and the temporary data folder of the tools did not move it. The screenshot
tool had ended on the Team view and left that behind. The smoke test started
the application, which opened on the Team view, where there is no board.

So the tools had been sharing a small piece of state with each other, and
with the installed application, since they were written. Each run now gets a
web view folder of its own inside its temporary folder, and starts as a new
installation.

## Why it went this way

**Pictures taken by a tool do not go stale**, and taking them is a check of
its own: it is the only moment at which somebody looks at the whole product
at full size. Two of this session's fixes came from looking.

**The team came before the agents** because it could. It gives the manager
what was asked for on the first day, and it gives the next milestone a
place to put a working agent: a member that already has a name, a purpose
and tasks.

**Agents and people are one kind of member.** A task is given to either in
the same way, so every rule and every view exists once
([decision 0009](../decisions/0009-a-team-per-project.md)).

**Small commits paid for themselves in one moment.** A crash in the middle
of a session cost nothing but the time to check that nothing was lost.

## Decisions

- [0009: Give every project a team of its own, with agents and people as
  one kind of member](../decisions/0009-a-team-per-project.md)

## Storyboard beats

1. A camera on a tripod in front of the application. It takes a picture.
   The room turns from night to day. It takes the same picture again.
2. A magnifying glass over the picture: a column of names, one of them a
   hair higher than its row. A fingertip nudges it into line.
3. A row of small lamps, all green but one. A hand wipes the dust off the
   red one. It has been red for a while.
4. Three blocks, stamped HE, FE and ME, slide into a rack under a brass
   disc. On the board, every card gets a small stamped block of its own.
5. In the middle of a stroke the picture goes black. Nothing. Then it comes
   back: the ledger of commits is whole, and the unfinished page still lies
   on the desk with the pen beside it.
6. A test stands in a room and reports that the board is missing. Beside it
   someone holds up a photograph of the board. They find a note on the
   wall, left by the visitor before: "last seen at: Team". From then on
   every visitor gets a room of its own.
