# teamwright-runtime

Runs Teamwright's team members on the user's own Claude Code
([decision 0005](../../docs/decisions/0005-drive-the-users-claude-code.md)).
It is the only crate that knows about Claude Code.

`ClaudeCode` implements the interface `AgentRuntime` that the application
crate declares. Nothing outside this crate knows the program's options or the
format of its output.

## What is in it

| Module | Contents |
|---|---|
| `claude_code` | `ClaudeCode`: the four things the interface asks for |
| `locate` | `Locator`: finds the program on the search path and in the documented places |
| `environment` | `Environment`: the exact set of variables the program is started with |
| `commands` | What a turn that may run a command needs: the cleared environment it is started with (`environment_for_commands`), the variables of a sign-in by a variable (`sign_in_variable`), and whether this computer can sandbox commands at all (`why_no_commands`) |
| `arguments` | The command lines: version, sign-in, a turn, the desktop app |
| `process` | Starting the program, feeding it the prompt, reading it, stopping it |
| `decode` | Turns the JSON lines of headless mode into `AgentEvent`s and a `TurnOutcome` |
| `plan_limit` | Reads from the program's sentence that the limit of the plan was reached, and the reset time it gives (`limit_in_words`, `reset_in_words`) |
| `sign_in` | Reads the answer of `claude auth status --json` |
| `team_bus` | The tool server through which a specialist asks the manager to decide or for a permission, hands its result over, writes to a teammate, and remembers and searches in the memory of the project, and the Advisor interviews and proposes |
| `memory_file` | `MemoryFile`: the current entries of the memory of a project in a file of their own for one turn, with the mark of the turn, which the team bus reads (`entries_from`) and which is removed when the turn is dropped |
| `earlier` | `EarlierConversationsOnDisk`: reads the conversations Claude Code keeps in `projects` of its settings folder, tolerant of lines it does not know, for digests ([decision 0041](../../docs/decisions/0041-earlier-conversations-as-digests.md)). The format is not published; the ignored test `reads_the_conversations_of_this_computer` reads the real ones and prints counts only |

## How a turn runs

One turn is one process:

```text
claude -p --output-format stream-json --verbose
       [--session-id <uuid> | --resume <id>] [--model …] [--effort …]
       [--append-system-prompt=…]
       --permission-mode acceptEdits --permission-prompts none
       --restricted --strict-mcp-config
       [--mcp-config <the team bus as JSON>] [--add-dir <folder> …]
       --tools Read Glob Grep Edit Write NotebookEdit Skill ToolSearch
       --allowedTools Read Glob Grep [mcp__teamwright__<tool> …]
       --disallowedTools Bash PowerShell [Edit(<folder>/**) …]  # //home/x/** on Unix, C:/x/** on Windows
```

The built-in tools and the rules come from the `PermissionProfile` of the
turn; the Advisor's (`PermissionProfile::advisor()`) has only `Read`,
`Glob` and `Grep`.

A turn with team tools (`team_tools`) is given the team bus when the
runtime knows how to start it (`ClaudeCode::with_team_bus`): the
configuration `{"mcpServers":{"teamwright":{"command":…,"args":[…]}}}` as
one argument, whose arguments are the configured ones followed by the
names of the tools and `--member <name>` for each member the project has
(`team_bus::bus_arguments`), and the full name of each tool,
`mcp__teamwright__<name>`, among the allowed tools. Without a bus the turn
runs without them. A folder the session may only read
(`readable_folders`) is resolved to its full form like the working folder,
added with `--add-dir`, and denied for writing with the rule
`Edit(//home/x/**)` on Unix, where one slash would mean "from the work
folder", and `Edit(C:/x/**)` on Windows. Characters a rule could misread
(spaces, commas, parentheses) are written as `?` (`write_rule_path` in
`arguments.rs`; decision 0013).

A folder to write in (`TurnRequest::written_folders`) is resolved the same
way and added with `--add-dir`, with no rule against writing. A turn with
full access (`TurnRequest::full_access`) keeps every other option of a
confined turn and adds the root as a working folder (`--add-dir /`; on
Windows each drive that exists). Each protected place
(`TurnRequest::protected_places`, in its full form, also when it does not
exist) is denied with `Read(…)` and `Edit(…)` in the same form, a folder
with `/**` and a file alone: every one of them with full access, and
without it those that lie inside a folder of the turn. A turn with full
access and a command is refused. `protected_places_in_home` names the
places of the home folder: `~/.claude`, `~/.claude.json`, the folder of
`CLAUDE_CONFIG_DIR`, `~/.ssh`, `~/.gnupg` and on Windows `%APPDATA%\gnupg`
([decision 0044](../../docs/decisions/0044-folders-to-write-in-and-full-access.md)).
The ignored tests in `tests/real_program.rs` proved both on Linux.

