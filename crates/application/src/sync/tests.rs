//! A round of syncing with stand-ins for the folder and the ledger.

use std::collections::BTreeMap;

use chrono::TimeZone;

use super::*;
use crate::in_memory::{ManualClock, PretendFileSystem};

/// The bookkeeping of syncing, kept in memory.
#[derive(Default)]
struct Ledger {
    this_computer: Option<ComputerId>,
    state: Mutex<LedgerState>,
}

#[derive(Default)]
struct LedgerState {
    operations: Vec<ExchangedOperation>,
    taken_in: Vec<ExchangedOperation>,
    computers: Vec<KnownComputer>,
    written_files: Vec<String>,
    settings: SyncSettings,
    notices: Vec<SyncNotice>,
    copies: usize,
}

impl Ledger {
    fn state(&self) -> MutexGuard<'_, LedgerState> {
        lock(&self.state)
    }
}

impl SyncLedger for Ledger {
    fn this_computer(&self) -> ComputerId {
        self.this_computer.unwrap_or_else(ComputerId::generate)
    }

    fn operations_after(&self, after: u64) -> Result<Vec<ExchangedOperation>, StorageError> {
        Ok(self
            .state()
            .operations
            .iter()
            .filter(|operation| operation.sequence > after)
            .cloned()
            .collect())
    }

    fn unwritten_since(&self) -> Result<Option<Timestamp>, StorageError> {
        Ok(None)
    }

    fn take_in(
        &self,
        from: ComputerId,
        operations: &[ExchangedOperation],
        read_up_to: u64,
    ) -> Result<TakeInReport, StorageError> {
        let mut state = self.state();
        state.taken_in.extend_from_slice(operations);
        if let Some(computer) = state.computers.iter_mut().find(|known| known.id == from) {
            computer.read_up_to = read_up_to;
        }
        Ok(TakeInReport {
            taken_in: operations.len(),
            ..TakeInReport::default()
        })
    }

    fn keep_copy_before_first_take_in(&self) -> Result<(), StorageError> {
        self.state().copies += 1;
        Ok(())
    }

    fn computers(&self) -> Result<Vec<KnownComputer>, StorageError> {
        Ok(self.state().computers.clone())
    }

    fn heard_from(&self, computer: &FoundComputer) -> Result<bool, StorageError> {
        let mut state = self.state();
        if state.computers.iter().any(|known| known.id == computer.id) {
            return Ok(false);
        }
        state.computers.push(KnownComputer {
            id: computer.id,
            name: computer.card.as_ref().map(|card| card.name.clone()),
            version: None,
            acceptance: Acceptance::Undecided,
            read_up_to: 0,
            last_heard: None,
            clock_ahead_by_millis: None,
            newer_format: None,
        });
        Ok(true)
    }

    fn set_acceptance(
        &self,
        computer: ComputerId,
        acceptance: Acceptance,
    ) -> Result<(), StorageError> {
        for known in &mut self.state().computers {
            if known.id == computer {
                known.acceptance = acceptance;
            }
        }
        Ok(())
    }

    fn mark_newer_format(&self, computer: ComputerId, format: u32) -> Result<(), StorageError> {
        for known in &mut self.state().computers {
            if known.id == computer {
                known.newer_format = Some(format);
            }
        }
        Ok(())
    }

    fn set_clock_ahead(
        &self,
        computer: ComputerId,
        ahead_by_millis: Option<i64>,
    ) -> Result<(), StorageError> {
        for known in &mut self.state().computers {
            if known.id == computer {
                known.clock_ahead_by_millis = ahead_by_millis;
            }
        }
        Ok(())
    }

