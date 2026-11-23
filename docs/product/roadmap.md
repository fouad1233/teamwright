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
- ✅ The same on Linux, through WebDriver under a display of its own
  (`tools/smoke/desktop-smoke-linux.mjs`, 76 checks at seventeen
  stations, run on 22 November 2026), and a test that opens an old
  database in a newer build (`tools/smoke/old-database-linux.mjs`): a
  database made by the program of the main branch, at format 17, opened
  in the build of the night of 21 November, went to format 29, and its
  project could be read and changed. No real specialist takes part in
  either: a stand-in for Claude Code fails every turn. Not covered:
  dragging, the save and folder windows, the print layout,
  notifications and syncing on Linux
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
  can do nothing else.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session was refused every read and write outside its folders
- ✅ The folders of a project: a work folder the manager chooses, folders
  the specialists may read and not change, and single files copied in for
  reference.
  The first real run on Linux, on 22 November 2026, found that a session
  could write into a folder it was given to read only: the rule that
  forbids it began with one slash, which Claude Code reads as a path
  inside the working folder, so it protected nothing on Linux and macOS.
  It had passed on Windows, and version 0.1.0 has the fault on the other
  two. The rule is now written as an absolute path, and a real session was
  refused in fourteen read-only folders with spaces, parentheses, brackets
  and stars in their names. Not run on macOS, and not run again on Windows
- 🚧 Open the session of a specialist in the Claude desktop app: built, not
  yet run against the real app
- ⬜ Run a Claude Code that npm installed. On Windows that is a script,
  and a turn on it is refused with the reason
- ✅ The limits of the plan that the program reports
  ([decision 0040](../decisions/0040-the-limit-of-the-plan.md)). A turn
  that meets the limit of the Claude plan ends as its own kind of
  ending, not as a failure of the specialist, and the manager is told
  once, with the time it resets when Claude Code gives it. A line at the
  foot of the navigation says so until it has reset. Tested on streams
  built from the program's own schema and from the words of a real
  limit met on 21 November 2026; never seen with a real run, and a reset
  time in a time zone other than this computer's or UTC stays unknown.
  With a real session of Claude Code 2.1.330 on 22 November 2026: the five
  hours and the week were reported by every turn and decoded as expected.
  A turn that really ends at the limit has still not been recorded
- ✅ The model and the effort of a specialist
- ✅ Instructions of a specialist's own, and a set of ready-made specialists
  ([decision 0031](../decisions/0031-instructions-and-ready-made-specialists.md)).
  A specialist's instructions reach every turn after Teamwright's own
  rules and never replace them. Twelve ready-made specialists, hardware as
  well as software, are added from a chooser in the Team place, and a
  member the Advisor proposes under one of their names starts with its
  instructions. Seen in a browser on demo data and tested at every layer.
  Not yet seen in the built program, and no real specialist has run with
  instructions: the test against the real Claude Code was written but
  could not run, because Claude Code on that night's computer was not
  signed in.
  Since then, in the built program on Linux (22 November 2026): a
  ready-made specialist was added, its instructions were changed, and
  both were there after a restart.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session that was given instructions of its own followed them
