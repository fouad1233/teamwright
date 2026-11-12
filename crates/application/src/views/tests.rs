//! The views of a project, as `demoViews.test.ts` describes them for the
//! demo of the interface: the same cases, under the same names, in the same
//! groups, with the same codes and messages.
//!
//! Two cases differ in what they start from. The demo's sample workspace is
//! a project made by hand; here the sample project is made through the use
//! case, as every project is. A project that was kept before it had views
//! gets them from a migration of the database, so that case is tested with
//! the migrations of the storage crate, under the same name. And an
//! identifier that names nothing is a well-formed identifier here, because
//! the commands refuse text that is not one before a use case runs.

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::{Member, MemberId, MemberKind, Project, Timestamp};

use super::*;
use crate::in_memory::{
    InMemoryConversationRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
    InMemoryProjectRepository, InMemorySprintRepository, InMemoryTaskRepository,
    InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::team::{NewMember, TeamService};

/// The kinds and the names of the views a new project starts with, in their
/// order.
const STARTING_VIEWS: [(ViewKind, &str); 6] = [
    (ViewKind::Board, "Board"),
    (ViewKind::Backlog, "Backlog"),
    (ViewKind::List, "List"),
    (ViewKind::Gantt, "Gantt"),
    (ViewKind::Calendar, "Calendar"),
    (ViewKind::Dashboard, "Dashboard"),
];

const EVERY_TASK: ViewFilter = ViewFilter::EVERY_TASK;

/// The services on shared storage, and a project called "Air quality node".
struct Workspace {
    projects: ProjectService,
    team: TeamService,
    views: ViewService,
    project: Project,
}

fn start_of_day() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
}

fn workspace() -> Workspace {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations_store = Arc::new(InMemoryConversationRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        Arc::new(InMemorySprintRepository::new(tasks_store.clone())),
        views_store.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store.clone(),
        members_store.clone(),
        tasks_store,
        conversations_store,
        views_store.clone(),
        clock,
    );
    let views = ViewService::new(projects_store, members_store, views_store);
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        projects,
        team,
        views,
        project,
    }
}

impl Workspace {
    fn views(&self) -> Vec<View> {
        self.views.list_for_project(self.project.id()).unwrap()
    }

    fn names(&self) -> Vec<String> {
        self.views()
            .iter()
            .map(|view| view.name().to_string())
            .collect()
    }

    /// The view of the project that has a name.
    fn view_named(&self, name: &str) -> View {
        self.views()
            .into_iter()
            .find(|view| view.name().as_str() == name)
            .unwrap_or_else(|| panic!("the project has no view called {name}"))
    }

    fn member(&self, name: &str) -> Member {
        self.member_of(name, self.project.id())
    }

    fn member_of(&self, name: &str, project_id: ProjectId) -> Member {
        self.team
            .add(NewMember {
                project_id,
                name: name.to_owned(),
                kind: MemberKind::Agent,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap()
    }

    fn add(&self, kind: ViewKind, name: Option<&str>) -> Result<View, ApplicationError> {
        self.views.add(NewView {
            project_id: self.project.id(),
            kind,
            name: name.map(str::to_owned),
        })
    }

    fn other_project(&self) -> Project {
        self.projects
            .create(NewProject {
                name: "Weather station".to_owned(),
                workflow: WorkflowTemplate::Standard,
                ..NewProject::default()
            })
            .unwrap()
    }
}

/// The kinds and the names of some views.
fn kinds_and_names(views: &[View]) -> Vec<(ViewKind, String)> {
    views
        .iter()
        .map(|view| (view.kind(), view.name().to_string()))
        .collect()
}

fn starting_kinds_and_names() -> Vec<(ViewKind, String)> {
    STARTING_VIEWS
        .iter()
        .map(|(kind, name)| (*kind, (*name).to_owned()))
        .collect()
}

fn filter(assignee: ViewAssignee, hides_finished: bool) -> ViewFilter {
    ViewFilter {
        assignee,
        hides_finished,
    }
}

mod the_views_of_the_sample_workspace {
    use super::*;

    #[test]
    fn gives_the_sample_project_the_six_views_a_new_project_starts_with() {
        let workspace = workspace();

        let views = workspace.views();

        assert_eq!(kinds_and_names(&views), starting_kinds_and_names());
        assert!(
            views
                .iter()
                .all(|view| view.project_id() == workspace.project.id())
        );
    }
}

mod the_views_of_a_project {
    use super::*;

    #[test]
    fn starts_a_new_project_with_one_view_of_each_kind_named_after_it_showing_every_task() {
        let workspace = workspace();

        let listed = workspace.views();

        assert_eq!(kinds_and_names(&listed), starting_kinds_and_names());
        for view in &listed {
            assert_eq!(view.project_id(), workspace.project.id());
            assert_eq!(view.filter(), EVERY_TASK);
        }
        let mut ids: Vec<ViewId> = listed.iter().map(View::id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 6);
    }

    #[test]
    fn refuses_to_list_the_views_of_a_project_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ProjectId::generate();

        let error = workspace.views.list_for_project(nowhere).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no project with the identifier {nowhere}")
        );
    }

