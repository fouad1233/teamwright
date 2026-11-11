# Landscape

*Surveyed on 11 November 2026 from each project's README, documentation or
product page. Star counts come from the GitHub API on that day. We read what
the projects say about themselves; we did not audit their code. Corrections
are welcome through an issue or a pull request.*

Teamwright was started in the belief that nothing like it existed in open
source. A survey on the first day showed that this was wrong in an important
way: almost every individual idea in Teamwright already exists somewhere. This
page records what we found, so that the project's claims stay honest and so
that we learn from the people who came first.

## Summary

- Putting AI agents on a task board, giving them roles in a "company", and
  drawing them as characters in a pixel office have all been done in open
  source, several times.
- We did not find one project that combines an interviewing advisor that
  proposes a right-sized team, full project-management views (board, timeline
  with dependencies, sprints), engineering work beyond software, structured
  decision requests, and sync through storage the user owns.
- That combination, built carefully for a manager who stays involved, is what
  Teamwright is for. Being first is not the claim.

## Agents as workers on a board or in a company

| Project | License | Stars | What it says it does |
|---|---|---:|---|
| [Paperclip](https://github.com/paperclipai/paperclip) | MIT | 113k | An agent "company": org chart, goals, tickets, heartbeats, budgets, approvals by a human board. Runs Claude Code headless, and other agent CLIs. Web app. |
| [Multica](https://github.com/multica-ai/multica) | Custom (Apache-2.0 with extra limits) | 56k | Issues assigned to agents like teammates, squads with a routing leader, an inbox, review gates. A local daemon starts agent CLIs. Desktop, web and mobile. |
| [Plane](https://github.com/makeplane/plane) | AGPL-3.0 | 62k | A general project-management tool. Agents were announced in September 2026 for paid plans. |
| [Vibe Kanban](https://github.com/BloopAI/vibe-kanban) | Apache-2.0 | 29k | A kanban of issues with one agent workspace each. Its company [shut down in April 2026](https://www.vibekanban.com/blog/shutdown); the project continues as community-maintained open source. |
| [Gas Town](https://github.com/gastownhall/gastown) and [Beads](https://github.com/gastownhall/beads) | MIT | 19k / 29k | A coordinator agent, worker agents, mail, escalations and a merge queue in the terminal; Beads is a dependency-graph issue tracker made for agents. |
| [Agent Orchestrator](https://github.com/OrchestratorInc/agent-orchestrator) | Apache-2.0 | 14k | A main agent plans and starts one worker per git worktree. The board has a "Needs you" lane. |
| [It's a Plan](https://github.com/croffasia/itsaplan) | AGPL-3.0, paid tier | 2k | A tracker with agents as assignees: kanban, table, timeline, calendar, cycles. Web app with a database server. |
| [Symphony](https://github.com/openai/symphony) | Apache-2.0 | 28k | Watches an issue board and runs one coding agent per issue. |
| [Claude Code agent teams](https://code.claude.com/docs/en/agent-teams) | Proprietary | | A lead and teammates with a mailbox and a shared, dependency-aware task list. Experimental, and not available in headless mode. |

Smaller or frozen relatives include
[Aperant](https://github.com/AndyMik90/Aperant),
[Cline Kanban](https://github.com/cline/kanban),
[Mission Control](https://github.com/builderz-labs/mission-control),
[Nimbalyst](https://github.com/nimbalyst/nimbalyst),
[Automaker](https://github.com/AutoMaker-Org/automaker), and the
parallel-session managers [Emdash](https://github.com/generalaction/emdash),
[Claude Squad](https://github.com/smtg-ai/claude-squad) and
[Conductor](https://conductor.build).

## Agents as characters

| Project | License | Stars | What it says it does |
|---|---|---:|---|
| [Pixel Agents](https://github.com/pixel-agents-hq/pixel-agents) | MIT | 10k | Shows and launches Claude Code sessions as characters in an office, with speech bubbles when one needs input and bars for plan limits and context. An editor extension. |
| [Claw-Empire](https://github.com/GreenSheep01201/claw-empire) | Apache-2.0 | 1.4k | An office simulator run from a "CEO desk": departments, meetings, a kanban, and work packs beyond software such as reports and video pre-production. Local database. |
| [bagidea-office](https://github.com/bagidea/bagidea-office) | MIT | 0.3k | A desktop office where a director agent plans, with an approvals queue, a kanban and per-agent budgets. |
| [Outworked](https://github.com/outworked/outworked) | GPL-3.0 | 0.4k | A desktop pixel office with a boss agent that routes work, agent-to-agent questions, approvals and per-agent spend. |
| [OpenOffice](https://github.com/longyangxi/OpenOffice) | MIT | 0.3k | A leader-coordinated team in a pixel office with per-agent token cost. |
| [Star-Office-UI](https://github.com/ringhyacinth/Star-Office-UI) | MIT code, non-commercial art | 8k | A status display for agents. |
| [AI Town](https://github.com/a16z-infra/ai-town) | MIT | 11k | A social simulation. The characters talk; they do not do project work. |

## Methods and building blocks

| Project | What it contributes |
|---|---|
| [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode) | A Socratic "deep interview" before a team pipeline. The user chooses the team size. |
| [CCPM](https://github.com/automazeio/ccpm) | Guided brainstorming into a requirements document, an epic, and tasks with dependencies. |
| [BMAD-METHOD](https://github.com/bmad-code-org/BMAD-METHOD) | An agile method packaged as skills. |
| [Claude Task Master](https://github.com/eyaltoledano/claude-task-master) | Requirements into dependency-ordered tasks. |
| [ruflo](https://github.com/ruvnet/ruflo) | A swarm layer inside Claude Code. |
| [MetaGPT](https://github.com/FoundationAgents/MetaGPT), [ChatDev](https://github.com/OpenBMB/ChatDev), [CrewAI](https://github.com/crewAIInc/crewAI), [AutoGen](https://github.com/microsoft/autogen), [OpenHands](https://github.com/OpenHands/OpenHands) | Role-playing agent frameworks that call model APIs directly. |
| [claude-hardware-in-the-loop](https://github.com/Kemerd/claude-hardware-in-the-loop) | A Claude Code skill for circuit boards, parametric CAD and firmware. |
| [agency-agents](https://github.com/msitarzewski/agency-agents) | A large library of role definitions, including embedded and electrical engineering. |

Closed-source reference points are
[ClickUp Super Agents](https://clickup.com/blog/super-agents/) and
[Linear's agents](https://linear.app/agents), where an agent can be assigned
work like a teammate.

## Capability comparison

● stated in the project's own material · ◐ partly · ○ not found

| | Board, timeline and sprints | Advisor proposes team and plan | Structured decision requests | Agent-to-agent handoffs | Non-software engineering | Office view | Per-agent usage | User-owned sync | Form |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|---|
| Paperclip | ◐ | ◐ | ● | ◐ | ○ | ○ | ● | ○ | Web |
| Multica | ◐ | ○ | ◐ | ◐ | ○ | ○ | ● | ○ | Desktop, web |
| It's a Plan | ◐ | ○ | ◐ | ◐ | ○ | ○ | ● | ○ | Web |
| Gas Town | ○ | ◐ | ◐ | ● | ○ | ○ | ○ | ○ | Terminal |
| Claude Code agent teams | ○ | ◐ | ◐ | ● | ○ | ○ | ◐ | ○ | Terminal |
| Claw-Empire | ◐ | ◐ | ◐ | ◐ | ◐ | ● | ○ | ○ | Web |
| bagidea-office | ◐ | ◐ | ● | ◐ | ○ | ● | ● | ○ | Desktop |
| Outworked | ○ | ◐ | ◐ | ● | ○ | ● | ● | ○ | Desktop |
| Pixel Agents | ○ | ○ | ◐ | ○ | ○ | ● | ◐ | ○ | Extension |
| **Teamwright (goal)** | ● | ● | ● | ● | ● | ● | ● | ● | Desktop |

The last row is what Teamwright aims for, not what it does today. The
[roadmap](roadmap.md) tracks the difference.

## What we take from it

**Tokens are the scarce resource.** Claude Code's own documentation says agent
teams use roughly seven times the tokens of a standard session when teammates
run in plan mode, and tells users to keep teams small because usage grows
with team size
([source](https://code.claude.com/docs/en/costs)). Teamwright therefore
proposes small teams, runs nobody who has nothing to do, and keeps
coordination out of the models.

**The team layer has to live in the app.** Claude Code's agent teams do not
run in headless mode
([source](https://code.claude.com/docs/en/agent-teams)), which is the mode an
app must use. The task list, the mailbox and the routing are Teamwright's own.

**Budgets must be counted in tokens.** On a subscription there is no bill per
request, so tools that cap spending in dollars can record zero and never stop
anything ([an example](https://github.com/paperclipai/paperclip/issues/339)).
Teamwright budgets tokens and shows plan limits where Claude Code reports
them.

**Tracking for humans is still a gap.** Several of the strongest orchestrators
are terminal tools with trackers designed for agents to read. Teamwright's
main interface is the set of views a human manager already knows.

**Staying small is a survival strategy.** More than one earlier project has
been sunset or frozen, in one case because a hosted service could not pay for
itself. Teamwright has no server to operate: it is a local program and a
folder you own.

**Art needs a clear license.** Some office views use artwork that cannot be
reused commercially. Teamwright's artwork is original and ships under the
project's own license.

**Names and sign-in have rules.** A product built on Claude Code may not use
"Claude" in its name, may not offer its own Claude sign-in, and must leave the
Claude Code program unmodified
([source](https://code.claude.com/docs/en/legal-and-compliance)). Teamwright
starts the copy of Claude Code that you installed and signed in to, and never
reads its credentials.

## Limits of this survey

- Searches for "pixel office agents" and "claude code kanban" return hundreds
  of repositories. Only the most visible ones were opened.
- Capability marks come from documentation, not from running each tool.
- The field moves weekly. The date at the top of this page matters.
