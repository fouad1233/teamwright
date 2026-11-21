# 0034. Read what happened from the log, and tell the pace as a range

- Status: accepted
- Date: 2026-11-21

## Context

Three charts compare the plan with what happened: the burndown of a
sprint, which has to follow tasks that join or leave it while it runs
([decision 0016](0016-sprints-and-a-backlog.md) left that out); the pace
of the team, as a forecast for the sprints to come; and the Gantt chart,
which is to show, under the planned dates of a task, when work on it
really began and ended.

Each needs the past: which sprint a task was in on each day, when it first
went into a status of a started category, when it reached done, whether it
was opened again, and which tasks moved on when a sprint was completed.

The log of operations already holds the complete state of a task after
every write, in the order of the stamps, also for writes that came from
another computer ([decision 0017](0017-sync-through-a-folder.md)). The
history of a task is read from it already, and so are the tasks a project
has had, deleted ones included. A deletion is a row of its own, with a
stamp whose clock part is the moment it was made.

## Decision

- **Nothing new is stored.** No table and no migration. The scope of a
  sprint day by day, the tasks that moved on at its completion, and the
  actual dates of a task are all read off the versions of the tasks in the
  log (`versions_of`, `ids_ever_in_project`), and the moment a task was
  deleted off the stamp of its deletion (`deleted_at`).
- **What a sprint held at the end of a day** is the last version of each
  task stored by then that names the sprint and is not a subtask. The
  tasks it was started with count from its first day. Whether a task was
  finished is read off its versions by the end of that day only, so a task
  is never finished earlier than it was. Versions stored after the moment
  of completion do not count.
- **A task moved on** when its version stored at the very moment the sprint
  was completed names another sprint, or none. Completing a sprint stores
  the sprint and the moving tasks in one write with one moment, so the
  moments match exactly; the storage tests check it through the real
  database.
- **Work on a task began** when it first went into a status of the
  categories active, review or done, and **ended** when it reached done,
  if it is there now. A task opened again is not finished until it is done
  again, and is marked as opened again. A status the workflow no longer
  has counts as not started. Lateness is counted in days against the
  planned start and due day; a task without them is never late.
- **The pace of a team is a range, never one number:** the least and the
  most it finished in each of its last completed sprints, read from the
  summaries the sprints kept. Up to the six latest count, all in the unit
  of the latest one. With fewer than three it is too early, and the
  program says so. The work not finished becomes a range of sprints, the
  best pace giving the fewest; a sprint that finished nothing leaves the
  longest open. Tasks without points, while the work is counted in points,
  are named, never counted silently as nothing.
- These are rules of the domain and the application, worked out and
  tested, in the manner of [decision 0020](0020-a-dashboard-worked-out-not-written.md).
  Nothing is written by a model.

## Consequences

- Every database that has a log already has the past these charts need,
  from its first write. Nothing has to be migrated, and nothing travels
  between computers that did not travel before.
- Reading costs one read of the log for each task of a project, as the
  history of a project already does. For the projects Teamwright is made
  for, hundreds of tasks, that is fine; a project with many thousands of
  tasks would want a summary kept by day, which can be added later behind
  the same use cases.
- The moment a task was deleted is the wall clock of the computer that
  deleted it, as its stamp records it. A computer whose clock is far off
  puts the deletion on the wrong day.
- Sprints of very different lengths make the range of the pace wide. The
  range is not divided by the length of the sprints; it says what the team
  finished in a sprint as the sprints were.
- The Gantt chart reads the actual dates only while the manager has
  switched them on, and the view keeps that choice in its own content
  ([decision 0024](0024-a-view-keeps-content-of-its-own.md)).

## Alternatives considered

- **A table of the membership of sprints, or of the actual dates (a
  migration).** Faster to read, but it would have to be kept right by
  every write, merged between computers, and filled for the past from the
  log anyway. The log already is that record.
- **Keep which tasks moved on in the summary of the sprint.** It would
  change the stored form of a sprint and its sync, for a fact the log
  already gives exactly.
- **The average of the last sprints as the pace.** One number reads as a
  promise. Three sprints are too few for an average to mean much, and a
  range shows how much the pace varies.
- **Count the started moment from a status called "In progress".** Logic
  follows the category of a status, never its name.
