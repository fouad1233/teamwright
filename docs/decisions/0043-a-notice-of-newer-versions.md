# 0043. Tell the manager when a newer version is out, looking only when allowed; installing from inside the program is not built

- Status: accepted
- Date: 2026-11-21

## Context

Version 0.1.0 is published as a pre-release on GitHub. The installers are
not signed. The roadmap (M8) ends its line about installers with "Updates
from inside the program are not built".

Installing an update from inside the program, as the Tauri updater does,
needs update files that are signed, and a signing key that only the founder
can make and keep. Without it the program would download and run code
nobody vouched for. So that half stays unbuilt, and the program does not
carry the updater plugin.

What can be done today, honestly, is smaller: the program can notice that a
newer version exists and say where to get it. The manager then installs it
the way they installed this one.

The program otherwise calls out only where the manager sent it: a Slack
channel, a web address, GitHub through the manager's own `gh`
([decision 0027](0027-integrations.md)). A look for a newer version would
be the first call the program makes on its own, to a service the manager
never named. Decision 0026 set the care for things the program does by
itself: nothing until the manager allows it.

## Decision

1. **A rule in `domain`** (`release.rs`): `Version` reads `0.2.0`,
   `v0.2.0` and `v1.0.0-rc.1`, and compares them part by part as numbers
   (`0.10.0` is newer than `0.9.0`), a version with a mark before the same
   numbers without it. A tag that is not a version (the repository has one,
   `website-film`) is ignored. `newer_release` picks the newest release
   that is newer than this program.
2. **Pre-releases.** A full release is offered when it is newer. A
   pre-release is offered only while this program is itself one (its
   version has a mark, or a published pre-release has its version), or
   while no full release has been published at all. Reason: the only
   release today is a pre-release, so ignoring them all would never show
   anything; but a manager on a settled release should not be sent to an
   unfinished one. A manager on the pre-release 0.1.0 hears of 0.2.0, and
   of 1.0.0 once it exists. The words in the interface call a pre-release
   one.
3. **A port in `application`** (`ReleaseSource`), implemented in
   `crates/integrations` with `ureq` against the public GitHub API, without
   a token, with a time limit of 8 seconds and at most 2 MB read. The
   request carries no identity: GitHub sees the address of the list and,
   in the `User-Agent` header that GitHub requires, `Teamwright/<version>`.
   A test listens on this computer and checks that these are the only
   headers. Any failure (no network, a status that is not 200, an answer
   that is not a list) is only "could not look": the status says so in
   words, and it is never shown as an error. The address of a release is
   written by the program from the repository and the tag; the address in
   the answer is not used.
4. **Off until switched on.** Decision 0026 applies: the program does not
   call out by itself until the manager allows it. The request says little
   (an address and a version), but it still tells GitHub that someone runs
   Teamwright from some network, and the program says everywhere else that
   nothing goes anywhere the manager did not send it. A switch on the page
   Computers, off at first, says plainly what is asked and how often.
   "Look now" works whether or not the switch is on, because the manager
   pressed it, and does not turn the switch on.
5. **At most one look a day by itself**, and one when the manager presses
   "Look now". The window asks the program whether a look is due when it
   opens and once an hour; the program decides, from the time of the last
   look. A look that failed counts as a look, so a network that is down is
   not asked again every hour. A clock set back does not stop looking for
   good.
6. **A fact about this computer.** The switch, the time of the last look,
   whether it failed, and the release it offered are kept in `meta`, not
   logged, so never synced, like the theme and the choice about
   notifications. No migration was needed. A release that is no newer than
   the program (because the manager installed it since) is no longer shown.
7. **The interface.** On the page Computers, a quiet part "Version": the
   version of this program, the switch, "Look now" with the time of the
   last look, and, when a newer version is known, a card with its number, a
   sentence and a button that opens its release page through the existing
   safe opening of web addresses (`open_web_address`, decision 0027). At the
   foot of the navigation, under the Computers line, a small line "0.2.0 is
   out". It has no joints and no gold (decision 0015): a newer version
   waits for nothing.

## Consequences

- Installing from inside the program is still not built. It needs signed
  update files and a signing key from the founder, then the Tauri updater
  plugin and its permission. This record is the half that works without
  them; when the updater arrives, the notice is where it starts.
- The program now knows one more address: `api.github.com`. It is named in
  one place, `crates/integrations/src/releases.rs`.
- GitHub limits unauthenticated requests by address (60 an hour). One look
  a day is far below it; a limit that is hit reads as "could not look".
- The demo backend pretends: off at first, and a look says that 0.2.0 is
  out, so the part shows in a browser. Nothing leaves the browser.
- The real answer of GitHub was read once by hand on 2026-11-21: two
  releases, `v0.1.0` (a pre-release) and `website-film` (not a version,
  ignored). The tests never call GitHub.

## Alternatives considered

- **On, with a switch.** Fewer managers would miss a new version. It
  lost because the first call a program makes on its own should be one the
  manager allowed, and because the founder's line is that nothing leaves
  the computer unasked. The switch is one press away, and the marks
  of a newer version appear as soon as it is on.
- **The Tauri updater plugin now.** It needs a key that only the founder
  can make; without it the plugin cannot verify anything.
- **Ignoring every pre-release.** Nothing would ever show while 0.1.0 is
  the only release.
- **Comparing the tags as text.** `v0.10.0` would sort before `v0.9.0`.
- **Using `html_url` from the answer.** The program writes the address
  itself, so an answer that is wrong cannot send the manager elsewhere.
- **Keeping the choice in the log of operations.** It would sync, and a
  choice to call out is about one computer.
