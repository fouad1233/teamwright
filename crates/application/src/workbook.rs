//! The project as a workbook: what the manager gets when they export a
//! project to Excel
//! ([decision 0025](../../../docs/decisions/0025-the-project-as-a-workbook.md)).
//!
//! The use case gathers what the workbook holds and returns it as plain
//! data: sheets, tables, rows, and cells that know their kind. A date is a
//! date and a number is a number, so that whoever writes the file can give
//! every cell its real type. Writing the file is input and output, and
//! happens outside this crate.
//!
//! Every figure follows the category of a status, never its name, and uses
//! the rules the rest of the program uses: a task is finished when it is in
//! a status of the category done, and a sprint counts its work as the
//! report of its burndown does.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use teamwright_domain::{
    Assignee, CalendarDate, Color, Effort, Field, FieldKind, FieldValue, Member, MemberId,
    MemberKind, Priority, Project, ProjectId, Sprint, SprintState, SprintSummary, SprintUnit, Task,
    TaskId,
};

use crate::error::ApplicationError;
use crate::fields::fields_of;
use crate::ports::{
    AreaRepository, Clock, ConversationRepository, FieldRepository, MemberRepository,
    ProjectRepository, SprintRepository, TagRepository, TaskRepository,
};
use crate::sprints::{as_listed, counted, is_finished};
use crate::tags::tags_of;

/// A workbook: its sheets, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    /// The sheets, the first one first.
    pub sheets: Vec<Sheet>,
}

/// One sheet of a workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    /// The name on its tab. At most 31 characters, as spreadsheets allow.
    pub name: String,
    /// What it holds.
    pub content: SheetContent,
}

/// What a sheet holds.
#[derive(Debug, Clone, PartialEq)]
pub enum SheetContent {
    /// A page to read from top to bottom: a title, then sections, each with
    /// a heading and a small table.
    Overview {
        /// The title, set large at the top.
        title: String,
        /// The sections under it.
        sections: Vec<Section>,
    },
    /// One list: a header row and a row for each thing listed. It is meant
    /// to be sorted and filtered.
    List(Table),
}

/// A part of an overview, under its heading.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The heading.
    pub heading: String,
    /// What the section says.
    pub table: Table,
}

/// Columns and rows of cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// The columns, the first one first.
    pub columns: Vec<Column>,
    /// The rows. Every row has as many cells as there are columns.
    pub rows: Vec<Vec<Cell>>,
}

impl Table {
    /// Whether the table has a header row: whether any column has a name.
    /// A table of facts, a label and a value on every row, has none.
    #[must_use]
    pub fn has_header(&self) -> bool {
        self.columns.iter().any(|column| !column.name.is_empty())
    }
}

/// A column of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// The name in the header row. Empty in a table of facts.
    pub name: String,
    /// Whether long text wraps onto more lines in the column, instead of
    /// making it wider.
    pub wraps: bool,
}

impl Column {
    /// A column whose text stays on one line.
    #[must_use]
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            wraps: false,
        }
    }

    /// A column for long text, which wraps.
    #[must_use]
    pub fn wrapping(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            wraps: true,
        }
    }
}

/// One cell, with its kind.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Nothing.
    Empty,
    /// Text.
    Text(String),
    /// Text set in from the left, as a subtask under its task. Level 0 is
    /// not set in.
    Indented {
        /// The text.
        text: String,
        /// How far it is set in.
        level: u8,
    },
    /// A whole number, which may be below zero, such as the days a sprint
    /// is past its end.
    Whole(i64),
    /// A number of hours, shown with one decimal.
    Hours(f64),
    /// A number with the decimals it has, such as the value of a number
    /// field.
    Number(f64),
    /// A day on the calendar.
    Date(CalendarDate),
    /// The status of a task: its name, and the color the project gives it.
    Status {
        /// The name of the status.
        name: String,
        /// Its color.
        color: Color,
    },
}

impl Cell {
    /// A text cell, or an empty one for empty text.
    fn text(text: &str) -> Self {
        if text.trim().is_empty() {
            Self::Empty
        } else {
            Self::Text(text.trim().to_owned())
        }
    }

