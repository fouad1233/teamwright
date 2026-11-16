# Roadmap

Teamwright is built in milestones. Each one leaves the app in a state that can
be run and shown. The order follows the core loop: first a place for work to
live, then one agent, then the Advisor, then a team.

Status marks: ✅ done · 🚧 in progress · ⬜ not started.

## M0 · Foundations ✅

The ground everything else stands on.

- ✅ Repository, Apache-2.0 license, notice of independence from Anthropic
- ✅ Vision, landscape survey, decision records, the project story
- ✅ Architecture overview and contributing guide
- ✅ Design philosophy and key visual
- ✅ Development documented, including Windows without administrator rights
- ✅ Continuous integration: format, lint, test, build, on Linux and Windows

## M1 · Walking skeleton ✅

A desktop app that opens and manages projects and tasks without any AI.

- ✅ Rust workspace: domain model, application services, SQLite storage
- ✅ Operation log with device identity, as the base for sync
- ✅ Desktop shell (Tauri) with a React and TypeScript interface
- ✅ Projects with a workflow from a template
- ✅ Tasks: priority, assignee, dates, estimate, subtasks, dependencies
- ✅ Board with drag and drop, list view, task panel
- ✅ Demo backend, so the interface also runs in a plain browser
- ✅ Smoke test that drives the real application
- ✅ A team for every project: AI specialists and people that tasks can be
  given to
- ✅ Screenshots of the real application, taken by a tool
- ✅ Subtasks shown under their task on the board and in the list, behind a
  fold
- ✅ Editing the statuses of a workflow: add, rename, recolor, reorder,
  remove
- ✅ Renaming, archiving, restoring and deleting a project from the
  interface
- ✅ History of a task, read from the log of operations

## M2 · One teammate 🚧

A single specialist that really works through Claude Code.

- ✅ Find the installed Claude Code, say whether the user is signed in to
  it, and say what to do if not
- ✅ Start, stop and resume a session; show every step as it happens
- ✅ A specialist carries out a task that was given to it: it gets a brief,
  works in the folder of the project, reports, and can be answered
- ✅ Ask a specialist something that is not about a task
- ✅ Token usage of every turn
- ✅ What each agent has spent and how full its context is: the context a
  session held at its last request against the window of its model, read
  from what Claude Code reports, kept with the turn, and shown as a meter
  in the office, at the head of a conversation and in the Team place; the
  Usage of the dashboard is a table with a row for every member and the
  Advisor. The rule is in `application`. Seen in a browser on demo data
  and tested on recorded streams; the built program answered the command
  for a project in which nobody had worked. Not yet seen with a real
  specialist in the built program
- ✅ A specialist reads and writes files in the folder of its project and
  can do nothing else
- ✅ The folders of a project: a work folder the manager chooses, folders
  the specialists may read and not change, and single files copied in for
  reference
- 🚧 Open the session of a specialist in the Claude desktop app: built, not
  yet run against the real app
- ⬜ Run a Claude Code that npm installed. On Windows that is a script,
  and a turn on it is refused with the reason
- ⬜ The limits of the plan that the program reports
- ✅ The model and the effort of a specialist
- ⬜ Instructions of a specialist's own, and a set of ready-made specialists
- ⬜ Skills and connectors for a specialist, chosen by the manager. A
  session sees nothing of what the user set up in Claude Code for their own
  use

## M3 · The Advisor 🚧

From a conversation to an approved team and plan.

- ✅ Interview with structured, clickable questions
- ✅ Project brief: the project in the Advisor's words, for the manager to
  check
- ✅ Staffing proposal with a reason, model and effort level per member
- ✅ Plan proposal: tasks, owners, dependencies, estimates
- ✅ Review, edit and approve; apply the approved plan to the board
- ✅ A later round that adds to the team and the tasks that are there. The
  Advisor is told when its proposal was approved, and proposes nobody a
  second time. That it repeats no task rests on what it is told: the
  program cannot tell a task that says the same in other words
- ⬜ Change the title, the size or the dependencies of a proposed task
  before approving
- ⬜ Import earlier Claude Code conversations as context digests

## M4 · Teamwork 🚧

Several specialists working together with the manager in the loop.

- 🚧 Team bus: the tools specialists use to talk to the project. It has
  three: a specialist asks the manager to decide, and the Advisor asks its
  questions and proposes a team and a plan
