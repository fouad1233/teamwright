//! Keeping several computers of one manager in step, through a folder the
//! manager owns
//! ([decision 0017](../../../docs/decisions/0017-sync-through-a-folder.md)).
//!
//! Every computer writes its own operations into its own files in the
//! folder and takes in the operations of the computers the manager
//! accepted. The application only hands operations from one side to the
//! other; it never looks inside the state an operation carries.

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use teamwright_domain::{
    ComputerId, ComputerName, ConversationId, DomainError, ProjectId, TaskId, Timestamp, TurnId,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, FileSystem, StorageError};

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

/// The version of the format of the files in the sync folder. It changes
/// only when the meaning of an existing field changes or a field is
/// removed; adding a field or an entity does not change it.
pub const SYNC_FORMAT: u32 = 1;

/// What a computer says about itself in the sync folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerCard {
    /// Its identity.
    pub id: ComputerId,
    /// The name the manager gave it, such as "Desk".
    pub name: String,
    /// The version of Teamwright it runs.
    pub version: String,
    /// The format of its files.
    pub format: u32,
    /// When it last wrote into the folder.
    pub written_at: Timestamp,
}

/// A computer that has a folder of its own in the sync folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundComputer {
    /// Its identity, from the name of its folder.
    pub id: ComputerId,
    /// What it says about itself, or nothing if its card is missing or
    /// cannot be read (yet).
    pub card: Option<ComputerCard>,
}

/// What a file of operations in the sync folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileContent {
    /// The whole file, with its operations in order.
    Complete(Vec<ExchangedOperation>),
    /// The file is not whole yet, or not there any more: a sync program is
    /// still copying it, or it was cut off. It is looked at again next
    /// time.
    NotYet,
    /// The file was written in a newer format than this version reads.
    NewerFormat(u32),
}

/// The folder that computers keep in step through. Each computer writes
/// only below its own folder, `devices/<identity>/`, and never changes a
/// file once its name appears.
pub trait SyncFolder: Send + Sync {
    /// Writes the card of a computer, replacing the one it had.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be written.
    fn write_card(&self, root: &Path, card: &ComputerCard) -> Result<(), StorageError>;

    /// Writes operations of a computer into a new file, and returns its
    /// name. The operations are in order and follow each other.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be written.
    fn write_file(
        &self,
        root: &Path,
        computer: ComputerId,
        operations: &[ExchangedOperation],
    ) -> Result<String, StorageError>;

    /// The computers that have a folder in the sync folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read.
    fn computers(&self, root: &Path) -> Result<Vec<FoundComputer>, StorageError>;

    /// The names of the files of a computer that hold an operation numbered
    /// above `after`, in order.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read.
    fn files_of(
        &self,
        root: &Path,
        computer: ComputerId,
        after: u64,
    ) -> Result<Vec<String>, StorageError>;

    /// Reads one file of a computer.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read. A file that is not
    /// whole is not an error but [`FileContent::NotYet`].
    fn read_file(
        &self,
        root: &Path,
        computer: ComputerId,
        name: &str,
    ) -> Result<FileContent, StorageError>;

    /// Joins the small files of a computer into one, when there are many,
    /// and removes them. Returns the name of the new file, if one was
    /// written.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read or written.
    fn join_small_files(
        &self,
        root: &Path,
        computer: ComputerId,
    ) -> Result<Option<String>, StorageError>;
}

/// How long the clock of another computer may be ahead of this computer's
/// before the manager is told: five minutes.
pub const CLOCK_AHEAD_LIMIT_MILLIS: i64 = 5 * 60 * 1000;

/// How many operations are taken in in one transaction at most.
const BATCH: usize = 500;

/// How many operations one file holds at most when this computer writes.
const OPERATIONS_PER_FILE: usize = 2000;

/// Whether the manager wants the operations of a computer taken in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Acceptance {
    /// Not decided yet: the computer was found in the folder and the
    /// manager has not answered.
    Undecided,
    /// Its operations are taken in.
    Accepted,
    /// Its operations are not taken in.
    Ignored,
}

