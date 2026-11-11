# Chapter 0 · The idea

*11 November 2026*

Teamwright began as one long spoken message from its founder to an AI
assistant. It was dictated in a single run and described a whole product:
what it is, who it is for, how it should feel, and why it should exist. This
chapter puts that message in order. Quotations are the founder's words,
lightly tidied from dictation.

## The picture in the founder's head

An engineer wants to build a small IoT device. It has a screen, it measures
temperature and oxygen and reads other sensors, and it connects to Wi-Fi. To
make it real, someone has to design the electronics, choose and find the
components, write the software, and design the case.

The engineer knows what should go in and what should come out. What the
engineer wants is a team.

So the idea is a project-management tool in the family of ClickUp and Jira,
"but a lot more advanced", in which the employees are AI agents running on
Claude Code. "You are the project manager and you can have a lot of agents
working for you."

## How it should go

**First, a conversation with an assistant.** The manager explains the project
to a helper, "the assistant of the manager", who is also the resource
allocator. The assistant asks many questions: you want this, you do not want
that, are you sure? If the manager says the chip should be cheap and small,
perhaps an ESP32, the assistant checks what that implies and whether it is
acceptable. The manager can also ask it for improvements to the idea.

**Then a proposal.** The assistant says whom it would put on the job: for the
device, three engineers, one for hardware, one for software and one for
mechanical design. It says which model and effort level each should run on,
and asks what context to give them, including earlier conversations from the
manager's Claude app. The team must be efficient: "it should not be a lot of
agents that doesn't make any sense." The manager can also define agents and
their skills in advance, and everything Claude Code offers (skills, plugins,
connectors) must stay within reach.

**Then approval, and the team is set up.** Once the manager approves the plan
and the resources, the assistant prepares the agents and coordinates them.
Work starts at the press of a button or automatically. Some jobs wait for
others, and that order must be visible as a timeline: which work runs from
when to when.

**Then the work, with the manager in the loop.** The hardware designer goes
first, because the software cannot be finished without knowing the pins. When
an agent needs a human answer, the question arrives as a notification or a
chat, and it has to be "very, very, very clear": why the decision matters,
what is proposed, and whether that is acceptable. The agents talk to each
other the way colleagues do. The hardware engineer tells the software
engineer that the manager agreed and that the pins are ready. The mechanical
engineer says the case can start once the board is done, and designs it
through a CAD program connected as an MCP server.

**Then the result.** When everything is finished it comes back to the manager
with all the details. "Like you have a real team and they are helping you."

## What else it has to be

- **A real management tool.** Scrum, Agile or a custom method. The manager can
  open a task for an agent the way one opens a task for a person, and the
  agent takes it and moves it along: from to-do, through testing, to done.
- **Careful with tokens.** Good context management, a memory, and an efficient
  way for agents to communicate.
- **Alive.** A board where the agents are characters, in the spirit of a
  farming-village game: talking to each other, each with a small speech box
  above its head saying what it is doing now and how long it will take. The
  manager watches the team work and sees the tokens used and the limit that
  remains.
- **Free to keep.** Many people cannot pay for a hosted database. The data
  should sync through the user's own Google Drive folder, work from several
  PCs, and know which PC did which work. Later, a PC that is online might be
  controlled from far away.
- **A proper application.** Rust in the backend, React in the frontend, one
  executable that opens like a native app. And a logo.
- **Open source and built in public.** A public repository, a commit for
  every small change, clean code that explains itself, and documentation that
  lets both people and AI agents contribute.
- **Remembered.** The story of what was done and why is saved, to become a
  storyboard and then an animated film.

## Why

The founder gave two reasons. The first is practical: one person should be
able to carry a project that normally takes a team. The second is larger: to
show what Anthropic's tools can do when they are organized well, to "maximize
the efficiency of AI", and to give something useful back as an open-source
project.

## Decisions

None yet. This chapter is the brief that every later decision answers to.

## Storyboard beats

1. A desk at night. One engineer, one small circuit board, three open
   notebooks labeled *hardware*, *software* and *case*. Too much for one pair
   of hands.
2. The engineer speaks. The words turn into sticky notes that fill a wall:
   sensors, Wi-Fi, small, cheap, a screen.
3. A figure with a clipboard steps up to the wall. This is the Advisor. It
   starts asking questions, and with each answer a note is moved, merged or
   thrown away.
4. The Advisor places three empty chairs at a table and writes a role on
   each. The engineer nods. The chairs fill.
5. A timeline unrolls across the floor. Bars light up from left to right. One
   bar waits for another to finish.
6. A hand goes up at one desk, with a question mark in a speech bubble. The
   engineer answers and the bar moves on.
7. Three finished pieces slide together: a board, a program, a case. One
   device.
