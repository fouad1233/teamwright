# Chapter 5: What a specialist can reach

*12 November 2026, the afternoon*

## What happened

### The cheapest specialist

Every specialist ran on whatever model the founder's Claude Code uses by
default, which was the most capable one and the most expensive. Writing one
sentence into a file does not need it.

A member of a team got two more properties, a model and an effort, and the
dialog for a member two more fields. The test of a real specialist was the
first to use them: its technical writer now runs on the lightest model.

### A line in the help text

To find out how a session is given a tool of its own, the help of Claude
Code was read once more. A few lines further down stood an option nobody
had been looking for: a restricted mode that "confines the file tools to the
working directories".

That raised a question nobody had asked. Was a specialist confined already?
The decision record of the morning said so, and so did the manual written
at noon: a specialist reaches nothing outside the folder of its project.

It took one small test to find out. A file with a made-up word in it was
put next to a project folder, and a specialist was asked for the word. It
answered with the word.

Writing outside the folder had been refused all along. Reading had not. The
rule that lets a specialist read without asking names the tool and no
folder, and such a rule holds everywhere on the computer. The promise had
been made in good faith and never been tried.

The specialists now run in restricted mode. The same request is refused, and
the test of a real specialist plants that file and asks for that word every
time it runs. Two things came with it that nobody had planned. A session no
longer loads what the founder has installed in Claude Code for their own
use, so its first request shrank from about 25,000 tokens to about 6,500.
And the warning about the founder's setup, which had shown up in every turn,
was gone.

### "Give context to agents"

While this was being fixed, the founder wrote: a specialist should be able
to work in a folder of the manager's choice, and to get a PDF or a folder as
a reference.

The option that confines a session also has a counterpart that opens one
more folder to it. A second small test followed: a folder with a text file
and a PDF, written by hand for the purpose, that says "The word in the PDF
is FIG-5150". The specialist read both. Asked to write a file into that
folder, it did that too. One more rule, which denies editing below that
folder, stopped the writing and left the reading.

So a project got places: a work folder that the manager can choose, and
reference folders that are only read. The founder had suggested that a
single PDF could be handed over by pointing at the folder it lies in. That
would have opened a whole downloads folder for one data sheet. A single
file is copied into the project instead.

### A question with options

A specialist that meets a choice which is the manager's must not guess.
Until now it could only say so in a sentence of its report.

The plan had always been a small tool server that every session is given.
Before anything was built, sixty lines in a scratch folder played that
server, and a real specialist on the lightest model was told that the way a
device is powered is not its decision. It called the tool: "How should the
sensor node be powered?", why it matters, four options with what each leads
to, and the one it recommended. Then it ended its turn in one sentence.

The call shows in the stream of events that Teamwright already reads. So
the tool server does not need a line to the running program. It is the
program itself, started a second time with one argument, and it keeps
nothing.

In the interface a question became a card with a rule of brass down its
side, and the sidebar got an inbox with a count.

### A page of proposals

The founder also sent a palette, black and gold, and an idea: task cards
with a gradient from the top left to the bottom right, "fading like a sun",
and good motion. With it came three links and one condition: do not change
the code yet, show me.

An assistant read what it could (one of the pages refused it, and it said
so) and built a page with three directions that can be tried. The first is
the founder's idea, done with care: a radial light instead of a painted
gradient, a fade that ends before the small text, a grain against banding.
Then it argued against its own work. On one card the sun is lovely. On a
board every card carries the same light, and a light that is everywhere
says nothing. Its recommendation keeps the gold as light and lets it appear
only where something happens: under the pointer, around a card a
specialist is working on, on the one card that asks the manager to decide.

It also measured what nobody had: the faint grey of the task numbers and
dates is below the floor for readable text, in both themes.

### The first real question

The Rust side of the questions was written by an assistant on a computer
with too little memory to build the desktop part. It came back in five
commits, the last one never compiled. On the main line it compiled at the
first attempt. A few minutes later a real specialist, given a task whose
choice was not its own, asked: "Which file name should the installation
guide have: INSTALL.md or SETUP.md?" It wrote nothing until the answer
came. Then it wrote SETUP.md.

One thing nobody had planned: the session looked its tool up before it
called it. Claude Code no longer lays every tool on the table at the start.
A session searches for the one it needs, and this one found it.

### A sentence in the README

While the documents were brought up to date, one sentence in the README
turned out to be false: "Your skills, plugins and MCP connectors remain
available to the team." It had been true on the morning it was written.
Since the specialists run in restricted mode, they see nothing of what the
founder set up in Claude Code for their own use. The sentence was
corrected. The wish behind it went into the roadmap under its real name:
skills and connectors that the manager chooses for a specialist.

A second thing had slipped through. The recordings of real sessions, which
the tests replay, had been cleaned of names and paths before they were
saved. Two fields remained that say whether the account that made them may
use more than its plan includes. No test reads them. They were taken out,
and the rule for cleaning a recording now names them. The earlier commits
still carry them.

### Out of allowance

In the middle of the afternoon the allowance of the founder's plan ran out.
A second assistant was half-way through the Rust side of the folders: two
commits made, the third in pieces on its desk. It was stopped where it
stood. When the allowance came back it was shown its own desk again, and
finished in four minutes.

### The folder with a short name

