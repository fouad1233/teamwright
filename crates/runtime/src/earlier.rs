//! Reads the conversations the user held with Claude Code before, from
//! where Claude Code keeps them on this computer
//! ([decision 0041](../../../docs/decisions/0041-earlier-conversations-as-digests.md)).
//!
//! Claude Code keeps every conversation as a file of its own,
//! `<projects folder>/<folder name>/<session>.jsonl`, with one JSON object
//! on each line. The projects folder is `projects` in the folder of Claude
//! Code's settings: `CLAUDE_CONFIG_DIR` if it is set, otherwise `.claude` in
//! the user's home. The folder name is made from the path of the folder
//! the conversation was held in; the path itself is in the field `cwd` of
//! most lines.
//!
//! This is not a published format. It was learned from the files of Claude
//! Code 2.1.222 to 2.1.328 and changes between versions, so the reader is
//! tolerant: it reads the few things it needs from the lines it
//! recognises, skips every line it does not understand, and never fails a
//! conversation over one odd line. It only reads: nothing there is ever
//! changed, copied or removed.
//!
//! What is read is what the user typed (not what tools answered, not what
//! Claude Code added by itself, not what helpers it started said), the
//! files its tools wrote or changed, its last answer in words, the moments
//! of the first and the last line, and the title it or the user gave the
//! conversation.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;
use teamwright_application::{EarlierConversations, FoundConversation, StorageError};
use teamwright_domain::{EarlierConversation, Timestamp};

/// The variable that names the folder of Claude Code's settings.
const CONFIG_FOLDER_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// How much of the beginning of a file the list reads, at most, to find
/// the first request. A conversation whose first request comes later is
/// listed without it.
const HEAD_BYTES: u64 = 4 * 1024 * 1024;

/// How much of the end of a file the list reads, to find when it last
/// changed and its title.
const TAIL_BYTES: u64 = 256 * 1024;

/// How many characters of each request are kept while a conversation is
/// read. The digest keeps far fewer; this bounds what a very long paste
/// holds in memory.
const REQUEST_CHARACTERS_READ: usize = 4000;

/// How many characters of the last answer are kept while it is read.
const ANSWER_CHARACTERS_READ: usize = 4000;

/// How many different changed files are kept while a conversation is read.
const MOST_FILES_READ: usize = 1000;

/// The tools of Claude Code that write or change a file, with the field of
/// their input that names it.
const WRITING_TOOLS: &[(&str, &str)] = &[
    ("Write", "file_path"),
    ("Edit", "file_path"),
    ("MultiEdit", "file_path"),
    ("NotebookEdit", "notebook_path"),
];

/// The conversations Claude Code keeps in a folder of this computer.
#[derive(Debug, Clone)]
pub struct EarlierConversationsOnDisk {
    /// The projects folder of Claude Code, or none if it cannot be known.
    projects_folder: Option<PathBuf>,
}

impl EarlierConversationsOnDisk {
    /// The conversations kept in a given projects folder.
    #[must_use]
    pub const fn in_folder(projects_folder: PathBuf) -> Self {
        Self {
            projects_folder: Some(projects_folder),
        }
    }

    /// The conversations of the user of this computer: in `projects` of
    /// `CLAUDE_CONFIG_DIR` if it is set, otherwise of `.claude` in the
    /// home folder.
    #[must_use]
    pub fn of_this_computer() -> Self {
        let home_variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        let settings = std::env::var_os(CONFIG_FOLDER_VARIABLE)
            .filter(|folder| !folder.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os(home_variable)
                    .filter(|home| !home.is_empty())
                    .map(|home| PathBuf::from(home).join(".claude"))
            });
        Self {
            projects_folder: settings.map(|folder| folder.join("projects")),
        }
    }

    /// The file of a source, `<folder name>/<session>`, if the source has
    /// that form. A source never leads out of the projects folder: each
    /// part must be one plain name.
    fn file_of(&self, source: &str) -> Option<PathBuf> {
        let (folder, session) = source.split_once('/')?;
        let is_plain = |part: &str| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.contains(['/', '\\', ':', '\0'])
        };
        if !is_plain(folder) || !is_plain(session) {
            return None;
        }
        Some(
            self.projects_folder
                .as_ref()?
                .join(folder)
                .join(format!("{session}.jsonl")),
        )
    }
}

