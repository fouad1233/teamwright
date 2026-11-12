# 0020. Work the dashboard out by rules, and have nothing written for it

- Status: accepted
- Date: 2026-11-12

## Context

The owner asked for a place that says how a project is going: "dashboards
that can give you some insights about the project and how is it going, or
some summary that you can update, or some little summary about the
project", and "it should be somehow easy to follow". The roadmap names it
under M5: usage per teammate, plan limits, workload by status.

The project dashboard of a commercial project tool was studied for it.
What is worth taking is the order of things: a summary in words at the
top, a few plain numbers, then small charts that each answer one
question, and lists of what is due. What is not worth taking is a wall of
cards that each need settings, and a summary written by a model that
costs a call every time the page opens.

Everything a summary needs is already known to the interface: the tasks
and the categories of their statuses, their due dates, who they are given
to, the sprints and the report of the running one, the conversations with
their turns and their tokens, and what waits for the manager.

## Decision

- **The dashboard is a tab of a project, after Calendar.** One page, read
  from top to bottom, with nothing to set up.
- **The summary is worked out by rules, not written by a model.** Two to
  five short sentences say how much is finished and in work, who is at
  work, how many tasks are late, how the running sprint stands, and how
  many things wait for the manager. A sentence that would have nothing to
  say is left out. The rules are pure functions in
  `features/dashboard/dashboardModel.ts`, tested sentence by sentence.
  Opening the page costs no tokens and can be done as often as the manager
  likes.
- **The manager's own words are the description of the project,** shown
  beside the summary as the note the manager keeps and edited in place.
- **Below the summary come the parts, in this order:** the tasks at risk
  by name (not finished and past their due date), six plain numbers, the
  work by status as one bar, the work by member, this week, the sprint,
  and the tokens used. Every number a chart shows is also written next to
  it.
- **Logic follows the category of a status.** Finished means a status of
  the category done; in work means active or in review.
- **Light stays where something waits for the manager.** The number of
  things that wait is held by the gold corners of a waiting card while it
  is not zero, and leads to the inbox. Nothing else on the page is lit
  ([decision 0015](0015-light-where-something-happens.md)).
- **What is not known is not shown.** There is no latest activity of the
  project, because no command lists it yet; nothing about the limits of
  the manager's plan, which Teamwright does not know; and no time left for
  anything, which it would have to guess.

## Consequences

- The dashboard shows only what the interface already has. It needs no
  new command and nothing in the Rust part changed for it.
- The sentences are fixed. They say what the rules know and no more: they
  cannot say why something is late or what to do about it.
- The parts of the page reuse what other views have: the burndown and the
  completed sprints of the Backlog view, and the count of what waits and
  the words for tokens of the office.
- A project without tasks, without sprints or without a team says what is
  missing in one quiet sentence and leaves those parts out.

## Alternatives considered

- **A summary written by the Advisor every time the page opens.** It would
  read better and could say why, but it costs tokens on every visit, it is
  slow, and it can say something that is not so. A summary written by the
  Advisor on request, stored and dated, can come later next to the one
  worked out by rules.
- **Cards the manager arranges and sets up.** Flexible, but every card
  then needs settings, and a page that has to be built first is not easy
  to follow. One fixed order is enough for now.
- **Pie charts and a legend.** A pie is hard to compare by eye and its
  legend lies far from its marks. One bar in the order of the workflow,
  with the counts written beside it, says the same plainly.
