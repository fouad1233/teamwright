//! What a turn that may run a command needs from this computer: a sandbox
//! that can start, and an environment with nothing secret in it
//! (decision 0038, note of 22 November 2026).
//!
//! The sandbox of Claude Code does not hide environment variables: a
//! sandboxed command inherits the environment Claude Code was started with.
//! So a turn that may run a command starts Claude Code with a cleared
//! environment, a small base set and nothing else. A manager who signs in
//! to Claude Code with an environment variable cannot have that, because
//! the variable would have to stay, and a command could read it; such a
//! computer runs no command.
//!
//! Whether the sandbox works can only be proven by a turn that runs. What
//! is checked here is what can be checked from outside without running
//! Claude Code: on Linux, that bubblewrap and socat are on the search path
//! and that bubblewrap can make a sandbox on this computer; on macOS, that
//! the system's sandbox program is there; and that this is not Windows or
//! the first version of WSL, where Claude Code has no sandbox at all.

use std::path::{Path, PathBuf};
use std::time::Duration;

use teamwright_domain::NoCommands;

use crate::connector_launcher::is_base_variable;
use crate::environment::Environment;
use crate::process::{command, run_briefly};

/// Variables, besides the base set every program is started with
/// ([`is_base_variable`]), that a turn which may run a command keeps: what
/// Claude Code needs to find its own settings and sign-in (`HOME` and the
/// user are in the base set), the shell its commands run in, and what it
/// needs to reach its service through a proxy. Compared without regard to
/// capitals, as proxies are often set in small letters.
const KEPT_FOR_COMMANDS: &[&str] = &[
    "SHELL",
    "CLAUDE_CONFIG_DIR",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
];

/// Variables with which a manager signs in to Claude Code, or sends it to
/// another service, without its own sign-in. With one set, a turn cannot be
/// started with a cleared environment, and a command could read it.
const SIGN_IN_VARIABLES: &[&str] = &[
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "AWS_BEARER_TOKEN_BEDROCK",
];

/// How long bubblewrap may take to make and leave an empty sandbox.
const SANDBOX_TRIAL_LIMIT: Duration = Duration::from_secs(10);

/// The environment a turn that may run a command is started with: only the
/// variables of the base set and those of [`KEPT_FOR_COMMANDS`], taken from
/// `base`. Nothing else of the manager's environment, not even the marks of
/// a running Claude Code session.
#[must_use]
pub fn environment_for_commands(base: &Environment) -> Environment {
    let mut cleared = Environment::empty();
    for (name, value) in base.iter() {
        let Some(text) = name.to_str() else {
            continue;
        };
        let kept = is_base_variable(text)
            || KEPT_FOR_COMMANDS
                .iter()
                .any(|kept| kept.eq_ignore_ascii_case(text));
        if kept {
            cleared.set(name, value);
        }
    }
    cleared
}

/// The name of a variable of `environment` with which the manager signs in
/// to Claude Code, if one is set: any `ANTHROPIC_…` variable, or one of
/// [`SIGN_IN_VARIABLES`].
#[must_use]
pub fn sign_in_variable(environment: &Environment) -> Option<String> {
    environment
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .filter_map(|(name, _)| name.to_str())
        .find(|name| {
            let upper = name.to_ascii_uppercase();
            upper.starts_with("ANTHROPIC_") || SIGN_IN_VARIABLES.contains(&upper.as_str())
        })
        .map(str::to_owned)
}

/// Why this computer can run no command, as far as can be known without
/// running Claude Code, or nothing if it may.
#[must_use]
pub fn why_no_commands(environment: &Environment) -> Option<NoCommands> {
    if sign_in_variable(environment).is_some() {
        return Some(NoCommands::SignedInWithVariable);
    }
    if !sandbox_can_start(environment) {
        return Some(NoCommands::NoSandbox);
    }
    None
}

/// Whether the sandbox of Claude Code can start on this computer.
fn sandbox_can_start(environment: &Environment) -> bool {
    if cfg!(windows) {
        return false;
    }
    if cfg!(target_os = "macos") {
        return Path::new("/usr/bin/sandbox-exec").is_file();
    }
    if cfg!(target_os = "linux") {
        // The first version of WSL has no sandbox; WSL2 is a Linux.
        if let Ok(version) = std::fs::read_to_string("/proc/version")
            && version.contains("Microsoft")
            && !version.contains("WSL2")
        {
            return false;
        }
        let (Some(bubblewrap), Some(_)) = (
            on_search_path(environment, "bwrap"),
            on_search_path(environment, "socat"),
        ) else {
            return false;
        };
        return bubblewrap_works(&bubblewrap, environment);
    }
    false
}

/// Whether bubblewrap can make an empty sandbox with every namespace of its
/// own and leave it again: unprivileged namespaces may be switched off, or
/// held back by the system's own rules.
fn bubblewrap_works(bubblewrap: &Path, environment: &Environment) -> bool {
    let arguments: Vec<std::ffi::OsString> = [
        "--ro-bind",
        "/",
        "/",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--unshare-all",
        "--new-session",
        "--die-with-parent",
        "--",
        "true",
    ]
    .iter()
    .map(Into::into)
    .collect();
    matches!(
        run_briefly(
            command(bubblewrap, &arguments, None, &environment_for_commands(environment)),
            SANDBOX_TRIAL_LIMIT,
            true,
        ),
        Ok(Some(finished)) if finished.code == Some(0)
    )
}

