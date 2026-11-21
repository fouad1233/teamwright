# 0033. Limits on the work in a status as a signal, and templates a project starts from

- Status: accepted
- Date: 2026-11-21

## Context

Milestone M6 of the [roadmap](../product/roadmap.md) names work-in-progress
limits, and templates for Scrum, Kanban and a hardware product.

A work-in-progress limit is the largest number of tasks that should stand
in one step of the work at once. In Kanban it is how a team stops starting
and starts finishing. Practice differs on what happens at the limit: some
tools refuse a card, most only show it. In Teamwright, much of the work is
moved by specialists, who finish a task and hand it on to review. A
specialist that cannot hand its work on because the next lane is full would
stop in the middle of a turn, for a reason it cannot see or fix.

A new project started with one of two workflows, Standard or Engineering,
chosen in the dialog of a new project from a list written by hand in the
interface. Every project started with the same six views and the area
"General". A Kanban project needs limits and no sprints; a Scrum project
needs its backlog in front; the [vision](../product/vision.md) speaks of
devices made of electronics, firmware and an enclosure, which a project
cuts into areas ([decision 0022](0022-areas-inside-a-project.md)).

## Decision

### Limits

- **A limit is a field of a status**: a whole number from 1 to 999, or none.
  It is set, changed and cleared where the statuses are edited, in a column
  of its own, and the change is stored as it is made.
- **A limit is a signal, never a wall.** Nothing refuses a task a status
  because the status is full: not a move on the board, not a specialist's
  change, not a task opened in the lane, not a limit set below the tasks
  already there. A specialist that finishes its work is never stopped by a
  limit. The board shows a lane over its limit; what to do about it is the
  manager's call.
- **A status of the category done has no limit.** Finished work only piles
  up there, so a limit would always end up exceeded and say nothing. Setting
  one is refused (`rule_broken`), and a status that becomes a done status
  loses its limit. Statuses in which work has not started may have one, as
  a Kanban "Ready" column has. The rule follows the category, never the
  name.
- **The limit counts the whole project.** Every task in the status counts,
  in every area and sprint, whatever the open view filters. A limit is about
  the work the team has under way, and that does not shrink because the
  manager looks at one area or one person. The head of a lane therefore
  shows the count of the project against the limit (as in `4/3`), and its
  tooltip says so.
- **A subtask does not count apart from its task.** The board shows a task
  with its subtasks as one card, and counting both would count the same
  work twice; the count then matches the cards.
- **The rule is in the domain.** `StatusLoad::of_workflow` counts each
  status against its limit and says whether it is under, at or over it. The
  use case `status_loads` hands that to the interface through the command
  `list_status_loads`, which the board asks again whenever the tasks or the
  limits change. The demo backend follows the same rules.
- **An exceeded limit is not lit.** Light falls where something waits for
  the manager ([decision 0015](0015-light-where-something-happens.md)). A
  full lane is a state of the flow, which a specialist may cause on purpose,
  not a decision that waits; gold there would cheapen the gold of a waiting
  card. A lane over its limit shows its figure and a gauge in the red of
  trouble, the same red the Workload view gives a day over its capacity,
  and the word "over" beside the figure, so that it reads without color. At
  the limit, the figure takes the ink of the name; under it, it is as quiet
  as a plain count.
- **The dashboard says it in a sentence** among the sentences worked out by
  rules ([decision 0020](0020-a-dashboard-worked-out-not-written.md)):
  "In progress is over its limit, with 4 tasks for 3."
- **It is stored with the status** (column `task_limit`, migration 0021),
  and is part of the state of the project in the operation log, like the
  name and the color of a status. It is logged, merged and synced with the
  workflow, which merges as one field: a limit changed on one computer and a
  status renamed on another at the same time keep the later of the two
  workflows, as two renames did before.

### Templates

- **A template decides what a new project starts with**: the statuses of
  its workflow with their categories, colors and limits; the views it
  starts with and their order; its areas; and whether a first sprint is
  planned. It never decides a team: the Advisor proposes one.
- **Five templates ship in code**, in `crates/domain` (`ProjectTemplate::ALL`),
  each with a name and two sentences for the manager:
  - **Standard**: to do, in progress, in review, done, the six views and
    the area General. It makes what every project was made with before.
  - **Kanban**: backlog, ready (limit 5), in progress (3), review (2), done;
    Board, List, Calendar and Dashboard. No backlog view and no sprint.
  - **Scrum**: the four steps of Standard, the Backlog view first, then
    Board, List and Dashboard, and "Sprint 1" planned, empty and without
    dates, ready to be filled from the backlog.
  - **Hardware product**: to do, design (4), design review (2), ordered,
    build (3), test (2), done; Board, Gantt, List, Calendar and Dashboard;
    the areas Electronics, Firmware and Enclosure, as in the sample project
    of the demo. "Ordered" has no limit: parts wait on suppliers, not on the
    team.
  - **Engineering**: the workflow that existed before, kept for those who
    used it.
- **A template is a starting point only.** The project does not store which
  template it came from. Everything a template made is ordinary: statuses,
  limits, views, areas and the sprint can be changed or removed like
  anything the manager made by hand, and changing the templates in a later
  version changes no project that exists.
- **What a template makes keeps the identifiers that follow from the
  project.** The first area takes the identifier of the starting area, so a
  task without an area still lands in the first one; a view of the six
  first kinds takes the identifier of its kind.
- **The interface reads the templates through the Backend**
  (`list_project_templates`). The chooser of a new project lists them, each
  with a small picture of its lanes drawn from its own statuses, colors and
  limits, and says what the chosen one is for and starts with. The demo
  makes projects from `apps/desktop/src/shared/api/projectTemplates.json`, a
  copy of the templates that a test of the desktop shell keeps equal to the
  domain; `TEAMWRIGHT_WRITE_TEMPLATES=1` writes it anew.

## Consequences

- A manager who sets no limit sees the board as before, with a plain count
  in every lane.
- A board shows a limit in every lane of a status that has one, also in a
  view filtered to one person or in an area. The figure may then be larger
  than the cards the lane shows; the tooltip says it counts the whole
  project.
- The count of a lane comes from the backend, so it changes when the move
  of a card has been stored, not while the card is dragged.
- An older version of the program that takes in a project from a newer one
  passes over the limits, and writes the workflow without them when it
  changes it. Every computer should run the same version.
- The wire name of the template stays `workflow` in `NewProjectInput`.
- Nothing has been checked in the built program; the demo backend, the
  tests at each layer and the pictures of the interface in a browser have.

## Alternatives considered

- **Refuse a move into a full lane.** What some tools do. It would stop a
  specialist in the middle of handing on its work, and the manager could not
  move a card to see what happens. A signal does the job without the harm.
- **Count only what the open view shows.** The figure would then match the
  cards, but the same lane would be full in one view and empty in another,
  and a filter could hide that the team has too much under way.
- **Count subtasks too.** Closer to "everything in the status", but the
  board shows them under their task, and a task with five subtasks would
  fill a lane by itself.
- **Light the lane that is over.** It would draw the eye, but it spends the
  one light that means "this waits for you" on a state that may be wanted.
- **Remember the template of a project**, to offer updates of it later.
  It would tie a project to a version of a list, and nothing needs it: the
  manager owns what the template made.
- **Templates as files the manager writes.** Flexible, but they need a
  place to live and a format to keep; a small set in code is enough to
  start, and the manager can change everything afterwards.
