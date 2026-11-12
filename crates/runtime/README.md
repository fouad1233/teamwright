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
| `arguments` | The command lines: version, sign-in, a turn, the desktop app |
| `process` | Starting the program, feeding it the prompt, reading it, stopping it |
| `decode` | Turns the JSON lines of headless mode into `AgentEvent`s and a `TurnOutcome` |
| `sign_in` | Reads the answer of `claude auth status --json` |

## How a turn runs

One turn is one process:

```text
claude -p --output-format stream-json --verbose
       [--session-id <uuid> | --resume <id>] [--model …] [--effort …]
       [--append-system-prompt=…]
       --permission-mode acceptEdits --permission-prompts none
       --restricted --strict-mcp-config
       --tools Read Glob Grep Edit Write NotebookEdit Skill ToolSearch
       --allowedTools Read Glob Grep --disallowedTools Bash PowerShell
```

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

## What Claude Code does that matters here

These were seen with Claude Code 2.1.322 on Windows, and are what the code
and the tests rely on.

- `modelUsage` in the `result` event adds up the whole session when a
  session is resumed. `model_usage_since_last_result` holds the turn alone,
  and is what the outcome reports. The same goes for `total_cost_usd`.
- A failed turn may still have the subtype `success`; `is_error` decides.
  Without a sign-in, the turn ends with the text "Not logged in" and an
  assistant event with `"error": "authentication_failed"`.
- Plan limits are reported: a `rate_limit_event` with the use of the
  five-hour and seven-day windows. They become `AgentEvent::PlanUsage`.
- A tool named in `--disallowedTools` is removed from the session. When the
  model calls it anyway, the result is the failure "No such tool", not a
  denied permission.
- With `--permission-prompts none`, tool use that would need approval, such
  as writing outside the working folder, is denied at once with a `system`
  event of subtype `permission_denied`, and listed in the result's
  `permission_denials`.
- Writing to a path that contains a Windows short name (`LONGNA~1`) is
  denied as suspicious. The system's temporary folder is often given in
  that form, so the working folder is resolved to its full long form before
  the program is started.
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

The repository is public, so a recording is cleaned before it is saved:
paths, identifiers of messages and requests, signatures of thinking, plan
usage, and the names of the user's MCP servers, plugins and skills are
replaced with sample values. The structure, the field names and the token
counts are kept exactly. Never save a recording without cleaning it.
