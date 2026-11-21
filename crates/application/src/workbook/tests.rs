//! The workbook of a project in memory: what each of its four sheets holds,
//! cell by cell where it matters.

use std::sync::Arc;

use chrono::TimeZone;
use teamwright_domain::{
    ComputerId, Conversation, Effort, MemberKind, StatusCategory, StatusId, TokenCounts, Turn,
    TurnOrigin,
};

use super::*;
use crate::fields::FieldService;
use crate::in_memory::{
    InMemoryAreaRepository, InMemoryConversationRepository, InMemoryFieldRepository,
    InMemoryMemberRepository, InMemoryPlacesRepository, InMemoryProjectRepository,
    InMemorySprintRepository, InMemoryTagRepository, InMemoryTaskRepository,
    InMemoryViewRepository, ManualClock,
};
use crate::projects::{NewProject, ProjectService};
use crate::sprints::{NewSprint, SprintService, SprintStart};
use crate::tags::TagService;
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};

/// A project, the services that fill it, and the use case under test, on
/// shared storage. The clock shows 13 November 2026 at nine.
struct Fixture {
    tasks: TaskService,
    sprints: SprintService,
    team: TeamService,
    tags: TagService,
    fields: FieldService,
    conversations: Arc<InMemoryConversationRepository>,
    workbook: WorkbookService,
    project: Project,
}

fn moment() -> teamwright_domain::Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 13, 9, 0, 0).unwrap()
}

/// A day of November 2026.
fn day(day_of_month: u32) -> CalendarDate {
    CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
}

