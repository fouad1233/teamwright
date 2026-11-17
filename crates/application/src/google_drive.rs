//! Syncing through Google Drive, in one press.
//!
//! Teamwright keeps computers in step through a folder the manager owns
//! ([`crate::sync`]). Google's own program, Google Drive for desktop, keeps
//! a folder of the manager's Drive on the computer. This module finds that
//! folder, so that the manager does not have to look for it, and makes the
//! folder `Teamwright` inside it. There is no sign-in to Google and no
//! token: Google's program does the syncing, Teamwright only writes files.
//!
//! Where the folder is:
//!
//! - On Windows, Drive for desktop mounts a drive (usually `G:`) whose root
//!   holds `My Drive`, named in the language of the program, beside its own
//!   hidden folders. In the mode that mirrors files it keeps `My Drive` in
//!   the user's home; the older Backup and Sync kept `Google Drive` there.
//! - On macOS each account has a folder
//!   `~/Library/CloudStorage/GoogleDrive-<address>`, which holds `My Drive`;
//!   older versions kept `~/Google Drive`.
//! - On Linux there is no Drive for desktop, so nothing is looked for.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::ports::StorageError;

/// The name of the folder Teamwright keeps in the manager's Drive.
pub const FOLDER_IN_DRIVE: &str = "Teamwright";

/// The hidden folder Drive for desktop keeps at the root of what it mounts,
/// which tells its drive from any other.
const DRIVE_MARKER: &str = ".shortcut-targets-by-id";

/// The names of the folder of the manager's own files in Drive, in the
/// languages Drive for desktop is most often set to.
const MY_DRIVE_NAMES: &[&str] = &[
    "My Drive",
    "Meine Ablage",
    "Mon Drive",
    "Mi unidad",
    "Il mio Drive",
    "Meu Drive",
    "Mijn Drive",
    "Mój dysk",
    "Drive'ım",
    "マイドライブ",
];

/// The other folders at the root of a Drive that are not the manager's own
/// files: the drives shared with them and the backups of other computers.
const OTHER_DRIVE_FOLDERS: &[&str] = &[
    "Shared drives",
    "Other computers",
    "Geteilte Ablagen",
    "Weitere Computer",
    "Drive partagés",
    "Autres ordinateurs",
    "Unidades compartidas",
    "Otros ordenadores",
    "Drive condivisi",
    "Altri computer",
    "Drives compartilhados",
    "Outros computadores",
    "Gedeelde drives",
    "Andere computers",
];

/// The prefix of a folder of an account under `~/Library/CloudStorage`.
const CLOUD_STORAGE_PREFIX: &str = "GoogleDrive-";

/// The kind of system Teamwright runs on, as far as Drive for desktop cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux and every other system, for which there is no Drive for desktop.
    Other,
}

/// What finding the folder needs from the disk and the environment.
pub trait DriveSurroundings: Send + Sync {
    /// The system Teamwright runs on.
    fn operating_system(&self) -> OperatingSystem;

    /// The home folder of the user, if the system says.
    fn home(&self) -> Option<PathBuf>;

    /// The roots of the drives of the computer (Windows only: `C:\`,
    /// `G:\`, ...). Only drives that are there.
    fn drive_roots(&self) -> Vec<PathBuf>;

    /// Whether a folder is at the path.
    fn is_folder(&self, path: &Path) -> bool;

    /// The names of what lies in a folder, in no particular order. A folder
    /// that cannot be read has none.
    fn names_in(&self, folder: &Path) -> Vec<String>;

    /// The full form of a path that exists, in which two paths of one
    /// folder are equal; nothing if it cannot be reached.
    fn full_form(&self, path: &Path) -> Option<PathBuf>;

    /// Creates a folder whose parent exists.
    ///
    /// # Errors
    ///
    /// Returns an error if it cannot be created.
    fn create_folder(&self, path: &Path) -> Result<(), StorageError>;
}

