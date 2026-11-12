//! Keeping several computers of one manager in step, through a folder the
//! manager owns
//! ([decision 0017](../../../docs/decisions/0017-sync-through-a-folder.md)).
//!
//! Every computer writes its own operations into its own files in the
//! folder and takes in the operations of the computers the manager
//! accepted. The application only hands operations from one side to the
//! other; it never looks inside the state an operation carries.

use std::fmt;
use std::str::FromStr;

use teamwright_domain::{
    ComputerId, ConversationId, DomainError, ProjectId, TaskId, Timestamp, TurnId,
};

/// One operation of the log of a computer, as it travels between
/// computers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangedOperation {
    /// The computer that made the change.
    pub computer: ComputerId,
    /// The number of the operation on that computer, counted from 1.
    pub sequence: u64,
    /// The stamp of the hybrid logical clock of the change. Stamps sort as
    /// text in the order the changes happened.
    pub stamp: String,
    /// The kind of thing that changed, as the log names it.
    pub entity: String,
    /// The identifier of the thing that changed.
    pub entity_id: String,
    /// What happened to it.
    pub change: ExchangedChange,
}

/// What an operation did to its thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangedChange {
    /// The thing was created or changed.
    Upsert {
        /// Its whole state, as JSON.
        state: String,
        /// The stamps of the fields of the state that are older than the
        /// operation, as a JSON object from the name of the field to its
        /// stamp, or nothing if every field has the stamp of the operation.
        field_stamps: Option<String>,
    },
    /// The thing was deleted.
    Delete {
        /// For each computer, the latest change of the thing the deleting
        /// computer had seen, as a JSON object from the identity of the
        /// computer to the stamp, or nothing if it had seen none.
        seen: Option<String>,
    },
}

/// What taking in a batch of operations did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TakeInReport {
    /// How many of the operations were new to this computer.
    pub taken_in: usize,
    /// The projects in which something changed.
    pub changed_projects: Vec<ProjectId>,
    /// The conversations that changed.
    pub changed_conversations: Vec<ConversationId>,
    /// The turns that ran on this computer and were deleted by another
    /// computer. Their programs are to be stopped.
    pub turns_to_stop: Vec<TurnId>,
    /// How many notices for the manager taking in added.
    pub notices: usize,
    /// How far, in milliseconds, the latest stamp taken in is ahead of the
    /// time of day of this computer, if it is ahead at all.
    pub clock_ahead_by_millis: Option<i64>,
}

/// What a notice of syncing tells the manager about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoticeKind {
    /// A task got a new number, because another computer had given its
    /// number to another task at the same time.
    TaskRenumbered,
    /// A project got a new key, because another computer had given its key
    /// to another project at the same time.
    ProjectRekeyed,
    /// A change made on this computer was dropped, because its thing was
    /// deleted on another computer.
    ChangeDropped,
    /// A task moved to another status, because its status was removed on
    /// another computer.
    TaskMoved,
    /// The clock of another computer is ahead of this computer's.
    ClockAhead,
}

impl NoticeKind {
    /// The stable text form of the kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskRenumbered => "task_renumbered",
            Self::ProjectRekeyed => "project_rekeyed",
            Self::ChangeDropped => "change_dropped",
            Self::TaskMoved => "task_moved",
            Self::ClockAhead => "clock_ahead",
        }
    }
}

impl fmt::Display for NoticeKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for NoticeKind {
    type Err = DomainError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        [
            Self::TaskRenumbered,
            Self::ProjectRekeyed,
            Self::ChangeDropped,
            Self::TaskMoved,
            Self::ClockAhead,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
        .ok_or_else(|| DomainError::UnknownValue {
            kind: "kind of notice",
            value: text.to_owned(),
        })
    }
}

/// Something syncing tells the manager, kept on this computer until the
/// manager dismisses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncNotice {
    /// The number of the notice on this computer.
    pub id: i64,
    /// When it was made.
    pub at: Timestamp,
    /// What it is about.
    pub kind: NoticeKind,
    /// The project it concerns, if one.
    pub project_id: Option<ProjectId>,
    /// The task it concerns, if one.
    pub task_id: Option<TaskId>,
    /// What happened, in words.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_of_notice_survives_its_text_form() {
        for kind in [
            NoticeKind::TaskRenumbered,
            NoticeKind::ProjectRekeyed,
            NoticeKind::ChangeDropped,
            NoticeKind::TaskMoved,
            NoticeKind::ClockAhead,
        ] {
            assert_eq!(kind.to_string().parse::<NoticeKind>().unwrap(), kind);
        }
        assert!("weather".parse::<NoticeKind>().is_err());
    }
}
