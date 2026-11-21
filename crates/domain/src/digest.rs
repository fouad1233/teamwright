//! Digests of earlier conversations: short accounts of a conversation the
//! manager held with Claude Code before the project came to Teamwright
//! ([decision 0041](../../../docs/decisions/0041-earlier-conversations-as-digests.md)).
//!
//! A digest is made by rules, without a model: the first request, the
//! requests that followed in order, the files that were written or
//! changed, the last answer, and how long it went. It never holds what a
//! tool answered or what a file contains, and what looks like a secret is
//! hidden ([`hide_secrets`]). The manager reads it, may cut or correct it,
//! and keeps it with the project. The Advisor starts from the kept digests.

use std::collections::HashSet;

use crate::error::DomainError;
use crate::ids::{DigestId, ProjectId};
use crate::secrets::hide_secrets;
use crate::text::DigestTitle;
use crate::time::Timestamp;

/// How many characters of the first request a digest keeps.
pub const FIRST_REQUEST_LENGTH: usize = 600;

/// How many characters of each later request a digest keeps.
pub const REQUEST_LENGTH: usize = 200;

/// How many later requests a digest lists at most: the first ones and the
/// last ones, with a line for those in between.
pub const MOST_REQUESTS: usize = 16;

/// How many files a digest lists at most.
pub const MOST_FILES: usize = 30;

/// How many characters of the path of a file a digest keeps.
pub const FILE_LENGTH: usize = 160;

/// How many characters of the last answer a digest keeps.
pub const LAST_ANSWER_LENGTH: usize = 600;

/// How many characters of the first request make a title, when the
/// conversation has no title of its own.
const TITLE_FROM_REQUEST_LENGTH: usize = 80;

/// What a reader found in one earlier conversation: plain data, read from
/// the files the agent program keeps. Nothing here is checked; a digest is
/// made from it by [`DigestDraft::of`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EarlierConversation {
    /// How the reader finds the conversation again.
    pub source: String,
    /// The folder the conversation was held in, as the program wrote it.
    pub held_in: String,
    /// The title the program or the user gave the conversation, if any.
    pub title: Option<String>,
    /// When it began, if any line said.
    pub started_at: Option<Timestamp>,
    /// When it last changed, if any line said.
    pub ended_at: Option<Timestamp>,
    /// What the user typed, in order. Not what tools answered, not what the
    /// program added by itself.
    pub requests: Vec<String>,
    /// The files the program wrote or changed, as its tools named them, in
    /// order. A file may be named more than once.
    pub changed_files: Vec<String>,
    /// The last thing the program answered in words.
    pub last_answer: Option<String>,
}

/// A digest made by the rules, for the manager to read and correct before
/// it is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestDraft {
    /// How the conversation is found again.
    pub source: String,
    /// The folder it was held in.
    pub held_in: String,
    /// Its title.
    pub title: DigestTitle,
    /// When it began.
    pub started_at: Option<Timestamp>,
    /// When it last changed.
    pub ended_at: Option<Timestamp>,
    /// The account, in plain lines.
    pub text: String,
}

impl DigestDraft {
    /// Makes the digest of an earlier conversation by the rules: a few
    /// lines on when and where it was held, the first request, the later
    /// requests in order (the first and the last ones, when there are
    /// many), the files that were written or changed, and the last answer.
    /// Each is cut to its length, and what looks like a secret is hidden
    /// before it is cut, so that no part of a secret is left.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation holds no request of the user:
    /// there is nothing to give an account of.
    pub fn of(conversation: &EarlierConversation) -> Result<Self, DomainError> {
        let requests: Vec<String> = conversation
            .requests
            .iter()
            .map(|request| one_line(&hide_secrets(request)))
            .filter(|request| !request.is_empty())
            .collect();
        let Some((first, later)) = requests.split_first() else {
            return Err(DomainError::NothingToDigest);
        };

        let mut lines = vec![when_and_where(conversation, requests.len())];
        lines.push(String::new());
        lines.push("First request:".to_owned());
        lines.push(cut(first, FIRST_REQUEST_LENGTH));

        if !later.is_empty() {
            lines.push(String::new());
            lines.push("Then, in order:".to_owned());
            lines.extend(listed_requests(later));
        }

        let files = changed_files(&conversation.changed_files, &conversation.held_in);
        if !files.is_empty() {
            lines.push(String::new());
            lines.push(format!("Files written or changed ({}):", files.len()));
            let shown = files.len().min(MOST_FILES);
            lines.extend(
                files
                    .iter()
                    .take(shown)
                    .map(|file| format!("- {}", cut(file, FILE_LENGTH))),
            );
            if files.len() > shown {
                lines.push(format!("- and {} more", files.len() - shown));
            }
        }

        if let Some(answer) = conversation
            .last_answer
            .as_deref()
            .map(|answer| one_line(&hide_secrets(answer)))
            .filter(|answer| !answer.is_empty())
        {
            lines.push(String::new());
            lines.push("Last answer:".to_owned());
            lines.push(cut(&answer, LAST_ANSWER_LENGTH));
        }

        let title = conversation
            .title
            .as_deref()
            .map(|title| one_line(&hide_secrets(title)))
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| cut(first, TITLE_FROM_REQUEST_LENGTH));
        Ok(Self {
            source: conversation.source.clone(),
            held_in: conversation.held_in.clone(),
            title: DigestTitle::parse(&cut(&title, DigestTitle::MAX_LENGTH))?,
            started_at: conversation.started_at,
            ended_at: conversation.ended_at,
            text: cut_lines(&lines.join("\n"), DigestText::MAX_LENGTH),
        })
    }
}