- ✅ Decision requests and the manager's inbox
- ⬜ Teammate messages, handoffs and registered artifacts
- ⬜ Project memory: decisions, facts and interface contracts
- ⬜ Coordinator: dependency-driven start, concurrency limit, token budgets
- ⬜ Permission requests for risky actions routed to the manager

## M5 · Seeing the work 🚧

- ✅ Gantt chart with dependencies; bars are dragged to change the dates
- ✅ Calendar by month
- ✅ Light and motion where something happens: work under way, a decision
  that waits for the manager, a card that is new or carried
  ([decision 0015](../decisions/0015-light-where-something-happens.md))
- ⬜ Planned against actual on the Gantt chart
- ✅ The office: specialists as characters with activity, time left and usage.
  A workshop in plan with a bench for everybody and a lamp lit where work
  goes on; "time left" is answered with how long work has taken and the
  size of the task. Seen in the built program with a real specialist at
  work; a bench that waits for you there was seen on demo data only
  ([decision 0018](../decisions/0018-the-office-as-a-plan-of-a-workshop.md)).
  People in the room: a pixel character for every kind of role and a
  default, at a desk of its own, larger in a wide window, who works, waits
  with a hand raised, does small things when free and walks the aisles to
  a shared place; a portrait at the bench and in the Team view.
  Seen in the built program with a real specialist at work at its desk;
  walking, waiting and the small things were seen in a browser on demo
  data only
  ([decision 0019](../decisions/0019-people-in-the-office.md))
- 🚧 Dashboards: usage per teammate, plan limits, workload by status. A
  Dashboard view of a project: where it stands in sentences worked out by
  rules, the manager's note, the numbers, work by status and by member,
  this week, the sprint and the tokens used
  ([decision 0020](../decisions/0020-a-dashboard-worked-out-not-written.md)).
  Seen in the built program on stored data, with nobody at work there.
  Plan limits are not shown, because they are not known, and there is no
  latest activity yet
- ✅ Views the manager adds, names, filters, orders and removes, next to
  the fixed places Team, Office and Advisor
  ([decision 0021](../decisions/0021-views-the-manager-can-add.md)).
  In the built program a view was added with the "+", named, and filtered
  to one member through its menu; the views are there after a restart and
  travel to another computer. Dragging a tab to another place and the
  "N more" of a narrow window were seen on demo data only. A view cannot
  be grouped by anything but status yet
- ✅ Every kind of view the tracker the owner uses has, asked for on
  15 November 2026, but for the map, which the owner said is not needed.
  A view may keep content of its own
  ([decision 0024](../decisions/0024-a-view-keeps-content-of-its-own.md)),
  and the interface finds a kind by its folder, so a kind is one more
  folder. Fourteen kinds were added to the six: Timeline (a row for every
  person along the days), Table (every field a column, changed in
  place), Workload (the hours on each person week by week), People (the
  kind `team`: a column for every person), Activity (the log of the
  project), Mind map (the project as a tree that folds), Gallery (cards
  with a picture the view keeps), Doc (a page of Markdown), Whiteboard
  (notes, boxes and arrows on a free ground; a note can become a task),
  Form (a sheet that makes a task), Embed (a page from elsewhere in a
  sandboxed frame), Chat (the talk with the specialists that is not
  about a task), Report (one page about the project, to print or save as
  PDF) and Files (what lies in the folders of the project). In the built
  program, on real storage, a view of every one of the fourteen kinds
  was added and opened without an error, and a page written in the Doc
  view was kept and drawn again after the window was loaded anew. Each
  view was looked at closely only in a browser on demo data. Not tried
  in the built program: dropping a picture on a card of the Gallery, a
  page in the frame of the Embed view, printing the Report, opening a
  file from the Files view, and content that arrives from another
  computer while a view is open. The model still knows the kind `map`;
  it has no interface
- ✅ A project as an Excel workbook, from the menu of the project
  ([decision 0025](../decisions/0025-the-project-as-a-workbook.md)):
  the sheets Summary, Tasks, Sprints and Team. A file the program's own
  code wrote was opened in Excel, and the built program wrote one through
  its command; the dialog that asks where to save was not tried
- ✅ Notifications of the operating system, off until the manager allows
  them ([decision 0026](../decisions/0026-notifications.md)): when a
  specialist or the Advisor waits for the manager, when work is ready,
  and when somebody could not go on, while the window is not in front.
  The program asks once, in the inbox; the choice and a switch for each
  kind are on the page Computers, with a test notification. What is
  worth telling is a rule in `application`, fed by a small module that
  says what happened in a project. In the built program the choice was
  made and kept and the test notification was sent without a refusal;
  nobody has seen a notification on the screen, and none was pressed
