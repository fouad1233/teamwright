# Whiteboard

A view that is a page: a free ground for notes and sketches, kept in the
content of the view.

## What lies on it

- **Notes**: cards in the paper of the program or the wash of one of its
  pigments (`data-tone`), never a bright yellow. A note can become a task
  ("Make a task" in its small menu): the first line is the title, the rest
  the description. The note then shows the task's status and key, and the
  key opens the task. If the task is deleted, the note stays a note.
- **Texts** in the serif, for headings. A text left without words goes.
- **Boxes**, hairline frames with their name in the corner in the
  monospace, to gather a group.
- **Ellipses**, hairline rings with their words in the middle.
- **Joins**: a line or an arrow from one shape to another. A join leaves
  the edge of its first shape and meets the edge of the second, box or
  ellipse, so it follows them when they move. It goes when either of them
  goes.

## Working with it

The bar at the top left holds the tools: select (V), note (N), text (T),
box (R), ellipse (O), line (L) and arrow (A). A press with a tool makes the
thing at its usual size; a drag makes it the size dragged. The tool then
goes back to selecting. Chosen with the keyboard, a tool makes its thing in
the middle of the frame at once, and the line and the arrow join the two
shapes selected.

A press selects, Shift adds or takes away, and a drag on the bare ground
selects with a frame. A double press or Enter writes in place; Escape or
Ctrl+Enter ends the writing. Delete removes, Ctrl+D duplicates, Ctrl+A
selects everything, the arrow keys nudge (with Shift, ten pixels), Escape
lets go. Ctrl+Z undoes and Ctrl+Shift+Z (or Ctrl+Y) redoes, within the
session.

The ground is the mind map's (`shared/ui/Ground`), with two options the
mind map does not use: it fits once and then stays put while things are
added (`fitsOnce`), and nothing holds it within reach of the content
(`reach: null`). Because a drag on the bare ground selects, the ground is
moved with the wheel, the middle button, or a drag while Space is held.

## Rules

- Everything is plain data in `whiteboardModel.ts`: the reading of the
  content, which never throws (a damaged or older content gives what can be
  read of it, or an empty board), every change, undo and redo, and the
  geometry of joins and frames. All of it is tested.
- `useKeptBoard.ts` keeps the board in the content of the view: a change is
  written a second after the last one and at once when the view is left. A
  change that would take the content past the 2 MiB a view may hold is
  refused with a message, and below 256 kB of room the board says how much
  is left.
- A copy of a note that became a task is a plain note: a task belongs to one
  note.
