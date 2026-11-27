# What a specialist may and may not do

This page says, in plain words, what an AI specialist can touch on your
computer, what it cannot, and why Teamwright is strict about it. Read it
before you let a specialist work on a folder that matters to you.

A specialist is a session of Claude Code on your own computer, next to your
own files. Normally Claude Code asks before it does anything that could do
harm. A specialist has no window to ask in, so Teamwright decides beforehand,
once, what it may do. The rule is the same for every specialist and cannot be
loosened from the interface. That is on purpose.

## What it may do

- **Read, search, create and change files in the work folder of its project.**
  By default that is `Teamwright\<key of the project>` in your documents. If
  you chose another folder for the project, it works there, and what it
  changes there is changed for real.
- **Read, create and change files in the folders to write in** you gave the
  project, as in its work folder.
- **Read the reference folders** you gave the project, PDF files included. It
  cannot change anything in them.
- **Use the skills and connector tools you gave it**, and only those. See
  [skills and connectors](the-team.md#skills-and-connectors).
- **Hand work over, write to a teammate, remember facts and search the memory
  of the project**, through the tools Teamwright gives it. See
  [work](work.md).

## What it may not do

- It cannot run commands on its own. It has no shell, so it cannot compile,
  run tests, install anything or start a program, unless you switched
  command permissions on for this computer and then allow one precise
  command. See [permissions you give](#permissions-you-give).
- It cannot fetch anything from the internet on its own, unless you allow one
  web domain, and it cannot order parts or publish anything.
- It can neither read nor change a file anywhere else on your computer. The
  session is confined to its folders, unless you give the project
  [full access](#full-access).
- It does not see what you have set up in your own Claude Code: your skills,
  plugins, connectors, hooks and settings stay yours. Every turn is started so
  that none of them is loaded.
- It cannot start other sessions or schedule work.
- It cannot choose to be less careful. No instruction you write for it, and no
  text in a document it reads, can switch these limits off.

Anything that would need approval is refused at once, so a turn never waits for
an answer that nobody can give. What was refused is shown to you with the turn.
When a task needs one of the things above, the specialist asks you for a
permission, or says so in its report, instead of working around it.

## Permissions you give

A specialist that needs to fetch from a web domain, or to run a command if
you switched command permissions on, can ask you, with its reason, and you
answer in the inbox (see
[work](work.md#when-a-specialist-asks-for-a-permission)). This is the one way
to open what is closed above, and it opens exactly what you read and nothing
more. Read this part before you allow anything.

**Commands are off until you switch them on.** Command permissions are a
choice of each computer, under *Commands* on the page *Computers* (see
[settings](settings.md#commands)), and they start off. While they are off a
specialist can ask only for a web domain, no request for a command reaches
you, and no turn runs a command, whatever you allowed before. They are off
because nothing of them has been seen working with a real Claude Code yet.
Switch them on only if you trust the files of your projects.

**A command can run anything the specialist writes.** A specialist may write
files in its folder, and most commands worth allowing run files from that
folder: `npm test` and `npm run` run `package.json`, `cargo build` runs
`build.rs`, `make` runs the `Makefile`, `pytest` runs `conftest.py`. So
allowing such a command lets the specialist run any program as you. Allow it
only when you would be content to run the files of the project yourself.

**A command is meant to run in a sandbox.** Every allowed command runs
inside the sandbox of Claude Code, which is to let it change files only in
the folder of the project, keep it from reading the rest of your home folder
(Claude Code still lets it read your git settings), and let it reach no web
domain you have not allowed. If the sandbox cannot start, the turn does not
start. Claude Code has no sandbox on Windows, so on Windows no command can be
allowed; on Linux it needs the programs `bubblewrap` and `socat`, and
Teamwright tries `bubblewrap` before it allows a command: an empty sandbox,
and then a second one inside it, which the sandbox of Claude Code needs for
its own helper. Ubuntu 24.04 as installed refuses the second, so there the
part *Commands* says no command can run, and only an administrator of the
computer could change that. Some of what the sandbox needs on Linux can only
be seen in a run. The sandbox has not
been seen working yet (see the note below).

**A command does not see your secrets.** The sandbox does not hide
environment variables, so a turn that may run a command is started with
almost none: the search path, your home folder and user name, the language,
the time zone, the temporary folder, the shell, where Claude Code keeps its
settings, and a proxy if you use one. Your tokens and keys in the
environment (for GitHub, a cloud, a database) do not reach it. If you sign
in to Claude Code with an environment variable, such as an API key, that
computer runs no command at all, because the key would have to stay.

**The shell reads too.** With any allowed command the specialist can also
read, list and search files inside the folders of the project with the
shell, as Claude Code allows its read-only commands without asking.

**There are no standing commands.** A command is allowed once or for one
task, never always. A web domain may be allowed always. A command runs only
in a turn you start yourself; a turn the coordinator starts on its own, to
begin a task, to go on after the limit of your plan or to answer the
questions of teammates, runs no command. A specialist that has a connector
given a secret in an environment variable is allowed no command, because a
command could read the secret.

Some choices keep the risk smaller:

- Prefer *Allow once* to *Allow for this task*.
- Read *What you allow* before you press *Answer*. It is one exact command,
  and nothing is allowed that is not shown.
- Look at *Permissions* in the *Team* place now and then, and press *Take
  back* on what you no longer need.
- A permission holds only on the computer where you gave it. An answer that
  arrives through the sync folder allows nothing, and so does a permission
  that anything from the sync folder has changed.

### What can never be allowed

Teamwright refuses these however a request is worded, and however you edit
it. The check is made on the request, on your answer and again when a turn
starts.

- **Anything but one exact command.** There is no rule for every command
  that starts with a program and its subcommand: what follows such a start
  can change what the program does (`git log --output=…`).
- **A command that joins, quotes or changes other commands**: one that
  contains any of ``; & | < > ` $ * ? ( ) { } [ ] \ ~ ! # % ^``, a quote
  (`"` or `'`), or that starts by setting a variable. Those join commands,
  redirect output, put in the output of another command, or make the shell
  read another word than the one checked.
- **A command on more than one line**, with a tab, or with a character
  outside plain keyboard letters, digits and signs.
- **A command that names an absolute path, a drive, `~` or `..`** anywhere in
  a word, also after `=` or glued to an option (`--output=/…`, `-o/…`), or
  has `.claude` or `teamwright` in it. So it cannot name the data of
  Teamwright, the sync folder or the settings of Claude Code.
- **These programs, anywhere in a command:** `claude`, `teamwright`, `sudo`,
  `su`, `doas`, `pkexec`, `runas` and `gsudo`. **And at its start:** the
  programs that change files, `rm`, `rmdir`, `mv`, `cp`, `mkdir`, `touch`,
  `ln`, `chmod`, `chown`, `chgrp`, `dd`, `tee`, `truncate`, `shred`,
  `install` and `sed` (a specialist changes files with its file tools,
  which stay in the folder), and the programs and words of the shell that
  only start another or run a file in the shell: `env`, `xargs`, `eval`,
  `exec`, `nohup`, `timeout`, `time`, `nice`, `ionice`, `watch`, `stdbuf`,
  `setsid`, `command`, `builtin`, `noglob`, `nocorrect`, `flock`,
  `busybox`, `toybox`, `chrt`, `taskset`, `setpriv`, `unshare`, `sg`,
  `script`, `.`, `source`, `strace`, `ltrace`, `nsenter`, `chroot`,
  `prlimit`, `numactl`, `setarch`, `runuser`, `systemd-run`, `fakeroot`,
  `faketime`, `unbuffer`, `rlwrap` and `caffeinate`.
- **A word that starts with `=`**: some shells turn `=python3` into the
  full path of a program.
- **A command allowed always**, and any command while command permissions
  are off, on Windows, on a computer signed in to Claude Code with an
  environment variable, or for a specialist with a secret in reach (see
  above).
- **A web domain** that is a single word, `localhost`, a name ending in
  `.local`, or an address in numbers. Your computer and your local network
  stay closed. A domain is one host. All the hosts under a name, such as
  `*.example.com`, cannot be allowed: under a name many people share, such
  as `*.github.io`, anybody can make a host.

A refused request adds nothing, and refused stays refused: the specialist is
told not to try another way.

What was not proven: Teamwright decides what may be allowed, and starts every
turn with exactly those rules and the sandbox. How Claude Code matches a
command against a rule, and how its sandbox holds a command in, are Claude
Code's own; its documentation warns that a rule on a program is not a
boundary around it, which is why the sandbox is there.

Note: permissions were tested at every layer and seen in the demo. They have
never been tried with a real specialist, no command has run on a permission
in the installed program, and the sandbox has never been seen working: Claude
Code was not signed in when it was built, and when it was, the computer it
was tried on (Ubuntu 24.04) could not start the sandbox. That is why command permissions
are off until you switch them on. Until a real run has shown it, allow a
command only as if there were no sandbox.

## Full access

*Full access* is a switch of each project, in its *Folders*. It is off until
you switch it on, and it is kept for this computer only: your other
computers keep their own choice, and nothing of it travels through the sync
folder.

**What it gives.** When it is on, the specialists of the project may read
and change files anywhere your account can: your documents, your other
projects, your downloads. A specialist can change or delete any of your
files, as you could. It holds from the next turn, for every turn of a
specialist of the project on this computer, also the turns the coordinator
starts by itself (the folders dialog and the coordinator say so). Only you
can switch it on, and it asks you once to confirm.

**What stays closed, even then.** These places can neither be read nor
changed by a specialist, whatever you allowed:

- Teamwright's own data folder: the database, the log of what happened, and
  while a turn runs the values of your connectors' variables. A specialist
  that rewrote it could forge the record of your projects.
- The sync folder, if you chose one: it holds the log of every project.
- The settings of Claude Code: `.claude` and `.claude.json` in your home
  folder (and the folder `CLAUDE_CONFIG_DIR` names, if you set it). They hold
  your own skills, hooks and connectors, and on Linux the sign-in itself.
- The keys of your system: `.ssh` and `.gnupg` in your home folder. Reading
  a private key would hand it to a model.

Besides, the reference folders stay read only, the specialist still has no
shell, no command can be allowed in a project with full access (its sandbox
would reach the whole disk), your own skills, plugins, hooks and connectors
are still shut out, and Claude Code itself, as its documentation says,
refuses writes to the files that hold the state of a repository or the
settings of programs (such as `.git`, `.bashrc` or `.npmrc`) when nobody can
be asked; that was not tried.

**What is not closed.** Everything else your account can read or change:
other keys and tokens your programs keep in files (a cloud's credentials,
the token of `gh`, the profiles of your browsers), and every document. Use
full access only for a project whose specialists you would let work at your
own desk.

**Prefer folders to write in.** When the work needs one or two folders more,
add them as folders to write in instead: they give exactly those folders.

**What was proven.** On Linux (Ubuntu 24.04, Claude Code 2.1.330), a real
specialist with full access wrote and read a file outside every folder it
was given, was refused every read and write in stand-ins for the data
folder, the settings of Claude Code and the keys, found nothing of them by
a search of the folder around them, had no shell and none of your own
skills, plugins or connectors, and could still use the tools of the team.
It was not tried on a Mac or on Windows. On a Mac the rules are written the
same way, as the documentation of Claude Code describes; on Windows the
folder of each drive is given and the rules take the form that was seen
working there for the read-only folders.

## A connector is a way out of the folder

A connector reaches something outside the folder of the project: a ticket
system, a parts catalog. A specialist given one can reach whatever that
connector reaches. For that reason:

- You give each tool of a connector by name. A tool you did not turn on is
  refused.
- Teamwright keeps no secret. A connector only names the environment variable
  that holds it.
- A program connector runs on your computer as you, like any program you
  start yourself. The tool rules limit what the session may call, not what
  the program can do. Add only connectors you trust.
- A connector is started on a computer only as you confirmed it on that
  computer. One that arrived or changed through the sync folder, and a tool
  given to a specialist on another computer, wait until you look at exactly
  what would start and confirm it there. Whoever can write the sync folder
  cannot make your computer start a program you did not see.
- A program connector starts in a fresh empty folder of Teamwright's own,
  made for it alone for that turn and removed after it, not in the folder of
  the project, and sees only the variables you named, so a file a specialist
  or another connector writes cannot take its place. The secrets of the
  connectors are kept out of the specialists' sessions, except the one of a
  web address connector given to that specialist.
- A skill is checked again before every turn: its copy must hold exactly
  what Teamwright wrote. A copy that changed is not given, and the library
  says why.

## Who decides

- A specialist asks you when a choice is yours: what is built, what it costs,
  when it is ready. It stops and waits for your answer.
- A specialist never marks a task as done. You accept the work.
- Work that you did not start, started by the coordinator, happens only when
  you switched the coordinator on, and a budget can stop it.

## What leaves your computer

A specialist is Claude Code at work. What it reads and writes is sent to
Anthropic under your own account, as when you use Claude Code yourself.
Teamwright itself sends nothing anywhere you did not choose: it writes into the
sync folder you chose, to the integrations you set, and, if you switch it on,
asks GitHub for the list of releases.

## Why it is this strict

The first version of this rule had a flaw, and it shows why the rule is
checked rather than trusted. It let a specialist read without asking by a rule
that named the tool and no folder, and such a rule holds everywhere: asked to,
a specialist read a file outside its folder. It was found on the day it was
first used, by a test that asks a real specialist to read and write outside its
folder. Since then the session is confined, and the same test is refused both
times. The reasoning is in
[decision 0011](../decisions/0011-what-a-specialist-may-do.md).

The checks of this page were made against the real Claude Code on the day the
rule was written. After the later changes (skills and connectors, and the
settings that shut out your own hooks and connectors), the tests against the
real Claude Code were written but have not passed yet, because Claude Code was
not signed in on the computer where they were built. They must be run again
before you rely on the new parts. On 27 November 2026 the test that asks a
specialist to read and write outside its folders passed again on Linux, with
those settings, together with the tests of folders to write in and of full
access.
