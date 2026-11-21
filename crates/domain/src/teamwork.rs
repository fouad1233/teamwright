//! What specialists hand each other: handoffs and messages.
//!
//! Specialists never read each other's conversations. They exchange small,
//! structured things instead, through the tools of the team bus:
//!
//! - A **handoff** is the result of a task for whoever builds on it: a
//!   summary, an interface contract if there is one, and the files that
//!   hold the result. The files are the task's *registered artifacts*:
//!   paths inside the work folder of the project, which the specialist
//!   names and Teamwright keeps a note of. The files themselves stay where
//!   they are.
//! - A **message** is a question or a note from one specialist to another.
//!   Nobody polls for it: it waits until the receiver starts its next turn,
//!   and is delivered with that turn.
//!
//! The rules here concern one handoff or one message. Which messages wait
//! for a member, and what a brief says about handoffs, is the business of
//! the application.

use crate::error::DomainError;
use crate::ids::{ConversationId, HandoffId, MemberId, ProjectId, TaskId, TeamMessageId, TurnId};
use crate::macros::text_enum;
use crate::project_files::PathInFolder;
use crate::time::Timestamp;

/// What a handoff says: a summary, an interface contract and files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffContent {
    summary: String,
    contract: String,
    files: Vec<PathInFolder>,
}

impl HandoffContent {
    /// The most characters of a summary.
    pub const MAX_SUMMARY_LENGTH: usize = 4_000;
    /// The most characters of an interface contract.
    pub const MAX_CONTRACT_LENGTH: usize = 20_000;
    /// The most files a handoff names.
    pub const MOST_FILES: usize = 50;

    /// Checks what a handoff says. Whitespace around the texts is removed,
    /// and every path is written with `/` between its parts.
    ///
    /// # Errors
    ///
    /// Returns an error if the summary is empty, if a text is too long, if
    /// there are too many files, if a path is empty or leaves the folder,
    /// or if a file is named twice.
    pub fn new(summary: &str, contract: &str, files: &[String]) -> Result<Self, DomainError> {
        let summary = summary.trim();
        if summary.is_empty() {
            return Err(DomainError::EmptyText {
                field: "summary of a handoff",
            });
        }
        check_length("summary of a handoff", summary, Self::MAX_SUMMARY_LENGTH)?;
        let contract = contract.trim();
        check_length(
            "interface contract of a handoff",
            contract,
            Self::MAX_CONTRACT_LENGTH,
        )?;
        if files.len() > Self::MOST_FILES {
            return Err(DomainError::TooManyFiles {
                max: Self::MOST_FILES,
                count: files.len(),
            });
        }
        let mut paths: Vec<PathInFolder> = Vec::with_capacity(files.len());
        for file in files {
            let path = PathInFolder::parse(file.trim())?;
            if path.is_folder_itself() {
                return Err(DomainError::EmptyText {
                    field: "path of a file",
                });
            }
            if paths.contains(&path) {
                return Err(DomainError::FileNamedTwice {
                    path: path.as_text(),
                });
            }
            paths.push(path);
        }
        Ok(Self {
            summary: summary.to_owned(),
            contract: contract.to_owned(),
            files: paths,
        })
    }

    /// What was done, for whoever builds on it. Never empty.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The interface contract: what others may rely on. Empty if there is
    /// none.
    #[must_use]
    pub fn contract(&self) -> &str {
        &self.contract
    }

    /// The files that hold the result, inside the work folder of the
    /// project, in the order they were named.
    #[must_use]
    pub fn files(&self) -> &[PathInFolder] {
        &self.files
    }
}

/// The plain data of a handoff, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffSnapshot {
    /// The identifier of the handoff.
    pub id: HandoffId,
    /// The project of the task.
    pub project_id: ProjectId,
    /// The task whose result it is.
    pub task_id: TaskId,
    /// The specialist who published it. It may have left the team since.
    pub author_id: MemberId,
    /// The conversation in which it was published.
    pub conversation_id: ConversationId,
    /// The turn in which it was published.
    pub turn_id: TurnId,
    /// What it says.
    pub content: HandoffContent,
    /// When it was published.
    pub published_at: Timestamp,
}

