# The table view

One task a row, every field a column: Task, Title, Status, Assignee,
Priority, Points, Estimate, Start, Due, Sprint, Area and Waits for. Each
cell is changed where it stands, with the program's own controls: the
menus of the task panel, the calendar field, the task picker.

- **Rows.** Every row has the same height. A subtask stands under its task,
  set in one step for every task it is part of. Without a sort the rows are
  in the order of the board: by status in the workflow, then as arranged.
- **Sorting.** A press on a head sorts by that column; a second press turns
  the order. Empty cells go last either way, and equal ones keep the order
  of the board. Priority sorts from the most urgent first. Subtasks are
  sorted the same way among themselves and stay under their task.
- **Columns.** The "Columns" menu shows and hides all but the task and its
  title, which always show and stay at the left while the rest scrolls.
  Hiding the column the rows are sorted by drops the sort.
- **The foot** counts the rows and sums their points and their estimates
  in hours. Every row counts, subtasks included.
- **The keyboard.** The table has one place in the order of Tab: the cell
  the keyboard stands on, ringed in brass. The arrows move between cells,
  Home and End to the ends of a row (with Ctrl, of the table), Page Up and
  Page Down by ten rows. Enter (or F2) changes the cell: it types into the
  title, the points and the estimate, opens the calendar of a date and the
  menu or sheet of the others. In a typed cell Enter keeps what was typed
  and Escape gives it up; both give the keyboard back to the cell. Enter on
  the task's reference opens the task.
- **What it keeps.** The shown columns and the sort are the view's content
  (`{"version":1,"columns":[…],"sort":{"column":…,"direction":…}}`),
  written about a second after the last change and at once when the view
  is left. Content that cannot be read falls back to every column,
  unsorted. Which cell the keyboard stands on is not kept.

`tableModel.ts` holds the rules as pure functions, with its tests: the
columns, the reading and writing of the content, the order of the rows,
the sums and where a key leads.
