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

## M3 · The Advisor ⬜

From a conversation to an approved team and plan.

- ⬜ Interview with structured, clickable questions
- ⬜ Project brief
- ⬜ Staffing proposal with a reason, model and effort level per member
- ⬜ Plan proposal: tasks, owners, dependencies, estimates
- ⬜ Review, edit and approve; apply the approved plan to the board
- ⬜ Import earlier Claude Code conversations as context digests

## M4 · Teamwork 🚧

Several specialists working together with the manager in the loop.

- 🚧 Team bus: the tools specialists use to talk to the project. It has its
  first tool, with which a specialist asks the manager to decide
- ✅ Decision requests and the manager's inbox
- ⬜ Teammate messages, handoffs and registered artifacts
- ⬜ Project memory: decisions, facts and interface contracts
- ⬜ Coordinator: dependency-driven start, concurrency limit, token budgets
- ⬜ Permission requests for risky actions routed to the manager

## M5 · Seeing the work 🚧

- ✅ Gantt chart with dependencies; bars are dragged to change the dates
- ✅ Calendar by month
- ⬜ Planned against actual on the Gantt chart
- ⬜ The office: specialists as characters with activity, time left and usage
- ⬜ Dashboards: usage per teammate, plan limits, workload by status
- ⬜ Project report at completion

## M6 · Agile ⬜

- ⬜ Backlog, sprints, story points
- ⬜ Burndown and velocity
- ⬜ Work-in-progress limits, custom fields, tags, checklists
- ⬜ Templates: Scrum, Kanban, hardware product

## M7 · Many computers ⬜

- ⬜ Sync through a folder the user owns
- ⬜ Conflict handling
- ⬜ View a project that is running on another computer

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
