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

## Note of 15 November 2026: fourteen more kinds

The owner asked for every kind of view the tool they use has. The kinds
are now twenty: board, backlog, list, gantt, calendar and dashboard, as
above, and timeline, table, workload, team, activity, mind_map, map,
whiteboard, chat, doc, embed, form, gallery and report. The domain, the
storage, both halves of the wire and the demo backend know all twenty.

- A project still starts with the six views above. None of the newer
  kinds is given to a new project.
- Only the backlog and the dashboard are single. A project may have
  several views of every other kind.
- Whether a kind can be filtered is now a rule of its own, no longer
  "not single": the kinds that show some of the tasks can be (board,
  list, gantt, calendar, timeline, table, workload, team, mind_map, map
  and gallery); the others cannot (backlog, dashboard, activity,
  whiteboard, chat, doc, embed, form and report). The message of the
  refusal no longer says that such a view "shows every task", which is
  not true of a document.
- Each newer kind gets its interface in a commit of its own, in a folder
  of its own that the interface finds by where it lies
  (`features/<kind>/kind.tsx`). Until a kind has its folder, the chooser
  does not offer it, and a view of that kind that arrives from elsewhere
  is drawn as a board.
- A view of the kind `team` is named "People". A project already has a
  place called Team, where its members are kept, and two tabs with one
  name and two meanings would mislead. The word on the wire stays `team`.

## Note of 16 November 2026: the files of a project

The owner asked for every kind of view the tool they use has, and that
tool has a "collection" of files. In Teamwright a project already has
folders on this computer, where its specialists work, so a twenty-first
kind, `files` (named "Files"), shows what lies in them.

- It is not a set of tasks, so it cannot be filtered. A project may have
  several, and a new project does not start with one.
- What it shows is read from the disk each time; nothing of it is stored
  in the database, and nothing is synced. Its content keeps only what the
  manager chose: the folder last opened and whether hidden names show.
- A path that comes from the interface is checked before it is read or
  handed to the system: it stays inside the folder of the project, also
  after links are resolved. A link that leads outside is listed, not
  followed.
- The disk is not watched. The view reads again when it is shown and when
  the manager asks it to.

## Note of 21 November 2026: grouping a view by more than status

The consequences above said that grouping was not a setting of a view. It
is now, for the two kinds that draw groups.

- **The setting.** A board and a list keep a grouping: status (as before,
  and the default), assignee, priority, area, tag or sprint. It is stored
  with the view like its filter, in a column of its own (migration 0030),
  and it merges and syncs with it. Any other kind stays grouped by status
  and refuses another grouping (`rule_broken`).
- **The rule is in the domain** (`group_tasks`), and a use case gathers
  what it needs (`GroupingService`), so the interface draws groups and
  decides nothing. The groups come in a fixed order: statuses as in the
  workflow; the manager, then the members in the order of the team, then
  nobody; priorities from the highest; areas in their own order; tags by
  name, then "no tag"; sprints as listed, then the backlog. Only tasks that
  are not subtasks are grouped, and a subtask goes with its parent.
- **A task with several tags is shown under each of its tags.** The
  alternative, one group for each combination of tags, is unreadable, and
  a task under only its first tag would hide the others.
- **Empty groups.** A group that a card can be dropped on is returned even
  when it is empty: every status, assignee, priority, area, tag, "no tag",
  every sprint that is not over, and the backlog. A sprint that is over, and
  the group of an area that is gone, come only with tasks. A list leaves
  empty sections out; a board keeps its empty lanes, because they are
  where a card can go.
- **Dragging a card to another lane** changes the thing grouped by through
  the command that already changes it (assign, prioritize, move to an area,
  plan into a sprint or the backlog), so every rule and the log apply. On a
  tag lane the card gets the tag of the lane and loses the tag of the lane
  it came from; to "no tag" it only loses it. The simpler alternative,
  offering no drop between tag lanes, would make the tag board a picture;
  this one keeps a card's other tags. A card cannot be put in place within
  such a lane: the order there is the order of the statuses, then the ranks.
- **Limits belong to statuses** (decision 0033), so they show only when a
  board is grouped by status. In any other lane a card says its own status
  with its mark and name. The sprint switch of the board of statuses is also
  left out of a grouped board, because a sprint is a grouping of its own.
- **The list folds.** Its sections have a head that folds them, grouped by
  status or not. Which sections are folded is not kept.
