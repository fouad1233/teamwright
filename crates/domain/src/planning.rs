//! What the Advisor asks and proposes when a project is planned.
//!
//! The Advisor is a session that plans a project with the manager. It asks
//! what it needs to know in an [`Interview`], and then makes a
//! [`Proposal`]: the members to add to the team and the tasks to create.
//! Nothing of a proposal exists before the manager approves it, and the
//! manager may approve less than was proposed.
//!
//! Before approving, the manager may change the proposal one
//! [`ProposalChange`] at a time: the title, the size and what a task waits
//! for, and the rest of what a task or a member says.
//!
//! A proposed task may be a part of another task of the same proposal, as
//! a subtask is a part of a task on the board. Parts follow the rules of
//! subtasks: a part may have parts of its own, at any depth, and no task is
//! a part of itself, directly or further down. A task with parts starts
//! only once its parts are finished, so a part may not wait for the task it
//! is a part of.
//!
//! The rules here are those about one proposal. Whether the owner of a task
//! is somebody the project has needs the team, and is checked where the
//! team is known.

use std::collections::HashSet;

use crate::conversation::DecisionOption;
use crate::dependencies::DependencyGraph;
use crate::error::DomainError;
use crate::member::Effort;
use crate::task::Priority;
use crate::text::{MemberName, ModelName, TaskTitle};
use crate::time::Timestamp;

/// One question of an interview: something the Advisor needs to know before
/// it can propose a team and a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterviewQuestion {
    /// The question. Never empty in an [`Interview`].
    pub question: String,
    /// Why the Advisor asks: what the answer changes. May be empty.
    pub reason: String,
    /// Two to four options, or none for a question that is answered in
    /// words.
    pub options: Vec<DecisionOption>,
    /// The option the Advisor recommends, by its place from 0, if it
    /// recommends one.
    pub recommended: Option<usize>,
}

/// The manager's answer to one question of an interview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterviewAnswer {
    /// The option the manager chose, by its place from 0, or nothing.
    pub option: Option<usize>,
    /// What the manager added to the option, or the whole answer if no
    /// option was chosen. Empty if an option was chosen and nothing added.
    pub note: String,
}

/// The plain data of an interview, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterviewSnapshot {
    /// The questions.
    pub questions: Vec<InterviewQuestion>,
    /// One answer for each question, once the manager has answered.
    pub answers: Option<Vec<InterviewAnswer>>,
    /// When the manager answered.
    pub answered_at: Option<Timestamp>,
}

/// The questions the Advisor asks the manager before it proposes. It asks
/// them all at once and ends its turn. The answers start the next turn of
/// the same conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interview {
    questions: Vec<InterviewQuestion>,
    answers: Option<Vec<InterviewAnswer>>,
    answered_at: Option<Timestamp>,
}

impl Interview {
    /// The most questions an interview has.
    pub const MOST_QUESTIONS: usize = 5;
    /// The fewest options a question with options has.
    pub const FEWEST_OPTIONS: usize = 2;
    /// The most options a question has.
    pub const MOST_OPTIONS: usize = 4;

