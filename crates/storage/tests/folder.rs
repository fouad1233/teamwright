//! The files in the sync folder, tested in a temporary folder of their own.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::thread;

use chrono::{TimeZone, Utc};
use teamwright_application::{
    ComputerCard, ExchangedChange, ExchangedOperation, FileContent, SYNC_FORMAT, SyncFolder,
};
use teamwright_domain::ComputerId;
use teamwright_storage::FolderOfComputers;

/// Operations of a computer, numbered from `first` to `last`.
fn operations(computer: ComputerId, first: u64, last: u64) -> Vec<ExchangedOperation> {
    (first..=last)
        .map(|sequence| ExchangedOperation {
            computer,
            sequence,
            stamp: format!("17629380000{:02}-00000-{computer}", sequence % 100),
            entity: "task".to_owned(),
            entity_id: format!("task-{sequence}"),
            change: if sequence % 5 == 0 {
                ExchangedChange::Delete {
                    seen: Some(format!(
                        r#"{{"{computer}":"1762938000000-00000-{computer}"}}"#
                    )),
                }
            } else {
                ExchangedChange::Upsert {
                    state: format!(r#"{{"id":"task-{sequence}","title":"Task {sequence}"}}"#),
                    field_stamps: (sequence % 2 == 0)
                        .then(|| r#"{"title":"1762938000000-00000-x"}"#.to_owned()),
                }
            },
        })
        .collect()
}

fn read_all(
    folder: &FolderOfComputers,
    root: &Path,
    computer: ComputerId,
    after: u64,
) -> Vec<ExchangedOperation> {
    let mut read = Vec::new();
    for name in folder.files_of(root, computer, after).unwrap() {
        let FileContent::Complete(operations) = folder.read_file(root, computer, &name).unwrap()
        else {
            panic!("{name} is not whole");
        };
        read.extend(
            operations
                .into_iter()
                .filter(|operation| operation.sequence > after),
        );
    }
    read
}

fn log_folder(root: &Path, computer: ComputerId) -> std::path::PathBuf {
    root.join("devices").join(computer.to_string()).join("log")
}

#[test]
fn a_file_written_and_read_back_gives_the_same_operations() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::new();
    let desk = ComputerId::generate();
    let written = operations(desk, 1, 12);

    let name = folder.write_file(root.path(), desk, &written).unwrap();

    assert_eq!(name, "000000000001-000000000012.jsonl");
    assert_eq!(
        folder.read_file(root.path(), desk, &name).unwrap(),
        FileContent::Complete(written)
    );
}

#[test]
fn a_file_cut_off_anywhere_is_not_read_until_it_is_whole() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::new();
    let desk = ComputerId::generate();
    let name = folder
        .write_file(root.path(), desk, &operations(desk, 1, 3))
        .unwrap();
    let path = log_folder(root.path(), desk).join(&name);
    let whole = fs::read(&path).unwrap();

    for length in 0..whole.len() {
        fs::write(&path, &whole[..length]).unwrap();
        assert_eq!(
            folder.read_file(root.path(), desk, &name).unwrap(),
            FileContent::NotYet,
            "cut off after {length} bytes"
        );
    }
    fs::write(&path, &whole).unwrap();
    assert!(matches!(
        folder.read_file(root.path(), desk, &name).unwrap(),
        FileContent::Complete(read) if read.len() == 3
    ));
}

#[test]
fn files_of_another_pattern_are_passed_over() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::new();
    let desk = ComputerId::generate();
    let name = folder
        .write_file(root.path(), desk, &operations(desk, 1, 3))
        .unwrap();
    let log = log_folder(root.path(), desk);
    let whole = fs::read(log.join(&name)).unwrap();
    for other in [
        ".writing-0196f3a2".to_owned(),
        "000000000001-000000000003 (conflicted copy).jsonl".to_owned(),
        "notes.txt".to_owned(),
    ] {
        fs::write(log.join(other), &whole).unwrap();
    }

    assert_eq!(folder.files_of(root.path(), desk, 0).unwrap(), [name]);
    assert!(folder.files_of(root.path(), desk, 3).unwrap().is_empty());
}

#[test]
fn a_reader_that_read_some_small_files_gets_every_operation_once_after_they_are_joined() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::joining_after(3);
    let desk = ComputerId::generate();
    for first in [1, 4, 7, 10] {
        folder
            .write_file(root.path(), desk, &operations(desk, first, first + 2))
            .unwrap();
    }
    let read_before = read_all(&folder, root.path(), desk, 0)
        .into_iter()
        .filter(|operation| operation.sequence <= 6)
        .collect::<Vec<_>>();

    let joined = folder.join_small_files(root.path(), desk).unwrap();
    let read_after = read_all(&folder, root.path(), desk, 6);

    assert_eq!(joined.as_deref(), Some("000000000001-000000000012.jsonl"));
    assert_eq!(folder.files_of(root.path(), desk, 0).unwrap().len(), 1);
    let mut all = read_before;
    all.extend(read_after);
    assert_eq!(all, operations(desk, 1, 12));
}