/// The first line of a digest: where it was held, when, for how long, and
/// how many requests the user made.
fn when_and_where(conversation: &EarlierConversation, requests: usize) -> String {
    let mut line = String::from("Held");
    if !conversation.held_in.trim().is_empty() {
        line.push_str(&format!(" in {}", conversation.held_in.trim()));
    }
    match (conversation.started_at, conversation.ended_at) {
        (Some(start), Some(end)) if start.date_naive() != end.date_naive() => {
            let days = (end.date_naive() - start.date_naive()).num_days() + 1;
            line.push_str(&format!(
                " from {} to {} ({days} days)",
                start.date_naive(),
                end.date_naive()
            ));
        }
        (Some(start), _) | (None, Some(start)) => {
            line.push_str(&format!(" on {}", start.date_naive()));
        }
        (None, None) => {}
    }
    let requests_in_words = if requests == 1 {
        "1 request".to_owned()
    } else {
        format!("{requests} requests")
    };
    format!("{line}, {requests_in_words} of the manager.")
}

/// The later requests, numbered from 2: all of them when there are few,
/// otherwise the first and the last ones with a line for those in between.
fn listed_requests(later: &[String]) -> Vec<String> {
    let numbered = |(place, request): (usize, &String)| {
        format!("{}. {}", place + 2, cut(request, REQUEST_LENGTH))
    };
    if later.len() <= MOST_REQUESTS {
        return later.iter().enumerate().map(numbered).collect();
    }
    let head = MOST_REQUESTS / 2;
    let tail = MOST_REQUESTS - head;
    let mut lines: Vec<String> = later.iter().enumerate().take(head).map(numbered).collect();
    lines.push(format!(
        "(the {} requests in between are left out)",
        later.len() - head - tail
    ));
    lines.extend(
        later
            .iter()
            .enumerate()
            .skip(later.len() - tail)
            .map(numbered),
    );
    lines
}

/// The files that were written or changed, each once, in the order they
/// were first named. A file inside the folder of the conversation is named
/// by its path from there; any other by its name alone, so that a digest
/// shows nothing of the rest of the computer.
fn changed_files(files: &[String], held_in: &str) -> Vec<String> {
    let folder = held_in.trim().trim_end_matches(['/', '\\']);
    let mut seen = HashSet::new();
    files
        .iter()
        .filter_map(|file| {
            let file = file.trim();
            if file.is_empty() {
                return None;
            }
            let inside = (!folder.is_empty())
                .then(|| file.strip_prefix(folder))
                .flatten()
                .and_then(|rest| rest.strip_prefix(['/', '\\']))
                .filter(|rest| !rest.is_empty());
            let shown = inside.map_or_else(
                || file.rsplit(['/', '\\']).next().unwrap_or(file).to_owned(),
                str::to_owned,
            );
            let shown = one_line(&hide_secrets(&shown));
            (!shown.is_empty() && seen.insert(shown.clone())).then_some(shown)
        })
        .collect()
}

/// A text on one line: every run of whitespace, line breaks included,
/// becomes one space.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cuts a line to at most `limit` characters, at the end of a word if that
/// keeps at least half, and marks the cut with "…".
#[must_use]
pub fn cut(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let kept: String = text.chars().take(limit.saturating_sub(1)).collect();
    let at_word = kept
        .rfind(' ')
        .filter(|space| kept[..*space].chars().count() * 2 >= limit)
        .map_or(kept.as_str(), |space| &kept[..space]);
    format!("{}…", at_word.trim_end())
}

