# Screenshots

Pictures of the real application on a sample project. The
[README](../../README.md) and the [user manual](../user-manual/README.md)
show them. Every picture exists twice: `-dark.png` for the dark theme and
`-light.png` for the light theme.

| Picture | What it shows |
|---|---|
| `first-run` | The first screen of a new installation |
| `board` | The board of the sample project |
| `task` | A task opened from the board |
| `list` | The list of the sample project |
| `gantt` | The Gantt chart of the sample project |
| `calendar` | The calendar of the sample project |
| `team` | The team of the sample project |
| `statuses` | The dialog in which the statuses of a project are edited |
| `specialist-working` | A real specialist at work on a task |
| `specialist-report` | The same specialist's report |
| `specialist-inbox` | A question of the same specialist, in the inbox |
| `advisor-interview` | The questions of a real Advisor |
| `advisor-proposal` | Its proposal: a team and a plan |
| `advisor-board` | The board after the proposal was approved |

Nobody edits these files by hand.
[A tool](../../tools/screenshots/capture-screenshots.mjs) starts the built
application on an empty data folder, creates the sample project through the
same commands the interface uses, and takes the pictures. The Claude Code
in them is a stand-in, like the project is a sample: what the pictures say
about the program and the sign-in is the same on every computer, and is
nobody's account.
[The development guide](../development/README.md#pictures-for-the-documentation)
says how to run it. Take the pictures again after a change that they would
show.

The three pictures of a specialist are taken by another tool,
[the test that lets a real specialist work](../../tools/smoke/specialist-smoke.mjs),
when it is given this folder. What they show was done by a real session of
Claude Code, so the words of the report and of the question differ from one
run to the next. That test uses a part of a Claude plan, so these three are
taken again only when the work of the specialists looks different.

The three pictures of the Advisor are taken the same way by
[the test that lets a real Advisor plan a project](../../tools/smoke/advisor-smoke.mjs).
The questions and the proposal in them were made by a real session, so they
differ from one run to the next.
