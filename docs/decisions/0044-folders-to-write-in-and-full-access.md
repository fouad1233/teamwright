# 0044. Folders to write in, and full access per project and computer, with a few places always closed

- Status: accepted
- Date: 2026-11-27
- Changes: [0011](0011-what-a-specialist-may-do.md) (a specialist reaches
  only the folders of its project), [0013](0013-the-folders-of-a-project.md)
  (a project has one work folder and folders only to read)
- Builds on: [0038](0038-permission-requests-routed-to-the-manager.md),
  [0039](0039-skills-and-connectors-chosen-by-the-manager.md)

## Context

A specialist reads and writes in the work folder of its project, reads its
reference folders, and reaches nothing else (decision 0011). The founder
wrote: "it should be permissions of accessing folders so i should be able
to give full access". Asked how far it should go, he chose: a switch on a
project, off by default and kept for this computer only, with which its
specialists may read and write anywhere the user's account can; and the
option to add more folders with write access without going that far.

This opens what the security model closes. The two reviews of decision
0038 found that what was allowed reached further than what the manager
read. So every choice below names exactly what is opened, keeps everything
else of decision 0011, and was tried against the real program.

### What Claude Code offers, as found on 27 November 2026

Claude Code 2.1.330 is installed and signed in (a Team plan). From
`claude --help` and the documentation at <https://code.claude.com/docs>
(pages `permissions`, `permission-modes`, `plugins/loading`), read that day:

- `--restricted` "confines the file tools to the working directories
  (`--add-dir` included)", removes the shell and WebFetch unless `--tools`
  names them, ignores the user, project and local settings files, and
  refuses `bypassPermissions`.
- A directory added with `--add-dir` is a working directory: its files are
  read without asking, and in `acceptEdits` written without asking. From
  such a directory Claude Code also loads skills, commands and agents in its
  `.claude`, but through the setting source `project`, and the keys
  `enabledPlugins` and `extraKnownMarketplaces` of its settings files only
  when the sources `project` or `local` are on. Every confined turn passes
  `--setting-sources=` (decision 0039), so none of it loads.
- Without `--restricted`, a bare `Read` allow rule holds everywhere (this is
  the defect decision 0011 corrected), and writing outside the working
  directories asks, which with `--permission-prompts none` means refused.
  Opening writing there needs a bare `Edit` allow rule.
- `Read(path)` and `Edit(path)` rules use the syntax of gitignore; `//path`
  is absolute (decision 0013, note of 22 November 2026). A rule that denies
  is checked first and always wins. A `Read` deny also stops `Write` and
  `Edit` on the path, not `NotebookEdit`; Claude Code "makes a best-effort
  attempt" to apply it to Grep and Glob. A deny applies when either the path
  asked for or the file it leads to matches.
- In `acceptEdits`, writes to the protected paths of Claude Code (`.git`,
  `.claude`, `.bashrc`, `.npmrc` and others) still ask, so are refused here.

## Decision

### Folders to write in

A project can have **folders to write in** besides its work folder. The
manager chooses each with the system's folder window; the folders dialog
lists it as *written* beside the work folder (*work*) and the reference
folders (*read only*), and removes it again. They are facts of this computer,
as decision 0013 keeps the other folders: in `project_written_folders`
(migration 30), logged in the state of `project_places`, which never leaves
the computer. The rules, in `PlacesService`, refuse a folder to write in that:

- **is already one**: it is kept once;
- **is, lies inside or contains the work folder, a reference folder or
  another folder to write in.** A folder has one kind. Nested folders of two
  kinds would leave the manager unsure whether a file may be changed (a
  read-only folder inside a written one is the case decision 0013 already
  refused for the work folder); nested folders of one kind add nothing. The
  work folder and a reference folder are checked against the folders to
  write in the same way;
- **is, lies inside or contains a protected place** (below). Containing one
  is refused too, so the home folder cannot be a folder to write in: a
  folder to write in grants exactly what it names, and the one switch that
  grants more, full access, is the one that keeps the protected places shut
  by rule.

A turn names each with `--add-dir` and no rule against writing. Its
instructions name them on lines of their own, and say to write there only
what the task needs.

### Full access