The prompt goes to standard input, never onto the command line: Windows
limits the length of a command line, and quoting would differ per system.
Standard output is read line by line on a thread of its own; every line is
decoded and its events go to the listener as they happen. The last lines of
standard error are kept for the message when the program ends without
finishing the turn. The listener's last event is always `TurnEnded`.

A later turn of the same session is a new process with `--resume <id>`. It
works in the folder the request names, which need not be the one the session
started in.

A Claude Code that is a script cannot run a turn. Installed through npm, it
is `claude.cmd` on Windows, and Windows hands a script its arguments through
the command prompt, which cannot take instructions of several lines. Such an
installation is still found and asked for its version and its sign-in, and a
turn on it is refused with words that say so and what to do: install Claude
Code with its own installer, which puts a `claude.exe` there.

Every option was checked against `claude --help` of Claude Code 2.1.322.
`--verbose` is required: without it Claude Code refuses `stream-json` output
in print mode. `--bare` is never used, because it ignores the subscription
sign-in. `--max-turns`, which older documentation mentions, is not in the
help of that version and is not used.

## Skills and connectors

[Decision 0039](../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md).
A confined turn always gets `--setting-sources=` and `--settings` with
`disableAllHooks`, `disableSkillShellExecution` and
`disableClaudeAiConnectors` (`kit::SESSION_SETTINGS`). Each skill of
`TurnRequest::kit` is a `--plugin-dir`, readable through `--add-dir` and
denied for writing, allowed as `Skill(<name>:<name>)`. Each connector is an
entry of the one `--mcp-config`, beside the team bus. Its tools are allowed
one by one as `mcp__<name>__<tool>`. The exact command line is tested in
`arguments.rs` and `kit.rs`.

A connector at an address is `{"type": "http", url, headers}`; Claude Code
fills `${NAME}` in a header from its own environment, so those variables stay
in the environment of a turn that is given that connector.

A connector that is a program is never given to Claude Code as it is
(note of 21 November 2026 in decision 0039). Claude Code would start it in
the folder of the project, which the specialist writes, with the whole
environment. Its entry is Teamwright's own program in the mode of
`connector_launcher.rs`:
`teamwright --connector --folder <data>/connectors/starts/teamwright-start-…
[--secrets <file>] [--pass NAME]... -- <program> <arguments>`. Each
connector of each turn starts in a fresh empty folder of its own
(`StartFolder`, 0700), made by `ClaudeCode::start_turn` and removed with
whatever the connector wrote there when the turn ends, so that no connector
can plant a module for another one; left-over folders are removed at
start-up (`remove_left_over_start_folders`; note of 22 November 2026 in
decision 0039). The launcher refuses a
folder that is not empty or not a full path, clears the environment, sets
`BASE_VARIABLES` (PATH, HOME, user, locale, temporary folder on Unix; the
system folders on Windows) and the named variables, finds a bare program
name only in folders of the search path that are full paths, changes to the
folder, and execs the program (on Windows it waits and passes the exit code
back). It writes nothing to standard output. The values of the variables
are written by `ClaudeCode::start_turn` to a file of
`<data>/connectors/secrets` (0600, folder 0700), one per connector, removed
when the turn ends (`SecretsFile`); left-over files are removed at start-up.
`ClaudeCode::with_connector_launcher` sets this up; without it a turn given
a program connector is refused.

Every turn starts without the variables in `TurnKit::withheld_variables`
(all those named by the library of the project) and those of its program
connectors, except header variables of its address connectors, the base
variables, and proxies and certificates (`KEPT_FOR_THE_PROGRAM`), which
Claude Code needs.

The launcher is started for real in `tests/process.rs` (a connector that
prints its folder and environment). The ignored test
`a_real_connector_starts_through_the_launcher_and_sees_only_its_own_variable`
needs a signed-in Claude Code, `python3` and the built program; it has never
run.

