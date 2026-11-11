# 0001. Record significant decisions

- Status: accepted
- Date: 2026-11-11

## Context

Teamwright is built in the open and wants two things from its history. New
contributors, human or AI, should be able to find out why the code is the way
it is without asking. And the project's story will be told in an animated
film, which needs the real reasons behind each turn, written down when they
were fresh.

## Decision

Decisions with lasting weight are recorded as short numbered documents in
`docs/decisions`. Each has a context, the decision, its consequences and the
alternatives that were considered. Records are append-only: a changed decision
gets a new record that supersedes the old one.

The narrative version of events lives in `docs/story` and links to these
records.

## Consequences

- A change that reverses a recorded decision must add a record.
- The records are part of code review, like tests.
- Small, easily reversed choices stay out of here and in commit messages.

## Alternatives considered

- **Only commit messages.** They explain a change, not the options that were
  rejected, and they are hard to browse.
- **A wiki.** It lives outside the repository and drifts away from the code.
