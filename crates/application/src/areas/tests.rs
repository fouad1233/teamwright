//! The areas of a project, as `demoAreas.test.ts` describes them for the
//! demo of the interface: the same cases, under the same names, in the same
//! groups, with the same codes and messages.
//!
//! Three cases differ in what they start from. The demo's sample workspace
//! is a project made by hand; here the sample project is made through the
//! use cases, as every project is. A project that was kept before it had
//! areas gets "General" from a migration of the database, so that case is
//! tested with the migrations of the storage crate, under the same name.
//! A task whose area the project does not have can only come from another
//! computer, so that case is tested where the storage crate takes in what
//! another computer did, under the same name. And an identifier that names
//! nothing is a well-formed identifier here, because the commands refuse
//! text that is not one before a use case runs.

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::Timestamp;

use super::*;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
    InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::sprints::{NewSprint, SprintService};
use crate::tasks::{NewTask, TaskService};

/// The services on shared storage, and a project called "Air quality node".
struct Workspace {
    projects: ProjectService,
    tasks: TaskService,
    sprints: SprintService,
    areas: AreaService,
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
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        sprints_store.clone(),
        Arc::new(InMemoryViewRepository::default()),
        areas_store.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store,
        conversations_store,
        sprints_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let sprints = SprintService::new(
        projects_store.clone(),
        tasks_store.clone(),
        sprints_store,
        clock.clone(),
    );
    let areas = AreaService::new(projects_store, tasks_store, areas_store, clock);
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
        sprints,
        areas,
        project,
    }
}

impl Workspace {
    fn areas(&self) -> Vec<Area> {
        self.areas.list_for_project(self.project.id()).unwrap()
    }

    fn names(&self) -> Vec<String> {
        names_of(&self.areas())
    }

    /// The area of the project that has a name.
    fn area_named(&self, name: &str) -> Area {
        self.areas()
            .into_iter()
            .find(|area| area.name().as_str() == name)
            .unwrap_or_else(|| panic!("the project has no area called {name}"))
    }

    /// Gives the project the areas named, after "General".
    fn add_areas(&self, added: &[&str]) {
        for name in added {
            self.areas.add(self.project.id(), name).unwrap();
        }
    }

    /// Opens a task in the project, as a part of `parent` and in `area` if
    /// they are given.
    fn task(
        &self,
        title: &str,
        parent: Option<&Task>,
        area: Option<AreaId>,
    ) -> Result<Task, ApplicationError> {
        self.tasks.create(NewTask {
            parent_id: parent.map(Task::id),
            area_id: area,
            ..NewTask::titled(self.project.id(), title)
        })
    }

    fn stored(&self, task: &Task) -> Task {
        self.tasks.get(task.id()).unwrap()
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

    /// The first area of another project.
    fn area_elsewhere(&self) -> Area {
        let other = self.other_project();
        self.areas.list_for_project(other.id()).unwrap().remove(0)
    }
}

fn names_of(areas: &[Area]) -> Vec<String> {
    areas.iter().map(|area| area.name().to_string()).collect()
}

mod the_areas_of_the_sample_workspace {
    use super::*;

    #[test]
    fn cuts_the_sample_project_into_three_areas_and_puts_every_task_into_one_of_them() {
        let workspace = workspace();
        let general = workspace.area_named("General");
        workspace.areas.rename(general.id(), "Electronics").unwrap();
        workspace.add_areas(&["Firmware", "Enclosure"]);
        let mut tasks = Vec::new();
        for (name, title) in [
            ("Electronics", "Draw the schematic"),
            ("Firmware", "Write the drivers"),
            ("Enclosure", "Design the enclosure"),
        ] {
            let area = workspace.area_named(name).id();
            let task = workspace.task(title, None, Some(area)).unwrap();
            tasks.push(workspace.task("A part of it", Some(&task), None).unwrap());
            tasks.push(task);
        }

        let areas = workspace.areas();
        let tasks = workspace
            .tasks
            .list_for_project(workspace.project.id())
            .unwrap();

        assert_eq!(names_of(&areas), ["Electronics", "Firmware", "Enclosure"]);
        for area in &areas {
            assert_eq!(area.project_id(), workspace.project.id());
            assert!(tasks.iter().any(|task| task.area_id() == area.id()));
        }
        for task in &tasks {
            let parent = tasks
                .iter()
                .find(|other| Some(other.id()) == task.parent_id());
            assert_eq!(task.area_id(), parent.map_or(task.area_id(), Task::area_id));
            assert!(areas.iter().any(|area| area.id() == task.area_id()));
        }
    }
}

mod the_areas_of_a_project {
    use super::*;

