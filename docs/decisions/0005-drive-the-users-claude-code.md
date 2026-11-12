# 0005. Drive the user's own Claude Code in headless mode

- Status: accepted
- Date: 2026-11-11
- Changed by: [0010](0010-one-process-for-each-turn.md), which runs one
  process for each turn instead of one for each active team member

## Context

Teamwright's team members are Claude Code sessions. There are three ways a
program can run one:

1. The Claude Agent SDK, which exists for TypeScript and Python only.
2. The model API directly, which needs an API key and gives up everything
   Claude Code adds: tools, skills, plugins, MCP connectors, permissions.
3. The Claude Code command-line program in its documented headless mode,
   where it reads prompts and writes events as lines of JSON.

Anthropic's terms set limits that matter here. A third-party product may not
offer its own Claude sign-in or handle Claude credentials, and the Claude Code
program must not be modified. A user signing in to the unmodified program with
their own subscription is explicitly allowed.

On 11 November 2026 the headless mode was run on the development machine with
Claude Code 2.1.321. It produced a `system` event describing the session
(model, tools, skills, plugins, MCP servers), `assistant` events, and a
`result` event with token usage per model. Run without a signed-in user it
answered "Not logged in" as an ordinary result with `is_error` set.

## Decision

- Each active team member is one Claude Code process started as
  `claude -p --input-format stream-json --output-format stream-json`, fed
  turns on standard input and read from standard output.
- A session is identified by its session ID and resumed with it. A team
  member with nothing to do has no process.
- Teamwright looks for an installed Claude Code and uses it as it is. It never
  reads, stores or forwards credentials, and it offers no sign-in of its own.
  If Claude Code is missing or signed out, the app explains what the user has
  to do.
- Everything that knows about Claude Code lives in one crate, behind an
  `AgentRuntime` interface that the rest of the system depends on.

## Consequences

- The user must install Claude Code and sign in once. Usage counts against
  the user's own plan.
- The event format belongs to Claude Code and can change with its versions.
  The parser accepts unknown events and fields, is tested against recorded
  output, and is the only place to update.
- The user's skills, plugins and connectors work for the team without extra
  configuration, and can be restricted per team member.
- Another agent runtime can be added later as a second implementation of the
  same interface.

## Alternatives considered

- **Agent SDK in a helper process.** Adds a Node.js or Python runtime to
  every install to wrap the same command-line program.
- **Model API.** Excludes subscription users and reimplements what Claude
  Code already does well.