    #[test]
    fn removes_the_views_with_their_project() {
        let workspace = workspace();
        let board = workspace.view_named("Board");

        workspace.projects.delete(workspace.project.id()).unwrap();

        let listed = workspace
            .views
            .list_for_project(workspace.project.id())
            .unwrap_err();
        assert_eq!(listed.code(), "not_found");
        let error = workspace.views.rename(board.id(), "Plan").unwrap_err();
        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no view with the identifier {}", board.id())
        );
    }

    #[test]
    fn keeps_the_views_of_a_project_apart_from_those_of_another() {
        let workspace = workspace();
        let other = workspace.other_project();

        workspace
            .views
            .add(NewView {
                project_id: other.id(),
                kind: ViewKind::Board,
                name: None,
            })
            .unwrap();

        let starting: Vec<String> = STARTING_VIEWS
            .iter()
            .map(|(_, name)| (*name).to_owned())
            .collect();
        assert_eq!(workspace.names(), starting);
        let others: Vec<String> = workspace
            .views
            .list_for_project(other.id())
            .unwrap()
            .iter()
            .map(|view| view.name().to_string())
            .collect();
        assert!(others.contains(&"Board 2".to_owned()));
    }
}

mod adding_a_view {
    use super::*;

    #[test]
    fn adds_it_at_the_end_named_after_its_kind_and_numbered_when_the_name_is_taken() {
        let workspace = workspace();

        let added = workspace.add(ViewKind::Board, None).unwrap();

        assert_eq!(added.project_id(), workspace.project.id());
        assert_eq!(added.kind(), ViewKind::Board);
        assert_eq!(added.name().as_str(), "Board 2");
        assert_eq!(added.filter(), EVERY_TASK);
        assert_eq!(workspace.views().last(), Some(&added));
    }

    #[test]
    fn takes_the_name_of_the_kind_as_it_is_when_no_view_has_it() {
        let workspace = workspace();
        workspace
            .views
            .rename(workspace.view_named("List").id(), "Everything")
            .unwrap();

        let added = workspace.add(ViewKind::List, None).unwrap();

        assert_eq!(added.name().as_str(), "List");
    }

    #[test]
    fn numbers_the_name_with_the_first_number_that_is_free_whatever_the_capitals() {
        let workspace = workspace();
        workspace.add(ViewKind::Gantt, Some("gantt 2")).unwrap();
        workspace.add(ViewKind::Gantt, Some("Gantt 4")).unwrap();

        let added = workspace.add(ViewKind::Gantt, None).unwrap();

        assert_eq!(added.name().as_str(), "Gantt 3");
    }

    #[test]
    fn names_it_after_its_kind_when_the_name_is_empty() {
        let workspace = workspace();

        let added = workspace.add(ViewKind::Calendar, Some("  ")).unwrap();

        assert_eq!(added.name().as_str(), "Calendar 2");
    }