    /// A whole number from a count.
    fn count(count: usize) -> Self {
        Self::Whole(i64::try_from(count).unwrap_or(i64::MAX))
    }

    /// A date cell, or an empty one without a date.
    fn date(date: Option<CalendarDate>) -> Self {
        date.map_or(Self::Empty, Self::Date)
    }
}

/// What the workbook use case needs from the outside world.
pub struct WorkbookPorts {
    /// Where projects are stored.
    pub projects: Arc<dyn ProjectRepository>,
    /// Where tasks are stored.
    pub tasks: Arc<dyn TaskRepository>,
    /// Where sprints are stored.
    pub sprints: Arc<dyn SprintRepository>,
    /// Where the areas of the projects are stored.
    pub areas: Arc<dyn AreaRepository>,
    /// Where the tags of the projects are stored.
    pub tags: Arc<dyn TagRepository>,
    /// Where the custom fields of the projects are stored.
    pub fields: Arc<dyn FieldRepository>,
    /// Where the members of the teams are stored.
    pub members: Arc<dyn MemberRepository>,
    /// Where conversations and their turns are stored, for the tokens used.
    pub conversations: Arc<dyn ConversationRepository>,
    /// The time, for the day of the export and the days a sprint has left.
    pub clock: Arc<dyn Clock>,
}

/// The use case that puts a project into a workbook.
#[derive(Clone)]
pub struct WorkbookService {
    ports: Arc<WorkbookPorts>,
}

impl WorkbookService {
    /// Creates the service.
    #[must_use]
    pub fn new(ports: WorkbookPorts) -> Self {
        Self {
            ports: Arc::new(ports),
        }
    }

    /// Gathers a project into four sheets: a summary, the tasks, the
    /// sprints and the team. `computer` is the name of the computer the
    /// export is made on, which the summary names.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn workbook_of(
        &self,
        project_id: ProjectId,
        computer: &str,
    ) -> Result<Workbook, ApplicationError> {
        let ports = &self.ports;
        let project =
            ports
                .projects
                .find(project_id)?
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "project",
                    id: project_id.to_string(),
                })?;
        let mut tasks = ports.tasks.list_for_project(project_id)?;
        tasks.sort_by_key(Task::number);
        let mut sprints = ports.sprints.list_for_project(project_id)?;
        sprints.sort_by(as_listed);
        let mut members = ports.members.list_for_project(project_id)?;
        members.sort_by_key(|member| (member.joined_at(), member.id()));
        let area_names: HashMap<_, _> = ports
            .areas
            .list_for_project(project_id)?
            .into_iter()
            .map(|area| (area.id(), area.name().to_string()))
            .collect();
        let tag_names: HashMap<_, _> = tags_of(ports.tags.as_ref(), project_id)?
            .into_iter()
            .map(|tag| (tag.id(), tag.name().to_string()))
            .collect();
        let fields = fields_of(ports.fields.as_ref(), project_id)?;
        let tokens = self.tokens_by_user(project_id)?;
        let today = ports.clock.day_of(ports.clock.now());

        let gathered = Gathered {
            project: &project,
            tasks: &tasks,
            sprints: &sprints,
            members: &members,
            area_names: &area_names,
            tag_names: &tag_names,
            fields: &fields,
            today,
            clock: ports.clock.as_ref(),
        };
        Ok(Workbook {
            sheets: vec![
                Sheet {
                    name: "Summary".to_owned(),
                    content: gathered.summary(computer),
                },
                Sheet {
                    name: "Tasks".to_owned(),
                    content: SheetContent::List(gathered.task_table()),
                },
                Sheet {
                    name: "Sprints".to_owned(),
                    content: SheetContent::List(gathered.sprint_table()),
                },
                Sheet {
                    name: "Team".to_owned(),
                    content: SheetContent::List(gathered.team_table(&tokens)),
                },
            ],
        })
    }

    /// The tokens every specialist of a project used, and the Advisor's.
    /// Conversations whose specialist has left are not counted for anyone.
    fn tokens_by_user(&self, project_id: ProjectId) -> Result<TokensUsed, ApplicationError> {
        let mut used = TokensUsed::default();
        for conversation in self.ports.conversations.list_for_project(project_id)? {
            let tokens: u64 = self
                .ports
                .conversations
                .turns_of(conversation.id())?
                .iter()
                .filter_map(teamwright_domain::Turn::tokens)
                .map(|counts| counts.all())
                .fold(0, u64::saturating_add);
            if conversation.is_with_advisor() {
                used.has_advisor = true;
                used.advisor = used.advisor.saturating_add(tokens);
            } else if let Some(member_id) = conversation.member_id() {
                let sum = used.by_member.entry(member_id).or_default();
                *sum = sum.saturating_add(tokens);
            }
        }
        Ok(used)
    }
}