/// The result of a task, handed over by the specialist who did it to
/// whoever builds on it.
///
/// A handoff never changes once it is published. A task may have several,
/// one from each turn that published one; the latest is the one that
/// counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    id: HandoffId,
    project_id: ProjectId,
    task_id: TaskId,
    author_id: MemberId,
    conversation_id: ConversationId,
    turn_id: TurnId,
    content: HandoffContent,
    published_at: Timestamp,
}

/// Where a handoff or a message was said: who said it, in which
/// conversation and in which turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaidIn {
    /// The specialist.
    pub member_id: MemberId,
    /// The conversation.
    pub conversation_id: ConversationId,
    /// The turn.
    pub turn_id: TurnId,
}

impl Handoff {
    /// Publishes a handoff of a task.
    #[must_use]
    pub fn publish(
        project_id: ProjectId,
        task_id: TaskId,
        said_in: SaidIn,
        content: HandoffContent,
        now: Timestamp,
    ) -> Self {
        Self {
            id: HandoffId::generate(),
            project_id,
            task_id,
            author_id: said_in.member_id,
            conversation_id: said_in.conversation_id,
            turn_id: said_in.turn_id,
            content,
            published_at: now,
        }
    }

    /// Recreates a handoff from stored data.
    #[must_use]
    pub fn restore(snapshot: HandoffSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            task_id: snapshot.task_id,
            author_id: snapshot.author_id,
            conversation_id: snapshot.conversation_id,
            turn_id: snapshot.turn_id,
            content: snapshot.content,
            published_at: snapshot.published_at,
        }
    }

    /// The identifier of the handoff.
    #[must_use]
    pub const fn id(&self) -> HandoffId {
        self.id
    }

    /// The project of the task.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The task whose result it is.
    #[must_use]
    pub const fn task_id(&self) -> TaskId {
        self.task_id
    }

    /// The specialist who published it.
    #[must_use]
    pub const fn author_id(&self) -> MemberId {
        self.author_id
    }

    /// The conversation in which it was published.
    #[must_use]
    pub const fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    /// The turn in which it was published.
    #[must_use]
    pub const fn turn_id(&self) -> TurnId {
        self.turn_id
    }

    /// What it says.
    #[must_use]
    pub const fn content(&self) -> &HandoffContent {
        &self.content
    }

    /// When it was published.
    #[must_use]
    pub const fn published_at(&self) -> Timestamp {
        self.published_at
    }
}

/// What a message is: a question that wants an answer, or a note that
/// only tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    /// The sender wants an answer.
    Question,
    /// The sender only tells something.
    Note,
}

text_enum!(MessageKind, kind = "kind of message", {
    Question => "question",
    Note => "note",
});

/// When and with which turn a message reached its receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageDelivery {
    /// The turn of the receiver that started with it.
    pub turn_id: TurnId,
    /// When that turn started.
    pub at: Timestamp,
}

/// The plain data of a message, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMessageSnapshot {
    /// The identifier of the message.
    pub id: TeamMessageId,
    /// The project of both specialists.
    pub project_id: ProjectId,
    /// Who sent it, and where.
    pub said_in: SaidIn,
    /// Who it is for.
    pub to: MemberId,
    /// A question or a note.
    pub kind: MessageKind,
    /// What it says.
    pub text: String,
    /// When it was sent.
    pub sent_at: Timestamp,
    /// When it reached the receiver, if it has.
    pub delivery: Option<MessageDelivery>,
}