    #[test]
    fn trims_the_name_it_is_given_and_puts_it_on_one_line() {
        let workspace = workspace();

        let added = workspace
            .add(ViewKind::Board, Some("  Firmware \n  only "))
            .unwrap();

        assert_eq!(added.name().as_str(), "Firmware only");
    }

    #[test]
    fn accepts_a_name_of_40_characters() {
        let workspace = workspace();

        let added = workspace
            .add(ViewKind::List, Some(&"x".repeat(40)))
            .unwrap();

        assert_eq!(added.name().as_str().chars().count(), 40);
    }

    #[test]
    fn refuses_a_name_of_more_than_40_characters() {
        let workspace = workspace();

        let error = workspace
            .add(ViewKind::List, Some(&"x".repeat(41)))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the view name can have at most 40 characters, but this one has 41"
        );
    }

    #[test]
    fn refuses_a_name_another_view_of_the_project_has_whatever_the_capitals() {
        let workspace = workspace();

        let error = workspace.add(ViewKind::List, Some(" BOARD ")).unwrap_err();

        assert_eq!(error.code(), "view_name_taken");
        assert_eq!(
            error.to_string(),
            "the project already has a view called \"BOARD\""
        );
    }

    #[test]
    fn refuses_a_second_backlog() {
        let workspace = workspace();

        let error = workspace.add(ViewKind::Backlog, None).unwrap_err();

        assert_eq!(error.code(), "view_kind_taken");
        assert_eq!(
            error.to_string(),
            "the project already has a backlog, \"Backlog\", and can have only one"
        );
    }

    #[test]
    fn refuses_a_second_dashboard_whatever_the_first_one_is_called() {
        let workspace = workspace();
        workspace
            .views
            .rename(workspace.view_named("Dashboard").id(), "Health")
            .unwrap();

        let error = workspace
            .add(ViewKind::Dashboard, Some("Another"))
            .unwrap_err();

        assert_eq!(error.code(), "view_kind_taken");
        assert_eq!(
            error.to_string(),
            "the project already has a dashboard, \"Health\", and can have only one"
        );
    }

    #[test]
    fn adds_a_backlog_again_once_it_has_been_removed() {
        let workspace = workspace();
        workspace
            .views
            .remove(workspace.view_named("Backlog").id())
            .unwrap();

        let added = workspace.add(ViewKind::Backlog, None).unwrap();

        assert_eq!(added.name().as_str(), "Backlog");
        assert_eq!(
            workspace.views().last().map(View::kind),
            Some(ViewKind::Backlog)
        );
    }

    #[test]
    fn refuses_a_view_for_a_project_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ProjectId::generate();

        let error = workspace
            .views
            .add(NewView {
                project_id: nowhere,
                kind: ViewKind::Board,
                name: None,
            })
            .unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no project with the identifier {nowhere}")
        );
    }
}

mod renaming_a_view {
    use super::*;

    #[test]
    fn trims_the_new_name_and_keeps_the_place_and_the_filter_of_the_view() {
        let workspace = workspace();
        let list = workspace.view_named("List");

        let renamed = workspace.views.rename(list.id(), "  Everything  ").unwrap();

        let mut expected = list.clone();
        expected.rename(ViewName::parse("Everything").unwrap());
        assert_eq!(renamed, expected);
        assert_eq!(workspace.names()[2], "Everything");
    }

    #[test]
    fn lets_a_view_change_the_capitals_of_its_own_name() {
        let workspace = workspace();

        let renamed = workspace
            .views
            .rename(workspace.view_named("Board").id(), "BOARD")
            .unwrap();

        assert_eq!(renamed.name().as_str(), "BOARD");
    }