/// A folder of the manager's Drive that Drive for desktop keeps on this
/// computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveFolder {
    /// Where it is.
    pub path: PathBuf,
    /// The address of the account, when the path names it (on macOS).
    pub account: Option<String>,
}

/// What was found of Google Drive on this computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriveFinding {
    /// One folder or more; with several accounts the manager chooses.
    Found(Vec<DriveFolder>),
    /// Drive for desktop is not on this computer, or keeps no folder.
    NotInstalled,
    /// There is no Drive for desktop for this system.
    NotForThisSystem,
}

/// What the page about computers shows of Google Drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleDrive {
    /// What was found.
    pub finding: DriveFinding,
    /// The folder of Drive that holds the sync folder, when syncing runs
    /// through Drive.
    pub syncs_through: Option<DriveFolder>,
}

/// Finds the folders of Google Drive on this computer.
#[must_use]
pub fn find_drive_folders(surroundings: &dyn DriveSurroundings) -> DriveFinding {
    let candidates = match surroundings.operating_system() {
        OperatingSystem::Other => return DriveFinding::NotForThisSystem,
        OperatingSystem::Windows => on_windows(surroundings),
        OperatingSystem::MacOs => on_mac(surroundings),
    };
    let mut found: Vec<(Option<PathBuf>, DriveFolder)> = Vec::new();
    for folder in candidates {
        let full = surroundings.full_form(&folder.path);
        let seen = found
            .iter()
            .any(|(other, known)| known.path == folder.path || (full.is_some() && *other == full));
        if !seen {
            found.push((full, folder));
        }
    }
    if found.is_empty() {
        DriveFinding::NotInstalled
    } else {
        DriveFinding::Found(found.into_iter().map(|(_, folder)| folder).collect())
    }
}

/// The drives Drive for desktop mounts, then the folders under the home.
fn on_windows(surroundings: &dyn DriveSurroundings) -> Vec<DriveFolder> {
    let mut folders: Vec<DriveFolder> = surroundings
        .drive_roots()
        .iter()
        .filter_map(|root| {
            let is_drive = surroundings
                .names_in(root)
                .iter()
                .any(|name| name == DRIVE_MARKER);
            my_drive_in(surroundings, root, is_drive)
        })
        .map(|path| DriveFolder {
            path,
            account: None,
        })
        .collect();
    folders.extend(in_home(surroundings));
    folders
}

/// A folder for each account under `~/Library/CloudStorage`, then the
/// folder of the older versions.
fn on_mac(surroundings: &dyn DriveSurroundings) -> Vec<DriveFolder> {
    let Some(home) = surroundings.home() else {
        return Vec::new();
    };
    let storage = home.join("Library").join("CloudStorage");
    let mut accounts: Vec<String> = surroundings
        .names_in(&storage)
        .into_iter()
        .filter(|name| name.starts_with(CLOUD_STORAGE_PREFIX))
        .collect();
    accounts.sort();
    let mut folders: Vec<DriveFolder> = accounts
        .iter()
        .filter_map(|name| {
            let root = storage.join(name);
            let path = my_drive_in(surroundings, &root, true)?;
            let account = name
                .strip_prefix(CLOUD_STORAGE_PREFIX)
                .filter(|address| address.contains('@'))
                .map(str::to_owned);
            Some(DriveFolder { path, account })
        })
        .collect();
    folders.extend(in_home(surroundings));
    folders
}

/// The folders Drive keeps in the home: `My Drive` of the mode that
/// mirrors, and `Google Drive` of the older programs.
fn in_home(surroundings: &dyn DriveSurroundings) -> Vec<DriveFolder> {
    let Some(home) = surroundings.home() else {
        return Vec::new();
    };
    let mut folders = Vec::new();
    let mirrored = home.join("My Drive");
    if surroundings.is_folder(&mirrored) {
        folders.push(mirrored);
    }
    let older = home.join("Google Drive");
    if surroundings.is_folder(&older) {
        folders.push(my_drive_in(surroundings, &older, false).unwrap_or(older));
    }
    folders
        .into_iter()
        .map(|path| DriveFolder {
            path,
            account: None,
        })
        .collect()
}

