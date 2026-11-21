//! Finding earlier conversations, making their digests and keeping them,
//! on stand-ins for storage and for the conversations of the computer.

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{EarlierConversation, Project};

use super::*;
use crate::in_memory::{
    FixedProjectFolders, InMemoryConversationRepository, InMemoryDigestRepository,
    InMemoryMemberRepository, InMemoryPlacesRepository, InMemoryProjectRepository,
    InMemorySprintRepository, InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
    PretendEarlierConversations, PretendFileSystem,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};

/// The folder the manager chose as the work folder of the project.
const WORK_FOLDER: &str = "/home/ada/air";

/// A folder the project may read.
const REFERENCE_FOLDER: &str = "/home/ada/datasheets";

struct Imports {
    digests: DigestService,
    conversations: Arc<PretendEarlierConversations>,
    clock: Arc<ManualClock>,
    project: Project,
}

fn moment(day: u32, hour: u32) -> Timestamp {
    chrono::Utc
        .with_ymd_and_hms(2026, 11, day, hour, 0, 0)
        .unwrap()
}

fn imports() -> Imports {
    let projects = Arc::new(InMemoryProjectRepository::default());
    let tasks = Arc::new(InMemoryTaskRepository::default());
    let members = Arc::new(InMemoryMemberRepository::default());
    let conversations = Arc::new(InMemoryConversationRepository::default());
    let places = Arc::new(InMemoryPlacesRepository::default());
    let file_system = Arc::new(PretendFileSystem::with_folders([
        WORK_FOLDER,
        REFERENCE_FOLDER,
    ]));
    let clock = Arc::new(ManualClock::starting_at(moment(21, 9)));
    let project = ProjectService::new(
        projects.clone(),
        tasks.clone(),
        members.clone(),
        conversations.clone(),
        places.clone(),
        Arc::new(InMemorySprintRepository::new(tasks.clone())),
        Arc::new(InMemoryViewRepository::default()),
        Arc::new(crate::in_memory::InMemoryAreaRepository::default()),
        clock.clone(),
    )
    .create(NewProject {
        name: "Air quality node".to_owned(),
        workflow: WorkflowTemplate::Engineering,
        ..NewProject::default()
    })
    .unwrap();
    let places = PlacesService::new(
        projects,
        tasks,
        members,
        conversations,
        places,
        Arc::new(FixedProjectFolders::inside("/documents/Teamwright")),
        file_system,
    );
    places
        .set_work_folder(project.id(), Some(Path::new(WORK_FOLDER)))
        .unwrap();
    places
        .add_reference_folder(project.id(), Path::new(REFERENCE_FOLDER))
        .unwrap();
    let earlier = Arc::new(PretendEarlierConversations::default());
    Imports {
        digests: DigestService::new(
            places,
            Arc::new(InMemoryDigestRepository::default()),
            earlier.clone(),
            clock.clone(),
        ),
        conversations: earlier,
        clock,
        project,
    }
}

impl Imports {
    /// Makes a conversation exist, held in a folder, that ended on a day of
    /// November and holds the given requests.
    fn held(&self, source: &str, folder: &str, ended_day: u32, requests: &[&str]) {
        self.conversations.add(
            FoundConversation {
                source: source.to_owned(),
                held_in: folder.to_owned(),
                title: None,
                first_request: requests.first().map(|request| (*request).to_owned()),
                started_at: Some(moment(ended_day, 8)),
                ended_at: Some(moment(ended_day, 17)),
                size_bytes: 2048,
            },
            EarlierConversation {
                source: source.to_owned(),
                held_in: folder.to_owned(),
                title: None,
                started_at: Some(moment(ended_day, 8)),
                ended_at: Some(moment(ended_day, 17)),
                requests: requests
                    .iter()
                    .map(|request| (*request).to_owned())
                    .collect(),
                changed_files: vec![format!("{folder}/notes.md")],
                last_answer: Some("Done.".to_owned()),
            },
        );
    }

    fn found(&self) -> Vec<ConversationsOfFolder> {
        self.digests.find_earlier(self.project.id()).unwrap()
    }

