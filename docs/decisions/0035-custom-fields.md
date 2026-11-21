# 0035. Custom fields of a project, with one value a field on each task

- Status: accepted
- Date: 2026-11-21

## Context

Milestone M6 of the roadmap asked for custom fields. A manager of a
hardware product keeps things on tasks that the program does not know:
the supplier of a part, its unit cost, whether it is ordered, the day it
arrives, its part number, a link to its datasheet. Tags
([decision 0030](0030-tags-and-checklists.md)) say that a task belongs to
a group; they cannot hold a number or a date, and they cannot say "one of
these, and only one".

Fields have to travel between the manager's computers like everything else
([decision 0017](0017-sync-through-a-folder.md)), where things merge field
by field and the later change wins.

## Decision

- **A field belongs to one project.** It has a name of at most 40
  characters, unique within the project whatever the capitals, a place
  among the fields of the project that the manager chooses, and a kind. A
  project has at most 30 fields: each is a column of the table and a line
  of the panel of a task, and beyond that neither is read.
- **Six kinds, chosen when the field is made:** text (one line of at most
  200 characters), number (an ordinary number, at most a thousand billion
  either side of zero, with an optional unit of at most 8 characters shown
  after it, such as "€" or "mm"), date, yes or no, a choice of one from a
  list (each choice a name of at most 30 characters, unique within the
  field, and one of the eight colors, like a tag; at most 50), and a link
  (a web address that starts with `https://` and names a host, the rule of
  `WebAddress`, opened through the same checked command as every other
  address the program opens).
- **The kind of a field never changes.** The values the tasks hold would
  have to be converted, and most conversions lose something: a date read
  as a number, a text matched to a choice, a choice to yes or no. A wrong
  kind is fixed by making a new field and removing the old one.
- **No "person" kind yet.** A value that names a member would have to be
  cleared when the member leaves the team, by the use case and by syncing,
  as the assignee is; that is a third repair path for little gain, since
  the assignee already says who does a task. It is left for later.
- **A task holds at most one value for each field of its project.** The
  domain checks every value: its kind is the kind of the field, a choice is
  one of its choices, a number is ordinary, a link is `https`. Removing a
  field clears its values from every task first, and removing a choice
  clears the tasks that held it, so that the history of each task says so.
- **The history of a task** says when a value was set, changed or
  cleared, in words: "Field "Unit cost" changed from "3.95 €" to "4.20 €"".
- **A view can be filtered by one value of one field:** one choice of a
  choice field, or yes, or no, of a yes-or-no field. A box nobody ticked
  reads as no. Other kinds cannot filter a view: their values are not few
  and named, and a menu cannot offer them.
- **The Table view** has a column for every field, after the columns
  every task has, shown, hidden, sorted and edited like the others. A
  table that never chose its fields shows all of them, a new one too; so
  does a table that kept its columns before there were fields, whose kept
  list still reads as it did. Once the manager turns a field on or off,
  the table keeps the list (`fieldsChosen` in its content), and a field
  added later stays hidden until it is turned on.
- **Cards of the board and rows of the list show no fields.** Choosing a
  field or two per view would be the first setting a board keeps of its
  own; it was left out to keep this change small.
- **The workbook** gets a column for each field at the end of its Tasks
  sheet, so that the columns every project has keep their places: a
  number is a number and a date a date in the spreadsheet; a unit goes into
  the name of the column ("Unit cost (€)").
- **Storage: a field is a thing of its own in the log (`field`), and the
  values are parts of the logged state of the task, one key each,**
  following decision 0030. A field merges by name, by unit and by place;
  each of its choices is a key `choice:<id>` of its state, the choice or
  null once removed, so two computers that change different choices both
  keep their change. The value of a field on a task is a key
  `field:<id>`, the value as a small object that says its kind
  (`{"number": 12.5}`) or null once cleared, so two computers that set
  different fields of one task both keep theirs; two that set the same
  field keep the later value.
- **What syncing repairs**, as for tags: a field removed on one computer
  is cleared from the tasks the other gave it a value, and out of the
  filters of views; a choice removed on one computer is cleared from the
  tasks that held it on the other; two fields given one name on two
  computers stay two, the younger numbered ("Supplier 2"), and so do two
  choices of one field.

## Consequences

- A task or a view logged before this version has none of these keys and
  is read as a task without values, or a view filtered by no field.
  Nothing is migrated in the log.
- The state of a task keeps the keys of values cleared, as null, and the
  state of a field the keys of choices removed, for good. With at most 30
  fields and 50 choices this stays small.
- A task made in a view filtered by a field does not hold the value yet,
  so it may leave that view at once, as for a tag.
- Choices are kept in the order they were made; moving a choice is not
  offered yet.

## Alternatives considered

- **One field for all values of a task** (a single key `fields` in its
  state): two computers that set different fields would lose one change.
- **A logged thing for every value**: it would need rules for what waits
  for what and a history read from several kinds of thing, as decision
  0030 found for tags.
- **Changing the kind with a conversion**: refused above; the losses are
  silent and the manager would not see them.