/// Cuts a text of several lines to at most `limit` characters, at the end
/// of a line where possible.
fn cut_lines(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let mut kept = String::new();
    for line in text.lines() {
        if kept.chars().count() + line.chars().count() + 2 > limit {
            break;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    kept.push('…');
    kept
}

/// The account a digest gives: plain lines, not empty, of limited length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestText(String);

impl DigestText {
    /// The largest number of characters a digest can have.
    pub const MAX_LENGTH: usize = 8000;

    /// Checks a text. Whitespace at both ends is removed, and line breaks
    /// are written as one line feed.
    ///
    /// # Errors
    ///
    /// Returns an error if nothing is left, or if the text is longer than
    /// [`Self::MAX_LENGTH`] characters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");
        let text = text.trim();
        if text.is_empty() {
            return Err(DomainError::EmptyText { field: "digest" });
        }
        let length = text.chars().count();
        if length > Self::MAX_LENGTH {
            return Err(DomainError::TextTooLong {
                field: "digest",
                max_length: Self::MAX_LENGTH,
                actual_length: length,
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// The text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Where a digest came from: how the conversation is found again on the
/// computer it was read on, and the folder it was held in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestSource {
    source: String,
    held_in: String,
}

impl DigestSource {
    /// The largest number of characters of the source.
    pub const MAX_SOURCE_LENGTH: usize = 300;

    /// The largest number of characters of the folder.
    pub const MAX_FOLDER_LENGTH: usize = 1000;

    /// Checks where a digest came from.
    ///
    /// # Errors
    ///
    /// Returns an error if the source is empty, or if either is too long.
    pub fn new(source: &str, held_in: &str) -> Result<Self, DomainError> {
        let source = source.trim();
        if source.is_empty() {
            return Err(DomainError::EmptyText {
                field: "source of the digest",
            });
        }
        check_length("source of the digest", source, Self::MAX_SOURCE_LENGTH)?;
        let held_in = held_in.trim();
        check_length(
            "folder of the conversation",
            held_in,
            Self::MAX_FOLDER_LENGTH,
        )?;
        Ok(Self {
            source: source.to_owned(),
            held_in: held_in.to_owned(),
        })
    }

    /// How the conversation is found again.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The folder the conversation was held in; empty if it is not known.
    #[must_use]
    pub fn held_in(&self) -> &str {
        &self.held_in
    }
}

/// Refuses a text with more characters than `max`.
fn check_length(field: &'static str, text: &str, max: usize) -> Result<(), DomainError> {
    let length = text.chars().count();
    if length > max {
        return Err(DomainError::TextTooLong {
            field,
            max_length: max,
            actual_length: length,
        });
    }
    Ok(())
}

/// The plain data of a digest, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestSnapshot {
    /// The identifier of the digest.
    pub id: DigestId,
    /// The project it belongs to.
    pub project_id: ProjectId,
    /// Where it came from.
    pub source: DigestSource,
    /// Its title.
    pub title: DigestTitle,
    /// The account.
    pub text: DigestText,
    /// When the conversation began, if known.
    pub started_at: Option<Timestamp>,
    /// When it last changed, if known.
    pub ended_at: Option<Timestamp>,
    /// When the manager kept the digest.
    pub kept_at: Timestamp,
}

/// The account of an earlier conversation, as the manager kept it with a
/// project. It never changes once kept: a digest the manager wants
/// otherwise is removed and made again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Digest {
    id: DigestId,
    project_id: ProjectId,
    source: DigestSource,
    title: DigestTitle,
    text: DigestText,
    started_at: Option<Timestamp>,
    ended_at: Option<Timestamp>,
    kept_at: Timestamp,
}

impl Digest {
    /// Keeps a digest with a project, as the manager read and corrected it.
    #[must_use]
    pub fn keep(
        project_id: ProjectId,
        source: DigestSource,
        title: DigestTitle,
        text: DigestText,
        held: (Option<Timestamp>, Option<Timestamp>),
        now: Timestamp,
    ) -> Self {
        Self {
            id: DigestId::generate(),
            project_id,
            source,
            title,
            text,
            started_at: held.0,
            ended_at: held.1,
            kept_at: now,
        }
    }

    /// Recreates a digest from stored data.
    #[must_use]
    pub fn restore(snapshot: DigestSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            source: snapshot.source,
            title: snapshot.title,
            text: snapshot.text,
            started_at: snapshot.started_at,
            ended_at: snapshot.ended_at,
            kept_at: snapshot.kept_at,
        }
    }

    /// The identifier of the digest.
    #[must_use]
    pub const fn id(&self) -> DigestId {
        self.id
    }

    /// The project it belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Where it came from.
    #[must_use]
    pub const fn source(&self) -> &DigestSource {
        &self.source
    }

    /// Its title.
    #[must_use]
    pub const fn title(&self) -> &DigestTitle {
        &self.title
    }

    /// The account.
    #[must_use]
    pub const fn text(&self) -> &DigestText {
        &self.text
    }

    /// When the conversation began, if known.
    #[must_use]
    pub const fn started_at(&self) -> Option<Timestamp> {
        self.started_at
    }

    /// When it last changed, if known.
    #[must_use]
    pub const fn ended_at(&self) -> Option<Timestamp> {
        self.ended_at
    }

    /// When the manager kept it.
    #[must_use]
    pub const fn kept_at(&self) -> Timestamp {
        self.kept_at
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn at(day: u32, hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 10, day, hour, 0, 0)
            .unwrap()
    }

    fn conversation(requests: &[&str]) -> EarlierConversation {
        EarlierConversation {
            source: "-home-ada-weather/abc".to_owned(),
            held_in: "/home/ada/weather".to_owned(),
            title: None,
            started_at: Some(at(3, 9)),
            ended_at: Some(at(5, 17)),
            requests: requests
                .iter()
                .map(|request| (*request).to_owned())
                .collect(),
            changed_files: Vec::new(),
            last_answer: None,
        }
    }

    #[test]
    fn a_conversation_without_a_request_gives_no_digest() {
        assert_eq!(
            DigestDraft::of(&conversation(&[])),
            Err(DomainError::NothingToDigest)
        );
        assert_eq!(
            DigestDraft::of(&conversation(&["  ", "\n"])),
            Err(DomainError::NothingToDigest)
        );
    }

    #[test]
    fn a_digest_says_where_when_and_how_many_requests() {
        let draft =
            DigestDraft::of(&conversation(&["Build a weather station", "Use a BME280"])).unwrap();

        assert_eq!(
            draft.text.lines().next().unwrap(),
            "Held in /home/ada/weather from 2026-10-03 to 2026-10-05 (3 days), 2 requests of the manager."
        );
    }

    #[test]
    fn a_conversation_of_one_day_is_said_to_be_held_on_that_day() {
        let mut held = conversation(&["Hello"]);
        held.ended_at = Some(at(3, 18));

        let draft = DigestDraft::of(&held).unwrap();

        assert!(
            draft
                .text
                .starts_with("Held in /home/ada/weather on 2026-10-03, 1 request")
        );
    }

    #[test]
    fn a_digest_has_the_first_request_and_the_later_ones_numbered() {
        let draft = DigestDraft::of(&conversation(&[
            "Build a weather station",
            "Use a BME280",
            "Make it run on batteries",
        ]))
        .unwrap();

        assert!(
            draft
                .text
                .contains("First request:\nBuild a weather station\n")
        );
        assert!(
            draft
                .text
                .contains("Then, in order:\n2. Use a BME280\n3. Make it run on batteries")
        );
    }

    #[test]
    fn many_requests_are_shown_first_and_last_with_a_line_for_the_rest() {
        let requests: Vec<String> = (1..=40).map(|number| format!("request {number}")).collect();
        let requests: Vec<&str> = requests.iter().map(String::as_str).collect();

        let draft = DigestDraft::of(&conversation(&requests)).unwrap();

        assert!(draft.text.contains("2. request 2\n"));
        assert!(draft.text.contains("9. request 9\n"));
        assert!(!draft.text.contains("10. request 10\n"));
        assert!(
            draft
                .text
                .contains("(the 23 requests in between are left out)")
        );
        assert!(draft.text.contains("33. request 33\n"));
        assert!(draft.text.contains("40. request 40"));
        assert!(draft.text.contains("40 requests of the manager"));
    }

    #[test]
    fn a_long_request_is_cut_and_its_line_breaks_become_spaces() {
        let long = format!("Line one\nline two {}", "word ".repeat(200));

        let draft = DigestDraft::of(&conversation(&["First", &long])).unwrap();
        let listed = draft
            .text
            .lines()
            .find(|line| line.starts_with("2. "))
            .unwrap();

        assert!(listed.starts_with("2. Line one line two word"));
        assert!(listed.ends_with('…'));
        assert!(listed.chars().count() <= REQUEST_LENGTH + 3);
    }

    #[test]
    fn files_are_named_once_from_the_folder_or_by_their_name_alone() {
        let mut held = conversation(&["Write the firmware"]);
        held.changed_files = vec![
            "/home/ada/weather/src/main.c".to_owned(),
            "/home/ada/weather/src/main.c".to_owned(),
            "/etc/private/notes.txt".to_owned(),
            "C:\\Users\\ada\\other.md".to_owned(),
        ];

        let draft = DigestDraft::of(&held).unwrap();

        assert!(
            draft
                .text
                .contains("Files written or changed (3):\n- src/main.c\n- notes.txt\n- other.md")
        );
        assert!(!draft.text.contains("/etc/private"));
    }

    #[test]
    fn many_files_end_with_how_many_more() {
        let mut held = conversation(&["Write it"]);
        held.changed_files = (0..45).map(|number| format!("file{number}.txt")).collect();

        let draft = DigestDraft::of(&held).unwrap();

        assert!(draft.text.contains("Files written or changed (45):"));
        assert!(draft.text.contains("- and 15 more"));
    }

    #[test]
    fn the_last_answer_is_kept_cut() {
        let mut held = conversation(&["Write it"]);
        held.last_answer = Some(format!("Done. {}", "more ".repeat(300)));

        let draft = DigestDraft::of(&held).unwrap();
        let answer = draft.text.lines().last().unwrap();

        assert!(draft.text.contains("Last answer:\nDone. more"));
        assert!(answer.chars().count() <= LAST_ANSWER_LENGTH);
    }

    #[test]
    fn secrets_are_hidden_before_anything_is_cut() {
        let mut held = conversation(&[
            "Deploy with TOKEN=abc123def456",
            "push with ghp_abcdefghijklmnop1234",
        ]);
        held.last_answer = Some("I used password: hunter2 as you said.".to_owned());

        let draft = DigestDraft::of(&held).unwrap();

        assert!(!draft.text.contains("abc123def456"));
        assert!(!draft.text.contains("ghp_abcdefghijklmnop1234"));
        assert!(!draft.text.contains("hunter2"));
        assert!(draft.text.contains("TOKEN=[hidden]"));
    }

    #[test]
    fn the_title_is_the_conversations_own_or_the_first_request() {
        let draft = DigestDraft::of(&conversation(&["Build a weather station"])).unwrap();
        assert_eq!(draft.title.as_str(), "Build a weather station");

        let mut titled = conversation(&["Build a weather station"]);
        titled.title = Some("Weather station planning".to_owned());
        let draft = DigestDraft::of(&titled).unwrap();
        assert_eq!(draft.title.as_str(), "Weather station planning");
    }

    #[test]
    fn a_digest_is_never_longer_than_its_limit() {
        let requests: Vec<String> = (0..100).map(|_| "word ".repeat(100)).collect();
        let requests: Vec<&str> = requests.iter().map(String::as_str).collect();
        let mut held = conversation(&requests);
        held.held_in = "/very/long/folder/".repeat(50);
        held.changed_files = (0..100)
            .map(|number| format!("{}{number}", "f".repeat(150)))
            .collect();
        held.last_answer = Some("answer ".repeat(500));

        let draft = DigestDraft::of(&held).unwrap();

        assert!(draft.text.chars().count() <= DigestText::MAX_LENGTH);
        assert!(DigestText::parse(&draft.text).is_ok());
    }

    #[test]
    fn a_digest_text_is_trimmed_and_its_line_breaks_are_line_feeds() {
        let text = DigestText::parse("  one\r\ntwo\rthree \n").unwrap();

        assert_eq!(text.as_str(), "one\ntwo\nthree");
    }

    #[test]
    fn an_empty_or_too_long_digest_text_is_refused() {
        assert_eq!(
            DigestText::parse(" \n "),
            Err(DomainError::EmptyText { field: "digest" })
        );
        assert!(matches!(
            DigestText::parse(&"x".repeat(DigestText::MAX_LENGTH + 1)),
            Err(DomainError::TextTooLong { .. })
        ));
    }

    #[test]
    fn a_source_must_be_named() {
        assert!(DigestSource::new(" ", "/home/ada").is_err());
        let source = DigestSource::new(" a/b ", " /home/ada ").unwrap();
        assert_eq!(source.source(), "a/b");
        assert_eq!(source.held_in(), "/home/ada");
    }

    #[test]
    fn cut_ends_at_a_word_and_marks_the_cut() {
        assert_eq!(cut("short", 10), "short");
        assert_eq!(cut("one two three four five", 12), "one two…");
        assert_eq!(cut("abcdefghijklmnop", 6), "abcde…");
    }
}
