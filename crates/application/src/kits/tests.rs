#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use std::path::PathBuf;
use std::sync::Arc;

use teamwright_domain::{
    Color, ConnectorStart, Member, MemberKind, MemberName, Project, ProjectKey, ProjectName,
    Workflow,
};

use super::*;
use crate::in_memory::{InMemoryMemberRepository, InMemoryProjectRepository, ManualClock};
use crate::ports::{MemberRepository, ProjectRepository};

const REVIEW: &str = "---\nname: pcb-review\ndescription: Reviews a board layout.\n---\nLook.\n";

struct Workspace {
    kits: Arc<InMemoryKitRepository>,
    shelf: Arc<PretendShelf>,
    trust: Arc<InMemoryConnectorTrust>,
    members: Arc<InMemoryMemberRepository>,
    service: KitService,
    project: Project,
    specialist: Member,
    person: Member,
}

fn workspace() -> Workspace {
    let now = chrono::Utc::now();
    let projects = Arc::new(InMemoryProjectRepository::default());
    let members = Arc::new(InMemoryMemberRepository::default());
    let kits = Arc::new(InMemoryKitRepository::default());
    let shelf = Arc::new(PretendShelf::default());
    let trust = Arc::new(InMemoryConnectorTrust::default());
    let project = Project::create(
        ProjectName::parse("Smart sensor node").unwrap(),
        ProjectKey::parse("SSN").unwrap(),
        String::new(),
        Workflow::standard(),
        now,
    );
    projects.save(&project).unwrap();
    let member = |name: &str, kind| {
        let member = Member::create(
            project.id(),
            MemberName::parse(name).unwrap(),
            kind,
            String::new(),
            Color::Teal,
            now,
        );
        members.save(&member).unwrap();
        member
    };
    let specialist = member("Hardware engineer", MemberKind::Agent);
    let person = member("Dana Ortiz", MemberKind::Human);
    let service = KitService::new(
        kits.clone(),
        shelf.clone(),
        trust.clone(),
        projects,
        members.clone(),
        Arc::new(ManualClock::starting_at(now)),
    );
    Workspace {
        kits,
        shelf,
        trust,
        members,
        service,
        project,
        specialist,
        person,
    }
}

fn parts(start: NewConnectorStart) -> NewConnector {
    NewConnector {
        name: "parts".to_owned(),
        start,
        tools: vec!["search".to_owned(), "order".to_owned()],
    }
}

fn parts_program(variables: &[&str]) -> NewConnector {
    parts(NewConnectorStart::Program {
        program: "parts-server".to_owned(),
        arguments: vec!["--stdio".to_owned()],
        variables: variables.iter().map(|name| (*name).to_owned()).collect(),
    })
}

#[test]
fn a_skill_is_added_from_its_folder_and_kept_on_this_computer() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);

    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();

    assert_eq!(skill.name().as_str(), "pcb-review");
    assert_eq!(skill.description(), "Reviews a board layout.");
    let library = space.service.library(space.project.id()).unwrap();
    assert_eq!(library.skills.len(), 1);
    assert!(library.skills[0].here);
}

#[test]
fn a_folder_that_is_no_skill_is_refused_with_its_reason() {
    let space = workspace();
    let refused = |path: &str| {
        space
            .service
            .add_skill(space.project.id(), Path::new(path))
            .unwrap_err()
            .to_string()
    };
    space.shelf.put_described_folder(
        "/skills/empty",
        SkillFolder {
            folder_name: "empty".to_owned(),
            skill_text: None,
            files: 0,
            bytes: 0,
            has_links: false,
        },
    );
    assert!(refused("/skills/empty").contains("SKILL.md"));
    space.shelf.put_described_folder(
        "/skills/linked",
        SkillFolder {
            folder_name: "linked".to_owned(),
            skill_text: Some(REVIEW.to_owned()),
            files: 2,
            bytes: 10,
            has_links: true,
        },
    );
    assert!(refused("/skills/linked").contains("link"));
    space.shelf.put_described_folder(
        "/skills/huge",
        SkillFolder {
            folder_name: "huge".to_owned(),
            skill_text: Some(REVIEW.to_owned()),
            files: MAX_SKILL_FILES + 1,
            bytes: 10,
            has_links: false,
        },
    );
    assert!(refused("/skills/huge").contains("larger"));
    space.shelf.put_folder(
        "/skills/runner",
        "---\ndescription: x\n---\nThe time: !`date`\n",
    );
    assert!(refused("/skills/runner").contains("runs a command"));
    assert!(
        space
            .service
            .library(space.project.id())
            .unwrap()
            .skills
            .is_empty()
    );
}