- ✅ Three integrations, set for a project on one computer
  ([decision 0027](../decisions/0027-integrations.md)). GitHub, through
  the manager's own `gh`, so that no token is kept: a task knows the
  pull requests and issues that name its key, and an issue can be opened
  for it. A Slack channel is told what happens, one plain line for each
  chosen event. Up to five web addresses get every chosen event as
  JSON, signed with a secret if one is given, which is how Zapier and
  Make listen ([the shape](../integrations/webhooks.md)). The pull
  requests of a real repository were read with the real `gh`; sending
  was tried against a listener on the same computer only. No real Slack
  channel and no real listener on the internet has received anything,
  and no real issue was opened
- ✅ Areas: a project cut into the jobs inside it, shown at the side
  under the project, each showing only its tasks in every view
  ([decision 0022](../decisions/0022-areas-inside-a-project.md)).
  In the built program areas were added, renamed and ordered, a task was
  moved with its subtask, and an area was opened on the board; they are
  there after a restart and travel to another computer. The dialog of
  the areas and the field in the panel of a task were seen on demo data
  only. Inside an area the sentence about the sprint and the burndown
  still count the whole sprint, and folders that group projects are not
  built
- ⬜ Project report at completion

## M6 · Agile 🚧

- ✅ Backlog, sprints, story points
  ([decision 0016](../decisions/0016-sprints-and-a-backlog.md)). A sprint
  is planned, filled, started and completed in the built program, and is
  there after a restart. The history of a task does not list its sprint or
  its points yet, and a row of the backlog is dragged with the pointer
  only: with the keyboard its menu moves it
- 🚧 Burndown of a sprint, and the completed sprints side by side. The
  burndown does not yet follow tasks that join or leave a sprint while it
  runs. Velocity as a forecast is not planned yet
- ✅ A completed sprint opens in the Backlog view, with its burndown and
  its tasks, and the board can show a previous sprint; both seen in the
  built program. Which tasks moved on when a sprint was completed is not
  recorded, and on the board the tasks of a previous sprint all stand in
  the last lanes, after empty ones
- ⬜ Work-in-progress limits, custom fields, tags, checklists
- ⬜ Templates: Scrum, Kanban, hardware product

## M7 · Many computers 🚧

- 🚧 Sync through a folder the user owns
  ([decision 0017](../decisions/0017-sync-through-a-folder.md)): a
  computer has a name, writes what it did into the folder, and takes in
  what computers it accepted did, every 20 seconds or when asked. The
  place Computers and a line at the foot of the navigation show how it
  stands. Run with two copies of the built program on one computer
  through one folder: a computer is taken in with its button, the line at
  the foot follows, and the window shows what the other computer did
  without being loaded again. The troubles (a folder that cannot be
  reached, two computers under one identity) were seen on demo data only.
  Two real computers with a real cloud drive between them were never
  tried
- 🚧 Conflict handling: two changes to different fields of one thing both
  hold; a deletion is final, and the computer whose change was dropped is
  told in a notice; a task number or a project key used twice and two
  running sprints are repaired. Seen with two copies of the program for a
  task; the repairs only in tests. The history of a task names the
  computer that made each change, when it was another one
- 🚧 View a project that is running on another computer: a conversation
  knows the computer it runs on, is read there and continued only on that
  computer, which is named where the reply box would be. Seen on demo
  data only: it needs a real specialist on a second computer

## M8 · Release ⬜

- ⬜ Installer and updates
- ⬜ User manual
- ⬜ Public demo in the browser
- ✅ A website on GitHub Pages that explains the project, with the film
  run by scrolling ([decision 0023](../decisions/0023-a-website-without-a-framework.md)):
  <https://fouad1233.github.io/teamwright/>, published on 14 November
  2026 with the film of 107 seconds. Seen from GitHub in a headless Edge
  at a desktop and a phone size; not seen in other browsers or on a real
  phone, and not yet known to any search engine

## Later

Ideas that are recorded but not scheduled:

- People on the team who use Teamwright themselves, from their own
  computers. Today a person on a team is a name that tasks can be given to
- Voice conversation with the Advisor
- Automation rules
- More agent runtimes than Claude Code