/// The folder of the manager's own files inside the root of a Drive: the
/// one with a known name, or, when the root is surely a Drive, the one
/// folder that is neither hidden nor one of Drive's others.
fn my_drive_in(
    surroundings: &dyn DriveSurroundings,
    root: &Path,
    is_surely_drive: bool,
) -> Option<PathBuf> {
    let names = surroundings.names_in(root);
    let folders: Vec<&String> = names
        .iter()
        .filter(|name| surroundings.is_folder(&root.join(name)))
        .collect();
    if let Some(known) = MY_DRIVE_NAMES
        .iter()
        .find(|known| folders.iter().any(|name| name == *known))
    {
        return Some(root.join(known));
    }
    if !is_surely_drive {
        return None;
    }
    let mut own = folders.iter().filter(|name| {
        !name.starts_with('.')
            && !name.starts_with('$')
            && !OTHER_DRIVE_FOLDERS.contains(&name.as_str())
            && name.as_str() != "System Volume Information"
    });
    match (own.next(), own.next()) {
        (Some(only), None) => Some(root.join(only)),
        _ => None,
    }
}

/// The use cases of syncing through Google Drive.
#[derive(Clone)]
pub struct GoogleDriveService {
    surroundings: Arc<dyn DriveSurroundings>,
}

impl GoogleDriveService {
    /// Creates the service.
    #[must_use]
    pub fn new(surroundings: Arc<dyn DriveSurroundings>) -> Self {
        Self { surroundings }
    }

    /// What there is of Google Drive on this computer, and whether the sync
    /// folder lies inside it.
    #[must_use]
    pub fn look(&self, sync_folder: Option<&Path>) -> GoogleDrive {
        let finding = find_drive_folders(self.surroundings.as_ref());
        let syncs_through = match (&finding, sync_folder) {
            (DriveFinding::Found(folders), Some(sync_folder)) => {
                let sync_folder = self.full_or_given(sync_folder);
                folders
                    .iter()
                    .find(|folder| sync_folder.starts_with(self.full_or_given(&folder.path)))
                    .cloned()
            }
            _ => None,
        };
        GoogleDrive {
            finding,
            syncs_through,
        }
    }

    /// Makes the folder `Teamwright` inside a folder of Drive that was
    /// found, if it is not there, and answers with it: the folder to sync
    /// through.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not one of Drive that was found on
    /// this computer (`folder_not_found`), or if the folder `Teamwright`
    /// cannot be made.
    pub fn prepare(&self, drive_folder: &Path) -> Result<PathBuf, ApplicationError> {
        let DriveFinding::Found(folders) = find_drive_folders(self.surroundings.as_ref()) else {
            return Err(Self::not_of_drive(drive_folder));
        };
        let given = self.full_or_given(drive_folder);
        let folder = folders
            .into_iter()
            .find(|folder| folder.path == drive_folder || self.full_or_given(&folder.path) == given)
            .ok_or_else(|| Self::not_of_drive(drive_folder))?;
        let ours = folder.path.join(FOLDER_IN_DRIVE);
        if !self.surroundings.is_folder(&ours) {
            self.surroundings.create_folder(&ours)?;
        }
        Ok(ours)
    }

    fn full_or_given(&self, path: &Path) -> PathBuf {
        self.surroundings
            .full_form(path)
            .unwrap_or_else(|| path.to_path_buf())
    }

