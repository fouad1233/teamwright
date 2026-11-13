//! Sprints and the backlog, as `demoSprints.test.ts` describes them for the
//! demo of the interface: the same cases, under the same names, with the
//! same codes and messages. The cases about the sample workspace of the
//! demo have nothing to mirror here.

use std::sync::Arc;

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{Color, StatusCategory, StatusId, Task, TaskId, Timestamp};

use super::*;
use crate::in_memory::{
    InMemoryConversationRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
    InMemoryProjectRepository, InMemorySprintRepository, InMemoryTaskRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService, StatusProperties, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};

/// The services on shared storage, a clock that starts on 11 November 2026
/// at nine, and a project with an engineering workflow: to do, in
/// progress, in revision, in test, done.
struct Backlog {
    projects: ProjectService,
    tasks: TaskService,
    sprints: SprintService,
    clock: Arc<ManualClock>,
    project: Project,
}

fn start_of_day() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
}

/// A day of November 2026.
fn day(day_of_month: u32) -> CalendarDate {
    CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
}

fn backlog() -> Backlog {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations_store = Arc::new(InMemoryConversationRepository::default());
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let area_store = Arc::new(crate::in_memory::InMemoryAreaRepository::default());
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemoryPlacesRepository::default()),
        sprints_store.clone(),
        Arc::new(crate::in_memory::InMemoryViewRepository::default()),
        area_store.clone(),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store,
        conversations_store,
        sprints_store.clone(),
        area_store.clone(),
        clock.clone(),
    );
    let sprints = SprintService::new(projects_store, tasks_store, sprints_store, clock.clone());
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    Backlog {
        projects,
        tasks,
        sprints,
        clock,
        project,
    }
}

impl Backlog {
    /// The first status of a category in the workflow of the project.
    fn status_of(&self, category: StatusCategory) -> StatusId {
        self.project.workflow().first_in(category).unwrap().id()
    }

    fn task(&self, title: &str) -> Task {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
    }

    fn subtask(&self, title: &str, parent: &Task) -> Task {
        self.tasks
            .create(NewTask {
                parent_id: Some(parent.id()),
                ..NewTask::titled(self.project.id(), title)
            })
            .unwrap()
    }

    /// A task as it is now.
    fn now(&self, task_id: TaskId) -> Task {
        self.tasks.get(task_id).unwrap()
    }

    fn sprint_now(&self, sprint_id: SprintId) -> Option<Sprint> {
        self.sprints
            .list_for_project(self.project.id())
            .unwrap()
            .into_iter()
            .find(|sprint| sprint.id() == sprint_id)
    }

    fn new_sprint(&self) -> Sprint {
        self.sprints
            .create(NewSprint::in_project(self.project.id()))
            .unwrap()
    }

    fn finish(&self, task_id: TaskId) {
        self.tasks
            .move_to(task_id, self.status_of(StatusCategory::Done), 0)
            .unwrap();
    }

    fn pass(&self, days: i64) {
        self.clock.advance(TimeDelta::days(days));
    }

    /// A planned sprint with one task in it, started from 10 to 23
    /// November.
    fn active_sprint(&self) -> (Sprint, Task) {
        let sprint = self.new_sprint();
        let planned = self.task("Draw the schematic");
        self.tasks.plan(planned.id(), Some(sprint.id())).unwrap();
        let started = self
            .sprints
            .start(
                sprint.id(),
                SprintStart {
                    name: sprint.name().to_string(),
                    goal: String::new(),
                    start_on: day(10),
                    end_on: day(23),
                },
            )
            .unwrap();
        (started, self.now(planned.id()))
    }

    fn complete_to_backlog(&self, sprint: &Sprint) -> Sprint {
        self.sprints.complete(sprint.id(), None).unwrap()
    }

    fn other_project(&self) -> Project {
        self.projects
            .create(NewProject {
                name: "Weather station".to_owned(),
                ..NewProject::default()
            })
            .unwrap()
    }
}

/// The code and the message of an error, as the interface receives them.
fn refusal<T: std::fmt::Debug>(result: Result<T, ApplicationError>) -> (&'static str, String) {
    let error = result.unwrap_err();
    (error.code(), error.to_string())
}

fn summary(unit: SprintUnit, committed: u32, completed: u32) -> Option<SprintSummary> {
    Some(SprintSummary {
        unit,
        committed,
        completed,
    })
}

