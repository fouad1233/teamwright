//! The tags of a project, as `demoTags.test.ts` describes them for the demo
//! of the interface: the same rules, with the same codes.

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::{Project, TaskChangeKind, Timestamp, ViewAssignee, ViewFilter, ViewKind};

use super::*;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
    InMemoryTagRepository, InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};
use crate::views::ViewService;

/// The services on shared storage, and a project called "Air quality node".
struct Workspace {
    projects: ProjectService,
    tasks: TaskService,
    views: ViewService,
    tags: TagService,
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
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let areas_store = Arc::new(InMemoryAreaRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let tags_store = Arc::new(InMemoryTagRepository::default());
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
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
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store,
        sprints_store,
        areas_store,
        clock.clone(),
    );
    let views = ViewService::new(
        projects_store.clone(),
        members_store,
        views_store.clone(),
        tags_store.clone(),
    );
    let tags = TagService::new(projects_store, tasks_store, views_store, tags_store, clock);
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    Workspace {
        projects,
        tasks,
        views,
        tags,
        project,
    }
}

impl Workspace {
    fn names(&self) -> Vec<String> {
        self.tags
            .list_for_project(self.project.id())
            .unwrap()
            .iter()
            .map(|tag| tag.name().to_string())
            .collect()
    }

    fn task(&self, title: &str) -> Task {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
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

#[test]
fn tags_are_listed_by_name_whatever_the_capitals() {
    let workspace = workspace();
    for name in ["power", "EMC", "Enclosure"] {
        workspace
            .tags
            .add(workspace.project.id(), name, None)
            .unwrap();
    }

    assert_eq!(workspace.names(), ["EMC", "Enclosure", "power"]);
}

#[test]
fn a_new_tag_gets_the_next_color_in_turn_unless_one_is_named() {
    let workspace = workspace();
    let project = workspace.project.id();

    let first = workspace.tags.add(project, "EMC", None).unwrap();
    let second = workspace.tags.add(project, "Power", None).unwrap();
    let chosen = workspace
        .tags
        .add(project, "Urgent", Some(Color::Red))
        .unwrap();

    assert_eq!(first.color(), Color::Blue);
    assert_eq!(second.color(), Color::Green);
    assert_eq!(chosen.color(), Color::Red);
}

#[test]
fn a_name_another_tag_has_is_refused_whatever_the_capitals() {
    let workspace = workspace();
    let project = workspace.project.id();
    let emc = workspace.tags.add(project, "EMC", None).unwrap();
    let power = workspace.tags.add(project, "Power", None).unwrap();

    let error = workspace.tags.add(project, " emc ", None).unwrap_err();
    assert_eq!(error.code(), "tag_name_taken");
    assert_eq!(
        error.to_string(),
        "the project already has a tag called \"emc\""
    );
    assert_eq!(
        workspace.tags.rename(power.id(), "Emc").unwrap_err().code(),
        "tag_name_taken"
    );
    assert_eq!(
        workspace
            .tags
            .rename(emc.id(), "emc")
            .unwrap()
            .name()
            .as_str(),
        "emc"
    );
    assert_eq!(
        workspace.tags.add(project, " ", None).unwrap_err().code(),
        "rule_broken"
    );
    assert_eq!(
        workspace
            .tags
            .add(project, &"x".repeat(31), None)
            .unwrap_err()
            .code(),
        "rule_broken"
    );
}

#[test]
fn two_projects_may_each_have_a_tag_with_one_name() {
    let workspace = workspace();
    let other = workspace.other_project();
    workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();

    assert!(workspace.tags.add(other.id(), "EMC", None).is_ok());
}

#[test]
fn a_project_has_at_most_a_hundred_tags() {
    let workspace = workspace();
    let project = workspace.project.id();
    for number in 0..Tag::MOST_IN_A_PROJECT {
        workspace
            .tags
            .add(project, &format!("Tag {number}"), None)
            .unwrap();
    }

    let error = workspace.tags.add(project, "One more", None).unwrap_err();

    assert_eq!(error.code(), "rule_broken");
    assert_eq!(error.to_string(), "a project has at most 100 tags");
}

#[test]
fn a_tag_is_recolored() {
    let workspace = workspace();
    let tag = workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();

    workspace.tags.recolor(tag.id(), Color::Purple).unwrap();

    assert_eq!(
        workspace
            .tags
            .list_for_project(workspace.project.id())
            .unwrap()[0]
            .color(),
        Color::Purple
    );
}

#[test]
fn a_tag_is_put_on_a_task_and_taken_off_and_the_history_says_so() {
    let workspace = workspace();
    let task = workspace.task("Shield the radio");
    let tag = workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();

    let tagged = workspace.tags.put_on(task.id(), tag.id()).unwrap();
    assert!(tagged.tags().contains(&tag.id()));
    assert_eq!(workspace.tags.put_on(task.id(), tag.id()).unwrap(), tagged);
    let untagged = workspace.tags.take_off(task.id(), tag.id()).unwrap();
    assert!(untagged.tags().is_empty());

    let kinds: Vec<TaskChangeKind> = workspace
        .tasks
        .history(task.id())
        .unwrap()
        .into_iter()
        .map(|change| change.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            TaskChangeKind::Created,
            TaskChangeKind::Tagged { tag: tag.id() },
            TaskChangeKind::Untagged { tag: tag.id() },
        ]
    );
}

#[test]
fn a_tag_of_another_project_is_not_put_on_a_task() {
    let workspace = workspace();
    let task = workspace.task("Shield the radio");
    let other = workspace.other_project();
    let foreign = workspace.tags.add(other.id(), "EMC", None).unwrap();

    let error = workspace.tags.put_on(task.id(), foreign.id()).unwrap_err();

    assert_eq!(error.code(), "different_projects");
    assert!(workspace.tasks.get(task.id()).unwrap().tags().is_empty());
}

#[test]
fn a_tag_typed_by_name_is_the_one_the_project_has_or_a_new_one() {
    let workspace = workspace();
    let task = workspace.task("Shield the radio");
    let emc = workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();

    let (_, found) = workspace.tags.put_on_by_name(task.id(), " emc").unwrap();
    let (tagged, made) = workspace
        .tags
        .put_on_by_name(task.id(), "Waiting for parts")
        .unwrap();

    assert_eq!(found.id(), emc.id());
    assert_eq!(made.color(), Color::Green);
    assert_eq!(workspace.names(), ["EMC", "Waiting for parts"]);
    assert_eq!(tagged.tags().len(), 2);
}

#[test]
fn removing_a_tag_takes_it_off_its_tasks_and_out_of_the_filters_of_views() {
    let workspace = workspace();
    let tag = workspace
        .tags
        .add(workspace.project.id(), "EMC", None)
        .unwrap();
    let kept = workspace
        .tags
        .add(workspace.project.id(), "Power", None)
        .unwrap();
    let task = workspace.task("Shield the radio");
    workspace.tags.put_on(task.id(), tag.id()).unwrap();
    workspace.tags.put_on(task.id(), kept.id()).unwrap();
    let board = workspace
        .views
        .list_for_project(workspace.project.id())
        .unwrap()[0]
        .clone();
    assert_eq!(board.kind(), ViewKind::Board);
    let filter = ViewFilter {
        assignee: ViewAssignee::Manager,
        hides_finished: true,
        tag: Some(tag.id()),
    };
    workspace.views.set_filter(board.id(), filter).unwrap();

    workspace.tags.remove(tag.id()).unwrap();

    let stored = workspace.tasks.get(task.id()).unwrap();
    assert_eq!(
        stored.tags().iter().copied().collect::<Vec<_>>(),
        [kept.id()]
    );
    assert!(matches!(
        workspace.tasks.history(task.id()).unwrap().last().map(|change| &change.kind),
        Some(TaskChangeKind::Untagged { tag: untagged }) if *untagged == tag.id()
    ));
    let board = workspace
        .views
        .list_for_project(workspace.project.id())
        .unwrap()[0]
        .clone();
    assert_eq!(
        board.filter(),
        ViewFilter {
            tag: None,
            ..filter
        }
    );
    assert_eq!(workspace.names(), ["Power"]);
    assert_eq!(
        workspace.tags.rename(tag.id(), "EMC").unwrap_err().code(),
        "not_found"
    );
}

#[test]
fn a_view_is_filtered_only_by_a_tag_of_its_own_project() {
    let workspace = workspace();
    let other = workspace.other_project();
    let foreign = workspace.tags.add(other.id(), "EMC", None).unwrap();
    let board = workspace
        .views
        .list_for_project(workspace.project.id())
        .unwrap()[0]
        .clone();

    let error = workspace
        .views
        .set_filter(
            board.id(),
            ViewFilter {
                tag: Some(foreign.id()),
                ..ViewFilter::EVERY_TASK
            },
        )
        .unwrap_err();

    assert_eq!(error.code(), "different_projects");
    let missing = workspace
        .views
        .set_filter(
            board.id(),
            ViewFilter {
                tag: Some(TagId::generate()),
                ..ViewFilter::EVERY_TASK
            },
        )
        .unwrap_err();
    assert_eq!(missing.code(), "not_found");
}
