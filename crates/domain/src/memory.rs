//! The memory of a project: what the team should not have to ask twice.
//!
//! An entry of memory is one of three kinds:
//!
//! - a **decision**: a question and the answer the manager gave;
//! - a **fact**: something true about the project that others need, such
//!   as "The display is on SPI2";
//! - a **contract**: an interface between parts, as a handoff carries one.
//!
//! Each entry says where it came from: the manager wrote it, a specialist
//! remembered it, a decision request was answered, or a handoff was
//! published. An entry is never changed in place. A newer entry replaces
//! it, and the older one says so and stays readable.
//!
//! The rules here concern one entry, and whether one entry fits the words
//! of a search. Which entries a search finds and in what order, and which
//! go into a brief, is the business of the application.

use crate::error::DomainError;
use crate::ids::{MemoryEntryId, ProjectId, TaskId};
use crate::macros::text_enum;
use crate::teamwork::SaidIn;
use crate::time::Timestamp;

/// What an entry of memory is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryKind {
    /// A question and the answer the manager gave.
    Decision,
    /// Something true about the project that others need.
    Fact,
    /// An interface between parts of the work.
    Contract,
}

text_enum!(MemoryKind, kind = "kind of memory entry", {
    Decision => "decision",
    Fact => "fact",
    Contract => "contract",
});

/// Where an entry of memory came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOrigin {
    /// The manager wrote it by hand.
    Manager,
    /// A specialist remembered it with the tool `remember`.
    Specialist(SaidIn),
    /// The manager answered a decision request of a specialist. The
    /// specialist and the turn are those that asked.
    DecisionRequest(SaidIn),
    /// A specialist published a handoff with an interface contract.
    Handoff(SaidIn),
}

/// The stable names of the origins, as they are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemorySource {
    /// The manager wrote it.
    Manager,
    /// A specialist remembered it.
    Specialist,
    /// An answered decision request.
    DecisionRequest,
    /// A published handoff.
    Handoff,
}

text_enum!(MemorySource, kind = "source of a memory entry", {
    Manager => "manager",
    Specialist => "specialist",
    DecisionRequest => "decision_request",
    Handoff => "handoff",
});

impl MemoryOrigin {
    /// The name of the origin, without who and where.
    #[must_use]
    pub const fn source(&self) -> MemorySource {
        match self {
            Self::Manager => MemorySource::Manager,
            Self::Specialist(_) => MemorySource::Specialist,
            Self::DecisionRequest(_) => MemorySource::DecisionRequest,
            Self::Handoff(_) => MemorySource::Handoff,
        }
    }

    /// The specialist, conversation and turn it came from, unless the
    /// manager wrote it.
    #[must_use]
    pub const fn said_in(&self) -> Option<SaidIn> {
        match self {
            Self::Manager => None,
            Self::Specialist(said_in) | Self::DecisionRequest(said_in) | Self::Handoff(said_in) => {
                Some(*said_in)
            }
        }
    }

    /// Puts a stored source and where it was said back together.
    ///
    /// # Errors
    ///
    /// Returns an error if the manager's entry names a specialist, or if
    /// another entry names none.
    pub fn from_parts(source: MemorySource, said_in: Option<SaidIn>) -> Result<Self, DomainError> {
        match (source, said_in) {
            (MemorySource::Manager, None) => Ok(Self::Manager),
            (MemorySource::Specialist, Some(said_in)) => Ok(Self::Specialist(said_in)),
            (MemorySource::DecisionRequest, Some(said_in)) => Ok(Self::DecisionRequest(said_in)),
            (MemorySource::Handoff, Some(said_in)) => Ok(Self::Handoff(said_in)),
            _ => Err(DomainError::MemoryOriginIncomplete),
        }
    }
}

/// What an entry says: a short title and a text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryContent {
    title: String,
    text: String,
}

impl MemoryContent {
    /// The most characters of a title.
    pub const MAX_TITLE_LENGTH: usize = 120;
    /// The most characters of a text. An interface contract may be as long
    /// as a handoff allows.
    pub const MAX_TEXT_LENGTH: usize = 20_000;