mod creating_a_sprint {
    use super::*;

    #[test]
    fn plans_a_sprint_without_dates_named_after_the_number_of_sprints_the_project_has_had() {
        let backlog = backlog();
        backlog.new_sprint();

        let second = backlog
            .sprints
            .create(NewSprint {
                name: "   ".to_owned(),
                ..NewSprint::in_project(backlog.project.id())
            })
            .unwrap();

        assert_eq!(second.project_id(), backlog.project.id());
        assert_eq!(second.name().as_str(), "Sprint 2");
        assert_eq!(second.goal(), "");
        assert_eq!(second.dates(), SprintDates::default());
        assert_eq!(second.state(), SprintState::Planned);
        assert_eq!(second.started_at(), None);
        assert_eq!(second.completed_at(), None);
        assert_eq!(second.summary(), None);
    }

    #[test]
    fn trims_the_name_and_the_goal_and_keeps_the_dates() {
        let backlog = backlog();

        let sprint = backlog
            .sprints
            .create(NewSprint {
                name: "  The   first board  ".to_owned(),
                goal: " A schematic that can be routed ".to_owned(),
                start_on: Some(day(16)),
                end_on: Some(day(29)),
                ..NewSprint::in_project(backlog.project.id())
            })
            .unwrap();

        assert_eq!(sprint.name().as_str(), "The first board");
        assert_eq!(sprint.goal(), "A schematic that can be routed");
        assert_eq!(sprint.dates().start_on(), Some(day(16)));
        assert_eq!(sprint.dates().end_on(), Some(day(29)));
    }

    #[test]
    fn counts_a_deleted_sprint_when_it_names_a_new_one() {
        let backlog = backlog();
        let first = backlog.new_sprint();
        backlog.sprints.delete(first.id()).unwrap();

        let next = backlog.new_sprint();

        assert_eq!(next.name().as_str(), "Sprint 2");
    }

    #[test]
    fn refuses_a_name_of_more_than_60_characters() {
        let backlog = backlog();

        let (code, message) = refusal(backlog.sprints.create(NewSprint {
            name: "x".repeat(61),
            ..NewSprint::in_project(backlog.project.id())
        }));

        assert_eq!(code, "rule_broken");
        assert_eq!(
            message,
            "the sprint name can have at most 60 characters, but this one has 61"
        );
    }

    #[test]
    fn refuses_a_start_date_after_the_end_date() {
        let backlog = backlog();

        let (code, message) = refusal(backlog.sprints.create(NewSprint {
            start_on: Some(day(20)),
            end_on: Some(day(19)),
            ..NewSprint::in_project(backlog.project.id())
        }));

        assert_eq!(code, "rule_broken");
        assert_eq!(
            message,
            "the start date 2026-11-20 is after the end date 2026-11-19"
        );
    }

    #[test]
    fn refuses_a_project_that_does_not_exist() {
        let backlog = backlog();

        let (code, _) = refusal(
            backlog
                .sprints
                .create(NewSprint::in_project(ProjectId::generate())),
        );

        assert_eq!(code, "not_found");
    }
}

mod listing_the_sprints {
    use super::*;

    #[test]
    fn lists_the_active_sprint_then_the_planned_ones_as_created_then_the_completed_ones_the_latest_first()
     {
        let backlog = backlog();
        let (first, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&first);
        backlog.pass(1);
        let (second, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&second);
        let planned = backlog.new_sprint();
        let later = backlog.new_sprint();
        let (active, _) = backlog.active_sprint();

        let listed: Vec<SprintId> = backlog
            .sprints
            .list_for_project(backlog.project.id())
            .unwrap()
            .iter()
            .map(Sprint::id)
            .collect();

        assert_eq!(
            listed,
            [
                active.id(),
                planned.id(),
                later.id(),
                second.id(),
                first.id()
            ]
        );
    }
}

mod changing_a_sprint {
    use super::*;

    #[test]
    fn replaces_the_name_the_goal_and_the_dates_of_a_planned_sprint() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();

        let changed = backlog
            .sprints
            .change(
                sprint.id(),
                SprintChanges {
                    name: " Board bring-up ".to_owned(),
                    goal: " Power on ".to_owned(),
                    start_on: Some(day(16)),
                    end_on: Some(day(20)),
                },
            )
            .unwrap();

