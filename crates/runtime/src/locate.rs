//! Finding the installed Claude Code program.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::environment::Environment;

/// The file names the program can have, in the order they are tried.
///
/// The native installer puts a `claude.exe` on Windows. An installation
/// through npm adds a `claude.cmd` script instead.
const PROGRAM_NAMES: &[&str] = if cfg!(windows) {
    &["claude.exe", "claude.cmd"]
} else {
    &["claude"]
};

/// Searches for the Claude Code program.
///
/// The folders on the search path come first, as in a terminal. After them
/// come the places the installers document, because a program started from
/// the desktop often has a shorter search path than a terminal: on this
/// project's own development machine Claude Code is not on the search path
/// at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locator {
    search_path: Option<OsString>,
    home: Option<PathBuf>,
}

impl Locator {
    /// A locator for the given search path (the value of `PATH`) and home
    /// folder.
    #[must_use]
    pub fn new(search_path: Option<OsString>, home: Option<PathBuf>) -> Self {
        Self { search_path, home }
    }

    /// A locator that uses the search path and home folder of an
    /// environment.
    #[must_use]
    pub fn for_environment(environment: &Environment) -> Self {
        let home_variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        Self::new(
            environment.get("PATH").map(ToOwned::to_owned),
            environment
                .get(home_variable)
                .filter(|home| !home.is_empty())
                .map(PathBuf::from),
        )
    }

    /// Every place that is looked at, in order.
    #[must_use]
    pub fn candidates(&self) -> Vec<PathBuf> {
        let mut folders: Vec<PathBuf> = self
            .search_path
            .as_deref()
            .map(|path| std::env::split_paths(path).collect())
            .unwrap_or_default();
        if let Some(home) = &self.home {
            folders.push(home.join(".local").join("bin"));
            folders.push(home.join(".claude").join("local"));
        }
        if !cfg!(windows) {
            // A program started from the macOS dock does not see the search
            // path the user's shell sets up.
            folders.push(PathBuf::from("/opt/homebrew/bin"));
            folders.push(PathBuf::from("/usr/local/bin"));
        }

        let mut candidates = Vec::new();
        for folder in folders
            .iter()
            .filter(|folder| !folder.as_os_str().is_empty())
        {
            for name in PROGRAM_NAMES {
                let candidate = folder.join(name);
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
        candidates
    }

    /// The first place where the program is, if any.
    #[must_use]
    pub fn find(&self) -> Option<PathBuf> {
        self.candidates()
            .into_iter()
            .find(|candidate| is_program(candidate))
    }
}

fn is_program(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_name() -> &'static str {
        PROGRAM_NAMES[0]
    }

    fn place_program(folder: &Path) -> PathBuf {
        let program = folder.join(program_name());
        std::fs::write(&program, b"").unwrap();
        program
    }

    #[test]
    fn the_search_path_is_tried_before_the_documented_places() {
        let search = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let local_bin = home.path().join(".local").join("bin");
        std::fs::create_dir_all(&local_bin).unwrap();
        place_program(&local_bin);
        let on_path = place_program(search.path());

        let locator = Locator::new(
            Some(search.path().as_os_str().to_owned()),
            Some(home.path().to_owned()),
        );
        assert_eq!(locator.find(), Some(on_path));
    }

    #[test]
    fn the_program_is_found_in_the_home_folder_when_the_search_path_lacks_it() {
        let search = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let local_bin = home.path().join(".local").join("bin");
        std::fs::create_dir_all(&local_bin).unwrap();
        let installed = place_program(&local_bin);

        let locator = Locator::new(
            Some(search.path().as_os_str().to_owned()),
            Some(home.path().to_owned()),
        );
        assert_eq!(locator.find(), Some(installed));
    }

    #[test]
    fn every_folder_of_the_search_path_is_tried() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let installed = place_program(second.path());
        let search_path = std::env::join_paths([first.path(), second.path()]).unwrap();

        let locator = Locator::new(Some(search_path), None);
        assert_eq!(locator.find(), Some(installed));
    }

    #[test]
    fn a_folder_with_the_programs_name_is_not_the_program() {
        let search = tempfile::tempdir().unwrap();
        std::fs::create_dir(search.path().join(program_name())).unwrap();
        let locator = Locator::new(Some(search.path().as_os_str().to_owned()), None);
        assert_eq!(locator.find(), None);
    }

    #[test]
    fn nothing_is_found_where_nothing_is_installed() {
        let empty = tempfile::tempdir().unwrap();
        let locator = Locator::new(
            Some(empty.path().as_os_str().to_owned()),
            Some(empty.path().to_owned()),
        );
        assert_eq!(locator.find(), None);
        assert!(
            locator
                .candidates()
                .contains(&empty.path().join(".local").join("bin").join(program_name()))
        );
    }

    #[test]
    fn the_search_path_and_home_come_from_the_environment() {
        let home_variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let environment = Environment::empty()
            .with("PATH", "first")
            .with(home_variable, "home");
        let locator = Locator::for_environment(&environment);
        let candidates = locator.candidates();
        assert_eq!(candidates[0], Path::new("first").join(program_name()));
        assert!(
            candidates.contains(
                &Path::new("home")
                    .join(".local")
                    .join("bin")
                    .join(program_name())
            )
        );
    }
}
