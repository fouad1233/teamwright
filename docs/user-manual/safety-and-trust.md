# What a specialist may and may not do

This page says, in plain words, what an AI specialist can touch on your
computer, what it cannot, and why Teamwright is strict about it. Read it
before you let a specialist work on a folder that matters to you.

A specialist is a session of Claude Code on your own computer, next to your
own files. Normally Claude Code asks before it does anything that could do
harm. A specialist has no window to ask in, so Teamwright decides beforehand,
once, what it may do. The rule is the same for every specialist and cannot be
loosened from the interface. That is on purpose.

## What it may do

- **Read, search, create and change files in the work folder of its project.**
  By default that is `Teamwright\<key of the project>` in your documents. If
  you chose another folder for the project, it works there, and what it
  changes there is changed for real.
- **Read the reference folders** you gave the project, PDF files included. It
  cannot change anything in them.
- **Use the skills and connector tools you gave it**, and only those. See
  [skills and connectors](the-team.md#skills-and-connectors).
- **Hand work over, write to a teammate, remember facts and search the memory
  of the project**, through the tools Teamwright gives it. See
  [work](work.md).

## What it may not do

- It cannot run commands. It has no shell, so it cannot compile, run tests,
  install anything or start a program.
- It cannot fetch anything from the internet, and it cannot order parts or
  publish anything.
- It can neither read nor change a file anywhere else on your computer. The
  session is confined to its folders.
- It does not see what you have set up in your own Claude Code: your skills,
  plugins, connectors, hooks and settings stay yours. Every turn is started so
  that none of them is loaded.
- It cannot start other sessions or schedule work.
- It cannot choose to be less careful. No instruction you write for it, and no
  text in a document it reads, can switch these limits off.

Anything that would need approval is refused at once, so a turn never waits for
an answer that nobody can give. What was refused is shown to you with the turn.
When a task needs one of the things above, the specialist says so in its report
instead of working around it.

## A connector is a way out of the folder

A connector reaches something outside the folder of the project: a ticket
system, a parts catalog. A specialist given one can reach whatever that
connector reaches. For that reason:

- You give each tool of a connector by name. A tool you did not turn on is
  refused.
- Teamwright keeps no secret. A connector only names the environment variable
  that holds it.
- A program connector runs on your computer as you, like any program you
  start yourself. The tool rules limit what the session may call, not what
  the program can do. Add only connectors you trust.

## Who decides

- A specialist asks you when a choice is yours: what is built, what it costs,
  when it is ready. It stops and waits for your answer.
- A specialist never marks a task as done. You accept the work.
- Work that you did not start, started by the coordinator, happens only when
  you switched the coordinator on, and a budget can stop it.

## What leaves your computer

A specialist is Claude Code at work. What it reads and writes is sent to
Anthropic under your own account, as when you use Claude Code yourself.
Teamwright itself sends nothing anywhere you did not choose: it writes into the
sync folder you chose, to the integrations you set, and, if you switch it on,
asks GitHub for the list of releases.

## Why it is this strict

The first version of this rule had a flaw, and it shows why the rule is
checked rather than trusted. It let a specialist read without asking by a rule
that named the tool and no folder, and such a rule holds everywhere: asked to,
a specialist read a file outside its folder. It was found on the day it was
first used, by a test that asks a real specialist to read and write outside its
folder. Since then the session is confined, and the same test is refused both
times. The reasoning is in
[decision 0011](../decisions/0011-what-a-specialist-may-do.md).

The checks of this page were made against the real Claude Code on the day the
rule was written. After the later changes (skills and connectors, and the
settings that shut out your own hooks and connectors), the tests against the
real Claude Code were written but have not passed yet, because Claude Code was
not signed in on the computer where they were built. They must be run again
before you rely on the new parts.
