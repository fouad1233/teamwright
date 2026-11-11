//! Build script of the desktop application.

use std::env;
use std::path::PathBuf;

/// Names a directory whose build tools are preferred over the ones on `PATH`.
const TOOLCHAIN_BIN_VARIABLE: &str = "TEAMWRIGHT_TOOLCHAIN_BIN";

fn main() {
    prefer_configured_toolchain();
    tauri_build::build();
}

/// Puts the directory named by `TEAMWRIGHT_TOOLCHAIN_BIN` first on this build
/// script's `PATH`. Does nothing if the variable is not set.
///
/// With Rust's GNU toolchain on Windows, Tauri compiles the application's
/// icon and manifest with a program called `windres` and finds it by name. A
/// computer that has an older MinGW ahead on its `PATH` then picks the wrong
/// one and the build fails. Naming the right directory in the variable, for
/// example in `.cargo/config.toml`, fixes that without changing the system.
/// See `docs/development/windows-without-admin.md`.
#[expect(
    unsafe_code,
    reason = "changing the environment is unsound only while another thread may read it; \
              this runs first, before the build script has started any thread"
)]
fn prefer_configured_toolchain() {
    println!("cargo:rerun-if-env-changed={TOOLCHAIN_BIN_VARIABLE}");
    let Some(toolchain_bin) = env::var_os(TOOLCHAIN_BIN_VARIABLE) else {
        return;
    };

    let mut directories = vec![PathBuf::from(toolchain_bin)];
    if let Some(path) = env::var_os("PATH") {
        directories.extend(env::split_paths(&path));
    }
    let Ok(path) = env::join_paths(directories) else {
        return;
    };

    // SAFETY: see the reason given on the function.
    unsafe {
        env::set_var("PATH", path);
    }
}
