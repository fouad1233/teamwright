//! The workbook written and read back, as another program reads it.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use teamwright_application::Column;

use super::*;

fn day(day_of_month: u32) -> CalendarDate {
    CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
}

fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

/// A small workbook with an overview and a list, and a cell of every kind.
fn sample() -> Workbook {
    Workbook {
        sheets: vec![
            Sheet {
                name: "Summary".to_owned(),
                content: SheetContent::Overview {
                    title: "Smart home hub".to_owned(),
                    sections: vec![
                        Section {
                            heading: "Project".to_owned(),
                            table: Table {
                                columns: vec![Column::named(""), Column::wrapping("")],
                                rows: vec![
                                    vec![text("Key"), text("SH")],
                                    vec![text("Exported"), Cell::Date(day(13))],
                                ],
                            },
                        },
                        Section {
                            heading: "Tasks by status".to_owned(),
                            table: Table {
                                columns: vec![Column::named("Status"), Column::named("Tasks")],
                                rows: vec![vec![
                                    Cell::Status {
                                        name: "To do".to_owned(),
                                        color: Color::Gray,
                                    },
                                    Cell::Whole(4),
                                ]],
                            },
                        },
                    ],
                },
            },
            Sheet {
                name: "Tasks".to_owned(),
                content: SheetContent::List(Table {
                    columns: vec![
                        Column::named("Task"),
                        Column::wrapping("Title"),
                        Column::named("Estimate (hours)"),
                        Column::named("Due"),
                        Column::named("Points"),
                        Column::named("Unit cost (€)"),
                    ],
                    rows: vec![
                        vec![
                            text("SH-1"),
                            Cell::Indented {
                                text: "Choose the radio".to_owned(),
                                level: 0,
                            },
                            Cell::Hours(1.5),
                            Cell::Date(day(20)),
                            Cell::Whole(5),
                            Cell::Number(0.375),
                        ],
                        vec![
                            text("SH-2"),
                            Cell::Indented {
                                text: "Compare the modules".to_owned(),
                                level: 1,
                            },
                            Cell::Empty,
                            Cell::Empty,
                            Cell::Whole(-1),
                            Cell::Empty,
                        ],
                    ],
                }),
            },
        ],
    }
}

fn read_back(workbook: &Workbook) -> Xlsx<Cursor<Vec<u8>>> {
    let bytes = to_bytes(workbook, "Smart home hub").unwrap();
    Xlsx::new(Cursor::new(bytes)).unwrap()
}

/// The serial number Excel gives a day: the days since 30 December 1899.
fn serial_of(date: CalendarDate) -> f64 {
    let origin = CalendarDate::from_ymd_opt(1899, 12, 30).unwrap();
    (date - origin).num_days() as f64
}

#[test]
fn the_sheets_come_in_their_order() {
    let excel = read_back(&sample());

    assert_eq!(excel.sheet_names(), ["Summary", "Tasks"]);
}

#[test]
fn a_list_has_its_header_on_the_first_row_and_its_cells_with_their_types() {
    let mut excel = read_back(&sample());
    let tasks = excel.worksheet_range("Tasks").unwrap();

    let header: Vec<String> = tasks
        .rows()
        .next()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        header,
        [
            "Task",
            "Title",
            "Estimate (hours)",
            "Due",
            "Points",
            "Unit cost (€)"
        ]
    );

    assert_eq!(
        tasks.get_value((1, 0)),
        Some(&Data::String("SH-1".to_owned()))
    );
    assert_eq!(
        tasks.get_value((1, 1)),
        Some(&Data::String("Choose the radio".to_owned()))
    );
    assert_eq!(tasks.get_value((1, 2)), Some(&Data::Float(1.5)));
    match tasks.get_value((1, 3)) {
        Some(Data::DateTime(date)) => assert!((date.as_f64() - serial_of(day(20))).abs() < 1e-9),
        other => panic!("the due date is {other:?}, not a date"),
    }
    assert_eq!(tasks.get_value((1, 4)), Some(&Data::Float(5.0)));
    assert_eq!(tasks.get_value((2, 4)), Some(&Data::Float(-1.0)));
    assert_eq!(tasks.get_value((1, 5)), Some(&Data::Float(0.375)));
}

