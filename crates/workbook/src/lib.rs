//! Writes a workbook as an `.xlsx` file that Excel, LibreOffice and Google
//! Sheets open
//! ([decision 0025](../../../docs/decisions/0025-the-project-as-a-workbook.md)).
//!
//! The application crate decides what a workbook holds. This crate only
//! decides how it looks: one typeface, a header row in bold on a faint fill,
//! thin lines under the rows, dates shown as `13 Nov 2026`, hours with one
//! decimal, and no colour but the status of a task. A list is frozen under
//! its header and has a filter on it; an overview has no grid.

use std::io::ErrorKind;
use std::path::Path;

use rust_xlsxwriter::{
    Color as ExcelColor, DocProperties, ExcelDateTime, Format, FormatAlign, FormatBorder,
    Workbook as ExcelWorkbook, Worksheet, XlsxError,
};
use teamwright_application::{Cell, Section, Sheet, SheetContent, Table, Workbook};
use teamwright_domain::{CalendarDate, Color};

/// The typeface of every cell. Excel has it, and LibreOffice and Google
/// Sheets put a typeface of the same widths in its place.
const TYPEFACE: &str = "Calibri";

/// The size of the type of every cell but the title.
const TYPE_SIZE: f64 = 11.0;

/// The size of the type of the title of an overview.
const TITLE_SIZE: f64 = 18.0;

/// The colour of text.
const INK: u32 = 0x2B_25_1F;

/// The colour of labels and of what matters less.
const MUTED: u32 = 0x7A_6F_60;

/// The colour of the lines under the rows.
const HAIRLINE: u32 = 0xE4_DD_D0;

/// The fill of a header row.
const HEADER_FILL: u32 = 0xF5_F1_EA;

/// How a date is shown: `13 Nov 2026`.
const DATE_FORMAT: &str = "d mmm yyyy";

/// How hours are shown: one decimal.
const HOURS_FORMAT: &str = "0.0";

/// How whole numbers are shown, with a separator for thousands.
const WHOLE_FORMAT: &str = "#,##0";

/// The narrowest and the widest a column is made, in characters.
const NARROWEST: f64 = 8.0;
const WIDEST: f64 = 40.0;

/// The widest a column of long text is made before it wraps, in characters.
const WIDEST_WRAPPING: f64 = 60.0;

/// How many characters one level of indent takes, roughly.
const INDENT_WIDTH: usize = 3;

/// Why a workbook could not be written.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// Another program holds the file open, or the file may not be changed.
    #[error(
        "{path} could not be written, because another program has it open or it may not be changed. Close it, or choose another name"
    )]
    InUse {
        /// The file.
        path: String,
    },

    /// The disk has no room left.
    #[error("{path} could not be written, because the disk is full")]
    DiskFull {
        /// The file.
        path: String,
    },

    /// The folder to write into does not exist.
    #[error("{path} could not be written, because its folder does not exist")]
    NoFolder {
        /// The file.
        path: String,
    },

    /// Writing failed for another reason, which is named.
    #[error("{path} could not be written: {reason}")]
    Failed {
        /// The file.
        path: String,
        /// What went wrong, as the system or the writer said it.
        reason: String,
    },
}

/// Writes a workbook to a file, replacing a file that is there.
///
/// The whole file is made in memory first, so that a failure of the
/// writer leaves an existing file as it was.
///
/// # Errors
///
/// Returns an error that says in plain words why the file could not be
/// written: it is open in another program, the disk is full, its folder is
/// gone, or something else, which is named.
pub fn write_workbook(workbook: &Workbook, title: &str, path: &Path) -> Result<(), WriteError> {
    let shown = path.display().to_string();
    let bytes = to_bytes(workbook, title).map_err(|error| WriteError::Failed {
        path: shown.clone(),
        reason: error.to_string(),
    })?;
    std::fs::write(path, bytes).map_err(|error| io_failure(&error, shown))
}

/// Makes the bytes of the `.xlsx` file of a workbook. `title` goes into the
/// properties of the document.
///
/// # Errors
///
/// Returns an error if the writer refuses something, such as a name of a
/// sheet that is too long.
pub fn to_bytes(workbook: &Workbook, title: &str) -> Result<Vec<u8>, XlsxError> {
    let styles = Styles::new();
    let mut excel = ExcelWorkbook::new();
    excel.set_properties(&DocProperties::new().set_title(title));
    for sheet in &workbook.sheets {
        let worksheet = excel.add_worksheet();
        write_sheet(worksheet, sheet, &styles)?;
    }
    excel.save_to_buffer()
}

