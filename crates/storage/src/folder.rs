//! The files in the sync folder
//! ([decision 0017](../../../docs/decisions/0017-sync-through-a-folder.md),
//! section 1).
//!
//! ```text
//! <chosen folder>/
//!   devices/
//!     <identity of a computer>/
//!       computer.json
//!       log/
//!         000000000001-000000004211.jsonl
//!         000000004212-000000004390.jsonl
//! ```
//!
//! A computer writes only below its own folder. A log file is written under
//! a temporary name that starts with a dot, flushed to the disk, and then
//! renamed; it never changes afterwards. Its first line is a header, its
//! last line a footer with the number of operations and the number of
//! bytes before it, and every line in between is one operation. A reader
//! takes only names of the exact pattern, and only a file whose footer
//! matches what it read: a file a sync program is still copying, a
//! half-copied file, a "conflicted copy" and a file of another program are
//! passed over and looked at again next time.
//!
//! This module knows nothing of SQLite.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use teamwright_application::{
    ComputerCard, ExchangedChange, ExchangedOperation, FileContent, FoundComputer, SYNC_FORMAT,
    StorageError, SyncFolder,
};
use teamwright_domain::ComputerId;
use uuid::Uuid;

use crate::codec;

/// The folder below the chosen folder that holds one folder per computer.
const DEVICES: &str = "devices";
/// The folder of a computer that holds its log files.
const LOG: &str = "log";
/// The card of a computer.
const CARD: &str = "computer.json";
/// The ending of a log file.
const LOG_ENDING: &str = ".jsonl";
/// The number of digits the numbers in the name of a log file are padded
/// to.
const NUMBER_DIGITS: usize = 12;
/// A file smaller than this counts as small when files are joined.
const SMALL_FILE_BYTES: u64 = 256 * 1024;
/// How many small files a computer keeps before it joins them into one.
const MOST_SMALL_FILES: usize = 64;

/// The files of the computers in a sync folder, on the disk.
#[derive(Debug, Clone, Copy)]
pub struct FolderOfComputers {
    /// How many small files are kept before they are joined.
    most_small_files: usize,
}

impl Default for FolderOfComputers {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderOfComputers {
    /// Creates the folder access with the usual limits: more than 64 small
    /// files are joined into one.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            most_small_files: MOST_SMALL_FILES,
        }
    }

    /// Creates the folder access that joins small files once there are
    /// more than `most_small_files`. Tests use it to join early.
    #[must_use]
    pub const fn joining_after(most_small_files: usize) -> Self {
        Self { most_small_files }
    }
}

/// The first line of a log file.
#[derive(Debug, Serialize, Deserialize)]
struct Header {
    format: u32,
    computer: String,
    first: u64,
    last: u64,
}

/// The last line of a log file.
#[derive(Debug, Serialize, Deserialize)]
struct Footer {
    count: u64,
    bytes: u64,
}

/// One operation, as a line of a log file.
#[derive(Debug, Serialize, Deserialize)]
struct Line {
    sequence: u64,
    stamp: String,
    entity: String,
    id: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field_stamps: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seen: Option<Value>,
}

/// A card as `computer.json` keeps it.
#[derive(Debug, Serialize, Deserialize)]
struct CardFile {
    format: u32,
    identity: String,
    name: String,
    version: String,
    written_at: String,
}

/// Turns a failure of the disk into the error the application knows.
fn failed(what: &str, path: &Path, error: std::io::Error) -> StorageError {
    StorageError::caused_by(format!("{what} {} failed", path.display()), error)
}

/// The folder of a computer.
fn folder_of(root: &Path, computer: ComputerId) -> PathBuf {
    root.join(DEVICES).join(computer.to_string())
}

/// The name of a log file that holds the operations from `first` to `last`.
fn file_name(first: u64, last: u64) -> String {
    format!("{first:0NUMBER_DIGITS$}-{last:0NUMBER_DIGITS$}{LOG_ENDING}")
}

/// The first and the last number of a log file, if its name has the exact
/// pattern.
fn numbers_of(name: &str) -> Option<(u64, u64)> {
    let numbers = name.strip_suffix(LOG_ENDING)?;
    let (first, last) = numbers.split_once('-')?;
    let is_number =
        |text: &str| text.len() == NUMBER_DIGITS && text.bytes().all(|byte| byte.is_ascii_digit());
    if !is_number(first) || !is_number(last) {
        return None;
    }
    let (first, last) = (first.parse().ok()?, last.parse().ok()?);
    (first <= last).then_some((first, last))
}

/// Writes a file under a temporary name that starts with a dot, flushes it
/// to the disk, and renames it to its name. A reader never sees it half
/// written.
fn write_whole(folder: &Path, name: &str, content: &[u8]) -> Result<(), StorageError> {
    fs::create_dir_all(folder).map_err(|error| failed("creating the folder", folder, error))?;
    let temporary = folder.join(format!(".writing-{}", Uuid::now_v7()));
    let written = File::create(&temporary).and_then(|mut file| {
        file.write_all(content)?;
        file.sync_all()
    });
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(failed("writing", &temporary, error));
    }
    let target = folder.join(name);
    fs::rename(&temporary, &target).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        failed("renaming to", &target, error)
    })
}