fn fixture() -> Fixture {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations = Arc::new(InMemoryConversationRepository::default());
    let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
    let areas_store = Arc::new(InMemoryAreaRepository::default());
    let views_store = Arc::new(InMemoryViewRepository::default());
    let tags_store = Arc::new(InMemoryTagRepository::default());
    let fields_store = Arc::new(InMemoryFieldRepository::default());
    let clock = Arc::new(ManualClock::starting_at(moment()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations.clone(),
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
        conversations.clone(),
        sprints_store.clone(),
        areas_store.clone(),
        clock.clone(),
    );
    let sprints = SprintService::new(
        projects_store.clone(),
        tasks_store.clone(),
        sprints_store.clone(),
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store.clone(),
        members_store.clone(),
        tasks_store.clone(),
        conversations.clone(),
        views_store.clone(),
        clock.clone(),
    );
    let tags = TagService::new(
        projects_store.clone(),
        tasks_store.clone(),
        views_store.clone(),
        tags_store.clone(),
        clock.clone(),
    );
    let fields = FieldService::new(
        projects_store.clone(),
        tasks_store.clone(),
        views_store,
        fields_store.clone(),
        clock.clone(),
    );
    let workbook = WorkbookService::new(WorkbookPorts {
        projects: projects_store,
        tasks: tasks_store,
        sprints: sprints_store,
        areas: areas_store,
        tags: tags_store,
        fields: fields_store.clone(),
        members: members_store,
        conversations: conversations.clone(),
        clock,
    });
    let project = projects
        .create(NewProject {
            name: "Smart home hub".to_owned(),
            key: Some("SH".to_owned()),
            description: "A hub that ties the lights together.".to_owned(),
            ..NewProject::default()
        })
        .unwrap();
    Fixture {
        tasks,
        sprints,
        team,
        tags,
        fields,
        conversations,
        workbook,
        project,
    }
}

impl Fixture {
    fn task(&self, title: &str) -> Task {
        self.tasks
            .create(NewTask::titled(self.project.id(), title))
            .unwrap()
    }

    fn status_of(&self, category: StatusCategory) -> StatusId {
        self.project.workflow().first_in(category).unwrap().id()
    }

    fn finish(&self, task: &Task) {
        self.tasks
            .move_to(task.id(), self.status_of(StatusCategory::Done), 0)
            .unwrap();
    }

    fn sheets(&self) -> Vec<Sheet> {
        self.workbook
            .workbook_of(self.project.id(), "Workshop laptop")
            .unwrap()
            .sheets
    }

    fn list(&self, name: &str) -> Table {
        match self.sheet(name) {
            SheetContent::List(table) => table,
            SheetContent::Overview { .. } => panic!("{name} is not a list"),
        }
    }

    fn sheet(&self, name: &str) -> SheetContent {
        self.sheets()
            .into_iter()
            .find(|sheet| sheet.name == name)
            .unwrap()
            .content
    }

    /// The sections of the summary.
    fn summary(&self) -> (String, Vec<Section>) {
        match self.sheet("Summary") {
            SheetContent::Overview { title, sections } => (title, sections),
            SheetContent::List(_) => panic!("the summary is not an overview"),
        }
    }

    /// The value of a fact of a section of the summary.
    fn fact(&self, heading: &str, label: &str) -> Option<Cell> {
        let (_, sections) = self.summary();
        let section = sections
            .into_iter()
            .find(|section| section.heading == heading)?;
        section
            .table
            .rows
            .into_iter()
            .find(|row| row.first() == Some(&Cell::Text(label.to_owned())))
            .and_then(|row| row.get(1).cloned())
    }
}

/// The cell of a column of a row, by the name of the column.
fn cell(table: &Table, row: usize, column: &str) -> Cell {
    let index = table
        .columns
        .iter()
        .position(|candidate| candidate.name == column)
        .unwrap();
    table.rows[row][index].clone()
}

fn text(value: &str) -> Cell {
    Cell::Text(value.to_owned())
}

#[test]
fn the_workbook_has_four_sheets_in_order() {
    let fixture = fixture();

    let names: Vec<String> = fixture
        .sheets()
        .into_iter()
        .map(|sheet| sheet.name)
        .collect();

    assert_eq!(names, ["Summary", "Tasks", "Sprints", "Team"]);
}

#[test]
fn every_row_has_a_cell_for_every_column() {
    let fixture = fixture();
    let parent = fixture.task("Choose the radio");
    fixture
        .tasks
        .create(NewTask {
            parent_id: Some(parent.id()),
            ..NewTask::titled(fixture.project.id(), "Compare the modules")
        })
        .unwrap();
    fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();

    for sheet in fixture.sheets() {
        let tables = match sheet.content {
            SheetContent::List(table) => vec![table],
            SheetContent::Overview { sections, .. } => {
                sections.into_iter().map(|section| section.table).collect()
            }
        };
        for table in tables {
            for row in &table.rows {
                assert_eq!(row.len(), table.columns.len(), "a row of {}", sheet.name);
            }
        }
    }
}

#[test]
fn the_summary_names_the_project_and_where_it_was_exported() {
    let fixture = fixture();

    let (title, _) = fixture.summary();

    assert_eq!(title, "Smart home hub");
    assert_eq!(fixture.fact("Project", "Key"), Some(text("SH")));
    assert_eq!(
        fixture.fact("Project", "Description"),
        Some(text("A hub that ties the lights together."))
    );
    assert_eq!(
        fixture.fact("Project", "Exported"),
        Some(Cell::Date(day(13)))
    );
    assert_eq!(
        fixture.fact("Project", "On the computer"),
        Some(text("Workshop laptop"))
    );
}

#[test]
fn the_summary_counts_finished_tasks_by_the_category_of_their_status() {
    let fixture = fixture();
    let first = fixture.task("Draw the schematic");
    fixture.task("Order the parts");
    fixture.task("Write the firmware");
    fixture.finish(&first);

    assert_eq!(fixture.fact("Tasks", "Open"), Some(Cell::Whole(2)));
    assert_eq!(fixture.fact("Tasks", "Finished"), Some(Cell::Whole(1)));
    assert_eq!(fixture.fact("Tasks", "All"), Some(Cell::Whole(3)));
}

#[test]
fn the_summary_counts_the_tasks_of_every_status_in_the_order_of_the_workflow() {
    let fixture = fixture();
    let first = fixture.task("Draw the schematic");
    fixture.task("Order the parts");
    fixture.finish(&first);

    let (_, sections) = fixture.summary();
    let by_status = &sections
        .iter()
        .find(|section| section.heading == "Tasks by status")
        .unwrap()
        .table;

    let statuses = fixture.project.workflow().statuses();
    assert_eq!(by_status.rows.len(), statuses.len());
    for (row, status) in by_status.rows.iter().zip(statuses) {
        let expected = match status.category() {
            StatusCategory::Todo | StatusCategory::Done => 1,
            StatusCategory::Active | StatusCategory::Review => 0,
        };
        assert_eq!(
            row,
            &vec![
                Cell::Status {
                    name: status.name().to_string(),
                    color: status.color(),
                },
                Cell::Whole(expected),
            ]
        );
    }
}

#[test]
fn a_project_without_sprints_has_no_sprint_in_its_summary() {
    let fixture = fixture();

    let (_, sections) = fixture.summary();

    assert!(sections.iter().all(|section| section.heading != "Sprint"));
}

#[test]
fn a_project_with_no_running_sprint_says_so() {
    let fixture = fixture();
    fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();

    assert_eq!(
        fixture.fact("Sprint", "Running"),
        Some(text("No sprint is running"))
    );
}

#[test]
fn the_summary_tells_the_running_sprint_with_its_days_left_and_its_points() {
    let fixture = fixture();
    let sprint = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    let first = fixture.task("Draw the schematic");
    let second = fixture.task("Order the parts");
    for (task, points) in [(&first, 3.0), (&second, 5.0)] {
        fixture.tasks.plan(task.id(), Some(sprint.id())).unwrap();
        fixture.tasks.set_points(task.id(), Some(points)).unwrap();
    }
    fixture.finish(&first);
    fixture
        .sprints
        .start(
            sprint.id(),
            SprintStart {
                name: "Sprint 1".to_owned(),
                goal: String::new(),
                start_on: day(10),
                end_on: day(23),
            },
        )
        .unwrap();

    assert_eq!(fixture.fact("Sprint", "Running"), Some(text("Sprint 1")));
    assert_eq!(fixture.fact("Sprint", "Start"), Some(Cell::Date(day(10))));
    assert_eq!(fixture.fact("Sprint", "End"), Some(Cell::Date(day(23))));
    assert_eq!(fixture.fact("Sprint", "Days left"), Some(Cell::Whole(10)));
    assert_eq!(fixture.fact("Sprint", "Points done"), Some(Cell::Whole(3)));
    assert_eq!(
        fixture.fact("Sprint", "Points planned"),
        Some(Cell::Whole(8))
    );
}

#[test]
fn a_sprint_without_points_counts_its_tasks() {
    let fixture = fixture();
    let sprint = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    let task = fixture.task("Draw the schematic");
    fixture.tasks.plan(task.id(), Some(sprint.id())).unwrap();
    fixture
        .sprints
        .start(
            sprint.id(),
            SprintStart {
                name: "Sprint 1".to_owned(),
                goal: String::new(),
                start_on: day(10),
                end_on: day(12),
            },
        )
        .unwrap();

    // The sprint ended yesterday.
    assert_eq!(fixture.fact("Sprint", "Days left"), Some(Cell::Whole(-1)));
    assert_eq!(fixture.fact("Sprint", "Tasks done"), Some(Cell::Whole(0)));
    assert_eq!(
        fixture.fact("Sprint", "Tasks planned"),
        Some(Cell::Whole(1))
    );
}

#[test]
fn a_task_row_holds_every_fact_of_the_task_with_its_kind() {
    let fixture = fixture();
    let member = fixture
        .team
        .add(NewMember {
            project_id: fixture.project.id(),
            name: "Ada".to_owned(),
            kind: MemberKind::Agent,
            responsibility: "Draws the boards".to_owned(),
            model: None,
            effort: None,
        })
        .unwrap();
    let sprint = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    let blocker = fixture.task("Choose the radio");
    let task = fixture
        .tasks
        .create(NewTask {
            priority: Priority::High,
            assignee: Some(Assignee::Member(member.id())),
            estimate_minutes: Some(90),
            start_on: Some(day(16)),
            due_on: Some(day(20)),
            ..NewTask::titled(fixture.project.id(), "Draw the schematic")
        })
        .unwrap();
    fixture.tasks.plan(task.id(), Some(sprint.id())).unwrap();
    fixture.tasks.set_points(task.id(), Some(5.0)).unwrap();
    fixture
        .tasks
        .add_dependency(task.id(), blocker.id())
        .unwrap();

    let tasks = fixture.list("Tasks");

    assert_eq!(cell(&tasks, 1, "Task"), text("SH-2"));
    assert_eq!(
        cell(&tasks, 1, "Title"),
        Cell::Indented {
            text: "Draw the schematic".to_owned(),
            level: 0
        }
    );
    let first_status = fixture.project.workflow().initial_status();
    assert_eq!(
        cell(&tasks, 1, "Status"),
        Cell::Status {
            name: first_status.name().to_string(),
            color: first_status.color(),
        }
    );
    assert_eq!(cell(&tasks, 1, "Assignee"), text("Ada"));
    assert_eq!(cell(&tasks, 1, "Priority"), text("High"));
    assert_eq!(cell(&tasks, 1, "Points"), Cell::Whole(5));
    assert_eq!(cell(&tasks, 1, "Estimate (hours)"), Cell::Hours(1.5));
    assert_eq!(cell(&tasks, 1, "Start"), Cell::Date(day(16)));
    assert_eq!(cell(&tasks, 1, "Due"), Cell::Date(day(20)));
    assert_eq!(cell(&tasks, 1, "Sprint"), text("Sprint 1"));
    assert_eq!(cell(&tasks, 1, "Area"), text("General"));
    assert_eq!(cell(&tasks, 1, "Waits for"), text("SH-1"));
    assert_eq!(cell(&tasks, 1, "Part of"), Cell::Empty);
    assert_eq!(cell(&tasks, 1, "Created"), Cell::Date(day(13)));
    assert_eq!(cell(&tasks, 1, "Finished"), Cell::Empty);
}

#[test]
fn a_task_of_nobody_has_an_empty_assignee_and_one_of_the_manager_says_you() {
    let fixture = fixture();
    let mine = fixture.task("Write the brief");
    fixture.task("Order the parts");
    fixture
        .tasks
        .assign(mine.id(), Some(Assignee::Manager))
        .unwrap();

    let tasks = fixture.list("Tasks");

    assert_eq!(cell(&tasks, 0, "Assignee"), text("You"));
    assert_eq!(cell(&tasks, 1, "Assignee"), Cell::Empty);
}

#[test]
fn the_tags_of_a_task_are_named_by_name_and_a_task_without_any_has_none() {
    let fixture = fixture();
    let tagged = fixture.task("Shield the radio");
    fixture.task("Write the brief");
    for name in ["Power", "emc"] {
        fixture.tags.put_on_by_name(tagged.id(), name).unwrap();
    }

    let tasks = fixture.list("Tasks");

    assert_eq!(cell(&tasks, 0, "Tags"), text("emc, Power"));
    assert_eq!(cell(&tasks, 1, "Tags"), Cell::Empty);
}

#[test]
fn every_field_is_a_column_at_the_end_with_real_numbers_and_dates() {
    let fixture = fixture();
    let project = fixture.project.id();
    let held = fixture.task("Order the sensors");
    fixture.task("Write the brief");
    let fields = &fixture.fields;
    let supplier = fields
        .add(project, "Supplier", FieldKind::Choice, None)
        .unwrap();
    let supplier = fields.add_choice(supplier.id(), "Mouser", None).unwrap();
    let cost = fields
        .add(project, "Unit cost", FieldKind::Number, Some("€"))
        .unwrap();
    let arrives = fields
        .add(project, "Arrives", FieldKind::Date, None)
        .unwrap();
    let ordered = fields
        .add(project, "Ordered", FieldKind::YesNo, None)
        .unwrap();
    let datasheet = fields
        .add(project, "Datasheet", FieldKind::Link, None)
        .unwrap();
    let values = [
        (
            supplier.id(),
            FieldValue::Choice(supplier.choices()[0].id()),
        ),
        (
            cost.id(),
            FieldValue::Number(teamwright_domain::FieldNumber::new(12.5).unwrap()),
        ),
        (arrives.id(), FieldValue::Date(day(30))),
        (ordered.id(), FieldValue::YesNo(true)),
        (
            datasheet.id(),
            FieldValue::link("https://www.ti.com/lit/ds/symlink/tps62840.pdf").unwrap(),
        ),
    ];
    for (field, value) in values {
        fields.set_value(held.id(), field, value).unwrap();
    }

    let tasks = fixture.list("Tasks");

    let names: Vec<&str> = tasks
        .columns
        .iter()
        .rev()
        .take(6)
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Datasheet",
            "Ordered",
            "Arrives",
            "Unit cost (€)",
            "Supplier",
            "Finished"
        ]
    );
    assert_eq!(cell(&tasks, 0, "Supplier"), text("Mouser"));
    assert_eq!(cell(&tasks, 0, "Unit cost (€)"), Cell::Number(12.5));
    assert_eq!(cell(&tasks, 0, "Arrives"), Cell::Date(day(30)));
    assert_eq!(cell(&tasks, 0, "Ordered"), text("Yes"));
    assert_eq!(
        cell(&tasks, 0, "Datasheet"),
        text("https://www.ti.com/lit/ds/symlink/tps62840.pdf")
    );
    assert_eq!(cell(&tasks, 1, "Unit cost (€)"), Cell::Empty);
    assert!(
        tasks
            .rows
            .iter()
            .all(|row| row.len() == tasks.columns.len())
    );
}

