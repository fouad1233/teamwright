# 0031. Give a specialist instructions of its own, and ship a set of ready-made specialists

- Status: accepted
- Date: 2026-11-21

## Context

A specialist is told who it is by the instructions Teamwright appends to
the system prompt of every turn: its name, the project, what it is on the
team for, and the fixed rules of how the team works (the folder, no
commands, ask the manager with `ask_manager`). Its model and effort can be
chosen. What it cares about, how it checks its work and what it must never
do could not be told, except by repeating it in every task.

[Decision 0009](0009-a-team-per-project.md) foresaw a place for "what
belongs to one specialist on one project: its instructions". The vision
speaks of projects with electronics, mechanics and firmware as well as
software, and a manager who starts a team should not have to write
instructions for a firmware engineer from nothing.

The Advisor proposes a team ([decision 0014](0014-the-advisor-plans-through-two-tools.md)).
A member it proposes should start as well instructed as one the manager
picks by hand.

## Decision

- **Instructions are a field of a member.** A value of the domain,
  `MemberInstructions`: text in the manager's words, with its lines kept,
  the space at both ends removed, at most **4,000 characters** (about a
  page, or a thousand tokens on every turn). Empty means none. Only an AI
  specialist has instructions; a person has none, as a person has no model.
  They are stored in the column `instructions` of `members` (migration
  0020), logged with the member, and merge as a field of their own, so that
  instructions changed on one computer and a rename on the other both
  survive.
- **They reach every turn in the system prompt, after Teamwright's own,
  never instead.** The application appends them at the end of the
  instructions of a specialist's turn, between `<own-instructions>` and
  `</own-instructions>`, under a line that says they add to the rules above
  and never replace them, and followed by a line that repeats the two rules
  that matter most: write only in the folder of the project, and ask the
  manager with `ask_manager`. They are never part of the prompt of the task.
  The rules about folders do not rest on words alone: what a specialist may
  touch is its permission profile
  ([decision 0011](0011-what-a-specialist-may-do.md)), which no instruction
  can change.
- **Twelve ready-made specialists ship in code**, in `crates/domain`
  (`ReadyMadeSpecialist::ALL`), because they are knowledge of the product
  with no input or output: Systems engineer, Electronics designer, Firmware
  engineer, Mechanical designer, Embedded test engineer, Front-end
  developer, Back-end developer, Interface designer, Code reviewer,
  Technical writer, Researcher and Data analyst. Each has a stable key, a
  name, a description for the manager, a model and an effort, and
  instructions of two or three paragraphs: what good work is in the role,
  what to check before reporting, and when to ask the manager. Every one of
  them is written for a specialist as Teamwright runs one: it says what the
  specialist cannot do (compile, measure, fetch from the internet) and what
  to do instead, so that no instruction sets it up to pretend.
- **Adding one is a single step that makes an ordinary member.** The team
  service creates an AI specialist with the name, the description as its
  responsibility, the instructions, the model and the effort of the role.
  If the team has a member of that name, the newcomer is numbered
  ("Firmware engineer 2"). Nothing ties the member to the set afterwards:
  everything can be changed, the instructions too. A member added by hand
  starts without instructions, as before.
- **The Advisor names ready-made specialists; the application matches by
  exact name.** The Advisor's instructions list the ready-made specialists
  by name and description and ask it to give a member exactly that name
  when one fits. When a proposal is approved, a member whose name is a
  ready-made name, capital and small letters counting for nothing, starts
  with the instructions of that role. The model, the effort and the
  responsibility stay those the Advisor proposed and the manager approved.
  No tool, no stored proposal and nothing on the wire changes.
- **The interface reads the set through the Backend.** The demo offers the
  same set from `apps/desktop/src/shared/api/readyMade.json`, a copy of what
  the program knows (the set and the limit of length), which a test of the
  desktop shell compares with the domain; `TEAMWRIGHT_WRITE_READY_MADE=1`
  writes it anew. The interface counts instructions against the same
  limit from that copy.

## Consequences

- A manager can tell a specialist once how to work, and it holds for every
  task and every question, on every computer.
- Every turn of a specialist with instructions costs their length in input
  tokens. The limit keeps that to about a thousand.
- A manager can write instructions that contradict Teamwright's rules. The
  model is told that Teamwright's rules win, and the permissions decide what
  it can touch whatever it is told; a contradicting instruction can still
  make a specialist less useful, which the manager will see in its work.
- The Advisor's choice of names now decides whether a proposed member gets
  instructions. A name that differs by a word ("Senior firmware engineer")
  gets none; the manager can add a ready-made specialist instead, or copy
  the instructions in.
- The ready-made instructions are part of the program: improving them
  changes what new members start with, never the members that exist.
- A new or changed ready-made specialist needs the copy for the interface
  written anew, and a character that fits its name in the office
  ([decision 0019](0019-people-in-the-office.md)); a test of the office
  checks that every ready-made name finds its character.

## Alternatives considered

- **Instructions as part of the profile, next to the model and the
  effort.** The profile is also what the Advisor's conversation runs on,
  and it has no instructions; a separate field kept both simple.
- **Put the instructions in the prompt of each task.** They would mix with
  the manager's words about the task, be repeated in the history of every
  conversation, and could be read as part of the task.
- **Put them before Teamwright's own instructions.** The rules about the
  folder and about asking the manager would then be the last word only if
  the model weighs what comes last; placed after, and framed, the manager's
  words add and Teamwright's rules close.
- **Let the Advisor name a ready-made specialist by its key in
  `propose_plan`.** It would need a new field in the tool, in the stored
  proposal, on the wire and in the interface of the proposal, while another
  piece of work changes the editing of proposals. Matching by name needs
  none of that, and a name is what the Advisor already gives.
- **Match a proposed member to a role by the words of its name**, as the
  office does for characters. "Hardware engineer" would then get the
  instructions of an Electronics designer, which the Advisor did not
  choose. Instructions are worth more than a picture; they come only when
  the name says so exactly.
- **Keep the set in a file the program reads at start.** It is knowledge of
  the product that changes with the program, not with the computer, and in
  code it is checked by the compiler and by tests.
