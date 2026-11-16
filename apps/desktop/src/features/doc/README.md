# The doc

A kind of view (`doc`) that is a page of writing kept with the project. It
is found by the table of kinds through `kind.tsx`.

- **The view keeps the text**, in Markdown, as `{"text": "…"}` in its
  content. Content that is missing, empty or of another shape is an empty
  page, and a page without words is kept as nothing (`readPage` and
  `writePage` in `docModel.ts`).
- **Reading is the default.** The page is drawn by the one drawing of
  Markdown the interface has, `shared/ui/Prose`, with a look of its own:
  a line of about 68 characters, headings in the serif, hairlines for
  rules. Nothing in the text is ever inserted as HTML, and a picture is
  named, not fetched.
- **The list at the side** shows the headings of the three largest sizes
  when the page has more than two of them (`contentsOf`). The title of the
  page, a first heading that is larger than every other, is left out. The
  entry the reader is at carries the gold. In a narrow view the list gives
  way to the page. The headings are found by reading the text line by line
  (`headingsOf`), not by the Markdown reader: where the two disagree, which
  takes an unusual page, a heading is missing from the list and nothing
  else goes wrong.
- **A box can be ticked while reading.** `Prose` says where in the text the
  item of the pressed box begins, and `withBoxTicked` changes the sign in
  that box.
- **The key of a task** of this project (`SSN-4`) is drawn as the mark of
  its status and the key. It opens the task and shows its title and status
  when pointed at. A key that no task has stays plain text.
- **A link** to an address of the web is pressed. The address gets a window
  of its own, cut loose from this one. Where no window opens, the address
  is copied and the manager is told to paste it into the browser
  (`followLink` in `DocView.tsx`). That is what the desktop program is
  expected to do today: its shell has no way to open an address in the
  browser and sets nothing up for new windows, which the web view then
  refuses. When the shell learns to open an address, `followLink` is the
  one place to change. Any other link shows its address beside its words.
- **Writing** turns the page into its plain text in one field that grows
  with it. The tools of the bar and their keys (Ctrl and B, I, K, E) are
  pure functions from the text and what is selected to a small change
  (`marked`, `linked`, `coded`, `withLinesAs`), and Enter goes on with a
  list or a quote (`continued`). A change is made the way typing is made,
  so Ctrl and Z takes it back. Tab is left alone: it leaves the field, as
  everywhere. Done or Escape goes back to reading, at the heading the caret
  was under; Write opens the field at the heading that was being read.
- **Saving** is the work of `PageKeeper` (`pageKeeper.ts`), which knows
  nothing of React. A change shows at once and is written a second after
  the last one, and five seconds after the first one at the latest, so
  that typing without a pause is kept too. Leaving the view, hiding or
  closing the window, Done and Ctrl and S write at once. The bar says when
  the page was last saved. A write the backend refuses is said there too
  ("Not saved"), stays on the page and is tried again the next time the
  page is written at once, not with every pause in the typing. A view that
  was removed is not written to.
- **Typing is never given up for what the list of views says.** The list is
  loaded again for many reasons while a page is open. A page that is being
  written, or holds something not yet kept, keeps its own text. A page at
  rest takes a stored text it has never held, which another computer wrote.
  A stored text the page wrote itself earlier means the list is behind, and
  the page asks for it again (`stepFor`). While a page is written here and
  changed on another computer at the same time, the one written here wins.
