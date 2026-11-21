//! The digests of earlier conversations, in real SQLite databases: what is
//! stored, what is logged, what goes with a project, and what reaches
//! another computer.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{DigestRepository, NewProject, ProjectService};
use teamwright_domain::{Digest, DigestSource, DigestText, DigestTitle, Project, Timestamp};
use teamwright_storage::{
    Database, SqliteAreaRepository, SqliteConversationRepository, SqliteDigestRepository,
    SqliteMemberRepository, SqlitePlacesRepository, SqliteProjectRepository,
    SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
};

fn moment(minutes: i64) -> Timestamp {
    Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap() + TimeDelta::minutes(minutes)
}

/// One computer: its database, its projects and its digests.
struct Computer {
    database: Arc<Database>,
    projects: ProjectService,
    digests: SqliteDigestRepository,
}

impl Computer {
    fn new() -> Self {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
        Self {
            projects: ProjectService::new(
                Arc::new(SqliteProjectRepository::new(database.clone())),
                tasks.clone(),
                Arc::new(SqliteMemberRepository::new(database.clone())),
                Arc::new(SqliteConversationRepository::new(database.clone())),
                Arc::new(SqlitePlacesRepository::new(database.clone())),
                Arc::new(SqliteSprintRepository::new(database.clone())),
                Arc::new(SqliteViewRepository::new(database.clone())),
                Arc::new(SqliteAreaRepository::new(database.clone())),
                Arc::new(ManualClock::starting_at(moment(0))),
            ),
            digests: SqliteDigestRepository::new(database.clone()),
            database,
        }
    }

    fn take_in_from(&self, other: &Self) {
        let operations = other.database.operations_to_send(0).unwrap();
        self.database.take_in(&operations).unwrap();
    }

    fn project(&self) -> Project {
        self.projects
            .create(NewProject {
                name: "Air quality node".to_owned(),
                ..NewProject::default()
            })
            .unwrap()
    }

    fn digests_of(&self, project: &Project) -> Vec<Digest> {
        self.digests.list_for_project(project.id()).unwrap()
    }

    /// The entity and the kind of every operation logged after the first
    /// `from`.
    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }
}

fn digest(project: &Project, title: &str, minute: i64, held: bool) -> Digest {
    Digest::keep(
        project.id(),
        DigestSource::new(&format!("-home-ada-air/{title}"), "/home/ada/air").unwrap(),
        DigestTitle::parse(title).unwrap(),
        DigestText::parse("Held in /home/ada/air.\n\nFirst request:\nPlan the sensor").unwrap(),
        if held {
            (Some(moment(-600)), Some(moment(-60)))
        } else {
            (None, None)
        },
        moment(minute),
    )
}

#[test]
fn a_digest_is_stored_logged_and_read_back_unchanged() {
    let desk = Computer::new();
    let project = desk.project();
    let before = desk.database.operations().unwrap().len();
    let kept = digest(&project, "Sensor plan", 1, true);
    let without_moments = digest(&project, "Enclosure", 2, false);

    desk.digests.save(&kept).unwrap();
    desk.digests.save(&without_moments).unwrap();

    assert_eq!(
        desk.logged_since(before),
        [
            ("digest".to_owned(), "upsert".to_owned()),
            ("digest".to_owned(), "upsert".to_owned())
        ]
    );
    assert_eq!(desk.digests.find(kept.id()).unwrap(), Some(kept.clone()));
    let mut listed = desk.digests_of(&project);
    listed.sort_by_key(Digest::kept_at);
    assert_eq!(listed, [kept, without_moments]);
}

#[test]
fn a_removed_digest_is_gone_and_its_removal_is_logged() {
    let desk = Computer::new();
    let project = desk.project();
    let kept = digest(&project, "Sensor plan", 1, true);
    desk.digests.save(&kept).unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.digests.delete(kept.id()).unwrap();
    desk.digests.delete(kept.id()).unwrap();

    assert_eq!(
        desk.logged_since(before),
        [("digest".to_owned(), "delete".to_owned())]
    );
    assert!(desk.digests_of(&project).is_empty());
}

#[test]
fn digests_go_with_their_project_and_each_removal_is_logged() {
    let desk = Computer::new();
    let project = desk.project();
    desk.digests
        .save(&digest(&project, "Sensor plan", 1, true))
        .unwrap();
    let before = desk.database.operations().unwrap().len();

    desk.projects.delete(project.id()).unwrap();

    assert!(
        desk.logged_since(before)
            .contains(&("digest".to_owned(), "delete".to_owned()))
    );
    assert!(desk.digests_of(&project).is_empty());
}

#[test]
fn a_digest_reaches_the_other_computer_and_so_does_its_removal() {
    let desk = Computer::new();
    let laptop = Computer::new();
    let project = desk.project();
    let kept = digest(&project, "Sensor plan", 1, true);
    desk.digests.save(&kept).unwrap();

    laptop.take_in_from(&desk);
    assert_eq!(laptop.digests_of(&project), std::slice::from_ref(&kept));

    laptop.digests.delete(kept.id()).unwrap();
    desk.take_in_from(&laptop);
    assert!(desk.digests_of(&project).is_empty());
}
