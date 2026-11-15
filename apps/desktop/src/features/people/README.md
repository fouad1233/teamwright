# People

A kind of view that shows each person and their work. On the wire its kind
is `team`; it is called People because the program already has a place
called Team, where the members themselves are added and changed. It is found
by the table of kinds through `kind.tsx`.

- **A column for every person.** You first, then every member of the team in
  the order they joined, then the tasks given to nobody. That last column is
  always there, so that a task can be dropped on it. Tasks given to a member
  who has left go in a column of their own, only when there are any, and
  nothing can be dropped there.
- **The head is the person,** set like a bench of the office: the portrait,
  the name, what they are, what they are on the team for, and what they are
  doing right now. That last line is worked out by the office
  (`placeOfMember`, `placeOfManager` in `office/officeModel.ts`), so both say
  the same; a specialist at work has the lamp of its bench lit. Then three
  figures: open, in work, finished.
- **Three short groups.** In work (a status that is active or in review),
  next (not begun, the one due first at the top, five and then "more"), and
  finished lately (the five finished last). The groups follow the category
  of a status, never its name. A task a specialist works on right now
  carries the light of a card at work on the board.
- **Giving a task to somebody else.** Drag it to their column, or press Space
  on it, carry it with the left and right arrows and press Space again. The
  same use case as the task's panel (`assign`) is used. The task shows in its
  new column at once, while the backend answers. A press or Enter opens the
  task.
- The view keeps nothing of its own.
