//! What lies in the folders of a project: the use case behind a view of the
//! kind `files`.
//!
//! The folders are the work folder and the reference folders of the project
//! (see [`crate::places`]). A path from the interface is never trusted: it
//! is read as a [`PathInFolder`], and after every link on the way is
//! followed, what it names must still lie inside its folder. Only then is
//! it read, or handed back to be opened or shown.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use teamwright_domain::{
    DomainError, EntryKind, FolderEntry, PathInFolder, ProjectId, project_files::listing_of,
};

use crate::error::ApplicationError;
use crate::places::PlacesService;
use crate::ports::{DiskEntry, FolderReader};

/// What lies in one folder, as the manager sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderListing {
    /// The entries: folders first, then by name, without the noise.
    pub entries: Vec<FolderEntry>,
    /// Whether the folder held more entries than are answered.
    pub has_more: bool,
}

/// The use cases of the files of a project.
#[derive(Clone)]
pub struct ProjectFilesService {
    places: PlacesService,
    reader: Arc<dyn FolderReader>,
}

impl ProjectFilesService {
    /// Creates the service.
    #[must_use]
    pub fn new(places: PlacesService, reader: Arc<dyn FolderReader>) -> Self {
        Self { places, reader }
    }

    /// What lies in a folder of a project, at a path inside it (empty for
    /// the folder itself). A link that leads outside the folder, or
    /// nowhere, is listed and not followed. Names that begin with a dot
    /// are shown only when `shows_hidden` is set; `.git`, `node_modules`
    /// and `target` never are.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if `folder` is not one
    /// of its folders, if the path leaves the folder, if no folder is at
    /// the path, or if the disk cannot be read.
    pub fn list(
        &self,
        project_id: ProjectId,
        folder: &Path,
        path: &str,
        shows_hidden: bool,
    ) -> Result<FolderListing, ApplicationError> {
        let place = self.locate_place(project_id, folder, path)?;
        if !self.reader.is_folder(&place.target) {
            return Err(ApplicationError::NothingAtPath {
                path: place.path.as_text(),
            });
        }
        let entries = self
            .reader
            .entries_in(&place.target)?
            .into_iter()
            .map(|entry| self.entry_of(&place, entry))
            .collect();
        let (entries, has_more) = listing_of(entries, shows_hidden);
        Ok(FolderListing { entries, has_more })
    }

    /// The full form of what lies at a path inside a folder of a project,
    /// checked, to be opened or shown by the system.
    ///
    /// # Errors
    ///
    /// The same as [`Self::list`], except that a file is accepted too.
    pub fn locate(
        &self,
        project_id: ProjectId,
        folder: &Path,
        path: &str,
    ) -> Result<PathBuf, ApplicationError> {
        Ok(self.locate_place(project_id, folder, path)?.target)
    }

    /// Checks the folder and the path, and finds what they name.
    fn locate_place(
        &self,
        project_id: ProjectId,
        folder: &Path,
        path: &str,
    ) -> Result<Place, ApplicationError> {
        let places = self.places.places(project_id)?;
        let is_a_folder_of_the_project = std::iter::once(&places.work_folder)
            .chain(&places.reference_folders)
            .any(|known| known == folder);
        if !is_a_folder_of_the_project {
            return Err(ApplicationError::NotAFolderOfProject {
                folder: folder.display().to_string(),
            });
        }
        let path = PathInFolder::parse(path)?;
        let root = self
            .reader
            .full_form(folder)
            .map_err(|_| ApplicationError::FolderNotFound {
                folder: folder.display().to_string(),
            })?;
        let named = path
            .parts()
            .iter()
            .fold(root.clone(), |above, part| above.join(part));
        let target =
            self.reader
                .full_form(&named)
                .map_err(|_| ApplicationError::NothingAtPath {
                    path: path.as_text(),
                })?;
        // A link on the way may lead elsewhere: what the path names in the
        // end must still lie inside the folder.
        if !target.starts_with(&root) {
            return Err(DomainError::PathLeavesFolder {
                path: path.as_text(),
            }
            .into());
        }
        Ok(Place { root, path, target })
    }

    /// An entry of a folder as the manager sees it. A link is followed only
    /// if it leads to a place inside the folder.
    fn entry_of(&self, place: &Place, entry: DiskEntry) -> FolderEntry {
        let stays_inside = !entry.is_link
            || self
                .reader
                .full_form(&place.target.join(&entry.name))
                .is_ok_and(|target| target.starts_with(&place.root));
        let kind = match (stays_inside, entry.is_folder) {
            (false, _) => EntryKind::Link,
            (true, true) => EntryKind::Folder,
            (true, false) => EntryKind::File,
        };
        FolderEntry {
            name: entry.name,
            kind,
            size: (kind == EntryKind::File).then_some(entry.size),
            changed: entry.changed,
        }
    }
}

/// A checked place inside a folder of a project.
struct Place {
    /// The full form of the folder.
    root: PathBuf,
    /// The path inside it, as it was asked for.
    path: PathInFolder,
    /// The full form of what the path names, inside the folder.
    target: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use chrono::TimeZone;