    /// Checks what an entry says. Whitespace around both is removed, and a
    /// title is one line.
    ///
    /// # Errors
    ///
    /// Returns an error if the title or the text is empty or too long, or
    /// if the title has more than one line.
    pub fn new(title: &str, text: &str) -> Result<Self, DomainError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(DomainError::EmptyText {
                field: "title of a memory entry",
            });
        }
        if title.contains(['\n', '\r']) {
            return Err(DomainError::TitleOnSeveralLines);
        }
        check_length("title of a memory entry", title, Self::MAX_TITLE_LENGTH)?;
        let text = text.trim();
        if text.is_empty() {
            return Err(DomainError::EmptyText {
                field: "text of a memory entry",
            });
        }
        check_length("text of a memory entry", text, Self::MAX_TEXT_LENGTH)?;
        Ok(Self {
            title: title.to_owned(),
            text: text.to_owned(),
        })
    }

    /// A title made from a longer text: its first line, shortened to fit
    /// with an ellipsis at the end. Nothing if the text is empty.
    #[must_use]
    pub fn title_from(text: &str) -> Option<String> {
        let line = text.trim().lines().next()?.trim();
        if line.is_empty() {
            return None;
        }
        Some(shorten(line, Self::MAX_TITLE_LENGTH))
    }

    /// The short title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// The plain data of an entry, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntrySnapshot {
    /// The identifier of the entry.
    pub id: MemoryEntryId,
    /// The project it belongs to.
    pub project_id: ProjectId,
    /// A decision, a fact or a contract.
    pub kind: MemoryKind,
    /// What it says.
    pub content: MemoryContent,
    /// Where it came from.
    pub origin: MemoryOrigin,
    /// The task it belongs to, if any. The task may have been deleted
    /// since.
    pub task_id: Option<TaskId>,
    /// When it was recorded.
    pub recorded_at: Timestamp,
    /// The newer entry that replaced it, if one has.
    pub replaced_by: Option<MemoryEntryId>,
}

/// One entry of the memory of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    id: MemoryEntryId,
    project_id: ProjectId,
    kind: MemoryKind,
    content: MemoryContent,
    origin: MemoryOrigin,
    task_id: Option<TaskId>,
    recorded_at: Timestamp,
    replaced_by: Option<MemoryEntryId>,
}

impl MemoryEntry {
    /// Records a new entry.
    #[must_use]
    pub fn record(
        project_id: ProjectId,
        kind: MemoryKind,
        content: MemoryContent,
        origin: MemoryOrigin,
        task_id: Option<TaskId>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: MemoryEntryId::generate(),
            project_id,
            kind,
            content,
            origin,
            task_id,
            recorded_at: now,
            replaced_by: None,
        }
    }

    /// Recreates an entry from stored data.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry names itself as its replacement.
    pub fn restore(snapshot: MemoryEntrySnapshot) -> Result<Self, DomainError> {
        if snapshot.replaced_by == Some(snapshot.id) {
            return Err(DomainError::MemoryEntryReplacesItself);
        }
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            kind: snapshot.kind,
            content: snapshot.content,
            origin: snapshot.origin,
            task_id: snapshot.task_id,
            recorded_at: snapshot.recorded_at,
            replaced_by: snapshot.replaced_by,
        })
    }

    /// Writes a newer entry that replaces `older`, and marks `older` as
    /// replaced by it. The newer entry has the kind and the task of the
    /// older one, and comes from `origin`.
    ///
    /// # Errors
    ///
    /// Returns an error if `older` was replaced already: only the current
    /// entry can be replaced.
    pub fn replace(
        older: &mut Self,
        content: MemoryContent,
        origin: MemoryOrigin,
        now: Timestamp,
    ) -> Result<Self, DomainError> {
        if older.replaced_by.is_some() {
            return Err(DomainError::MemoryEntryReplaced);
        }
        let newer = Self::record(
            older.project_id,
            older.kind,
            content,
            origin,
            older.task_id,
            now,
        );
        older.replaced_with(&newer)?;
        Ok(newer)
    }

    /// Marks this entry as replaced by a newer one that was recorded on its
    /// own, such as the contract of a later handoff of the same task.
    ///
    /// # Errors
    ///
    /// Returns an error if this entry was replaced already, or if `newer`
    /// is this entry itself.
    pub fn replaced_with(&mut self, newer: &Self) -> Result<(), DomainError> {
        if self.replaced_by.is_some() {
            return Err(DomainError::MemoryEntryReplaced);
        }
        if newer.id == self.id {
            return Err(DomainError::MemoryEntryReplacesItself);
        }
        self.replaced_by = Some(newer.id);
        Ok(())
    }

    /// The identifier of the entry.
    #[must_use]
    pub const fn id(&self) -> MemoryEntryId {
        self.id
    }

    /// The project it belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// A decision, a fact or a contract.
    #[must_use]
    pub const fn kind(&self) -> MemoryKind {
        self.kind
    }

    /// What it says.
    #[must_use]
    pub const fn content(&self) -> &MemoryContent {
        &self.content
    }

    /// The short title.
    #[must_use]
    pub fn title(&self) -> &str {
        self.content.title()
    }

    /// The text.
    #[must_use]
    pub fn text(&self) -> &str {
        self.content.text()
    }

    /// Where it came from.
    #[must_use]
    pub const fn origin(&self) -> MemoryOrigin {
        self.origin
    }

    /// The task it belongs to, if any.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// When it was recorded.
    #[must_use]
    pub const fn recorded_at(&self) -> Timestamp {
        self.recorded_at
    }

    /// The newer entry that replaced it, if one has.
    #[must_use]
    pub const fn replaced_by(&self) -> Option<MemoryEntryId> {
        self.replaced_by
    }

    /// Whether it is current: no newer entry replaced it. Only current
    /// entries are searched and go into a brief.
    #[must_use]
    pub const fn is_current(&self) -> bool {
        self.replaced_by.is_none()
    }
}

