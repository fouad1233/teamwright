# The mind map

A kind of view (`mind_map`) that shows the project as a tree that grows to
the right. It is found by the table of kinds through `kind.tsx`.

- **The tree.** The project stands at the left. Its areas hang on it, in
  their order, if the project has more than one; with one area open, only
  that area. The tasks of an area hang on it by number, and under each task
  its subtasks by number, to any depth. A project that is not cut into
  areas has its tasks on the project itself. An area without tasks is a
  leaf. A task whose area is not shown hangs on the project, after the
  areas, so that no task is lost. `mapTree` in `mindMapModel.ts` builds it.
- **The layout** is a pure function, `layoutTree`. Nodes that are equally
  deep stand in one column and begin on one line. A column is as wide as
  its widest node, with room for the stem of a node that has others under
  it, and a gutter lies before the next. Under a node, the nodes that hang
  on it are stacked with an even space, each with everything under it in a
  room of its own, so nothing overlaps; the node stands in the middle
  between the first and the last of them. All places are whole pixels. How
  wide a node is follows from its words: the page measures the nodes once
  they are drawn (and again when a typeface arrives), and the layout is
  made from what was measured.
- **The joins.** A line leaves a node at the middle of its right edge, runs
  straight to the edge of the node's column, and fans out from there in one
  curve to each node under it, which it meets at the middle of the left
  edge. Every fan of a column therefore has the same shape. The lines and
  the hairlines of the nodes keep their width on screen at every scale.
- **What a node looks like.** The project is set in the serif, the one
  place of the view where it speaks. An area is a label. A task is a plate
  with the mark of its status, its key and its title; a subtask is the
  same, a little smaller, on the bare ground. The keys are set as wide as
  the longest of them, so the titles begin on one line.
- **Folding.** A node with others under it ends in a mark that folds them
  away. While it is folded, the mark says how many tasks it hides, however
  deep. The project itself cannot be folded. The node that was folded
  stays where it is on screen, and the rest of the tree is laid out again
  around it. The mark is the arrow of every fold of the program
  (`tasks/SubtaskFold`), but not that control: it folds areas too, and
  counts every task it hides, not only the subtasks of one task.
- **What the view keeps.** Only which nodes are folded, as
  `{"version":1,"folded":["area:…","task:…"]}` in the content of the view;
  a map with nothing folded keeps nothing. `foldedIn` reads content that
  is missing, damaged or of another shape as nothing folded. A fold is
  written a second after the last one, and at once when the view is left
  (`useKeptFolds.ts`). The folds of tasks and areas that are gone are
  forgotten with the next fold. Where the ground lies, and what is
  selected, belong to the window and are not kept: the map opens fitted.
- **Selecting and opening.** A press selects a node; gold marks it. A
  second press on it, or Enter, opens its task in the panel at the side.
  A press on the bare ground lets the selection go.
- **Adding.** The selected node has a "+" beside it: under a task or a
  subtask it adds a subtask, under an area a task in that area, and under
  the project a task, when the project shows no areas. The title is typed
  in a line where the new task will hang. Enter makes the task and keeps
  the line open for the next; Escape, or leaving the empty line, closes
  it. In a view that shows one person's tasks, the new task is given to
  that person.
- **The keyboard.** The Tab key stops at one node of the tree: the
  selected one, or the project. From there the arrow keys step through
  the tree: left to the node this one hangs on, right to the nearest node
  under it (or, on a folded node, unfold it), up and down along the
  column. Space folds and unfolds, Enter opens the task, Home goes to the
  project. A node that is stepped to is brought into view.
- **Moving and zooming** are those of `shared/ui/Ground`: drag the ground,
  turn the wheel, Ctrl and the wheel to zoom about the pointer, and the
  buttons in the corner, with the keys + and -, 0 to fit and 1 for 100%.
  A press that drags the ground selects nothing.