A turn that may run a command (decision 0038) has, in the same one
`--settings` object, `permissions.blockReadsOutsideWorkingDirectories` and
`sandbox` with `enabled`, `failIfUnavailable`, `autoAllowBashIfSandboxed:
false`, `allowUnsandboxedCommands: false`, `network.strictAllowlist` and
`filesystem.denyWrite` with the folders it may only read
(`command_settings` in `arguments.rs`). The keys were read in the
documentation and found in Claude Code 2.1.289; none has been seen working.
The ignored test
`a_real_allowed_command_runs_in_a_sandbox_that_keeps_it_inside_the_folders`
would prove that a sandboxed command cannot read a file in the home folder.

A sandboxed command inherits the environment of Claude Code, so a turn that
may run a command is started with a cleared environment: the base set of
the connector launcher (`PATH`, `HOME`, the user, the locale, `TZ`,
`TMPDIR`), `SHELL`, `CLAUDE_CONFIG_DIR` and the variables of a proxy, and
nothing else (`environment_for_commands`; note of 22 November 2026 to
decision 0038). A manager signed in with a variable (`ANTHROPIC_…`,
`CLAUDE_CODE_OAUTH_TOKEN`, Bedrock, Vertex) runs no command on that
computer. `why_no_commands` answers the application: never on Windows or
the first WSL; on macOS `/usr/bin/sandbox-exec` must be there; on Linux
`bwrap` and `socat` must be on the search path and `bwrap` must make and
leave an empty sandbox, and then one more user namespace inside it, as the
sandbox of Claude Code does (`NoCommands::NoNestedNamespaces` when only that
fails: Ubuntu 24.04 as installed). `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB` is not set: in
2.1.289 it forces the permission mode back to `default`. The ignored test
`a_real_allowed_command_sees_none_of_the_managers_secrets` would prove that
an allowed command sees none of the manager's variables, and
`this_computer_can_make_the_sandbox` (in `commands.rs`) passes on a Linux
computer with `bwrap` and `socat` that allows nested user namespaces, and
fails on Ubuntu 24.04 as installed, as it should.
The ignored test
`a_real_session_gets_its_chosen_skill_and_connector_and_nothing_of_the_users_own`
in `tests/real_program.rs` has never run (needs the built program:
`cargo build -p teamwright-desktop`).

## The team bus

`team_bus::serve` is a tool server (MCP, one JSON-RPC 2.0 message per line
on standard input and output) named `teamwright`. It offers the tools it is
started with (`team_bus::tools_from_arguments` reads them from its
arguments):

- `ask_manager`: a specialist calls it when a choice changes what is built,
  what it costs or when it is ready. The arguments are a `question`, a
  `reason`, two to four `options` with a `label` and a `consequence`, and
  the option it `recommended`.
- `interview`: the Advisor asks one to five `questions`, each with a
  `reason`, and two to four `options` with one `recommended`, or none for a
  question answered in words.
- `propose_plan`: the Advisor proposes a `summary`, at most eight `members`
  (a name, a responsibility, a reason, a `model` of one word and an
  `effort` of `low`, `medium`, `high`, `xhigh` or `max`), and one to forty
  `tasks` (a title, a description, an `owner`, `estimate_hours`, a
  `priority`, normal unless said, the tasks it `waits_for`, and, for a part
  of a larger task, the task it is `part_of`). An owner
  is `manager`, a proposed member, or a member the project has, whatever
  the capitals. A task waits only for other tasks of the list, and not in a
  circle. A part names another task of the list, the tasks are not parts
  of each other in a circle, and a part does not wait for the task it is a
  part of, directly or through others (that task starts only once its
  parts are finished). A member the project has is not proposed again: the call is
  refused with the names and with what to leave out, because a model that
  planned a project once tends to hand in the whole plan a second time.

- `publish_handoff`: a specialist working on a task hands its result over
  to whoever builds on it: a `summary`, a `contract` (may be left out) and
  the `files` that hold the result, as paths inside the working folder. The
  server is told the folder (`--folder <path>`) and refuses a path that
  leaves it, a file named twice, more than fifty files, and a file that is
  not there, naming it. A turn hands over once.
- `send_message`: a specialist sends one teammate (`to`, one of the names
  the server was told with `--teammate <name>`, whatever the capitals) a
  `question` or a `note` (`kind`) with a `text` of at most 4,000
  characters. A turn sends at most five.

- `remember`: a specialist records a fact in the memory of the project: a
  `title` of one line (at most 120 characters) and a `text` of at most
  2,000. A turn remembers at most three.