    fn not_of_drive(folder: &Path) -> ApplicationError {
        ApplicationError::NotAGoogleDriveFolder {
            folder: folder.display().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::Mutex;

    use super::*;

    /// A pretended computer: its system, its home, its drives and the
    /// folders on them.
    struct Pretended {
        system: OperatingSystem,
        home: Option<PathBuf>,
        drives: Vec<PathBuf>,
        folders: Mutex<BTreeSet<PathBuf>>,
    }

    impl Pretended {
        fn new(system: OperatingSystem, home: &str, drives: &[&str], folders: &[&str]) -> Self {
            Self {
                system,
                home: Some(PathBuf::from(home)),
                drives: drives.iter().map(PathBuf::from).collect(),
                // The folders above a folder are there too.
                folders: Mutex::new(
                    folders
                        .iter()
                        .flat_map(|folder| Path::new(folder).ancestors())
                        .filter(|path| path.parent().is_some())
                        .map(Path::to_path_buf)
                        .collect(),
                ),
            }
        }

        fn service(self) -> (GoogleDriveService, Arc<Self>) {
            let shared = Arc::new(self);
            (GoogleDriveService::new(shared.clone()), shared)
        }
    }

    impl DriveSurroundings for Pretended {
        fn operating_system(&self) -> OperatingSystem {
            self.system
        }
        fn home(&self) -> Option<PathBuf> {
            self.home.clone()
        }
        fn drive_roots(&self) -> Vec<PathBuf> {
            self.drives.clone()
        }
        fn is_folder(&self, path: &Path) -> bool {
            self.drives.iter().any(|drive| drive == path)
                || self.folders.lock().unwrap().contains(path)
        }
        fn names_in(&self, folder: &Path) -> Vec<String> {
            self.folders
                .lock()
                .unwrap()
                .iter()
                .filter(|path| path.parent() == Some(folder))
                .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
                .collect()
        }
        fn full_form(&self, path: &Path) -> Option<PathBuf> {
            self.is_folder(path).then(|| path.to_path_buf())
        }
        fn create_folder(&self, path: &Path) -> Result<(), StorageError> {
            self.folders.lock().unwrap().insert(path.to_path_buf());
            Ok(())
        }
    }

    fn found(finding: &DriveFinding) -> Vec<(String, Option<String>)> {
        match finding {
            DriveFinding::Found(folders) => folders
                .iter()
                // Joined on Windows, a path holds both kinds of slash.
                .map(|folder| {
                    let path = folder.path.display().to_string().replace('\\', "/");
                    (path, folder.account.clone())
                })
                .collect(),
            other => panic!("nothing found: {other:?}"),
        }
    }

    const HOME_WINDOWS: &str = "C:/Users/ada";

    #[test]
    fn on_windows_the_drive_that_holds_my_drive_is_found() {
        let pretended = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["C:/", "G:/"],
            &[
                "C:/Users",
                "G:/My Drive",
                "G:/Shared drives",
                "G:/.shortcut-targets-by-id",
            ],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [("G:/My Drive".to_owned(), None)]
        );
    }

    #[test]
    fn on_windows_my_drive_is_found_in_another_language() {
        let pretended = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["G:/"],
            &[
                "G:/Meine Ablage",
                "G:/Geteilte Ablagen",
                "G:/.shortcut-targets-by-id",
            ],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [("G:/Meine Ablage".to_owned(), None)]
        );
    }

