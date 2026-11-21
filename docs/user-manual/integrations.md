# Integrations

This page explains how to connect a project to GitHub, Slack and web
addresses.

Choose *Integrations…* in the menu of a project. What you set there stays on
this computer: addresses are secrets and are never synced to your other
computers. Events are sent only while Teamwright runs on this computer. The
dialog also has a part for Google Drive, which only points you to the page
*Computers* (see [several computers](several-computers.md#with-google-drive)).

## GitHub

The tasks of the project know the pull requests and issues that name their
key, such as `SSN-4`, in their title or branch (a pull request also in its
description).

- Teamwright asks GitHub's own command line program, `gh`, signed in as you,
  and keeps no token. If `gh` is missing or not signed in, the dialog says so
  and what to do.
- The repository is the one your work folder is a clone of, or the one you
  name as `owner/name`. Press *Save* to keep it.
- Each task then has a part *GitHub* with its pull requests and issues. A line
  opens it in your browser.
- Nothing is read in the background. *Read now* in the dialog, and *Refresh*
  in a task, read again.
- *Open an issue for this task* shows the title, the text and the repository
  before anything is sent.

The pull requests of a real repository were read with the real `gh`. No real
issue was opened.

## Slack

Slack gives a channel an address when you add an incoming webhook to it.

1. In the dialog, press *Set the channel*.
2. Paste the address, and choose which events are told there. Each event is
   one plain line.
3. *Send a test message* tries it.

If a message cannot be sent, the dialog says why, and the work goes on. Sending
was tried only against a listener on the same computer. No real Slack channel
has received anything.

## Web addresses

Up to five addresses (https) hear the events you choose, as JSON. Zapier
(Catch Hook), Make and n8n listen this way, and so can your own tools. A
secret signs what is sent, so the listener can tell it comes from you. The last
ten deliveries of each address are shown with the answer. Press *Add the
address* to add one. The shape of what is sent is described in
[webhooks](../integrations/webhooks.md).

No real listener on the internet has received anything.