#[test]
fn a_finished_task_has_the_day_it_was_finished() {
    let fixture = fixture();
    let task = fixture.task("Draw the schematic");
    fixture.finish(&task);

    assert_eq!(
        cell(&fixture.list("Tasks"), 0, "Finished"),
        Cell::Date(day(13))
    );
}

#[test]
fn subtasks_come_under_their_task_set_in_and_naming_it() {
    let fixture = fixture();
    let first = fixture.task("Choose the radio");
    let second = fixture.task("Write the firmware");
    let subtask = |title: &str, parent: &Task| {
        fixture
            .tasks
            .create(NewTask {
                parent_id: Some(parent.id()),
                ..NewTask::titled(fixture.project.id(), title)
            })
            .unwrap()
    };
    let compare = subtask("Compare the modules", &first);
    subtask("Read the data sheets", &compare);
    subtask("Order samples", &first);
    let _ = second;

    let tasks = fixture.list("Tasks");
    let outline: Vec<(Cell, Cell, Cell)> = (0..tasks.rows.len())
        .map(|row| {
            (
                cell(&tasks, row, "Task"),
                cell(&tasks, row, "Title"),
                cell(&tasks, row, "Part of"),
            )
        })
        .collect();

    let indented = |title: &str, level| Cell::Indented {
        text: title.to_owned(),
        level,
    };
    assert_eq!(
        outline,
        [
            (text("SH-1"), indented("Choose the radio", 0), Cell::Empty),
            (
                text("SH-3"),
                indented("Compare the modules", 1),
                text("SH-1")
            ),
            (
                text("SH-4"),
                indented("Read the data sheets", 2),
                text("SH-3")
            ),
            (text("SH-5"), indented("Order samples", 1), text("SH-1")),
            (text("SH-2"), indented("Write the firmware", 0), Cell::Empty),
        ]
    );
}