    /// Makes an interview that has not been answered. Whitespace around the
    /// texts is removed.
    ///
    /// # Errors
    ///
    /// Returns an error if there are no questions or more than five, if a
    /// question is empty, if a question has one option or more than four,
    /// if an option has no label, or if a recommended option is not one of
    /// the options of its question.
    pub fn new(questions: Vec<InterviewQuestion>) -> Result<Self, DomainError> {
        if !(1..=Self::MOST_QUESTIONS).contains(&questions.len()) {
            return Err(DomainError::QuestionCount {
                count: questions.len(),
            });
        }
        let questions = questions
            .into_iter()
            .map(tidy_question)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            questions,
            answers: None,
            answered_at: None,
        })
    }

    /// Recreates an interview from stored data.
    #[must_use]
    pub fn restore(snapshot: InterviewSnapshot) -> Self {
        Self {
            questions: snapshot.questions,
            answers: snapshot.answers,
            answered_at: snapshot.answered_at,
        }
    }

    /// The questions, one to five.
    #[must_use]
    pub fn questions(&self) -> &[InterviewQuestion] {
        &self.questions
    }

    /// One answer for each question, in the same order. Nothing while the
    /// interview is open.
    #[must_use]
    pub fn answers(&self) -> Option<&[InterviewAnswer]> {
        self.answers.as_deref()
    }

    /// When the manager answered. Nothing while the interview is open.
    #[must_use]
    pub const fn answered_at(&self) -> Option<Timestamp> {
        self.answered_at
    }

    /// Whether the manager has not answered yet.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.answers.is_none()
    }

    /// Records the answers of the manager, one for each question.
    /// Whitespace around the words is removed.
    pub(crate) fn answer(
        &mut self,
        answers: Vec<InterviewAnswer>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if !self.is_open() {
            return Err(DomainError::AlreadyAnswered);
        }
        if answers.len() != self.questions.len() {
            return Err(DomainError::AnswerCount);
        }
        let answers = self
            .questions
            .iter()
            .zip(answers)
            .enumerate()
            .map(|(place, (question, answer))| {
                let number = place + 1;
                if answer
                    .option
                    .is_some_and(|option| option >= question.options.len())
                {
                    return Err(DomainError::NoSuchOptionOfQuestion { number });
                }
                let note = answer.note.trim();
                if answer.option.is_none() && note.is_empty() {
                    return Err(DomainError::EmptyAnswerToQuestion { number });
                }
                Ok(InterviewAnswer {
                    option: answer.option,
                    note: note.to_owned(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.answers = Some(answers);
        self.answered_at = Some(now);
        Ok(())
    }
}

/// Checks and tidies one question of an interview.
fn tidy_question(question: InterviewQuestion) -> Result<InterviewQuestion, DomainError> {
    let text = question.question.trim();
    if text.is_empty() {
        return Err(DomainError::EmptyText { field: "question" });
    }
    let count = question.options.len();
    if count != 0 && !(Interview::FEWEST_OPTIONS..=Interview::MOST_OPTIONS).contains(&count) {
        return Err(DomainError::InterviewOptionCount { count });
    }
    let options = question
        .options
        .into_iter()
        .map(|option| {
            let label = option.label.trim();
            if label.is_empty() {
                return Err(DomainError::EmptyText {
                    field: "label of an option",
                });
            }
            Ok(DecisionOption {
                label: label.to_owned(),
                consequence: option.consequence.trim().to_owned(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if question
        .recommended
        .is_some_and(|place| place >= options.len())
    {
        return Err(DomainError::NoSuchOption);
    }
    Ok(InterviewQuestion {
        question: text.to_owned(),
        reason: question.reason.trim().to_owned(),
        options,
        recommended: question.recommended,
    })
}

/// An AI specialist the Advisor proposes to add to the team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedMember {
    /// What the member is to be called.
    pub name: MemberName,
    /// What the member is responsible for. May be empty.
    pub responsibility: String,
    /// Why the project needs this member. May be empty.
    pub reason: String,
    /// The model the member is to run on, or the default of the program.
    pub model: Option<ModelName>,
    /// How much the member is to think, or the default of the program.
    pub effort: Option<Effort>,
}

/// Who a proposed task is given to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposedOwner {
    /// The manager.
    Manager,
    /// A member by name: a proposed member, or a member the project has.
    Member(MemberName),
}

/// A task the Advisor proposes to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedTask {
    /// What is to be done.
    pub title: TaskTitle,
    /// What the result is, and how to tell that it is done. May be empty.
    pub description: String,
    /// Who does it, or nobody yet.
    pub owner: Option<ProposedOwner>,
    /// The size of the task: the minutes a skilled person would need for
    /// it, if the Advisor said.
    pub estimate_minutes: Option<u32>,
    /// How urgent it is.
    pub priority: Priority,
    /// The tasks of the same proposal that must be finished first, by their
    /// places from 0.
    pub waits_for: Vec<usize>,
    /// The task of the same proposal this one is a part of, by its place
    /// from 0, or nothing for a task of its own. Approving makes it a
    /// subtask of that task.
    pub part_of: Option<usize>,
}

/// One change the manager makes to a proposal before approving it. Tasks
/// and members are named by their places from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalChange {
    /// Gives a task another title.
    Retitle {
        /// The task.
        task: usize,
        /// The new title.
        title: TaskTitle,
    },
    /// Gives a task another description. Whitespace around it is removed.
    Describe {
        /// The task.
        task: usize,
        /// The new description, which may be empty.
        description: String,
    },
    /// Gives a task another size, or none.
    Estimate {
        /// The task.
        task: usize,
        /// The minutes a skilled person would need, or nothing.
        estimate_minutes: Option<u32>,
    },
    /// Gives a task another priority.
    Prioritize {
        /// The task.
        task: usize,
        /// The new priority.
        priority: Priority,
    },
    /// Gives a task to somebody else, or to nobody.
    GiveTo {
        /// The task.
        task: usize,
        /// The new owner, or nobody.
        owner: Option<ProposedOwner>,
    },
    /// Lets a task wait for another task of the proposal. Waiting for a
    /// task it waits for already changes nothing.
    AddWait {
        /// The task that is to wait.
        task: usize,
        /// The task it is to wait for.
        waits_for: usize,
    },
    /// Lets a task stop waiting for another task of the proposal. A task
    /// it does not wait for changes nothing.
    RemoveWait {
        /// The task that waits.
        task: usize,
        /// The task it is to stop waiting for.
        waits_for: usize,
    },
    /// Adds a task of the manager's own at the end of the plan: given to
    /// nobody, of normal priority, without a size, waiting for nothing,
    /// and a part of a task of the proposal if one is named.
    AddTask {
        /// What is to be done.
        title: TaskTitle,
        /// The task it is a part of, or nothing for a task of its own.
        part_of: Option<usize>,
    },
    /// Makes a task a part of another task of the proposal, or, with
    /// nothing, a task of its own.
    MakePartOf {
        /// The task.
        task: usize,
        /// The task it is to be a part of, or nothing.
        part_of: Option<usize>,
    },
    /// Takes a task out. No task waits for it any more, its parts take its
    /// place under the task it was a part of (or become tasks of their
    /// own), and the places of the tasks after it move up by one.
    RemoveTask {
        /// The task.
        task: usize,
    },
    /// Takes a member out. The tasks given to that member stay, and are
    /// given to nobody: the work is still to be done.
    RemoveMember {
        /// The member.
        member: usize,
    },
    /// Chooses the model and the effort of a member.
    SetProfile {
        /// The member.
        member: usize,
        /// The model, or the default of the program.
        model: Option<ModelName>,
        /// The effort, or the default of the program.
        effort: Option<Effort>,
    },
}

/// The plain data of a proposal, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalSnapshot {
    /// The project in the Advisor's words.
    pub summary: String,
    /// The members to add.
    pub members: Vec<ProposedMember>,
    /// The tasks to create.
    pub tasks: Vec<ProposedTask>,
    /// When the manager approved it, if the manager has.
    pub approved_at: Option<Timestamp>,
}

/// What the Advisor proposes: who to add to the team, and which tasks to
/// create. Nothing of it exists before the manager approves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    summary: String,
    members: Vec<ProposedMember>,
    tasks: Vec<ProposedTask>,
    approved_at: Option<Timestamp>,
}

impl Proposal {
    /// Makes a proposal that has not been approved. Whitespace around the
    /// texts is removed, and a task that names the same task to wait for
    /// twice waits for it once.
    ///
    /// # Errors
    ///
    /// Returns an error if the summary is empty, or if the members and the
    /// tasks break a rule of a plan (see [`Self::approve`]).
    pub fn new(
        summary: &str,
        members: Vec<ProposedMember>,
        tasks: Vec<ProposedTask>,
    ) -> Result<Self, DomainError> {
        let summary = summary.trim();
        if summary.is_empty() {
            return Err(DomainError::EmptyText {
                field: "summary of the proposal",
            });
        }
        let (members, tasks) = tidy_plan(members, tasks)?;
        Ok(Self {
            summary: summary.to_owned(),
            members,
            tasks,
            approved_at: None,
        })
    }

    /// Recreates a proposal from stored data.
    #[must_use]
    pub fn restore(snapshot: ProposalSnapshot) -> Self {
        Self {
            summary: snapshot.summary,
            members: snapshot.members,
            tasks: snapshot.tasks,
            approved_at: snapshot.approved_at,
        }
    }

    /// The project in the Advisor's words, for the manager to check.
    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    /// The members to add. Once the proposal is approved, the members the
    /// manager approved.
    #[must_use]
    pub fn members(&self) -> &[ProposedMember] {
        &self.members
    }

    /// The tasks to create. Once the proposal is approved, the tasks the
    /// manager approved.
    #[must_use]
    pub fn tasks(&self) -> &[ProposedTask] {
        &self.tasks
    }

    /// When the manager approved it. Nothing while it is not approved.
    #[must_use]
    pub const fn approved_at(&self) -> Option<Timestamp> {
        self.approved_at
    }

    /// Whether the manager has not approved it yet.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.approved_at.is_none()
    }

    /// Checks what the manager approves of the proposal, which may be less
    /// than or different from what was proposed, and returns it tidied.
    /// Nothing is recorded.
    ///
    /// # Errors
    ///
    /// Returns an error if two members have the same name, whatever the
    /// capitals; if a task waits for itself or for a place that is not
    /// another task of the list; if tasks wait for each other in a circle;
    /// if a task is a part of itself or of a place that is not another task
    /// of the list; if tasks are parts of each other in a circle; if a part
    /// waits for the task it is a part of, directly or through other tasks;
    /// or if nothing is left: no members and no tasks.
    pub fn check_approval(
        members: Vec<ProposedMember>,
        tasks: Vec<ProposedTask>,
    ) -> Result<(Vec<ProposedMember>, Vec<ProposedTask>), DomainError> {
        tidy_plan(members, tasks)
    }

    /// Makes one change of the manager's to the proposal. A change that
    /// breaks a rule leaves the proposal as it was.
    ///
    /// # Errors
    ///
    /// Returns an error if the proposal has been approved already, if the
    /// change names a task or a member the proposal does not have, if a
    /// task is to wait for itself or for a task that waits for it, if a
    /// task is to be a part of itself or of one of its parts, or if
    /// nothing would be left: no members and no tasks.
    pub fn change(&mut self, change: ProposalChange) -> Result<(), DomainError> {
        if !self.is_open() {
            return Err(DomainError::AlreadyApproved);
        }
        let mut members = self.members.clone();
        let mut tasks = self.tasks.clone();
        match change {
            ProposalChange::Retitle { task, title } => task_at(&mut tasks, task)?.title = title,
            ProposalChange::Describe { task, description } => {
                task_at(&mut tasks, task)?.description = description;
            }
            ProposalChange::Estimate {
                task,
                estimate_minutes,
            } => task_at(&mut tasks, task)?.estimate_minutes = estimate_minutes,
            ProposalChange::Prioritize { task, priority } => {
                task_at(&mut tasks, task)?.priority = priority;
            }
            ProposalChange::GiveTo { task, owner } => task_at(&mut tasks, task)?.owner = owner,
            ProposalChange::AddWait { task, waits_for } => add_wait(&mut tasks, task, waits_for)?,
            ProposalChange::RemoveWait { task, waits_for } => {
                task_at(&mut tasks, waits_for)?;
                task_at(&mut tasks, task)?
                    .waits_for
                    .retain(|&earlier| earlier != waits_for);
            }
            ProposalChange::AddTask { title, part_of } => {
                if let Some(whole) = part_of {
                    task_at(&mut tasks, whole)?;
                }
                tasks.push(ProposedTask {
                    title,
                    description: String::new(),
                    owner: None,
                    estimate_minutes: None,
                    priority: Priority::Normal,
                    waits_for: Vec::new(),
                    part_of,
                });
            }
            ProposalChange::MakePartOf { task, part_of } => {
                make_part_of(&mut tasks, task, part_of)?;
            }
            ProposalChange::RemoveTask { task } => {
                let whole = task_at(&mut tasks, task)?.part_of;
                tasks.remove(task);
                let moved_up = |place: usize| if place > task { place - 1 } else { place };
                for other in &mut tasks {
                    other.waits_for = other
                        .waits_for
                        .iter()
                        .filter(|&&earlier| earlier != task)
                        .map(|&earlier| moved_up(earlier))
                        .collect();
                    // A part of the task that is taken out stays in the
                    // plan, because its work is still to be done. It goes
                    // where the task was.
                    if other.part_of == Some(task) {
                        other.part_of = whole;
                    }
                    other.part_of = other.part_of.map(moved_up);
                }
            }
            ProposalChange::RemoveMember { member } => {
                let gone = member_at(&mut members, member)?.name.clone();
                members.remove(member);
                let key = gone.as_str().to_lowercase();
                for task in &mut tasks {
                    if let Some(ProposedOwner::Member(name)) = &task.owner
                        && name.as_str().to_lowercase() == key
                    {
                        task.owner = None;
                    }
                }
            }
            ProposalChange::SetProfile {
                member,
                model,
                effort,
            } => {
                let member = member_at(&mut members, member)?;
                member.model = model;
                member.effort = effort;
            }
        }
        let (members, tasks) = tidy_plan(members, tasks)?;
        self.members = members;
        self.tasks = tasks;
        Ok(())
    }

    /// Records what the manager approved. The proposal keeps it in place
    /// of what was proposed.
    pub(crate) fn approve(
        &mut self,
        members: Vec<ProposedMember>,
        tasks: Vec<ProposedTask>,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        if !self.is_open() {
            return Err(DomainError::AlreadyApproved);
        }
        let (members, tasks) = tidy_plan(members, tasks)?;
        self.members = members;
        self.tasks = tasks;
        self.approved_at = Some(now);
        Ok(())
    }
}

/// Checks the members and the tasks of a plan, and returns them tidied.
fn tidy_plan(
    members: Vec<ProposedMember>,
    tasks: Vec<ProposedTask>,
) -> Result<(Vec<ProposedMember>, Vec<ProposedTask>), DomainError> {
    let mut names = HashSet::new();
    let members = members
        .into_iter()
        .map(|member| {
            if !names.insert(member.name.as_str().to_lowercase()) {
                return Err(DomainError::DuplicateProposedMember {
                    name: member.name.to_string(),
                });
            }
            Ok(ProposedMember {
                responsibility: member.responsibility.trim().to_owned(),
                reason: member.reason.trim().to_owned(),
                ..member
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let count = tasks.len();
    let tasks = tasks
        .into_iter()
        .enumerate()
        .map(|(place, task)| {
            let mut waits_for: Vec<usize> = Vec::new();
            for earlier in task.waits_for {
                if earlier >= count || earlier == place {
                    return Err(DomainError::WaitsForNoOtherTask {
                        title: task.title.to_string(),
                    });
                }
                if !waits_for.contains(&earlier) {
                    waits_for.push(earlier);
                }
            }
            if task
                .part_of
                .is_some_and(|whole| whole >= count || whole == place)
            {
                return Err(DomainError::PartOfNoOtherTask {
                    title: task.title.to_string(),
                });
            }
            Ok(ProposedTask {
                description: task.description.trim().to_owned(),
                waits_for,
                ..task
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if waits_of(&tasks).has_cycle() {
        return Err(DomainError::TasksWaitInCircle);
    }
    if wholes_of(&tasks).has_cycle() {
        return Err(DomainError::PartsInCircle);
    }
    if graph_of(&tasks).has_cycle() {
        return Err(DomainError::PartsWaitInCircle);
    }
    if members.is_empty() && tasks.is_empty() {
        return Err(DomainError::EmptyProposal);
    }
    Ok((members, tasks))
}

/// The task at a place of a plan, counted from 0.
fn task_at(tasks: &mut [ProposedTask], place: usize) -> Result<&mut ProposedTask, DomainError> {
    tasks.get_mut(place).ok_or(DomainError::NoSuchProposedTask {
        number: place.saturating_add(1),
    })
}

/// The member at a place of a plan, counted from 0.
fn member_at(
    members: &mut [ProposedMember],
    place: usize,
) -> Result<&mut ProposedMember, DomainError> {
    members
        .get_mut(place)
        .ok_or(DomainError::NoSuchProposedMember {
            number: place.saturating_add(1),
        })
}

/// Lets one task of a plan wait for another, if that closes no circle.
fn add_wait(tasks: &mut [ProposedTask], task: usize, waits_for: usize) -> Result<(), DomainError> {
    let other = task_at(tasks, waits_for)?.title.to_string();
    let title = task_at(tasks, task)?.title.to_string();
    if task == waits_for {
        return Err(DomainError::WaitsForNoOtherTask { title });
    }
    if graph_of(tasks).would_create_cycle(task, waits_for) {
        return Err(DomainError::WaitWouldCloseCircle { title, other });
    }
    let waiting = task_at(tasks, task)?;
    if !waiting.waits_for.contains(&waits_for) {
        waiting.waits_for.push(waits_for);
    }
    Ok(())
}

/// Makes a task of a plan a part of another, or a task of its own, if that
/// closes no circle.
fn make_part_of(
    tasks: &mut [ProposedTask],
    task: usize,
    part_of: Option<usize>,
) -> Result<(), DomainError> {
    let title = task_at(tasks, task)?.title.to_string();
    if let Some(whole) = part_of {
        let other = task_at(tasks, whole)?.title.to_string();
        if whole == task {
            return Err(DomainError::PartOfNoOtherTask { title });
        }
        // The task it is to be a part of must not lie below it already.
        if wholes_of(tasks).would_create_cycle(task, whole) {
            return Err(DomainError::PartWouldCloseCircle { title, other });
        }
    }
    task_at(tasks, task)?.part_of = part_of;
    Ok(())
}

/// What the tasks of a plan wait for, by their places: what each task
/// says it waits for and, as on the board, a task with parts for its
/// parts.
fn graph_of(tasks: &[ProposedTask]) -> DependencyGraph<usize> {
    DependencyGraph::from_waits(tasks.iter().enumerate().map(|(place, task)| {
        let parts = tasks
            .iter()
            .enumerate()
            .filter(move |(_, other)| other.part_of == Some(place))
            .map(|(part, _)| part);
        (place, task.waits_for.iter().copied().chain(parts))
    }))
}

/// What the tasks of a plan wait for, by their places, as they say it,
/// without their parts.
fn waits_of(tasks: &[ProposedTask]) -> DependencyGraph<usize> {
    DependencyGraph::from_waits(
        tasks
            .iter()
            .enumerate()
            .map(|(place, task)| (place, task.waits_for.iter().copied())),
    )
}

/// Which task of a plan each task is a part of, as a graph in which a part
/// waits for its task, so that a circle of parts shows as a circle.
fn wholes_of(tasks: &[ProposedTask]) -> DependencyGraph<usize> {
    DependencyGraph::from_waits(
        tasks
            .iter()
            .enumerate()
            .map(|(place, task)| (place, task.part_of)),
    )
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn moment(minute: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 12, 9, minute, 0)
            .unwrap()
    }

    fn option(label: &str) -> DecisionOption {
        DecisionOption {
            label: label.to_owned(),
            consequence: String::new(),
        }
    }

    fn question(text: &str, options: Vec<DecisionOption>) -> InterviewQuestion {
        InterviewQuestion {
            question: text.to_owned(),
            reason: String::new(),
            options,
            recommended: None,
        }
    }

    fn interview() -> Interview {
        Interview::new(vec![
            question("What matters most?", vec![option("Soon"), option("Cheap")]),
            question("What is fixed?", Vec::new()),
        ])
        .unwrap()
    }

    fn answer(option: Option<usize>, note: &str) -> InterviewAnswer {
        InterviewAnswer {
            option,
            note: note.to_owned(),
        }
    }

    fn member(name: &str) -> ProposedMember {
        ProposedMember {
            name: MemberName::parse(name).unwrap(),
            responsibility: String::new(),
            reason: String::new(),
            model: None,
            effort: None,
        }
    }

    fn task(title: &str, waits_for: Vec<usize>) -> ProposedTask {
        ProposedTask {
            title: TaskTitle::parse(title).unwrap(),
            description: String::new(),
            owner: None,
            estimate_minutes: None,
            priority: Priority::Normal,
            waits_for,
            part_of: None,
        }
    }

    #[test]
    fn an_interview_is_tidied_and_starts_without_answers() {
        let interview = Interview::new(vec![InterviewQuestion {
            question: "  What matters most? ".to_owned(),
            reason: " It sets the order. ".to_owned(),
            options: vec![option(" Soon "), option("Cheap")],
            recommended: Some(0),
        }])
        .unwrap();

        assert_eq!(interview.questions()[0].question, "What matters most?");
        assert_eq!(interview.questions()[0].reason, "It sets the order.");
        assert_eq!(interview.questions()[0].options[0], option("Soon"));
        assert!(interview.is_open());
        assert_eq!(interview.answers(), None);
        assert_eq!(interview.answered_at(), None);
    }

    #[test]
    fn an_interview_has_one_to_five_questions_with_two_to_four_options_or_none() {
        assert_eq!(
            Interview::new(Vec::new()).unwrap_err().to_string(),
            "an interview has one to five questions, but this one has 0"
        );
        assert_eq!(
            Interview::new(vec![question("Which?", Vec::new()); 6]),
            Err(DomainError::QuestionCount { count: 6 })
        );
        assert_eq!(
            Interview::new(vec![question("Which?", vec![option("Only")])])
                .unwrap_err()
                .to_string(),
            "a question of an interview has two to four options or none, but this one has 1"
        );
        assert_eq!(
            Interview::new(vec![question(" ", Vec::new())]),
            Err(DomainError::EmptyText { field: "question" })
        );
        assert_eq!(
            Interview::new(vec![InterviewQuestion {
                recommended: Some(0),
                ..question("What is fixed?", Vec::new())
            }]),
            Err(DomainError::NoSuchOption)
        );
    }

    #[test]
    fn an_interview_is_answered_once_with_one_answer_for_each_question() {
        let mut interview = interview();

        interview
            .answer(
                vec![answer(Some(1), "  Soon after.  "), answer(None, "Nothing.")],
                moment(3),
            )
            .unwrap();

        assert_eq!(
            interview.answers(),
            Some([answer(Some(1), "Soon after."), answer(None, "Nothing.")].as_slice())
        );
        assert_eq!(interview.answered_at(), Some(moment(3)));
        assert!(!interview.is_open());
        assert_eq!(
            interview.answer(vec![answer(Some(0), ""), answer(None, "No.")], moment(4)),
            Err(DomainError::AlreadyAnswered)
        );
    }

    #[test]
    fn answers_that_do_not_fit_the_interview_are_refused_with_the_number_of_the_question() {
        let refused = |answers: Vec<InterviewAnswer>| {
            let mut interview = interview();
            let error = interview.answer(answers, moment(3)).unwrap_err();
            assert!(interview.is_open());
            error.to_string()
        };

        assert_eq!(
            refused(vec![answer(Some(0), "")]),
            "every question needs an answer"
        );
        assert_eq!(
            refused(vec![answer(Some(2), ""), answer(None, "No.")]),
            "question 1 has no such option"
        );
        assert_eq!(
            refused(vec![answer(Some(0), ""), answer(Some(0), "")]),
            "question 2 has no such option"
        );
        assert_eq!(
            refused(vec![answer(Some(0), ""), answer(None, "  ")]),
            "the answer to question 2 names one of the options or says something in words"
        );
    }

    #[test]
    fn a_proposal_is_tidied_and_starts_without_approval() {
        let proposal = Proposal::new(
            "  A device.  ",
            vec![ProposedMember {
                responsibility: " The board. ".to_owned(),
                ..member("Hardware engineer")
            }],
            vec![
                task("Draw the schematic", Vec::new()),
                task("Route the board", vec![0, 0]),
            ],
        )
        .unwrap();

        assert_eq!(proposal.summary(), "A device.");
        assert_eq!(proposal.members()[0].responsibility, "The board.");
        assert_eq!(proposal.tasks()[1].waits_for, [0]);
        assert!(proposal.is_open());
        assert_eq!(proposal.approved_at(), None);
    }

    #[test]
    fn a_proposal_has_a_summary_and_something_in_it() {
        assert_eq!(
            Proposal::new(" ", vec![member("Engineer")], Vec::new()),
            Err(DomainError::EmptyText {
                field: "summary of the proposal"
            })
        );
        assert_eq!(
            Proposal::new("A device.", Vec::new(), Vec::new())
                .unwrap_err()
                .to_string(),
            "the proposal has no members and no tasks"
        );
    }

    #[test]
    fn no_two_members_of_a_proposal_have_the_same_name_whatever_the_capitals() {
        let error = Proposal::check_approval(
            vec![member("Hardware engineer"), member("hardware ENGINEER")],
            Vec::new(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "two members of the proposal are called \"hardware ENGINEER\""
        );
    }

    #[test]
    fn a_task_waits_only_for_another_task_of_the_proposal() {
        for waits_for in [vec![1], vec![2]] {
            let error = Proposal::check_approval(
                Vec::new(),
                vec![
                    task("Draw the schematic", Vec::new()),
                    task("Route the board", waits_for),
                ],
            )
            .unwrap_err();
            assert_eq!(
                error.to_string(),
                "\"Route the board\" waits for a task that is not another task of the proposal"
            );
        }
    }

    #[test]
    fn tasks_of_a_proposal_do_not_wait_for_each_other_in_a_circle() {
        let plan = |waits: [Vec<usize>; 4]| {
            Proposal::check_approval(
                Vec::new(),
                waits
                    .into_iter()
                    .map(|waits_for| task("A task", waits_for))
                    .collect(),
            )
        };

        assert!(plan([vec![], vec![0], vec![0, 1], vec![2]]).is_ok());
        assert!(plan([vec![2], vec![], vec![1], vec![]]).is_ok());
        assert_eq!(
            plan([vec![1], vec![0], vec![], vec![]]),
            Err(DomainError::TasksWaitInCircle)
        );
        assert_eq!(
            plan([vec![], vec![3], vec![1], vec![2]])
                .unwrap_err()
                .to_string(),
            "the tasks of the proposal wait for each other in a circle"
        );
    }

    #[test]
    fn a_proposal_is_approved_once_and_keeps_what_was_approved() {
        let mut proposal = Proposal::new(
            "A device.",
            vec![member("Hardware engineer"), member("Firmware engineer")],
            vec![task("Draw the schematic", Vec::new())],
        )
        .unwrap();

        proposal
            .approve(vec![member("Hardware engineer")], Vec::new(), moment(5))
            .unwrap();

        assert_eq!(proposal.members(), [member("Hardware engineer")]);
        assert!(proposal.tasks().is_empty());
        assert_eq!(proposal.approved_at(), Some(moment(5)));
        assert_eq!(
            proposal.approve(vec![member("Hardware engineer")], Vec::new(), moment(6)),
            Err(DomainError::AlreadyApproved)
        );
    }

    #[test]
    fn a_refused_approval_leaves_the_proposal_open_and_unchanged() {
        let mut proposal =
            Proposal::new("A device.", vec![member("Hardware engineer")], Vec::new()).unwrap();
        let before = proposal.clone();

        let refused = proposal.approve(Vec::new(), Vec::new(), moment(5));

        assert_eq!(refused, Err(DomainError::EmptyProposal));
        assert_eq!(proposal, before);
    }

    /// A proposal of two members and four tasks: 1 waits for 0, 2 for 1,
    /// and 3 for nothing. Task 0 is the hardware engineer's.
    fn plan() -> Proposal {
        Proposal::new(
            "A device.",
            vec![member("Hardware engineer"), member("Firmware engineer")],
            vec![
                ProposedTask {
                    owner: Some(ProposedOwner::Member(
                        MemberName::parse("hardware ENGINEER").unwrap(),
                    )),
                    ..task("Draw the schematic", Vec::new())
                },
                task("Route the board", vec![0]),
                task("Write the firmware", vec![1]),
                task("Write the manual", Vec::new()),
            ],
        )
        .unwrap()
    }

    fn title(text: &str) -> TaskTitle {
        TaskTitle::parse(text).unwrap()
    }

    fn changed(change: ProposalChange) -> Result<Proposal, DomainError> {
        let mut proposal = plan();
        let before = proposal.clone();
        let result = proposal.change(change);
        if result.is_err() {
            assert_eq!(
                proposal, before,
                "a refused change leaves the proposal as it was"
            );
        }
        result.map(|()| proposal)
    }

    #[test]
    fn the_title_the_size_and_the_rest_of_a_task_can_be_changed() {
        let mut proposal = plan();

        proposal
            .change(ProposalChange::Retitle {
                task: 1,
                title: title("Route the board in four layers"),
            })
            .unwrap();
        proposal
            .change(ProposalChange::Estimate {
                task: 1,
                estimate_minutes: Some(240),
            })
            .unwrap();
        proposal
            .change(ProposalChange::Describe {
                task: 1,
                description: "  Gerber files.  ".to_owned(),
            })
            .unwrap();
        proposal
            .change(ProposalChange::Prioritize {
                task: 1,
                priority: Priority::High,
            })
            .unwrap();
        proposal
            .change(ProposalChange::GiveTo {
                task: 1,
                owner: Some(ProposedOwner::Manager),
            })
            .unwrap();

        let task = &proposal.tasks()[1];
        assert_eq!(task.title.as_str(), "Route the board in four layers");
        assert_eq!(task.estimate_minutes, Some(240));
        assert_eq!(task.description, "Gerber files.");
        assert_eq!(task.priority, Priority::High);
        assert_eq!(task.owner, Some(ProposedOwner::Manager));
        assert_eq!(task.waits_for, [0]);
        assert!(proposal.is_open());
    }

    #[test]
    fn a_task_can_wait_for_another_task_of_the_proposal_and_stop_waiting() {
        let mut proposal = plan();

        proposal
            .change(ProposalChange::AddWait {
                task: 3,
                waits_for: 2,
            })
            .unwrap();
        proposal
            .change(ProposalChange::AddWait {
                task: 3,
                waits_for: 2,
            })
            .unwrap();
        assert_eq!(proposal.tasks()[3].waits_for, [2]);

        proposal
            .change(ProposalChange::RemoveWait {
                task: 2,
                waits_for: 1,
            })
            .unwrap();
        proposal
            .change(ProposalChange::RemoveWait {
                task: 2,
                waits_for: 1,
            })
            .unwrap();
        assert!(proposal.tasks()[2].waits_for.is_empty());
    }

    #[test]
    fn a_task_waits_neither_for_itself_nor_in_a_circle_nor_for_a_task_that_is_not_there() {
        assert_eq!(
            changed(ProposalChange::AddWait {
                task: 1,
                waits_for: 1
            })
            .unwrap_err()
            .to_string(),
            "\"Route the board\" waits for a task that is not another task of the proposal"
        );
        assert_eq!(
            changed(ProposalChange::AddWait {
                task: 0,
                waits_for: 2
            })
            .unwrap_err()
            .to_string(),
            "\"Draw the schematic\" cannot wait for \"Write the firmware\", which waits for it \
             already"
        );
        assert_eq!(
            changed(ProposalChange::AddWait {
                task: 0,
                waits_for: 4
            })
            .unwrap_err()
            .to_string(),
            "the proposal has no task 5"
        );
        assert_eq!(
            changed(ProposalChange::RemoveWait {
                task: 9,
                waits_for: 0
            }),
            Err(DomainError::NoSuchProposedTask { number: 10 })
        );
        assert_eq!(
            changed(ProposalChange::RemoveTask { task: usize::MAX }),
            Err(DomainError::NoSuchProposedTask { number: usize::MAX })
        );
    }

    #[test]
    fn taking_a_task_out_takes_out_what_waited_for_it_and_counts_the_places_anew() {
        let proposal = changed(ProposalChange::RemoveTask { task: 1 }).unwrap();

        let titles: Vec<_> = proposal
            .tasks()
            .iter()
            .map(|task| task.title.as_str())
            .collect();
        assert_eq!(
            titles,
            [
                "Draw the schematic",
                "Write the firmware",
                "Write the manual"
            ]
        );
        assert!(proposal.tasks()[1].waits_for.is_empty());

        let later = changed(ProposalChange::RemoveTask { task: 0 }).unwrap();
        assert!(later.tasks()[0].waits_for.is_empty());
        assert_eq!(later.tasks()[1].waits_for, [0]);
    }

    #[test]
    fn a_task_of_the_managers_own_comes_at_the_end_and_waits_for_nothing() {
        let proposal = changed(ProposalChange::AddTask {
            title: title("Order the parts"),
            part_of: None,
        })
        .unwrap();

        assert_eq!(proposal.tasks().len(), 5);
        assert_eq!(proposal.tasks()[4], task("Order the parts", Vec::new()));
    }

    #[test]
    fn taking_a_member_out_leaves_its_tasks_to_nobody() {
        let proposal = changed(ProposalChange::RemoveMember { member: 0 }).unwrap();

        assert_eq!(proposal.members(), [member("Firmware engineer")]);
        assert_eq!(proposal.tasks()[0].owner, None);
        assert_eq!(
            changed(ProposalChange::RemoveMember { member: 2 })
                .unwrap_err()
                .to_string(),
            "the proposal has no member 3"
        );
    }

    #[test]
    fn the_model_and_the_effort_of_a_member_can_be_chosen() {
        let proposal = changed(ProposalChange::SetProfile {
            member: 1,
            model: Some(ModelName::parse("haiku").unwrap()),
            effort: Some(Effort::Low),
        })
        .unwrap();

        assert_eq!(
            proposal.members()[1].model,
            Some(ModelName::parse("haiku").unwrap())
        );
        assert_eq!(proposal.members()[1].effort, Some(Effort::Low));
    }

    #[test]
    fn a_change_never_leaves_a_proposal_empty() {
        let mut proposal = Proposal::new(
            "A device.",
            Vec::new(),
            vec![task("Draw the schematic", Vec::new())],
        )
        .unwrap();

        assert_eq!(
            proposal.change(ProposalChange::RemoveTask { task: 0 }),
            Err(DomainError::EmptyProposal)
        );
        assert_eq!(proposal.tasks().len(), 1);
    }

    #[test]
    fn an_approved_proposal_is_not_changed() {
        let mut proposal = plan();
        proposal
            .approve(
                proposal.members().to_vec(),
                proposal.tasks().to_vec(),
                moment(5),
            )
            .unwrap();
        let before = proposal.clone();

        assert_eq!(
            proposal.change(ProposalChange::RemoveTask { task: 0 }),
            Err(DomainError::AlreadyApproved)
        );
        assert_eq!(proposal, before);
    }

    fn part(title: &str, part_of: usize) -> ProposedTask {
        ProposedTask {
            part_of: Some(part_of),
            ..task(title, Vec::new())
        }
    }

    /// A plan with parts: 0 has the parts 1 and 2, 2 has the part 3, and
    /// 4 is a task of its own that waits for 0.
    fn plan_with_parts() -> Proposal {
        Proposal::new(
            "A device.",
            Vec::new(),
            vec![
                task("Build the board", Vec::new()),
                part("Draw the schematic", 0),
                part("Route the board", 0),
                part("Check the clearances", 2),
                task("Write the manual", vec![0]),
            ],
        )
        .unwrap()
    }

    fn parts_changed(change: ProposalChange) -> Result<Proposal, DomainError> {
        let mut proposal = plan_with_parts();
        let before = proposal.clone();
        let result = proposal.change(change);
        if result.is_err() {
            assert_eq!(proposal, before);
        }
        result.map(|()| proposal)
    }

    fn wholes(proposal: &Proposal) -> Vec<Option<usize>> {
        proposal.tasks().iter().map(|task| task.part_of).collect()
    }

    #[test]
    fn a_task_can_be_a_part_of_another_task_and_a_part_can_have_parts() {
        let proposal = plan_with_parts();

        assert_eq!(wholes(&proposal), [None, Some(0), Some(0), Some(2), None]);
    }

    #[test]
    fn a_task_is_a_part_only_of_another_task_of_the_proposal() {
        for whole in [1, 2] {
            let error = Proposal::check_approval(
                Vec::new(),
                vec![
                    task("Build the board", Vec::new()),
                    part("Route the board", whole),
                ],
            )
            .unwrap_err();
            assert_eq!(
                error.to_string(),
                "\"Route the board\" is a part of a task that is not another task of the proposal"
            );
        }
    }

    #[test]
    fn tasks_are_not_parts_of_each_other_in_a_circle() {
        let error = Proposal::check_approval(
            Vec::new(),
            vec![
                part("Build the board", 2),
                part("Draw the schematic", 0),
                part("Route the board", 1),
            ],
        )
        .unwrap_err();

        assert_eq!(error, DomainError::PartsInCircle);
        assert_eq!(
            error.to_string(),
            "the tasks of the proposal are parts of each other in a circle"
        );
    }

    #[test]
    fn a_part_does_not_wait_for_the_task_it_is_a_part_of() {
        // The task waits for its parts, as on the board, so a part that
        // waits for it, directly or through another task, waits forever.
        for waits in [vec![0], vec![2]] {
            let error = Proposal::check_approval(
                Vec::new(),
                vec![
                    task("Build the board", Vec::new()),
                    ProposedTask {
                        part_of: Some(0),
                        ..task("Route the board", waits)
                    },
                    task("Order the parts", vec![0]),
                ],
            )
            .unwrap_err();
            assert_eq!(error, DomainError::PartsWaitInCircle);
        }
        // A part may wait for another part of the same task.
        assert!(
            Proposal::check_approval(
                Vec::new(),
                vec![
                    task("Build the board", Vec::new()),
                    part("Draw the schematic", 0),
                    ProposedTask {
                        part_of: Some(0),
                        ..task("Route the board", vec![1])
                    },
                ],
            )
            .is_ok()
        );
    }

    #[test]
    fn a_part_can_be_moved_to_another_task_or_made_a_task_of_its_own() {
        let moved = parts_changed(ProposalChange::MakePartOf {
            task: 3,
            part_of: Some(4),
        })
        .unwrap();
        assert_eq!(wholes(&moved), [None, Some(0), Some(0), Some(4), None]);

        let alone = parts_changed(ProposalChange::MakePartOf {
            task: 1,
            part_of: None,
        })
        .unwrap();
        assert_eq!(wholes(&alone), [None, None, Some(0), Some(2), None]);
    }

    #[test]
    fn a_task_is_not_made_a_part_of_itself_or_of_its_own_parts() {
        assert_eq!(
            parts_changed(ProposalChange::MakePartOf {
                task: 0,
                part_of: Some(3),
            })
            .unwrap_err()
            .to_string(),
            "\"Build the board\" cannot be a part of \"Check the clearances\", which is a part of \
             it already"
        );
        assert_eq!(
            parts_changed(ProposalChange::MakePartOf {
                task: 2,
                part_of: Some(2),
            }),
            Err(DomainError::PartOfNoOtherTask {
                title: "Route the board".to_owned()
            })
        );
        assert_eq!(
            parts_changed(ProposalChange::MakePartOf {
                task: 2,
                part_of: Some(5),
            }),
            Err(DomainError::NoSuchProposedTask { number: 6 })
        );
        // A task of its own that waits for the whole cannot become a part
        // of it: the whole would wait for it in turn.
        assert_eq!(
            parts_changed(ProposalChange::MakePartOf {
                task: 4,
                part_of: Some(0),
            }),
            Err(DomainError::PartsWaitInCircle)
        );
    }

    #[test]
    fn a_part_does_not_wait_for_its_task_once_it_is_one() {
        assert_eq!(
            parts_changed(ProposalChange::AddWait {
                task: 3,
                waits_for: 0,
            })
            .unwrap_err()
            .to_string(),
            "\"Check the clearances\" cannot wait for \"Build the board\", which waits for it \
             already"
        );
    }

    #[test]
    fn taking_a_task_out_leaves_its_parts_where_it_was() {
        let proposal = parts_changed(ProposalChange::RemoveTask { task: 2 }).unwrap();

        let titles: Vec<_> = proposal
            .tasks()
            .iter()
            .map(|task| task.title.as_str())
            .collect();
        assert_eq!(
            titles,
            [
                "Build the board",
                "Draw the schematic",
                "Check the clearances",
                "Write the manual"
            ]
        );
        assert_eq!(wholes(&proposal), [None, Some(0), Some(0), None]);
        assert_eq!(proposal.tasks()[3].waits_for, [0]);

        let top = parts_changed(ProposalChange::RemoveTask { task: 0 }).unwrap();
        assert_eq!(wholes(&top), [None, None, Some(1), None]);
        assert!(top.tasks()[3].waits_for.is_empty());
    }

    #[test]
    fn a_task_of_the_managers_own_can_be_added_as_a_part() {
        let proposal = parts_changed(ProposalChange::AddTask {
            title: title("Order the stencil"),
            part_of: Some(2),
        })
        .unwrap();

        assert_eq!(proposal.tasks()[5], part("Order the stencil", 2));
        assert_eq!(
            parts_changed(ProposalChange::AddTask {
                title: title("Order the stencil"),
                part_of: Some(9),
            }),
            Err(DomainError::NoSuchProposedTask { number: 10 })
        );
    }
}
