# 0040. The limit of the Claude plan is its own ending of a turn, known per computer, said once, and never guessed

- Status: accepted
- Date: 2026-11-21

## Context

On the evening of 21 November 2026, at about 20:15, the eight agents that
were building Teamwright stopped at once. They ran on the founder's plan,
and each ended with the same words: "You've hit your 5-hour limit · resets
10:40pm (Europe/Istanbul)", reported as an API error of type `rate_limit`,
HTTP 429. A manager whose specialists work through the night meets exactly
this. Teamwright showed it as eight failed turns with the raw sentence in
red, eight notifications "stopped", and nothing that said when work could
go on. Once the coordinator starts work by itself, it would start more
turns that fail the same way.

Decision 0008 said that plan limits are shown when Claude Code reports
them and are otherwise unknown, never guessed. The roadmap had "the limits
of the plan that the program reports" as not started.

### What Claude Code reports, and where it was found

Claude Code was not signed in on this computer, so nothing was run. What is
known comes from three places:

1. **The recordings of real turns** (`crates/runtime/tests/recordings`,
   Claude Code 2.1.322 on Windows, a subscription). Every turn has a
   `rate_limit_event` with `rate_limit_info`: `status` (`allowed`),
   `resetsAt` (seconds since 1970), `rateLimitType` (`five_hour`),
   `isUsingOverage`, and `unifiedWindows` with `five_hour` and `seven_day`,
   each with `utilization` (0.25 is a quarter used), `resetsAt` and
   `status`. So on a normal turn the program does report how much of the
   five hours and of the week is used, and when each resets. The decoder
   already turned this into `AgentEvent::PlanUsage`; nothing showed it.
2. **The schema inside the installed program** (2.1.289, read from the
   text of its binary). `rate_limit_info.status` is one of `allowed`,
   `allowed_warning` and `rejected`; `rateLimitType` one of `five_hour`,
   `seven_day`, `seven_day_opus`, `seven_day_sonnet`,
   `seven_day_overage_included` and `overage`; `utilization` and
   `resetsAt` are optional; `unifiedWindows` is marked internal, "as read
   from the anthropic-ratelimit-unified-* response headers", absent for
   API keys, Bedrock and Vertex, and its utilization may go above 1. The
   assistant message of an API error carries `error` (one of
   `authentication_failed`, `billing_error`, `rate_limit`, `overloaded`,
   and others), `is_api_error_message` and `api_error_status`. The
   sentence for people is built as "You've hit your {name} · resets
   {time}", where the name in 2.1.289 is "session limit", "weekly limit",
   "Opus limit", "Sonnet limit" or "Fable limit" (tonight's newer version
   said "5-hour limit"), and the time is "10:40pm" or "10pm" within a day,
   "Nov 23, 10am" further away, with a year when it differs, followed by
   the time zone of the computer in parentheses. Older versions wrote
   "Claude AI usage limit reached|<seconds>".
3. **The documentation** (code.claude.com/docs, the Agent SDK reference
   for TypeScript). `SDKRateLimitEvent` is documented with `status`
   (`allowed`, `allowed_warning`, `rejected`), `resetsAt` and
   `utilization`. `rateLimitType` and `unifiedWindows` are not documented,
   and neither is the sentence. The headless page lists `rate_limit` as a
   value of the `error` of an `api_retry` event.

Not found: a documented way to ask the program how much of a window is
used outside a turn; the shape of the stream on the turn that meets the
limit (no recording of it exists; the shape here is built from the schema
and from tonight's words); and whether a refused turn always carries a
`rate_limit_event` with `rejected` before its error.

## Decision

1. **Its own ending.** The runtime recognises a turn that ended because
   the limit was reached as `TurnFailure::PlanLimitReached`, not as an
   ordinary failure. The fields come first: a `rate_limit_event` whose
   status is `rejected`, with the window and the reset moment it names.
   The sentence is the fallback, read tolerantly (any of the wordings
   above, curly or straight apostrophe, on any line of the result, the last
   text or the error lines of a program that ended early). An ordinary
   error is never taken for it: "API Error: Rate limit reached" of an API
   key, an `error` of `rate_limit` without the sentence, "Overloaded",
   budgets and credits are ordinary failures.
2. **The reset as a moment, carefully.** A moment given as a number is
   used as it is. A time in words is turned into the next such time after
   now, in the zone it names, by the clock of the application: "10:40pm"
   said at 20:15 is the same evening; said at 23:00 it is tomorrow; a date
   without a year that has passed is next year's. The program carries no
   database of the world's zones: a zone is known if it is this
   computer's (Claude Code runs here and names this computer's zone) or
   Coordinated Universal Time. Any other zone, or words that are not
   understood ("in 3h"), leave the reset unknown and the limit known.