#[test]
fn a_skill_is_in_the_library_once_and_can_be_taken_again_on_another_computer() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let first = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let twice = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"));
    assert!(twice.unwrap_err().to_string().contains("already has"));

    // Another computer: the library knows the skill, the shelf does not.
    space.shelf.clear_kept();
    let library = space.service.library(space.project.id()).unwrap();
    assert!(!library.skills[0].here);
    let again = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    assert_eq!(again.id(), first.id());
    assert!(space.service.library(space.project.id()).unwrap().skills[0].here);
}

#[test]
fn a_connector_is_checked_and_its_name_is_used_once() {
    let space = workspace();
    let connector = space
        .service
        .add_connector(space.project.id(), parts_program(&["PARTS_TOKEN"]))
        .unwrap();
    assert_eq!(connector.tools().len(), 2);
    let twice = space
        .service
        .add_connector(space.project.id(), parts_program(&[]));
    assert!(twice.unwrap_err().to_string().contains("already has"));

    let secret_in_address = parts(NewConnectorStart::Address {
        url: "https://parts.example/mcp?token=hunter2".to_owned(),
        headers: Vec::new(),
    });
    let refusal = space
        .service
        .add_connector(space.project.id(), secret_in_address)
        .unwrap_err()
        .to_string();
    assert!(!refusal.contains("hunter2"));

    let library = space.service.library(space.project.id()).unwrap();
    assert_eq!(
        library.connectors[0]
            .missing_variables
            .iter()
            .map(VariableName::as_str)
            .collect::<Vec<_>>(),
        ["PARTS_TOKEN"]
    );
    space.shelf.set_variable("PARTS_TOKEN");
    let library = space.service.library(space.project.id()).unwrap();
    assert!(library.connectors[0].missing_variables.is_empty());
}

#[test]
fn a_specialist_is_given_skills_and_connectors_tool_by_tool() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let connector = space
        .service
        .add_connector(space.project.id(), parts_program(&[]))
        .unwrap();

    let kit = space
        .service
        .set_kit(
            space.specialist.id(),
            KitChoice {
                skills: vec![skill.id()],
                connectors: vec![(connector.id(), vec!["search".to_owned()])],
            },
        )
        .unwrap();
    assert_eq!(kit.connectors()[0].tools()[0].as_str(), "search");

    let unknown_tool = space.service.set_kit(
        space.specialist.id(),
        KitChoice {
            skills: Vec::new(),
            connectors: vec![(connector.id(), vec!["delete_everything".to_owned()])],
        },
    );
    assert!(
        unknown_tool
            .unwrap_err()
            .to_string()
            .contains("delete_everything")
    );
    let no_tool = space.service.set_kit(
        space.specialist.id(),
        KitChoice {
            skills: Vec::new(),
            connectors: vec![(connector.id(), Vec::new())],
        },
    );
    assert!(no_tool.is_err());
    let person = space.service.set_kit(
        space.person.id(),
        KitChoice {
            skills: vec![skill.id()],
            connectors: Vec::new(),
        },
    );
    assert!(person.is_err());
}