A project has a switch, **full access**, off until switched on, kept for
this computer in `meta` (`full_access:<project>`), written without the
recorder, like the choice about commands: never logged, never synced,
removed with its project. When it is on, every turn of a specialist of the
project on this computer is started with:

- everything a confined turn has: `--restricted`, `--setting-sources=`, the
  settings object of decision 0039 (`disableAllHooks`,
  `disableSkillShellExecution`, `disableClaudeAiConnectors`),
  `--strict-mcp-config`, `--permission-mode acceptEdits`,
  `--permission-prompts none`, the same built-in tools, and `Bash` and
  `PowerShell` denied;
- **the root of the file system as a working folder**: `--add-dir /`, and on
  Windows `--add-dir` for each drive from `A:\` to `Z:\` that exists;
- **a rule that denies reading and one that denies writing for each
  protected place**, in the form decision 0013 proved.

The home folder as a working folder was the other candidate. It drags in
more: the `.claude` of an added folder is where Claude Code looks for
skills and `enabledPlugins`, and the home folder's `.claude` is the user's
own. It is shut out by `--setting-sources=` either way, but the root holds
no `.claude` at all, and full access means the whole account, not only its
home. Leaving restricted mode and allowing `Read` and `Edit` bare was the
third: it would drop the confinement that decision 0011 depends on for
everything else, and leave the user's settings to `--setting-sources=`
alone. Neither was taken.

The exact command line of a specialist's new turn with full access on
Linux, without a team bus, kit or reference folder, where the data folder
is `~/.local/share/dev.teamwright.app` and the home folder `/home/sample`:

```text
claude -p --output-format stream-json --verbose
  --permission-mode acceptEdits --permission-prompts none
  --restricted --setting-sources=
  --settings {"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true}
  --strict-mcp-config
  --add-dir /
  --tools Read Glob Grep Edit Write NotebookEdit Skill ToolSearch
  --allowedTools Read Glob Grep
  --disallowedTools Bash PowerShell
    Read(//home/sample/.local/share/dev.teamwright.app/**) Edit(//home/sample/.local/share/dev.teamwright.app/**)
    Read(//home/sample/.claude/**) Edit(//home/sample/.claude/**)
    Read(//home/sample/.claude.json) Edit(//home/sample/.claude.json)
    Read(//home/sample/.ssh/**) Edit(//home/sample/.ssh/**)
    Read(//home/sample/.gnupg/**) Edit(//home/sample/.gnupg/**)
```

With a sync folder chosen, its two rules come after those of the data
folder. The test `a_turn_with_full_access_has_exactly_these_arguments` in
`crates/runtime/src/arguments.rs` holds it.

### What stays closed, and why

The **protected places**, neither read nor written, whatever is allowed:

- **Teamwright's own data folder.** A specialist that changed it could
  rewrite the database or forge operations of the log, which is something
  else than one that edits the manager's files. Reading is closed too: the
  database holds every project of the computer, and while a turn runs the
  values of the connectors' variables lie in it (decision 0039). The cost:
  under full access a specialist cannot read the files a skill brings beside
  its `SKILL.md`, because the copies of skills lie in the data folder. Only
  protecting the database and the folder of connectors was weighed, and
  left: the copy of the database Teamwright keeps before the first sync,
  and any file Teamwright adds there later, would be open.
- **The sync folder**, if one is chosen. It holds the log of every project,
  and what is written there reaches the other computers.
- **The settings of Claude Code**: `~/.claude`, `~/.claude.json`, and the
  folder `CLAUDE_CONFIG_DIR` names. They hold the user's own skills, hooks,
  agents and tool servers, the transcripts of earlier sessions, and on
  Linux the sign-in itself. A specialist that wrote a hook there would run
  it in the manager's next own session.
- **The keys of the system**: `~/.ssh` and `~/.gnupg` (on Windows also
  `%APPDATA%\gnupg`). Reading them is denied too: reading a private key
  hands it to a model.

The places are given by the program (`ProjectFolders::protected_places`):
the data folder, the sync folder as it is now, and those of the home folder
(`teamwright_runtime::protected_places_in_home`). The runtime names each in
its full form, with links followed; one that does not exist is protected
all the same. Without full access, a protected place that lies inside a
folder the turn was given (a work folder in the home folder, say) gets the
same two rules, which closes a gap the reference folders had.

Other secrets in files are **not** protected: the credentials of a cloud,
the token of `gh`, the profiles of browsers. Full access means the whole
account. The interface and the manual say so.

### Who is affected

- **The specialists' turns of the project, on this computer.** Every turn,
  also one the coordinator starts by itself, to begin a task, to go on
  after the limit of the plan or to answer teammates: the manager switched
  full access on for the project, not for a turn. Where the coordinator works
  on this computer, the folders dialog says so when full access is on, and
  the coordinator says so in the Team place.
- **Not the Advisor.** It plans from the work folder and the reference
  folders and has only tools to read. Folders to write in and full access do
  not widen what it reads.
- **No command.** In a project with full access, no command can be asked
  for or allowed (`NoCommands::FullAccess`), and the runtime refuses a turn
  that has both. The sandbox of a command opens the working folders, which
  would then be the whole disk, and it has never been seen holding a command
  in (decision 0038). Web domains are not affected.
- **A turn whose folder lies inside a protected place** is refused with full
  access, because the rules would keep it out of its own folder. That
  happens only where `TEAMWRIGHT_DATA_DIR` puts the folders of the projects
  inside the data folder, for tests.

### The interface

The folders dialog lists every folder with its kind, then the switch *Full
access* with three sentences: what it gives, that a specialist can then
change or delete any file of the manager's, what stays closed, and that it
is kept for this computer. Switching it on asks once, in the project's own
dialog. While it is on, a quiet mark *Full access* stands above the team and
at the head of a specialist's conversation on this computer.

## Consequences

- A manager can let a team work across several repositories, or everywhere,
  and sees which in one dialog.
- With full access a specialist can destroy the manager's files. That is
  the founder's choice and is said where it is made. The data of
  Teamwright, the sync folder, the settings of Claude Code and the keys stay
  out of reach, so the record of the work and the manager's own setup
  survive a specialist that goes wrong.
- A project with full access runs no command.
- Skills lose their extra files under full access (above).

## What was proven, and where

On this Linux computer (Ubuntu 24.04, Claude Code 2.1.330, model
`claude-haiku-5-5`), three real runs, all passed:

1. `a_real_session_with_full_access_reaches_outside_and_never_the_protected_places`:
   with full access and stand-ins the test made for the data folder, the
   settings of Claude Code and the keys, the session wrote and read a file
   in a folder it was not given; every read and write in the stand-ins was
   refused; a Grep over the folder around them found nothing of what they
   hold; it had no `Bash` or `PowerShell`; its only tool server was the team
   bus, whose tool `remember` worked; its plugins were the program's own
   built-in ones and none of its skills was the user's. Estimated cost
   0.0043 USD.
2. `a_real_session_writes_in_its_folder_to_write_in_and_still_nowhere_else`:
   a folder to write in, with parentheses in its name, was read and written;
   a reference folder and a folder not given were not written, and the file
   outside was not read. 0.0025 USD.
3. `a_real_session_reaches_its_folders_and_nothing_else`, unchanged: with
   the switch off, nothing changed. 0.0022 USD.

**Not run on macOS**, which the founder uses. There the rules are written
the same way as on Linux, which the documentation of Claude Code describes
(`//path` is absolute). Not run on Windows either, where the root is each
drive and the rules take the form seen working there for reference folders
(`C:/Users/...`). A reference folder under full access, the files Claude
Code protects itself (`.git`, `.bashrc`), and the loss of a skill's extra
files were not tried in a run.

## Alternatives considered

- **A list of folders to read and write in place of a switch.** It is the
  first half of this decision; the founder wanted the switch too.
- **Leave restricted mode for full access.** Simpler arguments, and it
  would give up the one confinement every other rule leans on.
- **The home folder as the root of full access.** See above.
- **Allow commands with full access.** Their sandbox would reach the whole
  disk, and it was never seen working.
- **Keep full access in the log, so it holds on every computer.** A folder
  of one computer means nothing on another (decision 0013), and the sync
  folder can be written by others (decision 0039, note of 22 November): a
  synced switch could be turned on from there.
