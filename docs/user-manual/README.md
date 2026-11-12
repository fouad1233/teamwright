# User manual

This manual describes what Teamwright can do today. It grows with every
milestone. This version plans and tracks projects, lets the Advisor propose
a team and a plan for you to approve, lets you put together a team
yourself, and lets the AI specialists on it carry out tasks, one at a time.

## Projects

A project holds one piece of work: its tasks and the statuses they move
through.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/first-run-dark.png">
  <img alt="The first screen of a new installation, with the button Create a project" src="../screenshots/first-run-light.png">
</picture>

**Create a project.** Press the plus sign next to *Projects* in the sidebar,
or *Create a project* on the first screen.

- **Name.** What the project is called.
- **Key.** A short code of two to six letters and digits that prefixes the
  numbers of the project's tasks, as in `IOT-12`. Leave it empty and
  Teamwright derives one from the name: "Smart sensor node" becomes `SSN`.
- **Workflow.** The statuses the project starts with.
  - *Standard:* To do, In progress, In review, Done.
  - *Engineering:* To do, In progress, In revision, In test, Done.

A new project is empty. It opens on the page of the
[Advisor](#planning-with-the-advisor), where it can be planned. The board is
one press away if you would rather add the tasks yourself.

**Switch projects** by choosing one in the sidebar. A small brass line marks
the project that is open. Teamwright reopens the last project you worked in.

**Change a project** from the menu with the three dots in its header.

- *Name and description* opens a form for both. The key stays as it is,
  because it is part of the number of every task.
- *Statuses* opens the list of the project's [statuses](#statuses).
- *Archive project* puts the project away. Nothing is lost: the sidebar
  then shows *Archived* under the list of projects, and from there a
  project is brought back, or deleted.
- *Delete project* removes the project for good, with its tasks and its
  team. Teamwright says what goes with it, and asks first.

### Statuses

The statuses are the steps a task goes through, and the lanes of the
board. A project starts with the statuses of its workflow, and you can
make them your own under *Statuses* in the menu of the project.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/statuses-dark.png">
  <img alt="The dialog for the statuses of a project: one row for each, with its color, its name and its meaning" src="../screenshots/statuses-light.png">
</picture>

Every status has a name, a color, and a meaning. The meaning is one of
four, and it is what Teamwright goes by, whatever the status is called:

| Meaning | What it says about a task |
|---|---|
| Not started | Work on the task has not started. |
| In progress | Work on the task is under way. |
| In review | The work waits to be checked. |
| Done | The task is finished. |

- **Rename** a status by typing over its name. Enter or leaving the field
  saves, Escape undoes.
- **Change the color** with the mark in front of the name, and **the
  meaning** with the word behind it. A status that becomes *Done* finishes
  the tasks in it, and one that stops being *Done* reopens them.
- **Move** a status by dragging the dots at the front of its row. The board
  follows.
- **Add** a status with *Add status*. It appears with the other steps of
  the work, in front of the ones for review and done, with a name ready to
  be typed over.
- **Remove** a status with the bin at the end of its row. If tasks are in
  it, Teamwright asks which status they should go to.

A project always keeps one status in which work has not started and one in
which it is done. The last one of each kind cannot be removed or given
another meaning.

Changes are saved as you make them.

### Folders

*Folders* in the menu of a project says where its specialists work on this
computer, and what else they may read.

- **Work folder.** Here the specialists read, create and change files.
  Teamwright makes a folder for every project. *Choose another folder* lets
  them work somewhere else, for example in a folder that already holds the
  project. What they change there is changed for real. *Use the folder of
  Teamwright* goes back. The work folder cannot change while a specialist
  works in the project.
- **Reference folders.** Folders with material the specialists should know:
  data sheets, an earlier design, a standard. They can read everything in
  these folders, PDF files included, and cannot change anything in them. A
  reference folder cannot be the work folder, lie inside it, or contain it.
- **Reference files.** *Add files* copies single files into the folder
  `references` inside the work folder. The specialists see the copies, and
  nothing else of the folder a file came from.

Teamwright never deletes a folder or a file of yours, also not when you
delete the project. Keep in mind that what a specialist reads is sent to
Anthropic under your account, like everything Claude Code reads.

## Planning with the Advisor

The *Advisor* turns what you want into a team and a plan. It is under the
tab *Advisor* of every project.

**Say what you want.** Write it in your own words: what is to be built, what
you have already, what matters most. The description of the project is
there to start from. Choose the model and the effort the Advisor runs on,
or leave both to Claude Code. Then press *Start planning*.

The Advisor can read the [folders](#folders) of the project and cannot
change anything in them. Give it what it should know there before you
start.

**Answer its questions.** If the Advisor needs to know something before it
can plan, it asks: up to five questions at once, each with the reason it
asks. Where it offers options, pick one. The one it recommends is marked.
Or pick *Something else* and say what you want. A question without options
is answered in words. Press *Send answers* when every question has an
answer. The Advisor may ask a second time.

**Read the proposal.** The Advisor proposes three things.

- *The project in its words*, for you to check: what is built, what "done"
  means, what is left out.
- *The specialists to add to the team.* Each has what it is responsible
  for, why the project needs it, and the model and the effort it runs on.
- *The tasks.* Each has who does it, its size, and the tasks it waits for.
  Work that needs hands, money or an account is given to you. Press the
  title of a task to read what the Advisor wrote about it.

The size of a task is the Advisor's estimate of the hours a skilled person
would need for it. It says how large the task is. It does not say how long
a specialist takes.

**Change it.** The cross at the end of a row takes a specialist or a task
out. Press the owner of a task to give it to somebody else. Press the model
or the effort of a specialist to choose another. The tasks of a specialist
you take out stay in the plan and are given to nobody. *Undo my changes*
brings the proposal back as the Advisor made it. For anything larger, write
to the Advisor under the proposal what to change, and it proposes again.

**Approve it.** Nothing is added to the project before you press *Approve*.
Then the specialists join the team, and the tasks appear in the first status
of the board, each given to its owner. No work starts by itself: you start a
task when you want it started. A specialist the team already has is not
added a second time.

**Later.** Write to the Advisor again when the project needs more. It knows
who is on the team and which tasks exist, and proposes what is to be added.
*Plan something new* starts a new conversation.

What the Advisor waits for, answers to its questions or a word about its
proposal, is also counted in [the inbox](#the-inbox).

**What it uses.** The Advisor is a session of your own Claude Code, like a
specialist. Planning takes two or three turns, and they count against your
plan. A lighter model makes a plan too. It will be a plainer one.

## The header of a project

Next to the name of the project are its key and, if the project has a team,
the avatars of the members. Pressing them shows the team.

On the right is a row of small blocks, one for every task, in the color of
the task's status. A hollow block is a task that has not been started. The
count next to it says how many tasks there are and how many are done.

## Tasks

**Create a task** in one of three ways:

- *New task* in the header opens a task by its title and shows it at once, so
  you can assign it and fill in the rest.
- *Add task* at the bottom of a status on the board adds a task to that
  status. The line stays open, so you can type one task after another. Escape
  closes it.
- *Add task* under *Subtasks* in a task adds a part of that task.

**Open a task** by clicking it on the board or in the list. Its panel shows
everything about it:

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/task-dark.png">
  <img alt="The panel of a task, open at the right of the board" src="../screenshots/task-light.png">
</picture>

| Field | What it is |
|---|---|
| Status | Where the task stands. Choosing another status moves the task. |
| Assignee | Who is responsible: you, a member of [the team](#the-team), or nobody. |
| Priority | Urgent, high, normal or low. Urgent and high show a colored flag. |
| Estimate | The expected working time. Type it as `90m`, `2h` or `1h 30m`. |
| Start, Due | The planned dates. A task cannot start after it is due. |
| Description | Details, acceptance criteria, links. |
| Work | What the AI specialist the task is given to does with it. See [the work of a specialist](#the-work-of-a-specialist). |
| Waits for | The tasks that must be finished before this one can start. |
| Subtasks | The parts of this task. |
| History | What has happened to the task since it was opened, the latest change first: who it was given to, where it moved, which dates it got. |

Changes are saved as you make them. The title, the description and the
estimate are saved when you leave the field or press Enter. Escape undoes an
edit that is not saved yet.

**Delete a task** from the menu with the three dots in the task's panel. Its
subtasks are deleted with it, and tasks that waited for it stop waiting.

### Subtasks

A task can be divided into subtasks, and a subtask can have subtasks of its
own. Add one with *Add task* under *Subtasks* in the panel of the task.

On the board and in the list a subtask is shown under its task, not next to
it. A task with subtasks has a small row that says how many there are, with
one block for each in the color of its status. Press it to unfold the
subtasks, and again to fold them away. Teamwright remembers which tasks you
unfolded.

A subtask has a status of its own, which the mark in front of its title
shows. It stays under its task whatever that status is. To change it, open
the subtask.

### Tasks that wait for other tasks

A task can wait for any number of other tasks of the same project. On the
board, an hourglass with a number shows how many of them are not finished.

Teamwright refuses a dependency that would make tasks wait for each other in
a circle, and says which tasks are involved: *"SSN-1 cannot wait for SSN-3,
because SSN-3 already waits for SSN-1."*

## The team

A project has a team: the help you bring in. A member of the team is an AI
specialist or a person, and a task can be given to either.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/team-dark.png">
  <img alt="The Team view: you as the manager, then the members of the team" src="../screenshots/team-light.png">
</picture>

**What a member can do today.** An AI specialist carries out the tasks you
give it and answers your questions; see
[the work of a specialist](#the-work-of-a-specialist). A person is a name
that tasks can be given to: Teamwright does not notify them.

**Add a member** with *Add member* at the bottom of the *Team* view.

- **Kind.** *AI specialist* or *Person*. It cannot be changed afterwards.
- **Name.** What the member is called on the team. For a specialist this is
  usually its discipline, as in "Hardware engineer". No two members of a
  team can have the same name.
- **Responsible for.** A sentence about what the member is on the team for.
- **Model** and **Effort.** For an AI specialist: the model it runs on, and
  how hard that model thinks. Both can be left to Claude Code. A lighter
  model and less effort use less of your plan and are enough for plain
  work. You can change them at any time; they hold from the next piece of
  work on. The *Team* view says how each specialist runs.

Teamwright gives every member a color that sets it apart from the others.

**Change or remove a member** from the menu with the three dots in its row.
Removing a member gives its tasks to nobody. Teamwright says how many tasks
that is, and asks first.

**Give a task to a member** by opening the task and choosing the member
under *Assignee*.

**How members are shown.** Where there is no room for a name, a member is
shown by its initials in its color.

| Shape | Who |
|---|---|
| A brass circle | You, the manager |
| A block with initials | An AI specialist |
| A circle with initials | A person |
| A dashed circle | Nobody |

In the *Team* view, the blocks next to a member are its tasks, in the colors
of their statuses.

## The work of a specialist

An AI specialist is a session of Claude Code, the program of Anthropic, that
runs on your computer under your own account. Teamwright starts it, shows
what it does, and keeps the record.

**What you need.** Claude Code has to be installed on this computer, and you
have to be signed in to it. The *Team* view says how that stands: the
version it found and how you are signed in, or what is missing. To sign in,
open a terminal and run `claude auth login`. Teamwright never asks for your
password and never reads how you are signed in.

**Start the work.** Open a task that is given to an AI specialist and press
*Start work*. The specialist gets a brief: the task and its description,
what it is a part of, its parts, the tasks it builds on and how they stand,
when it is due, and what you expect back. The task moves to the first status
in which work is under way.

**Follow it.** The panel of the task shows every step as it happens: what
the specialist writes, and which files it reads and changes. The card of
the task carries a brass mark while somebody works on it, and the *Team*
view says what each specialist is working on. *Stop* ends the work at once.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/specialist-working-dark.png">
  <img alt="A task that a specialist is working on: the panel says Working and offers Stop, and the card of the task carries a brass mark" src="../screenshots/specialist-working-light.png">
</picture>

**Read the report.** A specialist ends with a report: what it did, which
files it created or changed, what it decided on its own, and what is still
open. The task moves on to the first status in which work is checked. Under
the report are the model, the tokens the turn used and how long it took.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/specialist-report-dark.png">
  <img alt="The report of a specialist under the steps it took: what it did, the files it changed, what it decided on its own and what is open" src="../screenshots/specialist-report-light.png">
</picture>

**Answer.** Write under the report to accept the work, to correct it or to
ask for more. The specialist goes on in the same session, so it remembers
what it did, and the task goes back into work until it reports again.
Teamwright never calls a task done by itself. That is yours to say.

**When a specialist needs you to decide.** A specialist does not guess at a
choice that is yours: one that changes what is built, what it costs or when
it is ready. It stops and asks. Its question shows in the panel of the task
and in [the inbox](#the-inbox): what it wants to know, why it matters, the
options with what each leads to, and the one it recommends. Pick an option
and add a note if you like, or pick *Something else* and say what you want
in your own words. Then press *Decide*, and the specialist goes on with your
answer. Until then the task stays in work, its card carries a hollow brass
mark, and the specialist is free for other work.

**Ask something.** *Ask something* in the menu of a specialist in the
*Team* view opens a conversation that is not about a task.

**Where the work is.** A specialist works in the work folder of its
project. Unless you choose another one, that is
`Teamwright\<key of the project>` in your documents. What it produces are
files in that folder, yours to open with any program. See
[folders](#folders).

**What a specialist may do.** It may read, search, create and change files
in the work folder of its project, and it may read the reference folders
you gave the project. It cannot run commands, it cannot fetch anything from
the internet, and it can neither read nor change a file anywhere else. It
also does not see what you have set up in your own Claude Code: your
connectors, plugins and settings stay yours. When a task
needs one of these, the specialist says so in its report instead of
working around it.
[Why it is this strict](../decisions/0011-what-a-specialist-may-do.md).

**One thing at a time.** A specialist works on one thing at a time. While it
works it cannot be asked something else and cannot be removed from the team,
and its task and its project cannot be deleted or put away. Stop it first.

**When something goes wrong.** A turn that fails says why in words, for
example that Claude Code is not signed in. Answer it to try again. If you
close Teamwright while a specialist works, the specialist is stopped, and
its turn says so the next time you look.

**What it costs.** A specialist uses your own Claude plan or account,
exactly as if you had used Claude Code yourself. Nothing goes through
Teamwright or its authors.

## The inbox

The *Inbox*, at the top of the sidebar, holds what waits for you in all of
your projects: the questions your specialists asked instead of guessing,
and what an Advisor asks or proposes. The number next to it says how many
there are, and what has waited longest comes first.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/specialist-inbox-dark.png">
  <img alt="The inbox with one question of a specialist: what it wants to know, why it matters, two options with what each leads to and the one it recommends, a place for a note, and the button Decide" src="../screenshots/specialist-inbox-light.png">
</picture>

Every question of a specialist can be answered where it stands. The line
above it names the project and the task, and the title of the task leads to
it. What an Advisor wants needs more room: *Open* leads to its page.

## Views

**Board.** One lane for every status. Drag a card to another lane to change
its status, or up and down to change its place. An overdue date is shown in
red. The number next to the name of a status counts the tasks in its lane.
[Subtasks](#subtasks) are not counted: they travel with their task.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/board-dark.png">
  <img alt="The board of a project, with one lane for each of its five statuses" src="../screenshots/board-light.png">
</picture>

**List.** One table of all tasks, grouped by status, with the subtasks of a
task set in under it.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/list-dark.png">
  <img alt="The list of a project: its tasks in rows, grouped by status" src="../screenshots/list-light.png">
</picture>

**Gantt.** The plan against time: one row for every task, in the order of
the schedule, and a bar from its start to its due date.

- A bar is hollow while the work has not started. It takes the color of its
  status once it has, and is quieter when the task is done.
- A task with only a due date is a small mark on that day. A task with only
  a start fades out to the right. A task without any date has a row and no
  bar.
- A line leads from a task to each task that waits for it. The line is red
  when the dates contradict it: the waiting task begins before the other
  one is due.
- A thin red line behind a bar shows how late the task is. The line in
  brass is today, and *Today* in the corner scrolls back to it.
- Drag a bar to move the task. Drag one of its ends to change only its
  start or only its due date. The new dates are shown while you drag, and
  Escape puts the bar back.
- Press a bar or the name of a task to open it.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/gantt-dark.png">
  <img alt="The Gantt chart of a project: a bar for every task against the days, and lines between tasks that wait for each other" src="../screenshots/gantt-light.png">
</picture>

**Calendar.** One month at a time. A task lies on the days from its start
to its due date, and a task with one date lies on that day. The arrows go
to the month before and after, and *Today* comes back. A week grows with
what is in it; a day with more than six tasks gets a note such as
*+2 more*, which opens the list of all its tasks. Tasks without any date
are not on the calendar: their number is shown at the top right.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="../screenshots/calendar-dark.png">
  <img alt="The calendar of a project: a month of weeks with the tasks on their days" src="../screenshots/calendar-light.png">
</picture>

**Team.** You and the members of the team, each with its tasks. See
[the team](#the-team).

## Keyboard

| Key | Where | What it does |
|---|---|---|
| Tab | Everywhere | Moves to the next control |
| Enter | On a card or a row | Opens the task |
| Space | On a card | Picks the card up, and drops it |
| Arrow keys | While a card is picked up | Move the card |
| Escape | While a card is picked up | Puts the card back |
| Arrow left, arrow right | On a bar of the Gantt chart | Moves the task by a day |
| Shift with an arrow | On a bar of the Gantt chart | Moves only the due date by a day |
| Enter | On a bar or a name of the Gantt chart | Opens the task |
| Escape | In a task's panel | Closes the panel |
| Escape | In a field | Undoes the edit |

## Where your data is

Your projects, tasks, teams and the record of what the specialists did are
stored in one database file on your computer, in the folder your operating
system gives to the application (`%APPDATA%\dev.teamwright.app` on Windows).
What the specialists produce are files in the folders of the projects,
`Teamwright` in your documents.

Teamwright itself sends nothing anywhere. A specialist that works is Claude
Code at work: it sends what it reads and writes to Anthropic under your
account, as it does when you use it yourself.

## Light and dark

Teamwright follows the light or dark setting of your operating system.
