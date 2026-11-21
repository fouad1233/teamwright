# The activity

A kind of view (`activity`) that tells everything that happened to the tasks
of a project, the newest first, the way a log book is kept. It is found by
the table of kinds through `kind.tsx`.

- **What it tells.** The history of the project (`useProjectHistory`), which
  knows the tasks only: that a task was opened, renamed, described, moved to
  another status or given to somebody, and how its priority, its estimate,
  its dates and what it waits for changed. Sprints, views and members are
  not in it. Neither is who made a change, so the log does not say; when
  the history names the other computer a change was made on, the log says
  so after the sentence, as the history of a task does.
- **Days and entries.** Every day has a heading: "Today", "Yesterday", then
  the date. Under it stand the entries of the day: the time, the key of the
  task, and one sentence. A time is written once; the entries of the same
  minute that follow leave it out. Tasks that changed at the very same
  moment, such as the tasks of a plan that were opened together, follow
  their numbers.
- **One entry for one go.** The changes made to one task within one minute
  of the clock, on one computer, are one entry: the title of the task, and
  a part for each change in the order they happened. That is what filling
  in a new task in its panel looks like in the history, and five lines for
  it would bury the rest (`logEntries` in `activityModel.ts`).
- **The words are those of the history of a task.** A single change is the
  title of the task followed by `changeAfterTaskName`
  (`tasks/taskHistoryModel.ts`): "Draw the schematic moved from To do to In
  progress". The parts of an entry are `changeInWords`, as the panel of the
  task lists them. A change is worded once for both.
- **The key is the way to the task.** It opens the panel of the task, and
  the keyboard comes back to it when the panel closes. The key of the task
  the panel shows is set in ink. A task that was deleted since is named,
  with a note that says so, and its key leads nowhere.
- **Narrowing.** A control narrows the log to what was opened, what moved
  to another status, what was finished, or what was planned. Finished means
  that a task went into a status of the category done from one that is
  not, whatever the statuses are called. Planned is every change of dates,
  of an estimate, or of what a task waits for; a task that was opened with
  its dates is not listed, because its opening does not say so. The control
  is one stop for the keyboard, and the arrow keys go from key to key. The
  choice lasts while the view is open; it is not kept.
- **An open area.** While one area of the project is open, the log tells
  only what happened to its tasks. A deleted task is in no area any more
  and is left out.
- **Earlier.** The history comes in pages of fifty changes. "Earlier" at
  the foot reads the next one. The page stays where the reader had it, and
  the keyboard goes on from the first entry that arrived. Narrowed, the log
  reads on until an entry shows or the history is at its beginning, so that
  a press is never answered with nothing; a narrowing that shows nothing at
  first does the same by itself.
- **The reader's place is kept** (`useActivityLog.ts`). Whenever a task
  changes, the history is asked for again from its first page. Until it is
  read back as far as the reader had come, the log keeps what it showed. A
  page that came back short is asked on from at once: the history cuts a
  page short rather than split the changes of one moment, and only an empty
  page is its beginning (`isReadFarEnough`).

What this costs: narrowing is done here, on what is loaded, because the
history cannot be asked for one kind of change. A narrowing with few
entries in a long history reads many pages, and reads them again after
every change. A history that could be asked by kind would make that cheap.

**On the dashboard.** The latest activity of the dashboard is the newest
entries of this log, made by `logEntries` and said by `entryInWords` (one
sentence for an entry, its parts joined) and `whenInWords` ("Today,
14:02"). `useOpenActivity` opens this view from there, or adds it first
(decision 0042).
