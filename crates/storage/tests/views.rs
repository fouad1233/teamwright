//! The views of the projects in a real SQLite database: what is stored,
//! what is logged, and what goes with a project or a member.

// The helper functions below are test code as well, but the linter only
// recognizes functions marked `#[test]` as such.
#![allow(clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use teamwright_application::in_memory::ManualClock;
use teamwright_application::{
    NewMember, NewProject, NewView, ProjectService, TeamService, ViewRepository, ViewService,
};
use teamwright_domain::{MemberKind, Project, View, ViewAssignee, ViewFilter, ViewKind};
use teamwright_storage::{
    Database, SqliteConversationRepository, SqliteMemberRepository, SqlitePlacesRepository,
    SqliteProjectRepository, SqliteSprintRepository, SqliteTaskRepository, SqliteViewRepository,
};

/// The use cases on one database, and a project in it.
struct Workspace {
    database: Arc<Database>,
    projects: ProjectService,
    team: TeamService,
    views: ViewService,
    project: Project,
}

fn workspace() -> Workspace {
    let database = Arc::new(Database::open_in_memory().unwrap());
    let projects = Arc::new(SqliteProjectRepository::new(database.clone()));
    let tasks = Arc::new(SqliteTaskRepository::new(database.clone()));
    let members = Arc::new(SqliteMemberRepository::new(database.clone()));
    let conversations = Arc::new(SqliteConversationRepository::new(database.clone()));
    let sprints = Arc::new(SqliteSprintRepository::new(database.clone()));
    let views = Arc::new(SqliteViewRepository::new(database.clone()));
    let area_store = Arc::new(teamwright_storage::SqliteAreaRepository::new(
        database.clone(),
    ));
    let clock = Arc::new(ManualClock::starting_at(
        Utc.with_ymd_and_hms(2026, 11, 13, 9, 0, 0).unwrap(),
    ));
    let project_service = ProjectService::new(
        projects.clone(),
        tasks.clone(),
        members.clone(),
        conversations.clone(),
        Arc::new(SqlitePlacesRepository::new(database.clone())),
        sprints,
        views.clone(),
        area_store.clone(),
        clock.clone(),
    );
    let project = project_service
        .create(NewProject {
            name: "Air quality node".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        team: TeamService::new(
            projects.clone(),
            members.clone(),
            tasks,
            conversations,
            views.clone(),
            clock,
        ),
        views: ViewService::new(projects, members, views),
        projects: project_service,
        project,
        database,
    }
}

impl Workspace {
    /// The entity and the kind of every operation logged after the first
    /// `from`.
    fn logged_since(&self, from: usize) -> Vec<(String, String)> {
        self.database.operations().unwrap()[from..]
            .iter()
            .map(|operation| (operation.entity.clone(), operation.kind.clone()))
            .collect()
    }

    fn stored(&self) -> Vec<View> {
        let mut views = SqliteViewRepository::new(self.database.clone())
            .list_for_project(self.project.id())
            .unwrap();
        views.sort_by_key(View::position);
        views
    }
}

#[test]
fn a_new_project_stores_and_logs_its_six_views() {
    let workspace = workspace();

    assert_eq!(workspace.stored(), View::starting(workspace.project.id()));
    let views_logged = workspace
        .logged_since(0)
        .into_iter()
        .filter(|(entity, kind)| entity == "view" && kind == "upsert")
        .count();
    assert_eq!(views_logged, 6);
}

#[test]
fn every_change_to_a_view_is_stored_and_logged() {
    let workspace = workspace();
    let added = workspace
        .views
        .add(NewView {
            project_id: workspace.project.id(),
            kind: ViewKind::Gantt,
            name: Some("Firmware plan".to_owned()),
        })
        .unwrap();
    let before = workspace.database.operations().unwrap().len();

    workspace.views.rename(added.id(), "Radio plan").unwrap();
    let filter = ViewFilter {
        assignee: ViewAssignee::Nobody,
        hides_finished: true,
    };
    workspace.views.set_filter(added.id(), filter).unwrap();
    workspace.views.move_to(added.id(), 0).unwrap();

    let found = SqliteViewRepository::new(workspace.database.clone())
        .find(added.id())
        .unwrap()
        .unwrap();
    assert_eq!(found.name().as_str(), "Radio plan");
    assert_eq!(found.filter(), filter);
    assert_eq!(found.position(), 0);
    // Moving it first moves every view behind it one place on.
    let logged = workspace.logged_since(before);
    assert_eq!(logged.len(), 2 + 7);
    assert!(
        logged
            .iter()
            .all(|(entity, kind)| entity == "view" && kind == "upsert")
    );

    workspace.views.remove(added.id()).unwrap();

    assert_eq!(
        workspace.logged_since(before + 9),
        [("view".to_owned(), "delete".to_owned())]
    );
    assert!(
        SqliteViewRepository::new(workspace.database.clone())
            .find(added.id())
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_member_who_leaves_is_forgotten_by_the_stored_views() {
    let workspace = workspace();
    let engineer = workspace
        .team
        .add(NewMember {
            project_id: workspace.project.id(),
            name: "Firmware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    let board = workspace.stored().remove(0);
    workspace
        .views
        .set_filter(
            board.id(),
            ViewFilter {
                assignee: ViewAssignee::Member(engineer.id()),
                hides_finished: true,
            },
        )
        .unwrap();

    workspace.team.remove(engineer.id()).unwrap();

    assert_eq!(
        workspace.stored()[0].filter(),
        ViewFilter {
            assignee: ViewAssignee::Anyone,
            hides_finished: true,
        }
    );
}

#[test]
fn the_views_go_with_their_project_and_each_removal_is_logged() {
    let workspace = workspace();
    let before = workspace.database.operations().unwrap().len();

    workspace.projects.delete(workspace.project.id()).unwrap();

    assert!(workspace.stored().is_empty());
    let removed_views = workspace
        .logged_since(before)
        .into_iter()
        .filter(|(entity, kind)| entity == "view" && kind == "delete")
        .count();
    assert_eq!(removed_views, 6);
}
