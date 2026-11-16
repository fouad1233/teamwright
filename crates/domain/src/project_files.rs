//! What lies in the folders of a project, as the manager sees it in a view
//! of the kind `files`.
//!
//! The rules here concern one value: a path inside a folder, a name, a list
//! of entries. Reading the disk is not done here; the use case in
//! `application` does it through an interface.

use std::cmp::Ordering;

use crate::error::DomainError;
use crate::time::Timestamp;

/// The most entries of one folder that are answered. A folder with more is
/// cut, and the answer says so.
pub const MOST_ENTRIES: usize = 2000;

/// A path inside a folder of a project, relative to that folder, kept as
/// its parts. The empty path is the folder itself.
///
/// It never leaves the folder by how it is written: no part is `..`, it does
/// not start at the root of a drive, and no part holds a colon, which on
/// Windows names a drive or a hidden stream of a file. Whether it leaves the
/// folder through a link is a question for the disk, and the use case asks
/// it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PathInFolder {
    parts: Vec<String>,
}

impl PathInFolder {
    /// Reads a path as the interface writes it, with `/` or `\` between the
    /// parts. Empty parts and `.` are left out.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::PathLeavesFolder`] if the path is absolute,
    /// goes up with `..`, names a drive or holds a character no name may
    /// hold.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let refused = || DomainError::PathLeavesFolder {
            path: text.to_owned(),
        };
        if text.starts_with(['/', '\\']) {
            return Err(refused());
        }
        let mut parts = Vec::new();
        for part in text.split(['/', '\\']) {
            match part {
                "" | "." => {}
                // Windows drops the dots and spaces at the end of a part, so
                // `...` and `.. ` would go up as `..` does.
                _ if part.trim_end_matches([' ', '.']).is_empty()
                    || part.contains(':')
                    || part.chars().any(char::is_control) =>
                {
                    return Err(refused());
                }
                _ => parts.push(part.to_owned()),
            }
        }
        Ok(Self { parts })
    }

    /// The parts of the path, the outermost first.
    #[must_use]
    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    /// Whether the path is the folder itself.
    #[must_use]
    pub fn is_folder_itself(&self) -> bool {
        self.parts.is_empty()
    }

    /// The path written with `/` between its parts, as the interface keeps
    /// it.
    #[must_use]
    pub fn as_text(&self) -> String {
        self.parts.join("/")
    }
}

/// Whether a name is left out of a listing: what the tools of a programmer
/// keep for themselves (`.git`, `node_modules`, `target`), and names that
/// begin with a dot unless the manager asks to see hidden names. `.git` is
/// never shown.
#[must_use]
pub fn is_noise(name: &str, shows_hidden: bool) -> bool {
    match name {
        ".git" | "node_modules" | "target" => true,
        _ => name.starts_with('.') && !shows_hidden,
    }
}

/// The endings of the files that the system would run rather than open.
/// Such a file is not opened from Teamwright: a specialist may have written
/// it, and the manager is better served by seeing it in its folder.
const PROGRAM_ENDINGS: [&str; 22] = [
    "exe", "com", "bat", "cmd", "ps1", "psm1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "msi",
    "msp", "scr", "pif", "hta", "cpl", "lnk", "url", "reg", "jar",
];

/// Whether the system would run a file of this name instead of opening it
/// in a program, so that Teamwright does not open it.
#[must_use]
pub fn runs_as_program(name: &str) -> bool {
    // Windows drops the spaces and dots at the end of a name.
    let name = name.trim_end_matches([' ', '.']);
    name.rsplit_once('.').is_some_and(|(stem, ending)| {
        !stem.is_empty()
            && PROGRAM_ENDINGS
                .iter()
                .any(|program| program.eq_ignore_ascii_case(ending))
    })
}

/// What an entry of a folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// A folder, which can be opened in the view.
    Folder,
    /// A file.
    File,
    /// A link that leads outside the folder of the project, or nowhere. It
    /// is listed and not followed.
    Link,
}

/// One entry of a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderEntry {
    /// Its name.
    pub name: String,
    /// What it is.
    pub kind: EntryKind,
    /// The size of a file in bytes; nothing for a folder and a link.
    pub size: Option<u64>,
    /// When it was last changed, if the system says.
    pub changed: Option<Timestamp>,
}

/// The order of a listing: folders first, then by name without regard to
/// capitals, and names that differ only in capitals in a fixed order.
#[must_use]
pub fn listing_order(first: &FolderEntry, second: &FolderEntry) -> Ordering {
    let is_folder = |entry: &FolderEntry| entry.kind == EntryKind::Folder;
    is_folder(second)
        .cmp(&is_folder(first))
        .then_with(|| first.name.to_lowercase().cmp(&second.name.to_lowercase()))
        .then_with(|| first.name.cmp(&second.name))
}