impl EarlierConversations for EarlierConversationsOnDisk {
    fn list(&self) -> Result<Vec<FoundConversation>, StorageError> {
        let Some(root) = self.projects_folder.as_ref().filter(|root| root.is_dir()) else {
            // Claude Code was never used here.
            return Ok(Vec::new());
        };
        let folders = fs::read_dir(root).map_err(|error| {
            StorageError::caused_by(
                format!(
                    "the conversations of Claude Code in {} cannot be read",
                    root.display()
                ),
                error,
            )
        })?;
        let mut found = Vec::new();
        for folder in folders.flatten() {
            let path = folder.path();
            let Some(folder_name) = file_name_of(&path) else {
                continue;
            };
            if !path.is_dir() {
                continue;
            }
            // A folder that cannot be read is left out: the others still
            // count.
            let Ok(files) = fs::read_dir(&path) else {
                continue;
            };
            for file in files.flatten() {
                let file = file.path();
                if file.extension().and_then(|extension| extension.to_str()) != Some("jsonl")
                    || !file.is_file()
                {
                    continue;
                }
                let Some(session) = file
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_owned)
                else {
                    continue;
                };
                if let Ok(conversation) =
                    summary_of(&file, &format!("{folder_name}/{session}"), &folder_name)
                {
                    found.push(conversation);
                }
            }
        }
        Ok(found)
    }

    fn read(&self, source: &str) -> Result<Option<EarlierConversation>, StorageError> {
        let Some(file) = self.file_of(source).filter(|file| file.is_file()) else {
            return Ok(None);
        };
        let opened = File::open(&file).map_err(|error| cannot_read(&file, error))?;
        let mut reading = ConversationReading::default();
        for_each_line(BufReader::new(opened), u64::MAX, |line| reading.take(line))
            .map_err(|error| cannot_read(&file, error))?;
        let folder_name = source.split_once('/').map_or("", |(folder, _)| folder);
        Ok(Some(reading.into_conversation(source, folder_name)))
    }
}

/// The error of a conversation that cannot be read.
fn cannot_read(file: &Path, error: std::io::Error) -> StorageError {
    StorageError::caused_by(
        format!("the conversation {} cannot be read", file.display()),
        error,
    )
}

/// The last part of a path, if it is text.
fn file_name_of(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

/// What the list shows of one conversation, read from the beginning and the
/// end of its file only.
fn summary_of(file: &Path, source: &str, folder_name: &str) -> std::io::Result<FoundConversation> {
    let metadata = fs::metadata(file)?;
    let size = metadata.len();
    let mut head = ConversationReading::default();
    for_each_line(BufReader::new(File::open(file)?), HEAD_BYTES, |line| {
        head.take(line);
        // Enough is known once the first request, the folder and the start
        // are.
        !(!head.requests.is_empty() && head.held_in.is_some() && head.started_at.is_some())
    })?;

    let mut tail = ConversationReading::default();
    let mut opened = File::open(file)?;
    let skips_partial_line = size > TAIL_BYTES;
    if skips_partial_line {
        opened.seek(SeekFrom::Start(size - TAIL_BYTES))?;
    }
    let mut reader = BufReader::new(opened);
    if skips_partial_line {
        // The first line read from the middle of a file is a piece of one.
        let mut piece = Vec::new();
        reader.read_until(b'\n', &mut piece)?;
    }
    for_each_line(reader, TAIL_BYTES, |line| {
        tail.take(line);
        true
    })?;

    let changed = metadata.modified().ok().map(Timestamp::from);
    let title = tail.title().or_else(|| head.title());
    Ok(FoundConversation {
        source: source.to_owned(),
        held_in: head
            .held_in
            .or(tail.held_in)
            .unwrap_or_else(|| folder_name.to_owned()),
        title,
        first_request: head.requests.into_iter().next(),
        started_at: head.started_at.or(tail.started_at),
        ended_at: tail.ended_at.or(head.ended_at).or(changed),
        size_bytes: size,
    })
}

/// Calls `take` with every line of a reader, as text, until it answers
/// `false` or `limit` bytes were read. A line that is not valid UTF-8 is
/// read with its broken characters replaced; one that is not JSON is the
/// business of `take`.
fn for_each_line(
    mut reader: impl BufRead,
    limit: u64,
    mut take: impl FnMut(&str) -> bool,
) -> std::io::Result<()> {
    let mut read: u64 = 0;
    let mut line = Vec::new();
    loop {
        line.clear();
        let length = reader.read_until(b'\n', &mut line)?;
        if length == 0 {
            return Ok(());
        }
        read = read.saturating_add(u64::try_from(length).unwrap_or(u64::MAX));
        if !take(&String::from_utf8_lossy(&line)) || read >= limit {
            return Ok(());
        }
    }
}

/// What is gathered while the lines of a conversation are read.
#[derive(Debug, Default)]
struct ConversationReading {
    held_in: Option<String>,
    started_at: Option<Timestamp>,
    ended_at: Option<Timestamp>,
    custom_title: Option<String>,
    generated_title: Option<String>,
    requests: Vec<String>,
    changed_files: Vec<String>,
    seen_files: HashSet<String>,
    last_answer: Option<String>,
}

impl ConversationReading {
    /// Takes in one line. A line that is not a JSON object, and a kind of
    /// line that is not known, are skipped. Answers `true`, to go on.
    fn take(&mut self, line: &str) -> bool {
        let line = line.trim();
        if line.is_empty() {
            return true;
        }
        let Ok(Value::Object(object)) = serde_json::from_str::<Value>(line) else {
            return true;
        };
        let value = Value::Object(object);
        if let Some(moment) = text(&value, "timestamp").and_then(moment_of) {
            self.started_at.get_or_insert(moment);
            self.ended_at = Some(moment);
        }
        if self.held_in.is_none() {
            self.held_in = text(&value, "cwd")
                .filter(|folder| !folder.trim().is_empty())
                .map(str::to_owned);
        }
        match text(&value, "type") {
            Some("user") => {
                if let Some(request) = request_of(&value) {
                    self.requests
                        .push(request.chars().take(REQUEST_CHARACTERS_READ).collect());
                }
            }
            Some("assistant") if !flag(&value, "isSidechain") => {
                if let Some(answer) = answer_of(&value) {
                    self.last_answer = Some(answer.chars().take(ANSWER_CHARACTERS_READ).collect());
                }
                for file in changed_files_of(&value) {
                    if self.seen_files.len() < MOST_FILES_READ
                        && self.seen_files.insert(file.clone())
                    {
                        self.changed_files.push(file);
                    }
                }
            }
            Some("custom-title") => {
                self.custom_title = text(&value, "customTitle").map(str::to_owned);
            }
            Some("ai-title") => {
                self.generated_title = text(&value, "aiTitle").map(str::to_owned);
            }
            _ => {}
        }
        true
    }

    /// The title the user gave, or else the one Claude Code made.
    fn title(&self) -> Option<String> {
        self.custom_title
            .clone()
            .or_else(|| self.generated_title.clone())
            .filter(|title| !title.trim().is_empty())
    }

    /// Everything gathered, as the conversation it describes.
    fn into_conversation(self, source: &str, folder_name: &str) -> EarlierConversation {
        let title = self.title();
        EarlierConversation {
            source: source.to_owned(),
            held_in: self.held_in.unwrap_or_else(|| folder_name.to_owned()),
            title,
            started_at: self.started_at,
            ended_at: self.ended_at,
            requests: self.requests,
            changed_files: self.changed_files,
            last_answer: self.last_answer,
        }
    }
}

/// A field of a line, if it is text.
fn text<'value>(value: &'value Value, key: &str) -> Option<&'value str> {
    value.get(key).and_then(Value::as_str)
}

