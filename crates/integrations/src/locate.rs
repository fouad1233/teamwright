//! Finds `gh`: the program the manager named, then the search path, then
//! the places it is usually installed. A program started from the desktop
//! does not always see the search path of the manager's shell, and the
//! installer of `gh` for one user puts it in a folder of its own.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The names the program has.
const PROGRAM_NAMES: &[&str] = if cfg!(windows) { &["gh.exe"] } else { &["gh"] };

/// Where `gh` is looked for.
#[derive(Debug, Clone, Default)]
pub struct GhLocator {
    search_path: Option<OsString>,
    usual_places: Vec<PathBuf>,
}

impl GhLocator {
    /// A locator for a search path (the value of `PATH`) and the folders
    /// where the program is usually installed.
    #[must_use]
    pub const fn new(search_path: Option<OsString>, usual_places: Vec<PathBuf>) -> Self {
        Self {
            search_path,
            usual_places,
        }
    }

    /// A locator for the environment of this program.
    #[must_use]
    pub fn of_this_computer() -> Self {
        let variable = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
        let mut usual_places = Vec::new();
        if cfg!(windows) {
            if let Some(local) = variable("LOCALAPPDATA") {
                usual_places.push(PathBuf::from(local).join("Programs").join("gh").join("bin"));
            }
            for programs in ["ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(folder) = variable(programs) {
                    usual_places.push(PathBuf::from(folder).join("GitHub CLI"));
                }
            }
            if let Some(home) = variable("USERPROFILE") {
                usual_places.push(PathBuf::from(home).join("scoop").join("shims"));
            }
        } else {
            usual_places.push(PathBuf::from("/opt/homebrew/bin"));
            usual_places.push(PathBuf::from("/usr/local/bin"));
            usual_places.push(PathBuf::from("/usr/bin"));
            if let Some(home) = variable("HOME") {
                usual_places.push(PathBuf::from(home).join(".local").join("bin"));
            }
        }
        Self::new(variable("PATH"), usual_places)
    }

    /// The program: the one named, if it is a file; else the first found.
    #[must_use]
    pub fn find(&self, named: Option<&Path>) -> Option<PathBuf> {
        if let Some(named) = named {
            return named.is_file().then(|| named.to_path_buf());
        }
        let mut folders: Vec<PathBuf> = self
            .search_path
            .as_deref()
            .map(|path| std::env::split_paths(path).collect())
            .unwrap_or_default();
        folders.extend(self.usual_places.iter().cloned());
        folders
            .iter()
            .filter(|folder| !folder.as_os_str().is_empty())
            .flat_map(|folder| PROGRAM_NAMES.iter().map(move |name| folder.join(name)))
            .find(|candidate| candidate.is_file())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place_program(folder: &Path) -> PathBuf {
        let program = folder.join(PROGRAM_NAMES[0]);
        std::fs::write(&program, b"").unwrap();
        program
    }

    #[test]
    fn the_search_path_is_tried_before_the_usual_places() {
        let on_path = tempfile::tempdir().unwrap();
        let usual = tempfile::tempdir().unwrap();
        let expected = place_program(on_path.path());
        place_program(usual.path());
        let locator = GhLocator::new(
            Some(on_path.path().as_os_str().to_owned()),
            vec![usual.path().to_path_buf()],
        );
        assert_eq!(locator.find(None), Some(expected));
    }

    #[test]
    fn the_usual_places_are_tried_when_the_search_path_lacks_it() {
        let usual = tempfile::tempdir().unwrap();
        let expected = place_program(usual.path());
        let locator = GhLocator::new(None, vec![usual.path().to_path_buf()]);
        assert_eq!(locator.find(None), Some(expected));
    }

    #[test]
    fn a_named_program_is_used_only_if_it_is_there() {
        let folder = tempfile::tempdir().unwrap();
        let named = place_program(folder.path());
        let locator = GhLocator::new(None, Vec::new());
        assert_eq!(locator.find(Some(&named)), Some(named.clone()));
        assert_eq!(locator.find(Some(&folder.path().join("missing.exe"))), None);
        assert_eq!(locator.find(None), None);
    }
}