- `search_memory`: a specialist searches the memory with a few plain words
  (`query`). The server answers with the entries that fit, the newest
  first, at most eight, each text shortened to 600 characters, with where
  each came from, its task and its day (`found_in_words`), and with how to
  narrow the search if more fit. Each entry stands in a block of teammate
  words that carries the mark of the turn, which the file of memory holds
  (format version 2), so that its text cannot close the block or pass for
  the manager's (decision 0029, note of 22 November 2026). It is the one tool that answers with
  data. The server never opens the database: the runtime writes the
  current entries to a file (`memory_file`, readable by the user alone)
  when the turn starts, names it with `--memory <file>` among the
  arguments the bus is started with, and removes it when the turn is over.
  A fact the turn remembered is found by a later search of the same turn
  ([decision 0036](../../docs/decisions/0036-project-memory.md)).

- `ask_permission`: a specialist asks the manager for permission to do
  something it may not do on its own: a `kind` (`command` or
  `web_domain`), `what` (the command line or the domain) and a `reason`, checked by the rules of a permission of
  the domain (`permission_from`). A command can be asked for only when the
  bus is started with `--commands` after the tool's name; without it the
  tool offers `web_domain` only and refuses a command with the reason in
  words, and the decoder does not report one (`with_commands_askable`). Like a question, it ends the turn: the
  server answers that nothing is allowed yet and that the answer comes as
  the next message, and the decoder reports `PermissionAsked`
  ([decision 0038](../../docs/decisions/0038-permission-requests-routed-to-the-manager.md)).
  No `--permission-prompt-tool` is given: nothing in a running turn can
  approve anything.

None of `publish_handoff`, `send_message`, `remember` and `search_memory` ends the turn: the server answers that the
handoff is recorded, that the message is on its way, that the fact is
remembered or what the search found, and the session goes on
([decision 0029](../../docs/decisions/0029-handoffs-and-messages-between-specialists.md)).

A model counts from 1, so `recommended`, `waits_for` and `part_of` do; what comes out
counts from 0. An estimate in hours becomes minutes. Texts are trimmed, and
names and titles follow the rules of members and tasks. The names, the
descriptions and the schemas are those of the stand-in that was tried
against Claude Code before the tools were built
([decision 0014](../../docs/decisions/0014-the-advisor-plans-through-two-tools.md)).

The server does no more than check a call and answer the session. One
function for each tool, `team_bus::question_from`, `interview_from`,
`proposal_from`, `handoff_from` and `message_from`, checks the arguments
for the server and for the decoder; `missing_files` is the one check only
the server makes, because it looks at the disk. A refusal says in plain words what to change, because
the model reads it and tries again. A turn carries at most one question,
interview or proposal: after one is taken, the server refuses every further
call of the three.

The server answers `initialize`, `tools/list`, `tools/call` and `ping`. Any
other request is answered with the JSON-RPC error `-32601`; notifications
and lines that are not JSON get no answer. Claude Code 2.1.322 first sends a
request `server/discover`, which is fine to refuse that way.

In the output of the turn the call appears as a use of the tool
`mcp__teamwright__<name>`. The decoder, made for the team of the turn,
turns the first one that passes the check into `AgentEvent::ManagerAsked`,
`InterviewAsked` or `PlanProposed` and nothing else: no tool use, and the
result of the call is swallowed. A call that does not pass, and any after
the first, is an ordinary tool use with a failed result. The model then
writes one last sentence, and the turn ends with an ordinary `result`
event.

A call of `publish_handoff` or `send_message` that passes the check is held
until the answer of the server comes back in the stream. If the server took
it, it becomes `AgentEvent::HandoffPublished` or `MessageSent`; if the
server refused it (a file that is not there, or a sixth message), it
becomes an ordinary tool use with a failed result. The decoder is told the
names of the teammates (`Decoder::with_teammates`), as the server is. A
call of `remember` is held the same way and becomes
`AgentEvent::FactRemembered`, because the server allows only a few in a
turn. A call of `search_memory` changes nothing and is an ordinary tool
use, with the words searched as its summary.

## Rules for code in this crate

- **No credentials.** The sign-in state is asked of the program
  (`claude auth status --json`). Only whether the user is signed in, the
  method, the provider and the kind of subscription are read; the e-mail
  address and organization in the same answer are never put into a field,
  and error messages never repeat the answer. No credential file or key
  store is read.
- **The decoder is lenient.** The output format belongs to Claude Code and
  changes with its versions. Lines that are not JSON, unknown event types
  and unknown or missing fields are passed over. Only the decoder needs to
  change when the format does.
