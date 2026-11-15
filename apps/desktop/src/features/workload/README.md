# The workload

A kind of view (`workload`) that shows how much lies on each person, week by
week. It is found by the table of kinds through `kind.tsx`. The arithmetic
is in `workloadModel.ts`, a set of pure functions with tests.

- **Rows are people.** The manager first, then every member of the team in
  the order they joined. A row for the tasks given to nobody, and one for
  tasks given to someone who has left, follow only when they have work.
- **Columns are weeks.** Eight of them from the Monday of this week; a
  control in the corner shows the days of four weeks instead. On a
  Saturday or a Sunday the table begins with the next week, since no work
  is left in this one.
- **An estimate is spread evenly over the working days** of its task, from
  its start (or from today, if it started earlier or has no start) to its
  due date. A task that is late lies whole on the first working day from
  today: work that is not finished cannot be done in the past. Saturdays
  and Sundays carry nothing, unless a task lies on nothing else.
- **What is counted.** Finished tasks lie on nobody. A task whose parts
  carry estimates of their own is counted through its parts, so that the
  same work is not counted twice. A task without an estimate or without a
  due date cannot be laid on the days; a line under the table counts them
  and lists them on a press.
- **Capacity.** The manager, and a person on the team, can work 6 hours a
  day until the manager changes it in place under their name. A cell's
  capacity is those hours times the working days of the cell that are not
  over. Under each figure a hairline bar shows the hours against the
  capacity and turns to the colour of danger above it. An AI specialist has
  no working day: its bar is measured against the fullest cell among the
  specialists, and a line under the table says so once.
- **A press on a cell** lists the tasks behind it under the table: how much
  of each falls into the cell, of how much in all, and its dates. A press
  on a line opens the task.
- **What the view keeps** in its content: the scale (`weeks` or `days`)
  and the hours a day of each person, by the key of their row. Content
  that cannot be read falls back to the defaults. A change is written about
  a second after the last one, and at once when the view is left
  (`useWorkloadContent.ts`).