/// What this computer knows of another computer in the sync folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownComputer {
    /// Its identity.
    pub id: ComputerId,
    /// The name it gave itself, once its card was read.
    pub name: Option<String>,
    /// The version of Teamwright it runs, once its card was read.
    pub version: Option<String>,
    /// Whether its operations are taken in.
    pub acceptance: Acceptance,
    /// The number of the last of its operations taken in.
    pub read_up_to: u64,
    /// When it last wrote into the folder, by its card.
    pub last_heard: Option<Timestamp>,
    /// How far its clock was ahead of this computer's, if it was ahead
    /// more than [`CLOCK_AHEAD_LIMIT_MILLIS`].
    pub clock_ahead_by_millis: Option<i64>,
    /// The format of a file it wrote that this version cannot read, if it
    /// wrote one. Nothing of it is read from that file on.
    pub newer_format: Option<u32>,
}

/// The settings of syncing on this computer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncSettings {
    /// The folder computers keep in step through, or nothing while this
    /// computer does not sync.
    pub folder: Option<PathBuf>,
    /// The name of this computer.
    pub computer_name: Option<String>,
    /// The number of the last operation of this computer written into the
    /// folder.
    pub written_through: u64,
}

/// The bookkeeping of syncing, and the log, as storage keeps them. None of
/// it is logged.
pub trait SyncLedger: Send + Sync {
    /// The identity of this computer.
    fn this_computer(&self) -> ComputerId;

    /// This computer's operations after the one numbered `after`, in order,
    /// without the folders of the projects.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    fn operations_after(&self, after: u64) -> Result<Vec<ExchangedOperation>, StorageError>;

    /// When the latest operation of this computer that is not written into
    /// the folder yet was made, if there is one.
    ///
    /// # Errors
    ///
    /// Returns an error if the log cannot be read.
    fn unwritten_since(&self) -> Result<Option<Timestamp>, StorageError>;

    /// Takes in operations of another computer, and moves the number of
    /// its last operation taken in to `read_up_to`, in one transaction.
    ///
    /// # Errors
    ///
    /// Returns an error, and keeps nothing, if storage fails.
    fn take_in(
        &self,
        from: ComputerId,
        operations: &[ExchangedOperation],
        read_up_to: u64,
    ) -> Result<TakeInReport, StorageError>;

    /// Before anything is taken in for the first time, keeps a copy of the
    /// database next to it. Afterwards it does nothing.
    ///
    /// # Errors
    ///
    /// Returns an error if the copy cannot be made.
    fn keep_copy_before_first_take_in(&self) -> Result<(), StorageError>;

    /// The other computers this computer knows of.
    ///
    /// # Errors
    ///
    /// Returns an error if they cannot be read.
    fn computers(&self) -> Result<Vec<KnownComputer>, StorageError>;

    /// Remembers a computer found in the folder, with what its card says,
    /// and says whether it is new. A new computer is undecided.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn heard_from(&self, computer: &FoundComputer) -> Result<bool, StorageError>;

    /// Records whether the operations of a computer are taken in.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such computer or if storage fails.
    fn set_acceptance(
        &self,
        computer: ComputerId,
        acceptance: Acceptance,
    ) -> Result<(), StorageError>;

    /// Records that a computer wrote a file in a newer format.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn mark_newer_format(&self, computer: ComputerId, format: u32) -> Result<(), StorageError>;

    /// Records how far the clock of a computer is ahead, or that it is not.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn set_clock_ahead(
        &self,
        computer: ComputerId,
        ahead_by_millis: Option<i64>,
    ) -> Result<(), StorageError>;

    /// The names of the files this computer wrote into the folder.
    ///
    /// # Errors
    ///
    /// Returns an error if they cannot be read.
    fn written_files(&self) -> Result<Vec<String>, StorageError>;

    /// Remembers a file this computer wrote, and the number of the last
    /// operation written.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn remember_written_file(&self, name: &str, written_through: u64) -> Result<(), StorageError>;

    /// The settings of syncing.
    ///
    /// # Errors
    ///
    /// Returns an error if they cannot be read.
    fn settings(&self) -> Result<SyncSettings, StorageError>;

    /// Chooses the folder, or stops syncing with nothing. A folder other
    /// than the one the files written so far are in starts anew: nothing
    /// is written there yet.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn set_folder(&self, folder: Option<&Path>) -> Result<(), StorageError>;

    /// Names this computer.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn set_computer_name(&self, name: &str) -> Result<(), StorageError>;

    /// The notices kept for the manager, the newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if they cannot be read.
    fn notices(&self) -> Result<Vec<SyncNotice>, StorageError>;

    /// Keeps a notice for the manager.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn add_notice(&self, kind: NoticeKind, message: &str) -> Result<(), StorageError>;

    /// Forgets a notice.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be removed.
    fn dismiss_notice(&self, id: i64) -> Result<(), StorageError>;

    /// Gives this computer a new identity, which takes effect when
    /// Teamwright starts again. The old identity becomes another computer
    /// whose operations are taken in like any other, from after the last
    /// one this computer has; nothing is written under the new identity
    /// yet.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be written.
    fn new_identity(&self) -> Result<ComputerId, StorageError>;
}