    #[test]
    fn refuses_an_empty_name() {
        let workspace = workspace();

        let error = workspace
            .views
            .rename(workspace.view_named("Board").id(), "   ")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "the view name must not be empty");
    }

    #[test]
    fn refuses_a_name_of_more_than_40_characters() {
        let workspace = workspace();

        let error = workspace
            .views
            .rename(workspace.view_named("Board").id(), &"y".repeat(45))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the view name can have at most 40 characters, but this one has 45"
        );
    }

    #[test]
    fn refuses_the_name_of_another_view_of_the_project_whatever_the_capitals() {
        let workspace = workspace();

        let error = workspace
            .views
            .rename(workspace.view_named("Board").id(), "list")
            .unwrap_err();

        assert_eq!(error.code(), "view_name_taken");
        assert_eq!(
            error.to_string(),
            "the project already has a view called \"list\""
        );
    }

    #[test]
    fn refuses_a_view_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ViewId::generate();

        let error = workspace.views.rename(nowhere, "Plan").unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no view with the identifier {nowhere}")
        );
    }
}

mod filtering_a_view {
    use super::*;

    #[test]
    fn shows_the_tasks_of_one_member_of_the_team_and_can_hide_the_finished_ones() {
        let workspace = workspace();
        let engineer = workspace.member("Firmware engineer");
        let board = workspace.view_named("Board");
        let wanted = filter(ViewAssignee::Member(engineer.id()), true);

        let filtered = workspace.views.set_filter(board.id(), wanted).unwrap();

        let mut expected = board.clone();
        expected.set_filter(wanted).unwrap();
        assert_eq!(filtered, expected);
        assert_eq!(filtered.filter(), wanted);
        assert_eq!(workspace.view_named("Board"), filtered);
    }

    #[test]
    fn shows_the_tasks_of_the_manager_of_nobody_or_of_anyone_again() {
        let workspace = workspace();
        let gantt = workspace.view_named("Gantt");
        let calendar = workspace.view_named("Calendar");
        let list = workspace.view_named("List");

        workspace
            .views
            .set_filter(gantt.id(), filter(ViewAssignee::Manager, false))
            .unwrap();
        workspace
            .views
            .set_filter(calendar.id(), filter(ViewAssignee::Nobody, true))
            .unwrap();
        workspace
            .views
            .set_filter(list.id(), filter(ViewAssignee::Nobody, false))
            .unwrap();
        let cleared = workspace.views.set_filter(list.id(), EVERY_TASK).unwrap();

        assert_eq!(
            workspace.view_named("Gantt").filter().assignee,
            ViewAssignee::Manager
        );
        assert_eq!(
            workspace.view_named("Calendar").filter(),
            filter(ViewAssignee::Nobody, true)
        );
        assert_eq!(cleared.filter(), EVERY_TASK);
    }

    #[test]
    fn refuses_to_filter_a_backlog() {
        let workspace = workspace();

        let error = workspace
            .views
            .set_filter(
                workspace.view_named("Backlog").id(),
                filter(ViewAssignee::Manager, false),
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "\"Backlog\" is a backlog, which shows every task and cannot be filtered"
        );
    }

    #[test]
    fn refuses_to_filter_a_dashboard() {
        let workspace = workspace();

        let error = workspace
            .views
            .set_filter(
                workspace.view_named("Dashboard").id(),
                filter(ViewAssignee::Anyone, true),
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "\"Dashboard\" is a dashboard, which shows every task and cannot be filtered"
        );
    }

    #[test]
    fn refuses_a_member_of_the_team_of_another_project() {
        let workspace = workspace();
        let other = workspace.other_project();
        let stranger = workspace.member_of("Firmware engineer", other.id());

        let error = workspace
            .views
            .set_filter(
                workspace.view_named("Board").id(),
                filter(ViewAssignee::Member(stranger.id()), false),
            )
            .unwrap_err();

        assert_eq!(error.code(), "member_not_on_team");
        assert_eq!(
            error.to_string(),
            "the filter names a member who is not on the team of the project"
        );
    }

    #[test]
    fn refuses_a_member_who_does_not_exist() {
        let workspace = workspace();

        let error = workspace
            .views
            .set_filter(
                workspace.view_named("Board").id(),
                filter(ViewAssignee::Member(MemberId::generate()), false),
            )
            .unwrap_err();

        assert_eq!(error.code(), "member_not_on_team");
        assert_eq!(
            error.to_string(),
            "the filter names a member who is not on the team of the project"
        );
    }

