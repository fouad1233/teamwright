# 0016. Plan the work in sprints, with a backlog and story points

- Status: accepted
- Date: 2026-11-12

## Context

The [vision](../product/vision.md) promises that a manager can work in
Scrum: a backlog, sprints, points and a burndown chart. Milestone M6 of the
[roadmap](../product/roadmap.md) is where it belongs. The owner of the
project learned Scrum from two worked examples and asked for the feature to
follow them and Jira. Both teach the same course: all the work is written
down in a backlog, the most important part of it is taken into a sprint of
one to four weeks, the sprint is worked on a board, a burndown chart shows
what is left, and at the end the sprint is reviewed.

In Jira a sprint is created in the backlog and work is dragged into it.
Starting it asks for a name, a goal and two dates. Completing it asks where
the unfinished work goes. Only work in the last column of the board counts
as finished. Its reports are the burndown of one sprint and the velocity
across sprints.

Teamwright already has what a sprint is made of: tasks, subtasks, statuses
with a category, and a board.

## Decision

- **A sprint is a thing of its own, not a status.** A project has sprints.
  A sprint has a name, a goal that may be empty, a start date and an end
  date, which may both be missing while it is planned, and a state:
  planned, active or completed. It knows when it was started and when it
  was completed.
- **A task is in one sprint or in none.** A task in no sprint is in the
  backlog. A subtask goes where the task it is a part of goes, and cannot
  be planned by itself.
- **Story points are a second measure next to the estimate.** A task may
  have points: a whole number from 0 to 999. The estimate in hours stays
  what it is, the size of the work in the time of a skilled person.
- **Finished means a status of the category done**, never a status by its
  name, as everywhere else ([AGENTS.md](../../AGENTS.md)).
- **The rules are use cases.** Each refusal has a stable code.
  - Creating a sprint: the name is trimmed and has at most 60 characters.
    Left empty it is *Sprint N*, N being one more than the number of
    sprints the project has had. A start date is never after the end date.
  - A planned or an active sprint can be changed: name, goal, dates. A
    completed one cannot.
  - Only a planned sprint can be deleted. Its tasks go back to the backlog.
  - A task can be put into a planned or an active sprint, or back into the
    backlog, never into a completed sprint. Its subtasks go with it.
  - Starting a sprint takes its start and end date. It needs at least one
    task, and no other sprint of the project may be active.
  - Completing a sprint says where its unfinished tasks go: to the backlog
    or to a planned sprint of the same project. The finished tasks stay in
    the sprint as its record. The sprint keeps a summary of what was
    committed and what was completed: in points if any of its tasks had
    points, in tasks otherwise.
  - Deleting a project removes its sprints. Deleting a task takes it out
    of its sprint.
- **The burndown is a report the backend computes.** For each day from the
  start date to the end date it gives the work that remained at the end of
  that day, and nothing for the days that have not come. It is counted from
  the moments at which tasks were finished. Tasks that joined or left the
  sprint while it ran are not followed: the chart counts the tasks that are
  in the sprint now, as if they had been there from the first day.
- **The interface gets a Backlog view** after the Board: the active
  sprint, the planned sprints, the backlog, and the completed sprints at
  the foot. While a sprint is active, the board shows its tasks, and a
  switch shows all tasks again.

## Consequences

- A manager who does not want Scrum loses nothing: without sprints every
  task is in the backlog and the board shows them all, as before.
- The interface plans a task by dragging it or from a menu on its row, and
  shows only what the backend answers.
- Velocity can be seen without being computed: the completed sprints keep
  their summaries, and the view sets them side by side.
- The burndown is honest only while the scope of a sprint stays as it was
  when it started. A task added in the middle of a sprint raises the line
  from its first day. The manual says so.
- The demo backend and its tests are the specification. The Rust side
  follows: a table of sprints, a column for the sprint and one for the
  points of a task, the use cases, and the commands of the shell. Until
  then the program itself does not have sprints.

## Alternatives considered

- **Sprints as statuses or as a field of the board.** Nothing to build for
  the model, but a task would have to leave its status to enter a sprint,
  and a sprint could not have dates, a goal or a summary.
- **Several active sprints at once.** Jira allows it for teams that work in
  parallel. With one manager and one team it would make *the* sprint on
  the board ambiguous. It can be allowed later without changing the model.
- **Use the estimate in hours instead of points.** One number less to keep.
  But hours say how long, and points say how big compared with other work.
  A team of agents makes hours say even less about the time it takes, and
  the Advisor already gives hours as a size. Both are kept, and the
  summaries count points when there are any.
- **Follow every change of scope in the burndown.** It is what Jira does.
  It needs the history of which sprint a task was in, day by day, which
  the operation log can give later. This version leaves it out.

What this version leaves out, on purpose:

- Stories written by a template, *as a … I need … so that …*. A task has a
  title and a description, and a manager may write a story into them.
- Features or epics above the tasks. The subtask is the only level.
- Following changes of scope in the burndown, as above.
- Velocity as a forecast of how much the next sprint can take.