#[test]
fn the_sprints_are_listed_with_their_work() {
    let fixture = fixture();
    let running = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    let planned = fixture
        .sprints
        .create(NewSprint {
            goal: "Ship the first boards".to_owned(),
            ..NewSprint::in_project(fixture.project.id())
        })
        .unwrap();
    let first = fixture.task("Draw the schematic");
    let second = fixture.task("Order the parts");
    for task in [&first, &second] {
        fixture.tasks.plan(task.id(), Some(running.id())).unwrap();
    }
    fixture.tasks.set_points(first.id(), Some(3.0)).unwrap();
    fixture.finish(&first);
    fixture
        .sprints
        .start(
            running.id(),
            SprintStart {
                name: "Sprint 1".to_owned(),
                goal: String::new(),
                start_on: day(10),
                end_on: day(23),
            },
        )
        .unwrap();

    let sprints = fixture.list("Sprints");

    assert_eq!(
        sprints.rows[0],
        vec![
            text("Sprint 1"),
            text("Running"),
            Cell::Date(day(10)),
            Cell::Date(day(23)),
            Cell::Empty,
            Cell::Whole(2),
            Cell::Whole(3),
            Cell::Whole(3),
        ]
    );
    assert_eq!(
        sprints.rows[1],
        vec![
            text(planned.name().as_str()),
            text("Planned"),
            Cell::Empty,
            Cell::Empty,
            text("Ship the first boards"),
            Cell::Whole(0),
            Cell::Empty,
            Cell::Empty,
        ]
    );
}