/// Where syncing stands, in one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncPhase {
    /// No folder is chosen.
    Off,
    /// The last round went well.
    InStep,
    /// A round is writing or reading right now.
    Writing,
    /// The folder could not be reached in the last round.
    Unreachable,
    /// Syncing stopped: another computer uses this computer's identity, or
    /// a new identity waits for Teamwright to start again.
    Stopped,
}

impl SyncPhase {
    /// The stable text form of the phase.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::InStep => "in_step",
            Self::Writing => "writing",
            Self::Unreachable => "unreachable",
            Self::Stopped => "stopped",
        }
    }
}

/// Everything the manager sees of syncing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncState {
    /// The chosen folder, if syncing is on.
    pub folder: Option<PathBuf>,
    /// The identity of this computer.
    pub this_computer: ComputerId,
    /// The name of this computer, once the manager gave one.
    pub computer_name: Option<String>,
    /// The other computers.
    pub computers: Vec<KnownComputer>,
    /// Where syncing stands.
    pub phase: SyncPhase,
    /// When the last round ended, in this run of the program.
    pub last_round: Option<Timestamp>,
    /// What went wrong in the last round, in words.
    pub problem: Option<String>,
}

/// What one round did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoundReport {
    /// How many operations of this computer were written into the folder.
    pub written: usize,
    /// How many operations of other computers were new here.
    pub taken_in: usize,
    /// The projects in which something changed.
    pub changed_projects: Vec<ProjectId>,
    /// The conversations that changed.
    pub changed_conversations: Vec<ConversationId>,
    /// The turns that ran here and were deleted elsewhere; their programs
    /// are to be stopped.
    pub turns_to_stop: Vec<TurnId>,
    /// The computers found in the folder for the first time.
    pub new_computers: Vec<ComputerId>,
    /// The computers that wrote a file this version cannot read.
    pub newer_formats: Vec<ComputerId>,
}

/// Everything syncing needs from the outside world.
pub struct SyncPorts {
    /// The bookkeeping and the log.
    pub ledger: Arc<dyn SyncLedger>,
    /// The files in the sync folder.
    pub folder: Arc<dyn SyncFolder>,
    /// The folders of the computer, to check the chosen one.
    pub file_system: Arc<dyn FileSystem>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The version of Teamwright, which the card of this computer names.
    pub version: String,
    /// The folder Teamwright keeps its own data in. The sync folder may not
    /// lie inside it.
    pub data_folder: PathBuf,
}

/// The use cases of keeping several computers in step.
///
/// A round writes this computer's new operations into the folder and takes
/// in those of the computers the manager accepted. Two rounds never run at
/// once.
#[derive(Clone)]
pub struct SyncService {
    inner: Arc<SyncInner>,
}

/// The parts of the service, shared by the threads that use it.
struct SyncInner {
    ports: SyncPorts,
    /// Held for the whole of a round.
    round: Mutex<()>,
    /// Where syncing stands, readable while a round runs.
    status: Mutex<Status>,
}

/// Where syncing stands, in this run of the program.
#[derive(Debug, Clone, Default)]
struct Status {
    /// Set while a round runs.
    running: bool,
    /// Set once this computer got a new identity: syncing waits for
    /// Teamwright to start again with it.
    restart_needed: bool,
    /// What the last round ended with, if it ended with a problem.
    stopped: Option<SyncPhase>,
    /// When the last round ended.
    last_round: Option<Timestamp>,
    /// What went wrong in the last round, in words.
    problem: Option<String>,
}

