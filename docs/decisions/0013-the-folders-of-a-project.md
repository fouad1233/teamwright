# 0013. Give a project a work folder the manager can choose, and folders that are only read

- Status: accepted
- Date: 2026-11-12
- Changes: [0011](0011-what-a-specialist-may-do.md), which fixed the work
  folder to `Teamwright/<key of the project>` in the manager's documents

## Context

A specialist works in one folder and reaches nothing outside it
([0011](0011-what-a-specialist-may-do.md)). That folder was made by
Teamwright, one per project, in the manager's documents.

A manager often has the project already: a repository, a folder of drawings.
And a manager has material that the specialists should know and must not
change: data sheets, a standard, an earlier design. The founder asked for
both: to let a specialist work in a folder of their choice, and to hand it a
PDF or a folder as a reference.

Against Claude Code 2.1.322 it was tried what a confined session can be
given:

- With `--add-dir`, a session reads a folder outside its working folder,
  text files and PDF files alike.
- It can then also write there. A rule that denies editing below that
  folder, `Edit(<folder>/**)`, stops the writing and leaves the reading.

## Decision

- A project has a **work folder**. It is the folder Teamwright gives the
  project until the manager chooses another one. The specialists read,
  create and change files there, and nowhere else. It cannot change while a
  specialist works in the project.
- A project can have **reference folders**. The specialists may read them
  and cannot change them. A reference folder cannot be the work folder, lie
  inside it, or contain it.
- A **single file** that the manager hands over is copied into the folder
  `references` inside the work folder. The folder it came from is not
  opened to the specialists.
- These places are facts about one computer. They are kept with the project
  on this computer and are not part of what computers exchange when they
  synchronize.
- Teamwright never removes a file or a folder of the manager, also not when
  a project is deleted.

## Consequences

- A manager can point a team at an existing project and let it work there.
  What the specialists change there is changed for real, so the choice is
  the manager's and is shown plainly.
- A reference folder is read where it is. Nothing is copied, and a change
  the manager makes there is seen by the next turn.
- A single file is copied, so the specialists see an old copy if the
  manager changes the original afterwards.
- A specialist can be handed a large folder to read. What it reads goes to
  the model like everything else it reads, under the manager's own account.

## Alternatives considered

- **Open the folder of a single file.** It is what "point to the PDF" means
  taken literally, and it would show the specialists everything else in
  that folder too: a whole downloads folder for one data sheet.
- **Copy reference folders into the project.** Safe, slow for a large
  folder, and out of date the moment the original changes.
- **Let a specialist write in a reference folder.** Then it is a second
  work folder, and the manager could no longer tell where the work is.

## Note, 22 November 2026, after the first real run on Linux

**What was wrong.** The rule that keeps a reference folder read-only was
written `Edit(<folder>/**)` with the folder as the system names it. On
Windows that starts with a drive letter, and it worked. On Linux and macOS
the folder starts with one slash, and in the rules of Claude Code a single
leading slash does not mean the root of the file system: it means "from the
place of the settings", which for a session started by Teamwright is its
work folder. The rule forbade writing in a folder that does not exist and
left the real one open. The test
`a_real_session_reaches_its_folders_and_nothing_else` caught it: the
session wrote `data sheets (old)/smuggled.txt` into a read-only folder and
answered "File created successfully". Reads and writes outside every given
folder were still refused, as they should be.

**Since when.** The rule was written this way from the start, so version
0.1.0 has the defect on Linux and macOS. The shelf of skills of a
specialist (`Edit(<shelf>/<id>/**)`) had it too. On macOS this has not been
run; the documentation says the same.

**What the documentation says**
(<https://code.claude.com/docs/en/permissions>, "How path patterns match"
and "Special characters"): rules use the syntax of gitignore, and
`//path` is absolute from the root of the file system, `~/path` from the
home folder, `/path` from the place of the settings, `path` from the current
folder. A tool that writes is covered by `Edit(...)`: `Write`,
`NotebookEdit` and the old `MultiEdit` are all matched against it. The
sandbox is written differently (<https://code.claude.com/docs/en/sandboxing>,
"Path prefixes"): there `/tmp/build` is absolute, so the plain absolute
paths of `filesystem.denyWrite` were right and are unchanged.

**What it is now.** A folder of Unix is written `Edit(//home/sample/x/**)`.
A folder of Windows keeps the form that worked there,
`Edit(C:/Users/sample/x/**)` (the documentation names `//c/Users/...`; the
form was not changed because it is known to work, and Windows was not run
again).

**Names with special characters.** A real run with fourteen folders showed
two more ways to lose the protection. A star or a bracket, unescaped, may
be read as a pattern (`[1]` matches `1`, not `[1]`). And the rules go to
the program as a list that it splits at spaces outside parentheses: a name
like `paren (a) b` was cut into two rules and the folder was writable. Now
every character that is not a letter, a digit or one of `/ # ! - _ . :` is
written as `?`, which matches any one character, except `*`, `[`, `]` and
`\`, which are escaped (each was seen working). The rule can forbid more
than the folder (a folder `a b` also covers `a-b`), never less. So no
folder name is refused: a folder with parentheses and spaces is protected.
