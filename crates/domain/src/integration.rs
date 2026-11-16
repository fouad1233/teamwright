//! The values the integrations of a project are made of: the kinds of
//! events that can be passed on, a web address to send them to, and the
//! name of a GitHub repository.
//!
//! An integration is set on one computer for one project
//! ([decision 0027](../../../docs/decisions/0027-integrations.md)). The
//! values here only know what makes each of them well formed.

use std::collections::BTreeSet;

use crate::error::DomainError;
use crate::macros::text_enum;

/// The kinds of events of a project that can be passed on to a chat
/// channel or a web address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKind {
    /// A task was opened.
    TaskCreated,
    /// A task moved to another status.
    TaskStatusChanged,
    /// A specialist finished its work on a task.
    TaskFinished,
    /// A specialist or the Advisor waits for the manager.
    WaitsForManager,
    /// A specialist or the Advisor could not go on.
    AgentStopped,
    /// A sprint was started.
    SprintStarted,
    /// A sprint was completed.
    SprintCompleted,
}

text_enum!(EventKind, kind = "kind of event", {
    TaskCreated => "task.created",
    TaskStatusChanged => "task.status_changed",
    TaskFinished => "task.finished",
    WaitsForManager => "agent.waiting",
    AgentStopped => "agent.stopped",
    SprintStarted => "sprint.started",
    SprintCompleted => "sprint.completed",
});

/// A choice of kinds of events.
pub type EventKinds = BTreeSet<EventKind>;

/// The longest web address the program keeps.
pub const MOST_ADDRESS_CHARACTERS: usize = 2000;

/// The most web addresses a project sends its events to.
pub const MOST_WEB_ADDRESSES: usize = 5;

/// An address on the web that events are sent to: always `https`, so that
/// nothing about a project crosses the network unencrypted.
///
/// The address is a secret (a Slack address lets anyone who has it write to
/// the channel), so its `Debug` form does not show it.
#[derive(Clone, PartialEq, Eq)]
pub struct WebAddress(String);

impl WebAddress {
    /// Reads an address. `field` says what it is for, in the words of an
    /// error.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidWebAddress`] if it does not start with
    /// `https://`, names no host, holds a space or is too long.
    pub fn parse(text: &str, field: &'static str) -> Result<Self, DomainError> {
        let text = text.trim();
        let invalid = DomainError::InvalidWebAddress { field };
        let Some(rest) = text.strip_prefix("https://") else {
            return Err(invalid);
        };
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let is_well_formed = !host.is_empty()
            && !host.starts_with('.')
            && !host.starts_with(':')
            && !text.chars().any(char::is_whitespace)
            && text.chars().count() <= MOST_ADDRESS_CHARACTERS;
        if is_well_formed {
            Ok(Self(text.to_owned()))
        } else {
            Err(invalid)
        }
    }

    /// The address.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The address as the manager may see it again without it being given
    /// away: the host, and the rest hidden.
    #[must_use]
    pub fn shown(&self) -> String {
        let rest = self.0.trim_start_matches("https://");
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if host.len() == rest.len() {
            format!("https://{host}")
        } else {
            format!("https://{host}/…")
        }
    }
}

impl std::fmt::Debug for WebAddress {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "WebAddress({})", self.shown())
    }
}

/// The name of a repository on GitHub, `owner/name`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryName {
    owner: String,
    name: String,
}

impl RepositoryName {
    /// Reads a name of the form `owner/name`.
    ///
    /// # Errors
    ///
    /// [`DomainError::InvalidRepositoryName`] if it is of another form.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let mut parts = text.trim().split('/');
        let (Some(owner), Some(name), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(DomainError::InvalidRepositoryName);
        };
        let is_part = |part: &str| {
            !part.is_empty()
                && part.len() <= 100
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        if is_part(owner) && is_part(name) {
            Ok(Self {
                owner: owner.to_owned(),
                name: name.to_owned(),
            })
        } else {
            Err(DomainError::InvalidRepositoryName)
        }
    }

    /// The repository a git remote points at, if it is one on GitHub:
    /// `https://github.com/owner/name(.git)` or `git@github.com:owner/name(.git)`.
    #[must_use]
    pub fn of_remote(remote: &str) -> Option<Self> {
        let remote = remote.trim();
        let path = remote
            .strip_prefix("https://github.com/")
            .or_else(|| remote.strip_prefix("http://github.com/"))
            .or_else(|| remote.strip_prefix("git@github.com:"))
            .or_else(|| remote.strip_prefix("ssh://git@github.com/"))?;
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        Self::parse(path).ok()
    }

    /// The owner.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// The name, without the owner.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for RepositoryName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}/{}", self.owner, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_web_address_is_https_with_a_host() {
        let address = WebAddress::parse(" https://hooks.example.com/T1/B2/x ", "it").unwrap();
        assert_eq!(address.as_str(), "https://hooks.example.com/T1/B2/x");
        assert_eq!(address.shown(), "https://hooks.example.com/…");
        assert_eq!(
            WebAddress::parse("https://example.com", "it")
                .unwrap()
                .shown(),
            "https://example.com"
        );
        for wrong in [
            "http://example.com/x",
            "https://",
            "https:///x",
            "https://exa mple.com",
            "ftp://example.com",
            "",
        ] {
            assert_eq!(
                WebAddress::parse(wrong, "the address"),
                Err(DomainError::InvalidWebAddress {
                    field: "the address"
                }),
                "{wrong}"
            );
        }
    }

    #[test]
    fn the_debug_form_of_an_address_gives_nothing_away() {
        let address = WebAddress::parse("https://hooks.slack.com/services/SECRET", "it").unwrap();
        assert!(!format!("{address:?}").contains("SECRET"));
    }

    #[test]
    fn a_repository_is_owner_and_name() {
        let name = RepositoryName::parse("fouad1233/teamwright").unwrap();
        assert_eq!(name.owner(), "fouad1233");
        assert_eq!(name.name(), "teamwright");
        assert_eq!(name.to_string(), "fouad1233/teamwright");
        for wrong in ["teamwright", "a/b/c", "/b", "a/", "a b/c", "../x", "a/.."] {
            assert_eq!(
                RepositoryName::parse(wrong),
                Err(DomainError::InvalidRepositoryName),
                "{wrong}"
            );
        }
    }

    #[test]
    fn a_repository_is_read_from_the_remotes_of_github_only() {
        let expected = RepositoryName::parse("octo-org/sensor-node").ok();
        for remote in [
            "https://github.com/octo-org/sensor-node.git",
            "https://github.com/octo-org/sensor-node",
            "git@github.com:octo-org/sensor-node.git",
            "ssh://git@github.com/octo-org/sensor-node.git\n",
        ] {
            assert_eq!(RepositoryName::of_remote(remote), expected, "{remote}");
        }
        assert_eq!(
            RepositoryName::of_remote("https://gitlab.com/octo-org/sensor-node.git"),
            None
        );
    }

    #[test]
    fn the_kinds_of_events_have_stable_names() {
        assert_eq!(EventKind::TaskStatusChanged.as_str(), "task.status_changed");
        assert_eq!(
            "agent.waiting".parse::<EventKind>(),
            Ok(EventKind::WaitsForManager)
        );
        assert_eq!(EventKind::ALL.len(), 7);
    }
}