/// Turns an operation into a line of a log file.
fn line_of(operation: &ExchangedOperation) -> Result<String, StorageError> {
    let parse = |text: &str| {
        serde_json::from_str::<Value>(text)
            .map_err(|error| StorageError::caused_by("an operation cannot be written", error))
    };
    let mut line = Line {
        sequence: operation.sequence,
        stamp: operation.stamp.clone(),
        entity: operation.entity.clone(),
        id: operation.entity_id.clone(),
        kind: String::new(),
        state: None,
        field_stamps: None,
        seen: None,
    };
    match &operation.change {
        ExchangedChange::Upsert {
            state,
            field_stamps,
        } => {
            "upsert".clone_into(&mut line.kind);
            line.state = Some(parse(state)?);
            line.field_stamps = field_stamps.as_deref().map(parse).transpose()?;
        }
        ExchangedChange::Delete { seen } => {
            "delete".clone_into(&mut line.kind);
            line.seen = seen.as_deref().map(parse).transpose()?;
        }
    }
    serde_json::to_string(&line)
        .map_err(|error| StorageError::caused_by("an operation cannot be written", error))
}

/// Reads a line of a log file back into an operation.
fn operation_of(computer: ComputerId, line: Line) -> Option<ExchangedOperation> {
    let change = match line.kind.as_str() {
        "upsert" => ExchangedChange::Upsert {
            state: line.state.filter(Value::is_object)?.to_string(),
            field_stamps: line.field_stamps.map(|stamps| stamps.to_string()),
        },
        "delete" => ExchangedChange::Delete {
            seen: line.seen.map(|seen| seen.to_string()),
        },
        _ => return None,
    };
    Some(ExchangedOperation {
        computer,
        sequence: line.sequence,
        stamp: line.stamp,
        entity: line.entity,
        entity_id: line.id,
        change,
    })
}

/// Reads the content of a log file, which must be whole: a header, the
/// operations it announces, and a footer that matches what was read.
fn content_of(computer: ComputerId, name: &str, text: &str) -> FileContent {
    let Some((first, last)) = numbers_of(name) else {
        return FileContent::NotYet;
    };
    let Some(body) = text.strip_suffix('\n') else {
        return FileContent::NotYet;
    };
    let Some((before_footer, footer)) = body.rsplit_once('\n') else {
        return FileContent::NotYet;
    };
    let mut lines = before_footer.split('\n');
    let Some(Ok(header)) = lines.next().map(serde_json::from_str::<Header>) else {
        return FileContent::NotYet;
    };
    if header.format > SYNC_FORMAT {
        return FileContent::NewerFormat(header.format);
    }
    let Ok(footer) = serde_json::from_str::<Footer>(footer) else {
        return FileContent::NotYet;
    };
    // The bytes before the footer, the newline that ends them included.
    let bytes = u64::try_from(before_footer.len() + 1).unwrap_or(u64::MAX);
    let fits = header.computer == computer.to_string()
        && header.first == first
        && header.last == last
        && footer.bytes == bytes;
    if !fits {
        return FileContent::NotYet;
    }
    let mut operations = Vec::new();
    for line in lines {
        let Ok(line) = serde_json::from_str::<Line>(line) else {
            return FileContent::NotYet;
        };
        let Some(operation) = operation_of(computer, line) else {
            return FileContent::NotYet;
        };
        if operation.sequence < first || operation.sequence > last {
            return FileContent::NotYet;
        }
        operations.push(operation);
    }
    if u64::try_from(operations.len()).ok() != Some(footer.count) {
        return FileContent::NotYet;
    }
    FileContent::Complete(operations)
}

impl FolderOfComputers {
    /// The log files of a computer that have the exact pattern, with their
    /// numbers and sizes, in order.
    fn log_files(
        root: &Path,
        computer: ComputerId,
    ) -> Result<Vec<(String, u64, u64, u64)>, StorageError> {
        let folder = folder_of(root, computer).join(LOG);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(failed("reading the folder", &folder, error)),
        };
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| failed("reading the folder", &folder, error))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some((first, last)) = numbers_of(&name) else {
                continue;
            };
            let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
            files.push((name, first, last, size));
        }
        files.sort_by_key(|(_, first, last, _)| (*first, *last));
        Ok(files)
    }
}

impl SyncFolder for FolderOfComputers {
    fn write_card(&self, root: &Path, card: &ComputerCard) -> Result<(), StorageError> {
        let content = serde_json::to_vec_pretty(&CardFile {
            format: card.format,
            identity: card.id.to_string(),
            name: card.name.clone(),
            version: card.version.clone(),
            written_at: codec::moment_to_text(card.written_at),
        })
        .map_err(|error| StorageError::caused_by("the card cannot be written", error))?;
        write_whole(&folder_of(root, card.id), CARD, &content)
    }

