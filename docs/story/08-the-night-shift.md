# Chapter 8: The night shift

*12 to 13 November 2026, from the late evening into the night*

## What happened

### A list, and good night

Late in the evening the founder wrote one long message and went to bed.
Build the sync. Let me see the previous sprints when I want. Look at the
tracker I use, as closely as you like, and learn what they did and why:
views that can be added, folders and lists, a dashboard that says how a
project is going. Give every agent in the office a character, a pixel
person, so that it feels like a game where people really work. Use the
cheaper model for easy pieces and the stronger one for hard ones, and
stay the one who hands out the work. The message ended by asking whether
there was anything to ask before everything was left to the night.

Four questions went back, each with the answer that would be used if no
answer came. The answers came before the lights went out, and one of them
turned a plan over: do not bring me a proposal for views, build them.

### Reading a tracker with the lights off

The tracker was open in a browser beside the conversation, signed in. The
window was hidden, so no picture of it could be taken all night. It was
read as text instead, which turned out to be enough, and nothing in it was
created or changed.

Three shapes were written down. A place has a row of views; when the row
is full the rest is behind "2 more", and a chooser offers some twenty
kinds. A project is a folder, and inside it there is a list for each job.
And a dashboard begins with a summary in sentences, then a few numbers,
then small charts, then what is due.

What was not taken was the amount: the store of kinds, the cards that each
need settings, and a summary that asks a model every time the page opens.

### Three desks would not open at once

The first three pieces went out together: the Rust side of sync, the
people of the office, and the previous sprints. One assistant started.
The other two failed in the same second with the same sentence about a
configuration that could not be read. Three checkouts had been asked for
at once. They were started again one after the other, and from then on
always one at a time.

The computer set its own limit too. With four assistants at work, two and
a half gigabytes of memory were free. A fifth had to wait for a chair.

### Sentences that cost nothing

The dashboard came back first with its own answer to the founder's
"summary that you can update". Its first lines are sentences: "5 of 21
tasks are finished, and 4 are in work. 1 task is late. Sprint 2 has 9
days left and 16 of 22 points to go. 1 thing waits for you." No model
writes them. Rules work them out from the tasks, so opening the page
costs no tokens, and a sentence with nothing to say is left out. Beside
them stands the manager's own note.

### People who were too small

The people of the office came back as ten hand-drawn kinds, sixteen
pixels by twenty-four: a manager in a gold jacket, an Advisor in a long
coat, headphones for software, a loupe for hardware, an apron for the
mechanical engineer. They worked under the lit lamp, raised a hand when
they waited, and walked to a window or a coffee corner when free.

In the pictures they sat on the top edge of a card like figures on a
shelf. The room still read as cards. A second round made them half again
as large in a wide window and gave each a wooden desk with a brass rule
along its front. Now the card is the label of a desk, and an empty chair
says who is away.

### Two computers on one table

The assistant that built the sync wrote nine thousand lines in eight
commits. One of its own tests made three hundred random changes on two
computers and found a flaw in the design: a rule that let the larger of
two time stamps win kept two computers apart for good when their clocks
disagreed. The rule was changed before anyone saw it fail.

Then the built program was started twice on the same computer, each copy
on data of its own, with one folder between them. One was called Desk and
one Laptop. Laptop saw Desk and took nothing in until it was told to.
Then the project arrived with its tasks, its team and its sprint. A title
changed on one and a priority changed on the other both held. A task
deleted on Desk was gone on Laptop, and Laptop was told that its change
had been dropped. Every check but one passed.

That one looked at Laptop's window and found it showing "No projects
yet" while the database behind it held the project. Nothing listened to
the news. That was the interface, which another assistant was writing at
that moment. When it landed, the check passed, and a press on "Take in"
in the real window brought the project into view.

### The test that was wrong

The views were written twice over: an interface with a demo and
thirty-nine cases, and then the Rust side against those cases. Both
landed, and the test of the built program failed on one line. It expected
a refusal to say that the project already had a view called "Gantt". The
program said "gantt", the name as the manager had typed it. The program
was right, and so was the demo it had been written against. The test had
been written from memory.