        assert_eq!(changed.name().as_str(), "Board bring-up");
        assert_eq!(changed.goal(), "Power on");
        assert_eq!(changed.dates().start_on(), Some(day(16)));
        assert_eq!(changed.dates().end_on(), Some(day(20)));
        assert_eq!(changed.state(), SprintState::Planned);
    }

    #[test]
    fn changes_a_sprint_that_is_active() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();

        let changed = backlog
            .sprints
            .change(
                sprint.id(),
                SprintChanges {
                    name: sprint.name().to_string(),
                    goal: "Finish the schematic".to_owned(),
                    start_on: Some(day(10)),
                    end_on: Some(day(30)),
                },
            )
            .unwrap();

        assert_eq!(changed.goal(), "Finish the schematic");
        assert_eq!(changed.dates().end_on(), Some(day(30)));
    }

    #[test]
    fn refuses_to_take_the_dates_away_from_a_sprint_that_has_started() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();

        let (code, message) = refusal(backlog.sprints.change(
            sprint.id(),
            SprintChanges {
                name: sprint.name().to_string(),
                goal: String::new(),
                start_on: None,
                end_on: None,
            },
        ));

        assert_eq!(code, "rule_broken");
        assert_eq!(
            message,
            "a sprint that has started has a start and an end date"
        );
    }

    #[test]
    fn refuses_an_empty_name() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();

        let (code, message) = refusal(backlog.sprints.change(
            sprint.id(),
            SprintChanges {
                name: "  ".to_owned(),
                goal: String::new(),
                start_on: None,
                end_on: None,
            },
        ));

        assert_eq!(code, "rule_broken");
        assert_eq!(message, "the sprint name must not be empty");
    }

    #[test]
    fn refuses_to_change_a_completed_sprint() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&sprint);

        let (code, message) = refusal(backlog.sprints.change(
            sprint.id(),
            SprintChanges {
                name: "Again".to_owned(),
                goal: String::new(),
                start_on: None,
                end_on: None,
            },
        ));

        assert_eq!(code, "sprint_completed");
        assert_eq!(
            message,
            "\"Sprint 1\" is completed and can no longer be changed"
        );
    }
}

mod deleting_a_sprint {
    use super::*;

    #[test]
    fn deletes_a_planned_sprint_and_puts_its_tasks_back_into_the_backlog() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let planned = backlog.task("Route the board");
        backlog.tasks.plan(planned.id(), Some(sprint.id())).unwrap();

        backlog.sprints.delete(sprint.id()).unwrap();

        assert!(
            backlog
                .sprints
                .list_for_project(backlog.project.id())
                .unwrap()
                .is_empty()
        );
        assert_eq!(backlog.now(planned.id()).sprint_id(), None);
    }

    #[test]
    fn refuses_to_delete_a_sprint_that_is_active() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();

        let (code, message) = refusal(backlog.sprints.delete(sprint.id()));

        assert_eq!(code, "sprint_active");
        assert_eq!(
            message,
            "\"Sprint 1\" has started and cannot be deleted; complete it instead"
        );
    }

    #[test]
    fn refuses_to_delete_a_completed_sprint() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&sprint);

        let (code, message) = refusal(backlog.sprints.delete(sprint.id()));

        assert_eq!(code, "sprint_completed");
        assert_eq!(message, "\"Sprint 1\" is completed and is kept as a record");
    }
}

mod planning_a_task {
    use super::*;

    #[test]
    fn puts_a_task_into_a_planned_sprint_and_its_subtasks_with_it() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let parent = backlog.task("Write the sensor drivers");
        let part = backlog.subtask("Driver for the oxygen sensor", &parent);

        let planned = backlog.tasks.plan(parent.id(), Some(sprint.id())).unwrap();

