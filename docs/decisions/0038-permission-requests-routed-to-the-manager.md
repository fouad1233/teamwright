# 0038. Ask the manager for a precise permission, and start the next turn with it

- Status: accepted
- Date: 2026-11-21

## Context

A specialist may read and write in the folders of its project and nothing
else ([decision 0011](0011-what-a-specialist-may-do.md)). Running a command
(a build, a test, a script) or fetching a web page is refused by how the
turn is started, and until now the specialist could only say in its report
that it could not. The architecture has promised from the start that risky
tool use is forwarded to the manager and never approved on the manager's
behalf.

This feature can only open what decision 0011 closed, so every choice below
leans towards refusing.

### What Claude Code offers, as found on 21 November 2026

Claude Code 2.1.289 is installed on the computer this was built on. It is
not signed in, so no run was possible: everything below comes from
`claude --help` and from the documentation at <https://code.claude.com/docs>
(the pages `cli-reference`, `permissions`, `headless`, `errors`,
`settings-reference` and `agent-sdk/typescript`), read that night.

- **Permission modes** (`--permission-mode`): `manual` (alias of `default`),
  `acceptEdits`, `plan`, `dontAsk`, `auto` and `bypassPermissions`. `auto`
  lets a model judge what is safe and `bypassPermissions` approves
  everything; Teamwright's type for the mode has neither (decision 0011).
- **Who answers a prompt** (`--permission-prompts`, from 2.1.259): `host`
  (the Agent SDK host or the `--permission-prompt-tool`) or `none` (anything
  that would prompt is denied, and the model is told not to retry).
  Teamwright passes `none`.