/// Puts the reason a file could not be written into words.
fn io_failure(error: &std::io::Error, path: String) -> WriteError {
    // Windows reports a file another program holds open as a sharing or a
    // lock violation, which the standard library does not name.
    const SHARING_VIOLATION: i32 = 32;
    const LOCK_VIOLATION: i32 = 33;
    if cfg!(windows)
        && matches!(
            error.raw_os_error(),
            Some(SHARING_VIOLATION | LOCK_VIOLATION)
        )
    {
        return WriteError::InUse { path };
    }
    match error.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => WriteError::InUse { path },
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => WriteError::DiskFull { path },
        ErrorKind::NotFound => WriteError::NoFolder { path },
        _ => WriteError::Failed {
            path,
            reason: error.to_string(),
        },
    }
}

/// Writes one sheet.
fn write_sheet(worksheet: &mut Worksheet, sheet: &Sheet, styles: &Styles) -> Result<(), XlsxError> {
    worksheet.set_name(&sheet.name)?;
    match &sheet.content {
        SheetContent::Overview { title, sections } => {
            write_overview(worksheet, title, sections, styles)
        }
        SheetContent::List(table) => write_list(worksheet, table, styles),
    }
}

/// Writes an overview: its title large, then every section under a heading,
/// with an empty row between them. It has no grid: it is read, not edited.
fn write_overview(
    worksheet: &mut Worksheet,
    title: &str,
    sections: &[Section],
    styles: &Styles,
) -> Result<(), XlsxError> {
    worksheet.set_screen_gridlines(false);
    worksheet.write_string_with_format(0, 0, title, &styles.title)?;
    worksheet.set_row_height(0, 30)?;

    let mut row: u32 = 2;
    let mut widths = Widths::default();
    for section in sections {
        worksheet.write_string_with_format(row, 0, &section.heading, &styles.heading)?;
        row += 1;
        let table = &section.table;
        if table.has_header() {
            write_header(worksheet, row, table, styles, true)?;
            row += 1;
        }
        for cells in &table.rows {
            for (index, cell) in cells.iter().enumerate() {
                let column = column_number(index);
                let place = Place {
                    // In a table of facts the first column is the label.
                    is_label: !table.has_header() && index == 0,
                    wraps: wraps(table, index),
                    // A number stands under its label or beside it, not at
                    // the far end of a wide column of text.
                    numbers_left: true,
                };
                write_cell(worksheet, (row, column), cell, styles, place)?;
                widths.measure(column, cell, place.wraps);
            }
            row += 1;
        }
        // Headers count for the widths too, but the title does not: it may
        // run on to the right.
        for (column, it) in table.columns.iter().enumerate() {
            widths.measure_text(column_number(column), &it.name, it.wraps);
        }
        widths.measure_text(0, &section.heading, false);
        row += 1;
    }
    widths.apply(worksheet)
}

/// Writes a list: the header row frozen at the top with a filter on it,
/// then one row a thing.
fn write_list(worksheet: &mut Worksheet, table: &Table, styles: &Styles) -> Result<(), XlsxError> {
    write_header(worksheet, 0, table, styles, false)?;
    let mut widths = Widths::default();
    for (column, it) in table.columns.iter().enumerate() {
        // The header has a button for the filter beside its name.
        widths.measure_text(column_number(column), &format!("{}   ", it.name), false);
    }
    for (index, cells) in table.rows.iter().enumerate() {
        let row = u32::try_from(index + 1).unwrap_or(u32::MAX);
        for (index, cell) in cells.iter().enumerate() {
            let column = column_number(index);
            let place = Place {
                is_label: false,
                wraps: wraps(table, index),
                numbers_left: false,
            };
            write_cell(worksheet, (row, column), cell, styles, place)?;
            widths.measure(column, cell, place.wraps);
        }
    }
    let last_row = u32::try_from(table.rows.len()).unwrap_or(u32::MAX);
    let last_column = column_number(table.columns.len().saturating_sub(1));
    worksheet.set_freeze_panes(1, 0)?;
    worksheet.autofilter(0, 0, last_row, last_column)?;
    widths.apply(worksheet)
}

/// Writes the names of the columns of a table as a header row. Where
/// numbers stand right, the name of a column of numbers stands right too,
/// above them.
fn write_header(
    worksheet: &mut Worksheet,
    row: u32,
    table: &Table,
    styles: &Styles,
    numbers_left: bool,
) -> Result<(), XlsxError> {
    for (index, it) in table.columns.iter().enumerate() {
        let format = if numbers_left {
            &styles.header
        } else if holds_numbers(table, index) {
            &styles.header_of_numbers
        } else if holds_dates(table, index) {
            &styles.header_of_dates
        } else {
            &styles.header
        };
        worksheet.write_string_with_format(row, column_number(index), &it.name, format)?;
    }
    Ok(())
}

