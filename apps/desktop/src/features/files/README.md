# The files view

What lies in the folders of the project on this computer: the work folder,
where the specialists write, and the folders they may only read. The
folders stand down the left; what lies in the chosen one is a quiet table
at the right, with the path above it as crumbs.

- **It only reads.** A file is opened with the program the computer has for
  it, or shown in the file manager. Nothing is moved, renamed or removed
  here. A file that would run as a program (`.exe`, `.bat`, `.ps1`, `.lnk`
  and the like) is not opened; it can be shown in its folder.
- **Paths are checked in the shell.** The view sends the folder and a path
  inside it; the application refuses a path that goes up or starts
  elsewhere, and one that leaves the folder through a link. A link that
  leads outside is listed in a fainter ink and is not followed.
- **The noise of the tools is left out**: `.git`, `node_modules` and
  `target` always, other names that begin with a dot unless "Show hidden
  names" is chosen in the menu of the bar. At most 2000 entries are listed.
- **The disk is not watched.** The folder is read again whenever the view
  is shown and when the manager presses the button that reads it again.
- **The keyboard.** The arrow keys, Home and End move along the rows; Enter
  opens a folder or a file; Backspace goes up one folder; arrow down in the
  field that narrows the names lands on the first row. A double press opens
  too. Each row has a menu: open, show in folder, copy path.
- **What it keeps** (`view.content`, JSON): the folder last opened, the path
  inside it, and whether hidden names show. It is written a second after
  the last change and at once when the view is left. A kept folder the
  project no longer has falls back to the work folder.
- **The empty state** shows when the project has no reference folder and
  its work folder holds nothing yet, with a button that opens the dialog of
  the folders.

`filesModel.ts` holds what is worked out without drawing: the content, the
walking of a path, the crumbs, the marks by ending, the sizes in words and
the narrowing; its tests are beside it.
