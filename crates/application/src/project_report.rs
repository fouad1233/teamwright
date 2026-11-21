//! The report of a project: one document about the whole of it, from its
//! start to the day the report is made
//! ([decision 0042](../../../docs/decisions/0042-the-report-of-a-whole-project.md)).
//!
//! It says what was to be built, what was done and by whom, how the plan
//! compared with what happened, how the sprints ended, what the team used,
//! what the manager decided, what the specialists handed over, and what is
//! still open. Everything in it is worked out by rules from what is
//! stored; no model writes any of it, and a figure the stored data cannot
//! give is not guessed but said to be missing.
//!
//! There is no state "completed" for a project. A project is complete
//! when it has tasks and every one of them is in a status of the category
//! done. The report can be made at any time; until the project is
//! complete it is called the report of the project so far.
//!
//! [`ProjectReportService::report_of`] gathers the report as plain data.
//! The interface draws it, and [`markdown::report_in_markdown`] writes it
//! as a Markdown file for the work folder.

pub mod markdown;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use teamwright_domain::{
    AreaId, Assignee, CalendarDate, Conversation, Handoff, Member, MemberId, MemberKind, Project,
    ProjectId, Sprint, SprintState, SprintSummary, SprintUnit, Task, TaskId, Timestamp, Turn,
};

use crate::error::ApplicationError;
use crate::ports::{
    AreaRepository, Clock, ConversationRepository, MemberRepository, ProjectRepository,
    SprintRepository, TaskRepository, TeamworkRepository,
};
use crate::sprints::{counted, is_finished};
use crate::teamwork::latest_handoff_of;
use crate::usage::usage_of;
use crate::work::{ConversationWithTurns, reference_of};

/// The report of a whole project, as plain data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReport {
    /// The project it is about.
    pub project_id: ProjectId,
    /// The name of the project.
    pub name: String,
    /// The key of its tasks, such as `SSN`.
    pub key: String,
    /// The manager's description of the project. May be empty.
    pub description: String,
    /// The day the report was made.
    pub made_on: CalendarDate,
    /// The day the project was begun.
    pub begun_on: CalendarDate,
    /// Whether the project is complete: it has tasks, and every one of
    /// them is finished.
    pub is_complete: bool,
    /// What the report is called: "Report of the project", or "Report of
    /// the project so far" while it is not complete.
    pub title: String,
    /// A few sentences that say where the project ended, or stands.
    pub summary: Vec<String>,
    /// What was to be built, as the Advisor wrote it, if the manager
    /// approved a plan of the Advisor.
    pub brief: Option<ReportBrief>,
    /// The work of every area that has tasks. Empty when the project has
    /// only one area: it would repeat the whole.
    pub by_area: Vec<WorkShare>,
    /// The work of everyone who holds tasks: the members in the order they
    /// joined, then the manager, then the tasks given to nobody.
    pub by_holder: Vec<WorkShare>,
    /// The finished tasks, the one finished first first.
    pub finished: Vec<ReportTask>,
    /// The plan against what happened.
    pub plan: PlanAgainstActual,
    /// The sprints, in the order they began.
    pub sprints: Vec<SprintOutcome>,
    /// What the team used.
    pub usage: ReportUsage,
    /// The questions the manager answered, the one answered first first.
    pub decisions: Vec<ReportDecision>,
    /// What the specialists handed over: the latest handoff of every task
    /// that has one, in the order of the tasks.
    pub handoffs: Vec<ReportHandoff>,
    /// The tasks that are not finished, in the order of their numbers.
    pub open: Vec<ReportTask>,
    /// How many questions of the specialists and the Advisor still wait
    /// for the manager's answer.
    pub waiting: u32,
}

/// What the project was to build, in the Advisor's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBrief {
    /// The summary of the plan the manager approved.
    pub summary: String,
    /// The day the manager approved it.
    pub approved_on: CalendarDate,
}

/// What one area, or one holder of tasks, has finished and has open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkShare {
    /// The name of the area or of the holder.
    pub name: String,
    /// Its finished tasks.
    pub finished: u32,
    /// Its tasks that are not finished.
    pub open: u32,
}

