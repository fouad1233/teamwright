//! Keeping computers in step through a real folder: real databases in
//! temporary folders of their own, a `SyncService` for each, and one
//! temporary sync folder.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use teamwright_application::{
    Acceptance, FileSystem, NewProject, NewTask, ProjectService, StorageError, SyncPorts,
    SyncService, SystemClock, TaskRepository, TaskService,
};
use teamwright_domain::{Priority, Project, Task};
use teamwright_storage::{
    Database, FolderOfComputers, SqliteConversationRepository, SqliteMemberRepository,
    SqlitePlacesRepository, SqliteProjectRepository, SqliteSprintRepository, SqliteSyncLedger,
    SqliteTaskRepository, SqliteViewRepository,
};
use tempfile::TempDir;

/// The folders of this computer, through the standard library.
struct Disk;

impl FileSystem for Disk {
    fn is_folder(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn resolve(&self, path: &Path) -> Result<PathBuf, StorageError> {
        fs::canonicalize(path).map_err(|error| StorageError::caused_by("cannot resolve", error))
    }

    fn file_names_in(&self, _folder: &Path) -> Result<Vec<String>, StorageError> {
        Ok(Vec::new())
    }

    fn copy_file(&self, _from: &Path, _to: &Path) -> Result<(), StorageError> {
        Ok(())
    }
}

/// One computer: a data folder of its own with a database in it, the use
/// cases on it, and syncing.
struct Computer {
    data: TempDir,
    database: Arc<Database>,
    projects: ProjectService,
    tasks: TaskService,
    sync: SyncService,
}

impl Computer {
    /// A new computer named `name` that syncs through `root`.
    fn new(name: &str, root: &Path) -> Self {
        let data = tempfile::tempdir().unwrap();
        let computer = Self::open(data);
        computer.sync.choose_folder(root, name).unwrap();
        computer
    }

    /// Opens the database in a data folder, as Teamwright does when it
    /// starts.
    fn open(data: TempDir) -> Self {
        let database = Arc::new(Database::open(data.path().join("teamwright.db")).unwrap());
        let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
        let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
        let members = Arc::new(SqliteMemberRepository::new(database.clone()));
        let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
        let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
        let views = Arc::new(SqliteViewRepository::new(database.clone()));
        let clock = Arc::new(SystemClock);
        let sync = SyncService::new(SyncPorts {
            ledger: Arc::new(SqliteSyncLedger::new(database.clone())),
            folder: Arc::new(FolderOfComputers::new()),
            file_system: Arc::new(Disk),
            clock: clock.clone(),
            version: "0.1.0".to_owned(),
            data_folder: data.path().to_path_buf(),
        });
        Self {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(SqlitePlacesRepository::new(database.clone())),
                sprints.clone(),
                views.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(projects, tasks, members, conversations, sprints, clock),
            sync,
            database,
            data,
        }
    }

    /// Runs a round, and accepts every computer it found for the first
    /// time. Returns how many operations were written, and how many were
    /// taken in, counting a computer found for the first time as one.
    fn round(&self) -> (usize, usize) {
        let report = self.sync.round().unwrap();
        for found in &report.new_computers {
            self.sync.accept(*found).unwrap();
        }
        (report.written, report.taken_in + report.new_computers.len())
    }

    /// Everything the manager works with, in a form that can be compared.
    fn everything(&self) -> String {
        let mut projects = self.projects.list().unwrap();
        projects.sort_by_key(Project::id);
        let mut text = String::new();
        for project in projects {
            text.push_str(&format!("{project:?}\n"));
            let mut tasks = self.tasks.list_for_project(project.id()).unwrap();
            tasks.sort_by_key(Task::id);
            for task in tasks {
                text.push_str(&format!("  {task:?}\n"));
            }
        }
        text
    }

    fn task(&self, task: &Task) -> Option<Task> {
        SqliteTaskRepository::new(self.database.clone())
            .find(task.id())
            .unwrap()
    }
}

/// Lets computers run rounds in turn until none writes or takes in
/// anything. Returns how many operations were taken in in all.
fn settle(computers: &[&Computer]) -> usize {
    let mut taken_in_all = 0;
    for _ in 0..8 {
        let mut moved = 0;
        for computer in computers {
            let (written, taken_in) = computer.round();
            moved += written + taken_in;
            taken_in_all += taken_in;
        }
        if moved == 0 {
            return taken_in_all;
        }
    }
    panic!("the computers did not settle");
}

/// A desk with a project and a task, and a laptop that has taken them in,
/// through one sync folder.
fn desk_and_laptop(root: &Path) -> (Computer, Computer, Project, Task) {
    let desk = Computer::new("Desk", root);
    let laptop = Computer::new("Laptop", root);
    let project = desk
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let task = desk
        .tasks
        .create(NewTask::titled(project.id(), "Design the board"))
        .unwrap();
    settle(&[&desk, &laptop]);
    assert_eq!(laptop.everything(), desk.everything());
    (desk, laptop, project, task)
}

#[test]
fn a_computer_takes_in_what_another_wrote_once_it_is_accepted() {
    let root = tempfile::tempdir().unwrap();
    let desk = Computer::new("Desk", root.path());
    let laptop = Computer::new("Laptop", root.path());
    let project = desk
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();

    desk.sync.round().unwrap();
    let found = laptop.sync.round().unwrap();
    assert_eq!(found.new_computers, [desk.database.device_id()]);
    assert_eq!(
        found.taken_in, 0,
        "a computer that is not accepted is not read"
    );
    let state = laptop.sync.state().unwrap();
    assert_eq!(state.computers[0].name.as_deref(), Some("Desk"));
    assert_eq!(state.computers[0].acceptance, Acceptance::Undecided);

    laptop.sync.accept(desk.database.device_id()).unwrap();
    let taken = laptop.sync.round().unwrap();

    assert!(taken.taken_in > 0);
    assert_eq!(taken.changed_projects, [project.id()]);
    assert_eq!(laptop.everything(), desk.everything());
    let copies: Vec<_> = fs::read_dir(laptop.data.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("teamwright-before-sync-")
        })
        .collect();
    assert_eq!(
        copies.len(),
        1,
        "the database is copied before the first take-in"
    );
}