        assert_eq!(planned.sprint_id(), Some(sprint.id()));
        assert_eq!(backlog.now(part.id()).sprint_id(), Some(sprint.id()));
    }

    #[test]
    fn puts_a_task_into_the_sprint_that_is_active() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        let late = backlog.task("Order the parts");

        let planned = backlog.tasks.plan(late.id(), Some(sprint.id())).unwrap();

        assert_eq!(planned.sprint_id(), Some(sprint.id()));
    }

    #[test]
    fn takes_a_task_back_into_the_backlog_and_its_subtasks_with_it() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let parent = backlog.task("Write the sensor drivers");
        let part = backlog.subtask("Driver for the oxygen sensor", &parent);
        backlog.tasks.plan(parent.id(), Some(sprint.id())).unwrap();

        let back = backlog.tasks.plan(parent.id(), None).unwrap();

        assert_eq!(back.sprint_id(), None);
        assert_eq!(backlog.now(part.id()).sprint_id(), None);
    }

    #[test]
    fn refuses_to_plan_into_a_completed_sprint() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&sprint);
        let late = backlog.task("Order the parts");

        let (code, message) = refusal(backlog.tasks.plan(late.id(), Some(sprint.id())));

        assert_eq!(code, "sprint_completed");
        assert_eq!(message, "\"Sprint 1\" is completed and takes no more tasks");
    }

    #[test]
    fn refuses_to_plan_a_subtask_by_itself() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let parent = backlog.task("Write the sensor drivers");
        let part = backlog.subtask("Driver for the oxygen sensor", &parent);

        let (code, message) = refusal(backlog.tasks.plan(part.id(), Some(sprint.id())));

        assert_eq!(code, "subtask_planned_alone");
        assert_eq!(message, "AQN-2 is a part of AQN-1 and goes where it goes");
    }

    #[test]
    fn refuses_a_sprint_of_another_project() {
        let backlog = backlog();
        let other = backlog.other_project();
        let sprint = backlog
            .sprints
            .create(NewSprint::in_project(other.id()))
            .unwrap();
        let planned = backlog.task("Route the board");

        let (code, _) = refusal(backlog.tasks.plan(planned.id(), Some(sprint.id())));

        assert_eq!(code, "different_projects");
    }

    #[test]
    fn refuses_a_sprint_that_does_not_exist() {
        let backlog = backlog();
        let planned = backlog.task("Route the board");

        let (code, _) = refusal(backlog.tasks.plan(planned.id(), Some(SprintId::generate())));

        assert_eq!(code, "not_found");
    }

    #[test]
    fn opens_a_new_task_in_the_backlog_without_points() {
        let backlog = backlog();

        let created = backlog.task("Route the board");

        assert_eq!(created.sprint_id(), None);
        assert_eq!(created.points(), None);
    }

    #[test]
    fn opens_a_new_subtask_in_the_sprint_of_its_task() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let parent = backlog.task("Write the sensor drivers");
        backlog.tasks.plan(parent.id(), Some(sprint.id())).unwrap();
        let parent = backlog.now(parent.id());

        let part = backlog.subtask("Driver for the oxygen sensor", &parent);

        assert_eq!(part.sprint_id(), Some(sprint.id()));
        assert_eq!(backlog.now(part.id()).sprint_id(), Some(sprint.id()));
    }
}

mod starting_a_sprint {
    use super::*;

    fn start(name: &str, start_on: u32, end_on: u32) -> SprintStart {
        SprintStart {
            name: name.to_owned(),
            goal: String::new(),
            start_on: day(start_on),
            end_on: day(end_on),
        }
    }

    #[test]
    fn starts_a_planned_sprint_with_its_name_its_goal_and_its_dates() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let planned = backlog.task("Draw the schematic");
        backlog.tasks.plan(planned.id(), Some(sprint.id())).unwrap();

        let started = backlog
            .sprints
            .start(
                sprint.id(),
                SprintStart {
                    name: " Schematic ".to_owned(),
                    goal: " A schematic that can be routed ".to_owned(),
                    start_on: day(11),
                    end_on: day(24),
                },
            )
            .unwrap();