    use super::*;
    use crate::in_memory::{
        FixedProjectFolders, InMemoryConversationRepository, InMemoryMemberRepository,
        InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
        InMemoryTaskRepository, InMemoryViewRepository, ManualClock, PretendFileSystem,
    };
    use crate::ports::StorageError;
    use crate::projects::{NewProject, ProjectService, WorkflowTemplate};

    /// What lies at a path of the pretended disk.
    #[derive(Debug, Clone)]
    enum Node {
        Folder,
        File(u64),
        /// A link to a full path.
        Link(String),
    }

    /// A disk that exists only in what a test says: full paths written with
    /// `/`, each a folder, a file or a link.
    #[derive(Debug, Default)]
    struct PretendDisk {
        nodes: Mutex<BTreeMap<String, Node>>,
    }

    impl PretendDisk {
        fn put(&self, path: &str, node: Node) {
            self.nodes
                .lock()
                .unwrap()
                .insert(path.trim_end_matches('/').to_owned(), node);
        }

        /// The full form of a path, links followed, if something is there.
        fn follow(&self, path: &str) -> Option<String> {
            let nodes = self.nodes.lock().unwrap();
            let mut current = String::new();
            for part in path.split('/').filter(|part| !part.is_empty()) {
                current = format!("{current}/{part}");
                let mut hops = 0;
                while let Node::Link(target) = nodes.get(&current)? {
                    current.clone_from(target);
                    hops += 1;
                    if hops > 8 {
                        return None;
                    }
                }
            }
            Some(if current.is_empty() {
                "/".to_owned()
            } else {
                current
            })
        }

        fn text(path: &Path) -> String {
            path.to_string_lossy().replace('\\', "/")
        }
    }

    impl FolderReader for PretendDisk {
        fn full_form(&self, path: &Path) -> Result<PathBuf, StorageError> {
            self.follow(&Self::text(path))
                .map(PathBuf::from)
                .ok_or_else(|| StorageError::new(format!("nothing is at {}", path.display())))
        }

        fn is_folder(&self, path: &Path) -> bool {
            self.follow(&Self::text(path)).is_some_and(|full| {
                matches!(self.nodes.lock().unwrap().get(&full), Some(Node::Folder))
            })
        }

        fn entries_in(&self, folder: &Path) -> Result<Vec<DiskEntry>, StorageError> {
            let folder = Self::text(folder);
            let children: Vec<(String, Node)> = self
                .nodes
                .lock()
                .unwrap()
                .iter()
                .filter_map(|(path, node)| {
                    let name = path.strip_prefix(&format!("{folder}/"))?;
                    (!name.contains('/')).then(|| (name.to_owned(), node.clone()))
                })
                .collect();
            Ok(children
                .into_iter()
                .map(|(name, node)| {
                    let (is_link, followed) = match node {
                        Node::Link(_) => {
                            let full = self.follow(&format!("{folder}/{name}"));
                            let followed = full
                                .and_then(|full| self.nodes.lock().unwrap().get(&full).cloned());
                            (true, followed)
                        }
                        other => (false, Some(other)),
                    };
                    DiskEntry {
                        name,
                        is_link,
                        is_folder: matches!(followed, Some(Node::Folder)),
                        size: match followed {
                            Some(Node::File(size)) => size,
                            _ => 0,
                        },
                        changed: Some(chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap()),
                    }
                })
                .collect())
        }
    }

    struct Workspace {
        files: ProjectFilesService,
        places: PlacesService,
        project_id: ProjectId,
    }

    /// A project whose work folder is `/work`, with a reference folder
    /// `/sheets`, on a disk with a few files, a link inside the work folder
    /// and two that lead out of it.
    fn workspace() -> Workspace {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        let members = Arc::new(InMemoryMemberRepository::default());
        let conversations = Arc::new(InMemoryConversationRepository::default());
        let places_store = Arc::new(InMemoryPlacesRepository::default());
        let clock = Arc::new(ManualClock::starting_at(
            chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
        ));
        let project = ProjectService::new(
            projects.clone(),
            tasks.clone(),
            members.clone(),
            conversations.clone(),
            places_store.clone(),
            Arc::new(InMemorySprintRepository::new(tasks.clone())),
            Arc::new(InMemoryViewRepository::default()),
            Arc::new(crate::in_memory::InMemoryAreaRepository::default()),
            clock,
        )
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
        let places = PlacesService::new(
            projects,
            tasks,
            members,
            conversations,
            places_store,
            Arc::new(FixedProjectFolders::inside("/projects")),
            Arc::new(PretendFileSystem::with_folders(["/work", "/sheets"])),
        );
        places
            .set_work_folder(project.id(), Some(Path::new("/work")))
            .unwrap();
        places
            .add_reference_folder(project.id(), Path::new("/sheets"))
            .unwrap();

        let disk = PretendDisk::default();
        for folder in [
            "/work",
            "/work/firmware",
            "/work/firmware/src",
            "/work/.git",
            "/sheets",
        ] {
            disk.put(folder, Node::Folder);
        }
        disk.put("/work/firmware/src/main.c", Node::File(2048));
        disk.put("/work/README.md", Node::File(120));
        disk.put("/work/.env", Node::File(12));
        disk.put("/work/node_modules", Node::Folder);
        disk.put("/work/sources", Node::Link("/work/firmware/src".to_owned()));
        disk.put("/work/elsewhere", Node::Link("/secret".to_owned()));
        disk.put("/work/nowhere", Node::Link("/gone".to_owned()));
        disk.put("/secret", Node::Folder);
        disk.put("/secret/key.txt", Node::File(32));
        disk.put("/sheets/bme680.pdf", Node::File(900_000));

        Workspace {
            files: ProjectFilesService::new(places.clone(), Arc::new(disk)),
            places,
            project_id: project.id(),
        }
    }

