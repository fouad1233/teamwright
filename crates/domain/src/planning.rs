//! What the Advisor asks and proposes when a project is planned.
//!
//! The Advisor is a session that plans a project with the manager. It asks
//! what it needs to know in an [`Interview`], and then makes a
//! [`Proposal`]: the members to add to the team and the tasks to create.
//! Nothing of a proposal exists before the manager approves it, and the
//! manager may approve less than was proposed.
//!
//! The rules here are those about one proposal. Whether the owner of a task
//! is somebody the project has needs the team, and is checked where the
//! team is known.

use std::collections::HashSet;

use crate::conversation::DecisionOption;
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
    /// or if nothing is left: no members and no tasks.
    pub fn check_approval(
        members: Vec<ProposedMember>,
        tasks: Vec<ProposedTask>,
    ) -> Result<(Vec<ProposedMember>, Vec<ProposedTask>), DomainError> {
        tidy_plan(members, tasks)
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
            Ok(ProposedTask {
                description: task.description.trim().to_owned(),
                waits_for,
                ..task
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if wait_in_circle(&tasks) {
        return Err(DomainError::TasksWaitInCircle);
    }
    if members.is_empty() && tasks.is_empty() {
        return Err(DomainError::EmptyProposal);
    }
    Ok((members, tasks))
}

/// Whether some tasks wait for each other in a circle, so that none of them
/// could ever start. Every place a task waits for is one of the tasks.
fn wait_in_circle(tasks: &[ProposedTask]) -> bool {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        NotSeen,
        OnTheWay,
        Clear,
    }

    // A task is looked at once. While the tasks it waits for are followed,
    // it is on the way; meeting a task that is on the way closes a circle.
    fn closes_circle(place: usize, tasks: &[ProposedTask], marks: &mut [Mark]) -> bool {
        match marks.get(place).copied() {
            Some(Mark::Clear) | None => return false,
            Some(Mark::OnTheWay) => return true,
            Some(Mark::NotSeen) => {}
        }
        marks[place] = Mark::OnTheWay;
        let found = tasks[place]
            .waits_for
            .iter()
            .any(|&earlier| closes_circle(earlier, tasks, marks));
        marks[place] = Mark::Clear;
        found
    }

    let mut marks = vec![Mark::NotSeen; tasks.len()];
    (0..tasks.len()).any(|place| closes_circle(place, tasks, &mut marks))
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
}