/// The tokens used in a project, by who used them.
#[derive(Debug, Default)]
struct TokensUsed {
    by_member: BTreeMap<MemberId, u64>,
    advisor: u64,
    has_advisor: bool,
}

/// Everything the sheets are made from, loaded once.
struct Gathered<'a> {
    project: &'a Project,
    /// The tasks, by number.
    tasks: &'a [Task],
    /// The sprints, in the order they are listed.
    sprints: &'a [Sprint],
    /// The members, in the order they joined.
    members: &'a [Member],
    area_names: &'a HashMap<teamwright_domain::AreaId, String>,
    /// The names of the tags of the project.
    tag_names: &'a HashMap<teamwright_domain::TagId, String>,
    /// The custom fields of the project, in their order.
    fields: &'a [Field],
    today: CalendarDate,
    /// The clock, which tells on which day a moment falls where the program runs.
    clock: &'a dyn Clock,
}

impl Gathered<'_> {
    /// The first sheet: the project, its tasks in numbers, its tasks by
    /// status, and the running sprint.
    fn summary(&self, computer: &str) -> SheetContent {
        let project = self.project;
        let mut about = vec![fact("Key", Cell::text(project.key().as_str()))];
        if !project.description().trim().is_empty() {
            about.push(fact("Description", Cell::text(project.description())));
        }
        about.push(fact("Exported", Cell::Date(self.today)));
        about.push(fact("On the computer", Cell::text(computer)));

        let finished = self
            .tasks
            .iter()
            .filter(|task| is_finished(project, task))
            .count();
        let mut sections = vec![
            Section {
                heading: "Project".to_owned(),
                table: facts(about),
            },
            Section {
                heading: "Tasks".to_owned(),
                table: facts(vec![
                    fact("Open", Cell::count(self.tasks.len() - finished)),
                    fact("Finished", Cell::count(finished)),
                    fact("All", Cell::count(self.tasks.len())),
                ]),
            },
            Section {
                heading: "Tasks by status".to_owned(),
                table: Table {
                    columns: vec![Column::named("Status"), Column::named("Tasks")],
                    rows: project
                        .workflow()
                        .statuses()
                        .iter()
                        .map(|status| {
                            vec![
                                Cell::Status {
                                    name: status.name().to_string(),
                                    color: status.color(),
                                },
                                Cell::count(
                                    self.tasks
                                        .iter()
                                        .filter(|task| task.status_id() == status.id())
                                        .count(),
                                ),
                            ]
                        })
                        .collect(),
                },
            },
        ];
        if let Some(section) = self.running_sprint() {
            sections.push(section);
        }
        SheetContent::Overview {
            title: project.name().to_string(),
            sections,
        }
    }

    /// The running sprint, with its dates, the days it has left and its
    /// work. A project that uses no sprints has no such section; one that
    /// uses them with none running says so.
    fn running_sprint(&self) -> Option<Section> {
        if self.sprints.is_empty() {
            return None;
        }
        let Some(sprint) = self.sprints.iter().find(|sprint| sprint.is_active()) else {
            return Some(Section {
                heading: "Sprint".to_owned(),
                table: facts(vec![fact("Running", Cell::text("No sprint is running"))]),
            });
        };
        let dates = sprint.dates();
        let mut rows = vec![
            fact("Running", Cell::text(sprint.name().as_str())),
            fact("Start", Cell::date(dates.start_on())),
            fact("End", Cell::date(dates.end_on())),
        ];
        if let Some(end_on) = dates.end_on() {
            rows.push(fact(
                "Days left",
                Cell::Whole((end_on - self.today).num_days()),
            ));
        }
        let work = self.work_of(sprint);
        let (done, planned) = match work.unit {
            SprintUnit::Points => ("Points done", "Points planned"),
            SprintUnit::Tasks => ("Tasks done", "Tasks planned"),
        };
        rows.push(fact(done, Cell::Whole(work.completed.into())));
        rows.push(fact(planned, Cell::Whole(work.committed.into())));
        Some(Section {
            heading: "Sprint".to_owned(),
            table: facts(rows),
        })
    }

    /// The main tasks of a sprint: a subtask is part of its task, as the
    /// sprint counts it.
    fn main_tasks_of(&self, sprint: &Sprint) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| task.sprint_id() == Some(sprint.id()) && task.parent_id().is_none())
            .collect()
    }

    /// What a sprint took on and finished. A completed sprint keeps the
    /// summary it was completed with, since its unfinished tasks have moved
    /// on; any other is counted from the tasks in it now.
    fn work_of(&self, sprint: &Sprint) -> SprintSummary {
        sprint.summary().unwrap_or_else(|| {
            SprintSummary::of(
                &self
                    .main_tasks_of(sprint)
                    .iter()
                    .map(|task| counted(self.project, task))
                    .collect::<Vec<_>>(),
            )
        })
    }

    /// One row a task, every subtask under its task with its title set in.
    fn task_table(&self) -> Table {
        let key = self.project.key().as_str();
        let label = |id: TaskId| {
            self.tasks
                .iter()
                .find(|task| task.id() == id)
                .map(|task| format!("{key}-{}", task.number()))
        };
        let sprint_names: HashMap<_, _> = self
            .sprints
            .iter()
            .map(|sprint| (sprint.id(), sprint.name().to_string()))
            .collect();

        let rows = self
            .in_outline_order()
            .into_iter()
            .map(|(task, level)| {
                let status = self.project.workflow().status(task.status_id());
                let mut waits_for: Vec<&Task> = self
                    .tasks
                    .iter()
                    .filter(|other| task.blocked_by().contains(&other.id()))
                    .collect();
                waits_for.sort_by_key(|other| other.number());
                vec![
                    Cell::Text(format!("{key}-{}", task.number())),
                    Cell::Indented {
                        text: task.title().to_string(),
                        level,
                    },
                    status.map_or(Cell::Empty, |status| Cell::Status {
                        name: status.name().to_string(),
                        color: status.color(),
                    }),
                    Cell::text(&self.assignee_name(task.assignee())),
                    Cell::text(priority_in_words(task.priority())),
                    task.points()
                        .map_or(Cell::Empty, |points| Cell::Whole(points.value().into())),
                    task.estimate_minutes().map_or(Cell::Empty, |minutes| {
                        Cell::Hours(f64::from(minutes) / 60.0)
                    }),
                    Cell::date(task.schedule().start_on()),
                    Cell::date(task.schedule().due_on()),
                    task.sprint_id()
                        .and_then(|id| sprint_names.get(&id))
                        .map_or(Cell::Empty, |name| Cell::text(name)),
                    self.area_names
                        .get(&task.area_id())
                        .map_or(Cell::Empty, |name| Cell::text(name)),
                    Cell::text(&self.tags_in_words(task)),
                    Cell::text(
                        &waits_for
                            .iter()
                            .map(|other| format!("{key}-{}", other.number()))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    task.parent_id()
                        .and_then(label)
                        .map_or(Cell::Empty, Cell::Text),
                    Cell::Date(self.clock.day_of(task.created_at())),
                    Cell::date(task.completed_at().map(|moment| self.clock.day_of(moment))),
                ]
                .into_iter()
                .chain(self.fields.iter().map(|field| field_cell(field, task)))
                .collect()
            })
            .collect();

        Table {
            // The custom fields come last, one column each, so that the
            // columns every project has keep their places.
            columns: [
                Column::named("Task"),
                Column::wrapping("Title"),
                Column::named("Status"),
                Column::named("Assignee"),
                Column::named("Priority"),
                Column::named("Points"),
                Column::named("Estimate (hours)"),
                Column::named("Start"),
                Column::named("Due"),
                Column::named("Sprint"),
                Column::named("Area"),
                Column::wrapping("Tags"),
                Column::named("Waits for"),
                Column::named("Part of"),
                Column::named("Created"),
                Column::named("Finished"),
            ]
            .into_iter()
            .chain(self.fields.iter().map(field_column))
            .collect(),
            rows,
        }
    }

    /// The names of the tags a task carries, by name, whatever the
    /// capitals: "EMC, Power". A tag that is gone is left out.
    fn tags_in_words(&self, task: &Task) -> String {
        let mut names: Vec<&String> = task
            .tags()
            .iter()
            .filter_map(|tag| self.tag_names.get(tag))
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names
            .into_iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The tasks in the order of an outline: the tasks that are part of no
    /// other by number, each followed by its subtasks, and theirs, with how
    /// deep each one lies. A task whose parent is gone counts as a main
    /// task.
    fn in_outline_order(&self) -> Vec<(&Task, u8)> {
        let known: std::collections::HashSet<TaskId> = self.tasks.iter().map(Task::id).collect();
        let is_main = |task: &&Task| {
            task.parent_id()
                .is_none_or(|parent| !known.contains(&parent))
        };
        let mut ordered = Vec::with_capacity(self.tasks.len());
        let mut waiting: Vec<(&Task, u8)> = self
            .tasks
            .iter()
            .filter(is_main)
            .rev()
            .map(|task| (task, 0))
            .collect();
        while let Some((task, level)) = waiting.pop() {
            ordered.push((task, level));
            // The children go on the stack last to first, so that the first
            // one comes out next.
            waiting.extend(
                self.tasks
                    .iter()
                    .filter(|child| child.parent_id() == Some(task.id()))
                    .rev()
                    .map(|child| (child, level.saturating_add(1))),
            );
        }
        ordered
    }

    /// Who a task is given to, in words: "You" for the manager.
    fn assignee_name(&self, assignee: Option<Assignee>) -> String {
        match assignee {
            None => String::new(),
            Some(Assignee::Manager) => "You".to_owned(),
            Some(Assignee::Member(id)) => self
                .members
                .iter()
                .find(|member| member.id() == id)
                .map(|member| member.name().to_string())
                .unwrap_or_default(),
        }
    }

    /// One row a sprint, in the order they are listed: the running one,
    /// the planned ones, the completed ones.
    fn sprint_table(&self) -> Table {
        let rows = self
            .sprints
            .iter()
            .map(|sprint| {
                let work = self.work_of(sprint);
                let (points, points_done) = match work.unit {
                    SprintUnit::Points => (
                        Cell::Whole(work.committed.into()),
                        Cell::Whole(work.completed.into()),
                    ),
                    // Without points there is nothing to count in them.
                    SprintUnit::Tasks => (Cell::Empty, Cell::Empty),
                };
                vec![
                    Cell::text(sprint.name().as_str()),
                    Cell::text(sprint_state_in_words(sprint.state())),
                    Cell::date(sprint.dates().start_on()),
                    Cell::date(sprint.dates().end_on()),
                    Cell::text(sprint.goal()),
                    Cell::count(self.main_tasks_of(sprint).len()),
                    points,
                    points_done,
                ]
            })
            .collect();
        Table {
            columns: vec![
                Column::named("Sprint"),
                Column::named("State"),
                Column::named("Start"),
                Column::named("End"),
                Column::wrapping("Goal"),
                Column::named("Tasks"),
                Column::named("Points"),
                Column::named("Points done"),
            ],
            rows,
        }
    }

    /// The manager first, then every member in the order they joined, then
    /// the Advisor if the project has spoken with it.
    fn team_table(&self, tokens: &TokensUsed) -> Table {
        let given = |wanted: Assignee| {
            let theirs: Vec<&Task> = self
                .tasks
                .iter()
                .filter(|task| task.assignee() == Some(wanted))
                .collect();
            let finished = theirs
                .iter()
                .filter(|task| is_finished(self.project, task))
                .count();
            (Cell::count(theirs.len() - finished), Cell::count(finished))
        };

        let (open, finished) = given(Assignee::Manager);
        let mut rows = vec![vec![
            Cell::text("You"),
            Cell::text("Manager"),
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
            open,
            finished,
            Cell::Empty,
        ]];
        for member in self.members {
            let (open, finished) = given(Assignee::Member(member.id()));
            let is_agent = member.kind() == MemberKind::Agent;
            rows.push(vec![
                Cell::text(member.name().as_str()),
                Cell::text(if is_agent { "AI specialist" } else { "Person" }),
                Cell::text(member.responsibility()),
                member
                    .model()
                    .map_or(Cell::Empty, |model| Cell::text(model.as_str())),
                member
                    .effort()
                    .map_or(Cell::Empty, |effort| Cell::text(effort_in_words(effort))),
                open,
                finished,
                if is_agent {
                    Cell::Whole(whole(
                        tokens.by_member.get(&member.id()).copied().unwrap_or(0),
                    ))
                } else {
                    Cell::Empty
                },
            ]);
        }
        if tokens.has_advisor {
            rows.push(vec![
                Cell::text("Advisor"),
                Cell::text("Advisor"),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Whole(whole(tokens.advisor)),
            ]);
        }
        Table {
            columns: vec![
                Column::named("Member"),
                Column::named("Kind"),
                Column::wrapping("Responsibility"),
                Column::named("Model"),
                Column::named("Effort"),
                Column::named("Tasks open"),
                Column::named("Tasks finished"),
                Column::named("Tokens used"),
            ],
            rows,
        }
    }
}

/// A row of a table of facts: a label and a value.
fn fact(label: &str, value: Cell) -> Vec<Cell> {
    vec![Cell::text(label), value]
}

/// A table of facts, without a header.
fn facts(rows: Vec<Vec<Cell>>) -> Table {
    Table {
        columns: vec![Column::named(""), Column::wrapping("")],
        rows,
    }
}

/// A count of tokens as a whole number of a cell.
fn whole(tokens: u64) -> i64 {
    i64::try_from(tokens).unwrap_or(i64::MAX)
}

/// The words the interface uses for a priority.
const fn priority_in_words(priority: Priority) -> &'static str {
    match priority {
        Priority::Low => "Low",
        Priority::Normal => "Normal",
        Priority::High => "High",
        Priority::Urgent => "Urgent",
    }
}