    #[test]
    fn starts_a_new_project_with_one_area_called_general() {
        let workspace = workspace();

        let listed = workspace.areas();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].project_id(), workspace.project.id());
        assert_eq!(listed[0].name().as_str(), "General");
    }

    #[test]
    fn refuses_to_list_the_areas_of_a_project_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ProjectId::generate();

        let error = workspace.areas.list_for_project(nowhere).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no project with the identifier {nowhere}")
        );
    }

    #[test]
    fn removes_the_areas_with_their_project() {
        let workspace = workspace();
        let general = workspace.area_named("General");

        workspace.projects.delete(workspace.project.id()).unwrap();

        let listed = workspace
            .areas
            .list_for_project(workspace.project.id())
            .unwrap_err();
        assert_eq!(listed.code(), "not_found");
        let error = workspace.areas.rename(general.id(), "Board").unwrap_err();
        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {}", general.id())
        );
    }

    #[test]
    fn keeps_the_areas_of_a_project_apart_from_those_of_another() {
        let workspace = workspace();
        let other = workspace.other_project();

        workspace.areas.add(other.id(), "Firmware").unwrap();

        assert_eq!(workspace.names(), ["General"]);
        assert_eq!(
            names_of(&workspace.areas.list_for_project(other.id()).unwrap()),
            ["General", "Firmware"]
        );
    }
}

mod adding_an_area {
    use super::*;

    #[test]
    fn adds_it_at_the_end() {
        let workspace = workspace();

        let added = workspace
            .areas
            .add(workspace.project.id(), "Firmware")
            .unwrap();

        assert_eq!(added.project_id(), workspace.project.id());
        assert_eq!(added.name().as_str(), "Firmware");
        assert_eq!(workspace.areas().last(), Some(&added));
    }

    #[test]
    fn trims_the_name_it_is_given_and_puts_it_on_one_line() {
        let workspace = workspace();

        let added = workspace
            .areas
            .add(workspace.project.id(), "  Power \n  supply ")
            .unwrap();

        assert_eq!(added.name().as_str(), "Power supply");
    }

    #[test]
    fn accepts_a_name_of_40_characters() {
        let workspace = workspace();

        let added = workspace
            .areas
            .add(workspace.project.id(), &"x".repeat(40))
            .unwrap();

        assert_eq!(added.name().as_str().chars().count(), 40);
    }

    #[test]
    fn refuses_a_name_of_more_than_40_characters() {
        let workspace = workspace();

        let error = workspace
            .areas
            .add(workspace.project.id(), &"x".repeat(41))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the area name can have at most 40 characters, but this one has 41"
        );
    }

    #[test]
    fn refuses_an_empty_name() {
        let workspace = workspace();

        let error = workspace
            .areas
            .add(workspace.project.id(), " \n ")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "the area name must not be empty");
    }

    #[test]
    fn refuses_a_name_another_area_of_the_project_has_whatever_the_capitals() {
        let workspace = workspace();

        let error = workspace
            .areas
            .add(workspace.project.id(), " GENERAL ")
            .unwrap_err();

        assert_eq!(error.code(), "area_name_taken");
        assert_eq!(
            error.to_string(),
            "the project already has an area called \"GENERAL\""
        );
    }

    #[test]
    fn accepts_a_name_an_area_of_another_project_has() {
        let workspace = workspace();
        let other = workspace.other_project();
        workspace.areas.add(other.id(), "Firmware").unwrap();

        let added = workspace
            .areas
            .add(workspace.project.id(), "Firmware")
            .unwrap();

        assert_eq!(added.name().as_str(), "Firmware");
    }

    #[test]
    fn refuses_an_area_for_a_project_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = ProjectId::generate();

        let error = workspace.areas.add(nowhere, "Firmware").unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no project with the identifier {nowhere}")
        );
    }
}

mod renaming_an_area {
    use super::*;

    #[test]
    fn trims_the_new_name_and_keeps_the_place_of_the_area() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware", "Enclosure"]);
        let firmware = workspace.area_named("Firmware");

        let renamed = workspace
            .areas
            .rename(firmware.id(), "  Software  ")
            .unwrap();