- 🚧 Skills and connectors for a specialist, chosen by the manager
  ([decision 0039](../decisions/0039-skills-and-connectors-chosen-by-the-manager.md)). A project
  keeps a library of skills (checked, and copied on each computer) and of
  connectors (no secret is kept, only the name of a variable; tools are
  given one by one), and a turn gets exactly what its specialist was
  given. Every turn of a specialist is now also started with settings
  that shut out the user's own hooks, skills and connectors, which the
  earlier flags did not promise. Tested on storage, on two databases
  taking in each other's changes, on the exact command line and in the
  demo. After the same review a connector that is a program is started
  by Teamwright's own launcher in an empty folder, with only the
  variables it names, turns start without the secrets of the library,
  and the copy of a skill is checked whole before every turn; the
  launcher was started by hand on Linux. Since 22 November a connector
  or a tool that arrives or changes through the sync folder is started
  only after the manager confirmed, on that computer, exactly what it
  starts (a fingerprint kept for each computer and never synced), and
  each connector starts in a fresh empty folder of its own; proven by
  tests with two databases syncing through a folder and seen in the
  demo. Never run against the real
  Claude Code: the tests for it were written and have not passed yet,
  the earlier tests against the real program must be run again because
  every turn changed, and nothing of it was run on Windows.
  Since then, in the built program on Linux (22 November 2026): the
  library opened, a connector whose program is a bare name was added and
  kept after a restart, and one with a relative path was refused; no skill
  was added there.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session got exactly its chosen skill and connector and nothing of the
  user's own setup, and a connector started through the launcher ran in a
  fresh empty folder and saw only the variable it was given

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
- ✅ Change a proposed task before approving: its title, size,
  description, priority, owner and what it waits for; add or remove
  tasks. Each change is stored and reaches the other computers, and the
  Advisor is told the plan as it was approved. Tested at each layer and
  seen in a browser on the demo's simulated Advisor; not tried in the
  built program or with a real Advisor. The Advisor's first version is
  not kept once it was changed, so there is no "undo my changes"
- ✅ Import earlier Claude Code conversations as context digests
  ([decision 0041](../decisions/0041-earlier-conversations-as-digests.md)):
  the conversations found on this computer, by folder, the folders of the
  project first; a digest made by rules (the requests, the files
  changed, the last answer, with what looks like a secret hidden) that
  the manager reads, corrects and keeps with the project; the Advisor
  gets the kept digests with its first turn. The reader rests on a format
  Claude Code does not publish; it read the real conversations of the
  development computer in a test, and the whole flow was seen in a
  browser on demo data. Not tried in the built program, and no real
  Advisor has read a digest yet.
  Since then, in the built program on Linux (22 November 2026): the list
  of the computer's conversations opened, and a digest was drafted, kept
  and found after a restart

## M4 · Teamwork 🚧

Several specialists working together with the manager in the loop.

- 🚧 Team bus: the tools specialists use to talk to the project. It has
  five: a specialist asks the manager to decide, hands the result of its
  task over and writes to a teammate, and the Advisor asks its questions
  and proposes a team and a plan
- ✅ Decision requests and the manager's inbox
- 🚧 Teammate messages, handoffs and registered artifacts
  ([decision 0029](../decisions/0029-handoffs-and-messages-between-specialists.md)):
  a specialist hands the result of a task over (a summary, an interface
  contract, files that must exist in the folder of the project) and
  writes a question or a note to a teammate. The latest handoff of every
  task a task builds on goes into its brief, and a message comes with the
  receiver's next turn. The manager reads all of it: the handoff in the
  panel of the task, the messages in the Team place. Tested with recorded
  tool calls and seen in a browser on demo data. Not yet seen with a real
  specialist (the test against the real Claude Code could not run: it
  was not signed in) or in the built program.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session published a handoff through the team bus and the decoder found
  it with its summary, contract and files. A message between two real
  specialists was not tried
- 🚧 Project memory: decisions, facts and interface contracts
  ([decision 0036](../decisions/0036-project-memory.md)). Answered
  decisions and the contracts of handoffs become entries by themselves,
  a specialist remembers facts (three in a turn at most) and searches
  memory through the team bus, the brief of a task carries what belongs
  to it, and the manager reads, searches and changes memory in the place
  Memory. Tested at every layer and seen on demo data; the built bus
  answered a search and a remembered fact over standard input. Never run
  with a real session of Claude Code.
  Since then, in the built program on Linux (22 November 2026): a fact was
  added by hand, found by search, shown in the place Memory and kept after
  a restart.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session searched the memory, answered from what it found, and remembered
  a fact
