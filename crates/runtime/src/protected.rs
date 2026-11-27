//! The places of the user's home folder that no specialist may read or
//! change, whatever the manager allowed (decision 0044): the settings of
//! Claude Code, which hold the user's own skills, hooks, connectors and
//! sign-in, and the keys of the system.
//!
//! Teamwright's own data and the sync folder are added by the program,
//! which knows where they are.

use std::path::{Path, PathBuf};

use teamwright_application::{ProtectedKind, ProtectedPlace};

use crate::environment::Environment;

/// The places in the home folder `home` that are protected, with the
/// folder that `CLAUDE_CONFIG_DIR` names in `environment`, if it names one:
/// Claude Code then keeps its settings there instead of in `~/.claude`.
///
/// - `~/.claude` and `~/.claude.json`: the settings of Claude Code, the
///   user's own skills, hooks, agents and tool servers, the transcripts of
///   earlier sessions, and on Linux the sign-in itself
///   (`.credentials.json`).
/// - `~/.ssh`: the keys of SSH.
/// - `~/.gnupg`, and on Windows `%APPDATA%\gnupg`: the keys of GnuPG.
///
/// A place that does not exist is protected all the same, so that a
/// session cannot create it.
#[must_use]
pub fn protected_places_in_home(home: &Path, environment: &Environment) -> Vec<ProtectedPlace> {
    let mut places = vec![
        ProtectedPlace::folder(home.join(".claude"), ProtectedKind::AgentProgramSettings),
        ProtectedPlace::file(
            home.join(".claude.json"),
            ProtectedKind::AgentProgramSettings,
        ),
    ];
    if let Some(folder) = environment
        .get("CLAUDE_CONFIG_DIR")
        .filter(|folder| !folder.is_empty())
    {
        let folder = PathBuf::from(folder);
        if folder.is_absolute() && folder != home.join(".claude") {
            places.push(ProtectedPlace::folder(
                folder,
                ProtectedKind::AgentProgramSettings,
            ));
        }
    }
    places.push(ProtectedPlace::folder(
        home.join(".ssh"),
        ProtectedKind::Keys,
    ));
    places.push(ProtectedPlace::folder(
        home.join(".gnupg"),
        ProtectedKind::Keys,
    ));
    if cfg!(windows)
        && let Some(roaming) = environment
            .get("APPDATA")
            .filter(|folder| !folder.is_empty())
    {
        places.push(ProtectedPlace::folder(
            PathBuf::from(roaming).join("gnupg"),
            ProtectedKind::Keys,
        ));
    }
    places
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_of_claude_code_and_the_keys_are_protected() {
        let home = Path::new("/home/sample");
        // Without APPDATA, as on Linux and macOS, also when this runs on
        // Windows.
        let places = protected_places_in_home(home, &Environment::empty());

        assert_eq!(
            places,
            [
                ProtectedPlace::folder(home.join(".claude"), ProtectedKind::AgentProgramSettings),
                ProtectedPlace::file(
                    home.join(".claude.json"),
                    ProtectedKind::AgentProgramSettings
                ),
                ProtectedPlace::folder(home.join(".ssh"), ProtectedKind::Keys),
                ProtectedPlace::folder(home.join(".gnupg"), ProtectedKind::Keys),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_of_settings_named_by_the_environment_is_protected_too() {
        let home = Path::new("/home/sample");
        let elsewhere = Environment::empty().with("CLAUDE_CONFIG_DIR", "/srv/claude-settings");
        let places = protected_places_in_home(home, &elsewhere);
        assert!(
            places.iter().any(|place| {
                place.path == Path::new("/srv/claude-settings") && place.is_folder
            })
        );

        // A relative or empty one names no place.
        for value in ["", "settings"] {
            let places = protected_places_in_home(
                home,
                &Environment::empty().with("CLAUDE_CONFIG_DIR", value),
            );
            assert_eq!(places.len(), 4, "{value:?}");
        }
    }
}