/// The first file with this name in the folders of the search path of
/// `environment`.
fn on_search_path(environment: &Environment, name: &str) -> Option<PathBuf> {
    let path = environment.get("PATH")?;
    std::env::split_paths(path)
        .filter(|folder| !folder.as_os_str().is_empty())
        .map(|folder| folder.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsStr;

    use super::*;

    fn as_map(environment: &Environment) -> BTreeMap<String, String> {
        environment
            .iter()
            .map(|(name, value)| {
                (
                    name.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect()
    }

    fn managers_environment() -> Environment {
        Environment::empty()
            .with("PATH", "/usr/bin:/bin")
            .with("HOME", "/home/manager")
            .with("USER", "manager")
            .with("LOGNAME", "manager")
            .with("LANG", "en_GB.UTF-8")
            .with("LC_ALL", "en_GB.UTF-8")
            .with("TZ", "Europe/London")
            .with("TMPDIR", "/tmp")
            .with("SHELL", "/bin/bash")
            .with("CLAUDE_CONFIG_DIR", "/home/manager/.claude")
            .with("https_proxy", "http://proxy.example.com:3128")
            .with("NO_PROXY", "localhost")
            .with("GH_TOKEN", "secret-gh")
            .with("GITHUB_TOKEN", "secret-github")
            .with("AWS_ACCESS_KEY_ID", "secret-aws-id")
            .with("AWS_SECRET_ACCESS_KEY", "secret-aws")
            .with("OPENAI_API_KEY", "secret-openai")
            .with("SSH_AUTH_SOCK", "/run/user/1000/ssh-agent")
            .with("DBUS_SESSION_BUS_ADDRESS", "unix:path=/run/user/1000/bus")
            .with("XDG_RUNTIME_DIR", "/run/user/1000")
            .with("DOCKER_HOST", "unix:///var/run/docker.sock")
            .with("KUBECONFIG", "/home/manager/.kube/config")
            .with("NPM_TOKEN", "secret-npm")
            .with("CLAUDECODE", "1")
            .with("CLAUDE_CODE_ENTRYPOINT", "cli")
            .with("DATABASE_URL", "postgres://user:secret@db/app")
    }

    #[cfg(unix)]
    #[test]
    fn a_turn_with_a_command_gets_exactly_the_base_set_and_nothing_else() {
        let cleared = environment_for_commands(&managers_environment());
        let expected: BTreeMap<String, String> = [
            ("PATH", "/usr/bin:/bin"),
            ("HOME", "/home/manager"),
            ("USER", "manager"),
            ("LOGNAME", "manager"),
            ("LANG", "en_GB.UTF-8"),
            ("LC_ALL", "en_GB.UTF-8"),
            ("TZ", "Europe/London"),
            ("TMPDIR", "/tmp"),
            ("SHELL", "/bin/bash"),
            ("CLAUDE_CONFIG_DIR", "/home/manager/.claude"),
            ("https_proxy", "http://proxy.example.com:3128"),
            ("NO_PROXY", "localhost"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
        assert_eq!(as_map(&cleared), expected);
    }

    #[test]
    fn no_secret_of_the_manager_reaches_a_turn_with_a_command() {
        let cleared = environment_for_commands(&managers_environment());
        for secret in [
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "AWS_SECRET_ACCESS_KEY",
            "OPENAI_API_KEY",
            "SSH_AUTH_SOCK",
            "DBUS_SESSION_BUS_ADDRESS",
            "DOCKER_HOST",
            "NPM_TOKEN",
            "DATABASE_URL",
            "CLAUDECODE",
        ] {
            assert_eq!(cleared.get(secret), None, "{secret} reached the turn");
        }
    }

    #[test]
    fn signing_in_with_a_variable_is_found_whatever_its_name() {
        assert_eq!(sign_in_variable(&managers_environment()), None);
        for name in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_BASE_URL",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_VERTEX",
        ] {
            let environment = managers_environment().with(name, "value");
            assert_eq!(sign_in_variable(&environment).as_deref(), Some(name));
            assert_eq!(
                why_no_commands(&environment),
                Some(NoCommands::SignedInWithVariable)
            );
        }
        // An empty variable signs nobody in.
        let empty = managers_environment().with("ANTHROPIC_API_KEY", "");
        assert_eq!(sign_in_variable(&empty), None);
    }

    #[test]
    fn without_bubblewrap_and_socat_on_the_search_path_there_is_no_sandbox() {
        let nowhere = Environment::empty().with("PATH", OsStr::new(""));
        assert_eq!(why_no_commands(&nowhere), Some(NoCommands::NoSandbox));
    }

    /// Run on a Linux computer with bubblewrap and socat, where unprivileged
    /// namespaces are allowed: `cargo test -p teamwright-runtime
    /// this_computer_can_make_the_sandbox -- --ignored`.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "needs bubblewrap and socat, and namespaces this computer allows"]
    fn this_computer_can_make_the_sandbox() {
        let environment =
            Environment::empty().with("PATH", std::env::var_os("PATH").unwrap_or_default());
        assert_eq!(why_no_commands(&environment), None);
    }

    #[cfg(windows)]
    #[test]
    fn windows_has_no_sandbox() {
        assert_eq!(
            why_no_commands(&Environment::inherited()),
            Some(NoCommands::NoSandbox)
        );
    }
}