- 🚧 Coordinator: dependency-driven start, concurrency limit, token budgets
  ([decision 0037](../decisions/0037-the-coordinator.md)). Off until the
  manager switches it on for a project; then it starts a task when what
  it waits for is done and its specialist is free, never more
  specialists at once than the limit, and stops starting work against a
  budget that is used up, asking the manager once in the inbox. A task
  that failed waits for the manager, and work that waits to be checked
  still needs the manager's accept. It runs on one computer, and the
  others show where. Tested at every layer (the rule on 34 cases, a
  small project that runs to its end, a budget that stops it) and seen
  working in a browser on the demo. While the limit of the Claude plan
  is reached it starts nothing and says why once; work the limit
  stopped goes on by itself after the reset, once for each reset
  (decision 0040). It has never started a real specialist and never ran
  in the built program.
  Since then, in the built program on Linux (22 November 2026): it was
  switched on with a budget and watched for a minute with a stand-in for
  Claude Code that fails every turn; it started the task once, said why
  the turn failed, waited for the manager and started nothing more
- 🚧 Permission requests for risky actions routed to the manager
  ([decision 0038](../decisions/0038-permission-requests-routed-to-the-manager.md)).
  A specialist asks through the team bus for one exact command or a web
  domain, and says why; the manager allows it once or for the task, a
  web domain also always, or refuses. The next turn is started with
  exactly that rule added. Standing permissions are listed in the Team
  place and can be taken back. A review on the night it was built found
  that one allowed command was any program run as the manager, and that
  prefixes, quotes and paths passed the checks. Since then: no prefixes,
  a command is checked as the shell reads it, never allowed always,
  never next to a connector's secret, never in a turn that only answers
  teammates, and never on Windows; a permission holds only where this
  computer's own log wrote the answer; every allowed command runs in the
  sandbox of Claude Code, and the turn fails if there is none. Tested at
  every layer and seen in a browser on the demo. A second review, of the
  fixes, found that a permission could be revived and moved through the
  sync folder, and that a command still saw the manager's whole
  environment. Since 22 November 2026 command permissions are off on
  each computer until the manager switches them on on the page
  Computers; when on, a command runs only in a turn the manager starts
  by hand, with a cleared environment; a permission grants only if every
  state of it was written on this computer; a web domain is one host.
  No real specialist has asked, the shape of the events was read from
  the documentation, and no command has run in the built program: the
  sandbox and the cleared environment have never been seen working, and
  until a signed-in run has shown them the switch should stay off.
  With a real session of Claude Code 2.1.330 on 22 November 2026: a
  session asked for a command and the request arrived exactly as it wrote
  it, with nothing run. The command, once allowed, did not run: the
  sandbox could not start on that computer, an Ubuntu 24.04 as installed,
  which refuses a user namespace inside another. Nothing ran outside a
  sandbox, so it failed closed, and the page Computers now says that such
  a computer cannot run commands. That the sandbox holds a command in has
  still never been seen

## M5 · Seeing the work 🚧

- ✅ Gantt chart with dependencies; bars are dragged to change the dates
- ✅ Calendar by month
- ✅ Light and motion where something happens: work under way, a decision
  that waits for the manager, a card that is new or carried
  ([decision 0015](../decisions/0015-light-where-something-happens.md))
- ✅ Planned against actual on the Gantt chart
  ([decision 0034](../decisions/0034-what-happened-is-read-from-the-log.md)):
  a switch the view keeps shows under each bar when work really began
  and ended, read from the log by the category of the statuses. A late
  start, a late finish and work that runs past its due date are told
  apart by hatching and by words, not by colour alone. Seen in a browser
  on demo data only.
  Since then, in the built program on Linux (22 November 2026): the switch
  was seen at work and is kept by the view after a restart
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
  The Usage part shows the Claude plan: whether its limit is reached,
  when it resets, and the five hours and the week as Claude Code reports
  them, otherwise the word unknown (decision 0040). The latest
  activity is the newest entries of the log of the project, as the
  Activity view tells them, which today means changes to tasks; seen on
  demo data and in the built program on Linux