#[test]
fn changes_made_apart_meet_in_either_order_and_a_second_exchange_changes_nothing() {
    for desk_first in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let (desk, laptop, project, task) = desk_and_laptop(root.path());
        desk.tasks.rename(task.id(), "Route the board").unwrap();
        laptop
            .tasks
            .prioritize(task.id(), Priority::Urgent)
            .unwrap();
        desk.projects
            .rename(project.id(), "Sensor node, second board")
            .unwrap();
        laptop
            .tasks
            .create(NewTask::titled(project.id(), "Write the firmware"))
            .unwrap();

        let order: [&Computer; 2] = if desk_first {
            [&desk, &laptop]
        } else {
            [&laptop, &desk]
        };
        settle(&order);
        let after_first = desk.everything();
        let taken_again = settle(&order);

        assert_eq!(laptop.everything(), desk.everything());
        assert_eq!(desk.everything(), after_first);
        assert_eq!(taken_again, 0);
        let merged = desk.task(&task).unwrap();
        assert_eq!(merged.title().as_str(), "Route the board");
        assert_eq!(merged.priority(), Priority::Urgent);
        assert_eq!(
            laptop.projects.get(project.id()).unwrap().name().as_str(),
            "Sensor node, second board"
        );
    }
}

#[test]
fn tasks_numbered_alike_on_two_computers_end_up_numbered_apart_on_both() {
    let root = tempfile::tempdir().unwrap();
    let (desk, laptop, project, _) = desk_and_laptop(root.path());
    desk.tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    laptop
        .tasks
        .create(NewTask::titled(project.id(), "Test the radio"))
        .unwrap();

    settle(&[&desk, &laptop]);

    assert_eq!(laptop.everything(), desk.everything());
    let mut numbers: Vec<u32> = desk
        .tasks
        .list_for_project(project.id())
        .unwrap()
        .iter()
        .map(Task::number)
        .collect();
    numbers.sort_unstable();
    assert_eq!(numbers, [1, 2, 3]);
}

#[test]
fn a_task_deleted_on_one_computer_and_changed_on_the_other_is_gone_on_both() {
    let root = tempfile::tempdir().unwrap();
    let (desk, laptop, _, task) = desk_and_laptop(root.path());
    laptop.tasks.rename(task.id(), "Route the board").unwrap();
    desk.tasks.delete(task.id()).unwrap();

    settle(&[&desk, &laptop]);

    assert!(desk.task(&task).is_none());
    assert!(laptop.task(&task).is_none());
    assert_eq!(laptop.sync.notices().unwrap().len(), 1);
}

#[test]
fn a_third_computer_that_joins_late_takes_in_everything_from_both() {
    let root = tempfile::tempdir().unwrap();
    let (desk, laptop, project, _) = desk_and_laptop(root.path());
    laptop
        .tasks
        .create(NewTask::titled(project.id(), "Write the firmware"))
        .unwrap();
    settle(&[&desk, &laptop]);

    let workshop = Computer::new("Workshop", root.path());
    settle(&[&workshop, &desk, &laptop]);

    assert_eq!(workshop.everything(), desk.everything());
    assert_eq!(workshop.everything(), laptop.everything());
    assert_eq!(workshop.sync.state().unwrap().computers.len(), 2);
}

#[test]
fn a_copied_database_is_found_out_and_both_go_on_after_a_new_identity() {
    let root = tempfile::tempdir().unwrap();
    let (desk, laptop, project, task) = desk_and_laptop(root.path());

    // The data folder of the desk is copied, file for file, to another
    // computer, which starts with the same identity.
    let Computer { data, database, .. } = desk;
    drop(database);
    let copy_data = tempfile::tempdir().unwrap();
    for entry in fs::read_dir(data.path()).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), copy_data.path().join(entry.file_name())).unwrap();
    }
    let desk = Computer::open(data);
    let copy = Computer::open(copy_data);
    assert_eq!(copy.database.device_id(), desk.database.device_id());

    copy.tasks.rename(task.id(), "Renamed on the copy").unwrap();
    copy.round();
    desk.tasks.prioritize(task.id(), Priority::Urgent).unwrap();
    let error = desk.sync.round().unwrap_err();
    assert_eq!(error.code(), "identity_in_use");

    let old_identity = desk.database.device_id();
    desk.sync.new_identity().unwrap();
    assert_eq!(desk.sync.round().unwrap_err().code(), "identity_in_use");
    // Teamwright starts again with the new identity.
    let Computer { data, database, .. } = desk;
    drop(database);
    let desk = Computer::open(data);
    assert_ne!(desk.database.device_id(), old_identity);

    settle(&[&desk, &copy, &laptop]);

    assert_eq!(desk.everything(), copy.everything());
    assert_eq!(desk.everything(), laptop.everything());
    let kept = laptop.task(&task).unwrap();
    assert_eq!(kept.title().as_str(), "Renamed on the copy");
    assert_eq!(kept.priority(), Priority::Urgent);
    assert_eq!(
        laptop.projects.get(project.id()).unwrap().id(),
        project.id()
    );
}