/// Whether a column of a table wraps its text.
fn wraps(table: &Table, index: usize) -> bool {
    table.columns.get(index).is_some_and(|it| it.wraps)
}

/// Whether a column of a table holds numbers.
fn holds_numbers(table: &Table, index: usize) -> bool {
    table
        .rows
        .iter()
        .any(|row| matches!(row.get(index), Some(Cell::Whole(_) | Cell::Hours(_))))
}

/// Whether a column of a table holds dates.
fn holds_dates(table: &Table, index: usize) -> bool {
    table
        .rows
        .iter()
        .any(|row| matches!(row.get(index), Some(Cell::Date(_))))
}

/// Where a cell stands, as far as its look depends on it.
#[derive(Debug, Clone, Copy)]
struct Place {
    /// It is the label of a fact.
    is_label: bool,
    /// Its column wraps long text.
    wraps: bool,
    /// Numbers stand left in it, as text does.
    numbers_left: bool,
}

/// Writes one cell at a row and a column with the type and the look its
/// kind asks for.
fn write_cell(
    worksheet: &mut Worksheet,
    (row, column): (u32, u16),
    cell: &Cell,
    styles: &Styles,
    place: Place,
) -> Result<(), XlsxError> {
    let text_style = match (place.is_label, place.wraps) {
        (true, _) => &styles.label,
        (false, true) => &styles.wrapping,
        (false, false) => &styles.text,
    };
    let (whole, hours) = if place.numbers_left {
        (&styles.whole_left, &styles.hours_left)
    } else {
        (&styles.whole, &styles.hours)
    };
    match cell {
        Cell::Empty => {
            worksheet.write_blank(row, column, text_style)?;
        }
        Cell::Text(text) => {
            worksheet.write_string_with_format(row, column, text, text_style)?;
        }
        Cell::Indented { text, level } => {
            let format = text_style.clone().set_indent(*level);
            worksheet.write_string_with_format(row, column, text, &format)?;
        }
        Cell::Whole(number) => {
            // Counts, days and tokens stay far below 2 to the 53rd, where a
            // floating-point number would begin to lose whole numbers.
            let value = *number as f64;
            worksheet.write_number_with_format(row, column, value, whole)?;
        }
        Cell::Hours(value) => {
            worksheet.write_number_with_format(row, column, *value, hours)?;
        }
        Cell::Date(date) => {
            worksheet.write_datetime_with_format(
                row,
                column,
                &excel_date(*date)?,
                if place.numbers_left {
                    &styles.date
                } else {
                    &styles.date_set_in
                },
            )?;
        }
        Cell::Status { name, color } => {
            worksheet.write_string_with_format(row, column, name, &styles.status(*color))?;
        }
    }
    Ok(())
}

/// A day on the calendar as Excel keeps it.
fn excel_date(date: CalendarDate) -> Result<ExcelDateTime, XlsxError> {
    use chrono::Datelike;
    let year =
        u16::try_from(date.year()).map_err(|_| XlsxError::DateTimeRangeError(date.to_string()))?;
    let month =
        u8::try_from(date.month()).map_err(|_| XlsxError::DateTimeRangeError(date.to_string()))?;
    let day =
        u8::try_from(date.day()).map_err(|_| XlsxError::DateTimeRangeError(date.to_string()))?;
    ExcelDateTime::from_ymd(year, month, day)
}

/// The number of a column, as the writer counts columns.
fn column_number(index: usize) -> u16 {
    u16::try_from(index).unwrap_or(u16::MAX)
}

/// The looks of the cells, made once.
struct Styles {
    title: Format,
    heading: Format,
    header: Format,
    header_of_numbers: Format,
    label: Format,
    text: Format,
    wrapping: Format,
    whole: Format,
    whole_left: Format,
    hours: Format,
    hours_left: Format,
    date: Format,
    date_set_in: Format,
    header_of_dates: Format,
}

