# 0027. Three integrations, set per computer: GitHub through the manager's own gh, a Slack channel, and web addresses that listen

- Status: accepted
- Date: 2026-11-16

## Context

The owner looked at the integrations of the tracker they use, about fifty,
whose list begins with Slack, GitHub, Zapier, GitLab and Webhooks, and
asked: "implement top 3 like github implementation".

Teamwright runs on the manager's own computer. It has no server, so nothing
on the internet can call into it, and it keeps no account of its own with
any service. The events of a project are already said in plain data by the
module of events ([0026](0026-notifications.md)); notifications listen to
them.

## Decision

1. **These three.** GitHub, because the work of a project often is code
   whose pull requests and issues name the tasks. Slack, because a team
   reads its channel more than any tracker. Web addresses that listen,
   because that is what Zapier ("Catch Hook"), Make, n8n and a company's
   own tools take in: Zapier cannot reach into a program on a desk, but
   it can listen at an address. One way out serves all of them, so the
   third is named for what it is, with one sentence that names Zapier and
   Make.
2. **GitHub through the manager's own `gh`.** Teamwright keeps no token.
   It finds GitHub's command line program (the one the manager named, the
   search path, then the places it is usually installed), asks it whether
   it is signed in, and runs it with a time limit for every read. Text of
   a task is passed as arguments, never through a shell. A pull request or
   an issue is about a task when the task's key stands, whole, in its
   title or its branch (for a pull request also in its body). What is read
   is kept in memory with its age; nothing is read in the background. An
   issue is opened only after the manager saw what will be sent and
   where.
3. **Per computer, not logged.** The repository, the channel and the
   addresses are kept for one project on one computer, in tables of their
   own (version 17), written without the operation recorder, so they never
   reach the log or another computer. An address is a secret: its `Debug`
   form and what the interface gets back show only its host, and no
   error repeats it. Another computer of the same manager may sit behind
   other doors and set its own.
4. **Listeners of the one module of events.** The Slack channel and the
   web addresses listen in the same place the notifications do
   (`events::tell` in the shell). Two events were added for them: a task
   was opened, a task changed its status. Sending happens on a thread of
   its own with a time limit; a failure never disturbs the work, it is
   kept in memory and shown on the integrations page.
5. **One stable shape for web addresses**, documented in
   [webhooks](../integrations/webhooks.md), with a header that carries its
   version, and an optional secret that signs the body with HMAC-SHA256.

## Consequences

- A manager who has `gh` signed in sees pull requests and issues on a task
  with no setup beyond a work folder that is a clone of the repository.
- Nothing is told while Teamwright is closed: there is no server to tell
  it. A web address hears only what happened on this computer.
- The last deliveries are lost when the program closes; they are a help
  for setting an address up, not a record.
- A new event needs a stable name and a line in the documented shape.

## Alternatives considered

- **A GitHub app or a token kept by Teamwright.** It would need a place to
  keep a secret safely and a flow to sign in; the manager's `gh` already
  has both.
- **Syncing the integrations with the project.** An address is a secret
  that should not travel through a shared folder, and two computers that
  both send would tell everything twice.
- **Slack as an app with its own sign-in, and reading the channel.** Far
  more than telling a channel needs; an incoming webhook does it.
- **Zapier as an integration of its own.** It would be a web address with
  another name.
- **GitLab and the other forty.** Later, one at a time, if they are asked
  for.

## Note of 18 November 2026: Google Drive, and the marks

- **Google Drive** is a fourth part of "Integrations…". It sets nothing per
  project: syncing is chosen once for the computer, on the page Computers
  ([decision 0017](0017-sync-through-a-folder.md)), and goes through the
  folder that Google Drive for desktop keeps, the way GitHub goes through
  the manager's own `gh`. The part says how it works and whether this
  computer syncs through Drive, and leads to Computers.
- **The official marks** of GitHub and Google Drive head their parts, small
  and unaltered (`ServiceMark`, `shared/logos/`). GitHub's black or white
  form follows the ground of the theme. Slack is named in words: its mark
  is handed out only to those signed in to its media kit.