- ✅ Views the manager adds, names, filters, orders and removes, next to
  the fixed places Team, Office and Advisor
  ([decision 0021](../decisions/0021-views-the-manager-can-add.md)).
  In the built program a view was added with the "+", named, and filtered
  to one member through its menu; the views are there after a restart and
  travel to another computer. Dragging a tab to another place and the
  "N more" of a narrow window were seen on demo data only. A board or a
  list can be grouped by status, assignee, priority, area, tag or sprint:
  the rule is in the domain, a card dragged to another lane changes that
  field through the existing commands, and the sections of a list fold.
  Tested in every layer and seen in a browser on demo data. A board
  grouped by assignee and one grouped by tag were seen in the built
  program on Linux and kept after a restart; dragging a card between
  lanes was not tried there
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
- ✅ Thirteen themes, chosen on the page Computers and kept for that computer
  ([decision 0028](../decisions/0028-themes.md),
  [the reasoning](../design/themes.md)). Lamplight, the look the program
  had, stays the default and follows the system. Five palettes are built
  for the eyes and for long work, each with what it does said plainly
  and its contrast ratios tested: Paper, Dusk, Slate, Clear and Even.
  Four special palettes carry a pattern drawn in code: Blossom, Wing,
  Fern and Tide. The window crosses from one theme to the next in 400
  milliseconds. Seen in a browser on demo data; not seen in the built
  program, where the crossing and the script that sets the theme before
  the first paint were never tried. Three of one colour on white were
  added on 22 November 2026, Rose, Delft and Poppy: light only, with no
  pattern, their contrast measured by tests, and in Poppy trouble keeps
  a darker red of its own. Seen in a browser on the demo; not tried in
  the built program
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
- ✅ Project report at completion
  ([decision 0042](../decisions/0042-the-report-of-a-whole-project.md)):
  the report of a whole project, made at any time and called complete
  when every task is in a done category. It is a second sheet of the
  Report view, printed like it and saved as Markdown into the work
  folder. Seen in a browser on demo data, including a finished project;
  not tried in the built program, and the print layout was not looked at.
  Since then, in the built program on Linux (22 November 2026): the sheet
  of the whole project was shown, and saved as Markdown into the work
  folder through the command the interface calls after the save window;
  the save window itself was not tried

## M6 · Agile 🚧

- ✅ Backlog, sprints, story points
  ([decision 0016](../decisions/0016-sprints-and-a-backlog.md)). A sprint
  is planned, filled, started and completed in the built program, and is
  there after a restart. The history of a task tells its sprint and its
  points, and a row of the backlog is dragged with the pointer
  only: with the keyboard its menu moves it
- ✅ Burndown of a sprint, and the completed sprints side by side. The
  burndown follows tasks that join or leave a running sprint, read from
  the log, and says each day of change in words. Velocity is told as a
  range once three sprints are completed, compared with each planned
  sprint, and turned into a range of sprints for the work ahead
  ([decision 0034](../decisions/0034-what-happened-is-read-from-the-log.md)).
  Tested in the domain, the application and on the real database; seen
  in a browser on demo data only
- ✅ A completed sprint opens in the Backlog view, with its burndown and
  its tasks, and the board can show a previous sprint; both seen in the
  built program. A completed sprint lists the tasks that moved on and where each
  went, read from the log (seen on demo data only), and on the board the tasks of a previous sprint all stand in
  the last lanes, after empty ones
- ✅ Tags and checklists
  ([decision 0030](../decisions/0030-tags-and-checklists.md)): the tags
  of a project on tasks, cards, the list, the table, the filter of a view
  and the workbook; one checklist for a task, with its progress on cards
  and rows. Both are stored and synced key by key, so two computers that
  change different tags or items of one task both keep their change.
  Tested in every layer, the taking in with two databases, and seen in a
  browser on demo data; not yet run in the built program. A task made in
  a view that is filtered by a tag does not get that tag yet.
  Since then, in the built program on Linux (22 November 2026): a tag made
  in place and a checklist with items ticked, on the card, in the panel,
  in the history and after a restart
- ✅ Work-in-progress limits
  ([decision 0033](../decisions/0033-work-in-progress-limits-and-templates.md)):
  a status can have a limit, set where the statuses are edited and shown
  at the head of its lane, in the red of trouble and marked "over" when
  it is exceeded. A limit never stops a task from moving. Tested in
  every layer and seen in the demo, not yet in the built program.
  Since then, in the built program on Linux (22 November 2026): a lane of
  a Kanban project showed that it was over its limit