A script that was to press the "+" in the real window first pressed the
tab called List instead of the entry called List in the chooser, and then
twice found nothing to press at all, because of a backslash lost on the
way. The fourth time it added a view, named it Firmware, filtered it to
one member, and the view said: "Shows the tasks of Firmware engineer. New
ones go to them."

### A film in a quarter of an hour

One assistant was given the 3D program that sits unpacked in a download
folder, a skill for short product films, and a computer with no graphics
card to speak of. The skill says to stop after the storyboard and wait
for the owner; the founder had waved that through for this one trial. The
storyboard has six shots: one lamp over a bench, the plan appearing, more
lamps for the team, a card held by four gold corners, and the benches
turning into the mark. Its 336 frames took about a quarter of an hour
and left a film of fourteen seconds. Nobody has watched it move.

### The limit, once more

At two in the night the plan's limit came again, in the middle of two
pieces. Both assistants stopped where they stood, one about to take a
picture, one about to edit a file. Half an hour later they were told that
nothing had been wrong with their work, and went on.

## Why it went this way

**Ask with the answer attached.** A question to someone who is about to
sleep should say what happens if it is not answered. Then no answer is
also an answer, and the night is not lost to waiting.

**Take the shape, leave the amount.** A tool that many people use has
solved where things go on a screen. It has also grown a store of things
nobody misses. The first can be learned from; the second is the reason to
build something smaller.

**What costs nothing can run every time.** A summary written by a model
is better prose. A summary worked out by rules is there the moment the
page opens, on every project, for nothing. The first can come later, on
request.

**Draw it, look at it, draw it again.** Nobody could have told from the
numbers of a pixel grid that the people were too small. It took a picture
of the room.

**A test can be wrong.** When the program and a test disagree, the
specification decides. Here the demo was the specification, and it sided
with the program.

**Run it twice to see it once.** Syncing cannot be seen in one window.
Two copies of the program on one table are not two computers, and the
roadmap says so, but they showed in two minutes what a hundred tests in
one process could not: that a window did not know what its own database
had learned.

**Do not run what you were not asked to run.** The founder named a
command that fetches and runs other people's code. It was left alone
until the founder said in so many words to run it.

## Decisions

- [0017: Keep computers in step through files of operations in a folder the user owns](../decisions/0017-sync-through-a-folder.md), accepted, with a note on what changed while it was built
- [0019: Put people in the office, drawn as pixel characters](../decisions/0019-people-in-the-office.md)
- [0020: Work the dashboard out by rules, and have nothing written for it](../decisions/0020-a-dashboard-worked-out-not-written.md)
- [0021: Views the manager can add](../decisions/0021-views-the-manager-can-add.md)
- [0022: Cut a project into areas, one for each job inside it](../decisions/0022-areas-inside-a-project.md)

## Storyboard beats

1. A figure puts a long list on a table, switches off the light and
   leaves. On the list, next to each line, a second hand has already
   written a small answer in pencil: "if you say nothing".
2. A dark window with a page behind it. Nothing can be seen. A hand runs
   along the page like a reader of raised print and writes three shapes
   on a card: a row, a folder, a page that begins with words.
3. Three doors are opened at once and two slam shut. They are opened
   again, one after the other.
4. A workshop seen from above. Small people sit on the edges of cards.
   A pencil draws a desk under each, and they grow until the cards are
   only labels.
5. Two identical desks, one marked Desk and one Laptop, with a box
   between them. A paper goes into the box on the left. On the right
   nothing moves, though the drawer under the right desk has filled. A
   bell is hung over the right desk. The next paper rings it.
6. A ruler measures a word and says it is wrong. A second ruler is laid
   beside it: the first ruler was bent.
7. A gauge runs to its end and two people stop in the middle of a step,
   one with a camera raised, one with a pen on paper. The gauge fills
   again. Both finish the step.

## Note of 13 November 2026, before morning

This chapter was written while two pieces were still on their way. Both
landed before the founder woke. The six rough spots that the interface had
found in the Rust side of syncing were mended; one of them had been listed
as not built at all: the history of a task now names the computer that
made each change. And the lists of the tracker became areas: a project can
be cut into the jobs inside it, with a name of their own because
Teamwright already has a view called List. Folders that group projects
were left for a manager who has more projects than fit on a screen.