#[test]
fn a_completed_sprint_keeps_the_work_it_was_completed_with() {
    let fixture = fixture();
    let sprint = fixture
        .sprints
        .create(NewSprint::in_project(fixture.project.id()))
        .unwrap();
    let first = fixture.task("Draw the schematic");
    let second = fixture.task("Order the parts");
    for (task, points) in [(&first, 3.0), (&second, 5.0)] {
        fixture.tasks.plan(task.id(), Some(sprint.id())).unwrap();
        fixture.tasks.set_points(task.id(), Some(points)).unwrap();
    }
    fixture.finish(&first);
    fixture
        .sprints
        .start(
            sprint.id(),
            SprintStart {
                name: "Sprint 1".to_owned(),
                goal: String::new(),
                start_on: day(10),
                end_on: day(23),
            },
        )
        .unwrap();
    fixture.sprints.complete(sprint.id(), None).unwrap();

    let sprints = fixture.list("Sprints");

    assert_eq!(cell(&sprints, 0, "State"), text("Completed"));
    // The unfinished task went back to the backlog; the record stays.
    assert_eq!(cell(&sprints, 0, "Tasks"), Cell::Whole(1));
    assert_eq!(cell(&sprints, 0, "Points"), Cell::Whole(8));
    assert_eq!(cell(&sprints, 0, "Points done"), Cell::Whole(3));
}