#[test]
fn an_empty_cell_holds_nothing() {
    let mut excel = read_back(&sample());
    let tasks = excel.worksheet_range("Tasks").unwrap();

    assert!(matches!(tasks.get_value((2, 2)), None | Some(Data::Empty)));
    assert!(matches!(tasks.get_value((2, 3)), None | Some(Data::Empty)));
}

#[test]
fn an_overview_has_its_title_then_its_sections_under_their_headings() {
    let mut excel = read_back(&sample());
    let summary = excel.worksheet_range("Summary").unwrap();
    let at = |row: u32, column: u32| summary.get_value((row, column)).cloned();
    let string = |value: &str| Some(Data::String(value.to_owned()));

    assert_eq!(at(0, 0), string("Smart home hub"));
    assert_eq!(at(2, 0), string("Project"));
    assert_eq!(at(3, 0), string("Key"));
    assert_eq!(at(3, 1), string("SH"));
    assert_eq!(at(4, 0), string("Exported"));
    assert!(matches!(at(4, 1), Some(Data::DateTime(_))));
    // An empty row, then the next section with its header.
    assert_eq!(at(6, 0), string("Tasks by status"));
    assert_eq!(at(7, 0), string("Status"));
    assert_eq!(at(7, 1), string("Tasks"));
    assert_eq!(at(8, 0), string("To do"));
    assert_eq!(at(8, 1), Some(Data::Float(4.0)));
}

#[test]
fn a_file_is_written_and_replaces_one_that_was_there() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("Smart home hub.xlsx");
    std::fs::write(&path, "an older file").unwrap();

    write_workbook(&sample(), "Smart home hub", &path).unwrap();

    let mut excel: Xlsx<_> = calamine::open_workbook(&path).unwrap();
    assert_eq!(excel.sheet_names(), ["Summary", "Tasks"]);
    assert!(excel.worksheet_range("Tasks").is_ok());
}

#[cfg(unix)]
#[test]
fn a_link_at_the_name_is_refused_and_its_target_is_not_overwritten() {
    let folder = tempfile::tempdir().unwrap();
    let target = folder.path().join("precious.txt");
    std::fs::write(&target, "precious").unwrap();
    let path = folder.path().join("Smart home hub.xlsx");
    std::os::unix::fs::symlink(&target, &path).unwrap();

    let error = write_workbook(&sample(), "Smart home hub", &path).unwrap_err();

    assert!(matches!(error, WriteError::Failed { .. }));
    assert!(error.to_string().contains("link"));
    assert_eq!(std::fs::read(&target).unwrap(), b"precious");
}

#[test]
fn a_workbook_leaves_no_temporary_file_beside_it() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("Smart home hub.xlsx");

    write_workbook(&sample(), "Smart home hub", &path).unwrap();

    let names: Vec<_> = std::fs::read_dir(folder.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["Smart home hub.xlsx"]);
}

#[test]
fn a_file_in_a_folder_that_does_not_exist_is_refused_in_plain_words() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("gone").join("Smart home hub.xlsx");

    let error = write_workbook(&sample(), "Smart home hub", &path).unwrap_err();

    assert!(matches!(error, WriteError::NoFolder { .. }));
    assert!(
        error
            .to_string()
            .ends_with("because its folder does not exist")
    );
}

#[test]
fn a_disk_that_is_full_and_a_file_that_is_held_open_are_named() {
    let full = std::io::Error::from(ErrorKind::StorageFull);
    let refused = std::io::Error::from(ErrorKind::PermissionDenied);
    let shared = std::io::Error::from_raw_os_error(32);

    assert!(matches!(
        io_failure(&full, "a.xlsx".to_owned()),
        WriteError::DiskFull { .. }
    ));
    assert!(matches!(
        io_failure(&refused, "a.xlsx".to_owned()),
        WriteError::InUse { .. }
    ));
    if cfg!(windows) {
        assert!(matches!(
            io_failure(&shared, "a.xlsx".to_owned()),
            WriteError::InUse { .. }
        ));
    }
}

#[test]
fn the_shades_of_a_status_are_different_for_every_color() {
    let colors = [
        Color::Gray,
        Color::Blue,
        Color::Purple,
        Color::Amber,
        Color::Green,
        Color::Red,
        Color::Pink,
        Color::Teal,
    ];
    let shades: std::collections::HashSet<(u32, u32)> =
        colors.iter().map(|color| shades_of(*color)).collect();

    assert_eq!(shades.len(), colors.len());
}