- ✅ Custom fields
  ([decision 0035](../decisions/0035-custom-fields.md)): six kinds
  (text, a number with a unit, a date, yes or no, a choice from a
  coloured list, a link), made for a project in a dialog and set in the
  panel of a task. Each field is a column of the Table view and of the
  workbook, a view can be filtered by a choice or by yes or no, and the
  history says when a value changed. Synced value by value. Tested in
  every layer, the taking in with two databases, and seen in a browser
  on demo data; not yet run in the built program. A field's kind cannot
  be changed, and there is no kind for a person.
  Since then, in the built program on Linux (22 November 2026): a value of
  each of the six kinds, refused when of the wrong kind, in the panel, as
  columns of the table, in the history, in a filter and after a restart
- ✅ Templates: a new project starts from Standard, Kanban, Scrum,
  Hardware product or Engineering, which set its statuses and limits,
  its views, its areas and, for Scrum, a first sprint (decision 0033).
  Nothing remembers the template afterwards. Tested in every layer and
  seen in the demo, not yet in the built program.
  Since then, in the built program on Linux (22 November 2026): a project
  was made from each of the five, the Kanban one in the dialog

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
  tried. "Sync with Google Drive" finds the folder that Google Drive for
  desktop keeps on the computer and makes a folder for Teamwright in it
  in one press, with no sign-in to Google and no token; run in the built
  program against a pretended Drive folder, never with the real program
  of Google, which this computer does not have
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

- 🚧 Installers: a workflow builds them for Windows, macOS (Apple silicon
  and Intel) and Linux and publishes them as a release when a version is
  tagged. [Version 0.1.0](https://github.com/fouad1233/teamwright/releases/tag/v0.1.0)
  was published on 18 November 2026 as a pre-release, and
  [version 0.2.0](https://github.com/fouad1233/teamwright/releases/tag/v0.2.0)
  on 23 November 2026, with what was built from 21 to 23 November and
  the correction of the read-only folders on Linux and macOS. In that
  run the job for Macs with Apple silicon built its installer and then
  failed, because two jobs tried to create the release in the same
  second; it was run a second time and its files were added. The
  workflow still has that fault. The installers are
  not signed, so Windows and macOS warn before they open; none of them
  was installed and started by anybody yet, and the macOS and Linux
  builds were never run at all. The program tells the manager when a
  newer version is out
  ([decision 0043](../decisions/0043-a-notice-of-newer-versions.md)):
  off until switched on, at most one look a day, "Look now", and a quiet
  mark at the foot of the navigation. The rule, the reading of GitHub's
  list against a listener on the same computer and the interface on the
  demo were verified; and "Look now" in the built program on Linux asked GitHub once and
  found nothing newer than 0.1.0. Installing
  an update from inside the program is still not built: it needs signed
  update files and a signing key only the founder can make
- ✅ [User manual](../user-manual/README.md): a table of contents and
  fifteen pages, written from the decision records, the roadmap and the
  interface running on demo data, with eleven pictures from the demo.
  Every page says what was never tried in the built program or with a
  real specialist. Its links were checked. Nobody new to the program has
  tried to follow it, and three older pictures of the built program no
  longer match what it looks like
- ✅ Public demo in the browser
  ([decision 0032](../decisions/0032-a-public-demo-in-the-browser.md)):
  the interface, built for the browser (`npm run build:demo`), is
  published by the website's workflow under `demo/`, with a band that
  says it is a demo, a page for phones and a specialist at work when it
  opens. The commands of the workflow were run on a clean copy and the
  result clicked through in a headless Chrome at desktop and phone
  sizes. It was published on 23 November 2026 at
  <https://fouad1233.github.io/teamwright/demo/>: the workflow's first
  run on the main branch succeeded, and the published demo was opened
  once in a browser and loaded without an error. A real phone and
  browsers other than Chrome were not tried
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