        assert_eq!(started.name().as_str(), "Schematic");
        assert_eq!(started.goal(), "A schematic that can be routed");
        assert_eq!(started.dates().start_on(), Some(day(11)));
        assert_eq!(started.dates().end_on(), Some(day(24)));
        assert_eq!(started.state(), SprintState::Active);
        assert_eq!(started.started_at(), Some(start_of_day()));
    }

    #[test]
    fn refuses_a_sprint_without_tasks() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();

        let (code, message) = refusal(
            backlog
                .sprints
                .start(sprint.id(), start(sprint.name().as_str(), 11, 24)),
        );

        assert_eq!(code, "sprint_empty");
        assert_eq!(
            message,
            "\"Sprint 1\" has no tasks; plan some into it before you start it"
        );
    }

    #[test]
    fn refuses_while_another_sprint_of_the_project_is_active() {
        let backlog = backlog();
        backlog.active_sprint();
        let next = backlog.new_sprint();
        let planned = backlog.task("Route the board");
        backlog.tasks.plan(planned.id(), Some(next.id())).unwrap();

        let (code, message) = refusal(
            backlog
                .sprints
                .start(next.id(), start(next.name().as_str(), 24, 30)),
        );

        assert_eq!(code, "another_sprint_active");
        assert_eq!(
            message,
            "\"Sprint 1\" is still running; complete it before you start another sprint"
        );
    }

    #[test]
    fn refuses_a_start_date_after_the_end_date() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let planned = backlog.task("Draw the schematic");
        backlog.tasks.plan(planned.id(), Some(sprint.id())).unwrap();

        let (code, _) = refusal(
            backlog
                .sprints
                .start(sprint.id(), start(sprint.name().as_str(), 24, 11)),
        );

        assert_eq!(code, "rule_broken");
        assert_eq!(
            backlog.sprint_now(sprint.id()).unwrap().state(),
            SprintState::Planned
        );
    }

    #[test]
    fn refuses_to_start_a_completed_sprint_again() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&sprint);

        let (code, _) = refusal(
            backlog
                .sprints
                .start(sprint.id(), start(sprint.name().as_str(), 24, 30)),
        );

        assert_eq!(code, "sprint_completed");
    }
}

mod completing_a_sprint {
    use super::*;

    #[test]
    fn sends_the_unfinished_tasks_with_their_subtasks_to_the_backlog_and_keeps_the_finished_ones() {
        let backlog = backlog();
        let (sprint, unfinished) = backlog.active_sprint();
        let part = backlog.subtask("Power supply", &unfinished);
        let finished = backlog.task("Choose the microcontroller");
        backlog
            .tasks
            .plan(finished.id(), Some(sprint.id()))
            .unwrap();
        backlog.finish(finished.id());
        // A finished subtask of an unfinished task goes with its task.
        backlog.finish(part.id());

        let completed = backlog.complete_to_backlog(&sprint);

        assert_eq!(completed.state(), SprintState::Completed);
        assert_eq!(completed.completed_at(), Some(start_of_day()));
        assert_eq!(backlog.now(unfinished.id()).sprint_id(), None);
        assert_eq!(backlog.now(part.id()).sprint_id(), None);
        assert_eq!(backlog.now(finished.id()).sprint_id(), Some(sprint.id()));
    }

    #[test]
    fn sends_the_unfinished_tasks_to_a_planned_sprint_of_the_same_project() {
        let backlog = backlog();
        let (sprint, unfinished) = backlog.active_sprint();
        let next = backlog.new_sprint();

        backlog
            .sprints
            .complete(sprint.id(), Some(next.id()))
            .unwrap();

        assert_eq!(backlog.now(unfinished.id()).sprint_id(), Some(next.id()));
    }

    #[test]
    fn counts_as_finished_what_is_in_a_status_of_the_category_done_whatever_its_name() {
        let mut backlog = backlog();
        let (sprint, in_review) = backlog.active_sprint();
        let accepted = backlog.task("Choose the microcontroller");
        backlog
            .tasks
            .plan(accepted.id(), Some(sprint.id()))
            .unwrap();
        // A status of review that is called "Finished", and one of done that
        // is not.
        let review = backlog.status_of(StatusCategory::Review);
        backlog.project = backlog
            .projects
            .change_status(
                backlog.project.id(),
                review,
                StatusProperties {
                    name: "Finished".to_owned(),
                    category: StatusCategory::Review,
                    color: Color::Amber,
                },
            )
            .unwrap();
        let done = backlog.status_of(StatusCategory::Done);
        backlog.project = backlog
            .projects
            .change_status(
                backlog.project.id(),
                done,
                StatusProperties {
                    name: "Accepted".to_owned(),
                    category: StatusCategory::Done,
                    color: Color::Green,
                },
            )
            .unwrap();
        backlog.tasks.move_to(in_review.id(), review, 0).unwrap();
        backlog.finish(accepted.id());

        let completed = backlog.complete_to_backlog(&sprint);

        assert_eq!(completed.summary(), summary(SprintUnit::Tasks, 2, 1));
        assert_eq!(backlog.now(in_review.id()).sprint_id(), None);
    }