impl Styles {
    fn new() -> Self {
        let base = Format::new()
            .set_font_name(TYPEFACE)
            .set_font_size(TYPE_SIZE)
            .set_font_color(ExcelColor::RGB(INK))
            .set_align(FormatAlign::Top);
        let ruled = base
            .clone()
            .set_border_bottom(FormatBorder::Thin)
            .set_border_bottom_color(ExcelColor::RGB(HAIRLINE));
        let header = ruled
            .clone()
            .set_bold()
            .set_background_color(ExcelColor::RGB(HEADER_FILL))
            .set_border_bottom_color(ExcelColor::RGB(MUTED));
        let whole = ruled.clone().set_num_format(WHOLE_FORMAT);
        let hours = ruled.clone().set_num_format(HOURS_FORMAT);
        Self {
            title: base
                .clone()
                .set_font_size(TITLE_SIZE)
                .set_align(FormatAlign::VerticalCenter),
            heading: base.clone().set_bold(),
            header_of_numbers: header.clone().set_align(FormatAlign::Right),
            // In a list a date is set in a little, so that it does not run
            // into a number standing right in the column before it.
            header_of_dates: header.clone().set_indent(1),
            header,
            label: ruled.clone().set_font_color(ExcelColor::RGB(MUTED)),
            text: ruled.clone(),
            wrapping: ruled.clone().set_text_wrap(),
            whole_left: whole.clone().set_align(FormatAlign::Left),
            whole,
            hours_left: hours.clone().set_align(FormatAlign::Left),
            hours,
            date_set_in: ruled
                .clone()
                .set_num_format(DATE_FORMAT)
                .set_align(FormatAlign::Left)
                .set_indent(1),
            date: ruled
                .set_num_format(DATE_FORMAT)
                .set_align(FormatAlign::Left),
        }
    }

    /// The look of the status of a task: its name in a deep shade of its
    /// colour on a pale one, as the interface shows it.
    fn status(&self, color: Color) -> Format {
        let (fill, ink) = shades_of(color);
        self.text
            .clone()
            .set_background_color(ExcelColor::RGB(fill))
            .set_font_color(ExcelColor::RGB(ink))
    }
}

/// The pale fill and the deep ink of a colour of a status.
const fn shades_of(color: Color) -> (u32, u32) {
    match color {
        Color::Gray => (0xF1_F2_F4, 0x4B_52_5A),
        Color::Blue => (0xE8_F0_FD, 0x1F_4F_B8),
        Color::Purple => (0xF1_EC_FB, 0x5E_3A_A8),
        Color::Amber => (0xFC_F3_E0, 0x86_57_00),
        Color::Green => (0xE6_F4_EA, 0x1D_6B_38),
        Color::Red => (0xFC_EA_E9, 0xA6_23_1A),
        Color::Pink => (0xFB_E9_F2, 0x9C_2A_5E),
        Color::Teal => (0xE2_F4_F2, 0x11_6A_63),
    }
}

/// The widths of the columns of a sheet, worked out from what they hold.
#[derive(Default)]
struct Widths {
    /// For every column: how wide its widest cell is, in characters, and
    /// whether it wraps.
    columns: Vec<(usize, bool)>,
}

impl Widths {
    /// Counts a cell into the width of its column.
    fn measure(&mut self, column: u16, cell: &Cell, wraps: bool) {
        let characters = match cell {
            Cell::Empty => 0,
            Cell::Text(text) | Cell::Status { name: text, .. } => text.chars().count(),
            Cell::Indented { text, level } => {
                text.chars().count() + usize::from(*level) * INDENT_WIDTH
            }
            Cell::Whole(number) => number.to_string().len() + number.to_string().len() / 3,
            Cell::Hours(hours) => format!("{hours:.1}").len(),
            // "13 Nov 2026"
            Cell::Date(_) => 11,
        };
        self.count(column, characters, wraps);
    }

    /// Counts a text into the width of its column.
    fn measure_text(&mut self, column: u16, text: &str, wraps: bool) {
        self.count(column, text.chars().count(), wraps);
    }

    fn count(&mut self, column: u16, characters: usize, wraps: bool) {
        let index = usize::from(column);
        if self.columns.len() <= index {
            self.columns.resize(index + 1, (0, false));
        }
        if let Some(entry) = self.columns.get_mut(index) {
            entry.0 = entry.0.max(characters);
            entry.1 |= wraps;
        }
    }

    /// Gives every column its width: enough for its widest cell and a
    /// little air, within bounds; a column of long text wraps beyond its
    /// widest.
    fn apply(&self, worksheet: &mut Worksheet) -> Result<(), XlsxError> {
        for (index, (characters, wraps)) in self.columns.iter().enumerate() {
            let wanted = *characters as f64 + 2.0;
            let widest = if *wraps { WIDEST_WRAPPING } else { WIDEST };
            worksheet.set_column_width(column_number(index), wanted.clamp(NARROWEST, widest))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