    fn written_files(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.state().written_files.clone())
    }

    fn remember_written_file(&self, name: &str, written_through: u64) -> Result<(), StorageError> {
        let mut state = self.state();
        state.written_files.push(name.to_owned());
        state.settings.written_through = written_through;
        Ok(())
    }

    fn settings(&self) -> Result<SyncSettings, StorageError> {
        Ok(self.state().settings.clone())
    }

    fn set_folder(&self, folder: Option<&Path>) -> Result<(), StorageError> {
        self.state().settings.folder = folder.map(Path::to_path_buf);
        Ok(())
    }

    fn set_computer_name(&self, name: &str) -> Result<(), StorageError> {
        self.state().settings.computer_name = Some(name.to_owned());
        Ok(())
    }

    fn notices(&self) -> Result<Vec<SyncNotice>, StorageError> {
        Ok(self.state().notices.clone())
    }

    fn add_notice(&self, kind: NoticeKind, message: &str) -> Result<(), StorageError> {
        let mut state = self.state();
        let id = i64::try_from(state.notices.len()).unwrap_or_default();
        state.notices.push(SyncNotice {
            id,
            at: moment(),
            kind,
            project_id: None,
            task_id: None,
            message: message.to_owned(),
        });
        Ok(())
    }

    fn dismiss_notice(&self, id: i64) -> Result<(), StorageError> {
        self.state().notices.retain(|notice| notice.id != id);
        Ok(())
    }

    fn new_identity(&self) -> Result<ComputerId, StorageError> {
        Ok(ComputerId::generate())
    }
}

/// The files of the sync folder, kept in memory.
#[derive(Default)]
struct Folder {
    files: Mutex<BTreeMap<ComputerId, Vec<(String, FileContent)>>>,
    cards: Mutex<BTreeMap<ComputerId, ComputerCard>>,
}

impl Folder {
    fn put(&self, computer: ComputerId, name: &str, content: FileContent) {
        lock(&self.files)
            .entry(computer)
            .or_default()
            .push((name.to_owned(), content));
    }

    fn files(&self, computer: ComputerId) -> Vec<(String, FileContent)> {
        lock(&self.files)
            .get(&computer)
            .cloned()
            .unwrap_or_default()
    }
}

impl SyncFolder for Folder {
    fn write_card(&self, _root: &Path, card: &ComputerCard) -> Result<(), StorageError> {
        lock(&self.cards).insert(card.id, card.clone());
        Ok(())
    }

    fn write_file(
        &self,
        _root: &Path,
        computer: ComputerId,
        operations: &[ExchangedOperation],
    ) -> Result<String, StorageError> {
        let first = operations.first().map_or(0, |operation| operation.sequence);
        let last = operations.last().map_or(0, |operation| operation.sequence);
        let name = format!("{first:012}-{last:012}.jsonl");
        self.put(computer, &name, FileContent::Complete(operations.to_vec()));
        Ok(name)
    }

    fn computers(&self, _root: &Path) -> Result<Vec<FoundComputer>, StorageError> {
        Ok(lock(&self.files)
            .keys()
            .map(|id| FoundComputer {
                id: *id,
                card: lock(&self.cards).get(id).cloned(),
            })
            .collect())
    }

    fn files_of(
        &self,
        _root: &Path,
        computer: ComputerId,
        after: u64,
    ) -> Result<Vec<String>, StorageError> {
        Ok(self
            .files(computer)
            .into_iter()
            .filter(|(name, _)| {
                name.get(13..25)
                    .and_then(|last| last.parse::<u64>().ok())
                    .is_some_and(|last| last > after)
            })
            .map(|(name, _)| name)
            .collect())
    }

    fn read_file(
        &self,
        _root: &Path,
        computer: ComputerId,
        name: &str,
    ) -> Result<FileContent, StorageError> {
        Ok(self
            .files(computer)
            .into_iter()
            .find(|(file, _)| file == name)
            .map_or(FileContent::NotYet, |(_, content)| content))
    }

    fn join_small_files(
        &self,
        _root: &Path,
        _computer: ComputerId,
    ) -> Result<Option<String>, StorageError> {
        Ok(None)
    }
}

fn moment() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap()
}

fn operation(computer: ComputerId, sequence: u64) -> ExchangedOperation {
    ExchangedOperation {
        computer,
        sequence,
        stamp: format!("1762938000000-{sequence:05}-{computer}"),
        entity: "task".to_owned(),
        entity_id: format!("task-{sequence}"),
        change: ExchangedChange::Upsert {
            state: "{}".to_owned(),
            field_stamps: None,
        },
    }
}

/// A service on a ledger and a folder in memory, with the folder chosen.
struct Setup {
    service: SyncService,
    ledger: Arc<Ledger>,
    folder: Arc<Folder>,
    me: ComputerId,
}