    fn keep(&self, source: &str) -> Result<Digest, ApplicationError> {
        let draft = self.digests.draft(self.project.id(), source)?;
        self.digests.keep(
            self.project.id(),
            DigestToKeep {
                source: draft.source,
                held_in: draft.held_in,
                title: draft.title.to_string(),
                text: draft.text,
                started_at: draft.started_at,
                ended_at: draft.ended_at,
            },
        )
    }
}

fn sources(folder: &ConversationsOfFolder) -> Vec<&str> {
    folder
        .conversations
        .iter()
        .map(|card| card.source.as_str())
        .collect()
}

#[test]
fn nothing_is_found_on_a_computer_where_claude_code_was_never_used() {
    let imports = imports();

    assert!(imports.found().is_empty());
}

#[test]
fn the_folders_of_the_project_come_first_then_the_others_with_the_newest_first() {
    let imports = imports();
    imports.held("other/old", "/home/ada/website", 1, &["Make a website"]);
    imports.held("other/new", "/home/ada/game", 20, &["Make a game"]);
    imports.held("air/a", WORK_FOLDER, 3, &["Plan the sensor"]);
    imports.held("air/b", WORK_FOLDER, 9, &["Choose a battery"]);
    imports.held("sheets/a", REFERENCE_FOLDER, 2, &["Read the datasheet"]);

    let found = imports.found();

    let folders: Vec<(&str, bool)> = found
        .iter()
        .map(|folder| (folder.folder.as_str(), folder.is_of_project))
        .collect();
    assert_eq!(
        folders,
        [
            (WORK_FOLDER, true),
            (REFERENCE_FOLDER, true),
            ("/home/ada/game", false),
            ("/home/ada/website", false),
        ]
    );
    assert_eq!(sources(&found[0]), ["air/b", "air/a"]);
}

#[test]
fn a_conversation_held_inside_a_folder_of_the_project_counts_as_the_projects() {
    let imports = imports();
    imports.held("air/firmware", "/home/ada/air/firmware", 3, &["Write it"]);
    imports.held("near/one", "/home/ada/airplane", 4, &["Fly"]);

    let found = imports.found();

    assert_eq!(found[0].folder, "/home/ada/air/firmware");
    assert!(found[0].is_of_project);
    assert!(
        !found[1].is_of_project,
        "a name that only begins alike is another folder"
    );
}

#[test]
fn the_list_shows_the_first_request_on_one_line_cut_with_secrets_hidden() {
    let imports = imports();
    let long = format!(
        "Use the key ghp_abcdefghijklmnop1234\nand {}",
        "go on ".repeat(100)
    );
    imports.held("air/a", WORK_FOLDER, 3, &[&long]);

    let card = &imports.found()[0].conversations[0];
    let shown = card.first_request.as_deref().unwrap();

    assert!(shown.starts_with("Use the key [hidden] and go on"));
    assert!(!shown.contains('\n'));
    assert!(shown.chars().count() <= FIRST_REQUEST_IN_LIST);
    assert_eq!(card.size_bytes, 2048);
}

#[test]
fn a_conversation_whose_digest_is_kept_says_so() {
    let imports = imports();
    imports.held("air/a", WORK_FOLDER, 3, &["Plan the sensor"]);
    imports.held("air/b", WORK_FOLDER, 4, &["Choose a battery"]);
    imports.keep("air/a").unwrap();

    let found = imports.found();

    let kept: Vec<(&str, bool)> = found[0]
        .conversations
        .iter()
        .map(|card| (card.source.as_str(), card.is_kept))
        .collect();
    assert_eq!(kept, [("air/b", false), ("air/a", true)]);
}

