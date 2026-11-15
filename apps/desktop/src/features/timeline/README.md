# The timeline

A kind of view (`timeline`) that shows who works on what, along the days. It
is found by the table of kinds through `kind.tsx`.

- **Rows are people.** The manager first, then every member of the team in
  the order they joined, then the tasks given to nobody; that last row is
  left out when there are none. A small control in the corner groups the
  rows by status (every status, in the order of the board) or by area
  instead. A task whose row is gone, such as one given to a member who has
  left, goes in a last row of its own, so that no task is lost. The choice
  of grouping lasts while the view is open; it is not kept.
- **The scale is the Gantt chart's.** `DayScale.tsx` draws the months, the
  days, the weeks and today with the arithmetic of `gantt/ganttModel.ts`
  and the classes of `gantt/GanttView.module.css`, so both look the same.
  The chart could draw its own head with `DayScale` too; it does not yet,
  because this view was built without changing the chart.
- **Bars share a row.** Bars that would overlap in time lie on lines of
  their own (`layoutLanes` in `timelineModel.ts`). A task with only a start
  or only a due date is a mark on its day with its key beside it, and keeps
  three days of its line free so that the key is not covered. There are no
  lines of dependencies: this view is about people, not about what waits
  for what.
- **Tasks without dates** are listed under the name of their row, three at
  a time, with a press to see the rest.
- **Changing dates** works as in the Gantt chart, through the same use
  case (`reschedule`) and the same arithmetic (`draggedDates`): drag a bar
  to move it, drag an end to change one date, or with the keyboard the
  arrow keys move it by a day and Shift with an arrow changes the due date.
  Enter or a press opens the task. While a bar is dragged its line stays
  where it is; the lines are laid out again once the new dates arrive.