fn setup() -> Setup {
    let me = ComputerId::generate();
    let ledger = Arc::new(Ledger {
        this_computer: Some(me),
        ..Ledger::default()
    });
    let folder = Arc::new(Folder::default());
    let service = SyncService::new(SyncPorts {
        ledger: ledger.clone(),
        folder: folder.clone(),
        file_system: Arc::new(PretendFileSystem::with_folders(["sync", "data"])),
        clock: Arc::new(ManualClock::starting_at(moment())),
        version: "0.1.0".to_owned(),
        data_folder: PathBuf::from("data"),
    });
    service.choose_folder(Path::new("sync"), "Desk").unwrap();
    Setup {
        service,
        ledger,
        folder,
        me,
    }
}

#[test]
fn a_round_writes_only_the_operations_not_written_yet() {
    let setup = setup();
    setup.ledger.state().operations = (1..=3).map(|n| operation(setup.me, n)).collect();

    let first = setup.service.round().unwrap();
    setup.ledger.state().operations.push(operation(setup.me, 4));
    let second = setup.service.round().unwrap();

    assert_eq!((first.written, second.written), (3, 1));
    let names: Vec<String> = setup
        .folder
        .files(setup.me)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        names,
        [
            "000000000001-000000000003.jsonl",
            "000000000004-000000000004.jsonl"
        ]
    );
    assert_eq!(
        lock(&setup.folder.cards).get(&setup.me).unwrap().name,
        "Desk"
    );
    let state = setup.service.state().unwrap();
    assert_eq!(state.phase, SyncPhase::InStep);
    assert_eq!(state.problem_code, None);
}

#[test]
fn a_computer_that_is_not_accepted_is_not_read() {
    let setup = setup();
    let laptop = ComputerId::generate();
    setup.folder.put(
        laptop,
        "000000000001-000000000002.jsonl",
        FileContent::Complete(vec![operation(laptop, 1), operation(laptop, 2)]),
    );

    let first = setup.service.round().unwrap();
    assert_eq!(first.new_computers, [laptop]);
    assert_eq!(first.taken_in, 0);
    setup.service.accept(laptop).unwrap();
    let second = setup.service.round().unwrap();
    let third = setup.service.round().unwrap();

    assert_eq!(second.taken_in, 2);
    assert_eq!(third.taken_in, 0, "what was read is not read again");
    assert_eq!(setup.ledger.state().taken_in.len(), 2);
    assert_eq!(setup.ledger.state().copies, 1);
}

#[test]
fn a_file_this_computer_did_not_write_in_its_own_folder_stops_the_round() {
    let setup = setup();
    setup.folder.put(
        setup.me,
        "000000000001-000000000001.jsonl",
        FileContent::Complete(vec![operation(setup.me, 1)]),
    );

    let error = setup.service.round().unwrap_err();

    assert_eq!(error.code(), "identity_in_use");
    let state = setup.service.state().unwrap();
    assert_eq!(state.phase, SyncPhase::Stopped);
    assert!(state.problem.is_some());
    assert_eq!(state.problem_code, Some("identity_in_use"));
}

#[test]
fn a_file_in_a_newer_format_stops_reading_that_computer_and_is_reported() {
    let setup = setup();
    let laptop = ComputerId::generate();
    setup.folder.put(
        laptop,
        "000000000001-000000000001.jsonl",
        FileContent::Complete(vec![operation(laptop, 1)]),
    );
    setup.folder.put(
        laptop,
        "000000000002-000000000002.jsonl",
        FileContent::NewerFormat(2),
    );
    setup.service.round().unwrap();
    setup.service.accept(laptop).unwrap();

    let report = setup.service.round().unwrap();
    setup.folder.put(
        laptop,
        "000000000003-000000000003.jsonl",
        FileContent::Complete(vec![operation(laptop, 3)]),
    );
    let later = setup.service.round().unwrap();

    assert_eq!(report.taken_in, 1);
    assert_eq!(report.newer_formats, [laptop]);
    assert_eq!(later.taken_in, 0);
    let known = setup.service.state().unwrap().computers;
    assert_eq!(known[0].newer_format, Some(2));
}

