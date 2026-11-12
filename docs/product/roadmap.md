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
  stands. The Rust side was run with two copies of the built program on
  one computer through one folder; the interface was seen on demo data
  only. Two real computers with a real cloud drive between them were
  never tried
- 🚧 Conflict handling: two changes to different fields of one thing both
  hold; a deletion is final, and the computer whose change was dropped is
  told in a notice; a task number or a project key used twice and two
  running sprints are repaired. Seen with two copies of the program for a
  task; the repairs only in tests. The computer beside each version in
  the history of a task is not shown yet
- 🚧 View a project that is running on another computer: a conversation
  knows the computer it runs on, is read there and continued only on that
  computer, which is named where the reply box would be. Seen on demo
  data only: it needs a real specialist on a second computer

## M8 · Release ⬜

- ⬜ Installer and updates
- ⬜ User manual
- ⬜ Public demo in the browser

## Later

Ideas that are recorded but not scheduled:

- People on the team who use Teamwright themselves, from their own
  computers. Today a person on a team is a name that tasks can be given to
- Voice conversation with the Advisor
- Whiteboards and documents inside projects
- Automation rules
- More agent runtimes than Claude Code
