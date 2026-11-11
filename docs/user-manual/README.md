# User manual

This manual describes what Teamwright can do today. It grows with every
milestone. The AI team is not part of the application yet; this version
plans and tracks projects.

## Projects

A project holds one piece of work: its tasks and the statuses they move
through.

**Create a project.** Press the plus sign next to *Projects* in the sidebar,
or *Create a project* on the first screen.

- **Name.** What the project is called.
- **Key.** A short code of two to six letters and digits that prefixes the
  numbers of the project's tasks, as in `IOT-12`. Leave it empty and
  Teamwright derives one from the name: "Smart sensor node" becomes `SSN`.
- **Workflow.** The statuses the project starts with.
  - *Standard:* To do, In progress, In review, Done.
  - *Engineering:* To do, In progress, In revision, In test, Done.

**Switch projects** by choosing one in the sidebar. A small brass line marks
the project that is open. Teamwright reopens the last project you worked in.

## The header of a project

Next to the name of the project is a row of small blocks, one for every task,
in the color of the task's status. A hollow block is a task that has not been
started. The count next to it says how many tasks there are and how many are
done.

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

| Field | What it is |
|---|---|
| Status | Where the task stands. Choosing another status moves the task. |
| Assignee | Who is responsible. Today that is you or nobody; team members will appear here. |
| Priority | Urgent, high, normal or low. Urgent and high show a colored flag. |
| Estimate | The expected working time. Type it as `90m`, `2h` or `1h 30m`. |
| Start, Due | The planned dates. A task cannot start after it is due. |
| Description | Details, acceptance criteria, links. |
| Waits for | The tasks that must be finished before this one can start. |
| Subtasks | The parts of this task. |

Changes are saved as you make them. The title, the description and the
estimate are saved when you leave the field or press Enter. Escape undoes an
edit that is not saved yet.

**Delete a task** from the menu with the three dots in the task's panel. Its
subtasks are deleted with it, and tasks that waited for it stop waiting.

### Tasks that wait for other tasks

A task can wait for any number of other tasks of the same project. On the
board, an hourglass with a number shows how many of them are not finished.

Teamwright refuses a dependency that would make tasks wait for each other in
a circle, and says which tasks are involved: *"SSN-1 cannot wait for SSN-3,
because SSN-3 already waits for SSN-1."*

## Views

**Board.** One lane for every status. Drag a card to another lane to change
its status, or up and down to change its place. An overdue date is shown in
red.

**List.** One table of all tasks, grouped by status.

## Keyboard

| Key | Where | What it does |
|---|---|---|
| Tab | Everywhere | Moves to the next control |
| Enter | On a card or a row | Opens the task |
| Space | On a card | Picks the card up, and drops it |
| Arrow keys | While a card is picked up | Move the card |
| Escape | While a card is picked up | Puts the card back |
| Escape | In a task's panel | Closes the panel |
| Escape | In a field | Undoes the edit |

## Where your data is

Everything is stored in one database file on your computer, in the folder
your operating system gives to the application
(`%APPDATA%\dev.teamwright.app` on Windows). Nothing is sent anywhere.

## Light and dark

Teamwright follows the light or dark setting of your operating system.
