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

## What was tried

On 21 November 2026, on Ubuntu 24.04 with Rust 1.100: the four checks of
[the development page](README.md) pass with this setup, the desktop crate
included. The folder takes 54 MB. The built program was not started from
this setup on that day.

When Ubuntu updates WebKitGTK, the links above may point at a file that is
gone. Run the two loops again.