    #[test]
    fn sums_up_in_points_when_any_of_its_tasks_has_points() {
        let backlog = backlog();
        let (sprint, unfinished) = backlog.active_sprint();
        backlog
            .tasks
            .set_points(unfinished.id(), Some(5.0))
            .unwrap();
        let finished = backlog.task("Choose the microcontroller");
        backlog
            .tasks
            .plan(finished.id(), Some(sprint.id()))
            .unwrap();
        backlog.tasks.set_points(finished.id(), Some(3.0)).unwrap();
        backlog.finish(finished.id());
        // A task without points counts as no points.
        let unpointed = backlog.task("Order the parts");
        backlog
            .tasks
            .plan(unpointed.id(), Some(sprint.id()))
            .unwrap();

        let completed = backlog.complete_to_backlog(&sprint);

        assert_eq!(completed.summary(), summary(SprintUnit::Points, 8, 3));
    }

    #[test]
    fn sums_up_in_tasks_when_none_of_its_tasks_has_points_and_leaves_subtasks_out() {
        let backlog = backlog();
        let (sprint, unfinished) = backlog.active_sprint();
        backlog.subtask("Power supply", &unfinished);
        let finished = backlog.task("Choose the microcontroller");
        backlog
            .tasks
            .plan(finished.id(), Some(sprint.id()))
            .unwrap();
        backlog.finish(finished.id());

        let completed = backlog.complete_to_backlog(&sprint);

        assert_eq!(completed.summary(), summary(SprintUnit::Tasks, 2, 1));
    }

    #[test]
    fn refuses_a_sprint_that_has_not_started() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();

        let (code, message) = refusal(backlog.sprints.complete(sprint.id(), None));

        assert_eq!(code, "sprint_not_active");
        assert_eq!(
            message,
            "\"Sprint 1\" has not started, so it cannot be completed"
        );
    }

    #[test]
    fn refuses_to_send_the_unfinished_tasks_to_a_completed_sprint() {
        let backlog = backlog();
        let (earlier, _) = backlog.active_sprint();
        backlog.complete_to_backlog(&earlier);
        let (sprint, _) = backlog.active_sprint();

        let (code, _) = refusal(backlog.sprints.complete(sprint.id(), Some(earlier.id())));

        assert_eq!(code, "sprint_completed");
        assert_eq!(
            backlog.sprint_now(sprint.id()).unwrap().state(),
            SprintState::Active
        );
    }

    #[test]
    fn refuses_to_keep_the_unfinished_tasks_in_the_sprint_that_is_completed() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();

        let (code, _) = refusal(backlog.sprints.complete(sprint.id(), Some(sprint.id())));

        assert_eq!(code, "rule_broken");
    }

    #[test]
    fn refuses_to_send_the_unfinished_tasks_to_a_sprint_of_another_project() {
        let backlog = backlog();
        let (sprint, _) = backlog.active_sprint();
        let other = backlog.other_project();
        let elsewhere = backlog
            .sprints
            .create(NewSprint::in_project(other.id()))
            .unwrap();

        let (code, _) = refusal(backlog.sprints.complete(sprint.id(), Some(elsewhere.id())));

        assert_eq!(code, "different_projects");
    }
}

mod story_points {
    use super::*;

    #[test]
    fn gives_a_task_points_and_takes_them_away_again() {
        let backlog = backlog();
        let pointed = backlog.task("Draw the schematic");

        let given = backlog.tasks.set_points(pointed.id(), Some(8.0)).unwrap();
        let taken = backlog.tasks.set_points(pointed.id(), None).unwrap();

        assert_eq!(given.points().map(|points| points.value()), Some(8));
        assert_eq!(taken.points(), None);
    }

    #[test]
    fn refuses_points_that_are_not_a_whole_number_from_0_to_999() {
        let backlog = backlog();
        let pointed = backlog.task("Draw the schematic");

        let errors: Vec<(&str, String)> = [-1.0, 1000.0, 2.5]
            .into_iter()
            .map(|points| refusal(backlog.tasks.set_points(pointed.id(), Some(points))))
            .collect();

        assert_eq!(
            errors.iter().map(|(code, _)| *code).collect::<Vec<_>>(),
            ["rule_broken", "rule_broken", "rule_broken"]
        );
        assert_eq!(errors[0].1, "story points are a whole number from 0 to 999");
    }
}

mod the_report_of_a_sprint {
    use super::*;

    fn days(report: &SprintReport) -> Vec<(CalendarDate, Option<u32>)> {
        report
            .days
            .iter()
            .map(|day| (day.date, day.remaining))
            .collect()
    }

