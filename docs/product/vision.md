# Product vision

## In one sentence

Teamwright lets one person run a multi-discipline project as the manager of a
small team of AI specialists, with the planning, visibility and control that
project-management tools give human teams.

## Who it is for

**The solo builder with a project bigger than one conversation.** An engineer,
maker or founder who knows what the result should be and can judge whether it
is right, but cannot be the hardware designer, the firmware developer, the
mechanical designer and the project manager at the same time.

The reference scenario throughout the design is a small connected device: a
custom board around a low-cost Wi-Fi microcontroller, a display, several
sensors, firmware, and an enclosure. It was chosen because one agent cannot
finish it in one sitting. It has disciplines, ordering constraints, and
choices that only the owner can make.

Software-only projects are a subset of this and work the same way.

## The roles

| Role | Who | Responsibility |
|---|---|---|
| **Manager** | You | Sets the goal, approves the team and the plan, makes the decisions that are yours, accepts the results |
| **Advisor** | An AI agent | Your chief of staff. Interviews you, proposes staffing and a plan, replans when reality changes, summarizes |
| **Specialists** | AI agents | Each does the work of one discipline, reports progress, asks when blocked, and hands results over |

The **coordinator** is neither a role nor an AI. It is program logic inside
Teamwright that starts tasks when their dependencies are done, delivers
messages, and enforces budgets.

## Principles

1. **The manager decides.** Nothing starts before a plan is approved. A choice
   that changes scope, cost or direction goes to the manager as a decision
   request, never silently into the work.
2. **The smallest team that can do the job.** Every team member costs tokens
   and adds coordination. The Advisor proposes few specialists and gives a
   reason for each. Three well-briefed specialists beat ten vague ones.
3. **Coordination in code, judgment in models.** Anything a program can decide
   (what is unblocked, who is idle, which message goes where) is decided by a
   program and costs nothing. Models are called when judgment is needed.
4. **Brief, do not broadcast.** A teammate receives a task brief and pointers,
   and pulls the details it needs. Results travel as handoffs: a short
   summary, an interface contract, and files. Transcripts are never passed
   around.
5. **Ask once, remember forever.** Every answered decision becomes project
   memory that all teammates can look up.
6. **Idle costs nothing.** A specialist with nothing to do is not running.
   Nothing polls.
7. **Honest numbers.** Usage is shown as measured tokens and reported plan
   limits. Estimates are labeled as estimates.
8. **Local first, user owned.** The data is a file on your disk. Sync uses
   storage you already own. There is no account, no server and no telemetry.
9. **Stand on Claude Code, do not replace it.** Skills, plugins, MCP
   connectors, subagent definitions and permissions belong to Claude Code.
   Teamwright adds the team around them.

## What you can do with it

*These are planned capabilities. The [roadmap](roadmap.md) says which exist.*

**Plan**

- Describe a project to the Advisor and be interviewed about it.
- Attach files and earlier Claude Code conversations as context. The Advisor
  works from a digest, not from the raw transcript.
- Review, edit and approve a staffing proposal and a plan.
- Define your own specialist profiles: instructions, model, effort level,
  skills, tools and connectors.

**Run**

- See tasks on a board, in a list, and on a timeline with dependencies.
- Work in Scrum (backlog, sprints, points, burndown), in Kanban (flow and
  limits), or in a workflow of statuses that you define.
- Let tasks start automatically when they are unblocked, or start them one by
  one.
- Create a task yourself and assign it to a specialist, as you would to a
  person. Specialists can also propose tasks to you.

**Stay in the loop**

- Answer decision requests from an inbox: the question, why it matters, the
  options with their consequences, and a recommendation.
- Approve or deny risky actions before they happen.
- Talk to any teammate, or mention one in a task comment.
- Watch the office: who is working on what, the expected time left, the
  tokens used, and the remaining plan limits.

**Finish**

- Accept completed work or send it back.
- Get a project report: deliverables, decisions made, and usage.
- Keep every artifact (schematics, firmware, CAD, documents) as ordinary
  files in the project folder.

**Across computers**

- Sync through a folder you own, for example Google Drive.
- See which computer did which work.

## What it is not

- **Not an agent swarm.** There is no setting for "more agents". Parallelism
  comes from the plan's dependency structure and a concurrency limit.
- **Not a hosted service.** There is no Teamwright cloud and no account.
- **Not a model, and not a wrapper around a model API.** At this stage it runs
  your Claude Code and nothing else.
- **Not autonomous.** It is built for a manager who wants to stay involved. A
  project that should run unattended for days is a different product.
- **Not a ClickUp or Jira clone for human teams.** It borrows their proven
  ideas for a team of agents with one human manager.

## How we will know it works

The reference scenario runs end to end on an ordinary subscription: a manager
describes the connected device, approves a team of three and a plan, answers a
handful of decision requests, and receives a schematic with a bill of
materials, firmware that builds, and an enclosure model, together with a
record of every decision and an honest account of what it cost.