- **Only the given environment.** The program is started with exactly the
  variables of an `Environment`. `ClaudeCode::new` passes on the
  environment of Teamwright's process, without the variables by which a
  running Claude Code session marks the programs it starts.
- **No console window.** On Windows the program is started with
  `CREATE_NO_WINDOW`, because Teamwright is a windowed program.
- **No rules about the team.** What a team member may do is decided by the
  application and arrives as a `PermissionProfile`; this crate only
  translates it.

## A turn with allowed rules

A profile that `PermissionProfile::with_granted` widened (decision 0038)
reaches the arguments like any other: `Bash` among `--tools` when a command
is allowed, `WebFetch` when a domain is, each rule in `--allowedTools`
(`Bash(npm test)`, `Bash(cargo test *)`, `WebFetch(domain:docs.rs)`), the
programs that change files in `--disallowedTools` (`Bash(rm *)`, …), and,
for commands, `"permissions":{"blockReadsOutsideWorkingDirectories":true}`
(`READS_BLOCKED_SETTINGS`) joined to the one `--settings` object of a
confined turn (`settings_of`), so that a command that reads outside the
folders would need approval, which is denied. `--permission-mode
acceptEdits`, `--permission-prompts none`, `--restricted` and
`--strict-mcp-config` stay. These flags and rules were read in
`claude --help` of 2.1.289 and in the documentation; a turn with them has
never run (the test `a_real_session_asks_to_run_a_command_and_runs_it_once_allowed`
in `tests/real_program.rs` would prove it).

## What Claude Code does that matters here

These were seen with Claude Code 2.1.322 on Windows, and are what the code
and the tests rely on.

- `modelUsage` in the `result` event adds up the whole session when a
  session is resumed. `model_usage_since_last_result` holds the turn alone,
  and is what the outcome reports. The same goes for `total_cost_usd`.
- Every `assistant` event carries the `usage` of the request it answers:
  `input_tokens`, `cache_read_input_tokens` and
  `cache_creation_input_tokens` add up to everything the session sent the
  model then. The last such event of the session itself (not of a helper,
  whose events have a `parent_tool_use_id`) is how full the context is;
  its `output_tokens` there are only what was written so far. Each model in
  `modelUsage` and `model_usage_since_last_result` names its
  `contextWindow` (1,000,000 for every model in the recordings). Together
  they become `TurnOutcome::context` (`SessionContext`). A turn that never
  reached a model (the made-up message of `<synthetic>`, all zero) has no
  context, and a turn that ended without a `result` has no window. What the
  program reports when it makes room in a full session (the context then
  shrinks) has not been seen.
- A failed turn may still have the subtype `success`; `is_error` decides.
  Without a sign-in, the turn ends with the text "Not logged in" and an
  assistant event with `"error": "authentication_failed"`.
- Plan limits are reported: a `rate_limit_event` with the use of the
  five-hour and seven-day windows. They become `AgentEvent::PlanUsage`,
  with the `status` of the plan (`allowed`, `allowed_warning` or
  `rejected`).
- A turn that meets the limit of the plan ends as
  `TurnFailure::PlanLimitReached`
  ([decision 0040](../../docs/decisions/0040-the-limit-of-the-plan.md)):
  from a `rate_limit_event` whose status is `rejected` (its
  `rateLimitType` and `resetsAt`), or else from the program's sentence,
  "You've hit your 5-hour limit · resets 10:40pm (Europe/Istanbul)" and
  its other wordings, which `plan_limit` reads. The reset in words is
  passed on as it was said (`PlanReset::Said`); the application makes it a
  moment. No such turn was recorded: the tests build its stream from the
  schema inside the program (2.1.289) and from the words seen on
  21 November 2026. An `error` of `rate_limit` alone, such as an API
  key's "Rate limit reached", stays an ordinary failure.
- A tool named in `--disallowedTools` is removed from the session. When the
  model calls it anyway, the result is the failure "No such tool", not a
  denied permission.
- With `--permission-prompts none`, tool use that would need approval, such
  as writing outside the working folder, is denied at once with a `system`
  event of subtype `permission_denied`, and listed in the result's
  `permission_denials`.
