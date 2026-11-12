# 0009. Give every project a team of its own, with agents and people as one kind of member

- Status: accepted
- Date: 2026-11-12

## Context

A task could be given to "a member", but nothing said what a member is. The
identifier existed without the thing it identifies.

The manager wants to give a task to a named specialist the way a task is
given to a person, and expects a team to hold both: AI agents, and later
people. The Advisor will propose a team for one project, the smallest set of
specialists that can do that project, with a reason for each of them. The
runtime will need a place for what belongs to one specialist on one project:
its instructions, its session, what it has used.

## Decision

- A member is on the team of exactly one project. The manager is not a
  member: the manager owns the project and is always there.
- An AI agent and a person are the same kind of thing, a `Member`, told
  apart by its kind. Everything that deals with assignees treats them alike.
- A member has a name that no other member of its team has, a sentence
  about what it is on the team for, and a color.
- The color is chosen by the application when the member joins, so that it
  differs from the others on the team, and it is stored.
- A task can be given to the manager or to a member of the team of its own
  project, and to nobody else. The application checks this.
- Removing a member gives its tasks to nobody.

## Consequences

- The staffing proposal of the Advisor becomes a list of members to create,
  and the reason for each one has a home: what the member is there for.
- The same discipline in two projects is two members. Describing a
  specialist once and using it in many projects needs a second thing, a
  profile. The roadmap has it in the milestone of the first working agent.
- A stored color stays the same when other members come and go. The views
  to come (timeline, office, usage) can use it to tell members apart.
- Removing a member deletes its record. Once sessions and usage hang on a
  member, a removal must keep that history. That will need a new decision.
- The database does not tie a task to its assignee with a foreign key,
  because the column is older than the table of members. The application
  keeps the two consistent: it checks every assignment, and on a removal it
  stores the tasks before it deletes the member.

## Alternatives considered

- **One pool of members for all projects.** Easier to assign from, but the
  context, the memory and the budget of an agent belong to a project, and
  the Advisor sizes a team for one project.
- **Separate types for agents and for people.** Every rule and every view
  about assignees would have two branches for the same behavior.
- **A color computed from the name or from the position in the team.** It
  changes when a member is renamed or when another member leaves, and two
  members of a small team can end up looking alike.
