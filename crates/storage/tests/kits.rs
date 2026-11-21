//! The libraries of skills and connectors and the kits of the specialists
//! in real SQLite databases: what is stored, what is logged, what goes with
//! a project, and what reaches another computer
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use teamwright_application::{
    FileSystem, KitChoice, KitRepository, KitService, NewConnector, NewConnectorStart, NewMember,
    NewProject, PretendShelf, ProjectService, StorageError, SyncPorts, SyncService, SystemClock,
    TeamService,
};
use teamwright_domain::{MemberKind, Project};
use teamwright_storage::{
    Database, FolderOfComputers, SqliteAreaRepository, SqliteConversationRepository,
    SqliteKitRepository, SqliteMemberRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteSyncLedger, SqliteTaskRepository, SqliteViewRepository,
};
use tempfile::TempDir;

const REVIEW: &str = "---\nname: pcb-review\ndescription: Reviews a board layout.\n---\nLook.\n";

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

/// One computer with a database of its own, the use cases, and syncing.
struct Computer {
    _data: TempDir,
    database: Arc<Database>,
    projects: ProjectService,
    team: TeamService,
    kits: KitService,
    shelf: Arc<PretendShelf>,
    sync: SyncService,
}

impl Computer {
    fn new(name: &str, root: &Path) -> Self {
        let data = tempfile::tempdir().unwrap();
        let database = Arc::new(Database::open(data.path().join("teamwright.db")).unwrap());
        let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
        let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
        let members = Arc::new(SqliteMemberRepository::new(database.clone()));
        let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
        let views = Arc::new(SqliteViewRepository::new(database.clone()));
        let clock = Arc::new(SystemClock);
        let shelf = Arc::new(PretendShelf::default());
        let sync = SyncService::new(SyncPorts {
            ledger: Arc::new(SqliteSyncLedger::new(database.clone())),
            folder: Arc::new(FolderOfComputers::new()),
            file_system: Arc::new(Disk),
            clock: clock.clone(),
            version: "0.1.0".to_owned(),
            data_folder: data.path().to_path_buf(),
        });
        sync.choose_folder(root, name).unwrap();
        Self {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(SqlitePlacesRepository::new(database.clone())),
                Arc::new(SqliteSprintRepository::new(database.clone())),
                views.clone(),
                Arc::new(SqliteAreaRepository::new(database.clone())),
                clock.clone(),
            ),
            team: TeamService::new(
                projects.clone(),
                members.clone(),
                tasks,
                conversations,
                views,
                clock.clone(),
            ),
            kits: KitService::new(
                Arc::new(SqliteKitRepository::new(database.clone())),
                shelf.clone(),
                projects,
                members,
                clock,
            ),
            shelf,
            sync,
            database,
            _data: data,
        }
    }

    fn round(&self) -> usize {
        let report = self.sync.round().unwrap();
        for found in &report.new_computers {
            self.sync.accept(*found).unwrap();
        }
        report.written + report.taken_in + report.new_computers.len()
    }

    fn logged_entities_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn logged_states(&self) -> String {
        self.database
            .operations()
            .unwrap()
            .iter()
            .map(|operation| format!("{operation:?}"))
            .collect()
    }
}

fn settle(computers: &[&Computer]) {
    for _ in 0..8 {
        let moved: usize = computers.iter().map(|computer| computer.round()).sum();
        if moved == 0 {
            return;
        }
    }
    panic!("the computers did not settle");
}