    impl Workspace {
        fn list(&self, folder: &str, path: &str) -> Result<FolderListing, ApplicationError> {
            self.files
                .list(self.project_id, Path::new(folder), path, false)
        }
    }

    fn shown(listing: &FolderListing) -> Vec<(&str, EntryKind, Option<u64>)> {
        listing
            .entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.size))
            .collect()
    }

    #[test]
    fn lists_a_folder_with_folders_first_and_without_the_noise() {
        let workspace = workspace();

        let listing = workspace.list("/work", "").unwrap();

        assert_eq!(
            shown(&listing),
            [
                ("firmware", EntryKind::Folder, None),
                ("sources", EntryKind::Folder, None),
                ("elsewhere", EntryKind::Link, None),
                ("nowhere", EntryKind::Link, None),
                ("README.md", EntryKind::File, Some(120)),
            ]
        );
        assert!(!listing.has_more);
    }

    #[test]
    fn shows_the_hidden_names_when_asked_but_never_the_folder_of_git() {
        let workspace = workspace();

        let listing = workspace
            .files
            .list(workspace.project_id, Path::new("/work"), "", true)
            .unwrap();

        let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&".env"));
        assert!(!names.contains(&".git"));
        assert!(!names.contains(&"node_modules"));
    }

    #[test]
    fn lists_a_folder_inside_and_one_reached_through_a_link_that_stays_inside() {
        let workspace = workspace();

        let inside = workspace.list("/work", "firmware/src").unwrap();
        let through_link = workspace.list("/work", "sources").unwrap();

        assert_eq!(shown(&inside), [("main.c", EntryKind::File, Some(2048))]);
        assert_eq!(inside, through_link);
    }

    #[test]
    fn lists_a_reference_folder_of_the_project() {
        let workspace = workspace();

        let listing = workspace.list("/sheets", "").unwrap();

        assert_eq!(
            shown(&listing),
            [("bme680.pdf", EntryKind::File, Some(900_000))]
        );
    }

    #[test]
    fn refuses_a_path_that_goes_up_or_starts_elsewhere() {
        let workspace = workspace();

        for path in ["..", "firmware/../..", "/secret", "C:/secret"] {
            let error = workspace.list("/work", path).unwrap_err();
            assert_eq!(error.code(), "rule_broken", "{path}");
        }
    }

    #[test]
    fn refuses_to_follow_a_link_that_leads_outside_the_folder() {
        let workspace = workspace();

        let listing = workspace.list("/work", "elsewhere").unwrap_err();
        let located = workspace
            .files
            .locate(
                workspace.project_id,
                Path::new("/work"),
                "elsewhere/key.txt",
            )
            .unwrap_err();

        assert_eq!(listing.code(), "rule_broken");
        assert_eq!(
            listing.to_string(),
            "the path elsewhere does not stay inside the folder"
        );
        assert_eq!(located.code(), "rule_broken");
    }

    #[test]
    fn refuses_a_folder_that_is_not_one_of_the_project() {
        let workspace = workspace();

        let error = workspace.list("/secret", "").unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "/secret is not a folder of this project");
    }

    #[test]
    fn says_when_nothing_is_at_the_path() {
        let workspace = workspace();

        let missing = workspace.list("/work", "drawings").unwrap_err();
        let a_file = workspace.list("/work", "README.md").unwrap_err();
        let broken_link = workspace.list("/work", "nowhere").unwrap_err();

        for error in [missing, a_file, broken_link] {
            assert_eq!(error.code(), "not_found");
        }
    }

    #[test]
    fn locates_a_file_inside_a_folder_in_its_full_form() {
        let workspace = workspace();

        let file = workspace
            .files
            .locate(workspace.project_id, Path::new("/work"), "sources/main.c")
            .unwrap();

        assert_eq!(file, PathBuf::from("/work/firmware/src/main.c"));
    }

    #[test]
    fn refuses_a_folder_the_project_no_longer_has() {
        let workspace = workspace();
        workspace
            .places
            .set_work_folder(workspace.project_id, None)
            .unwrap();

        let error = workspace.list("/work", "").unwrap_err();

        assert_eq!(error.code(), "rule_broken");
    }
}
