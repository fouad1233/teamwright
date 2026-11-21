# 0030. Tags that belong to a project, and one checklist inside each task

- Status: accepted
- Date: 2026-11-21

## Context

Milestone M6 of the roadmap asked for tags and checklists. A manager wants
to mark work across statuses and areas ("EMC", "Waiting for parts", "Cost")
and see it on the board, and wants to break a task into a few small steps
without making each step a task with an owner, a status and dates.

Both have to travel between the manager's computers like everything else
([decision 0017](0017-sync-through-a-folder.md)). Things merge there field
by field: the later change of a field wins. A field that holds a list, such
as what a task waits for, is merged as one value, so two computers that
each add one entry to it keep only one of them. For tags and checklists
that would be wrong: putting a tag on a task on the laptop and another one
on the desk is an everyday case, and both must stay.

## Decision

- **A tag belongs to one project.** It has a name of at most 30 characters,
  unique within the project whatever the capitals, and one of the eight
  colors of the domain. A project has at most 100 tags. A new tag without
  a color gets the next one in a fixed turn. Tags are listed by name, not
  in an order the manager chooses: there is nothing to order.
  Tags of the whole program, shared by every project, were left out: a
  word means something in one project and something else in another, and
  a tag of the program would have to be removed from projects that never
  used it.
- **A task carries any number of the tags of its project.** It carries
  them as a set; putting on a tag it has changes nothing. Removing a tag
  takes it off every task first, so that each task's history says so, and
  a view that showed only the tasks with it shows the tasks with any tag
  again.
- **A task has one checklist.** Its items are short texts (at most 200
  characters, like a title) that are done or not done, in an order the
  manager chooses, at most 100 of them. One list is enough: a task that
  needs two lists of steps is two tasks, or a task with subtasks, each with
  its own checklist. A second level would bring names and orders of lists,
  and the progress on a card ("3/5") would no longer say one thing.
- **An item is lighter than a subtask.** It has no owner, no status, no
  dates and no number, and no history of its own beyond being ticked.
  Something that needs any of those is made a subtask.
- **A view can be filtered by one tag**, beside whose tasks it shows and
  whether finished tasks are hidden. A task passes if it carries the tag,
  or one of its subtasks does, as for the other parts of the filter.
- **Storage: the tags and the items are parts of the logged state of the
  task, one key each.** A tag is a thing of its own in the log (`tag`),
  like an area. The tags a task carries are keys `tag:<id>` of the state
  of the task, true while it carries the tag and false once it was taken
  off; the items of its checklist are keys `item:<id>` holding the item
  (text, done, rank), or null once it was removed. A key that no field of
  the merge names merges on its own, so two computers that change
  different tags, or different items, of one task both keep their change.
  Two changes to one item meet as whole items: the later one wins.
  The order of the items is a rank in each item, so moving an item changes
  that item alone.
  The history of a task, which is read from its logged states, therefore
  says when a tag was put on or taken off and when an item was ticked, with
  nothing added to it.
  The alternative, a logged thing of its own for every pair of a task and
  a tag and for every item, would have needed rules for what waits for
  what, what goes with a deleted task, and a history read from several
  kinds of thing at once. It was not worth it for so small a part.
- **What syncing repairs.** A tag removed on one computer comes off the
  tasks the other put it on and out of the filters of its views. Two tags
  given one name on two computers stay two tags: the younger is numbered
  ("EMC 2"), as for areas, and the manager removes the one not wanted.

## Consequences

- A task state written before this version has no such keys and reads as
  a task without tags or a checklist. Nothing is migrated in the log.
- The state of a task keeps the keys of tags taken off and items removed,
  as false and null, for good. A task that has had very many items carries
  them in every logged state. With at most 100 items at a time this stays
  small.
- A new task made in a view filtered by a tag does not carry the tag yet,
  so it leaves that view at once. The view says what it shows; carrying
  the tag over is left for later.
- The workbook names the tags of each task in a column of its Tasks sheet.