/// A task as the report lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTask {
    /// The task.
    pub task_id: TaskId,
    /// Its reference, such as `SSN-4`.
    pub reference: String,
    /// Its title.
    pub title: String,
    /// The name of its area, when the project has more than one.
    pub area: Option<String>,
    /// Who holds it, in words: a member, "The manager" or "Nobody".
    pub holder: String,
    /// The name of its status, or nothing for a status that is gone.
    pub status: Option<String>,
    /// When it was due, if it had a due date.
    pub due_on: Option<CalendarDate>,
    /// The day it was finished, for a finished task.
    pub finished_on: Option<CalendarDate>,
    /// Its estimate in minutes, if it had one.
    pub estimate_minutes: Option<u32>,
    /// How long the turns of the specialists on it ran, in minutes.
    /// Nothing if no specialist had a turn on it that ended.
    pub worked_minutes: Option<u64>,
    /// How it stands against its due date.
    pub due: DueOutcome,
}

/// How a task stands against its due date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueOutcome {
    /// It has no due date.
    NoDueDate,
    /// It was finished on or before its due date.
    Met,
    /// It was finished after its due date, by so many days.
    Missed {
        /// The days it was finished after its due date.
        days: u32,
    },
    /// It is open, and its due date has not passed.
    Coming,
    /// It is open, and its due date has passed by so many days.
    Late {
        /// The days since its due date.
        days: u32,
    },
}

/// The plan against what happened: only what the stored data can say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanAgainstActual {
    /// The finished tasks.
    pub finished: u32,
    /// The finished tasks that had an estimate.
    pub estimated: u32,
    /// Their estimates together, in minutes.
    pub estimated_minutes: u64,
    /// The finished tasks on which a specialist had a turn that ended.
    pub worked: u32,
    /// How long those turns ran together, in minutes.
    pub worked_minutes: u64,
    /// The finished tasks that had both an estimate and turns.
    pub compared: u32,
    /// The estimates of those tasks together, in minutes.
    pub compared_estimate_minutes: u64,
    /// How long the turns on those tasks ran together, in minutes.
    pub compared_worked_minutes: u64,
    /// The finished tasks that had a due date.
    pub with_due_date: u32,
    /// Those of them finished on or before it.
    pub met: u32,
    /// Those of them finished after it.
    pub missed: u32,
    /// The open tasks whose due date has passed.
    pub late: u32,
    /// What the figures say, in sentences, with what is missing said
    /// plainly.
    pub sentences: Vec<String>,
}

/// How a sprint went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintOutcome {
    /// Its name.
    pub name: String,
    /// Its goal. May be empty.
    pub goal: String,
    /// Where it stands.
    pub state: SprintState,
    /// Its first day, if it has one.
    pub start_on: Option<CalendarDate>,
    /// Its last day, if it has one.
    pub end_on: Option<CalendarDate>,
    /// What its work is counted in.
    pub unit: SprintUnit,
    /// The work it took on: when it was completed, or now.
    pub committed: u32,
    /// The work of it that was finished.
    pub completed: u32,
}

/// What the team used, in tokens and turns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportUsage {
    /// By who used it: the members, the one who used the most first, then
    /// the Advisor, then the members who left, together.
    pub by_who: Vec<UsageLine>,
    /// By the model the turns ran on, the most used first.
    pub by_model: Vec<UsageLine>,
    /// Everything together.
    pub total: UsageLine,
}

/// One line of what was used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageLine {
    /// Who or what it is about.
    pub name: String,
    /// Every kind of token together.
    pub tokens: u64,
    /// The turns that ended.
    pub turns: u32,
}

/// A question the manager answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDecision {
    /// Who asked: a specialist, "The Advisor", or "A former member".
    pub asked_by: String,
    /// The reference of the task it was about, if it was about one.
    pub task: Option<String>,
    /// The question.
    pub question: String,
    /// The label of the option the manager chose, if one was chosen.
    pub chosen: Option<String>,
    /// What the manager wrote. May be empty when an option was chosen.
    pub note: String,
    /// The day the manager answered.
    pub answered_on: CalendarDate,
}

/// What a specialist handed over for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportHandoff {
    /// The reference of the task.
    pub reference: String,
    /// The title of the task.
    pub title: String,
    /// Who handed it over.
    pub author: String,
    /// The day it was handed over.
    pub published_on: CalendarDate,
    /// What was done, in the specialist's words.
    pub summary: String,
    /// The interface others may rely on. May be empty.
    pub contract: String,
    /// The files, as paths inside the work folder.
    pub files: Vec<String>,
}

/// The title of a complete project's report.
pub const TITLE_OF_COMPLETE: &str = "Report of the project";
/// The title of the report of a project that is not complete.
pub const TITLE_SO_FAR: &str = "Report of the project so far";