#[test]
fn a_specialist_is_given_nothing_of_another_project() {
    let space = workspace();
    let other = workspace();
    other.shelf.put_folder("/skills/pcb-review", REVIEW);
    let foreign = other
        .service
        .add_skill(other.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let refusal = space.service.set_kit(
        space.specialist.id(),
        KitChoice {
            skills: vec![foreign.id()],
            connectors: Vec::new(),
        },
    );
    assert!(matches!(
        refusal,
        Err(ApplicationError::NotFound { kind: "skill", .. })
    ));
}

#[test]
fn removing_from_the_library_takes_it_from_every_specialist() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let connector = space
        .service
        .add_connector(space.project.id(), parts_program(&[]))
        .unwrap();
    space
        .service
        .set_kit(
            space.specialist.id(),
            KitChoice {
                skills: vec![skill.id()],
                connectors: vec![(connector.id(), vec!["search".to_owned()])],
            },
        )
        .unwrap();

    space
        .service
        .remove_skill(space.project.id(), skill.id())
        .unwrap();
    space
        .service
        .remove_connector(space.project.id(), connector.id())
        .unwrap();

    let kit = space.kits.kit_of(space.specialist.id()).unwrap().unwrap();
    assert!(kit.is_empty());
    assert!(space.shelf.kept(skill.id()).is_none());
    assert!(
        space
            .service
            .remove_skill(space.project.id(), skill.id())
            .is_err()
    );
}

#[test]
fn a_turn_gets_only_what_this_computer_has_and_still_passes_the_check() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    let local = space
        .service
        .add_connector(space.project.id(), parts_program(&[]))
        .unwrap();
    let mut needs_token = parts_program(&["CATALOG_TOKEN"]);
    needs_token.name = "catalog".to_owned();
    let remote = space
        .service
        .add_connector(space.project.id(), needs_token)
        .unwrap();
    space
        .service
        .set_kit(
            space.specialist.id(),
            KitChoice {
                skills: vec![skill.id()],
                connectors: vec![
                    (local.id(), vec!["search".to_owned()]),
                    (remote.id(), vec!["order".to_owned()]),
                ],
            },
        )
        .unwrap();
    let member = space.members.find(space.specialist.id()).unwrap().unwrap();

    let turn = turn_kit_of(
        space.kits.as_ref(),
        space.shelf.as_ref(),
        space.trust.as_ref(),
        &member,
    )
    .unwrap();
    assert_eq!(turn.skills.len(), 1);
    assert_eq!(turn.skills[0].name, "pcb-review");
    assert_eq!(
        turn.skills[0].folder,
        PathBuf::from("shelf").join(skill.id().to_string())
    );
    // The connector whose variable is not set here is left out.
    assert_eq!(turn.connectors.len(), 1);
    assert_eq!(turn.connectors[0].name, "parts");
    assert_eq!(turn.connectors[0].tools, ["search"]);
    assert!(matches!(
        turn.connectors[0].start,
        ConnectorStart::Program { .. }
    ));

    space.shelf.set_variable("CATALOG_TOKEN");
    let turn = turn_kit_of(
        space.kits.as_ref(),
        space.shelf.as_ref(),
        space.trust.as_ref(),
        &member,
    )
    .unwrap();
    assert_eq!(turn.connectors.len(), 2);

    // A copy changed on the disk so that it would run commands is no
    // longer given.
    space.shelf.change_kept(
        skill.id(),
        "---\nname: pcb-review\ndescription: x\nhooks:\n  Stop: []\n---\n",
    );
    let turn = turn_kit_of(
        space.kits.as_ref(),
        space.shelf.as_ref(),
        space.trust.as_ref(),
        &member,
    )
    .unwrap();
    assert!(turn.skills.is_empty());
    let library = space.service.library(space.project.id()).unwrap();
    assert!(!library.skills[0].here);
}

#[test]
fn a_specialist_without_a_kit_gets_nothing() {
    let space = workspace();
    let turn = turn_kit_of(
        space.kits.as_ref(),
        space.shelf.as_ref(),
        space.trust.as_ref(),
        &space.specialist,
    )
    .unwrap();
    assert!(turn.is_empty());
}