    #[test]
    fn refuses_a_view_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ViewId::generate();

        let error = workspace.views.set_filter(nowhere, EVERY_TASK).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no view with the identifier {nowhere}")
        );
    }

    #[test]
    fn shows_everybody_again_in_a_view_that_showed_only_a_member_who_left_the_team() {
        let workspace = workspace();
        let engineer = workspace.member("Firmware engineer");
        let designer = workspace.member("Industrial designer");
        let board = workspace.view_named("Board");
        let list = workspace.view_named("List");
        workspace
            .views
            .set_filter(
                board.id(),
                filter(ViewAssignee::Member(engineer.id()), true),
            )
            .unwrap();
        workspace
            .views
            .set_filter(
                list.id(),
                filter(ViewAssignee::Member(designer.id()), false),
            )
            .unwrap();

        workspace.team.remove(engineer.id()).unwrap();

        assert_eq!(
            workspace.view_named("Board").filter(),
            filter(ViewAssignee::Anyone, true)
        );
        assert_eq!(
            workspace.view_named("List").filter().assignee,
            ViewAssignee::Member(designer.id())
        );
    }
}

mod moving_a_view {
    use super::*;

    #[test]
    fn puts_it_at_the_place_it_is_given_and_answers_with_the_views_in_their_new_order() {
        let workspace = workspace();
        let calendar = workspace.view_named("Calendar");

        let moved = workspace.views.move_to(calendar.id(), 0).unwrap();

        let order = ["Calendar", "Board", "Backlog", "List", "Gantt", "Dashboard"];
        let moved_names: Vec<String> = moved.iter().map(|view| view.name().to_string()).collect();
        assert_eq!(moved_names, order);
        assert_eq!(workspace.names(), order);
    }

    #[test]
    fn moves_a_view_one_place_to_the_right() {
        let workspace = workspace();

        workspace
            .views
            .move_to(workspace.view_named("Board").id(), 1)
            .unwrap();

        assert_eq!(
            workspace.names(),
            ["Backlog", "Board", "List", "Gantt", "Calendar", "Dashboard"]
        );
    }

    #[test]
    fn puts_it_last_for_a_place_past_the_end_and_first_for_a_place_before_the_start() {
        let workspace = workspace();

        workspace
            .views
            .move_to(workspace.view_named("Board").id(), 99)
            .unwrap();
        workspace
            .views
            .move_to(workspace.view_named("Dashboard").id(), -3)
            .unwrap();

        assert_eq!(
            workspace.names(),
            ["Dashboard", "Backlog", "List", "Gantt", "Calendar", "Board"]
        );
    }

    #[test]
    fn refuses_a_view_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ViewId::generate();

        let error = workspace.views.move_to(nowhere, 0).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no view with the identifier {nowhere}")
        );
    }
}

mod removing_a_view {
    use super::*;

    #[test]
    fn removes_it_and_keeps_the_others_in_their_order() {
        let workspace = workspace();

        workspace
            .views
            .remove(workspace.view_named("Gantt").id())
            .unwrap();

        assert_eq!(
            workspace.names(),
            ["Board", "Backlog", "List", "Calendar", "Dashboard"]
        );
    }

    #[test]
    fn refuses_to_remove_the_last_view_of_a_project() {
        let workspace = workspace();
        for name in ["Backlog", "List", "Gantt", "Calendar", "Dashboard"] {
            workspace
                .views
                .remove(workspace.view_named(name).id())
                .unwrap();
        }

        let error = workspace
            .views
            .remove(workspace.view_named("Board").id())
            .unwrap_err();

        assert_eq!(error.code(), "last_view");
        assert_eq!(
            error.to_string(),
            "\"Board\" is the last view of the project and cannot be removed"
        );
        assert_eq!(workspace.names(), ["Board"]);
    }

    #[test]
    fn refuses_a_view_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ViewId::generate();

        let error = workspace.views.remove(nowhere).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no view with the identifier {nowhere}")
        );
    }
}
