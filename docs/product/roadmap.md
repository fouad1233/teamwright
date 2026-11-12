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

## M1 · Walking skeleton 🚧

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
  given to. They cannot work yet
- ✅ Screenshots of the real application, taken by a tool
- ✅ Subtasks shown under their task on the board and in the list, behind a
  fold
- ⬜ Editing the statuses of a workflow
- ⬜ Activity history of a task
- ⬜ Renaming and archiving a project from the interface

## M2 · One teammate ⬜

A single specialist that really works through Claude Code.

- ⬜ Find the installed Claude Code and guide the user through sign-in
- ⬜ Start, stop and resume a session; stream its activity live
- ⬜ Capture token usage per turn and reported plan limits
- ⬜ Specialist profiles: built-in set and user-defined
- ⬜ Discover the user's skills, plugins and MCP connectors
- ⬜ Chat with a teammate; let it carry out a task that was given to it

## M3 · The Advisor ⬜

From a conversation to an approved team and plan.

- ⬜ Interview with structured, clickable questions
- ⬜ Project brief
- ⬜ Staffing proposal with a reason, model and effort level per member
- ⬜ Plan proposal: tasks, owners, dependencies, estimates
- ⬜ Review, edit and approve; apply the approved plan to the board
- ⬜ Import earlier Claude Code conversations as context digests

## M4 · Teamwork ⬜

Several specialists working together with the manager in the loop.

- ⬜ Team bus: the tools specialists use to talk to the project
- ⬜ Decision requests and the manager's inbox
- ⬜ Teammate messages, handoffs and registered artifacts
- ⬜ Project memory: decisions, facts and interface contracts
- ⬜ Coordinator: dependency-driven start, concurrency limit, token budgets
- ⬜ Permission requests for risky actions routed to the manager

## M5 · Seeing the work ⬜

- ⬜ Timeline (Gantt) with dependencies, planned against actual
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
- Calendar view, whiteboards and documents inside projects
- Automation rules
- More agent runtimes than Claude Code
