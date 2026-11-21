# 0039. Give a specialist only the skills and connectors the manager chose, tool by tool

- Status: accepted
- Date: 2026-11-21
- Builds on: [0005](0005-drive-the-users-claude-code.md),
  [0011](0011-what-a-specialist-may-do.md),
  [0013](0013-the-folders-of-a-project.md),
  [0027](0027-integrations.md)

## Context

A specialist is a Claude Code session confined to its folder
([0011](0011-what-a-specialist-may-do.md)). It sees nothing of the
manager's own setup. Some work needs more: a skill (a folder with a
`SKILL.md` that teaches a kind of work) or a connector (an MCP server
that reaches a ticket system or a parts catalog).

What "sees nothing" rests on was read again in `claude --help` of the
installed 2.1.289 and the documentation at code.claude.com in November
2026. `--restricted` ignores the user's, project and local settings files
and confines the file tools. `--strict-mcp-config` skips every MCP server
not given with `--mcp-config`. But the documentation does not say that
restricted mode leaves out the personal skills in `~/.claude/skills`
(they come from the setting source `user`), nor that the strict flag
leaves out the connectors of the manager's claude.ai account. And a skill
can run commands three ways: `hooks` in its front matter, `` !`…` `` and
` ```! ` blocks run when it is used, and `allowed-tools`, which approves
tools for the turn that uses it.

## Decision

1. **A library per project, a kit per specialist.** The manager adds
   skills and connectors to the library of the project, and gives each
   specialist some of them. Both are project data: logged (entities
   `skill`, `connector`, `member_kit`, migration 25), synced, merged whole.
   The ready-made specialists suggest nothing, and no connector of a third
   party ships with Teamwright.
2. **A skill is copied, and the copy belongs to the computer.** As a
   single file is copied in [0013](0013-the-folders-of-a-project.md), the
   folder is copied onto a shelf in Teamwright's data folder, in the form
   of a plugin: `.claude-plugin/plugin.json` and `skills/<name>/`, nothing
   else. A copy is fixed and checked; a folder referred to could change
   between the check and the turn. Only the name and description travel
   to other computers; another computer shows the skill as "not on this
   computer" until the manager adds the folder there too, and its turns go
   without it.
3. **The check of a skill is a rule of the domain.** It needs front
   matter with a description, a name of the open format (lower-case,
   digits, hyphens, at most 64), and it refuses `hooks`, `allowed-tools`
   beyond `Read`, `Glob` and `Grep`, and commands run when the skill is
   used. A folder with a link or larger than 200 files or 10 MiB is
   refused. The copy is checked again before every turn.
4. **A connector keeps no secret.** It is a program with arguments or an
   `https://` address (`http://` only on this computer itself), and the
   names of its tools. A secret is named by an environment variable,
   never given a value: the program is handed `NAME: ${NAME}`, an address
   a header `${NAME}` or `Bearer ${NAME}`, and Claude Code replaces them
   from the environment Teamwright was started in. No value reaches the
   database, the log, the sync folder or a command line. A `$` in a
   program, an argument or an address is refused, so that no other
   variable can be handed over; an address with a user and password or a
   parameter named like a secret is refused; variables named `CLAUDE…` or
   `ANTHROPIC…` are refused. A connector whose variables are not set on
   this computer is shown as such and left out of its turns.
5. **A connector is given tool by tool.** It is a way out of the folder,
   so each tool is allowed by name (`mcp__<name>__<tool>`); there is no
   rule that allows a server whole, nor a pattern. Its other tools are
   denied at once, as nobody can be asked. The interface says: this
   specialist can reach whatever this connector reaches.
6. **Every confined turn now also gets** `--setting-sources=` (no source:
   no personal or project skills, commands or agents) and `--settings`
   with `disableAllHooks`, `disableSkillShellExecution` and
   `disableClaudeAiConnectors`. Each skill comes with `--plugin-dir`, is
   readable through `--add-dir` and denied for writing, and allowed as
   `Skill(<name>:<name>)`. Connectors share the one `--mcp-config` with
   the team bus.

## Consequences

- The manager decides, per specialist and per tool, what reaches outside.
  What is given is shown on the specialist's row.
- A program connector runs on this computer as the manager, like any
  program the manager starts. The tool rules limit what the session may
  call, not what the program can do.
- An address travels with the project. A service that puts its secret in
  the path of the address cannot be added safely; the interface says so.
- None of this was run against the real program tonight: it was not
  signed in. The arguments are tested exactly; the ignored test
  `a_real_session_gets_its_chosen_skill_and_connector_and_nothing_of_the_users_own`
  is the proof still to be had. Item 6 changes every confined turn, so
  the earlier real tests must be run again too.

## Alternatives considered

- **Refer to the skill's folder in place.** Fresh, but unchecked between
  the check and the turn, and an `--add-dir` skill depends on the setting
  source `project`, which item 6 turns off.
- **Keep tokens in Teamwright.** It would need a safe store and would put
  secrets in a log that travels; [0005](0005-drive-the-users-claude-code.md)
  and [0027](0027-integrations.md) avoided it, and so does this.
- **Allow a connector's tools wholesale** (`mcp__<name>__*`). Simple, and
  a tool added to the server later would be allowed without the manager.
- **Keep the library on one computer only.** The kits would name things
  another computer does not know; the library is the team's.
