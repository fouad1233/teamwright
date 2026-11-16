# Chapter 10: Every view

*15 and 16 November 2026*

## What happened

### Five wishes in one message

On the evening of 15 November the founder signed in to the tracker his
company uses, in the browser beside the session, and wrote five things in
one message. Look at all its views, list them with pictures, and build
every one, "commit and push after each new view". See what integrations it
has, list them, and build the top three. Show how many tokens each agent
spends, and the context window of the agent. Notifications, and a way to
allow them. Excel. The message ended: "Don't stop utill you finish all of
them".

The session looked and touched nothing there. The chooser of views offered
thirteen looks at the tasks (List, Gantt, Calendar, Board, Dashboard,
Table, Timeline, Activity, Workload, Mind Map, Team, Map, Gallery), five
kinds of file (Doc, Whiteboard, Form, Embed, Collection), and a row of
pages an assistant writes on request. The public list of integrations had
about seventy names and began with Slack, GitHub, Zapier, GitLab and
Webhooks. Teamwright had six kinds of view and no integration at all.

### The ground first

Fourteen agents could not all add a line to the same table of kinds. So
before any view was built, one agent laid the ground in the model: the
kinds went from six to twenty, a view learned to keep content of its own
(a page of text, a form, the pins of a board, at most two mebibytes), and
the history of a whole project could be read, not only that of one task.
The session itself changed the interface so that it finds a kind of view
by where it lies: a kind is a folder with one file that says what it is.
After that, a new view touched nothing that another view touched. The
chooser got two headings and two columns, because a menu of twenty lines
is too long.

The same evening another agent wrote the project as an Excel workbook:
four sheets, Summary, Tasks, Sprints and Team, from the menu of the
project. The file was opened in Excel before anybody called it done.

### One view after another

Each view was built by one agent in its own copy of the repository, and
each came back with pictures. The session looked at the pictures, brought
the commit over, ran the checks and pushed, one view at a time.

The Timeline came first: a row for every person, their tasks as bars along
the days. Then the Table, every field a column that is changed in place.
The Workload, the hours on each person week by week. The view the tracker
calls Team became People, because Teamwright already has a place called
Team where the members themselves are kept, and two tabs with one name
would mislead. Then Embed, Gallery, Form, the Mind map, Activity, Doc,
Chat, Report, the Whiteboard and Files. The Mind map brought a ground
that can be moved and zoomed, and the Whiteboard was built on the same
ground. The Report is a sheet of paper that stays paper in the dark theme
and prints alone.

One view was not built. A map needs the outline of the land, and the
session would not download one without asking. It asked. The founder
answered in four words: "no need for map".

When the last view was in, the program was built and started on an empty
data folder. A view of each of the fourteen kinds was added and opened in
the real window, on real storage, and none reported an error. The first
run of that check failed on the Doc view, and the fault was the check's:
it had stored a bare page where the view keeps a page wrapped in a small
record, and the view, as it should, read what it did not know as an empty
page.

### What an agent has spent

The agent program reports, with every message, what the request behind it
read and wrote. The last such report of a session is how full its context
is; the last message of a turn names the size of the window. An agent
read the recorded streams, found both, and carried them to where the
manager looks: a thin meter in the office under every specialist, at the
head of a conversation and in the Team place, and a table on the dashboard
with a row for every member. A meter turns to the warning colour at four
fifths.

### Told, when allowed

Notifications are off until the manager allows them. The program asks
once, in the inbox, the first time something waits there, and never again
after a no. What is worth telling is a rule, and it lives where rules
live: a small module that says in plain data what happened in a project.
A specialist waits for you. Work is ready. Somebody stopped.

The three integrations listen to the same module. GitHub goes through the
manager's own command line program, the way the specialists go through
the manager's own agent program, so Teamwright keeps no token: a pull
request or an issue is about a task when the task's key stands in its
title. A Slack channel gets one plain line for each chosen event. And up
to five web addresses get every chosen event as JSON, signed if the
manager gives a secret, which is how Zapier and Make listen. The
addresses are secrets, so they stay on the computer where they were
typed.

### Three things that went wrong

**The build that lied.** The agents shared one build folder, to save an
hour of compiling each. An agent then saw errors that made no sense: a
name that plainly existed was "not found". Cargo names a crate's files
the same in every copy of the repository and judges by the age of a file,
so it had taken a crate another agent built from other sources for its
own. A check could have passed on stale tests. From then on one agent at
a time compiled, and every check that counted first made its own sources
the newest.

**Fifty browsers.** The computer has sixteen gigabytes and was down to
one. Thirty headless browsers were still alive from picture scripts of
earlier days, and a quarter of an hour later twenty more from that
evening's agents.
Every script ended with a line that stops the browser it started. On
Windows that process hands over to another and ends at once, so the line
stopped nothing and said so: it returned false, and nobody had read it.
The scripts now ask the browser to close itself.

**The limit.** The founder had written that the other views could go to
agents on the strongest model. Six of them worked at once, beside one
more, and used up five hours of allowance in about half an hour. At ten
past midnight all seven stopped in the middle of a sentence. At twenty to
four they went on; an hour later they stopped again. In the morning the
founder wrote: "be token efficient because you are spending alot of
tokens". Three views turned out to be finished on disk, lacking only
their last commit, and the session landed them itself instead of waking
their agents. The rest went to cheaper agents, a few at a time. The four
views built by the cheaper model the evening before had been as good.

## Why it went this way

**A kind of view is a folder.** The work could only be split fourteen
ways if no two agents had to change the same line. Making the interface
find its kinds was an hour of work that saved a merge for every view.

**Only what is true on one desk.** The tracker's views assume many
people, shared links and a server. Each brief said to leave that out: a
form is filled in by the manager and makes a task, nothing is shared with
anybody; a report leaves the program as paper or a PDF; the workload of
an AI specialist has no working day and says so.

**Through the manager's own programs.** Teamwright runs specialists on
the manager's own agent program and keeps no key. GitHub follows the same
rule. What cannot be done that way (a server that Zapier could call into)
is not pretended: the program can only send.

**Off until allowed.** A program that starts to interrupt without being
asked is switched off for good. So it asks once, where the manager is
already looking, and remembers a no.

## Decisions

- [0024: a view keeps content of its own](../decisions/0024-a-view-keeps-content-of-its-own.md)
- [0025: the project as a workbook](../decisions/0025-the-project-as-a-workbook.md)
- [0026: notifications](../decisions/0026-notifications.md)
- [0027: integrations](../decisions/0027-integrations.md)
- A note of 15 November in
  [0021](../decisions/0021-views-the-manager-can-add.md): twenty kinds, a
  kind found by its folder, and the view of the people named People.
- The map was left out on the founder's word.

## Storyboard beats

1. One message with five wishes on the screen, the last line underlined
   by nobody: "Don't stop utill you finish all of them".
2. A menu with six lines grows a second column and a second heading.
3. Fourteen desks in a row, each with one folder on it. Nobody reaches
   across to a neighbour's desk.
4. A memory gauge at the red end. Behind the window, fifty small browsers
   with no window of their own, all idle. One line of code that says
   "stop" and a small "false" beside it.
5. Ten past midnight. Seven lamps go out in the same second. At twenty to
   four they come on again.
6. A question on the screen, about a map. The answer: four words.
7. A thin meter under a figure at a desk fills to four fifths and turns
   amber.