- **`--permission-prompt-tool mcp__<server>__<tool>`** names a tool of an
  MCP server that answers each prompt. It does not appear in `--help` of
  2.1.289 but is in the reference. Its answer is one text block whose text
  is `{"behavior": "allow"}` (optionally with `updatedInput`) or
  `{"behavior": "deny", "message": "…"}` (page `errors`, "Permission prompt
  tool returned an invalid result"). With such a tool and the default
  `host`, Claude Code emits no `permission_denied` events at all; with
  `--permission-prompts none` the tool is not asked.
- **Rules** (`--allowedTools`, `--disallowedTools`), page `permissions`:
  `Tool` or `Tool(specifier)`. Deny is checked first, then ask, then allow,
  and an allow can never carve an exception out of a deny. A bare tool name
  as a deny removes the tool. `Bash(npm run build)` is one exact command;
  `Bash(npm run *)` (or `Bash(npm run:*)`) is a prefix, and the
  documentation asks to put the `*` after the subcommand. Claude Code knows
  the shell operators `&&`, `||`, `;`, `|`, `|&`, `&` and newlines, and
  every part of a compound command must be allowed. It strips a few
  wrappers (`timeout`, `nice`, `nohup`, …) before matching, and warns that
  a rule on the arguments of a command is fragile and that a deny rule is
  not a security boundary around a program. `WebFetch(domain:docs.rs)`
  allows one host; `*.example.com` its subdomains. Paths in `Read` and
  `Edit` rules: `//abs/path`, `~/path`, `/relative-to-settings`, `./path`.
- **Read-only commands** (`ls`, `cat`, `echo`, `grep`, `find`, read-only
  `git`, …) run without a prompt in every mode once the shell is there,
  unless `permissions.blockReadsOutsideWorkingDirectories` is set, which
  makes those that read outside the working folders prompt (from 2.1.257).
  In `acceptEdits`, common file commands (`mkdir`, `touch`, `mv`, `cp`) are
  approved in the working folders.
- **Restricted mode** (`--restricted`, from 2.1.248) removes the shell and
  WebFetch unless `--tools` names them, confines the file tools to the
  working folders, ignores the user's settings (but reads `--settings`) and
  refuses `bypassPermissions`.
- **The stream**: with `stream-json`, a denial is a system message
  `permission_denied` with `tool_name` and `tool_use_id`, and the result
  lists `permission_denials` with `tool_name`, `tool_use_id` and
  `tool_input` (`SDKPermissionDeniedMessage`, `SDKPermissionDenial`). The
  decoder already reads both.

## Decision

### How a request comes about

**The specialist asks on purpose, with a tool of the team bus,
`ask_permission`.** Its arguments are a kind (`command`, `command_prefix`
or `web_domain`), what (the command line, the prefix or the domain) and
why. The bus checks the call by the rules of a permission (below) and
answers that nothing is allowed yet, that the turn is to end, and that the
answer comes as the next message. Like `ask_manager`, it is the one thing a
turn may ask; the decoder reports it as `PermissionAsked`, and the work of
the specialists records a request from the stream. Asking never blocks a
process (decisions 0010 and 0012).

**No permission prompt tool.** It was the other candidate, and it was left
out for four reasons:

1. In restricted mode the shell and WebFetch are not there at all, so the
   session can never try them and no prompt would ever reach the tool. It
   would catch only uses of tools the turn has, which decision 0011 already
   handles.
2. To make it catch commands, every turn would have to be given the shell.
   Then the read-only commands run without a prompt, everywhere the user's
   account can read, which undoes the first promise of decision 0011.
3. A tool whose answer can be "allow" is a tool that can approve on the
   manager's behalf. Ours would always deny; but nothing in the arguments
   would then prove that it does, and a mistake in it would open
   everything. `--permission-prompts none` proves it: nothing can approve.
4. With a prompt tool, Claude Code stops reporting denials in the stream,
   which the manager sees today.

What the specialist tried and was refused is still shown with its turn, as
before. A request carries a reason in the specialist's words, which a
caught denial would not.

### What can be allowed, and what never can

A permission is a precise rule, checked in the domain
(`PermissionRule::new`), so that no other part can make a wider one. The
same check reads the specialist's request, the manager's answer (which may
be an edit of the request) and what storage gives back.

- **A command** is one line. It may not contain any of
  `; & | < > \` $ * ? ( ) { } [ ] \ ~ ! # % ^`: they join commands,
  redirect output, put in a variable or the output of another command, or
  would widen or break the rule. It names no absolute path, no drive, no
  `..`, nothing with `.claude` (the agent program's settings) or
  `teamwright` (Teamwright's own data and program) in it, and it does not
  start by setting a variable. So it cannot name Teamwright's data folder or
  the sync folder, which are outside the folder of the project.
- **Never allowed**, as any command: `claude`, `teamwright`, `sudo`, `su`,
  `doas`, `pkexec`, `runas`, `gsudo`; and the programs that change files
  (`rm`, `rmdir`, `mv`, `cp`, `mkdir`, `touch`, `ln`, `chmod`, `chown`,
  `chgrp`, `dd`, `tee`, `truncate`, `shred`, `install`). A specialist changes
  files with its file tools, which stay inside the folder; with the shell,
  Claude Code would approve some of these on its own in that folder and in
  the folders it may only read.
- **A prefix** is a command of at least two words, the program and its
  subcommand, as the documentation advises, and never a program that runs
  whatever follows it: shells, interpreters (`python`, `node`, …), `env`,
  `xargs`, `eval`, `exec`, `nohup`, `timeout`, `watch`, `find`, `npx`,
  `curl`, `wget`, `ssh`, … Such a program may be allowed only as one exact
  command, which the manager reads in full.
- **A web domain** is a host name with at least two parts, or all the hosts
  under one (`*.example.com`), never `localhost`, `.local`, or an address in
  numbers: the computer itself and the local network stay closed.

### What the manager sees and answers

In the inbox, beside the questions, the oldest first: the specialist, the
project and the task, exactly what is asked in the type of code, why, and,
for a command, one plain sentence: *A command runs on this computer with
your rights, like a command you type yourself. Allow only what you would
run.* The answers are: allow once (for the turn the answer starts), allow
for this task (offered only in a conversation about a task), always allow
this specialist in this project, or refuse, each with an optional word.
Before allowing, the manager can narrow a command to the commands of its
program and subcommand, or edit it; the result is checked again, and the
answer is refused if it could never be allowed. Nothing is allowed that is
not shown.

A notification of the kind "waits for the manager" says "*name* asks for
permission: To run the command …" (decision 0026). The request is a step
of the turn that asked; the answer is the request of the turn it starts.
The standing permissions of the specialists are listed in Team, each with
"Take back".

### What happens after

The answer starts a new turn of the same session, as an answer to a
question does, with the origin of a decision. It tells the specialist in
words what was allowed, for how long, or that it was refused and must not
be tried another way.

Every turn of a specialist is started with the rules its requests give it
(`granted_rules`): the standing ones, those of its task, and the one the
starting answer allows once; only for that specialist, in that project,
**on the computer where the manager answered**. A command is a command of
one computer, so a permission holds only where it was given, as the folders
of decision 0013 and the services of decision 0027 belong to one computer.
The instructions of the turn list them in words.

`PermissionProfile::with_granted` adds exactly the rules to the profile of
decision 0011. A command rule names `Bash` in `--tools` (restricted mode
removes it otherwise), stops denying `Bash` as a whole, adds the rule to
`--allowedTools`, denies the programs that change files by name in
`--disallowedTools`, and adds
`"permissions":{"blockReadsOutsideWorkingDirectories":true}` to the one
`--settings` object every confined turn has (decision 0039), so that a
read-only command outside the folders prompts, and so is denied. One object
and not a second `--settings`, so that neither can replace the other. If
the installed program refused a key of that object, it might ignore the
whole object, this key with it; the keys were taken from the documentation
and never seen in a run.
A domain rule names `WebFetch`. The mode (`acceptEdits`),
`--permission-prompts none`, `--restricted` and `--strict-mcp-config` do not
change, and the rules go into the arguments only: nothing in a prompt can
widen what a turn may do. A refused request adds nothing; refused stays
refused.

### Storage

Migration 29 adds `permission_requests`: who asked, in which conversation,
turn and task, what and why, and the answer (allowed with a scope and the
rule, or refused; the word; when; on which computer; the turn it started;
when it was taken back). Every write goes through `Database::write` and is
logged as `permission_request`; it merges by field (the answer, and the
taking back), travels to the other computers like a decision request, and
goes with its project.

## Consequences

- A specialist can build, test and run the scripts of a project, and read
  documentation on the web, when the manager says so, and only that.
- A command runs with the manager's rights. A command such as `npm test` or
  `cargo build` runs code of the project, which the specialist can write.
  This is said to the manager where the command is allowed, in one plain
  sentence, and is the reason the rules of a prefix are strict.
- The read-only commands of Claude Code run without asking in the folders
  of a turn that has the shell. Outside them they are refused by the
  setting above, which the documentation describes but which was never seen
  in a run.
- A specialist must use the tool to ask. One that only writes in its report
  that it needs a command is not caught as a request; its refused attempts
  are still shown with the turn.
- What was allowed is checked when Teamwright starts a turn; how Claude Code
  matches a command against the rule is Claude Code's. The documentation
  says a deny rule on a program is not a boundary around it: a program that
  is allowed can start others. A sandbox for commands would be the next
  step, and is not part of this decision.
- Windows: commands go through `Bash`, as Claude Code's shell tool is named
  in the rules; `PowerShell` stays denied. Not tried on Windows.

## Alternatives considered

- **A permission prompt tool on the team bus** that always answers "refused
  for now, the manager has been asked". See above: in restricted mode it
  would never be asked about a command, giving every turn the shell to make
  it work opens reading everywhere, and an "allow" path is an opening that
  arguments cannot prove closed.
- **Combining both.** The tool adds nothing the request does not, for the
  reasons above, and costs the denials in the stream.
- **Turning a denial from the stream into a request.** With restricted mode
  a command is never denied: the shell is not there. It would turn refused
  file writes outside the folder into requests, which this decision does
  not allow at all.
- **Allowing writing outside the folder of the project.** Left out: the
  manager chooses the folders of a project (decision 0013), and Teamwright's
  own data and the sync folder are outside it.
- **A permission for every computer.** A command differs between computers
  and can be harmless on one and not on another; the manager allows it where
  it runs.
