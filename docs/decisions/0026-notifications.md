# 0026. Tell the manager through notifications of the system, off until allowed, worded by a rule in the application

- Status: accepted
- Date: 2026-11-16

## Context

The owner asked: "Implement notifications and allow notifications". A
specialist can work for many minutes. The manager meanwhile works in another
window and does not see that the specialist asks a question, has finished,
or has stopped. Other ways out of the window will follow soon: a message to
a chat channel, a call to a web address, links to GitHub. They need to know
the same things.

## Decision

1. **A module of events.** `teamwright_application::project_events` says in
   plain data what happened in a project: `TaskFinished`, `WaitsForManager`,
   `AgentStopped`, `SprintStarted`, `SprintCompleted`. The work of the
   specialists finds the event of a turn when the turn ends and hands it to
   its listener (`WorkListener::happened`), once. The shell finds the events
   of a sprint after its commands. There is no second way of noticing
   changes.
2. **The rule is in the application, not in the shell.** Which events are
   worth a notification, and their words, is a pure function
   (`notifications::notification_for`) with tests. It sees several stored
   things (the project, the specialist, the task), so by `AGENTS.md` it
   belongs in `application`. The shell only knows whether its window is in
   front and how to send.
3. **Three kinds, nothing else.** Someone waits for the manager, work on a
   task is ready, someone could not go on. A sprint is not told: the
   manager did it. A turn the manager stopped is not told either.
4. **Off until allowed.** Nothing is sent until the manager allows it.
   The program asks once, in its own quiet line in the inbox, when
   something first waits there; "No, thanks" is kept, and the program does
   not ask again. The page Computers has the switch, a switch for each
   kind, and a button that sends a test notification.
5. **A fact about this computer.** The choice is kept in `meta` and not
   logged, so it is never synced: one computer may show notifications and
   another not.
6. **Sent from Rust.** The shell sends through the official plugin
   (`tauri-plugin-notification`), only while the window is not in front.
   The interface has no part in sending, so the plugin's JavaScript package
   is not used and its permissions are not granted to the window.
   Pressing a notification brings the window to the front, and the
   interface opens what it was about (the inbox, the task, the Advisor or
   the conversation).

## Consequences

- The next listeners (chat, web address) are added beside the
  notifications in the shell's `events::tell`, on the same events.
- On Windows a desktop program needs no permission of the system to show a
  notification; "allowing" is the program's own switch. The system's own
  settings can still silence Teamwright.
- On Windows the plugin reports a press only while the notification is on
  the screen, not once it moved to the notification centre. A development
  build shows its notifications under the name of PowerShell; an installed
  one under its own.

## What was not taken

- **Asking the system for permission from the interface** through the
  plugin's JavaScript side: it needs an npm package and capabilities for
  the window, and on desktop the plugin answers "granted" anyway.
- **The rule in the shell or in the interface**: it would be untested, and
  every later listener would have to work it out again.
- **A notification for every change**: the manager would turn them off.
- **Keeping the choice in the synced data**: whether a computer shows
  notifications depends on the computer.