#[test]
fn a_draft_is_made_by_the_rules_and_nothing_is_kept() {
    let imports = imports();
    imports.held(
        "air/a",
        WORK_FOLDER,
        3,
        &["Plan the sensor", "Add a battery"],
    );

    let draft = imports
        .digests
        .draft(imports.project.id(), "air/a")
        .unwrap();

    assert_eq!(draft.title.as_str(), "Plan the sensor");
    assert!(draft.text.contains("First request:\nPlan the sensor"));
    assert!(draft.text.contains("2. Add a battery"));
    assert!(draft.text.contains("- notes.md"));
    assert!(
        imports
            .digests
            .digests_of(imports.project.id())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_draft_of_a_conversation_that_is_not_there_is_refused() {
    let imports = imports();

    let error = imports
        .digests
        .draft(imports.project.id(), "air/gone")
        .unwrap_err();

    assert_eq!(error.code(), "not_found");
}

#[test]
fn a_draft_of_a_conversation_without_requests_is_refused() {
    let imports = imports();
    imports.held("air/empty", WORK_FOLDER, 3, &[]);

    let error = imports
        .digests
        .draft(imports.project.id(), "air/empty")
        .unwrap_err();

    assert_eq!(error.code(), "rule_broken");
    assert_eq!(
        error.to_string(),
        "the conversation holds no request of yours, so there is nothing to digest"
    );
}

#[test]
fn a_digest_is_kept_as_the_manager_corrected_it() {
    let imports = imports();
    imports.held("air/a", WORK_FOLDER, 3, &["Plan the sensor"]);

    let kept = imports
        .digests
        .keep(
            imports.project.id(),
            DigestToKeep {
                source: "air/a".to_owned(),
                held_in: WORK_FOLDER.to_owned(),
                title: " Sensor plan ".to_owned(),
                text: "We chose the SCD41.\r\nNot the MH-Z19.".to_owned(),
                started_at: Some(moment(3, 8)),
                ended_at: Some(moment(3, 17)),
            },
        )
        .unwrap();

    assert_eq!(kept.title().as_str(), "Sensor plan");
    assert_eq!(kept.text().as_str(), "We chose the SCD41.\nNot the MH-Z19.");
    assert_eq!(kept.source().held_in(), WORK_FOLDER);
    assert_eq!(kept.kept_at(), moment(21, 9));
    assert_eq!(
        imports.digests.digests_of(imports.project.id()).unwrap(),
        [kept]
    );
}

#[test]
fn an_empty_digest_is_refused() {
    let imports = imports();

    let error = imports
        .digests
        .keep(
            imports.project.id(),
            DigestToKeep {
                source: "air/a".to_owned(),
                held_in: WORK_FOLDER.to_owned(),
                title: "Sensor plan".to_owned(),
                text: " \n ".to_owned(),
                started_at: None,
                ended_at: None,
            },
        )
        .unwrap_err();

    assert_eq!(error.code(), "rule_broken");
}

#[test]
fn a_conversation_is_kept_once_with_a_project() {
    let imports = imports();
    imports.held("air/a", WORK_FOLDER, 3, &["Plan the sensor"]);
    imports.keep("air/a").unwrap();

    let error = imports.keep("air/a").unwrap_err();

    assert_eq!(error.code(), "digest_already_kept");
    assert_eq!(
        error.to_string(),
        "a digest of this conversation is kept with the project already: \"Plan the sensor\""
    );
}

#[test]
fn digests_are_listed_with_the_one_kept_last_first_and_removed() {
    let imports = imports();
    imports.held("air/a", WORK_FOLDER, 3, &["Plan the sensor"]);
    imports.held("air/b", WORK_FOLDER, 4, &["Choose a battery"]);
    let first = imports.keep("air/a").unwrap();
    imports.clock.advance(TimeDelta::minutes(1));
    let second = imports.keep("air/b").unwrap();

    let listed = imports.digests.digests_of(imports.project.id()).unwrap();
    assert_eq!(listed, [second.clone(), first.clone()]);

    imports.digests.remove(second.id()).unwrap();
    assert_eq!(
        imports.digests.digests_of(imports.project.id()).unwrap(),
        [first]
    );
    assert_eq!(
        imports.digests.remove(second.id()).unwrap_err().code(),
        "not_found"
    );
}

#[test]
fn the_conversations_of_a_project_that_is_not_there_are_refused() {
    let imports = imports();

    let error = imports
        .digests
        .find_earlier(ProjectId::generate())
        .unwrap_err();

    assert_eq!(error.code(), "not_found");
}