#[test]
fn a_folder_inside_the_data_folder_and_a_computer_without_a_name_are_refused() {
    let setup = setup();
    let file_system = PretendFileSystem::with_folders(["data/sync"]);
    let service = SyncService::new(SyncPorts {
        ledger: setup.ledger.clone(),
        folder: setup.folder.clone(),
        file_system: Arc::new(file_system),
        clock: Arc::new(ManualClock::starting_at(moment())),
        version: "0.1.0".to_owned(),
        data_folder: PathBuf::from("data"),
    });

    let inside = service
        .choose_folder(Path::new("data/sync"), "Desk")
        .unwrap_err();
    let nameless = setup
        .service
        .choose_folder(Path::new("sync"), "  ")
        .unwrap_err();

    assert_eq!(inside.code(), "sync_folder_in_data");
    assert_eq!(nameless.code(), "rule_broken");
}

#[test]
fn without_a_folder_nothing_runs() {
    let setup = setup();
    setup.service.stop().unwrap();

    let error = setup.service.round().unwrap_err();

    assert_eq!(error.code(), "sync_off");
    assert_eq!(setup.service.state().unwrap().phase, SyncPhase::Off);
}

#[test]
fn a_new_identity_stops_syncing_until_the_program_starts_again() {
    let setup = setup();

    setup.service.new_identity().unwrap();

    assert_eq!(setup.service.round().unwrap_err().code(), "identity_in_use");
    let state = setup.service.state().unwrap();
    assert_eq!(state.phase, SyncPhase::Stopped);
    assert_eq!(state.problem_code, Some(RESTART_NEEDED));
}

/// A folder that counts how many rounds work in it at the same moment.
#[derive(Default)]
struct WatchedFolder {
    inner: Folder,
    working: Mutex<usize>,
    most_at_once: Mutex<usize>,
}

impl SyncFolder for WatchedFolder {
    fn write_card(&self, root: &Path, card: &ComputerCard) -> Result<(), StorageError> {
        {
            let mut working = lock(&self.working);
            *working += 1;
            let mut most = lock(&self.most_at_once);
            *most = (*most).max(*working);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        *lock(&self.working) -= 1;
        self.inner.write_card(root, card)
    }

    fn write_file(
        &self,
        root: &Path,
        computer: ComputerId,
        operations: &[ExchangedOperation],
    ) -> Result<String, StorageError> {
        self.inner.write_file(root, computer, operations)
    }

    fn computers(&self, root: &Path) -> Result<Vec<FoundComputer>, StorageError> {
        self.inner.computers(root)
    }

    fn files_of(
        &self,
        root: &Path,
        computer: ComputerId,
        after: u64,
    ) -> Result<Vec<String>, StorageError> {
        self.inner.files_of(root, computer, after)
    }

    fn read_file(
        &self,
        root: &Path,
        computer: ComputerId,
        name: &str,
    ) -> Result<FileContent, StorageError> {
        self.inner.read_file(root, computer, name)
    }

    fn join_small_files(
        &self,
        root: &Path,
        computer: ComputerId,
    ) -> Result<Option<String>, StorageError> {
        self.inner.join_small_files(root, computer)
    }
}

#[test]
fn two_rounds_started_at_once_run_one_after_the_other() {
    let me = ComputerId::generate();
    let folder = Arc::new(WatchedFolder::default());
    let service = SyncService::new(SyncPorts {
        ledger: Arc::new(Ledger {
            this_computer: Some(me),
            ..Ledger::default()
        }),
        folder: folder.clone(),
        file_system: Arc::new(PretendFileSystem::with_folders(["sync"])),
        clock: Arc::new(ManualClock::starting_at(moment())),
        version: "0.1.0".to_owned(),
        data_folder: PathBuf::from("data"),
    });
    service.choose_folder(Path::new("sync"), "Desk").unwrap();

    let rounds: Vec<_> = (0..4)
        .map(|_| {
            let service = service.clone();
            std::thread::spawn(move || service.round().map(|_| ()))
        })
        .collect();
    for round in rounds {
        round.join().unwrap().unwrap();
    }

    assert_eq!(*lock(&folder.most_at_once), 1);
}
