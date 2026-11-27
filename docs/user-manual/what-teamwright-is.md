# What Teamwright is, and what you need

This page says what Teamwright is for, what must be on your computer before
it can do its main job, and how to install it.

## What it is

Teamwright is a desktop program for running a project the way a manager runs
a team. You are the manager. The team members are AI specialists, and a task
can also be given to a person. You keep the project in Teamwright: its tasks,
their order, who is responsible, what is done. The Advisor helps you plan
it, and the specialists carry out tasks. They work through Claude Code, the
program of Anthropic, on your own computer and under your own account.

Teamwright is an independent project. It is not made by Anthropic.

You decide. A specialist never marks a task as done by itself, and it stops
and asks when a choice is yours.

You can use Teamwright as a plain project tracker without any specialist. The
board, the list, the Gantt chart and the other views work without Claude
Code.

## What you need

- **To track projects:** only Teamwright.
- **For the Advisor and the specialists:** Claude Code must be installed on
  the same computer, and you must be signed in to it. Teamwright does not
  ask for your password and does not read how you are signed in. The *Team*
  place says what it found: the version of Claude Code and how you are
  signed in, or what is missing. To sign in, open a terminal and run
  `claude auth login`.
- **What it costs.** A specialist uses your own Claude plan or account, as
  if you had used Claude Code yourself. Nothing goes through Teamwright or
  its authors.

Note: a Claude Code that was installed with npm can be a script on Windows.
Teamwright cannot run a turn on it yet, and says why. This has not been
solved.

## Installing

1. Open the [releases](https://github.com/fouad1233/teamwright/releases) of
   Teamwright and choose the newest one.
2. Download the file for your system:
   - Windows: the file that ends in `-setup.exe`, or the `.msi`.
   - macOS: the `.dmg`. `aarch64` is for Macs with Apple silicon, `x64` for
     Macs with an Intel processor.
   - Linux: the `.AppImage`, the `.deb` or the `.rpm`.
3. Open the file and follow the installer.

**The installers are not signed.** The project has no signing certificates
yet, so your system warns you before it opens the program.

- Windows shows "Windows protected your PC". Choose "More info", then "Run
  anyway".
- macOS says the app cannot be opened. Open System Settings, then Privacy &
  Security, and choose "Open Anyway". Or right-click the app and choose
  "Open".

Version 0.1.0 is a pre-release. Nobody has yet installed it and started it
from these installers, and the macOS and Linux builds have never been run.
Tell the project if one of them does not start.

On a Mac, move Teamwright to Applications before you open it the first time.

**Updates.** The program can tell you when a newer version is out, and
install it when you press the button (see
[settings of this computer](settings.md#installing-a-newer-version)). Every
update is checked against the project's signing key before it is installed.
Version 0.2.0 cannot do this yet: install the next version by hand once, the
same way as above, and from then on Teamwright updates itself. On Linux this
holds for the AppImage; a `.deb` or `.rpm` is updated by installing the new
package.

## Trying it without installing

The project publishes a demo that runs in a browser, with sample data and a
pretended specialist. Nothing in it is saved and nothing is sent to a model.
It is not published yet at the time of writing; the roadmap says when it is.