#[test]
fn the_team_lists_the_manager_the_members_and_the_advisor_with_their_tokens() {
    let fixture = fixture();
    let ada = fixture
        .team
        .add(NewMember {
            project_id: fixture.project.id(),
            name: "Ada".to_owned(),
            kind: MemberKind::Agent,
            responsibility: "Draws the boards".to_owned(),
            model: Some("opus".to_owned()),
            effort: Some(Effort::ExtraHigh),
        })
        .unwrap();
    fixture
        .team
        .add(NewMember {
            project_id: fixture.project.id(),
            name: "Ben".to_owned(),
            kind: MemberKind::Human,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    let drawn = fixture.task("Draw the schematic");
    fixture
        .tasks
        .assign(drawn.id(), Some(Assignee::Member(ada.id())))
        .unwrap();
    fixture.finish(&drawn);
    let open = fixture.task("Route the board");
    fixture
        .tasks
        .assign(open.id(), Some(Assignee::Member(ada.id())))
        .unwrap();
    let mine = fixture.task("Write the brief");
    fixture
        .tasks
        .assign(mine.id(), Some(Assignee::Manager))
        .unwrap();

    let here = ComputerId::generate();
    let tokens = TokenCounts {
        input: 100,
        output: 20,
        cache_read: 3,
        cache_written: 4,
    };
    let with_ada = Conversation::start(fixture.project.id(), ada.id(), None, moment(), here);
    let with_advisor =
        Conversation::start_with_advisor(fixture.project.id(), None, None, moment(), here);
    for (conversation, turns) in [(&with_ada, 2), (&with_advisor, 1)] {
        fixture
            .conversations
            .save_conversation(conversation)
            .unwrap();
        for _ in 0..turns {
            let mut turn =
                Turn::start(conversation.id(), "Begin", TurnOrigin::Message, moment()).unwrap();
            turn.finish("Done", tokens, moment()).unwrap();
            fixture.conversations.save_turn(&turn).unwrap();
        }
    }

    let team = fixture.list("Team");

    assert_eq!(
        team.rows,
        vec![
            vec![
                text("You"),
                text("Manager"),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Whole(1),
                Cell::Whole(0),
                Cell::Empty,
            ],
            vec![
                text("Ada"),
                text("AI specialist"),
                text("Draws the boards"),
                text("opus"),
                text("Extra high"),
                Cell::Whole(1),
                Cell::Whole(1),
                Cell::Whole(254),
            ],
            vec![
                text("Ben"),
                text("Person"),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Whole(0),
                Cell::Whole(0),
                Cell::Empty,
            ],
            vec![
                text("Advisor"),
                text("Advisor"),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Whole(127),
            ],
        ]
    );
}

#[test]
fn a_project_that_does_not_exist_has_no_workbook() {
    let fixture = fixture();

    let result = fixture
        .workbook
        .workbook_of(ProjectId::generate(), "Laptop");

    assert!(matches!(
        result,
        Err(ApplicationError::NotFound {
            kind: "project",
            ..
        })
    ));
}