/// The words of a search in memory.
///
/// A search is a few plain words. An entry fits when each of them begins a
/// word of its title or its text, whatever the capitals: "spi" fits "SPI2"
/// and "display" fits "displays". There is no model and no guessing, so
/// the manager and the specialists find the same entries with the same
/// words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchWords {
    words: Vec<String>,
}

impl SearchWords {
    /// The most characters of a search.
    pub const MAX_LENGTH: usize = 200;
    /// The most words of a search.
    pub const MOST_WORDS: usize = 12;

    /// Reads the words of a search: everything between letters and digits
    /// separates two words, small letters are used, and a word given twice
    /// counts once.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no word, if the search is too long, or
    /// if it has too many words.
    pub fn parse(query: &str) -> Result<Self, DomainError> {
        check_length("search", query.trim(), Self::MAX_LENGTH)?;
        let mut words: Vec<String> = Vec::new();
        for word in words_of(query) {
            if !words.contains(&word) {
                words.push(word);
            }
        }
        if words.is_empty() {
            return Err(DomainError::EmptyText { field: "search" });
        }
        if words.len() > Self::MOST_WORDS {
            return Err(DomainError::TooManySearchWords {
                max: Self::MOST_WORDS,
                count: words.len(),
            });
        }
        Ok(Self { words })
    }

    /// The words, in small letters, in the order they were given.
    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// Whether a title and a text fit: each word of the search begins a
    /// word of one or the other.
    #[must_use]
    pub fn fit(&self, title: &str, text: &str) -> bool {
        let found: Vec<String> = words_of(title).chain(words_of(text)).collect();
        self.words
            .iter()
            .all(|wanted| found.iter().any(|word| word.starts_with(wanted.as_str())))
    }
}

/// The words of a text, in small letters: runs of letters and digits.
fn words_of(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}

