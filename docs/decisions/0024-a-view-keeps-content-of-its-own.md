# 0024. Let a view keep content of its own, as text the domain does not read

- Status: accepted
- Date: 2026-11-15

## Context

[Decision 0021](0021-views-the-manager-can-add.md) made a view a kind with
a name and a filter. Every kind it had was a way of looking at the tasks
of the project: everything a view drew came from the tasks.

Fourteen more kinds are coming, and some of them are pages rather than
looks at the tasks. A document keeps its text. A whiteboard keeps its
notes and its arrows. A form keeps its fields, an embedded page its
address. A map keeps where each of its tasks is pinned, a gallery the
picture of each task. That content belongs to one view, not to the
project and not to a task: a second whiteboard has notes of its own.

The content must travel with the view to the manager's other computers,
as its name and filter do, and it is written often: the interface saves
while the manager types or drags.

## Decision

- **A view has `content`: text, or nothing.** A view that never had any
  has nothing. The interface of each kind writes its content as JSON and
  reads it back; the domain does not read it. The one rule about it is
  its size: at most 2 MiB (`DomainError::ViewContentTooLong`,
  `rule_broken`), so that one view cannot swell the database and every
  exchange between computers.
- **One use case sets it**, `ViewService::set_content(view_id, content)`,
  beside rename and set_filter and with the same care: the view must
  exist, and the row is updated in place through `Database::write`, so
  the change is logged. Setting nothing clears it.
- **It is a field of the view's logged state**, `content`, which merges
  on its own like the name: the later write wins. A state logged before
  this version has no `content` and is read as a view without any.
- **On the wire**, `View` has `content: string | null`, and the command
  `set_view_content` takes the view and the content. The interface's hook
  `useSetViewContent` shows the new content at once and writes after; its
  callers write at most about once a second.
- **Every kind's component is given the view** (`KindProps.view`), so
  that it can read its content and knows its id.

## Consequences

- A kind that keeps content needs nothing new in Rust: its interface
  decides the shape of its JSON.
- The log grows with every save. The whole state of a view is logged on
  each write, so a long document that is saved once a second while the
  manager types adds its whole text each time, and a rename logs the
  content again too. The 2 MiB limit and the once-a-second rule keep this
  bounded; trimming old states of the log is a later question for the
  log as a whole.
- The content is not understood by the domain, so no rule can be made
  about it there: not that a pinned task exists, not that a form's
  fields are well formed. Each kind's interface must accept content it
  does not expect, from an older version or a damaged file, and fall
  back to something sensible.
- Two computers that change the content of one view at the same time
  keep the later content whole. A note added on one computer and another
  added on the other at once will lose one of them. Merging inside the
  content would need the domain to understand it.

## Alternatives considered

- **A table for each kind of page**: notes and arrows for a whiteboard,
  fields for a form, pins for a map. The domain could then hold rules
  about them, and two computers could add notes at the same time without
  losing one. But it is fourteen kinds of thing, each with its storage,
  its log entity, its merging and its wire, before the first page can be
  drawn, and the shapes are not known yet. It can still be done for a
  kind whose content proves to need it.
- **Content on the task**, such as a picture or a place for each task.
  A gallery and a map would then read the task, and two galleries would
  show the same pictures. But the content of a whiteboard or a document
  is about no task, and a task would carry fields that only one kind of
  view uses.
