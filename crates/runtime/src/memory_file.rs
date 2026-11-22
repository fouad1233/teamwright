//! The file through which the team bus reads the memory of a project.
//!
//! The bus is a process of its own, started by Claude Code for one turn,
//! and it never opens the database (decision 0012). To answer
//! `search_memory` it needs the memory of the project, so the runtime
//! writes the current entries to a file of their own when the turn starts
//! and names the file to the bus with `--memory` (decision 0036). The file
//! is removed when the turn is over.
//!
//! The file is JSON: a format version, the mark that frames the entries in
//! the answer of a search (decision 0029, note of 22 November 2026), and
//! the entries, each with its kind, title, text, where it came from and its
//! task in words, and when it was recorded.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use teamwright_application::{MemoryKind, RememberedEntry};
use teamwright_domain::WordsMark;

/// The version of the format of the file.
const FORMAT_VERSION: u32 = 2;

/// Counts the files this program writes, so that two turns that start in
/// the same instant get files of their own.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// The memory of a project in a file of its own, removed when it is
/// dropped.
#[derive(Debug)]
pub struct MemoryFile {
    path: PathBuf,
}

impl MemoryFile {
    /// Writes the entries to a new file in the folder for temporary files
    /// of the system. Only the user who runs Teamwright may read it.
    ///
    /// # Errors
    ///
    /// Returns an error if the file could not be written.
    pub fn write(entries: &[RememberedEntry], mark: &WordsMark) -> io::Result<Self> {
        Self::write_in(&std::env::temp_dir(), entries, mark)
    }

    /// Writes the entries to a new file in `folder`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file could not be written.
    pub fn write_in(
        folder: &Path,
        entries: &[RememberedEntry],
        mark: &WordsMark,
    ) -> io::Result<Self> {
        let contents = serde_json::to_vec(&FileContents {
            version: FORMAT_VERSION,
            mark: mark.as_str().to_owned(),
            entries: entries.iter().map(EntryInFile::of).collect(),
        })
        .map_err(io::Error::other)?;
        let (path, mut file) = new_file(folder)?;
        let written = file.write_all(&contents).and_then(|()| file.sync_all());
        let memory_file = Self { path };
        written?;
        Ok(memory_file)
    }

    /// Where the file is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for MemoryFile {
    fn drop(&mut self) {
        // A file that is gone already is no matter, and a failure here has
        // nobody to be told to: the folder is the system's to clean.
        let _ = fs::remove_file(&self.path);
    }
}

/// Reads the entries and the mark back from a file that [`MemoryFile`]
/// wrote.
///
/// # Errors
///
/// Returns what is wrong, in words, if the file cannot be read or is not
/// in the format.
pub fn entries_from(path: &Path) -> Result<(Vec<RememberedEntry>, WordsMark), String> {
    let bytes = fs::read(path).map_err(|error| format!("it cannot be read: {error}"))?;
    let contents: FileContents =
        serde_json::from_slice(&bytes).map_err(|error| format!("it is not readable: {error}"))?;
    if contents.version != FORMAT_VERSION {
        return Err(format!(
            "its format version {} is not known",
            contents.version
        ));
    }
    let mark = WordsMark::parse(&contents.mark).map_err(|error| error.to_string())?;
    let entries = contents
        .entries
        .into_iter()
        .map(EntryInFile::into_entry)
        .collect::<Result<_, _>>()?;
    Ok((entries, mark))
}