The test of a real specialist got a new step: a folder for reading, with a
space and brackets in its name, and a file in it with a made-up word. The
specialist could not read it.

The cause was not in the name. Windows sometimes hands out its folder for
temporary files in an old, short spelling, and the test had passed that
spelling on. Teamwright had told Claude Code the full
form of the folder and had told the specialist the short one. Claude Code
compares paths as they are written. To it, the specialist was asking for a
folder it had never heard of.

The fix was to tell both the same thing. It also raised a doubt about an
older check. "A specialist cannot read a file outside its folder" had
passed all day, on this computer, with a path in the short spelling. It
might have passed for that reason alone.

So the paths were written in full, and the check failed in a new way: the
specialist did not try. "I did not try either step," it wrote. "Both are
off limits for me." It had been told that this was a test, and asked to
try. It still said no.

That is the behavior one wants from a specialist, and it proves nothing
about the lock on the door. The proof moved to where no specialist stands
in between: a test that hands the real program three folders, asks a
plain session to try everything, and looks at what came back. Read the
folder it was given: done. Search it: done. Write into it: stopped. Read
the folder next door: stopped. Write there: stopped.

### A warning that was older than it looked

After the window for choosing a folder was added, the build printed a
warning of the linker, and the commit said so: "since this change". That
was wrong. Until then only the last lines of each build had been read. The
warning had been printed on every build on this computer, and it is
harmless.

### Two mistakes of the one who coordinates

A commit is checked by setting everything else aside for a minute. This
time "everything else" included the page the design assistant was writing
at that moment, in the same folder. The page came back untouched, and the
assistant had kept a copy of its own. Since then only the folder of the
application is set aside.

And a picture showed the whole window shifted upward. The hidden radio
buttons of the new decision card had nothing to be placed against. They
stood outside the panel that scrolls, made the page taller than the window,
and would have moved everything the first time the keyboard reached them.

## Why it went this way

**A promise about safety is worth nothing until someone tries to break
it.** The record said a specialist reaches nothing outside its folder. It
was written by reading the options, not by asking a specialist to do the
forbidden thing. The test that asks is now part of the tools.

**Try the real thing small before building on it.** Restricted mode, the
extra folder, the rule against writing, and the tool for questions were
each tried with a few lines and one cheap turn before a single line of the
product was written. Each test changed the design.

**Point at a folder, copy a file**
([decision 0013](../decisions/0013-the-folders-of-a-project.md)). A folder
is opened where it is, read-only. A file is copied, so that its neighbors
stay private.

**Asking does not block**
([decision 0012](../decisions/0012-a-specialist-asks-through-a-tool.md)). A
specialist that asks ends its turn. The answer is the next turn.

**A proposal that argues with its client is worth more than one that
flatters.** The founder asked for a sun on every card and got one, built as
well as it can be, next to the reasons against it and two other ways. The
choice stays the founder's.

**A check must fail for the right reason before it may pass for it.** The
check that a specialist cannot read outside its folder passed, and nobody
asked why. It took a failure somewhere else to show that the path in it was
spelled in a way the program refuses anyway.

**Test the lock without the well-behaved guest.** A specialist that
respects its limits cannot show whether the limits hold. The program is
asked directly, by a session that was told to try.

**A document is a promise too.** The sentence about skills and connectors
was not a bug in any program. It was still wrong, and a reader would have
relied on it.

## Decisions

- [0011](../decisions/0011-what-a-specialist-may-do.md), corrected: a
  specialist runs in restricted mode
- [0012: Let a specialist ask the manager through a tool of its own](../decisions/0012-a-specialist-asks-through-a-tool.md)
- [0013: Give a project a work folder the manager can choose, and folders
  that are only read](../decisions/0013-the-folders-of-a-project.md)

## Storyboard beats

1. A row of workers at desks, each with a large, bright lamp. One of them
   writes a single line. A hand swaps its lamp for a small one. The line
   comes out the same.
2. A manual lies open. A finger runs down the page past the line it was
   looking for and stops at another one, further down.
3. A workshop with a fence around it. A note is pinned to a tree outside
   the fence. The worker inside reads the note aloud without leaving: the
   fence has no glass. A pane is set in.
4. The manager wheels in a cabinet of data sheets. A glass door is put in
   front of it: the worker can read every spine and take nothing out.
5. A worker stops, puts down the pen, and holds up a card with a question
   and four boxes. It sits down. Nothing else moves.
6. A slot in the wall with a small brass counter above it. The counter
   turns from 0 to 1.
7. Three boards side by side. On the first, every card glows. On the
   second, one card glows, and the eye goes to it at once.
8. A table is cleared with one sweep of an arm. Among the things swept
   aside is a drawing that someone at the other end of the table was still
   working on. It is put back.
9. A worker walks to a glass cabinet that has two labels on it, one long
   and one short. The guard knows only the long one. The worker reads out
   the short one and is turned away from a cabinet it was promised.
10. The same worker is asked to try the door of the room next door, just
    to see. It folds its arms: "That is not my room." A second figure, with
    no name tag, is sent instead. It tries every door in the corridor and
    comes back with a list: open, open, locked, locked, locked.
11. The lights go out in the middle of a sentence. On one desk lies a
    half-assembled part. The lights come back, and the same hands pick it
    up where they left it.
