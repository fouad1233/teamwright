# Troubles and answers

This page lists what can go wrong and what the program says about it, then
the keys you can use, and where your data is. Every answer here comes from how
the program is built and documented. If your trouble is not here, the
[roadmap](../product/roadmap.md) says what was never tried.

## Specialists

**A turn failed and says Claude Code is not signed in.** Open a terminal and
run `claude auth login`. Then answer the task to try again. The *Team* place
shows the version and how you are signed in, or what is missing. Teamwright
never asks for your password.

**Claude Code is not found.** Install it on this computer. A Claude Code that
npm installed is a script on Windows, and a turn on it is refused with the
reason. This is not solved yet.

**Every specialist stopped at once.** Most likely the limit of your Claude plan
was reached. Look at the foot of the navigation: *The limit of your Claude plan
is reached. It resets …* The stopped turns say *The limit of the Claude plan was
reached.* Nothing is lost and the tasks stay in work. After the reset, press
*Try again* under each stopped turn, or let the coordinator go on by itself if
it is on. If the line says that when it resets is not known, press *Try again*
once to find out. If there is no such line, read the turns: another reason,
such as Claude Code not being signed in, says so in words. See
[the limit of your Claude plan](work.md#the-limit-of-your-claude-plan).

**A specialist says in its report that it could not do something.** It cannot
run commands, fetch from the internet or leave its folders unless you allow
it. It should ask you for a permission, which waits in the *Inbox*; see
[when a specialist asks for a permission](work.md#when-a-specialist-asks-for-a-permission).
For what can never be allowed, see
[what a specialist may and may not do](safety-and-trust.md#what-can-never-be-allowed).
Do the step yourself, or give it a skill or a connector tool that does it.

**I allowed a permission and want it back.** Open the *Team* place, find
*Permissions*, and press *Take back* in its row. The next turn starts without
it. A row marked *no longer valid* already allows nothing; *Take back*
removes it.

**The request I wanted to allow was refused as I typed it.** Teamwright
refuses a command that could join or quote other commands, name a path
outside the folder anywhere in a word, or start with a program that only
starts another. It also refuses a command allowed always, any command on
Windows, and a command for a specialist that has a connector given a secret,
and says which. Allow one exact command, once or for the task. The list is
in
[what can never be allowed](safety-and-trust.md#what-can-never-be-allowed).

**I cannot remove or ask a specialist, or delete its task or project.** It is
working. A specialist works on one thing at a time. Press *Stop* first.

**I closed Teamwright while a specialist worked.** The specialist was stopped,
and its turn says so the next time you look.

**A specialist writes outside the folder I expected.** It works in the work
folder of its project. Check *Folders* in the menu of the project: you may
have chosen another folder. What it changes there is changed for real.

**A task was not started by the coordinator.** Read the list *Waiting* in the
*Team* place: it says why. Common reasons: the task waits for a task that is
not in a status that means done, its specialist is at work on something else
or waits for your answer, the limit of specialists at work at once is reached,
the task is given to a person or to nobody, a budget is used up, or somebody
already worked on the task (a task that has a conversation is never started
again), or the limit of your Claude plan is reached.

**The coordinator asked about a budget.** Look in the inbox. You can raise the
budget by half, by all of it again, or leave the work stopped.

## Tasks and views

**A lane shows a figure in red with the word *over*.** The limit of the status
is exceeded. Nothing is blocked; the limit is a signal.

**A dependency was refused.** It would make tasks wait for each other in a
circle. The message names the tasks.

**A task I added vanished from a view.** The view is filtered by a tag or a
field, and the new task does not carry it yet. Open it from another view, or
clear the filter with *Show every task*.

**I cannot move a task into a sprint.** A completed sprint takes no more tasks.

**I cannot change a field's kind.** The kind never changes. Make a new field
and remove the old one.

## Several computers

**The line at the foot of the sidebar says "The folder cannot be reached".**
The chosen folder is not there, for example the cloud drive is not running.
Syncing goes on when it is back. Nothing is lost.

**Another computer waits under *Other computers*.** Choose *Take in* or
*Ignore*. Nothing of it is taken in until you do.

**Teamwright stopped syncing and talks of one identity.** You copied the data
folder from one computer to another. Press *Give this computer a new identity*
on the computer the folder was copied to, and start Teamwright again. See
[a copied computer](several-computers.md#a-copied-computer).

**A computer shows a clock more than five minutes ahead, or a newer
Teamwright.** Set its clock right, or install the same version on every
computer.

**I cannot continue a conversation on my other computer.** A conversation can
be read everywhere but continued only on the computer where it started.

## The program

**Windows or macOS warns before opening the installer.** The installers are not
signed. See [installing](what-teamwright-is.md#installing).

**There is no notification.** They are off until you allow them on the page
*Computers*, they show only while the window is not in front, and each kind has
its own switch. Nobody has yet seen one on the screen, so tell the project if
it does not appear.

**The demo does not keep my changes.** The demo is a browser version with
sample data. It says "Demo data, changes are not saved".

## Keyboard

| Key | Where | What it does |
|---|---|---|
| Tab | Everywhere | Moves to the next control |
| Enter | On a card or a row | Opens the task |
| Tab | On a row of the Backlog view | Reaches its menu, which moves the task to a sprint |
| Space | On a card | Picks the card up, and drops it |
| Arrow keys | While a card is picked up | Move the card |
| Escape | While a card is picked up | Puts the card back |
| Arrow left, arrow right | On a bar of the Gantt chart | Moves the task by a day |
| Shift with an arrow | On a bar of the Gantt chart | Moves only the due date by a day |
| Enter | On a bar or a name of the Gantt chart | Opens the task |
| Escape | In a field | Undoes the edit |
| Ctrl and Enter | In the Advisor's message | Sends it |

## Where your data is

Your projects, tasks, teams, memory and the record of what the specialists did
are stored in one database file on your computer, in the folder your operating
system gives to the application (`%APPDATA%\dev.teamwright.app` on Windows).
What the specialists produce are files in the folders of the projects,
`Teamwright` in your documents by default.

Teamwright itself sends nothing anywhere you did not choose. See
[what leaves your computer](several-computers.md#what-leaves-your-computer)
and [safety and trust](safety-and-trust.md#what-leaves-your-computer).
