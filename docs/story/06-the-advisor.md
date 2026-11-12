# Chapter 6: The Advisor, and a lamp

*12 November 2026, the evening*

## What happened

### Five questions and sixteen tasks

The product is supposed to begin with the Advisor: the manager says what
they want, the Advisor asks what it needs to know, and then proposes the
smallest team that can do the job and a plan. All day the work had been
about one specialist. The Advisor was still a paragraph in the vision.

By the evening the way to build it was lying on the table. A specialist
asks the manager through a tool, and its question is read from the stream
of events. An interview is several such questions at once. A proposal is
one more call with a larger shape.

Before a line of the product was written, the idea was tried, as the
afternoon had taught. A stand-in tool server of two hundred lines offered
two tools, `interview` and `propose_plan`, and a real session on a middle
model was given a page of instructions and a brief of five sentences: a
small device for the desk that measures the air, with a screen, Wi-Fi, a
board of the manager's own and a printed case.

It asked five questions. Which microcontroller, with three options and what
each leads to. Which sensor. Which screen and how it is powered. Where the
history lives, so that it can be seen on a phone. Which design tool. Then it
ended its turn in one sentence.

The answers were written back the way the program would write them. In its
second turn the session proposed a team of three, a hardware engineer on the
strongest model "because the custom board is the core of the project", a
firmware engineer and a mechanical designer on the middle one. And sixteen
tasks. "Define the pin map" stood between the parts and the schematic, so
that the firmware could start before the board was drawn. "Approve the parts
choice" was given to the manager. So were ordering, soldering and printing:
the instructions had said that a specialist has no hands.

The call passed every check at the first attempt. The two turns took 29
seconds and wrote about 3,400 tokens.

Two things in it were not as wanted, and both went into the instructions.
It asked the manager to choose a microcontroller, which is a specialist's
work. And it wrote "Recommended." into the text of an option that it had
already marked as recommended.

### A page, not a panel

The decision was written down
([0014](../decisions/0014-the-advisor-plans-through-two-tools.md)), then the
contract between the interface and the application, then a stand-in of the
application for the browser with some forty tests that say how planning
behaves. An assistant took those tests to the Rust side.

The screens were built against the stand-in. A question of a specialist
fits into the panel of its task. A proposal does not: it is a team and a
table of tasks, and the manager is meant to change it. So the Advisor got a
page of its own, a tab next to the board.

The proposal can be edited where it stands. A cross takes a member or a task
out. The tasks of a member that is taken out stay in the plan and are given
to nobody, because the work is still to be done when the one who was to do
it is gone. The line above the button says how many there are.

A new project used to open on an empty board, lanes with nothing in them,
and nothing that pointed to the Advisor. It now opens on the Advisor's page,
with the description of the project already in the field.

On the stand-in the whole loop ran for the first time: a new project, three
questions, a proposal, one press on "Approve", and a board with twelve tasks
in it, each with its owner.

### The first real plan

The Rust side came back in five commits, the last one never compiled, like
the two before it. On the main line it compiled at the first attempt.

Then the Advisor planned its first project in the real program. Twelve
seconds after the brief it asked four questions: where the history is to be
seen, which sensor, how the device is powered, and whether the board is made
by a factory. It did not ask for the microcontroller this time. It wrote it
into the summary instead, under "Assumptions to confirm in the first task".

Eighteen seconds after the answers, the proposal stood in the window: a
hardware engineer, a firmware engineer and a mechanical designer, and twelve
tasks, three of them the manager's. Nothing was in the project. One press on
"Approve", and the board had twelve cards.

### The space above a question

Looking at the questions of the interview, something was off: each question
stood too far below its number, and too far above its reason. The decision
card of the afternoon had the same gap, and it is in the picture of the
inbox in the manual.

Nobody had put it there. The style sheet takes the margins off headings, but
only down to the third level, and the question is a heading of the fourth.
The browser had been adding a line of space above and below it all along.

### The lamp

In the afternoon the founder had been shown three directions for the look
of the board and had not answered. In the evening the answer came: "I
accept lamplight".

Lamplight is the direction that argued with the founder's own idea. The
cards stay flat. Gold appears only as light, and only where something is
happening: under the pointer, around the card a specialist is working on,
on the one card that asks the manager to decide. The founder's sun, which
was to be on every card, rises on that one.

It was handed to an assistant in a checkout of its own, with the page of
proposals as its reference and a list of what not to touch. Its first
commit changed a sentence of the design philosophy: "Nothing glows."

### Three desks

For a while three lines of work ran at once: the Rust side of the Advisor,
the lamp, and the screens. The connection of the first assistant stalled
after its first commit, and it was stopped. It was shown its desk again and
went on with the second.

## Why it went this way

**The Advisor is not on the team**
([decision 0014](../decisions/0014-the-advisor-plans-through-two-tools.md)).
A team is who tasks can be given to. Making the Advisor a member would have
given it a face for free, and every place where a task is assigned would
have had to leave it out.

**Ask everything at once.** One question in each turn needs no new tool.
Five questions would be five turns, and the manager would answer the first
without seeing the fifth.

**Nothing before the approval.** The Advisor proposes. It has no tool that
creates a member or a task, and no tool that writes a file. That nothing
starts before the manager agrees does not depend on the Advisor asking
first.

**The work stays when the worker goes.** Taking a member out of a proposal
is a judgment about the team, not about what has to be done.

**Begin where the product begins.** An empty board says "add a task". The
product says "tell me what you want".

**Try it before building on it**, again. The experiment cost half a minute
and changed two sentences of the instructions before they existed.

## Decisions

- [0014: Let the Advisor interview and propose through two tools](../decisions/0014-the-advisor-plans-through-two-tools.md)

## Storyboard beats

1. A manager at a desk says five sentences to a figure with a notebook. The
   figure does not start to build. It turns the notebook round: five
   questions, each with boxes to tick.
2. The boxes are ticked. The figure draws three chairs, writes a reason
   under each, and pins sixteen cards on a wall with threads between them.
   Three of the cards it hands to the manager: the ones that need hands.
3. The manager takes one chair away. The cards that belonged to it do not
   fall. They hang there with no name on them.
4. A stamp comes down: "Approved". Only now do the chairs fill and the cards
   move to the board.
5. A door opens on an empty room with four bare walls. Cut. The same door
   opens on the figure with the notebook, waiting.
6. A ruler is laid against a page. Every heading has a gap above it that
   nobody drew. A finger finds the line in the rule book that stops one
   level too early.
7. A dark workshop. A single lamp is lit over the one bench where somebody
   is working. A second lamp is lit over a card that says "decide".
   Everywhere else it stays dark, and that is why the two are seen.