/// A project with a specialist that was given a skill and a connector.
fn equipped(computer: &Computer) -> (Project, teamwright_domain::Member) {
    let project = computer
        .projects
        .create(NewProject {
            name: "Smart sensor node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    let specialist = computer
        .team
        .add(NewMember {
            project_id: project.id(),
            name: "Hardware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    computer.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = computer
        .kits
        .add_skill(project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let connector = computer
        .kits
        .add_connector(
            project.id(),
            NewConnector {
                name: "tickets".to_owned(),
                start: NewConnectorStart::Address {
                    url: "https://tickets.example/mcp".to_owned(),
                    headers: vec![teamwright_application::NewHeader {
                        name: "Authorization".to_owned(),
                        variable: "TICKETS_TOKEN".to_owned(),
                        bearer: true,
                    }],
                },
                tools: vec!["read_ticket".to_owned(), "close_ticket".to_owned()],
            },
        )
        .unwrap();
    computer
        .kits
        .set_kit(
            specialist.id(),
            KitChoice {
                skills: vec![skill.id()],
                connectors: vec![(connector.id(), vec!["read_ticket".to_owned()])],
            },
        )
        .unwrap();
    (project, specialist)
}

#[test]
fn the_library_and_a_kit_are_stored_and_every_change_is_logged() {
    let root = tempfile::tempdir().unwrap();
    let desk = Computer::new("Desk", root.path());
    let from = desk.database.operations().unwrap().len();

    let (project, specialist) = equipped(&desk);

    let logged = desk.logged_entities_since(from);
    for entity in ["skill", "connector", "member_kit"] {
        assert!(
            logged
                .iter()
                .any(|(logged, kind)| logged == entity && kind == "upsert"),
            "{entity} was not logged: {logged:?}"
        );
    }
    let library = desk.kits.library(project.id()).unwrap();
    assert_eq!(library.skills.len(), 1);
    assert_eq!(library.connectors.len(), 1);
    assert_eq!(library.kits.len(), 1);
    assert_eq!(library.kits[0].member_id(), specialist.id());
    assert_eq!(
        library.kits[0].connectors()[0].tools()[0].as_str(),
        "read_ticket"
    );
    // A secret is named, never kept.
    assert!(desk.logged_states().contains("TICKETS_TOKEN"));
}

#[test]
fn removing_from_the_library_is_logged_with_the_kits_that_change() {
    let root = tempfile::tempdir().unwrap();
    let desk = Computer::new("Desk", root.path());
    let (project, specialist) = equipped(&desk);
    let library = desk.kits.library(project.id()).unwrap();
    let from = desk.database.operations().unwrap().len();

    desk.kits
        .remove_connector(project.id(), library.connectors[0].connector.id())
        .unwrap();

    assert_eq!(
        desk.logged_entities_since(from),
        [
            ("member_kit".to_owned(), "upsert".to_owned()),
            ("connector".to_owned(), "delete".to_owned()),
        ]
    );
    let kit = SqliteKitRepository::new(desk.database.clone())
        .kit_of(specialist.id())
        .unwrap()
        .unwrap();
    assert!(kit.connectors().is_empty());
    assert_eq!(kit.skills().len(), 1);
}

#[test]
fn deleting_a_project_deletes_and_logs_its_library_and_kits() {
    let root = tempfile::tempdir().unwrap();
    let desk = Computer::new("Desk", root.path());
    let (project, _) = equipped(&desk);
    let from = desk.database.operations().unwrap().len();

    desk.projects.delete(project.id()).unwrap();

    let logged = desk.logged_entities_since(from);
    for entity in ["skill", "connector", "member_kit"] {
        assert!(
            logged
                .iter()
                .any(|(logged, kind)| logged == entity && kind == "delete"),
            "{entity} was not deleted: {logged:?}"
        );
    }
    let kits = SqliteKitRepository::new(desk.database.clone());
    assert!(kits.skills_of_project(project.id()).unwrap().is_empty());
    assert!(kits.kits_of_project(project.id()).unwrap().is_empty());
}

#[test]
fn another_computer_takes_in_the_library_but_has_no_copy_of_the_skill() {
    let root = tempfile::tempdir().unwrap();
    let desk = Computer::new("Desk", root.path());
    let laptop = Computer::new("Laptop", root.path());
    let (project, specialist) = equipped(&desk);

    settle(&[&desk, &laptop]);

    let there = laptop.kits.library(project.id()).unwrap();
    assert_eq!(there.skills.len(), 1);
    assert_eq!(there.skills[0].skill.name().as_str(), "pcb-review");
    // The folder belongs to the desk; the laptop has to be given it.
    assert!(!there.skills[0].here);
    assert_eq!(there.connectors.len(), 1);
    assert_eq!(there.connectors[0].missing_variables.len(), 1);
    assert_eq!(there.kits.len(), 1);
    assert_eq!(there.kits[0].member_id(), specialist.id());
    assert_eq!(there.kits[0].skills(), [there.skills[0].skill.id()]);
}
