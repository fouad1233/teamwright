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
