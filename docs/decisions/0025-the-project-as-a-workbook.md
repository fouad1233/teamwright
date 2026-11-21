# 0025. Give the manager the project as an Excel workbook, gathered in Rust and written by a crate of its own

- Status: accepted
- Date: 2026-11-15

## Context

The owner asked for "building excel": the manager must be able to take the
open project out of Teamwright as a file that opens in Excel, LibreOffice
and Google Sheets, to send it to somebody, to keep a record of it, or to
work on its figures. A file somebody else opens is read without the
program beside it, so it has to be complete and look made with care.

What the file holds are facts the program already keeps: the project, its
tasks, its sprints and its team. Some of the figures it should show are
worked out today, and the dashboard works out its numbers in the
interface ([decision 0020](0020-a-dashboard-worked-out-not-written.md)).

## Decision

- **"Export to Excel..." in the menu of a project.** The save window of
  the operating system opens with `<Project name>.xlsx`. When the file is
  written the program says so with its path and offers to show it in its
  folder; when it is not, it says why in plain words: the file is open in
  another program, the disk is full, the folder is gone.
- **Four sheets: Summary, Tasks, Sprints, Team.** Real types everywhere:
  dates are dates shown as `13 Nov 2026`, numbers are numbers, hours have
  one decimal. A list has its header frozen and a filter on it; a subtask
  comes under its task, set in. The look is quiet: one typeface, a header
  in bold on a faint fill, thin lines, and no colour but the status of a
  task, a deep shade of its colour on a pale one.
- **What the workbook holds is gathered in `application`, as plain data.**
  `WorkbookService::workbook_of` returns sheets, tables, rows and cells,
  each cell with its kind (text, whole number, hours, date, status). It is
  tested on a project in memory. It follows the category of a status,
  never its name, and the rules the rest of the program already has in
  Rust: finished means the category done, and a sprint counts its work as
  its report does.
- **A figure whose rule lives only in the interface is left out.** The
  dashboard counts tasks in work, late and given to nobody in
  `dashboardModel.ts`. Writing those rules a second time in Rust would let
  the two disagree one day. The workbook shows what Rust knows: the tasks
  open and finished, the tasks of every status, and the running sprint
  with the days it has left and its work. Moving the dashboard's counting
  into Rust is a change of its own.
- **The file is written by a small crate of its own, `crates/workbook`.**
  Writing a file is input and output, so it is not in `domain` or
  `application`. It is not in the desktop shell either: on its own it can
  be built and tested quickly, reading its own files back with `calamine`,
  without the shell and its windows. It uses `rust_xlsxwriter`, which
  writes `.xlsx` in pure Rust; no Excel is needed. The file is made in
  memory first and then written in one go, so a failure leaves an old file
  as it was.
- **The program shows only a file it wrote.** Showing the file in its
  folder takes a path from the interface, and the shell shows it only if
  it wrote that path in this session, so the interface can never make the
  program open a place the manager did not choose.

## Consequences

- The manager gets a file that sorts, filters and sums as a spreadsheet
  should, without any program of Teamwright's beside it.
- The summary says less than the dashboard: it has no count of the tasks
  in work, late, or given to nobody until those rules move into Rust.
- The wire contract has three new commands: `export_workbook`,
  `show_exported_workbook`, and the save window of the dialog plugin,
  which the shell's capabilities now allow.
- The demo in the browser has no files; there the entry says that, as
  the other actions that need the computer do.
- A new dependency, `rust_xlsxwriter`, with `calamine` for the tests only.

## Alternatives considered

- **CSV.** Every program opens it, but it has no types and no sheets: a
  date is text that each program reads in its own way, and four lists
  would be four files.
- **Writing the file in the interface with a JavaScript library.** It
  would put the rules of what the workbook holds in the interface, where
  rules do not belong, and add a dependency of the interface that is
  larger than the Rust crate.
- **Writing the file in the desktop shell.** Possible, but the shell is
  slow to build and has no tests of its own for such work; a crate of its
  own keeps the writer next to its tests.
- **Formulas in the workbook for the missing figures** (late tasks counted
  by Excel against `TODAY()`). It would write the rule a third time, in a
  third language.

## Note of 21 November 2026: the file is written without following a link, after a review

The workbook was written with a plain write, which follows a symbolic link
at the name. The work folder of a project, where a workbook may be saved, is
one a specialist can write in. The workbook is now written the way the report
of a project is ([decision 0042](0042-the-report-of-a-whole-project.md)):
through a temporary file of a unique name in the same folder that is renamed
to the wanted name, with a link at the name refused. A half-written workbook
is never left either.