3. **The turn.** It ends failed, with the words "The limit of the Claude
   plan was reached." and a mark (`Turn::reached_plan_limit`). It is no
   `AgentStopped` event and no notification of its own, the task stays in
   work, and the interface shows it quietly, without the red mark of a
   failure. A version that does not know the mark shows a failed turn with
   those words.
4. **Kept per computer, never synced.** What is known about the plan (the
   limit if reached, when it was reached, its window and reset, and the
   windows as last reported) is a fact about the Claude account signed in
   on this computer. It is kept in `meta` under `plan_limit`, written
   without the recorder, like the choice of notifications and the theme
   (decisions 0026 and 0028). Another computer may run another account. The
   mark on a turn is logged with the turn, because it says how that turn
   ended; it is written only when true, so every other turn is logged as
   before. Migration 0028 adds its column.
5. **Told once.** Eight turns that end at the same limit change what is
   known once: one notification of the kind "stopped", "The limit of your
   Claude plan is reached" with "It resets at 22:40. No new work starts
   before then." One more when it has reset. The shell wakes once at the
   known reset to look; it never polls. A reset that is not known is not
   waited for.
6. **When it lifts.** At its reset; when the program reports that the plan
   allows work (`allowed` or `allowed_warning`) and the reset is not known
   to be still to come; and when a turn that started after the limit was
   reached went through. A turn that started before and finished after
   says nothing, because the limit may have reached it later.
7. **For the coordinator.** `may_start_work(state, now)` (and
   `PlanLimitService::may_start_work`) answers `Allowed`, `NotUntil(moment)`
   or `NotUntilUnknown`. The coordinator, which another change builds,
   starts nothing while the answer is not `Allowed`.
8. **By hand, with a warning.** The manager may still start work or press
   "Try again" while the limit is reached. The manager may know better:
   the plan may have been upgraded, extra usage bought, or the reset may
   have come early, and only a turn that is tried can tell. The words
   beside the button say that it will most likely stop again at once.
9. **What is shown.** One plain line at the foot of the navigation, where
   the things of this computer stand: "The limit of your Claude plan is
   reached. It resets at 22:40." It is not lit (decision 0015): nothing
   waits for the manager, only for the time. The Usage part of the
   Dashboard shows the plan: reached or not, the reset, and of the five
   hours and the week the share used and the reset, as Claude Code last
   reported them, or "unknown". A window whose reset has passed is
   unknown again, because it started over.

## Consequences

- The night of 21 November would now read as one line, one notification,
  eight quiet turns with "Try again", and no work started before 22:40.
- The percentages come from a field the program marks as internal. If it
  disappears, the windows show "unknown" and the limit is still recognised
  from the status and the sentence.
- The sentence is in English and changes with the versions of the program.
  The reader is tested on several wordings; a new one may be missed, and
  the turn is then an ordinary failure with the sentence, as before.
- With a limit whose reset is unknown, the coordinator waits until the
  manager tries a turn by hand. The bound that a five-hour window resets
  at most five hours after it was reached is not used, because it is not
  what the program said.
- The test against the real program
  (`a_real_turn_reports_the_plan_or_ends_at_its_limit`) has never run.

## Alternatives considered

- **A new state of a turn.** Truer as a model, but a computer with an
  older version could not read a synced turn in a state it does not know.
  A mark on a failed turn degrades to what was shown before.
- **Keeping the limit in the log, synced.** The account of one computer
  is not the account of another; a limit reached on the laptop must not
  stop the desktop.
- **A timer that polls the program.** Nothing can be asked outside a
  turn, and a turn spends the plan. One wake at the known moment is
  enough.
- **A database of time zones.** A crate of some megabytes for a zone that
  is nearly always this computer's.
- **Refusing to start work by hand.** The manager may know better, and
  refusing would leave no way to learn that an unknown reset has passed.

## Note of 21 November 2026: the coordinator follows it

Point 7 is built. The coordinator starts nothing while `may_start_work` is
not `Allowed`, and says so once in its record for each time the limit is
reached. A task whose last turn ended at the limit is not counted as
failed: when the limit lifts, the coordinator goes on with it in its
conversation, as "Try again" does, once for each reset (decision 0037 has
the rule and its guard against a loop). When the limit lifts, the shell
nudges the coordinator of every project that runs on this computer; with a
reset that is not known, that is when a turn the manager started goes
through. To make this safe, a turn that ends now tells what it learned
about the plan before it tells that the conversation changed, so that the
coordinator, which that wakes, already knows the limit is reached. The
demo follows the same rule. Never run with a real Claude Code.