/// What the report needs from the outside world.
pub struct ProjectReportPorts {
    /// Where projects are stored.
    pub projects: Arc<dyn ProjectRepository>,
    /// Where tasks are stored.
    pub tasks: Arc<dyn TaskRepository>,
    /// Where sprints are stored.
    pub sprints: Arc<dyn SprintRepository>,
    /// Where the areas of the projects are stored.
    pub areas: Arc<dyn AreaRepository>,
    /// Where the members of the teams are stored.
    pub members: Arc<dyn MemberRepository>,
    /// Where conversations and their turns are stored.
    pub conversations: Arc<dyn ConversationRepository>,
    /// Where the handoffs of tasks are stored.
    pub teamwork: Arc<dyn TeamworkRepository>,
    /// The time, for the day of the report and the days of moments.
    pub clock: Arc<dyn Clock>,
}

/// The use case that gathers the report of a project.
#[derive(Clone)]
pub struct ProjectReportService {
    ports: Arc<ProjectReportPorts>,
}

impl ProjectReportService {
    /// Creates the service.
    #[must_use]
    pub fn new(ports: ProjectReportPorts) -> Self {
        Self {
            ports: Arc::new(ports),
        }
    }

    /// Gathers the report of a project as it stands now.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn report_of(&self, project_id: ProjectId) -> Result<ProjectReport, ApplicationError> {
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
        let sprints = ports.sprints.list_for_project(project_id)?;
        let mut members = ports.members.list_for_project(project_id)?;
        members.sort_by_key(|member| (member.joined_at(), member.id()));
        let mut areas = ports.areas.list_for_project(project_id)?;
        areas.sort_by_key(|area| (area.position(), area.id()));
        let area_names: Vec<(AreaId, String)> = areas
            .iter()
            .map(|area| (area.id(), area.name().to_string()))
            .collect();
        let conversations = ports
            .conversations
            .list_for_project(project_id)?
            .into_iter()
            .map(|conversation| {
                let turns = ports.conversations.turns_of(conversation.id())?;
                Ok(ConversationWithTurns {
                    conversation,
                    turns,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let handoffs = ports.teamwork.handoffs_of_project(project_id)?;

        Ok(report_of(&Stored {
            project: &project,
            tasks: &tasks,
            sprints: &sprints,
            members: &members,
            areas: &area_names,
            conversations: &conversations,
            handoffs: &handoffs,
            now: ports.clock.now(),
            clock: ports.clock.as_ref(),
        }))
    }
}

/// Everything a report is made from, loaded once.
struct Stored<'a> {
    project: &'a Project,
    /// The tasks, by number.
    tasks: &'a [Task],
    sprints: &'a [Sprint],
    /// The members, in the order they joined.
    members: &'a [Member],
    /// The areas with their names, in their order.
    areas: &'a [(AreaId, String)],
    /// The conversations with their turns, the oldest first.
    conversations: &'a [ConversationWithTurns],
    handoffs: &'a [Handoff],
    now: Timestamp,
    /// Tells on which day a moment falls, where the manager is.
    clock: &'a dyn Clock,
}

/// The report, worked out from what is stored.
fn report_of(stored: &Stored<'_>) -> ProjectReport {
    let project = stored.project;
    let today = stored.clock.day_of(stored.now);
    let worked = worked_minutes_by_task(stored.conversations);
    let report_task = |task: &Task| stored.report_task(task, &worked, today);

    let mut finished: Vec<ReportTask> = stored
        .tasks
        .iter()
        .filter(|task| is_finished(project, task))
        .map(report_task)
        .collect();
    // The order in which the work was finished; a task without a day of
    // finishing, which only data from an older version can have, last.
    finished.sort_by_key(|task| (task.finished_on.is_none(), task.finished_on));
    let open: Vec<ReportTask> = stored
        .tasks
        .iter()
        .filter(|task| !is_finished(project, task))
        .map(report_task)
        .collect();
    let is_complete = !stored.tasks.is_empty() && open.is_empty();

    let decisions = stored.decisions();
    let waiting = waiting_questions(stored.conversations);
    let usage = stored.usage();
    let plan = plan_against_actual(&finished, &open, stored.has_untimed_work());

    ProjectReport {
        project_id: project.id(),
        name: project.name().to_string(),
        key: project.key().as_str().to_owned(),
        description: project.description().trim().to_owned(),
        made_on: today,
        begun_on: stored.clock.day_of(project.created_at()),
        is_complete,
        title: if is_complete {
            TITLE_OF_COMPLETE
        } else {
            TITLE_SO_FAR
        }
        .to_owned(),
        summary: summary_of(&SummarySources {
            begun_on: stored.clock.day_of(project.created_at()),
            finished: &finished,
            open: open.len(),
            usage: &usage,
            decisions: decisions.len(),
            waiting,
        }),
        brief: stored.brief(),
        by_area: stored.by_area(),
        by_holder: stored.by_holder(),
        finished,
        plan,
        sprints: stored.sprints(),
        usage,
        decisions,
        handoffs: stored.handoffs(),
        open,
        waiting,
    }
}

impl Stored<'_> {
    fn report_task(
        &self,
        task: &Task,
        worked: &HashMap<TaskId, u64>,
        today: CalendarDate,
    ) -> ReportTask {
        let finished_on = is_finished(self.project, task)
            .then(|| task.completed_at().map(|moment| self.clock.day_of(moment)))
            .flatten();
        let due_on = task.schedule().due_on();
        ReportTask {
            task_id: task.id(),
            reference: reference_of(self.project, task),
            title: task.title().as_str().to_owned(),
            area: (self.areas.len() > 1)
                .then(|| self.area_name(task.area_id()))
                .flatten(),
            holder: self.holder_name(task.assignee()),
            status: self
                .project
                .workflow()
                .status(task.status_id())
                .map(|status| status.name().to_string()),
            due_on,
            finished_on,
            estimate_minutes: task.estimate_minutes(),
            worked_minutes: worked.get(&task.id()).copied(),
            due: due_outcome(is_finished(self.project, task), due_on, finished_on, today),
        }
    }

    fn area_name(&self, area_id: AreaId) -> Option<String> {
        self.areas
            .iter()
            .find(|(id, _)| *id == area_id)
            .map(|(_, name)| name.clone())
    }

    fn member(&self, member_id: MemberId) -> Option<&Member> {
        self.members.iter().find(|member| member.id() == member_id)
    }

    /// Who holds a task, in words.
    fn holder_name(&self, assignee: Option<Assignee>) -> String {
        match assignee {
            None => NOBODY.to_owned(),
            Some(Assignee::Manager) => MANAGER.to_owned(),
            Some(Assignee::Member(member_id)) => self.member_name(Some(member_id)),
        }
    }

    /// The name of a member, or the words for one who has left.
    fn member_name(&self, member_id: Option<MemberId>) -> String {
        member_id.and_then(|id| self.member(id)).map_or_else(
            || FORMER_MEMBER.to_owned(),
            |member| member.name().to_string(),
        )
    }

    /// Who a conversation is with, in words.
    fn speaker_of(&self, conversation: &Conversation) -> String {
        if conversation.is_with_advisor() {
            ADVISOR.to_owned()
        } else {
            self.member_name(conversation.member_id())
        }
    }

    /// Whether some finished work is not timed by Teamwright: work given
    /// to a person, to the manager, or to nobody.
    fn has_untimed_work(&self) -> bool {
        self.tasks.iter().any(|task| {
            is_finished(self.project, task)
                && match task.assignee() {
                    None | Some(Assignee::Manager) => true,
                    Some(Assignee::Member(member_id)) => self
                        .member(member_id)
                        .is_some_and(|member| member.kind() == MemberKind::Human),
                }
        })
    }

    /// The summary of the latest plan of the Advisor the manager approved.
    fn brief(&self) -> Option<ReportBrief> {
        self.conversations
            .iter()
            .filter(|conversation| conversation.conversation.is_with_advisor())
            .flat_map(|conversation| &conversation.turns)
            .filter_map(Turn::proposal)
            .filter_map(|proposal| {
                let approved_at = proposal.approved_at()?;
                let summary = proposal.summary().trim();
                (!summary.is_empty()).then(|| (approved_at, summary.to_owned()))
            })
            .max_by_key(|(approved_at, _)| *approved_at)
            .map(|(approved_at, summary)| ReportBrief {
                summary,
                approved_on: self.clock.day_of(approved_at),
            })
    }

    /// The work of every area that has tasks, when there is more than one.
    fn by_area(&self) -> Vec<WorkShare> {
        if self.areas.len() < 2 {
            return Vec::new();
        }
        self.areas
            .iter()
            .filter_map(|(area_id, name)| self.share(name, |task| task.area_id() == *area_id))
            .collect()
    }

    /// The work of every member who holds tasks, then the manager's, then
    /// the tasks given to nobody.
    fn by_holder(&self) -> Vec<WorkShare> {
        let members = self.members.iter().filter_map(|member| {
            self.share(member.name().as_str(), |task| {
                task.assignee() == Some(Assignee::Member(member.id()))
            })
        });
        let gone = self.share(FORMER_MEMBER, |task| {
            matches!(task.assignee(), Some(Assignee::Member(id)) if self.member(id).is_none())
        });
        let manager = self.share(MANAGER, |task| task.assignee() == Some(Assignee::Manager));
        let nobody = self.share(NOBODY, |task| task.assignee().is_none());
        members.chain(gone).chain(manager).chain(nobody).collect()
    }

    /// What the tasks that match have finished and have open, or nothing
    /// when no task matches.
    fn share(&self, name: &str, matches: impl Fn(&Task) -> bool) -> Option<WorkShare> {
        let (finished, open) = self.tasks.iter().filter(|task| matches(task)).fold(
            (0_u32, 0_u32),
            |(finished, open), task| {
                if is_finished(self.project, task) {
                    (finished.saturating_add(1), open)
                } else {
                    (finished, open.saturating_add(1))
                }
            },
        );
        (finished + open > 0).then(|| WorkShare {
            name: name.to_owned(),
            finished,
            open,
        })
    }

    /// Every sprint, in the order it began or was planned. A completed
    /// sprint keeps what it was completed with; any other is counted from
    /// its tasks now, as its report counts it.
    fn sprints(&self) -> Vec<SprintOutcome> {
        let mut sprints: Vec<&Sprint> = self.sprints.iter().collect();
        sprints.sort_by_key(|sprint| {
            (
                sprint.dates().start_on().is_none(),
                sprint.dates().start_on(),
                sprint.created_at(),
                sprint.id(),
            )
        });
        sprints
            .into_iter()
            .map(|sprint| {
                let summary = sprint.summary().unwrap_or_else(|| {
                    SprintSummary::of(
                        &self
                            .tasks
                            .iter()
                            .filter(|task| {
                                task.sprint_id() == Some(sprint.id()) && task.parent_id().is_none()
                            })
                            .map(|task| counted(self.project, task))
                            .collect::<Vec<_>>(),
                    )
                });
                SprintOutcome {
                    name: sprint.name().to_string(),
                    goal: sprint.goal().trim().to_owned(),
                    state: sprint.state(),
                    start_on: sprint.dates().start_on(),
                    end_on: sprint.dates().end_on(),
                    unit: summary.unit,
                    committed: summary.committed,
                    completed: summary.completed,
                }
            })
            .collect()
    }

    /// What the team used: by who, as the Usage table counts it, and by
    /// model.
    fn usage(&self) -> ReportUsage {
        let usage = usage_of(self.conversations);
        let line = |name: &str, usage: &crate::usage::AgentUsage| UsageLine {
            name: name.to_owned(),
            tokens: usage.tokens.all(),
            turns: usage.turns,
        };
        let mut by_who: Vec<UsageLine> = usage
            .members
            .iter()
            .map(|member| line(&self.member_name(Some(member.member_id)), &member.usage))
            .collect();
        if let Some(advisor) = &usage.advisor {
            by_who.push(line(ADVISOR, advisor));
        }
        if let Some(former) = &usage.former {
            by_who.push(line(FORMER_MEMBERS, former));
        }
        ReportUsage {
            by_who,
            by_model: usage_by_model(self.conversations),
            total: line("Everyone", &usage.total),
        }
    }

    /// The questions the manager answered: the decisions the specialists
    /// asked for and the questions of the Advisor's interviews, the one
    /// answered first first.
    fn decisions(&self) -> Vec<ReportDecision> {
        let mut decisions: Vec<(Timestamp, ReportDecision)> = Vec::new();
        for conversation in self.conversations {
            let asked_by = self.speaker_of(&conversation.conversation);
            let task = conversation
                .conversation
                .task_id()
                .and_then(|task_id| self.tasks.iter().find(|task| task.id() == task_id))
                .map(|task| reference_of(self.project, task));
            for turn in &conversation.turns {
                if let Some(decision) = turn.decision()
                    && let Some(answer) = decision.answer()
                {
                    decisions.push((
                        answer.at,
                        ReportDecision {
                            asked_by: asked_by.clone(),
                            task: task.clone(),
                            question: decision.question().to_owned(),
                            chosen: answer
                                .option
                                .and_then(|place| decision.options().get(place))
                                .map(|option| option.label.clone()),
                            note: answer.note.trim().to_owned(),
                            answered_on: self.clock.day_of(answer.at),
                        },
                    ));
                }
                if let Some(interview) = turn.interview()
                    && let (Some(answers), Some(at)) =
                        (interview.answers(), interview.answered_at())
                {
                    for (question, answer) in interview.questions().iter().zip(answers) {
                        decisions.push((
                            at,
                            ReportDecision {
                                asked_by: asked_by.clone(),
                                task: task.clone(),
                                question: question.question.clone(),
                                chosen: answer
                                    .option
                                    .and_then(|place| question.options.get(place))
                                    .map(|option| option.label.clone()),
                                note: answer.note.trim().to_owned(),
                                answered_on: self.clock.day_of(at),
                            },
                        ));
                    }
                }
            }
        }
        // A stable sort keeps the questions of one interview in their order.
        decisions.sort_by_key(|(at, _)| *at);
        decisions
            .into_iter()
            .map(|(_, decision)| decision)
            .collect()
    }

    /// The latest handoff of every task that has one, in the order of the
    /// tasks. The handoff of a task that was deleted is left out with it.
    fn handoffs(&self) -> Vec<ReportHandoff> {
        self.tasks
            .iter()
            .filter_map(|task| {
                let handoff = latest_handoff_of(self.handoffs, task.id())?;
                let content = handoff.content();
                Some(ReportHandoff {
                    reference: reference_of(self.project, task),
                    title: task.title().as_str().to_owned(),
                    author: self.member_name(Some(handoff.author_id())),
                    published_on: self.clock.day_of(handoff.published_at()),
                    summary: content.summary().to_owned(),
                    contract: content.contract().to_owned(),
                    files: content.files().iter().map(|path| path.as_text()).collect(),
                })
            })
            .collect()
    }
}

/// The manager, as a holder of tasks and on paper.
const MANAGER: &str = "The manager";
/// The holder of the tasks given to nobody.
const NOBODY: &str = "Nobody";
/// The Advisor, as a speaker.
const ADVISOR: &str = "The Advisor";
/// A member who has left the team since.
const FORMER_MEMBER: &str = "A former member";
/// The members who have left, together.
const FORMER_MEMBERS: &str = "Former members";
/// The model of turns that did not name one.
const MODEL_NOT_NAMED: &str = "Model not named";

/// How a task stands against its due date, on `today`.
fn due_outcome(
    is_finished: bool,
    due_on: Option<CalendarDate>,
    finished_on: Option<CalendarDate>,
    today: CalendarDate,
) -> DueOutcome {
    let Some(due_on) = due_on else {
        return DueOutcome::NoDueDate;
    };
    let days_after = |day: CalendarDate| u32::try_from((day - due_on).num_days()).unwrap_or(0);
    if is_finished {
        // A finished task whose day of finishing is not known cannot be
        // said to have met its date or missed it.
        return finished_on.map_or(DueOutcome::NoDueDate, |finished_on| {
            if finished_on <= due_on {
                DueOutcome::Met
            } else {
                DueOutcome::Missed {
                    days: days_after(finished_on),
                }
            }
        });
    }
    if today > due_on {
        DueOutcome::Late {
            days: days_after(today),
        }
    } else {
        DueOutcome::Coming
    }
}

/// How long the turns of the specialists on each task ran, in whole
/// minutes. A turn that is still running is not counted; nor is the
/// Advisor, which works on no task.
fn worked_minutes_by_task(conversations: &[ConversationWithTurns]) -> HashMap<TaskId, u64> {
    let mut seconds: HashMap<TaskId, u64> = HashMap::new();
    for conversation in conversations {
        let Some(task_id) = conversation.conversation.task_id() else {
            continue;
        };
        for turn in &conversation.turns {
            let Some(ended_at) = turn.ended_at() else {
                continue;
            };
            let ran = u64::try_from((ended_at - turn.started_at()).num_seconds()).unwrap_or(0);
            let sum = seconds.entry(task_id).or_default();
            *sum = sum.saturating_add(ran);
        }
    }
    seconds
        .into_iter()
        .map(|(task_id, seconds)| (task_id, seconds.div_ceil(60)))
        .collect()
}

/// The questions that wait for the manager: decisions not answered and
/// interviews not answered, in turns that have ended.
fn waiting_questions(conversations: &[ConversationWithTurns]) -> u32 {
    let count = conversations
        .iter()
        .flat_map(|conversation| &conversation.turns)
        .filter(|turn| !turn.is_running())
        .filter(|turn| {
            turn.decision().is_some_and(|decision| decision.is_open())
                || turn
                    .interview()
                    .is_some_and(|interview| interview.is_open())
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The tokens and turns of every model the turns that ended ran on, the
/// most used first. Turns that named no model are counted together.
fn usage_by_model(conversations: &[ConversationWithTurns]) -> Vec<UsageLine> {
    let mut by_model: BTreeMap<String, UsageLine> = BTreeMap::new();
    for turn in conversations
        .iter()
        .flat_map(|conversation| &conversation.turns)
        .filter(|turn| !turn.is_running())
    {
        let name = turn.model().unwrap_or(MODEL_NOT_NAMED);
        let line = by_model
            .entry(name.to_owned())
            .or_insert_with(|| UsageLine {
                name: name.to_owned(),
                ..UsageLine::default()
            });
        line.tokens = line
            .tokens
            .saturating_add(turn.tokens().map_or(0, |tokens| tokens.all()));
        line.turns = line.turns.saturating_add(1);
    }
    let mut lines: Vec<UsageLine> = by_model.into_values().collect();
    // The names are in order already, which settles a tie.
    lines.sort_by_key(|line| std::cmp::Reverse(line.tokens));
    lines
}

/// The plan against what happened, from the finished and the open tasks.
/// `has_untimed_work` says whether some finished work was done by people,
/// whose time Teamwright does not know.
fn plan_against_actual(
    finished: &[ReportTask],
    open: &[ReportTask],
    has_untimed_work: bool,
) -> PlanAgainstActual {
    let mut plan = PlanAgainstActual {
        finished: count(finished.len()),
        late: count(
            open.iter()
                .filter(|task| matches!(task.due, DueOutcome::Late { .. }))
                .count(),
        ),
        ..PlanAgainstActual::default()
    };
    for task in finished {
        if let Some(estimate) = task.estimate_minutes {
            plan.estimated += 1;
            plan.estimated_minutes += u64::from(estimate);
        }
        if let Some(worked) = task.worked_minutes {
            plan.worked += 1;
            plan.worked_minutes += worked;
        }
        if let (Some(estimate), Some(worked)) = (task.estimate_minutes, task.worked_minutes) {
            plan.compared += 1;
            plan.compared_estimate_minutes += u64::from(estimate);
            plan.compared_worked_minutes += worked;
        }
        match task.due {
            DueOutcome::Met => {
                plan.with_due_date += 1;
                plan.met += 1;
            }
            DueOutcome::Missed { .. } => {
                plan.with_due_date += 1;
                plan.missed += 1;
            }
            DueOutcome::NoDueDate | DueOutcome::Coming | DueOutcome::Late { .. } => {}
        }
    }
    plan.sentences = plan_in_words(&plan, has_untimed_work);
    plan
}

/// What the plan against what happened says, in sentences.
fn plan_in_words(plan: &PlanAgainstActual, has_untimed_work: bool) -> Vec<String> {
    if plan.finished == 0 {
        let mut sentences = vec!["No task is finished yet, so nothing can be compared.".to_owned()];
        if plan.late > 0 {
            sentences.push(late_in_words(plan.late));
        }
        return sentences;
    }
    let mut sentences = Vec::new();
    sentences.push(if plan.estimated == 0 {
        "No finished task had an estimate.".to_owned()
    } else {
        format!(
            "{} had an estimate: {} together.",
            of_the_finished(plan.estimated, plan.finished),
            duration_in_words(plan.estimated_minutes),
        )
    });
    if plan.worked == 0 {
        sentences.push(
            "No specialist had a turn on a finished task, so no time was measured.".to_owned(),
        );
    } else {
        sentences.push(format!(
            "The specialists' turns on {} ran {} together.",
            tasks_in_words(plan.worked, "finished task"),
            duration_in_words(plan.worked_minutes),
        ));
    }
    if plan.compared > 0 {
        sentences.push(format!(
            "On the {} with both, the estimates came to {} and the turns ran {}. An estimate is the time a skilled person would need; the turns are how long the specialists worked.",
            tasks_in_words(plan.compared, "task"),
            duration_in_words(plan.compared_estimate_minutes),
            duration_in_words(plan.compared_worked_minutes),
        ));
    }
    if has_untimed_work {
        sentences.push(
            "Teamwright does not time the work of people, so their tasks are counted and not measured."
                .to_owned(),
        );
    }
    sentences.push(if plan.with_due_date == 0 {
        "No finished task had a due date.".to_owned()
    } else if plan.missed == 0 {
        format!(
            "{} a due date, and every one was finished by it.",
            if plan.with_due_date == 1 {
                "One finished task had".to_owned()
            } else {
                format!("{} finished tasks had", plan.with_due_date)
            }
        )
    } else {
        format!(
            "Of the {} with a due date, {} finished by it and {} after it.",
            tasks_in_words(plan.with_due_date, "finished task"),
            plan.met,
            plan.missed,
        )
    });
    if plan.late > 0 {
        sentences.push(late_in_words(plan.late));
    }
    sentences
}

/// "3 of the 8 finished tasks", or "Every finished task" when it is all.
fn of_the_finished(part: u32, finished: u32) -> String {
    if part == finished {
        if finished == 1 {
            "The finished task".to_owned()
        } else {
            format!("All {finished} finished tasks")
        }
    } else {
        format!("{part} of the {finished} finished tasks")
    }
}

/// "1 task", "3 tasks".
fn tasks_in_words(count: u32, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// "2 open tasks are past their due date."
fn late_in_words(late: u32) -> String {
    if late == 1 {
        "1 open task is past its due date.".to_owned()
    } else {
        format!("{late} open tasks are past their due date.")
    }
}

/// What the summary is made from.
struct SummarySources<'a> {
    begun_on: CalendarDate,
    finished: &'a [ReportTask],
    open: usize,
    usage: &'a ReportUsage,
    decisions: usize,
    waiting: u32,
}

/// A few sentences that say where the project ended, or stands.
fn summary_of(sources: &SummarySources<'_>) -> Vec<String> {
    let finished = count(sources.finished.len());
    let open = count(sources.open);
    let all = finished + open;
    let mut sentences = Vec::new();
    let begun = format!(
        "The project was begun on {}",
        date_in_words(sources.begun_on)
    );
    if all == 0 {
        sentences.push(format!("{begun}. It has no tasks yet."));
    } else if open == 0 {
        let last = sources
            .finished
            .iter()
            .filter_map(|task| task.finished_on)
            .max();
        sentences.push(match last {
            Some(last) => format!(
                "{begun}, and its last task was finished on {}.",
                date_in_words(last)
            ),
            None => format!("{begun}."),
        });
        sentences.push(if all == 1 {
            "Its one task is finished.".to_owned()
        } else {
            format!("All {all} of its tasks are finished.")
        });
    } else {
        sentences.push(format!("{begun}."));
        sentences.push(format!(
            "{finished} of its {} {} finished, and {} still open.",
            tasks_in_words(all, "task"),
            if finished == 1 { "is" } else { "are" },
            if open == 1 {
                "1 is".to_owned()
            } else {
                format!("{open} are")
            },
        ));
    }
    let total = &sources.usage.total;
    if total.turns > 0 {
        sentences.push(format!(
            "The team took {} {} and used {} tokens.",
            total.turns,
            if total.turns == 1 { "turn" } else { "turns" },
            number_in_words(total.tokens),
        ));
    }
    if sources.decisions > 0 {
        sentences.push(if sources.decisions == 1 {
            "The manager answered 1 question.".to_owned()
        } else {
            format!("The manager answered {} questions.", sources.decisions)
        });
    }
    if sources.waiting > 0 {
        sentences.push(if sources.waiting == 1 {
            "1 question still waits for the manager.".to_owned()
        } else {
            format!("{} questions still wait for the manager.", sources.waiting)
        });
    }
    sentences
}

/// A count that fits the figures of the report.
fn count(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// A day in words: "21 November 2026".
#[must_use]
pub fn date_in_words(date: CalendarDate) -> String {
    date.format("%-d %B %Y").to_string()
}

/// A length of time in words: "45 minutes", "1 hour", "12.5 hours".
#[must_use]
pub fn duration_in_words(minutes: u64) -> String {
    if minutes < 60 {
        return if minutes == 1 {
            "1 minute".to_owned()
        } else {
            format!("{minutes} minutes")
        };
    }
    // Tenths of an hour, rounded.
    let tenths = (minutes * 10 + 30) / 60;
    let hours = if tenths.is_multiple_of(10) {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    };
    if hours == "1" {
        "1 hour".to_owned()
    } else {
        format!("{hours} hours")
    }
}

/// A number with its thousands set apart: "1,234,567".
#[must_use]
pub fn number_in_words(number: u64) -> String {
    let digits = number.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (place, digit) in digits.chars().enumerate() {
        if place > 0 && (digits.len() - place).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests;
