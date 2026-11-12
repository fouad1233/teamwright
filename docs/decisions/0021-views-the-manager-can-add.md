# 0021. Let the manager add, name, filter, order and remove the views of a project

- Status: accepted
- Date: 2026-11-12

## Context

A project had a fixed row of eight tabs: Board, Backlog, List, Gantt,
Calendar, Team, Office and Advisor. The owner of the project asked for
what the tool they use for their own work offers: a project has some views
from the start, and more can be added, such as a second board or another
list.

That tool was studied. Worth taking: every project has a row of views; a
view is a kind (board, list and so on) with a name and settings of its
own; some are there from the start; the manager adds more from a short
chooser, names them, orders them and removes them; a row that is full
puts the rest behind "N more". Not worth taking: a store of dozens of
kinds, private and protected views (Teamwright has one manager), and a
panel of twenty settings for each view.

## Decision

- **Views and places.** The tabs of a project split in two. A *view* shows
  the tasks of the project and belongs to the manager: it can be added,
  named, ordered, filtered and removed. A *place* is about the project
  itself and is always there: Team, Office and Advisor. In the row the
  views come first, then a quiet divider, then the places. A view tab
  carries the mark of its kind; a place has none.
- **Kinds.** A view is of one kind: board, backlog, list, gantt, calendar
  or dashboard. A new project starts with one view of each, in that
  order, each named after its kind. A project can have several boards,
  lists, Gantt charts and calendars, and at most one backlog and one
  dashboard: those two can be removed and added again, not doubled.
- **A view has a filter of its own**, so that a second board means
  something: whose tasks it shows (everybody's, the manager's, one
  member's, or nobody's) and whether finished tasks are hidden. Finished
  means a status of the category done, never a status by its name. Only
  a board, a list, a Gantt chart and a calendar can be filtered in this
  version. A task is shown if it passes the filter or one of its subtasks
  does; its subtasks are shown with it.
- **The filter is applied where the tasks are handed to a view**, so the
  views themselves stay as they are. The board is the one exception: it
  keeps every task and is told which get a card, because it places a
  dropped card among all the tasks of a status and counts the tasks a
  card waits for.
- **Views are stored with the project**, not in the window. They are
  data that is to travel with the project to the manager's other
  computers. Which view is open stays in the window, as before.
- **The rules are use cases.** Each refusal has a stable code.
  - A name is trimmed and put on one line, is not empty, and has at most
    40 characters (`rule_broken`). No two views of a project have the
    same name, whatever the capitals (`view_name_taken`). A view added
    without a name gets the name of its kind, numbered if that is taken:
    "Board 2".
  - A second backlog or dashboard is refused (`view_kind_taken`).
  - A backlog or a dashboard cannot be filtered (`rule_broken`).
  - A filter names only a member of the team of the project
    (`member_not_on_team`).
  - The last view of a project cannot be removed (`last_view`).
  - A view or project that does not exist is `not_found`.
  - Deleting a project deletes its views. Removing a member from the team
    sets every view that showed only that member's tasks back to
    everybody's, and keeps whether it hides finished tasks.
- **The interface draws the views from one table of kinds**: the kind,
  its label, what it is for, its mark and what draws it. A new kind is
  one entry.

## Consequences

- A manager who never touches the row loses nothing: the six views are
  there from the start, in the order the tabs had.
- The demo backend and `demoViews.test.ts` are the specification. The
  Rust side follows them case by case: a table of views with the
  project, the kind, the name, the place in the row and the filter; the
  use cases; the six commands of the shell. A database that has projects
  from before must give each of them the six views. Until then the built
  program cannot list views, so it shows none, and the tools that drive
  it by its tabs fail.
- The filter is the only setting of a view. Grouping, sorting and the
  columns of a list stay as each view has them.
- A new way of showing tasks is a kind of view, not a tab of its own. The
  dashboard of [decision 0020](0020-a-dashboard-worked-out-not-written.md),
  which was a fixed tab after Calendar, is now the kind `dashboard`. Its
  way to the backlog leads to the backlog view, and is left out when the
  project has removed it.

## Alternatives considered

- **Keep the views in the storage of the window.** Nothing to build in
  Rust, but a view would not follow the project to another computer, and
  a second board on one computer would be missing on the other.
- **Make everything a view, places included.** One kind of tab, but the
  team, the office and the Advisor would then be removable, and a
  project without its team makes no sense.
- **A settings panel for each view.** What the tool the owner uses has. A
  filter of two questions covers what a second board is for; more can be
  added when a need appears.
- **Filter inside each view.** Every view would have to learn about
  filters. Handing a view fewer tasks keeps them as they are; only the
  board needs to know what it does not show.