        let mut expected = firmware.clone();
        expected.rename(AreaName::parse("Software").unwrap());
        assert_eq!(renamed, expected);
        assert_eq!(workspace.names(), ["General", "Software", "Enclosure"]);
    }

    #[test]
    fn lets_an_area_change_the_capitals_of_its_own_name() {
        let workspace = workspace();

        let renamed = workspace
            .areas
            .rename(workspace.area_named("General").id(), "GENERAL")
            .unwrap();

        assert_eq!(renamed.name().as_str(), "GENERAL");
    }

    #[test]
    fn refuses_an_empty_name() {
        let workspace = workspace();

        let error = workspace
            .areas
            .rename(workspace.area_named("General").id(), "   ")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "the area name must not be empty");
    }

    #[test]
    fn refuses_a_name_of_more_than_40_characters() {
        let workspace = workspace();

        let error = workspace
            .areas
            .rename(workspace.area_named("General").id(), &"y".repeat(45))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the area name can have at most 40 characters, but this one has 45"
        );
    }

    #[test]
    fn refuses_the_name_of_another_area_of_the_project_whatever_the_capitals() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);

        let error = workspace
            .areas
            .rename(workspace.area_named("General").id(), "firmware")
            .unwrap_err();

        assert_eq!(error.code(), "area_name_taken");
        assert_eq!(
            error.to_string(),
            "the project already has an area called \"firmware\""
        );
    }

    #[test]
    fn refuses_an_area_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = AreaId::generate();

        let error = workspace.areas.rename(nowhere, "Board").unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }
}

mod moving_an_area {
    use super::*;

    fn with_four_areas() -> Workspace {
        let workspace = workspace();
        workspace.add_areas(&["Board", "Firmware", "Enclosure"]);
        workspace
    }

    #[test]
    fn puts_it_at_the_place_it_is_given_and_answers_with_the_areas_in_their_new_order() {
        let workspace = with_four_areas();

        let moved = workspace
            .areas
            .move_to(workspace.area_named("Enclosure").id(), 0)
            .unwrap();

        let order = ["Enclosure", "General", "Board", "Firmware"];
        assert_eq!(names_of(&moved), order);
        assert_eq!(workspace.names(), order);
    }

    #[test]
    fn moves_an_area_one_place_down() {
        let workspace = with_four_areas();

        workspace
            .areas
            .move_to(workspace.area_named("General").id(), 1)
            .unwrap();

        assert_eq!(
            workspace.names(),
            ["Board", "General", "Firmware", "Enclosure"]
        );
    }

    #[test]
    fn puts_it_last_for_a_place_past_the_end_and_first_for_a_place_before_the_start() {
        let workspace = with_four_areas();

        workspace
            .areas
            .move_to(workspace.area_named("General").id(), 99)
            .unwrap();
        workspace
            .areas
            .move_to(workspace.area_named("Firmware").id(), -3)
            .unwrap();

        assert_eq!(
            workspace.names(),
            ["Firmware", "Board", "Enclosure", "General"]
        );
    }

    #[test]
    fn refuses_an_area_that_does_not_exist() {
        let workspace = with_four_areas();
        let nowhere = AreaId::generate();

        let error = workspace.areas.move_to(nowhere, 0).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }
}

mod removing_an_area {
    use super::*;

    #[test]
    fn removes_an_area_without_tasks_and_keeps_the_others_in_their_order() {
        let workspace = workspace();
        workspace.add_areas(&["Board", "Firmware"]);

        workspace
            .areas
            .remove(workspace.area_named("Board").id(), None)
            .unwrap();

        assert_eq!(workspace.names(), ["General", "Firmware"]);
    }

    #[test]
    fn does_not_look_at_where_the_tasks_go_when_the_area_has_none() {
        let workspace = workspace();
        workspace.add_areas(&["Board"]);

        workspace
            .areas
            .remove(workspace.area_named("Board").id(), Some(AreaId::generate()))
            .unwrap();

        assert_eq!(workspace.names(), ["General"]);
    }

    #[test]
    fn refuses_to_remove_the_last_area_of_a_project() {
        let workspace = workspace();

        let error = workspace
            .areas
            .remove(workspace.area_named("General").id(), None)
            .unwrap_err();

        assert_eq!(error.code(), "last_area");
        assert_eq!(
            error.to_string(),
            "\"General\" is the last area of the project and cannot be removed"
        );
        assert_eq!(workspace.names(), ["General"]);
    }

    #[test]
    fn refuses_to_remove_an_area_with_a_task_without_saying_where_it_goes() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();

        let error = workspace.areas.remove(firmware.id(), None).unwrap_err();