/// A text of at most `max` characters: the text itself, or its beginning
/// with an ellipsis.
#[must_use]
pub fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", kept.trim_end())
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::ids::{ConversationId, MemberId, TurnId};

    fn now() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap()
    }

    fn said_in() -> SaidIn {
        SaidIn {
            member_id: MemberId::generate(),
            conversation_id: ConversationId::generate(),
            turn_id: TurnId::generate(),
        }
    }

    fn content(title: &str, text: &str) -> MemoryContent {
        MemoryContent::new(title, text).unwrap()
    }

    #[test]
    fn an_entry_says_something_in_a_short_title_and_a_text() {
        let content = content("  Display bus ", "  The display is on SPI2.  ");
        assert_eq!(content.title(), "Display bus");
        assert_eq!(content.text(), "The display is on SPI2.");

        assert_eq!(
            MemoryContent::new(" ", "text"),
            Err(DomainError::EmptyText {
                field: "title of a memory entry"
            })
        );
        assert_eq!(
            MemoryContent::new("Title", "\n"),
            Err(DomainError::EmptyText {
                field: "text of a memory entry"
            })
        );
        assert_eq!(
            MemoryContent::new("Two\nlines", "text"),
            Err(DomainError::TitleOnSeveralLines)
        );
        assert!(matches!(
            MemoryContent::new(&"t".repeat(121), "text"),
            Err(DomainError::TextTooLong {
                max_length: 120,
                ..
            })
        ));
        assert!(matches!(
            MemoryContent::new("Title", &"x".repeat(20_001)),
            Err(DomainError::TextTooLong { .. })
        ));
    }

    #[test]
    fn a_title_is_made_from_the_first_line_of_a_longer_text() {
        assert_eq!(
            MemoryContent::title_from("  Which bus?\nMore."),
            Some("Which bus?".to_owned())
        );
        let long = MemoryContent::title_from(&"word ".repeat(40)).unwrap();
        assert_eq!(long.chars().count(), 120);
        assert!(long.ends_with('…'));
        assert_eq!(MemoryContent::title_from("   "), None);
    }

    #[test]
    fn an_origin_names_a_specialist_unless_the_manager_wrote_it() {
        let said = said_in();
        assert_eq!(MemoryOrigin::Manager.said_in(), None);
        assert_eq!(MemoryOrigin::Handoff(said).said_in(), Some(said));
        assert_eq!(
            MemoryOrigin::from_parts(MemorySource::DecisionRequest, Some(said)),
            Ok(MemoryOrigin::DecisionRequest(said))
        );
        assert_eq!(
            MemoryOrigin::from_parts(MemorySource::Manager, Some(said)),
            Err(DomainError::MemoryOriginIncomplete)
        );
        assert_eq!(
            MemoryOrigin::from_parts(MemorySource::Specialist, None),
            Err(DomainError::MemoryOriginIncomplete)
        );
        assert_eq!(MemorySource::DecisionRequest.as_str(), "decision_request");
        assert_eq!("contract".parse::<MemoryKind>(), Ok(MemoryKind::Contract));
    }

    #[test]
    fn a_newer_entry_replaces_an_older_one_which_says_so() {
        let task_id = TaskId::generate();
        let mut older = MemoryEntry::record(
            ProjectId::generate(),
            MemoryKind::Fact,
            content("Enclosure", "Under 15 mm."),
            MemoryOrigin::Specialist(said_in()),
            Some(task_id),
            now(),
        );
        assert!(older.is_current());

        let newer = MemoryEntry::replace(
            &mut older,
            content("Enclosure", "Under 12 mm."),
            MemoryOrigin::Manager,
            now(),
        )
        .unwrap();

        assert_eq!(older.replaced_by(), Some(newer.id()));
        assert!(!older.is_current());
        assert_eq!(older.text(), "Under 15 mm.");
        assert_eq!(newer.kind(), MemoryKind::Fact);
        assert_eq!(newer.task_id(), Some(task_id));
        assert_eq!(newer.origin(), MemoryOrigin::Manager);
        assert!(newer.is_current());
        assert_eq!(
            MemoryEntry::replace(
                &mut older,
                content("Again", "No."),
                MemoryOrigin::Manager,
                now()
            ),
            Err(DomainError::MemoryEntryReplaced)
        );
    }

    #[test]
    fn an_entry_recorded_on_its_own_may_replace_a_current_one() {
        let project_id = ProjectId::generate();
        let record = |text: &str| {
            MemoryEntry::record(
                project_id,
                MemoryKind::Contract,
                content("Interface of SSN-1", text),
                MemoryOrigin::Handoff(said_in()),
                None,
                now(),
            )
        };
        let mut older = record("SPI2 at 8 MHz.");
        let newer = record("SPI2 at 10 MHz.");

        assert_eq!(
            older.clone().replaced_with(&older),
            Err(DomainError::MemoryEntryReplacesItself)
        );
        older.replaced_with(&newer).unwrap();
        assert_eq!(older.replaced_by(), Some(newer.id()));
        assert_eq!(
            older.replaced_with(&record("Again.")),
            Err(DomainError::MemoryEntryReplaced)
        );
    }

    #[test]
    fn an_entry_cannot_be_stored_as_its_own_replacement() {
        let entry = MemoryEntry::record(
            ProjectId::generate(),
            MemoryKind::Decision,
            content("Power", "Battery."),
            MemoryOrigin::Manager,
            None,
            now(),
        );
        let snapshot = MemoryEntrySnapshot {
            id: entry.id(),
            project_id: entry.project_id(),
            kind: entry.kind(),
            content: entry.content().clone(),
            origin: entry.origin(),
            task_id: None,
            recorded_at: now(),
            replaced_by: Some(entry.id()),
        };
        assert_eq!(
            MemoryEntry::restore(snapshot),
            Err(DomainError::MemoryEntryReplacesItself)
        );
    }

    #[test]
    fn the_words_of_a_search_are_small_and_counted_once() {
        let words = SearchWords::parse("  SPI, display/spi  ").unwrap();
        assert_eq!(words.words(), ["spi", "display"]);

        assert_eq!(
            SearchWords::parse(" ,. "),
            Err(DomainError::EmptyText { field: "search" })
        );
        assert!(matches!(
            SearchWords::parse(&"a".repeat(201)),
            Err(DomainError::TextTooLong { .. })
        ));
        assert_eq!(
            SearchWords::parse("a b c d e f g h i j k l m"),
            Err(DomainError::TooManySearchWords { max: 12, count: 13 })
        );
    }

    #[test]
    fn an_entry_fits_when_every_word_begins_a_word_of_it() {
        let words = SearchWords::parse("spi display").unwrap();
        assert!(words.fit("Display bus", "The display is on SPI2."));
        assert!(words.fit("Displays", "spi"));
        assert!(!words.fit("Display bus", "On I2C."));
        // A word must begin a word, not sit inside one.
        assert!(!SearchWords::parse("play").unwrap().fit("Display", ""));
        assert!(
            SearchWords::parse("12")
                .unwrap()
                .fit("Enclosure", "Under 12mm thick.")
        );
        assert!(
            SearchWords::parse("größe")
                .unwrap()
                .fit("GRÖSSE", "Größe des Gehäuses")
        );
    }

    #[test]
    fn a_long_text_is_shortened_with_an_ellipsis() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("a long sentence", 7), "a long…");
    }
}
