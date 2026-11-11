# Chapter 2 · The first build, and the interface that was thrown away

*11 November 2026*

## What happened

### From the inside out

The code was written from the inside out. First came the domain: what a
project, a workflow and a task are, and the rules they keep. It touches no
file and no database, so every rule could be stated as a test. Then came the
use cases, then SQLite storage, and only then the window.

Two choices made early will matter much later:

- A manager may name a status anything, so the program never looks at the
  name. Every status belongs to one of four categories, and the logic follows
  the category.
- Every write to the database also appends an entry to an operation log,
  stamped with the identity of the computer and a hybrid logical clock. Sync
  between computers is a milestone far down the road, but a log cannot be
  added afterwards to data that was written without one.

By the end of the day there were 150 Rust tests and 67 tests of the interface.

### The machine said no again

Getting the desktop shell to build without administrator rights took three
rounds. An old 32-bit compiler from years ago sat first on the computer's
search path and answered whenever a build tool was called by name. First the
import libraries failed, then the icon of the application, and then the link
step failed for a reason of its own: a library format that only mobile builds
need. Each failure was fixed by naming the right tool in full, and the whole
recipe was written down for the next person without administrator rights
([development](../development/windows-without-admin.md)).

### Whose name is on the commits

The first thirty commits carried the name of the computer account. The
founder asked that every commit carry the founder's GitHub identity instead.
Nothing had been published yet, so the history was rewritten once, dates and
contents untouched.

### "AI slop"

The first interface worked. A project could be created, tasks added and
dragged between statuses, a task opened and edited, a dependency loop refused
with a clear message. All of it had been checked in a browser.

The founder looked at it, called it AI slop, and asked for an interface that
looked designed by people.

The verdict was right. It was the look a language model reaches for by
default: a near-black background, an indigo accent, soft tinted pills, the
browser's own drop-downs and date fields. Nothing in it was a decision.

The founder pointed to a method: write a design philosophy first, then
express it. So the work stopped and a philosophy was written, called
*Joinery*: parts that fit, hairlines instead of shadows, paper and graphite,
one metal, pigment only for meaning
([philosophy](../design/philosophy.md)). It was first expressed away from the
product, as a single sheet: *Plate 01*, an engineering plate with the logo
taken apart into five parts and a plan drawn as bars on a scale
([key visual](../design/key-visual.png)).

Then the interface was rebuilt in that language. The logic underneath did not
change by a line. What changed:

- Three typefaces that ship with the application: a grotesque for the
  interface, a serif used once per view, a monospace for anything counted.
- A warm graphite ground in the dark theme and paper in the light one.
- Brass for exactly one thing: where you are, and who you are.
- Statuses drawn as small blocks that fill up as work proceeds.
- A tally in the header: one block for every task.
- Lanes ruled like a ledger instead of boxed columns.
- Menus, a calendar and dialogs made for the application, with the browser's
  own controls gone.

The logo changed with it. The colored blocks became the plate's first figure:
four parts and a brass keystone.

## Why it went this way

A project-management tool is looked at all day. If it looks generic, the
claim that it was made with care is not believed, whatever the code is like.
The rejection cost a few hours and gave the project a visual language it can
use everywhere, including in the film.

Keeping the rules out of the interface is what made the redesign cheap. The
board, the panel and the list were redrawn; the model, the use cases, the
storage and their tests were not touched.

## Decisions

No new decision records. The design language is described in
[`docs/design`](../design/README.md).

## Storyboard beats

1. A cutaway of a small machine being assembled from the inside: a core, a
   ring around it, a casing, and last of all a glass front.
2. A crowded search path drawn as a queue. An ancient tool at the front keeps
   answering every call. A hand reaches past it and taps the right tool on
   the shoulder, by name.
3. The first screen: dark, purple, soft. The founder's verdict lands on it as
   a rubber stamp: two words.
4. The screen is wiped. A sheet of paper slides onto a drafting table. A
   pencil draws five squares, hatches three, fills one, and gilds the last.
5. The sheet is held up against the empty window. The lines of the plate
   become the lines of the interface: lanes, cards, a row of small blocks.
6. Dark theme, light theme: graphite, then paper, the same drawing on both.