        assert_eq!(error.code(), "area_in_use");
        assert_eq!(
            error.to_string(),
            "the area \"Firmware\" still has 1 task, which need another area to go to"
        );
    }

    #[test]
    fn counts_the_subtasks_among_the_tasks_that_need_another_area() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        let drivers = workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();
        workspace
            .task("Driver for the oxygen sensor", Some(&drivers), None)
            .unwrap();
        workspace
            .task("Driver for the humidity sensor", Some(&drivers), None)
            .unwrap();

        let error = workspace
            .areas
            .remove(firmware.id(), Some(firmware.id()))
            .unwrap_err();

        assert_eq!(error.code(), "area_in_use");
        assert_eq!(
            error.to_string(),
            "the area \"Firmware\" still has 3 tasks, which need another area to go to"
        );
        assert_eq!(workspace.names(), ["General", "Firmware"]);
    }

    #[test]
    fn moves_its_tasks_to_the_area_it_is_given_in_their_status_and_at_their_place() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let general = workspace.area_named("General");
        let firmware = workspace.area_named("Firmware");
        let drivers = workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();
        let part = workspace
            .task("Driver for the oxygen sensor", Some(&drivers), None)
            .unwrap();

        workspace
            .areas
            .remove(firmware.id(), Some(general.id()))
            .unwrap();

        assert_eq!(workspace.names(), ["General"]);
        let moved = workspace.stored(&drivers);
        assert_eq!(moved.area_id(), general.id());
        assert_eq!(moved.status_id(), drivers.status_id());
        assert_eq!(moved.rank(), drivers.rank());
        assert_eq!(workspace.stored(&part).area_id(), general.id());
    }

    #[test]
    fn refuses_to_move_its_tasks_to_an_area_of_another_project() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();
        let elsewhere = workspace.area_elsewhere();

        let error = workspace
            .areas
            .remove(firmware.id(), Some(elsewhere.id()))
            .unwrap_err();

        assert_eq!(error.code(), "different_projects");
        assert_eq!(
            error.to_string(),
            "the tasks of an area can only go to another area of the same project"
        );
        assert_eq!(workspace.names(), ["General", "Firmware"]);
    }

    #[test]
    fn refuses_to_move_its_tasks_to_an_area_that_does_not_exist() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();
        let nowhere = AreaId::generate();

        let error = workspace
            .areas
            .remove(firmware.id(), Some(nowhere))
            .unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }

    #[test]
    fn refuses_an_area_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = AreaId::generate();

        let error = workspace.areas.remove(nowhere, None).unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }
}

mod the_area_of_a_new_task {
    use super::*;

    #[test]
    fn puts_a_task_into_the_first_area_when_none_is_named() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        workspace
            .areas
            .move_to(workspace.area_named("Firmware").id(), 0)
            .unwrap();

        let created = workspace.task("Write the drivers", None, None).unwrap();

        assert_eq!(created.area_id(), workspace.area_named("Firmware").id());
    }

    #[test]
    fn puts_a_task_into_the_area_it_names() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware", "Enclosure"]);
        let enclosure = workspace.area_named("Enclosure");

        let created = workspace
            .task("Design the enclosure", None, Some(enclosure.id()))
            .unwrap();

        assert_eq!(created.area_id(), enclosure.id());
    }

    #[test]
    fn puts_a_subtask_into_the_area_of_its_task_named_or_not() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        let drivers = workspace
            .task("Write the drivers", None, Some(firmware.id()))
            .unwrap();

        let first = workspace
            .task("Driver for the oxygen sensor", Some(&drivers), None)
            .unwrap();
        let second = workspace
            .task(
                "Driver for the humidity sensor",
                Some(&drivers),
                Some(firmware.id()),
            )
            .unwrap();

        assert_eq!(first.area_id(), firmware.id());
        assert_eq!(second.area_id(), firmware.id());
    }

    #[test]
    fn refuses_a_subtask_in_another_area_than_its_task() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let drivers = workspace
            .task(
                "Write the drivers",
                None,
                Some(workspace.area_named("Firmware").id()),
            )
            .unwrap();

        let error = workspace
            .task(
                "Driver for the oxygen sensor",
                Some(&drivers),
                Some(workspace.area_named("General").id()),
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "a subtask is in the area of the task it is a part of, AQN-1"
        );
    }

    #[test]
    fn refuses_an_area_of_another_project() {
        let workspace = workspace();
        let elsewhere = workspace.area_elsewhere();

        let error = workspace
            .task("Write the drivers", None, Some(elsewhere.id()))
            .unwrap_err();

        assert_eq!(error.code(), "different_projects");
        assert_eq!(
            error.to_string(),
            "a task can only be put into an area of its own project"
        );
    }

    #[test]
    fn refuses_an_area_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = AreaId::generate();

        let error = workspace
            .task("Write the drivers", None, Some(nowhere))
            .unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }
}

