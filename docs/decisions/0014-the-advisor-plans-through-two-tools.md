# 0014. Let the Advisor interview and propose through two tools

- Status: accepted
- Date: 2026-11-12

## Context

The product begins with the Advisor ([vision](../product/vision.md)): the
manager says what they want in their own words, the Advisor asks what it
needs to know, and then proposes the smallest team that can do the job and
a plan of tasks. Nothing is created before the manager approves.

[Decision 0006](0006-team-layer-in-the-app.md) already says that the
Advisor returns structure through tools, so that the program can show a
proposal as a form instead of reading it out of prose.
[Decision 0012](0012-a-specialist-asks-through-a-tool.md) built the way for
it: a tool server that a session is given for one turn, whose calls are
read from the stream of events.

Before anything was built, the idea was tried against Claude Code 2.1.322
with a stand-in tool server and a session on a middle model. Its brief was
five sentences about a small device that measures the air.

- In its first turn the session called a tool `interview` with five
  questions. Each had a reason and two or three options with what they lead
  to, and one recommended. Then it ended its turn.
- Given the answers, it called a tool `propose_plan` with a summary of the
  project, a team of three with a reason, a model and an effort for each,
  and sixteen tasks with owners, sizes and the tasks they wait for. The
  work that needs hands was given to the manager. The call passed every
  check at the first attempt.
- The two turns took 29 seconds and wrote about 3,400 tokens.

## Decision

- **The Advisor is a session of its own kind, not a member of the team.** A
  team is who tasks can be given to
  ([decision 0009](0009-a-team-per-project.md)), and nothing is given to the
  Advisor. A conversation is either with a specialist or with the Advisor.
- **It can read and nothing else.** It runs in the work folder of the
  project with the tools to read and search, and it may read the reference
  folders ([decision 0013](0013-the-folders-of-a-project.md)). Material for
  a plan is given to it the way it is given to specialists. It has no tool
  to write, so planning never changes a file.
- **It has two tools on the team bus.**
  - `interview`: one to five questions. Each has the question, why it is
    asked, and either two to four options with what each leads to and one
    recommended, or no options, which makes it a question that is answered
    in words.
  - `propose_plan`: a summary of the project for the manager to check, the
    members to add, each with a responsibility, a reason, a model and an
    effort, and the tasks, each with a description, an owner, a size, a
    priority and the tasks it waits for. An owner is named: a member, or
    the manager.
- **A turn carries at most one of them.** The server refuses a second
  call, as it does for `ask_manager`.
- **Both are recorded from the stream of events** and checked by the same
  functions the server uses.
- **Neither blocks.** After an interview the Advisor ends its turn. The
  answers are the request of the next turn. While an interview is open,
  answering it is the only way to go on.
- **A proposal changes nothing until the manager approves it.** The manager
  can take a member or a task out of it, change who owns a task, and change
  the model and the effort of a member. For anything larger the manager
  says what to change, and the Advisor proposes again. Only the proposal of
  the latest turn can be approved.
- **Approving creates the members and the tasks in one step**, or nothing.
  An owner that names a member the project already has means that member.
  Tasks go into the first status of the workflow. No work starts by itself.
- **A later round adds.** The Advisor is told who is on the team and which
  tasks exist, and proposes what is to be added. Approving never changes or
  removes what is there.
- **The model and the effort of the Advisor are chosen when a conversation
  with it starts**, and hold for that conversation. Planning is where
  judgment matters most, so the default is the default of the program.
- **What waits for the manager** now includes an open interview and a
  proposal that is not approved. The inbox lists them and leads to the
  Advisor.

## Consequences

- The interview is a form with options that can be picked, and the
  proposal is a list of people and a list of tasks that can be edited. The
  interface never reads prose to find them.
- The team bus is no longer one fixed tool. Which tools a session gets is
  part of the request of a turn, and the server is started with their
  names.
- A plan is only as good as the model that makes it. The checks say
  whether a proposal is well formed: names that exist, no task that waits
  for itself. They cannot say whether it is wise. The manager reads it.
- The size of a task is a judgment of the Advisor, in hours of a skilled
  person's work. It is a measure of size, shown as an estimate, and says
  little about how long a specialist takes.
- The Advisor does not see what earlier conversations with specialists
  found. Bringing those in as digests is a separate piece of work.
- A proposal cannot be approved in parts. Whoever wants half of it takes
  the other half out first.

## Alternatives considered

- **Make the Advisor a member of the team.** It would get a profile, an
  avatar and a place in the Team view for free. It would also appear
  wherever a task is given to somebody, and every such place would have to
  leave it out.
- **Ask one question in each turn, with `ask_manager`.** It needs no new
  tool. An interview of five questions would be five turns, each starting
  the program again, and the manager would answer without seeing what else
  is asked.
- **One tool for the plan and one for the team.** The owners of the tasks
  are members of the proposed team, so the two would have to be checked
  against each other across two calls, and one could arrive without the
  other.
- **Let the Advisor create members and tasks itself, through tools.** The
  principle that nothing starts before the manager approves would then
  depend on the Advisor asking first.