/// A question or a note from one specialist to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMessage {
    id: TeamMessageId,
    project_id: ProjectId,
    said_in: SaidIn,
    to: MemberId,
    kind: MessageKind,
    text: String,
    sent_at: Timestamp,
    delivery: Option<MessageDelivery>,
}

impl TeamMessage {
    /// The most characters of a message.
    pub const MAX_LENGTH: usize = 4_000;

    /// Sends a message to another member of the team. Whitespace around
    /// the text is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if the text is empty or too long, or if the
    /// message is to the sender itself.
    pub fn send(
        project_id: ProjectId,
        said_in: SaidIn,
        to: MemberId,
        kind: MessageKind,
        text: &str,
        now: Timestamp,
    ) -> Result<Self, DomainError> {
        let text = check_message_text(text)?;
        if to == said_in.member_id {
            return Err(DomainError::MessageToOneself);
        }
        Ok(Self {
            id: TeamMessageId::generate(),
            project_id,
            said_in,
            to,
            kind,
            text,
            sent_at: now,
            delivery: None,
        })
    }

    /// Recreates a message from stored data.
    ///
    /// # Errors
    ///
    /// Returns an error if the stored values break a rule of a message.
    pub fn restore(snapshot: TeamMessageSnapshot) -> Result<Self, DomainError> {
        let text = check_message_text(&snapshot.text)?;
        if snapshot.to == snapshot.said_in.member_id {
            return Err(DomainError::MessageToOneself);
        }
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            said_in: snapshot.said_in,
            to: snapshot.to,
            kind: snapshot.kind,
            text,
            sent_at: snapshot.sent_at,
            delivery: snapshot.delivery,
        })
    }

    /// The identifier of the message.
    #[must_use]
    pub const fn id(&self) -> TeamMessageId {
        self.id
    }

    /// The project of both specialists.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Who sent it, in which conversation and turn.
    #[must_use]
    pub const fn said_in(&self) -> SaidIn {
        self.said_in
    }

    /// Who sent it.
    #[must_use]
    pub const fn from(&self) -> MemberId {
        self.said_in.member_id
    }

    /// Who it is for.
    #[must_use]
    pub const fn to(&self) -> MemberId {
        self.to
    }

    /// A question or a note.
    #[must_use]
    pub const fn kind(&self) -> MessageKind {
        self.kind
    }

    /// What it says.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// When it was sent.
    #[must_use]
    pub const fn sent_at(&self) -> Timestamp {
        self.sent_at
    }

    /// When and with which turn it reached the receiver, if it has.
    #[must_use]
    pub const fn delivery(&self) -> Option<MessageDelivery> {
        self.delivery
    }

    /// Whether it still waits for its receiver.
    #[must_use]
    pub const fn is_waiting(&self) -> bool {
        self.delivery.is_none()
    }

    /// Records that the message reached its receiver, with the turn that
    /// started with it. A message is delivered once: delivering it again
    /// changes nothing.
    pub fn deliver(&mut self, turn_id: TurnId, now: Timestamp) {
        if self.delivery.is_none() {
            self.delivery = Some(MessageDelivery { turn_id, at: now });
        }
    }
}

