# 0042. Report a whole project at any time, call it complete when every task is done, and show the latest activity from the log

- Status: accepted
- Date: 2026-11-21

## Context

The roadmap (M5) had "Project report at completion" as not started, and
the dashboard had no latest activity
([decision 0020](0020-a-dashboard-worked-out-not-written.md)). A manager
who ends a project wants one document that says what was to be built,
what was done and by whom, how the plan compared with what happened, what
the team used, what was decided and what was handed over, and keeps it
with the work.

A project could already be archived. The Report view
([decision 0024](0024-a-view-keeps-content-of-its-own.md)) already draws
one page about how a project stands this week, to print or save as PDF,
from the dashboard's figures. The Activity view already turns the log of
the project into lines.

## Decision

- **No new state "completed".** A project is complete when it has tasks
  and every one is in a status of the category done. That follows the
  category of a status, needs no migration, nothing to sync and no
  ceremony, and a project that reopens a task is simply not complete any
  more. Archiving stays the way to put a project away.
- **The report can be made at any time.** It is called "Report of the
  project" when the project is complete and "Report of the project so
  far" before. The dashboard offers it once every task is finished.
- **It is a second sheet of the Report view, not a new kind.** The view
  shows "Now" (the page that existed) or "Whole project", and keeps the
  choice in its content; content written before reads as "Now". The
  sheets share the paper, the print style sheet and the button that
  prints. Opening the report from the dashboard turns the project's
  Report view to the whole project, adding the view if there is none.
- **The report is gathered in `application`**
  (`ProjectReportService::report_of`) from what is stored, with tests,
  and the interface only draws it. Its summary and the plan against what
  happened are sentences worked out there, so that the page and the file
  say the same. Nothing is written by a model.
- **Only what the data can say.** The brief is the summary of the latest
  plan of the Advisor the manager approved. Time is the time the
  specialists' turns ran; an estimate is the size of the work for a
  skilled person, and the report says so beside the comparison. The work
  of people is not timed, and the report says that too. Due dates are met
  or missed by the day of finishing. Sprints keep the summary they were
  completed with. Nothing is extrapolated: the sprint charts work out the
  course of the work.
- **Kept as Markdown in the work folder.** `report_in_markdown` is a pure
  function; the save window opens in the work folder of the project
  ([decision 0013](0013-the-folders-of-a-project.md)) with
  `<Project> report <date>.md`, as the workbook asks where to save
  ([decision 0025](0025-the-project-as-a-workbook.md)). A part with
  nothing to say is left out, never an empty heading.
- **The latest activity is the log.** The dashboard shows the newest six
  entries made by the Activity view's own rule (`logEntries`) and worded
  by it (`entryInWords`, `whenInWords`), with the other computer named,
  and a way to the Activity view, or to add it.

## Consequences

- The latest activity knows what the log knows: what happened to the
  tasks. A specialist finishing a turn, an answer of the manager and a
  task planned into a sprint are not in the log, so they are not in the
  latest activity either. When the history of a project learns them, the
  Activity view and the dashboard both tell them, through one rule.
- The demo gathers the same report in `demo/projectReport.ts`, which must
  follow the rules of `project_report.rs`; its tests pin the sentences.
- The dashboard says whether the project is complete with its own counts;
  it is the same rule as the application's, said with numbers the
  dashboard already has.

## Alternatives considered

- **A state "completed" set by the manager.** A ceremony with a
  migration, syncing and a way back, for something the tasks already say.
- **A third kind of view for the report.** It would be a twin of the
  Report view, with its own printing and paper.
- **A summary written by the Advisor.** It costs tokens and can say what
  is not so ([decision 0020](0020-a-dashboard-worked-out-not-written.md)).
- **Merging the turns and answers into the latest activity on the
  dashboard alone.** It would be a second rule beside the Activity view's.
