//! The environment variables Claude Code is started with.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};

/// Variables that a running Claude Code session sets for the programs it
/// starts. If Teamwright itself was started from inside a session, for
/// example during development, passing them on would make Claude Code
/// believe it runs inside that session, and it refuses to nest.
const SESSION_MARKERS: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_HOST_SESSION_ID",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_PID",
    "CLAUDE_AGENT_SDK_VERSION",
];

/// The complete set of environment variables a program is started with.
///
/// Nothing else is passed on: the program does not see the variables of
/// Teamwright's own process unless they are in this set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    variables: BTreeMap<OsString, OsString>,
}

impl Environment {
    /// An environment without any variable.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// The environment of the current process, without the variables by
    /// which a running Claude Code session marks the programs it starts.
    ///
    /// This is what Teamwright starts Claude Code with, so that the user's
    /// own settings, such as a proxy or the path to Git Bash, still apply.
    #[must_use]
    pub fn inherited() -> Self {
        let mut environment = Self {
            variables: std::env::vars_os().collect(),
        };
        for name in SESSION_MARKERS {
            environment.remove(name);
        }
        environment
    }

    /// The environment of the current process without any variable whose
    /// name starts with `CLAUDE` or `ANTHROPIC`, whatever the capitals.
    ///
    /// Tests and tools use it to run Claude Code the way a freshly started
    /// program would, unaffected by the session they may run in.
    #[must_use]
    pub fn inherited_without_claude_variables() -> Self {
        let variables = std::env::vars_os()
            .filter(|(name, _)| {
                let name = name.to_string_lossy().to_ascii_uppercase();
                !(name.starts_with("CLAUDE") || name.starts_with("ANTHROPIC"))
            })
            .collect();
        Self { variables }
    }

    /// Sets a variable and returns the environment, for building one in a
    /// single expression.
    #[must_use]
    pub fn with(mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.set(name, value);
        self
    }

    /// Sets a variable, replacing an earlier value.
    pub fn set(&mut self, name: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        self.remove(name.as_ref());
        self.variables
            .insert(name.as_ref().to_owned(), value.as_ref().to_owned());
    }

    /// Removes a variable. On Windows the name is compared without regard to
    /// capitals, as Windows does.
    pub fn remove(&mut self, name: impl AsRef<OsStr>) {
        let name = name.as_ref();
        self.variables
            .retain(|existing, _| !same_name(existing.as_os_str(), name));
    }

    /// The value of a variable, if it is set.
    #[must_use]
    pub fn get(&self, name: impl AsRef<OsStr>) -> Option<&OsStr> {
        let name = name.as_ref();
        self.variables
            .iter()
            .find(|(existing, _)| same_name(existing.as_os_str(), name))
            .map(|(_, value)| value.as_os_str())
    }

    /// All variables, by name.
    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.variables
            .iter()
            .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
    }
}

fn same_name(first: &OsStr, second: &OsStr) -> bool {
    if cfg!(windows) {
        first.to_string_lossy().to_ascii_uppercase()
            == second.to_string_lossy().to_ascii_uppercase()
    } else {
        first == second
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variable_can_be_set_read_and_removed() {
        let mut environment = Environment::empty().with("NAME", "first");
        environment.set("NAME", "second");
        assert_eq!(environment.get("NAME"), Some(OsStr::new("second")));
        assert_eq!(environment.iter().count(), 1);
        environment.remove("NAME");
        assert_eq!(environment.get("NAME"), None);
    }

    #[test]
    fn the_inherited_environment_has_no_session_markers() {
        let environment = Environment::inherited();
        for name in SESSION_MARKERS {
            assert_eq!(environment.get(name), None);
        }
    }

    #[test]
    fn the_clean_environment_has_no_claude_variables() {
        let environment = Environment::inherited_without_claude_variables();
        assert!(environment.iter().all(|(name, _)| {
            let name = name.to_string_lossy().to_ascii_uppercase();
            !name.starts_with("CLAUDE") && !name.starts_with("ANTHROPIC")
        }));
    }

    #[cfg(windows)]
    #[test]
    fn names_are_compared_without_capitals_on_windows() {
        let environment = Environment::empty().with("Path", "here");
        assert_eq!(environment.get("PATH"), Some(OsStr::new("here")));
    }
}
