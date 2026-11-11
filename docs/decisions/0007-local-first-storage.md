# 0007. Store locally and sync through a user-owned folder

- Status: accepted
- Date: 2026-11-11

## Context

The founder wants the app to cost its users nothing beyond their Claude plan,
and wants the same projects on several computers by keeping the data in a
folder that Google Drive synchronizes. Each computer should be identifiable,
so it is clear where a piece of work happened.

A database file inside a synchronized folder is a known way to lose data. The
sync program uploads the file while it is being written, and two computers
writing the same file produce conflict copies that nothing can merge.

## Decision

- **Current state** lives in a SQLite database in the application's own data
  directory, outside any synchronized folder.
- **Every change** is also appended to an operation log. Each entry carries
  the ID of the computer that made it and a hybrid logical clock timestamp,
  which orders changes correctly even when computer clocks disagree.
- **Sync** means exchanging log entries through a folder the user chooses.
  Each computer writes only its own files there and reads everyone else's.
  Since no file ever has two writers, the sync program has nothing to
  conflict over.
- **Merging** is last-writer-wins per record, decided by the logical clock.
- All writes go through one storage layer, so nothing can change state
  without being logged.

Sync itself is a later milestone. The log and the device ID exist from the
first version of the storage layer, because adding them afterwards would mean
rewriting it.

## Consequences

- The app works fully offline and needs no account or server.
- Any provider that synchronizes a folder works, not only Google Drive.
- If two computers edit the same record while disconnected, the later edit
  wins and the earlier one is kept only in the log. This is acceptable for
  one manager using several computers and must be revisited before several
  people share a project.
- The log grows and will need compaction.

## Alternatives considered

- **The database file in the synchronized folder.** Corruption and conflict
  copies.
- **A hosted backend.** Costs money to run and needs accounts.
- **The Google Drive API.** Requires registering and verifying an
  application with Google and ties the feature to one provider.
- **A general CRDT library.** More machinery than records with fields need.
