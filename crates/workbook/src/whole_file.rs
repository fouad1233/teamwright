//! Writing a file that the program makes for the manager, in one piece and
//! without following a link.
//!
//! A specialist may write in the work folder of a project, which is where
//! a report or a workbook is saved by default. A specialist could place a
//! symbolic link there under the name the save window suggests, and a plain
//! write would follow it and overwrite whatever it points at. So the file is
//! written under a name of its own, flushed, and then renamed to the wanted
//! name: a rename replaces what stands at the name, a link included, and
//! never writes through it. A link that stands at the name is refused all
//! the same, so that the manager is told instead of finding the link gone.

use std::fs::{self, OpenOptions};
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// How many names for the temporary file are tried before giving up. A name
/// that is taken is only possible when something else guesses ours.
const MOST_TRIES: u32 = 100;

/// Writes `bytes` to the file at `path`, replacing the file that is there.
///
/// The bytes go to a new file of a unique name in the same folder, are
/// flushed to the disk, and are then renamed to `path`, so that a reader
/// never sees a half-written file and a failure leaves an existing file as
/// it was. The temporary file is removed on a failure.
///
/// # Errors
///
/// Returns an error of the kind `Other` if a symbolic link stands at `path`,
/// and the error of the disk if anything else fails: the folder is not
/// there, the disk is full, the file may not be changed.
pub fn write_whole_file(path: &Path, bytes: &[u8]) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(found) if found.file_type().is_symlink() => {
            return Err(Error::other(
                "a link stands there, and Teamwright does not write through links",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let (temporary, mut file) = create_temporary_beside(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

/// Creates a new, empty file of a name nobody has in the folder of `path`,
/// and opens it for writing. `create_new` refuses a name that is taken, a
/// link included, so it never opens something that was there before.
fn create_temporary_beside(path: &Path) -> Result<(PathBuf, fs::File)> {
    let folder = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty());
    let name = path
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "there is no file name"))?
        .to_string_lossy();
    let moment = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    for attempt in 0..MOST_TRIES {
        let temporary_name = format!(".{name}.{}-{moment}-{attempt}.writing", process::id());
        let temporary = folder.map_or_else(
            || PathBuf::from(&temporary_name),
            |folder| folder.join(&temporary_name),
        );
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(Error::new(
        ErrorKind::AlreadyExists,
        "no free name was found for the temporary file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_a_new_file_and_leaves_nothing_else_in_the_folder() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("report.md");

        write_whole_file(&path, b"one").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"one");
        assert_eq!(names_in(folder.path()), ["report.md"]);
    }

    #[test]
    fn replaces_a_file_that_is_there() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("report.md");
        fs::write(&path, "old and longer than the new").unwrap();

        write_whole_file(&path, b"new").unwrap();

        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(names_in(folder.path()), ["report.md"]);
    }

    #[test]
    fn a_folder_that_is_not_there_is_not_found() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("gone").join("report.md");

        let error = write_whole_file(&path, b"x").unwrap_err();

        assert_eq!(error.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn a_directory_at_the_name_is_left_alone() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("report.md");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("inside.txt"), "kept").unwrap();

        assert!(write_whole_file(&path, b"x").is_err());

        assert_eq!(fs::read(path.join("inside.txt")).unwrap(), b"kept");
        // The temporary file is gone.
        assert_eq!(names_in(folder.path()), ["report.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_at_the_name_is_refused_and_what_it_points_at_is_not_written() {
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("precious.txt");
        fs::write(&target, "precious").unwrap();
        let path = folder.path().join("report.md");
        std::os::unix::fs::symlink(&target, &path).unwrap();

        let error = write_whole_file(&path, b"overwritten").unwrap_err();

        assert!(error.to_string().contains("link"));
        assert_eq!(fs::read(&target).unwrap(), b"precious");
        assert!(
            fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(names_in(folder.path()), ["precious.txt", "report.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_points_nowhere_is_refused_too() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("report.md");
        std::os::unix::fs::symlink(folder.path().join("missing"), &path).unwrap();

        assert!(write_whole_file(&path, b"x").is_err());

        assert!(!folder.path().join("missing").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_that_appears_after_the_check_is_replaced_and_not_followed() {
        // The rename replaces the entry at the name, so even a link placed
        // between the check and the rename writes nothing through itself.
        let folder = tempfile::tempdir().unwrap();
        let target = folder.path().join("precious.txt");
        fs::write(&target, "precious").unwrap();
        let path = folder.path().join("report.md");
        let (temporary, mut file) = create_temporary_beside(&path).unwrap();
        file.write_all(b"report").unwrap();
        drop(file);
        std::os::unix::fs::symlink(&target, &path).unwrap();

        fs::rename(&temporary, &path).unwrap();

        assert_eq!(fs::read(&target).unwrap(), b"precious");
        assert_eq!(fs::read(&path).unwrap(), b"report");
        assert!(
            !fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
