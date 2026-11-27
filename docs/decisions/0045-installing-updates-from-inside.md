# 0045. Install a newer version from inside the program, when the manager presses the button

- Status: accepted
- Date: 2026-11-27

## Context

[Decision 0043](0043-a-notice-of-newer-versions.md) built the half of
updating that needed no key: the program notices that a newer version is
out and opens its release page. It left the other half unbuilt because
installing from inside needs update files signed with a key that only the
founder can make. The founder asked for it ("release an update so i can
automatically update") and made the key. Its private half is a secret of
the repository, `TAURI_SIGNING_PRIVATE_KEY`, without a password; its public
half is in `tauri.conf.json`.

The founder uses Teamwright on a Mac, installed from the release page.
Version 0.2.0 was published on 23 November 2026 as a pre-release. Its
installers are not signed with Apple's or Microsoft's certificates.

Three facts about the repository shape the answer:

- Every release of the program is a pre-release, and the release GitHub
  marks "Latest" is another one: the tag `website-film`, which holds the
  website's film. The usual address of the updater,
  `…/releases/latest/download/latest.json`, would therefore find nothing.
- The release workflow ran the same four jobs, each of which tried to
  create the release. On 23 November two did so in the same second, and
  one failed after it had built.
- tauri-action, which builds the installers, can write `latest.json`
  itself. It reads the file already on the release, adds its own entry,
  deletes the old file and uploads the new one. Four jobs that do this at
  the same time can each read the file before the others have written it,
  and the last to write wins: entries can be lost. Its version 1 also
  changed the address it writes for each file to an address of GitHub's
  interface, not the public download address. (Read in its source, tags
  `action-v0.6.2` and `action-v1.0.0`, on 27 November 2026.)

## Decision

1. **The updater plugin of Tauri, called from the shell.** The program
   carries `tauri-plugin-updater` (2.14). The window has no permission for
   it: the commands of the shell call it, so the rules stay in Rust and the
   window keeps the one way it talks to the program. A restart goes
   through Tauri's own `request_restart`, so no process plugin is needed.
2. **Only when the manager presses the button.** Decision 0043 still
   decides when the program looks: never by itself until the manager
   switches looking on, at most once a day, and when "Look now" is pressed.
   The notice is what shows a newer version. Only when the manager presses
   "Install 0.3.0" does the program ask the address of updates, download,
   have the signature checked and put the version in place. It never
   restarts by itself: it says the new version is ready and offers
   "Restart now" and "Later".
3. **The address of updates decides what is installed; the rule of the
   domain decides what is newer.** The notice reads GitHub's list of
   releases; the updater reads `latest.json`. They could disagree, for
   example while a release is published and its `latest.json` is not yet.
   What is installed is what the address of updates offers, because only
   that has signed files. Whether it is newer is decided by
   `is_installable` in `crates/domain/src/release.rs`, for both: the plugin
   is given a comparator that calls it, and the use case asks it again. It
   is the rule of decision 0043, with one more clause: a version with a
   mark (`1.1.0-rc.1`) is installed only over a program that has a mark
   itself. When the address offers nothing newer although the notice knows
   of a release, the card says so in words and keeps the release page.
4. **Where the program asks:** the fixed release `updates`, at
   `https://github.com/fouad1233/teamwright/releases/download/updates/latest.json`.
   The release workflow replaces its one file, `latest.json`, at the end
   of each release. It is HTTPS, under the project's own repository, and it
   stays right whatever GitHub marks "Latest". It is a pre-release and is
   not marked latest, so it never takes "Latest" from the film; its tag is
   not a version, so the notice ignores it. The alternative, `latest.json`
   published with the website on GitHub Pages, lost: the website is
   published by another workflow from `main`, so every change to the
   website would have to carry the file along or remove it, and a Pages
   build that fails or lags would hold updates back. Its address would also
   be on a domain the program does not otherwise name.
5. **What an attacker can do with the address.** The signature of every
   file is checked against the public key built into the program; a file
   whose signature does not match is refused before anything is replaced.
   `requireSignedVersion` is on: the version the address announces must be
   the version the signature was made for, so an old, genuine file cannot
   be offered under a newer number. Someone who could replace `latest.json`
   but not sign could therefore only withhold updates or offer one the
   program already refuses. Losing the private key means the copies in use
   can never be updated from inside again; the founder keeps a copy outside
   GitHub.
6. **The release workflow.** A first job creates the release once, as a
   draft. Four builds add their installers, update files and `.sig` files
   to it; none of them writes `latest.json` (`includeUpdaterJson: false`).
   A last job runs only when all four succeeded: it writes `latest.json`
   from the four signatures with `tools/release/updater-manifest.mjs`,
   publishes the release, and replaces the file on the release `updates`.
   The script refuses, and nothing is published, unless every computer has
   its update and every signature names the version of the tag. A build
   that fails therefore leaves every copy on the version before, never a
   `latest.json` that points at missing files.
7. **Which copies install their own updates.** The bundler of Tauri marks
   the program with the kind of installer it was built for. The `.app` on
   macOS, the setup program and the `.msi` on Windows, and the AppImage on
   Linux install their own updates. A `.deb` or `.rpm` does not: its files
   belong to the package manager of the system, replacing them needs the
   administrator's password, and the package manager would go on believing
   the old version is installed. The plugin can install a `.deb` with the
   administrator's password, but the workflow does not publish one in
   `latest.json`, and Teamwright does not ask for that password. A copy
   built from the source has nothing to replace. Those copies keep the
   notice and the release page, and the card says why in a sentence.
8. **Specialists at work.** A restart ends the turns that run on this
   computer, as closing the program does. While any run, the card says how
   many, and the button reads "Stop them and restart"; the shell refuses a
   restart that does not carry that yes (`specialists_at_work`).
9. **Windows.** There the installer replaces a program that is no longer
   running, so the update is downloaded and checked first, and put in place
   only when the manager chooses "Restart now". The specialists are
   stopped before the installer starts. "Later" keeps the download until
   the program closes.
10. **The first step is by hand.** Version 0.2.0 carries no updater. The
    founder installs the next version from the release page once; from
    then on, the program updates itself. The notes of the release say so.

## What is known about macOS, and what was never tried

Nothing was run on a Mac. What follows is from Tauri's documentation, the
source of the plugin (2.14.1) and what is publicly known about Gatekeeper.

- The first install of an unsigned program is the one macOS warns about:
  the `.dmg` downloaded by a browser carries the quarantine attribute, and
  the manager allows it once under Privacy & Security.
- The plugin downloads the `.app.tar.gz` itself, over HTTPS, and unpacks
  it with its own code into a temporary folder on the same volume, then
  swaps it with the installed bundle. It does not set the quarantine
  attribute, and macOS sets it only on files written by programs that ask
  for it (`LSFileQuarantineEnabled`) or that run in a sandbox; Teamwright
  does neither. So the updated bundle is expected not to be quarantined
  again, and not to be checked by Gatekeeper at its next start. Expected,
  not seen.
- Programs from the internet that were never moved out of the Downloads
  folder or a disk image may be run from a hidden, read-only copy (App
  Translocation). An update cannot replace that copy. Moving Teamwright to
  Applications before the first start avoids it.
- Since macOS 13, the setting "App Management" can stop one program from
  changing another program's bundle. Whether it stops an unsigned program
  from replacing its own bundle was not found in writing and was never
  tried. If it does, the plugin falls back to asking for the
  administrator's password through a system dialog, and the card says that
  the version could not be put in place if that fails too.
- The program built on the Mac runners carries an ad-hoc signature (Apple
  silicon refuses to run code without one). The update keeps the signature
  it was built with.

## Consequences

- The local proof (`tools/smoke/update-linux.mjs`) builds an older and a
  newer AppImage with throwaway keys, serves a hand-written `latest.json`,
  and drives the older program through its window: it finds the update,
  refuses the copy signed with another key in words, downloads the right
  one with its progress, installs it and restarts as the new version. It
  proves the program and the plugin; it cannot prove GitHub, the workflow
  or macOS.
- What only a real release can confirm: that tauri-action 0.6 uploads to
  a draft named by its id, that it uploads the `.sig` files with the names
  the script expects, that `gh` finds a draft by its tag, that the release
  `updates` is created and its file replaced, and that GitHub's download
  address redirects in a way the plugin follows.
- Every local build with installers now needs the signing key, since
  `createUpdaterArtifacts` is on. The development builds use `--no-bundle`
  and do not; a build of installers without the key fails with Tauri's own
  message.
- The program knows one more address, the one in `tauri.conf.json`.
- The variable `TEAMWRIGHT_RELEASES_ADDRESS` points the notice at another
  list of releases, for the local proof. It cannot change the address of
  updates or the key, which are built in.

## Alternatives considered

- **tauri-action writing `latest.json`.** It loses entries when the four
  builds write at once, and its address scheme changed between versions.
- **`…/releases/latest/download/latest.json`.** "Latest" is the film's
  release.
- **The JavaScript half of the plugin, called from the window.** It would
  need the window's permission to download and install, and would put the
  rules of updating in the interface.
- **Installing by itself, or restarting by itself.** Decision 0026: the
  program does nothing on its own that the manager did not allow, and a
  restart stops the specialists.
- **Updating `.deb` and `.rpm` copies with the administrator's password.**
  The plugin can, but a program asking for that password is a habit worth
  not teaching, and the package manager would no longer know the version.