/// Opens a new file in `folder` that nobody else has, with a name of its
/// own.
fn new_file(folder: &Path) -> io::Result<(PathBuf, File)> {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    loop {
        let number = WRITTEN.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!(
            "teamwright-memory-{}-{started}-{number}.json",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

/// What the file holds.
#[derive(Debug, Serialize, Deserialize)]
struct FileContents {
    version: u32,
    mark: String,
    entries: Vec<EntryInFile>,
}

/// One entry as the file holds it.
#[derive(Debug, Serialize, Deserialize)]
struct EntryInFile {
    kind: String,
    title: String,
    text: String,
    source: String,
    task: Option<String>,
    recorded_at: String,
}

impl EntryInFile {
    fn of(entry: &RememberedEntry) -> Self {
        Self {
            kind: entry.kind.to_string(),
            title: entry.title.clone(),
            text: entry.text.clone(),
            source: entry.source.clone(),
            task: entry.task.clone(),
            recorded_at: entry.recorded_at.to_rfc3339(),
        }
    }

    fn into_entry(self) -> Result<RememberedEntry, String> {
        Ok(RememberedEntry {
            kind: self
                .kind
                .parse::<MemoryKind>()
                .map_err(|error| error.to_string())?,
            title: self.title,
            text: self.text,
            source: self.source,
            task: self.task,
            recorded_at: DateTime::parse_from_rfc3339(&self.recorded_at)
                .map_err(|error| error.to_string())?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn mark() -> WordsMark {
        WordsMark::parse("testmark").unwrap()
    }

    fn entries() -> Vec<RememberedEntry> {
        vec![
            RememberedEntry {
                kind: MemoryKind::Fact,
                title: "Display bus".to_owned(),
                text: "The display is on SPI2.\nAt 8 MHz.".to_owned(),
                source: "remembered by Hardware engineer".to_owned(),
                task: Some("SSN-1: Draw the schematic".to_owned()),
                recorded_at: Utc.with_ymd_and_hms(2026, 11, 21, 9, 30, 0).unwrap(),
            },
            RememberedEntry {
                kind: MemoryKind::Decision,
                title: "Power".to_owned(),
                text: "Battery only.".to_owned(),
                source: "written by the manager".to_owned(),
                task: None,
                recorded_at: Utc.with_ymd_and_hms(2026, 11, 20, 17, 0, 0).unwrap(),
            },
        ]
    }

    #[test]
    fn the_entries_are_read_back_as_they_were_written_and_the_file_goes_with_its_owner() {
        let folder = tempfile::tempdir().unwrap();
        let file = MemoryFile::write_in(folder.path(), &entries(), &mark()).unwrap();
        let path = file.path().to_owned();

        assert_eq!(entries_from(&path), Ok((entries(), mark())));
        drop(file);
        assert!(!path.exists());
    }

    #[test]
    fn two_files_written_at_once_do_not_meet() {
        let folder = tempfile::tempdir().unwrap();
        let first = MemoryFile::write_in(folder.path(), &[], &mark()).unwrap();
        let second = MemoryFile::write_in(folder.path(), &entries(), &mark()).unwrap();

        assert_ne!(first.path(), second.path());
        assert_eq!(entries_from(first.path()), Ok((Vec::new(), mark())));
    }

    #[cfg(unix)]
    #[test]
    fn only_its_owner_may_read_the_file() {
        use std::os::unix::fs::PermissionsExt;

        let folder = tempfile::tempdir().unwrap();
        let file = MemoryFile::write_in(folder.path(), &entries(), &mark()).unwrap();

        let mode = fs::metadata(file.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn a_file_that_is_missing_or_not_in_the_format_is_refused_with_words() {
        let folder = tempfile::tempdir().unwrap();
        assert!(
            entries_from(&folder.path().join("missing.json"))
                .unwrap_err()
                .starts_with("it cannot be read")
        );
        let other = folder.path().join("other.json");
        fs::write(&other, r#"{"version":1,"mark":"testmark","entries":[]}"#).unwrap();
        assert_eq!(
            entries_from(&other),
            Err("its format version 1 is not known".to_owned())
        );
        fs::write(&other, r#"{"version":2,"mark":"BAD MARK","entries":[]}"#).unwrap();
        assert!(entries_from(&other).is_err());
        fs::write(&other, "not json").unwrap();
        assert!(
            entries_from(&other)
                .unwrap_err()
                .starts_with("it is not readable")
        );
    }
}