    fn write_file(
        &self,
        root: &Path,
        computer: ComputerId,
        operations: &[ExchangedOperation],
    ) -> Result<String, StorageError> {
        let (Some(first), Some(last)) = (operations.first(), operations.last()) else {
            return Err(StorageError::new("a file of no operations is not written"));
        };
        let header = Header {
            format: SYNC_FORMAT,
            computer: computer.to_string(),
            first: first.sequence,
            last: last.sequence,
        };
        let mut text = serde_json::to_string(&header)
            .map_err(|error| StorageError::caused_by("a header cannot be written", error))?;
        text.push('\n');
        for operation in operations {
            text.push_str(&line_of(operation)?);
            text.push('\n');
        }
        let footer = Footer {
            count: u64::try_from(operations.len()).unwrap_or(u64::MAX),
            bytes: u64::try_from(text.len()).unwrap_or(u64::MAX),
        };
        let footer = serde_json::to_string(&footer)
            .map_err(|error| StorageError::caused_by("a footer cannot be written", error))?;
        text.push_str(&footer);
        text.push('\n');
        let name = file_name(header.first, header.last);
        write_whole(&folder_of(root, computer).join(LOG), &name, text.as_bytes())?;
        Ok(name)
    }

    fn computers(&self, root: &Path) -> Result<Vec<FoundComputer>, StorageError> {
        let folder = root.join(DEVICES);
        let entries = match fs::read_dir(&folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(failed("reading the folder", &folder, error)),
        };
        let mut found = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| failed("reading the folder", &folder, error))?;
            if !entry.path().is_dir() {
                continue;
            }
            let Ok(id) = entry.file_name().to_string_lossy().parse::<ComputerId>() else {
                continue;
            };
            let card = fs::read(entry.path().join(CARD))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<CardFile>(&bytes).ok())
                .and_then(|card| {
                    Some(ComputerCard {
                        id: card
                            .identity
                            .parse()
                            .ok()
                            .filter(|identity| *identity == id)?,
                        name: card.name,
                        version: card.version,
                        format: card.format,
                        written_at: codec::moment_from_text(&card.written_at).ok()?,
                    })
                });
            found.push(FoundComputer { id, card });
        }
        found.sort_by_key(|computer| computer.id);
        Ok(found)
    }

    fn files_of(
        &self,
        root: &Path,
        computer: ComputerId,
        after: u64,
    ) -> Result<Vec<String>, StorageError> {
        Ok(Self::log_files(root, computer)?
            .into_iter()
            .filter(|(_, _, last, _)| *last > after)
            .map(|(name, _, _, _)| name)
            .collect())
    }

    fn read_file(
        &self,
        root: &Path,
        computer: ComputerId,
        name: &str,
    ) -> Result<FileContent, StorageError> {
        let path = folder_of(root, computer).join(LOG).join(name);
        match fs::read(&path) {
            Ok(bytes) => Ok(
                String::from_utf8(bytes).map_or(FileContent::NotYet, |text| {
                    content_of(computer, name, &text)
                }),
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(FileContent::NotYet),
            Err(error) => Err(failed("reading", &path, error)),
        }
    }

    fn join_small_files(
        &self,
        root: &Path,
        computer: ComputerId,
    ) -> Result<Option<String>, StorageError> {
        let files = Self::log_files(root, computer)?;
        let small = files
            .iter()
            .filter(|(_, _, _, size)| *size < SMALL_FILE_BYTES)
            .count();
        if small <= self.most_small_files {
            return Ok(None);
        }
        // Every operation once, by its number. A file that is not whole is
        // not joined now; the next round tries again.
        let mut operations = BTreeMap::new();
        for (name, _, _, _) in &files {
            match self.read_file(root, computer, name)? {
                FileContent::Complete(read) => {
                    for operation in read {
                        operations.insert(operation.sequence, operation);
                    }
                }
                FileContent::NotYet | FileContent::NewerFormat(_) => return Ok(None),
            }
        }
        let operations: Vec<_> = operations.into_values().collect();
        // The numbers must follow each other, or the joined file would
        // claim operations it does not have.
        let follow = operations
            .windows(2)
            .all(|pair| pair[1].sequence == pair[0].sequence + 1);
        if !follow {
            return Ok(None);
        }
        let joined = self.write_file(root, computer, &operations)?;
        let folder = folder_of(root, computer).join(LOG);
        for (name, _, _, _) in files {
            if name != joined {
                let path = folder.join(&name);
                fs::remove_file(&path).map_err(|error| failed("removing", &path, error))?;
            }
        }
        Ok(Some(joined))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_of_the_exact_pattern_gives_its_numbers() {
        assert_eq!(numbers_of(&file_name(1, 4211)), Some((1, 4211)));
        assert_eq!(
            numbers_of("000000000001-000000004211.jsonl"),
            Some((1, 4211))
        );
        for passed_over in [
            ".writing-0196f3a2",
            "000000000001-000000004211 (conflicted copy).jsonl",
            "000000000001-000000004211.json",
            "1-4211.jsonl",
            "000000000009-000000000001.jsonl",
            "notes.txt",
        ] {
            assert_eq!(numbers_of(passed_over), None, "{passed_over}");
        }
    }
}