    #[test]
    fn tells_what_remained_at_the_end_of_each_day_and_nothing_for_the_days_to_come() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let mut planned = Vec::new();
        for (title, points) in [
            ("Draw the schematic", 3.0),
            ("Route the board", 5.0),
            ("Order the parts", 2.0),
        ] {
            let task = backlog.task(title);
            backlog.tasks.plan(task.id(), Some(sprint.id())).unwrap();
            backlog.tasks.set_points(task.id(), Some(points)).unwrap();
            planned.push(task);
        }
        backlog
            .sprints
            .start(
                sprint.id(),
                SprintStart {
                    name: sprint.name().to_string(),
                    goal: String::new(),
                    start_on: day(10),
                    end_on: day(14),
                },
            )
            .unwrap();
        // Finished on 11 and on 12 November.
        backlog.finish(planned[0].id());
        backlog.pass(1);
        backlog.finish(planned[1].id());

        let report = backlog.sprints.report(sprint.id()).unwrap();

        assert_eq!(report.sprint_id, sprint.id());
        assert_eq!(report.unit, SprintUnit::Points);
        assert_eq!(report.total, 10);
        assert_eq!(
            days(&report),
            [
                (day(10), Some(10)),
                (day(11), Some(7)),
                (day(12), Some(2)),
                (day(13), None),
                (day(14), None),
            ]
        );
    }

    #[test]
    fn counts_a_task_that_was_finished_before_the_sprint_began_on_its_first_day() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();
        let early = backlog.task("Choose the microcontroller");
        backlog.finish(early.id());
        backlog.tasks.plan(early.id(), Some(sprint.id())).unwrap();
        backlog.pass(2);
        backlog
            .sprints
            .start(
                sprint.id(),
                SprintStart {
                    name: sprint.name().to_string(),
                    goal: String::new(),
                    start_on: day(13),
                    end_on: day(14),
                },
            )
            .unwrap();

        let report = backlog.sprints.report(sprint.id()).unwrap();

        assert_eq!(report.unit, SprintUnit::Tasks);
        assert_eq!(
            report
                .days
                .iter()
                .map(|day| day.remaining)
                .collect::<Vec<_>>(),
            [Some(0), None]
        );
    }

    #[test]
    fn tells_nothing_of_a_planned_sprint_but_its_total_and_has_no_days_without_both_dates() {
        let backlog = backlog();
        let sprint = backlog
            .sprints
            .create(NewSprint {
                start_on: Some(day(10)),
                ..NewSprint::in_project(backlog.project.id())
            })
            .unwrap();
        let planned = backlog.task("Draw the schematic");
        backlog.tasks.plan(planned.id(), Some(sprint.id())).unwrap();

        let report = backlog.sprints.report(sprint.id()).unwrap();

        assert_eq!(
            report,
            SprintReport {
                sprint_id: sprint.id(),
                unit: SprintUnit::Tasks,
                total: 1,
                days: Vec::new(),
            }
        );
    }

    #[test]
    fn stops_on_the_day_the_sprint_was_completed() {
        let backlog = backlog();
        let (sprint, planned) = backlog.active_sprint();
        backlog.finish(planned.id());
        backlog.complete_to_backlog(&sprint);
        backlog.pass(1);

        let report = backlog.sprints.report(sprint.id()).unwrap();

        assert_eq!(
            days(&report)[..3],
            [(day(10), Some(1)), (day(11), Some(0)), (day(12), None)]
        );
    }
}

mod removing_what_sprints_are_made_of {
    use super::*;

    #[test]
    fn removes_the_sprints_of_a_project_with_the_project() {
        let backlog = backlog();
        let sprint = backlog.new_sprint();

        backlog.projects.delete(backlog.project.id()).unwrap();

        let (code, _) = refusal(backlog.sprints.report(sprint.id()));
        assert_eq!(code, "not_found");
    }

    #[test]
    fn takes_a_deleted_task_out_of_its_sprint() {
        let backlog = backlog();
        let (sprint, planned) = backlog.active_sprint();
        let other = backlog.task("Route the board");
        backlog.tasks.plan(other.id(), Some(sprint.id())).unwrap();

        backlog.tasks.delete(planned.id()).unwrap();

        assert_eq!(backlog.sprints.report(sprint.id()).unwrap().total, 1);
    }
}