#[test]
fn a_copy_that_holds_more_than_teamwright_wrote_is_not_given_and_says_why() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    space
        .service
        .set_kit(
            space.specialist.id(),
            KitChoice {
                skills: vec![skill.id()],
                connectors: Vec::new(),
            },
        )
        .unwrap();
    let member = space.members.find(space.specialist.id()).unwrap().unwrap();
    let given = || {
        turn_kit_of(
            space.kits.as_ref(),
            space.shelf.as_ref(),
            space.trust.as_ref(),
            &member,
        )
        .unwrap()
        .skills
        .len()
    };
    let problem = || {
        space.service.library(space.project.id()).unwrap().skills[0]
            .problem
            .clone()
    };
    assert_eq!(given(), 1);
    assert_eq!(problem(), None);

    for (change, expected) in [
        (
            Box::new(|layout: &mut KeptLayout| layout.top.push("hooks".to_owned()))
                as Box<dyn Fn(&mut KeptLayout)>,
            "more than Teamwright wrote: hooks",
        ),
        (
            Box::new(|layout: &mut KeptLayout| layout.top.push(".mcp.json".to_owned())),
            ".mcp.json",
        ),
        (
            Box::new(|layout: &mut KeptLayout| layout.manifest.push("marketplace.json".to_owned())),
            "plugin.json",
        ),
        (
            Box::new(|layout: &mut KeptLayout| layout.skills.push("other".to_owned())),
            "the skill pcb-review",
        ),
        (
            Box::new(|layout: &mut KeptLayout| layout.has_links = true),
            "link",
        ),
        (
            Box::new(|layout: &mut KeptLayout| layout.bytes = MAX_SKILL_BYTES + 1),
            "larger",
        ),
    ] {
        space
            .service
            .remove_skill(space.project.id(), skill.id())
            .ok();
        // A fresh copy for each case.
        let skill = space
            .service
            .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
            .unwrap();
        space
            .service
            .set_kit(
                space.specialist.id(),
                KitChoice {
                    skills: vec![skill.id()],
                    connectors: Vec::new(),
                },
            )
            .unwrap();
        assert_eq!(given(), 1);
        space
            .shelf
            .change_kept_layout(skill.id(), |layout| change(layout));
        assert_eq!(given(), 0, "{expected}");
        let said = problem().unwrap_or_default();
        assert!(said.contains(expected), "{said}");
        assert!(!space.service.library(space.project.id()).unwrap().skills[0].here);
    }
}

#[test]
fn a_damaged_copy_can_be_replaced_by_adding_the_folder_again() {
    let space = workspace();
    space.shelf.put_folder("/skills/pcb-review", REVIEW);
    let skill = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    space
        .shelf
        .change_kept_layout(skill.id(), |layout| layout.top.push("agents".to_owned()));
    let again = space
        .service
        .add_skill(space.project.id(), Path::new("/skills/pcb-review"))
        .unwrap();
    assert_eq!(again.id(), skill.id());
    let library = space.service.library(space.project.id()).unwrap();
    assert!(library.skills[0].here);
    assert_eq!(library.skills[0].problem, None);
}

#[test]
fn a_program_named_by_a_relative_path_is_refused_and_a_stored_one_is_never_given() {
    let space = workspace();
    let mut relative = parts_program(&[]);
    if let NewConnectorStart::Program { program, .. } = &mut relative.start {
        "./parts-server".clone_into(program);
    }
    let refusal = space
        .service
        .add_connector(space.project.id(), relative)
        .unwrap_err()
        .to_string();
    assert!(refusal.contains("relative path"), "{refusal}");

    // One stored before the rule, or by an older version on another
    // computer, is shown with its problem and left out of every turn.
    let stored = Connector::add(
        space.project.id(),
        ConnectorName::parse("parts").unwrap(),
        ConnectorStart::program_as_stored("bin/parts-server", Vec::new(), Vec::new()).unwrap(),
        vec![ToolName::parse("search").unwrap()],
        chrono::Utc::now(),
    )
    .unwrap();
    space.kits.save_connector(&stored).unwrap();
    space
        .service
        .set_kit(
            space.specialist.id(),
            KitChoice {
                skills: Vec::new(),
                connectors: vec![(stored.id(), vec!["search".to_owned()])],
            },
        )
        .unwrap();
    let library = space.service.library(space.project.id()).unwrap();
    assert!(
        library.connectors[0]
            .problem
            .as_deref()
            .is_some_and(|problem| problem.contains("relative path"))
    );
    let member = space.members.find(space.specialist.id()).unwrap().unwrap();
    let turn = turn_kit_of(
        space.kits.as_ref(),
        space.shelf.as_ref(),
        space.trust.as_ref(),
        &member,
    )
    .unwrap();
    assert!(turn.connectors.is_empty());
}
