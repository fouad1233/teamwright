# Form

A kind of view that keeps a form which makes a task of this project when it
is sent. It is a page the view keeps (group `pages`), found by the table of
kinds through `kind.tsx`. Nothing is shared with anybody: Teamwright runs on
one computer, and the form is filled in there.

- **What it keeps.** The form as JSON in the view's content: its title, a few
  words of introduction, and its fields in order, each with the property of
  a task it fills (title, description, assignee, priority, start, due,
  points, estimate), a label of its own and whether it is required.
  `parseForm` reads it with care: content that is missing or damaged reads as
  the form a new view starts with, a field it does not know or a second field
  of one property is dropped, and a form that lost its title field gets one
  back at the top. The title field is always there and always required.
- **A new view is useful at once.** Until the manager changes something, the
  view keeps nothing and shows the starting form: named after the view, with
  a title, a description, a priority and a due date. The first change writes
  the form; changes are written about a second after the last one, and at
  once when the view is left (`useTaskForm`).
- **Fill in** is the default face: the form as a printed sheet, one column,
  the labels above the program's own fields (the text field, the choice
  menus, the calendar field). Points and an estimate are typed as they are in
  the panel of a task and read with the same functions (`parsePoints`,
  `parseEstimate`). When "Send" is pressed, what is required and missing, or
  cannot be read, is said at its field in a sentence, and the first such
  field takes the keyboard.
- **Sending** makes the task with the program's use case for a new task,
  which takes everything but the points; the points are set once the task
  exists. If that second step is refused, the line at the foot says the task
  was made without its points, so the manager is never left guessing. A task
  that was made is named at the foot by its key and title, as a link that
  opens it, and the form is empty again for the next one.
- **Edit the form** is the same sheet with handles: the title and the
  introduction are written in place, each label is changed in place (an empty
  label brings back the usual name), a field is marked required or optional,
  removed (not the title), or moved by its grip, with the pointer or with
  the keyboard (Enter or Space lifts it, the arrows move it, Enter or Space
  puts it down, Escape puts it back). "Add a field" offers the properties not
  on the form yet. Every change is a pure function in `formModel.ts`, with
  tests.
