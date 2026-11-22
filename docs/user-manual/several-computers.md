# Several computers

This page explains how to keep your projects the same on more than one
computer, for example a desk at home and a laptop.

Teamwright keeps the computers in step through a folder that you own and that
every one of them can reach, such as a folder that a cloud drive keeps in
step. There is no account and no server: each computer writes what changed
into that folder, and takes in what the others wrote.

Honest status: this was run with two copies of the installed program on one
computer through one folder, and the window showed what the other copy did
without being loaded again. Two real computers with a real cloud drive between
them were never tried. The repairs described under "What you are told" were
seen in tests and, for task numbers, with two copies. Treat it as new.

## Starting

1. Open *Computers* at the foot of the sidebar.
2. Press *Choose a folder*. The folder window of your system opens.
3. Teamwright asks for the name of this computer, with the name the system
   gives it filled in. The name is written into the folder, where your other
   computers show it. The system's name often holds your own name, so you may
   want one that only says which computer it is, such as "Desk".
4. Press *Keep in step*.
5. Do the same on your other computer, with the same folder.

## With Google Drive

If Google Drive for desktop is on the computer, *Sync with Google Drive* on
the page *Computers* does the first steps in one press. Teamwright finds the
folder that Google Drive for desktop keeps on this computer, makes a folder
named Teamwright inside it, and chooses it. The page names the folder it
found, as in "Google Drive for desktop keeps G:\My Drive on this computer".

There is no sign-in to Google and Teamwright keeps no token. It only uses the
folder. On Linux, where there is no Drive for desktop, nothing is looked for.
Whatever Google Drive does with the folder is between you and Google. Never
tried with the real program of Google: it was run against a pretended Drive
folder.

## A new computer

When another computer first writes into the folder, it appears under *Other
computers* with *Take in* and *Ignore*, held by the gold corners of something
that waits for you, and the line at the foot of the sidebar says that it
waits. *Take in* brings in its projects and its work, and from then on
everything it changes. *Ignore* takes in nothing of it; you can change your
mind with *Take in after all*. This is so that two libraries of projects are
never mixed up by mistake.

## How syncing stands

The line at the foot of the sidebar says it in words: "In step" and since when,
"Writing to the folder", "The folder cannot be reached", or "Stopped" with the
reason. It turns gold only when something is wrong or waits for you. Press it
to open *Computers*.

On *Computers* you see the name of this computer, which you can change by
clicking it; the folder, with *Stop syncing*; when the last round was, with
*Sync now* to run one at once instead of waiting (it runs by itself every 20
seconds); and the other computers, with their version of Teamwright and when
they were last heard from. A quiet line under a computer says when its clock
is more than five minutes ahead of this one, or when it runs a newer
Teamwright whose files this one cannot read yet. Stopping removes nothing from
the folder; choosing it again goes on where it stopped.

## What you are told

Now and then two computers do something at the same time that cannot both
stand: both give a new task the same number, for example. Teamwright repairs
it by itself and says so under *Notices*, in a sentence such as "SSN-21 is now
SSN-22: another computer gave the number 21 to another task at the same
time." Each notice leads to its task or project and can be dismissed.

- When two computers change *different* fields of the same thing, both
  changes stay. Tags, checklist steps and custom field values are kept one by
  one.
- When they change the *same* field, the later change wins, and nothing is
  announced.
- A deletion is final. The computer whose change was dropped is told in a
  notice.

## Conversations live where they started

A specialist works as Claude Code on the computer where its conversation was
started, in a folder of that computer. On your other computers you can read the
conversation as it goes on, but not continue it. Where the box to reply would
be, it says which computer it runs on and when it last had news. A question a
specialist asked there shows its options, says that it is answered on that
computer, and also appears in your inbox. The same holds for the Advisor. You
can start new work on any computer. The coordinator acts on one computer only
(see [the coordinator](coordinator-and-budgets.md#the-coordinator)).

Files that a handoff names are on the computer where they were made. The paths
match on another computer only if the folders are kept in step by other means.

This was seen on demo data only, because it needs a real specialist on a
second computer.

## A copied computer

If you copied Teamwright's data folder from one computer to another, both
write into the sync folder as the same computer. Teamwright notices, stops
syncing, and explains it on *Computers*. Press *Give this computer a new
identity* on the computer the data folder was copied to, not on the one where
your conversations with the specialists were started. Then start Teamwright
again. Nothing is lost.

## What leaves your computer

Everything of your projects goes into the folder: projects, tasks, teams,
sprints, the memory, and the full text of the conversations with the
specialists, which can quote files they read. Also the name of each computer
and its version of Teamwright. The folders of a project and the files in them
do not. Nothing in the folder is encrypted, so choose a folder only you can
read. What stays on this computer: integration addresses, the theme, the
choices about notifications and about looking for a newer version, the
copies of skills, and which connectors and tools you confirmed here.

Every computer should run the same version of Teamwright. An older version
passes over what it does not know.

## Connectors from another computer

The library of skills and connectors travels with the project, but each
computer decides for itself what it starts. A connector added or changed on
another computer, and a tool given to a specialist there, wait on this
computer until you confirm them here (see
[skills and connectors](the-team.md#skills-and-connectors)). What you
confirmed stays on this computer and never goes into the folder, so someone
who can write the folder cannot confirm anything for you. A skill was
already of one computer: its folder must be added on each.