/// Whether a field of a line is `true`.
fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool) == Some(true)
}

/// A moment written as Claude Code writes it, `2026-11-21T09:00:00.000Z`.
fn moment_of(text: &str) -> Option<Timestamp> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|moment| moment.with_timezone(&chrono::Utc))
}

/// What the user typed, if the line is a request of the user: not the
/// answer of a tool, not a line Claude Code added by itself (a reminder, a
/// command and its output, a notice, the summary of an earlier part), and
/// not a line of a helper it started.
fn request_of(line: &Value) -> Option<String> {
    let added_by_the_program = [
        "isMeta",
        "isSidechain",
        "isCompactSummary",
        "isVisibleInTranscriptOnly",
    ]
    .iter()
    .any(|key| flag(line, key));
    if added_by_the_program || line.get("toolUseResult").is_some() {
        return None;
    }
    // A line that says where it came from must say it came from a person.
    if let Some(origin) = line.get("origin")
        && !origin.is_null()
        && origin.get("kind").and_then(Value::as_str) != Some("human")
    {
        return None;
    }
    let content = line.get("message")?.get("content")?;
    let typed = match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => {
            if blocks
                .iter()
                .any(|block| text(block, "type") == Some("tool_result"))
            {
                return None;
            }
            blocks
                .iter()
                .filter(|block| text(block, "type") == Some("text"))
                .filter_map(|block| text(block, "text"))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return None,
    };
    // What Claude Code adds behind what was typed is not part of it.
    let typed = typed
        .split_once("<system-reminder>")
        .map_or(typed.as_str(), |(before, _)| before)
        .trim();
    // A line that begins with a tag is a command, its output or a notice
    // the program wrapped; "[Request interrupted…" is the program's too.
    if typed.is_empty() || typed.starts_with('<') || typed.starts_with("[Request interrupted") {
        return None;
    }
    Some(typed.to_owned())
}

/// The words of an answer of Claude Code: its blocks of text, without what
/// it thought and without its tool calls.
fn answer_of(line: &Value) -> Option<String> {
    let blocks = line.get("message")?.get("content")?.as_array()?;
    let words = blocks
        .iter()
        .filter(|block| text(block, "type") == Some("text"))
        .filter_map(|block| text(block, "text"))
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!words.is_empty()).then_some(words)
}

/// The files the tool calls of an answer write or change.
fn changed_files_of(line: &Value) -> Vec<String> {
    let Some(blocks) = line
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    blocks
        .iter()
        .filter(|block| text(block, "type") == Some("tool_use"))
        .filter_map(|block| {
            let name = text(block, "name")?;
            let (_, field) = WRITING_TOOLS.iter().find(|(tool, _)| *tool == name)?;
            text(block.get("input")?, field)
                .filter(|path| !path.trim().is_empty())
                .map(str::to_owned)
        })
        .collect()
}

#[cfg(test)]
mod tests;