    #[test]
    fn on_windows_an_unknown_language_is_found_by_the_marker_of_drive() {
        let pretended = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["G:/"],
            &[
                "G:/Mijn Schijf",
                "G:/.shortcut-targets-by-id",
                "G:/$RECYCLE.BIN",
            ],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [("G:/Mijn Schijf".to_owned(), None)]
        );
    }

    #[test]
    fn a_drive_without_the_marker_needs_the_known_name() {
        let pretended = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["D:/"],
            &["D:/Photos"],
        );
        assert_eq!(find_drive_folders(&pretended), DriveFinding::NotInstalled);
    }

    #[test]
    fn on_windows_the_folders_in_the_home_are_found_too() {
        let pretended = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["C:/"],
            &["C:/Users/ada/My Drive", "C:/Users/ada/Google Drive"],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [
                ("C:/Users/ada/My Drive".to_owned(), None),
                ("C:/Users/ada/Google Drive".to_owned(), None)
            ]
        );
    }

    #[test]
    fn on_macos_each_account_gives_a_folder_with_its_address() {
        let pretended = Pretended::new(
            OperatingSystem::MacOs,
            "/Users/ada",
            &[],
            &[
                "/Users/ada/Library/CloudStorage/GoogleDrive-ada@example.com/My Drive",
                "/Users/ada/Library/CloudStorage/GoogleDrive-ada@work.example/My Drive",
                "/Users/ada/Library/CloudStorage/OneDrive-Personal/Documents",
            ],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [
                (
                    "/Users/ada/Library/CloudStorage/GoogleDrive-ada@example.com/My Drive"
                        .to_owned(),
                    Some("ada@example.com".to_owned())
                ),
                (
                    "/Users/ada/Library/CloudStorage/GoogleDrive-ada@work.example/My Drive"
                        .to_owned(),
                    Some("ada@work.example".to_owned())
                )
            ]
        );
    }

    #[test]
    fn on_macos_the_older_folder_in_the_home_is_found() {
        let pretended = Pretended::new(
            OperatingSystem::MacOs,
            "/Users/ada",
            &[],
            &["/Users/ada/Google Drive"],
        );
        assert_eq!(
            found(&find_drive_folders(&pretended)),
            [("/Users/ada/Google Drive".to_owned(), None)]
        );
    }

    #[test]
    fn nothing_is_looked_for_on_linux() {
        let pretended = Pretended::new(
            OperatingSystem::Other,
            "/home/ada",
            &[],
            &["/home/ada/Google Drive"],
        );
        assert_eq!(
            find_drive_folders(&pretended),
            DriveFinding::NotForThisSystem
        );
    }

    #[test]
    fn without_drive_for_desktop_nothing_is_found() {
        let pretended = Pretended::new(OperatingSystem::Windows, HOME_WINDOWS, &["C:/"], &[]);
        assert_eq!(find_drive_folders(&pretended), DriveFinding::NotInstalled);
    }

    #[test]
    fn preparing_makes_the_folder_of_teamwright_once() {
        let (service, pretended) = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["G:/"],
            &["G:/My Drive", "G:/.shortcut-targets-by-id"],
        )
        .service();
        let ours = service.prepare(Path::new("G:/My Drive")).unwrap();
        assert_eq!(ours, PathBuf::from("G:/My Drive/Teamwright"));
        assert!(pretended.is_folder(&ours));
        assert_eq!(service.prepare(Path::new("G:/My Drive")).unwrap(), ours);
    }

    #[test]
    fn preparing_refuses_a_folder_that_is_not_of_drive() {
        let (service, pretended) = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["G:/"],
            &["G:/My Drive", "G:/.shortcut-targets-by-id", "C:/Elsewhere"],
        )
        .service();
        let refused = service.prepare(Path::new("C:/Elsewhere")).unwrap_err();
        assert_eq!(refused.code(), "folder_not_found");
        assert!(!pretended.is_folder(Path::new("C:/Elsewhere/Teamwright")));
    }

    #[test]
    fn the_look_says_when_syncing_runs_through_drive() {
        let (service, _) = Pretended::new(
            OperatingSystem::Windows,
            HOME_WINDOWS,
            &["G:/"],
            &["G:/My Drive", "G:/My Drive/Teamwright", "C:/Elsewhere"],
        )
        .service();
        let through = service.look(Some(Path::new("G:/My Drive/Teamwright")));
        assert_eq!(
            through.syncs_through.map(|folder| folder.path),
            Some(PathBuf::from("G:/My Drive"))
        );
        assert_eq!(
            service.look(Some(Path::new("C:/Elsewhere"))).syncs_through,
            None
        );
        assert_eq!(service.look(None).syncs_through, None);
    }
}
