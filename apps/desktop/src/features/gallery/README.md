# Gallery

A kind of view that shows the tasks as cards in a grid, the picture first.
It is found by the table of kinds through `kind.tsx`.

- **A card** is the picture the view keeps for the task, then the title (two
  lines at most), the status and the person. A press on the card, or Enter,
  opens the task. The menu in the corner of the card ("Choose a picture",
  "Take the picture off", "Open the task") is reached with Tab and opened
  with Enter.
- **A card without a picture** is a plate: a sunken field in a hairline frame
  with ticks at its corners, and the task's key set large and quiet in the
  middle. A gallery without any picture is a wall of such plates, with one
  line in the bar that says how a picture gets onto a card.
- **A picture gets onto a card** by dropping a picture file on it, by pasting
  one while the card (or its menu button) has the focus, or with "Choose a
  picture" in its menu, which opens a hidden file field. The browser decodes
  it, a canvas draws it at most 640 pixels on its longest side, and it is
  kept as a JPEG of quality 0.72 (`reducePicture.ts`). The arithmetic of the
  size is `reducedSize` in `galleryModel.ts`.
- **The weight is counted before anything is sent.** The content of a view
  may hold at most 2 MiB, counted in UTF-8 bytes, and the backend refuses
  more. `fitPicture` counts the content with the new picture (a picture that
  replaces another gives back its room) and refuses one that does not fit
  with a sentence that says how much it needs and how much is left. When
  less than 512 kB is left, the bar says how many more pictures of the same
  weight would fit; otherwise it says how many pictures there are and what
  they weigh. A drawing weighs 10 to 20 kB; a photograph about 70 to 100 kB
  as text, so a dozen take about half of the room; pictures of pure noise,
  the worst case, about 160 kB, and twelve of them still fit.
- **Three sizes of card,** small, medium and large, chosen in the bar.
- **What the view keeps** is `{"size": "medium", "pictures": {"<task id>":
  "<base64 of the JPEG>"}}`. Content that is missing or damaged gives an
  empty gallery; a picture that is not base64 is left out. A change is
  written about a second after the last one, and at once when the view is
  left. The pictures of tasks that no longer exist are dropped at the next
  change, so that they do not go on taking room.
- **In the desktop program** a file dragged from the file manager reaches
  the page only if the window does not take file drops for itself
  (`dragDropEnabled: false` in `tauri.conf.json`).
