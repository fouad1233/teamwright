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

## Note of 21 November 2026: the manager changes a proposal before approving

- **Every change is stored as it is made.** Before, the manager's changes
  lived only in the window until the proposal was approved, and were lost
  when the program closed. Now each change (`change_proposal`) is checked
  by the domain and stored with the turn, logged as the whole proposal like
  the rest of it. So a change survives a restart and reaches the other
  computers the way the proposal does. As with approving, only the
  computer the conversation runs on changes it, because the proposal
  merges whole and two computers changing it at once would lose one side.
- **What can be changed:** the title, the description, the size, the
  priority and the owner of a task; the tasks of the same proposal it
  waits for (one at a time, added or taken out); a task of the manager's
  own, added at the end; a task or a member taken out; the model and the
  effort of a member. Taking a task out takes out what waited for it.
  Taking a member out leaves its tasks to nobody, and a proposed member
  the team already has cannot be taken out, because approving does not
  add it anyway.
- **The rule of circles is written once.** `DependencyGraph` works on the
  identifiers of tasks of a project and on the places of the tasks of a
  proposal. The proposal, the team bus and the board use the same one.
- **The proposal as the Advisor first made it is not kept.** "Undo my
  changes" went with the draft in the window. Each change can be made
  back by hand, except a removed task or member, which the manager adds
  again by hand or asks the Advisor for. Keeping the first form would
  have needed a second copy of the proposal in storage, for little use.
- **The Advisor is told what was approved, as it was approved.** Its
  session remembers its proposal as it made it. When the manager writes
  after approving, the message now lists the members and the tasks that
  were approved, with their owners, sizes and what they wait for, and
  says that what it proposed and does not find there was left out on
  purpose.

## Note of 27 November 2026: a proposed task may be split into parts

- **What was missing.** The founder wrote that the Advisor "is not able to
  create subtask". A proposal could say who does a task and what it waits
  for, not that one task is a part of another, so a plan arrived flat.
- **A part names its task.** A task of `propose_plan` may say `part_of`:
  the place of another task of the same list. The rule is the one of the
  subtasks of the board: a part may have parts, at any depth, and no task
  is a part of itself, directly or further down. The coordinator starts a
  task with parts only once its parts are finished, so a part may not wait
  for its own task, directly or through others; that is refused as a
  circle. The domain checks it for what is proposed, for each change and
  for what is approved, and the team bus before the call is taken, with
  the reason in words the model reads.
- **When to split.** The tool and the instructions say: a task too large
  for one piece of work is split into parts; most tasks have none.
- **The manager changes parts as tasks.** A part is changed like any task;
  it can be moved to another task, made a task of its own, and added.
  **Taking out a task with parts keeps the parts**, where the task was:
  their work is still to be done, as the tasks of a member taken out stay.
  Deleting a task on the board deletes its subtasks; a proposal has no
  undo, so the gentler rule was chosen here. Each part has its own cross.
- **The review shows parts under their task**, set in, unfolded at first:
  the manager approves what is in the plan, so nothing starts hidden.
- **Approving creates real subtasks** under the tasks created for their
  wholes, with their owners, sizes and dependencies, in the same write. The
  Advisor is told after approval which task is a part of which.
- **Only parts of the same proposal.** A proposal names tasks by their
  places in it and cannot refer to a task that exists, so a later round
  cannot add parts to a task on the board. The instructions say so. Letting
  it would need a second kind of reference in the proposal; it was left.
- **Storage** keeps the place of the whole in a column of its own
  (migration 31). The proposal is still logged whole; a state logged
  before has no `part_of` and reads as a plan without parts.
- A real Advisor on the cheapest model, asked to split one task in two,
  called `propose_plan` with two parts at the first attempt.
