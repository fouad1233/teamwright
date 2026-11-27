# Building on Linux without administrator rights

The usual way to build Tauri on Linux installs development packages with
`sudo apt-get install`
([the Tauri prerequisites](https://tauri.app/start/prerequisites/)). This
page describes a setup that stays entirely inside your home folder.
Teamwright's second development machine, a shared lab computer with Ubuntu
24.04, works this way.

If you can use `sudo`, install the packages the build server installs
(`.github/workflows/ci.yml`) and skip this page.

## What you need already

- A desktop Ubuntu, so that the libraries the program runs with are there:
  GTK 3, WebKitGTK 4.1, libsoup 3. Only their development packages are
  missing.
- A C compiler and `pkg-config`.
- Node.js 22 or later.

## Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup-init.sh
sh rustup-init.sh -y --no-modify-path --default-toolchain stable --profile minimal -c rustfmt -c clippy
```

Rust goes to `~/.cargo` and `~/.rustup`. `--no-modify-path` leaves your
shell profile alone; the file at the end of this page puts Cargo on the
`PATH` when you ask for it.

## The development packages, unpacked into a folder

`apt-get download` fetches a package without installing it and needs no
administrator. `dpkg -x` unpacks it anywhere. Ask `apt-get` which packages
it would add, download exactly those, and unpack them into one folder:

```bash
SYSROOT="$HOME/.local/opt/tauri-sysroot"
mkdir -p "$SYSROOT/debs" && cd "$SYSROOT/debs"

apt-get -s install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
    librsvg2-dev libxdo-dev libssl-dev \
  | awk '/^Inst/ {print $2}' > packages.txt
echo libxdo3 >> packages.txt

apt-get download $(cat packages.txt)
for package in *.deb; do dpkg -x "$package" "$SYSROOT"; done
```

Run this in `bash`. In `zsh` a variable is not split into words, so write
the list out or use `bash`.

Two things in the unpacked folder still point at the system and have to be
turned around.

**The links.** A development package holds a link such as
`libgtk-3.so -> libgtk-3.so.0.2404.30`, and that file is in
`/usr/lib/x86_64-linux-gnu`, not in the folder. Point every broken link
there:

```bash
LIBRARIES="$SYSROOT/usr/lib/x86_64-linux-gnu"
for link in "$LIBRARIES"/*.so; do
  if [ -L "$link" ] && [ ! -e "$link" ]; then
    name=$(basename "$(readlink "$link")")
    if [ -e "/usr/lib/x86_64-linux-gnu/$name" ]; then
      ln -sfn "/usr/lib/x86_64-linux-gnu/$name" "$link"
    fi
  fi
done
```

**The `.pc` files.** They say `prefix=/usr`. Make them say where the files
really are:

```bash
for folder in "$LIBRARIES/pkgconfig" "$SYSROOT/usr/share/pkgconfig"; do
  sed -i "s|^prefix=/usr|prefix=$SYSROOT/usr|; \
          s|=/usr/lib/x86_64-linux-gnu|=$SYSROOT/usr/lib/x86_64-linux-gnu|g; \
          s|=/usr/include|=$SYSROOT/usr/include|g" "$folder"/*.pc
done
```

## A file to source before building

Save this as `~/.local/opt/tauri-sysroot/env.sh`:

```bash
SYSROOT="$HOME/.local/opt/tauri-sysroot"
export PATH="$HOME/.cargo/bin:$PATH"
export PKG_CONFIG_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig:$SYSROOT/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export LIBRARY_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu${LIBRARY_PATH:+:$LIBRARY_PATH}"
export LD_LIBRARY_PATH="$SYSROOT/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
```

`PKG_CONFIG_PATH` lets the build scripts find the libraries.
`LIBRARY_PATH` lets the linker find `libxdo`, which has no `.pc` file.
`LD_LIBRARY_PATH` lets the tests and the program find the few libraries
that were not on the computer at all (`libxdo3` among them).

Then, in every shell that builds:

```bash
. ~/.local/opt/tauri-sysroot/env.sh
npm ci
npm run build
cargo clippy --all-targets -- -D warnings
cargo test
```

## The smoke test

[The smoke test on Linux](README.md#smoke-test-of-the-real-application-on-linux)
drives the built program through WebDriver. It needs three programs.

**Xvfb**, the virtual display, is part of a desktop Ubuntu
(`/usr/bin/Xvfb`).

**tauri-driver**, Tauri's WebDriver server, is built with Cargo into
`~/.cargo/bin`:

```bash
cargo install tauri-driver --locked
```

**WebKitWebDriver** comes with Ubuntu's package `webkit2gtk-driver`, which
is not installed on a desktop. Fetch it and unpack it into the same folder
as the development packages. Its version must be the version of the
WebKitGTK that is installed; `apt-get download` takes the one that goes
with it:

```bash
cd "$SYSROOT/debs"
apt-get download webkit2gtk-driver
dpkg -x webkit2gtk-driver_*.deb "$SYSROOT"
"$SYSROOT/usr/bin/WebKitWebDriver" --help
```

The smoke test finds it there by itself, and finds `tauri-driver` in
`~/.cargo/bin`. Name others in `WEBKIT_WEBDRIVER` and `TAURI_DRIVER`. Then:

```bash
. ~/.local/opt/tauri-sysroot/env.sh
npm run tauri --workspace @teamwright/desktop -- build --debug --no-bundle
node tools/smoke/desktop-smoke-linux.mjs target/debug/teamwright-desktop /tmp/teamwright-pictures
```

The program writes two warnings about DRI3 on the virtual display; they
mean only that it draws without the graphics card.

## Building an AppImage

The test of the updater builds AppImages. The bundler's `linuxdeploy` and
its plugin for GTK ask `pkg-config` where GTK's library folder is, and copy
the modules of GTK from there. With the unpacked packages, that folder is the
one under `$SYSROOT`, which holds no modules, and the build stops with
"Failed to run plugin: gtk". Point `pkg-config` at the system's folder for
GTK only, with a copy of its `.pc` file that says so, put before the others:

```bash
mkdir -p "$SYSROOT/appimage-pkgconfig"
sed 's|^libdir=.*|libdir=/usr/lib/x86_64-linux-gnu|' \
  "$SYSROOT/usr/lib/x86_64-linux-gnu/pkgconfig/gtk+-3.0.pc" \
  > "$SYSROOT/appimage-pkgconfig/gtk+-3.0.pc"
export PKG_CONFIG_PATH="$SYSROOT/appimage-pkgconfig:$PKG_CONFIG_PATH"
```

The linker still finds `libgtk-3.so` through `LIBRARY_PATH`. Do this only in
the shell that builds AppImages.

## What was tried

On 21 November 2026, on Ubuntu 24.04 with Rust 1.100: the four checks of
[the development page](README.md) pass with this setup, the desktop crate
included. The folder takes 54 MB. The built program was not started from
this setup on that day.

On 22 November 2026, on the same computer, with WebKitGTK 2.52.6,
`webkit2gtk-driver` 2.52.6 and `tauri-driver` 2.1.0: the built debug
program started under Xvfb, and the smoke test on Linux and the test of an
old database passed against it.

When Ubuntu updates WebKitGTK, the links above may point at a file that is
gone. Run the two loops again.