impl SyncService {
    /// Creates the service.
    #[must_use]
    pub fn new(ports: SyncPorts) -> Self {
        Self {
            inner: Arc::new(SyncInner {
                ports,
                round: Mutex::new(()),
                status: Mutex::new(Status::default()),
            }),
        }
    }

    /// Where syncing stands, with the other computers.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn state(&self) -> Result<SyncState, ApplicationError> {
        let ledger = &self.inner.ports.ledger;
        let settings = ledger.settings()?;
        let status = lock(&self.inner.status).clone();
        let phase = if status.restart_needed {
            SyncPhase::Stopped
        } else if settings.folder.is_none() {
            SyncPhase::Off
        } else if status.running {
            SyncPhase::Writing
        } else {
            status.stopped.unwrap_or(SyncPhase::InStep)
        };
        Ok(SyncState {
            folder: settings.folder,
            this_computer: ledger.this_computer(),
            computer_name: settings.computer_name,
            computers: ledger.computers()?,
            phase,
            last_round: status.last_round,
            problem: status.problem,
        })
    }

    /// Chooses the folder to keep computers in step through, and names this
    /// computer. The first round writes the whole log into it.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not named (`rule_broken`), does not
    /// exist (`folder_not_found`) or lies inside Teamwright's own data
    /// folder (`sync_folder_in_data`), if the name is empty or too long
    /// (`rule_broken`), or if storage fails.
    pub fn choose_folder(&self, folder: &Path, name: &str) -> Result<SyncState, ApplicationError> {
        let ports = &self.inner.ports;
        let name = ComputerName::parse(name)?;
        if folder.as_os_str().is_empty() {
            return Err(ApplicationError::FolderNotNamed);
        }
        if !ports.file_system.is_folder(folder) {
            return Err(ApplicationError::FolderNotFound {
                folder: folder.display().to_string(),
            });
        }
        let chosen = ports.file_system.resolve(folder)?;
        let data = if ports.file_system.is_folder(&ports.data_folder) {
            ports.file_system.resolve(&ports.data_folder)?
        } else {
            ports.data_folder.clone()
        };
        if chosen.starts_with(&data) {
            return Err(ApplicationError::SyncFolderInDataFolder {
                folder: folder.display().to_string(),
            });
        }
        let _round = lock(&self.inner.round);
        ports.ledger.set_computer_name(name.as_str())?;
        ports.ledger.set_folder(Some(&chosen))?;
        lock(&self.inner.status).stopped = None;
        self.state()
    }

    /// Stops syncing. Nothing in the folder is removed, and choosing the
    /// same folder again goes on where it stopped.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn stop(&self) -> Result<SyncState, ApplicationError> {
        let _round = lock(&self.inner.round);
        self.inner.ports.ledger.set_folder(None)?;
        *lock(&self.inner.status) = Status::default();
        self.state()
    }

    /// Gives this computer another name, which its card says from the next
    /// round on.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty or too long (`rule_broken`), or
    /// if storage fails.
    pub fn rename_this_computer(&self, name: &str) -> Result<SyncState, ApplicationError> {
        let name = ComputerName::parse(name)?;
        self.inner.ports.ledger.set_computer_name(name.as_str())?;
        self.state()
    }

    /// Takes in the operations of a computer from the next round on.
    ///
    /// # Errors
    ///
    /// Returns an error if the computer is not known (`not_found`) or if
    /// storage fails.
    pub fn accept(&self, computer: ComputerId) -> Result<SyncState, ApplicationError> {
        self.set_acceptance(computer, Acceptance::Accepted)
    }

    /// Takes in nothing more of a computer. What was taken in stays.
    ///
    /// # Errors
    ///
    /// Returns an error if the computer is not known (`not_found`) or if
    /// storage fails.
    pub fn ignore(&self, computer: ComputerId) -> Result<SyncState, ApplicationError> {
        self.set_acceptance(computer, Acceptance::Ignored)
    }

    fn set_acceptance(
        &self,
        computer: ComputerId,
        acceptance: Acceptance,
    ) -> Result<SyncState, ApplicationError> {
        let ledger = &self.inner.ports.ledger;
        if !ledger.computers()?.iter().any(|known| known.id == computer) {
            return Err(ApplicationError::NotFound {
                kind: "computer",
                id: computer.to_string(),
            });
        }
        ledger.set_acceptance(computer, acceptance)?;
        self.state()
    }

    /// The notices kept for the manager, the newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn notices(&self) -> Result<Vec<SyncNotice>, ApplicationError> {
        Ok(self.inner.ports.ledger.notices()?)
    }

    /// Forgets a notice. A notice that is not kept is ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn dismiss(&self, notice: i64) -> Result<(), ApplicationError> {
        Ok(self.inner.ports.ledger.dismiss_notice(notice)?)
    }

    /// Gives this computer a new identity, for when its data folder was
    /// copied to another computer, which now uses the same identity.
    /// Syncing stops until Teamwright starts again with the new identity;
    /// the old identity then becomes another computer whose operations are
    /// taken in like any other, so nothing is lost.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn new_identity(&self) -> Result<ComputerId, ApplicationError> {
        let _round = lock(&self.inner.round);
        let identity = self.inner.ports.ledger.new_identity()?;
        let mut status = lock(&self.inner.status);
        status.restart_needed = true;
        status.stopped = Some(SyncPhase::Stopped);
        status.problem =
            Some("This computer has a new identity. Start Teamwright again to go on.".to_owned());
        Ok(identity)
    }

    /// When the latest change of this computer that is not written into
    /// the folder yet was made, if syncing is on and there is one. The
    /// program uses it to decide when to run a round.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn unwritten_since(&self) -> Result<Option<Timestamp>, ApplicationError> {
        let ledger = &self.inner.ports.ledger;
        if ledger.settings()?.folder.is_none() {
            return Ok(None);
        }
        Ok(ledger.unwritten_since()?)
    }

    /// Runs one round: writes the operations of this computer that are not
    /// in the folder yet, its card, joins its small files, and takes in the
    /// new operations of every accepted computer. A computer found for the
    /// first time is remembered as undecided and reported.
    ///
    /// # Errors
    ///
    /// Returns an error if no folder is chosen (`sync_off`), if the folder
    /// cannot be reached (`sync_folder_unreachable`), if another computer
    /// uses the identity of this one (`identity_in_use`), if syncing waits
    /// for Teamwright to start again (`identity_in_use`), or if storage
    /// fails.
    pub fn round(&self) -> Result<RoundReport, ApplicationError> {
        let _round = lock(&self.inner.round);
        if lock(&self.inner.status).restart_needed {
            return Err(ApplicationError::IdentityInUse);
        }
        lock(&self.inner.status).running = true;
        let result = self.run_round();
        let now = self.inner.ports.clock.now();
        let mut status = lock(&self.inner.status);
        status.running = false;
        status.last_round = Some(now);
        match &result {
            Ok(_) => {
                status.stopped = None;
                status.problem = None;
            }
            Err(error) => {
                status.stopped = Some(match error {
                    ApplicationError::IdentityInUse => SyncPhase::Stopped,
                    ApplicationError::SyncOff => SyncPhase::Off,
                    _ => SyncPhase::Unreachable,
                });
                status.problem = Some(error.to_string());
            }
        }
        result
    }

    /// The work of a round. The caller holds the lock on rounds.
    fn run_round(&self) -> Result<RoundReport, ApplicationError> {
        let ports = &self.inner.ports;
        let ledger = &ports.ledger;
        let settings = ledger.settings()?;
        let root = settings.folder.ok_or(ApplicationError::SyncOff)?;
        if !ports.file_system.is_folder(&root) {
            return Err(ApplicationError::SyncFolderUnreachable {
                folder: root.display().to_string(),
            });
        }
        let unreachable = |error: StorageError| ApplicationError::SyncFolderUnreachable {
            folder: format!("{} ({error})", root.display()),
        };
        let me = ledger.this_computer();
        let mut report = RoundReport::default();

        // A file in this computer's folder that it did not write means that
        // another computer uses its identity.
        let written: BTreeSet<String> = ledger.written_files()?.into_iter().collect();
        let in_folder = ports.folder.files_of(&root, me, 0).map_err(unreachable)?;
        if in_folder.iter().any(|name| !written.contains(name)) {
            return Err(ApplicationError::IdentityInUse);
        }

        let unwritten = ledger.operations_after(settings.written_through)?;
        for part in unwritten.chunks(OPERATIONS_PER_FILE) {
            let Some(last) = part.last() else {
                continue;
            };
            let name = ports
                .folder
                .write_file(&root, me, part)
                .map_err(unreachable)?;
            ledger.remember_written_file(&name, last.sequence)?;
            report.written += part.len();
        }
        let card = ComputerCard {
            id: me,
            name: settings.computer_name.unwrap_or_default(),
            version: ports.version.clone(),
            format: SYNC_FORMAT,
            written_at: ports.clock.now(),
        };
        ports.folder.write_card(&root, &card).map_err(unreachable)?;
        if let Some(joined) = ports
            .folder
            .join_small_files(&root, me)
            .map_err(unreachable)?
        {
            let through = ledger.settings()?.written_through;
            ledger.remember_written_file(&joined, through)?;
        }

        let known = ledger.computers()?;
        for found in ports.folder.computers(&root).map_err(unreachable)? {
            if found.id == me {
                continue;
            }
            if ledger.heard_from(&found)? {
                report.new_computers.push(found.id);
            }
            let Some(computer) = known.iter().find(|known| known.id == found.id) else {
                continue;
            };
            if computer.acceptance != Acceptance::Accepted || computer.newer_format.is_some() {
                continue;
            }
            self.read_computer(&root, computer, &mut report)?;
        }
        report.changed_projects.sort();
        report.changed_projects.dedup();
        report.changed_conversations.sort();
        report.changed_conversations.dedup();
        Ok(report)
    }

    /// Takes in the new complete files of one accepted computer, in order,
    /// up to the first file that is not whole yet.
    fn read_computer(
        &self,
        root: &Path,
        computer: &KnownComputer,
        report: &mut RoundReport,
    ) -> Result<(), ApplicationError> {
        let ports = &self.inner.ports;
        let unreachable = |error: StorageError| ApplicationError::SyncFolderUnreachable {
            folder: format!("{} ({error})", root.display()),
        };
        let mut read_up_to = computer.read_up_to;
        let mut ahead: Option<i64> = None;
        for name in ports
            .folder
            .files_of(root, computer.id, read_up_to)
            .map_err(unreachable)?
        {
            let operations = match ports
                .folder
                .read_file(root, computer.id, &name)
                .map_err(unreachable)?
            {
                FileContent::Complete(operations) => operations,
                // A later file would leave a gap: the rest waits for the
                // next round.
                FileContent::NotYet => break,
                FileContent::NewerFormat(format) => {
                    ports.ledger.mark_newer_format(computer.id, format)?;
                    report.newer_formats.push(computer.id);
                    break;
                }
            };
            let new: Vec<ExchangedOperation> = operations
                .into_iter()
                .filter(|operation| operation.sequence > read_up_to)
                .collect();
            for batch in new.chunks(BATCH) {
                let Some(last) = batch.last() else {
                    continue;
                };
                ports.ledger.keep_copy_before_first_take_in()?;
                let taken = ports.ledger.take_in(computer.id, batch, last.sequence)?;
                read_up_to = last.sequence;
                report.taken_in += taken.taken_in;
                report.changed_projects.extend(taken.changed_projects);
                report
                    .changed_conversations
                    .extend(taken.changed_conversations);
                report.turns_to_stop.extend(taken.turns_to_stop);
                ahead = ahead.max(taken.clock_ahead_by_millis);
            }
        }
        let ahead = ahead.filter(|ahead| *ahead > CLOCK_AHEAD_LIMIT_MILLIS);
        if ahead.is_some() && computer.clock_ahead_by_millis.is_none() {
            let name = computer.name.as_deref().unwrap_or("Another computer");
            ports.ledger.add_notice(
                NoticeKind::ClockAhead,
                &format!(
                    "The clock of {name} is ahead of this computer's. Changes made there may \
                     win over later changes made here; set the time on both computers."
                ),
            )?;
        }
        if ahead.is_some() || computer.clock_ahead_by_millis.is_some() {
            ports.ledger.set_clock_ahead(computer.id, ahead)?;
        }
        Ok(())
    }
}

/// Locks a mutex, and keeps working even if another thread panicked while it
/// held the lock: what the mutexes here guard stays valid between single
/// operations.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod kinds_of_notices {
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