/// The entries of a folder as the manager sees them: noise left out (see
/// [`is_noise`]), in [`listing_order`], at most [`MOST_ENTRIES`]. The flag
/// says whether there were more.
#[must_use]
pub fn listing_of(entries: Vec<FolderEntry>, shows_hidden: bool) -> (Vec<FolderEntry>, bool) {
    let mut shown: Vec<FolderEntry> = entries
        .into_iter()
        .filter(|entry| !is_noise(&entry.name, shows_hidden))
        .collect();
    shown.sort_by(listing_order);
    let has_more = shown.len() > MOST_ENTRIES;
    shown.truncate(MOST_ENTRIES);
    (shown, has_more)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: EntryKind) -> FolderEntry {
        FolderEntry {
            name: name.to_owned(),
            kind,
            size: None,
            changed: None,
        }
    }

    fn names(entries: &[FolderEntry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    #[test]
    fn a_path_is_read_in_parts_with_either_slash() {
        let path = PathInFolder::parse("firmware\\src/./main.c/").unwrap();

        assert_eq!(path.parts(), ["firmware", "src", "main.c"]);
        assert_eq!(path.as_text(), "firmware/src/main.c");
        assert!(!path.is_folder_itself());
    }

    #[test]
    fn the_empty_path_is_the_folder_itself() {
        assert!(PathInFolder::parse("").unwrap().is_folder_itself());
        assert!(PathInFolder::parse("./").unwrap().is_folder_itself());
    }

    #[test]
    fn a_path_that_goes_up_or_starts_elsewhere_is_refused() {
        for text in [
            "..",
            "docs/../..",
            "docs/..",
            "docs/...",
            ".. /secret",
            "/etc/passwd",
            "\\Windows",
            "\\\\server\\share",
            "C:\\Windows",
            "C:notes.txt",
            "notes.txt:hidden",
            "line\nbreak",
        ] {
            let error = PathInFolder::parse(text).unwrap_err();
            assert_eq!(error.code(), "rule_broken", "{text}");
        }
        assert_eq!(
            PathInFolder::parse("../secret").unwrap_err().to_string(),
            "the path ../secret does not stay inside the folder"
        );
    }

    #[test]
    fn a_name_with_two_dots_inside_it_is_only_a_name() {
        let path = PathInFolder::parse("release..notes/v1..2").unwrap();

        assert_eq!(path.parts(), ["release..notes", "v1..2"]);
    }

    #[test]
    fn the_noise_of_the_tools_is_left_out() {
        for name in [".git", "node_modules", "target"] {
            assert!(is_noise(name, false), "{name}");
            assert!(is_noise(name, true), "{name}");
        }
        assert!(is_noise(".env", false));
        assert!(!is_noise(".env", true));
        assert!(!is_noise("main.c", false));
        assert!(!is_noise("targets", false));
    }

    #[test]
    fn a_program_is_known_by_its_ending() {
        for name in [
            "setup.exe",
            "build.BAT",
            "run.cmd",
            "script.ps1",
            "macro.vbs",
            "tool.js",
            "link.lnk",
            "setup.exe.",
            "setup.exe ",
        ] {
            assert!(runs_as_program(name), "{name}");
        }
        for name in [
            "main.c",
            "schematic.pdf",
            "exe",
            ".bat",
            "notes",
            "board.json",
        ] {
            assert!(!runs_as_program(name), "{name}");
        }
    }

    #[test]
    fn folders_come_first_then_names_without_regard_to_capitals() {
        let entries = vec![
            entry("readme.md", EntryKind::File),
            entry("src", EntryKind::Folder),
            entry("Build", EntryKind::Folder),
            entry("Makefile", EntryKind::File),
            entry("elsewhere", EntryKind::Link),
            entry("makefile", EntryKind::File),
        ];

        let (shown, has_more) = listing_of(entries, false);

        assert_eq!(
            names(&shown),
            [
                "Build",
                "src",
                "elsewhere",
                "Makefile",
                "makefile",
                "readme.md"
            ]
        );
        assert!(!has_more);
    }

    #[test]
    fn a_listing_leaves_out_the_noise_and_shows_hidden_names_when_asked() {
        let entries = || {
            vec![
                entry(".git", EntryKind::Folder),
                entry(".env", EntryKind::File),
                entry("node_modules", EntryKind::Folder),
                entry("main.c", EntryKind::File),
            ]
        };

        assert_eq!(names(&listing_of(entries(), false).0), ["main.c"]);
        assert_eq!(names(&listing_of(entries(), true).0), [".env", "main.c"]);
    }

    #[test]
    fn a_listing_has_at_most_two_thousand_entries_and_says_when_there_were_more() {
        let entries = |count: usize| {
            (0..count)
                .map(|number| entry(&format!("file {number:05}"), EntryKind::File))
                .collect::<Vec<_>>()
        };

        let (exactly, has_more) = listing_of(entries(MOST_ENTRIES), false);
        assert_eq!(exactly.len(), MOST_ENTRIES);
        assert!(!has_more);

        let (cut, has_more) = listing_of(entries(MOST_ENTRIES + 1), false);
        assert_eq!(cut.len(), MOST_ENTRIES);
        assert!(has_more);
        assert_eq!(cut[0].name, "file 00000");
    }
}