#[test]
fn few_small_files_are_left_as_they_are() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::joining_after(3);
    let desk = ComputerId::generate();
    for first in [1, 4, 7] {
        folder
            .write_file(root.path(), desk, &operations(desk, first, first + 2))
            .unwrap();
    }

    assert_eq!(folder.join_small_files(root.path(), desk).unwrap(), None);
    assert_eq!(folder.files_of(root.path(), desk, 0).unwrap().len(), 3);
}

#[test]
fn two_computers_write_into_one_folder_without_touching_each_other() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::joining_after(1);
    let desk = ComputerId::generate();
    let laptop = ComputerId::generate();

    for first in [1, 3, 5] {
        folder
            .write_file(root.path(), desk, &operations(desk, first, first + 1))
            .unwrap();
        folder
            .write_file(root.path(), laptop, &operations(laptop, first, first + 1))
            .unwrap();
    }
    folder.join_small_files(root.path(), desk).unwrap();

    assert_eq!(
        read_all(&folder, root.path(), desk, 0),
        operations(desk, 1, 6)
    );
    assert_eq!(folder.files_of(root.path(), laptop, 0).unwrap().len(), 3);
    assert_eq!(
        read_all(&folder, root.path(), laptop, 0),
        operations(laptop, 1, 6)
    );
}

#[test]
fn the_computers_in_the_folder_are_found_with_their_cards() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::new();
    let desk = ComputerId::generate();
    let laptop = ComputerId::generate();
    let card = ComputerCard {
        id: desk,
        name: "Desk".to_owned(),
        version: "0.1.0".to_owned(),
        format: SYNC_FORMAT,
        written_at: Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
    };
    folder.write_card(root.path(), &card).unwrap();
    folder
        .write_file(root.path(), laptop, &operations(laptop, 1, 1))
        .unwrap();
    fs::create_dir_all(root.path().join("devices").join("not a computer")).unwrap();

    let found = folder.computers(root.path()).unwrap();

    assert_eq!(found.len(), 2);
    let desk_found = found.iter().find(|computer| computer.id == desk).unwrap();
    assert_eq!(desk_found.card.as_ref(), Some(&card));
    let laptop_found = found.iter().find(|computer| computer.id == laptop).unwrap();
    assert_eq!(laptop_found.card, None);
}

#[test]
fn a_card_replaced_while_it_is_read_is_the_old_or_the_new_one() {
    let root = tempfile::tempdir().unwrap();
    let folder = Arc::new(FolderOfComputers::new());
    let desk = ComputerId::generate();
    let card = move |name: &str| ComputerCard {
        id: desk,
        name: name.to_owned(),
        version: "0.1.0".to_owned(),
        format: SYNC_FORMAT,
        written_at: Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
    };
    folder.write_card(root.path(), &card("Desk")).unwrap();
    let path = root.path().to_path_buf();
    let writer = {
        let folder = Arc::clone(&folder);
        let path = path.clone();
        thread::spawn(move || {
            for round in 0..50 {
                let name = if round % 2 == 0 { "Laptop" } else { "Desk" };
                // On some systems a rename fails while the file is open
                // for reading; the card is then simply written next time.
                let _ = folder.write_card(&path, &card(name));
            }
        })
    };
    for _ in 0..200 {
        let found = folder.computers(&path).unwrap();
        if let Some(card) = found.first().and_then(|computer| computer.card.as_ref()) {
            assert!(card.name == "Desk" || card.name == "Laptop");
        }
    }
    writer.join().unwrap();
}

#[test]
fn a_file_in_a_newer_format_is_reported() {
    let root = tempfile::tempdir().unwrap();
    let folder = FolderOfComputers::new();
    let desk = ComputerId::generate();
    let name = folder
        .write_file(root.path(), desk, &operations(desk, 1, 2))
        .unwrap();
    let path = log_folder(root.path(), desk).join(&name);
    let newer = fs::read_to_string(&path).unwrap().replacen(
        &format!("\"format\":{SYNC_FORMAT}"),
        "\"format\":99",
        1,
    );
    fs::write(&path, newer).unwrap();

    assert_eq!(
        folder.read_file(root.path(), desk, &name).unwrap(),
        FileContent::NewerFormat(99)
    );
}
