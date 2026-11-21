# teamwright-workbook

Writes a project as an Excel workbook (`.xlsx`) that Excel, LibreOffice and
Google Sheets open
([decision 0025](../../docs/decisions/0025-the-project-as-a-workbook.md)).

The application crate decides what a workbook holds
(`WorkbookService::workbook_of` returns a `Workbook` of sheets, tables and
cells). This crate decides only how it looks and writes the file, with
[`rust_xlsxwriter`](https://crates.io/crates/rust_xlsxwriter), in pure
Rust: no Excel is needed on the computer.

## What is in it

| Item | Contents |
|---|---|
| `write_workbook` | Writes a workbook to a path, replacing a file that is there. The file is made in memory first, then written with `write_whole_file`: through a temporary file that is renamed, and never through a symbolic link. |
| `write_whole_file` | Writes bytes to a path through a temporary file of a unique name in the same folder, flushed and then renamed, so that a half-written file is never left. A symbolic link at the path is refused. The report of a project is written with it ([decision 0042](../../docs/decisions/0042-the-report-of-a-whole-project.md)). |
| `to_bytes` | The bytes of the file, for whoever wants to keep them elsewhere. |
| `WriteError` | Why a file could not be written, in plain words: it is open in another program or may not be changed (`InUse`), the disk is full (`DiskFull`), its folder is gone (`NoFolder`), or something else, which is named (`Failed`). |

## The look

- One typeface, Calibri, 11 points; the title of an overview at 18.
- Warm, quiet colours taken from the program: dark ink, muted labels,
  hairlines under the rows, a header row in bold on a faint fill.
- No colour but the status of a task: its name in a deep shade of its
  colour on a pale one.
- Every cell has its real type. Dates are dates shown as `13 Nov 2026`
  (Excel writes the month in the language of the computer that opens the
  file), whole numbers have a separator for thousands, hours have one
  decimal.
- A list (Tasks, Sprints, Team) has its header row frozen and a filter on
  it. A column of numbers is set right with its name; a date in a list is
  set in a little. Text that is long wraps in its column.
- An overview (Summary) has no grid: a title, then sections under bold
  headings, labels in the muted colour, numbers standing left beside them.
- Every column is as wide as its widest cell and a little air, between 8
  and 40 characters, or 60 for a column that wraps.

## Tests

```bash
cargo test -p teamwright-workbook
```

The tests write a workbook and read it back with `calamine`, as another
program would: the order of the sheets, the header, and every kind of cell
with its type. They also write a real file, replace one, and refuse a
folder that does not exist.
