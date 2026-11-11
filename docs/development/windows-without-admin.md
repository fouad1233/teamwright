# Building on Windows without administrator rights

The usual way to build Rust and Tauri on Windows needs the Microsoft C++
Build Tools, and installing those needs an administrator. This page describes
a setup that stays entirely inside your user folder. Teamwright's first
development machine works this way.

If you can install the Microsoft C++ Build Tools ("Desktop development with
C++"), do that instead and skip this page. It is the standard path and the
one release builds use.

## What you install

| Tool | How | Administrator needed |
|---|---|---|
| Rust with the GNU toolchain | `rustup-init.exe -y --default-host x86_64-pc-windows-gnu` from [rustup.rs](https://rustup.rs) | No |
| MinGW-w64 GCC, MSVCRT build | `winget install --id BrechtSanders.WinLibs.POSIX.MSVCRT -e` | No |
| Node.js 22 or later | [nodejs.org](https://nodejs.org) | Depends on the installer |
| WebView2 runtime | Part of Windows 11 | |

The MSVCRT build of MinGW-w64 is the one that matches Rust's
`x86_64-pc-windows-gnu` target.

## If no other GCC is on your computer

Then you are done. `winget` puts the compiler on your user `PATH`, and Cargo
finds every tool by name.

## If an older MinGW or GCC comes first on your `PATH`

A system-wide `PATH` entry, for example an old TDM-GCC, wins over anything in
your user `PATH`, and you cannot change it without an administrator. The
build then picks up the wrong tools. Name the right ones explicitly in a file
`.cargo/config.toml` in the root of the repository. The file is ignored by
git, because it describes one computer.

```toml
[target.x86_64-pc-windows-gnu]
linker = "<mingw>/bin/x86_64-w64-mingw32-gcc.exe"
rustflags = ["-C", "dlltool=<mingw>/bin/dlltool.exe"]

[env]
CC_x86_64_pc_windows_gnu = "<mingw>/bin/gcc.exe"
AR_x86_64_pc_windows_gnu = "<mingw>/bin/ar.exe"
TEAMWRIGHT_TOOLCHAIN_BIN = "<mingw>/bin"
```

Replace `<mingw>` with the folder that `winget` unpacked, written with
forward slashes. It is below
`%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.MSVCRT_…\mingw64`.

What each line is for:

| Line | Without it |
|---|---|
| `linker` | Rust links with its own minimal linker, which does not match the objects the newer compiler produces. |
| `dlltool` | `rustc` creates import libraries with the first `dlltool` on the `PATH`. An old 32-bit one fails with "Invalid bfd target". |
| `CC_…`, `AR_…` | Crates that compile C code, such as SQLite, use the first `gcc` on the `PATH`. |
| `TEAMWRIGHT_TOOLCHAIN_BIN` | Tauri compiles the application's icon with a program called `windres` and finds it by name. The build script of the desktop crate puts this folder first on its own `PATH` so the right one is found. An old `windres` fails with "Can't detect target endianness and architecture". |

## Limits of this setup

- A Tauri library crate cannot be built as a `cdylib` with the GNU toolchain
  ("export ordinal too large"). Teamwright's desktop crate is a plain binary
  and is not affected.
- With the GNU toolchain the application needs `WebView2Loader.dll` next to
  the executable. The build puts it there.
- Release builds for distribution are made with the Microsoft toolchain on a
  build server, not with this setup.

## Checked with

Rust 1.99.1, GCC 16.2.0 (WinLibs r3), Tauri 2.12, on Windows 11 without
administrator rights, on 11 November 2026.