mod moving_a_task_to_another_area {
    use super::*;

    #[test]
    fn moves_the_task_with_its_subtasks_and_answers_with_the_task() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let firmware = workspace.area_named("Firmware");
        let drivers = workspace.task("Write the drivers", None, None).unwrap();
        let part = workspace
            .task("Driver for the oxygen sensor", Some(&drivers), None)
            .unwrap();
        let deeper = workspace
            .task("Read the sensor in sleep", Some(&part), None)
            .unwrap();

        let moved = workspace
            .areas
            .move_task(drivers.id(), firmware.id())
            .unwrap();

        assert_eq!(moved.id(), drivers.id());
        assert_eq!(moved.area_id(), firmware.id());
        assert_eq!(workspace.stored(&part).area_id(), firmware.id());
        assert_eq!(workspace.stored(&deeper).area_id(), firmware.id());
    }

    #[test]
    fn keeps_the_status_the_place_and_the_sprint_of_the_task() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let sprint = workspace
            .sprints
            .create(NewSprint {
                project_id: workspace.project.id(),
                name: String::new(),
                goal: String::new(),
                start_on: None,
                end_on: None,
            })
            .unwrap();
        let drivers = workspace.task("Write the drivers", None, None).unwrap();
        workspace
            .tasks
            .plan(drivers.id(), Some(sprint.id()))
            .unwrap();

        let moved = workspace
            .areas
            .move_task(drivers.id(), workspace.area_named("Firmware").id())
            .unwrap();

        assert_eq!(moved.status_id(), drivers.status_id());
        assert_eq!(moved.rank(), drivers.rank());
        assert_eq!(moved.sprint_id(), Some(sprint.id()));
    }

    #[test]
    fn changes_nothing_when_the_task_is_in_the_area_already() {
        let workspace = workspace();
        let drivers = workspace.task("Write the drivers", None, None).unwrap();

        let moved = workspace
            .areas
            .move_task(drivers.id(), drivers.area_id())
            .unwrap();

        assert_eq!(moved, drivers);
    }

    #[test]
    fn refuses_to_move_a_subtask_by_itself() {
        let workspace = workspace();
        workspace.add_areas(&["Firmware"]);
        let drivers = workspace.task("Write the drivers", None, None).unwrap();
        let part = workspace
            .task("Driver for the oxygen sensor", Some(&drivers), None)
            .unwrap();

        let error = workspace
            .areas
            .move_task(part.id(), workspace.area_named("Firmware").id())
            .unwrap_err();

        assert_eq!(error.code(), "subtask_moved_alone");
        assert_eq!(
            error.to_string(),
            "AQN-2 is a part of AQN-1 and goes where it goes"
        );
    }

    #[test]
    fn refuses_an_area_of_another_project() {
        let workspace = workspace();
        let drivers = workspace.task("Write the drivers", None, None).unwrap();
        let elsewhere = workspace.area_elsewhere();

        let error = workspace
            .areas
            .move_task(drivers.id(), elsewhere.id())
            .unwrap_err();

        assert_eq!(error.code(), "different_projects");
        assert_eq!(
            error.to_string(),
            "a task can only be put into an area of its own project"
        );
        assert_eq!(workspace.stored(&drivers).area_id(), drivers.area_id());
    }

    #[test]
    fn refuses_a_task_that_does_not_exist() {
        let workspace = workspace();
        let nowhere = TaskId::generate();

        let error = workspace
            .areas
            .move_task(nowhere, workspace.area_named("General").id())
            .unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no task with the identifier {nowhere}")
        );
    }

    #[test]
    fn refuses_an_area_that_does_not_exist() {
        let workspace = workspace();
        let drivers = workspace.task("Write the drivers", None, None).unwrap();
        let nowhere = AreaId::generate();

        let error = workspace
            .areas
            .move_task(drivers.id(), nowhere)
            .unwrap_err();

        assert_eq!(error.code(), "not_found");
        assert_eq!(
            error.to_string(),
            format!("there is no area with the identifier {nowhere}")
        );
    }
}