/// Checks the text of a message and removes the whitespace around it.
fn check_message_text(text: &str) -> Result<String, DomainError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DomainError::EmptyText { field: "message" });
    }
    check_length("message", text, TeamMessage::MAX_LENGTH)?;
    Ok(text.to_owned())
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

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_handoff_keeps_its_texts_tidied_and_its_paths_with_forward_slashes() {
        let content = HandoffContent::new(
            "  Pin map v1 is done. ",
            " Display on SPI2. ",
            &files(&["hardware\\pinmap.md", "./firmware/pins.h"]),
        )
        .unwrap();

        assert_eq!(content.summary(), "Pin map v1 is done.");
        assert_eq!(content.contract(), "Display on SPI2.");
        let paths: Vec<String> = content.files().iter().map(PathInFolder::as_text).collect();
        assert_eq!(paths, ["hardware/pinmap.md", "firmware/pins.h"]);
    }

    #[test]
    fn a_handoff_may_have_no_contract_and_no_files() {
        let content = HandoffContent::new("Done.", "", &[]).unwrap();

        assert_eq!(content.contract(), "");
        assert!(content.files().is_empty());
    }

    #[test]
    fn a_handoff_that_says_nothing_or_points_outside_the_folder_is_refused() {
        assert_eq!(
            HandoffContent::new(" ", "", &[]),
            Err(DomainError::EmptyText {
                field: "summary of a handoff"
            })
        );
        for outside in ["../secret.txt", "/etc/passwd", "C:\\notes.txt", "a/../../b"] {
            assert!(
                matches!(
                    HandoffContent::new("Done.", "", &files(&[outside])),
                    Err(DomainError::PathLeavesFolder { .. })
                ),
                "{outside}"
            );
        }
        assert_eq!(
            HandoffContent::new("Done.", "", &files(&["."])),
            Err(DomainError::EmptyText {
                field: "path of a file"
            })
        );
        assert_eq!(
            HandoffContent::new("Done.", "", &files(&["a.md", "./a.md"])),
            Err(DomainError::FileNamedTwice {
                path: "a.md".to_owned()
            })
        );
        let many: Vec<String> = (0..=HandoffContent::MOST_FILES)
            .map(|number| format!("file-{number}.md"))
            .collect();
        assert_eq!(
            HandoffContent::new("Done.", "", &many),
            Err(DomainError::TooManyFiles { max: 50, count: 51 })
        );
        assert!(matches!(
            HandoffContent::new(&"x".repeat(4_001), "", &[]),
            Err(DomainError::TextTooLong { .. })
        ));
    }

    #[test]
    fn a_published_handoff_belongs_to_its_task_and_its_author() {
        let said = said_in();
        let task_id = TaskId::generate();
        let content = HandoffContent::new("Done.", "", &[]).unwrap();

        let handoff = Handoff::publish(ProjectId::generate(), task_id, said, content, now());

        assert_eq!(handoff.task_id(), task_id);
        assert_eq!(handoff.author_id(), said.member_id);
        assert_eq!(handoff.turn_id(), said.turn_id);
        assert_eq!(handoff.published_at(), now());
    }

    #[test]
    fn a_message_waits_until_it_is_delivered_once() {
        let said = said_in();
        let mut message = TeamMessage::send(
            ProjectId::generate(),
            said,
            MemberId::generate(),
            MessageKind::Question,
            "  Can the connector move 3 mm left? ",
            now(),
        )
        .unwrap();
        assert_eq!(message.text(), "Can the connector move 3 mm left?");
        assert!(message.is_waiting());

        let first = TurnId::generate();
        message.deliver(first, now());
        message.deliver(TurnId::generate(), now());

        assert!(!message.is_waiting());
        assert_eq!(
            message.delivery().map(|delivery| delivery.turn_id),
            Some(first)
        );
    }

    #[test]
    fn a_message_to_oneself_or_without_words_is_refused() {
        let said = said_in();
        let send = |to: MemberId, text: &str| {
            TeamMessage::send(
                ProjectId::generate(),
                said,
                to,
                MessageKind::Note,
                text,
                now(),
            )
        };

        assert_eq!(
            send(said.member_id, "Hello"),
            Err(DomainError::MessageToOneself)
        );
        assert_eq!(
            send(MemberId::generate(), "  "),
            Err(DomainError::EmptyText { field: "message" })
        );
        assert!(send(MemberId::generate(), &"x".repeat(4_001)).is_err());
    }

    #[test]
    fn the_kind_of_a_message_has_a_stable_text() {
        assert_eq!(MessageKind::Question.as_str(), "question");
        assert_eq!("note".parse::<MessageKind>(), Ok(MessageKind::Note));
        assert!("letter".parse::<MessageKind>().is_err());
    }
}
