# 0022. Cut a project into areas, one for each job inside it

- Status: accepted
- Date: 2026-11-13

## Context

A project of Teamwright was flat: all its tasks in one heap, told apart
only by who they were given to. The owner of the project asked for "lists
for different jobs under some project", as the tool they use for their
own work has: a device is one project, and its electronics, its firmware
and its enclosure are each a list of their own.

That tool calls them lists. Teamwright already has a view called List,
and "the List view of the list Firmware" helps nobody.

The same tool also groups projects into folders. A manager with a handful
of projects does not need that yet.

## Decision

- **The name is "area".** An area is a part of a project with a job of
  its own: Electronics, Firmware, Enclosure.
- **A project has one or more areas, in an order the manager chooses.**
  Every project starts with one, called "General". A project with a
  single area looks exactly as before: nothing of areas is shown until
  there are two. Nobody who does not want them pays for them.
- **Every task is in exactly one area.** A subtask is in the area of its
  task and moves with it. A new task goes into the area that is open, or
  into the first area when all are shown. The tasks of a plan the Advisor
  made go into the first area.
- **An area is a way to look at the whole project, not one more filter of
  a view.** With two or more areas, the navigation at the side shows them
  indented under the name of the project. Pressing the project shows all
  areas; pressing an area shows only its tasks in every view and in the
  tally of the header, and the heading then reads "project > area". The
  filter of a view applies on top. Like the filter, the area is applied
  where the tasks are handed to a view, so the views stay as they are.
  The places (Team, Office, Advisor) are about the whole project and show
  all of it.
- **Sprints span areas.** A sprint belongs to the project. In an area,
  the backlog and the board of a sprint show the tasks of the area that
  are in it; the burndown and the sentence of the dashboard about the
  sprint count the whole sprint.
- **The areas are stored with the project**, not in the window, so that
  they travel to the manager's other computers. Which area is open is
  where the manager is, like the open view: remembered between sessions,
  falling back to all areas when the remembered one is gone, and kept
  while the inbox or the computers are open.
- **Managing areas** is a dialog reached from the menu of the project,
  built like the dialog of the statuses: add, rename in place, drag to
  order, remove. The panel of a task shows its area once there are two,
  and choosing another moves the task with its subtasks.
- **The rules are use cases.** Each refusal has a stable code.
  - A name is trimmed and put on one line, is not empty, and has at most
    40 characters (`rule_broken`). No two areas of a project share a
    name, whatever the capitals (`area_name_taken`).
  - The last area of a project cannot be removed (`last_area`).
  - An area that holds tasks is removed only when it is said which area
    they go to (`area_in_use`), and only to an area of the same project
    (`different_projects`). They keep their status and their place.
  - A subtask is not moved by itself: it goes where its task goes
    (`subtask_moved_alone`). A new subtask that names another area than
    its task's is refused (`rule_broken`).
  - A task goes only into an area of its own project
    (`different_projects`).
  - An area, a project or a task that does not exist is `not_found`.
  - Deleting a project deletes its areas.

## Consequences

- A manager with one area sees nothing new except *Areas* in the menu of
  the project.
- The demo backend and `demoAreas.test.ts` are the specification. The
  Rust side follows them case by case: a table of areas with the project,
  the name and the place in the order; a column with the area of every
  task; the use cases; the six commands of the shell. A database that has
  projects from before must give each of them the area "General" with all
  its tasks in it. Until then the built program cannot list areas and
  shows a project as one heap, as before.
- The area of a task is not in the history of the task yet.

## Alternatives considered

- **Call them lists**, as the tool the owner uses does. Refused because
  of the List view.
- **Make an area one more filter of a view.** Simpler to build, but the
  manager would have to set it on every view, and "where am I" would
  change with every tab. An area is a place, so it lives in the
  navigation.
- **Folders that group projects.** Left for later: with a handful of
  projects, the list at the side is short enough as it is.
- **Projects inside projects.** A second project per job would split the
  team, the sprints and the statuses, which the jobs of one device share.