/// The words the interface uses for an effort.
const fn effort_in_words(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "Low",
        Effort::Medium => "Medium",
        Effort::High => "High",
        Effort::ExtraHigh => "Extra high",
        Effort::Max => "Maximum",
    }
}

/// Where a sprint stands, in words.
const fn sprint_state_in_words(state: SprintState) -> &'static str {
    match state {
        SprintState::Planned => "Planned",
        SprintState::Active => "Running",
        SprintState::Completed => "Completed",
    }
}

/// The column of a custom field on the Tasks sheet: its name, with its
/// unit if it has one, "Unit cost (€)". A text column wraps.
fn field_column(field: &Field) -> Column {
    let name = match field.unit() {
        Some(unit) => format!("{} ({unit})", field.name()),
        None => field.name().to_string(),
    };
    if matches!(field.kind(), FieldKind::Text) {
        Column::wrapping(&name)
    } else {
        Column::named(&name)
    }
}

/// The value a task holds for a custom field, as a cell of its kind: a
/// number is a number and a date a date, so that the spreadsheet can sum
/// and sort them. A choice is its name; a choice removed since is left
/// out.
fn field_cell(field: &Field, task: &Task) -> Cell {
    match task.field_values().get(&field.id()) {
        None => Cell::Empty,
        Some(FieldValue::Text(text)) => Cell::text(text.as_str()),
        Some(FieldValue::Number(number)) => Cell::Number(number.value()),
        Some(FieldValue::Date(date)) => Cell::Date(*date),
        Some(FieldValue::YesNo(yes)) => Cell::text(if *yes { "Yes" } else { "No" }),
        Some(FieldValue::Choice(choice)) => field
            .choice(*choice)
            .map_or(Cell::Empty, |choice| Cell::text(choice.name().as_str())),
        Some(FieldValue::Link(address)) => Cell::text(address.as_str()),
    }
}

#[cfg(test)]
mod tests;