- Paths are compared as they are written. A path that contains a Windows
  short name (`LONGNA~1`) is taken to be outside the folders of the
  session, even if it names a file inside them: reading it, searching it
  and writing to it are refused. The system's temporary folder is often
  given in that form. So the working folder and the folders a session may
  read are resolved to their full long form before the program is started
  (`full_form`), and whoever names a folder to a session must name it in
  that form too. In its full form, a folder added with `--add-dir` can be
  read and searched (`Read`, `Glob` and `Grep` were tried), also when its
  name has a space and brackets in it.
- A rule that allows a tool without naming a path, such as `Read` in
  `--allowedTools`, allows it everywhere. With it alone, a session read a
  file outside its working folder when asked to. `--restricted` confines
  the file tools to the working folder, also for reading, and that is why a
  specialist runs with it.
- `--restricted` also ignores the user's settings files, so their hooks,
  plugins and rules do not apply, and with `--strict-mcp-config` the session
  has none of the user's MCP servers and claude.ai connectors either. The
  sign-in still works. A session without all of that is also much lighter:
  its first request wrote about 6,500 tokens to the cache instead of 25,000.
- `--tools` ignores names it does not know.

## Stopping a turn

`RunningTurn::stop` ends the Claude Code process. A program that Claude Code
itself started, such as a local MCP server or a command of a hook, is not
ended with it: ending a whole tree of processes would need the Windows API
for job objects, which needs `unsafe` code. Such a program ends when it
notices that Claude Code is gone. If one of them keeps the output of the
turn open, the turn ends only when that program ends.

## Tests

```bash
cargo test -p teamwright-runtime
```

None of the default tests needs Claude Code or a network:

- the decoder runs over recordings of real turns in `tests/recordings/`;
- the command lines, the locator (with search paths given by the test) and
  the reading of the sign-in answer have unit tests;
- `tests/process.rs` starts, feeds, reads and stops a stand-in for Claude
  Code. The stand-in is the test program itself, started again with the
  variable `TEAMWRIGHT_STAND_IN`. That file has `harness = false` and its
  own `main`, so that nothing else writes to standard output while it plays
  the stand-in.

`tests/real_program.rs` runs the real program. Its tests are ignored by
default, need Claude Code installed and signed in, and spend a little of the
user's plan on the cheapest model:

```bash
cargo test -p teamwright-runtime --test real_program -- --ignored --nocapture --test-threads 1
```

One of them is the proof of what a session can reach
(`a_real_session_reaches_its_folders_and_nothing_else`). It gives a session
its working folder, a folder to read and a folder it was not given, and asks
it to try everything. It passes if the folder to read was read and searched
and not written to, and the third folder was neither read nor written to.
Run it after every change to the permissions of a turn and after a new
version of Claude Code. A specialist that runs through Teamwright cannot
stand in for this test: it is told which folders are its own, and declines
to leave them without trying.

Another (`the_own_instructions_of_a_specialist_reach_a_real_session`) gives
a session the instructions the application builds for a specialist whose
own instructions say to end every reply with the word PERSIMMON, and asks
it a short question that does not name the word. It passes if the last line
of the answer is that word: the instructions a manager writes reach the
session through the appended system prompt
([decision 0031](../../docs/decisions/0031-instructions-and-ready-made-specialists.md)).

### Recordings

Each file in `tests/recordings/` is the standard output of one real turn of
Claude Code 2.1.322, made by hand from an empty folder outside the
repository:

| File | What the turn did |
|---|---|
| `text_reply.jsonl` | Answered one word |
| `file_written.jsonl` | Wrote a file with the specialist's permissions |
| `resumed_with_denials.jsonl` | Resumed that session; a shell command and a write outside the folder were refused |
| `not_signed_in.jsonl` | Ran without a sign-in |
| `manager_asked.jsonl` | Was given a tool server named `teamwright` that answers as `team_bus` does, and asked the manager how a sensor node is powered |
| `interview_asked.jsonl` | The Advisor, with a stand-in for the bus that had `interview` and `propose_plan`, looked into the folder and asked five questions about a device that measures the air |
| `plan_proposed.jsonl` | The same session, resumed with the answers, proposed three members and sixteen tasks |

The repository is public, so a recording is cleaned before it is saved:
paths, identifiers of messages and requests, signatures of thinking, plan
usage, and the names of the user's MCP servers, plugins and skills are
replaced with sample values. What the event about the limits of a plan
says about extra usage (`overageStatus`, `overageDisabledReason`) is taken
out, because it describes the account that made the recording. Apart from
that, the structure, the field names and the token counts are kept exactly.
Never save a recording without cleaning it.
