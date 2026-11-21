//! Grouping the tasks of a view, from the use cases to the groups: the rule
//! itself is tested in the domain, next to [`group_tasks`].

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::{
    Assignee, GroupKey, Grouping, MemberKind, Priority, Project, TaskId, View, ViewKind,
};

use super::*;
use crate::areas::AreaService;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
    InMemoryTagRepository, InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::sprints::{NewSprint, SprintService};
use crate::tags::TagService;
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};
use crate::views::ViewService;

/// The services on shared storage, and a project called "Air quality node".
struct Workspace {
    tasks: TaskService,
    team: TeamService,
    areas: AreaService,
    sprints: SprintService,
    tags: TagService,
    views: ViewService,
    groups: GroupingService,
    project: Project,
}

fn workspace() -> Workspace {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations_store = Arc::new(InMemoryConversationRepository::default());
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let areas_store = Arc::new(InMemoryAreaRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let tags_store = Arc::new(InMemoryTagRepository::default());
    let clock = Arc::new(ManualClock::starting_at(
        chrono::Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap(),
    ));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        sprints_store.clone(),
        views_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        tasks: TaskService::new(
            projects_store.clone(),
            tasks_store.clone(),
            members_store.clone(),
            conversations_store.clone(),
            sprints_store.clone(),
            areas_store.clone(),
            clock.clone(),
        ),
        team: TeamService::new(
            projects_store.clone(),
            members_store.clone(),
            tasks_store.clone(),
            conversations_store,
            views_store.clone(),
            clock.clone(),
        ),
        areas: AreaService::new(
            projects_store.clone(),
            tasks_store.clone(),
            areas_store.clone(),
            clock.clone(),
        ),
        sprints: SprintService::new(
            projects_store.clone(),
            tasks_store.clone(),
            sprints_store.clone(),
            clock.clone(),
        ),
        tags: TagService::new(
            projects_store.clone(),
            tasks_store.clone(),
            views_store.clone(),
            tags_store.clone(),
            clock,
        ),
        views: ViewService::new(
            projects_store.clone(),
            members_store.clone(),
            views_store.clone(),
            tags_store.clone(),
        ),
        groups: GroupingService::new(
            projects_store,
            tasks_store,
            members_store,
            areas_store,
            sprints_store,
            tags_store,
            views_store,
        ),
        project,
    }
}

impl Workspace {
    fn view_of(&self, kind: ViewKind) -> View {
        self.views
            .list_for_project(self.project.id())
            .unwrap()
            .into_iter()
            .find(|view| view.kind() == kind)
            .unwrap()
    }

    fn task(&self, title: &str) -> TaskId {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
            .id()
    }

    fn groups(&self, view: &View) -> Vec<TaskGroup> {
        self.groups.groups_of_view(view.id()).unwrap()
    }
}

#[test]
fn a_new_view_groups_by_status_and_every_status_is_a_lane() {
    let workspace = workspace();
    let board = workspace.view_of(ViewKind::Board);
    let task = workspace.task("Draw the schematic");

    let groups = workspace.groups(&board);

    assert_eq!(board.grouping(), Grouping::Status);
    assert_eq!(groups.len(), workspace.project.workflow().statuses().len());
    assert_eq!(groups[0].task_ids, [task]);
}

#[test]
fn a_board_grouped_by_assignee_follows_the_team_and_the_tasks_given() {
    let workspace = workspace();
    let board = workspace.view_of(ViewKind::Board);
    let member = workspace
        .team
        .add(NewMember {
            project_id: workspace.project.id(),
            name: "Hardware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    let given = workspace.task("Draw the schematic");
    workspace.task("Pick the sensor");
    workspace
        .tasks
        .assign(given, Some(Assignee::Member(member.id())))
        .unwrap();

    let board = workspace
        .views
        .set_grouping(board.id(), Grouping::Assignee)
        .unwrap();
    let groups = workspace.groups(&board);

    let keys: Vec<GroupKey> = groups.iter().map(|group| group.key).collect();
    assert_eq!(
        keys,
        [
            GroupKey::Manager,
            GroupKey::Member(member.id()),
            GroupKey::Nobody
        ]
    );
    assert_eq!(groups[1].task_ids, [given]);
    assert_eq!(groups[2].task_ids.len(), 1);
}

#[test]
fn the_groups_follow_a_priority_an_area_a_tag_and_a_sprint_changed_by_the_use_cases() {
    let workspace = workspace();
    let project = workspace.project.id();
    let list = workspace.view_of(ViewKind::List);
    let task = workspace.task("Draw the schematic");

    workspace.tasks.prioritize(task, Priority::Urgent).unwrap();
    let list = workspace
        .views
        .set_grouping(list.id(), Grouping::Priority)
        .unwrap();
    assert_eq!(workspace.groups(&list)[0].task_ids, [task]);

    let hardware = workspace.areas.add(project, "Hardware").unwrap();
    workspace.areas.move_task(task, hardware.id()).unwrap();
    let list = workspace
        .views
        .set_grouping(list.id(), Grouping::Area)
        .unwrap();
    let in_hardware = workspace
        .groups(&list)
        .into_iter()
        .find(|group| group.key == GroupKey::Area(hardware.id()))
        .unwrap();
    assert_eq!(in_hardware.task_ids, [task]);

    let tag = workspace.tags.add(project, "EMC", None).unwrap();
    workspace.tags.put_on(task, tag.id()).unwrap();
    let list = workspace
        .views
        .set_grouping(list.id(), Grouping::Tag)
        .unwrap();
    assert_eq!(workspace.groups(&list)[0].key, GroupKey::Tag(tag.id()));
    assert_eq!(workspace.groups(&list)[0].task_ids, [task]);

    let sprint = workspace
        .sprints
        .create(NewSprint {
            project_id: project,
            name: String::new(),
            goal: String::new(),
            start_on: None,
            end_on: None,
        })
        .unwrap();
    workspace.tasks.plan(task, Some(sprint.id())).unwrap();
    let list = workspace
        .views
        .set_grouping(list.id(), Grouping::Sprint)
        .unwrap();
    assert_eq!(
        workspace.groups(&list)[0].key,
        GroupKey::Sprint(sprint.id())
    );
    assert_eq!(workspace.groups(&list)[0].task_ids, [task]);
}

#[test]
fn a_view_that_cannot_be_grouped_refuses_it_and_stays_grouped_by_status() {
    let workspace = workspace();
    let calendar = workspace.view_of(ViewKind::Calendar);

    let error = workspace
        .views
        .set_grouping(calendar.id(), Grouping::Assignee)
        .unwrap_err();

    assert_eq!(error.code(), "rule_broken");
    assert_eq!(
        workspace.view_of(ViewKind::Calendar).grouping(),
        Grouping::Status
    );
}

#[test]
fn a_view_that_does_not_exist_has_no_groups() {
    let workspace = workspace();

    let error = workspace
        .groups
        .groups_of_view(teamwright_domain::ViewId::generate())
        .unwrap_err();

    assert_eq!(error.code(), "not_found");
}
