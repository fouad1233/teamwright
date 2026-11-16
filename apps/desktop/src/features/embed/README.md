# Embed

A kind of view that shows one page from elsewhere in a frame, under a thin
bar. It is a page the view keeps (group `pages`), found by the table of kinds
through `kind.tsx`.

- **What it keeps.** One address, as JSON in the view's content
  (`{"address": "https://…"}`). Content that is missing, damaged or holds an
  address that is not https reads as an empty view. The address is written at
  once when the manager presses "Show the page".
- **The address is checked** by `checkAddress` before it is kept: it must
  parse, be `https`, have a host and carry no name or password. Anything else
  is refused at the field, in a sentence.
- **Well-known addresses are turned** by `embedAddress` into the form made
  for embedding: YouTube watch, short and youtu.be addresses (on the
  no-cookie host), Vimeo, a Loom share, a Figma file or design, and a Google
  Docs, Sheets or Slides address, published or shared. An address it does not
  know is shown as it is. The kept address is always the one the manager
  gave; only the frame gets the turned one.
- **The frame is sandboxed** with `allow-scripts allow-same-origin
  allow-forms allow-popups`, sends no referrer, and may go full screen
  (`allow="fullscreen"`) and nothing more.
- **The content security policy** of the desktop program
  (`src-tauri/tauri.conf.json`) has `frame-src https:` so that a frame can
  show an https page; no other directive was loosened.
- **A site that refuses** to be framed leaves a blank frame, and that cannot
  be detected. A quiet line under the bar says so, and "Open in the browser"
  opens the kept address outside.
