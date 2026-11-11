# Chapter 1 · Day one: checking the ground

*11 November 2026*

## What happened

The first working session wrote no product code. It checked whether the idea
was new, whether it was allowed, whether it was possible on the machine at
hand, and what to call it.

### The machine said no, twice

The founder's development PC had Git and Node.js, but no Rust, no GitHub
command-line tool and no Claude Code on the command line. It also had no
administrator rights. Visual Studio was installed, but without its C++ tools,
and adding them needs an administrator.

The way through was to stay inside the user's own folder: Rust's GNU
toolchain installed by `rustup`, a portable MinGW-w64 compiler, and the GitHub
command-line tool unpacked from its official archive. Every download was
checked against its published checksum. Nothing asked for elevated rights.

### Can an app drive Claude Code at all?

Yes. Claude Code has a headless mode that reads prompts and writes events as
lines of JSON, and one process can take turn after turn. That is the seam
Teamwright is built on
([decision 0005](../decisions/0005-drive-the-users-claude-code.md)).

Three findings shaped the design:

- Claude Code's own team feature does not run in headless mode. Teamwright has
  to provide the task list, the mailbox and the coordination itself
  ([decision 0006](../decisions/0006-team-layer-in-the-app.md)).
- An app may start the user's Claude Code, but it may not handle the user's
  sign-in. Teamwright therefore never touches credentials.
- The product's name may not contain "Claude".

A first real run on the machine printed exactly the events the documentation
promised, and one line nobody had planned for: *Not logged in*. The copy of
Claude Code that lives inside the desktop app is not signed in when something
else starts it. The first-run experience has to handle that, so it went on
the roadmap.

### Is it new?

The founder believed that nobody had built this in open source. A survey of
the field on the same day said otherwise. Paperclip, with more than a hundred
thousand stars, organizes agents as a company with an org chart, budgets and
approvals. Multica puts agents on a board as teammates. Claw-Empire and Pixel
Agents draw them as characters in an office. Almost every single idea in the
founder's message already existed somewhere.

What did not turn up was the combination: an advisor that interviews the
manager and proposes a right-sized team, real project-management views with a
timeline and sprints, engineering work beyond software, structured decision
requests, and sync through storage the user owns.

So the pitch changed on day one, from "the first" to "this combination, done
carefully". The survey is published in full in the
[landscape](../product/landscape.md), together with the lessons other projects
paid for: token use grows with team size, budgets in dollars do not work on a
subscription, and projects that need a server to exist tend to disappear.

### A name

The founder had found the obvious names taken. A long list was cut down by
checking the package registries, GitHub and the web. Some candidates fell
because they were in use, one because a project with the same name had
appeared ten days earlier, and one because dictation software would always
spell it as the name of a famous cartoon town.

The founder picked **Teamwright**. A wright is a builder, and said aloud the
name is also "team right"
([decision 0002](../decisions/0002-name-teamwright.md)).

### The first commits

The repository began with a license, a notice that the project is independent
of Anthropic, and documents: the vision, the landscape, the roadmap, eight
decision records and this story. The founder's reference screenshots stay out
of the repository, because they show another company's product and private
work.

## Why it went this way

The founder's instruction was to save the story and to commit every small
change. Taken literally, that means writing the reasons down before writing
the code, while they can still be checked.

Finding out on the first day that the idea was not new was the most useful
result of the session. It replaced a claim that would not have survived its
first informed reader with a position that can be defended.

## Decisions

- [0001](../decisions/0001-record-decisions.md) Record significant decisions
- [0002](../decisions/0002-name-teamwright.md) Name the project Teamwright
- [0003](../decisions/0003-apache-2-license.md) License under Apache-2.0
- [0004](../decisions/0004-desktop-stack.md) Build with Tauri, Rust and React
- [0005](../decisions/0005-drive-the-users-claude-code.md) Drive the user's
  own Claude Code
- [0006](../decisions/0006-team-layer-in-the-app.md) Keep the team layer in
  the app
- [0007](../decisions/0007-local-first-storage.md) Store locally, sync through
  a user-owned folder
- [0008](../decisions/0008-budgets-in-tokens.md) Measure budgets in tokens

## Storyboard beats

1. A locked door marked *Administrator*. The builder looks at it, shrugs, and
   walks around the building to a small side entrance marked *User folder*.
2. A terminal window. Three lines of JSON scroll past, then one in red: *Not
   logged in*. A sticky note goes on the monitor: "first-run sign-in".
3. A map of an island that was supposed to be empty. As the fog lifts it turns
   out to be full of settlements: a company tower, a kanban harbor, a pixel
   village. One valley between them is still empty. A flag goes in there.
4. A wall of name tags. A hand crosses them out one by one until a single tag
   is left: *Teamwright*.
5. A shelf with eight thin folders numbered 0001 to 0008, and a first commit
   stamped onto a timeline.
