//! What happened in a project, said in plain data: a specialist finished
//! its work on a task, a specialist or the Advisor waits for the manager,
//! one of them could not go on, a sprint was started or completed.
//!
//! An event is told once, when it happens, to whatever listens outside the
//! window: the notifications of the operating system today, a message to a
//! chat channel or a call to a web address later. Each listener decides for
//! itself what is worth passing on. An event carries the names that are
//! true at the moment it happened, so that a listener needs nothing else to
//! word it.
//!
//! The functions here are pure. The work of the specialists finds the event
//! of a turn when the turn ends ([`event_of_ended_turn`]); the shell finds
//! the events of a sprint after the manager started or completed one.

use teamwright_domain::{
    Conversation, ConversationId, Member, MemberId, Project, ProjectId, Sprint, SprintId,
    SprintUnit, Task, TaskId, Timestamp, Turn, TurnState,
};

use crate::work::reference_of;

/// Something that happened in a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEvent {
    /// The project it happened in.
    pub project_id: ProjectId,
    /// The name of the project when it happened.
    pub project_name: String,
    /// When it happened.
    pub at: Timestamp,
    /// What happened.
    pub happening: Happening,
}

/// What happened, with what a listener needs to say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Happening {
    /// A specialist finished its work on a task. The task waits to be
    /// checked.
    TaskFinished {
        /// The conversation in which the work was done.
        conversation_id: ConversationId,
        /// The specialist that did it.
        specialist: Agent,
        /// The task.
        task: TaskOfEvent,
        /// The specialist's report: its last words in the turn. May be
        /// empty.
        report: String,
    },
    /// A specialist or the Advisor ended its turn with something that is
    /// the manager's: a decision, questions, a proposal.
    WaitsForManager {
        /// The conversation that waits.
        conversation_id: ConversationId,
        /// Who waits.
        agent: Agent,
        /// The task the conversation is about, if it is about one.
        task: Option<TaskOfEvent>,
        /// What it waits for.
        waits_for: WaitsFor,
    },
    /// A specialist or the Advisor could not go on: its turn failed. A turn
    /// the manager stopped is not an event; the manager knows of it.
    AgentStopped {
        /// The conversation whose turn failed.
        conversation_id: ConversationId,
        /// Who stopped.
        agent: Agent,
        /// The task the conversation is about, if it is about one.
        task: Option<TaskOfEvent>,
        /// Why, in the words of the program.
        reason: String,
    },
    /// The manager started a sprint.
    SprintStarted {
        /// The sprint.
        sprint_id: SprintId,
        /// Its name.
        name: String,
        /// Its goal. May be empty.
        goal: String,
    },
    /// The manager completed a sprint.
    SprintCompleted {
        /// The sprint.
        sprint_id: SprintId,
        /// Its name.
        name: String,
        /// What its work was counted in.
        unit: SprintUnit,
        /// The work it took on.
        committed: u32,
        /// The work of it that was finished.
        completed: u32,
    },
}

/// Who an event is about: a specialist of the team, or the Advisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Agent {
    /// A specialist, with its name when the event happened. Nothing if it
    /// has left the team since the conversation started.
    Specialist {
        /// The specialist, if it is still on the team.
        member_id: Option<MemberId>,
        /// Its name, or "A specialist" if it has left.
        name: String,
    },
    /// The Advisor, which plans the project.
    Advisor,
}

impl Agent {
    /// The name of the agent, to start a sentence with.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Specialist { name, .. } => name,
            Self::Advisor => "The Advisor",
        }
    }
}

/// A task as an event names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOfEvent {
    /// The task.
    pub task_id: TaskId,
    /// Its reference, such as `SH-4`.
    pub reference: String,
    /// Its title.
    pub title: String,
}

impl TaskOfEvent {
    /// The task of a project, as an event names it.
    #[must_use]
    pub fn of(project: &Project, task: &Task) -> Self {
        Self {
            task_id: task.id(),
            reference: reference_of(project, task),
            title: task.title().as_str().to_owned(),
        }
    }
}

/// What a specialist or the Advisor waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitsFor {
    /// A decision: the question a specialist asked.
    Decision {
        /// The question.
        question: String,
    },
    /// Answers to the questions of the Advisor.
    Answers {
        /// The first question.
        first_question: String,
        /// How many questions it asked.
        questions: usize,
    },
    /// A word about the team and the plan the Advisor proposed.
    Approval {
        /// How many members it proposes.
        members: usize,
        /// How many tasks it proposes.
        tasks: usize,
    },
}

/// The event of a turn that has just ended, or nothing if its end is not
/// one: a turn that answered a question of the manager's, a turn the
/// manager stopped, a turn that is still running.
///
/// `member` is the specialist of the conversation, if it is still on the
/// team, and `task` the task the conversation is about, if it is about one.
#[must_use]
pub fn event_of_ended_turn(
    project: &Project,
    conversation: &Conversation,
    turn: &Turn,
    member: Option<&Member>,
    task: Option<&Task>,
) -> Option<ProjectEvent> {
    let at = turn.ended_at()?;
    let conversation_id = conversation.id();
    let agent = if conversation.is_with_advisor() {
        Agent::Advisor
    } else {
        Agent::Specialist {
            member_id: member.map(Member::id),
            name: member.map_or_else(
                || "A specialist".to_owned(),
                |member| member.name().as_str().to_owned(),
            ),
        }
    };
    let task = task
        .filter(|task| conversation.task_id() == Some(task.id()))
        .map(|task| TaskOfEvent::of(project, task));

    let happening = match turn.state() {
        TurnState::Running | TurnState::Stopped => return None,
        TurnState::Failed => Happening::AgentStopped {
            conversation_id,
            agent,
            task,
            reason: turn.failure().unwrap_or_default().trim().to_owned(),
        },
        TurnState::Finished => {
            if let Some(waits_for) = waits_for_of(turn) {
                Happening::WaitsForManager {
                    conversation_id,
                    agent,
                    task,
                    waits_for,
                }
            } else {
                // A finished turn in a conversation that is not about a
                // task answered something the manager asked, in the window.
                Happening::TaskFinished {
                    conversation_id,
                    specialist: agent,
                    task: task?,
                    report: turn.answer().to_owned(),
                }
            }
        }
    };
    Some(ProjectEvent {
        project_id: project.id(),
        project_name: project.name().as_str().to_owned(),
        at,
        happening,
    })
}

/// What a finished turn left for the manager, if anything.
fn waits_for_of(turn: &Turn) -> Option<WaitsFor> {
    if let Some(decision) = turn.decision() {
        return Some(WaitsFor::Decision {
            question: decision.question().to_owned(),
        });
    }
    if let Some(interview) = turn.interview() {
        let questions = interview.questions();
        return Some(WaitsFor::Answers {
            first_question: questions
                .first()
                .map(|question| question.question.clone())
                .unwrap_or_default(),
            questions: questions.len(),
        });
    }
    turn.proposal().map(|proposal| WaitsFor::Approval {
        members: proposal.members().len(),
        tasks: proposal.tasks().len(),
    })
}

/// The event of a sprint the manager has just started, or nothing if it
/// has not started.
#[must_use]
pub fn event_of_started_sprint(project: &Project, sprint: &Sprint) -> Option<ProjectEvent> {
    let at = sprint.started_at()?;
    Some(ProjectEvent {
        project_id: project.id(),
        project_name: project.name().as_str().to_owned(),
        at,
        happening: Happening::SprintStarted {
            sprint_id: sprint.id(),
            name: sprint.name().as_str().to_owned(),
            goal: sprint.goal().to_owned(),
        },
    })
}

/// The event of a sprint the manager has just completed, or nothing if it
/// has not been completed.
#[must_use]
pub fn event_of_completed_sprint(project: &Project, sprint: &Sprint) -> Option<ProjectEvent> {
    let at = sprint.completed_at()?;
    let summary = sprint.summary()?;
    Some(ProjectEvent {
        project_id: project.id(),
        project_name: project.name().as_str().to_owned(),
        at,
        happening: Happening::SprintCompleted {
            sprint_id: sprint.id(),
            name: sprint.name().as_str().to_owned(),
            unit: summary.unit,
            committed: summary.committed,
            completed: summary.completed,
        },
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{
        AreaId, Color, ComputerId, DecisionOption, DecisionRequest, Interview, InterviewQuestion,
        MemberKind, MemberName, ProjectKey, ProjectName, Rank, TaskDraft, TaskTitle, TokenCounts,
        TurnOrigin, Workflow,
    };

    use super::*;

    fn moment(minute: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 16, 9, minute, 0)
            .unwrap()
    }

    struct Setting {
        project: Project,
        engineer: Member,
        task: Task,
        conversation: Conversation,
    }

    fn setting() -> Setting {
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::engineering(),
            moment(0),
        );
        let engineer = Member::create(
            project.id(),
            MemberName::parse("Hardware engineer").unwrap(),
            MemberKind::Agent,
            String::new(),
            Color::Gray,
            moment(0),
        );
        let task = Task::create(
            project.id(),
            AreaId::generate(),
            4,
            project.workflow().initial_status(),
            Rank::between(None, None),
            TaskDraft::titled(TaskTitle::parse("Draw the schematic").unwrap()),
            moment(0),
        );
        let conversation = Conversation::start(
            project.id(),
            engineer.id(),
            Some(task.id()),
            moment(1),
            ComputerId::generate(),
        );
        Setting {
            project,
            engineer,
            task,
            conversation,
        }
    }

    fn running(conversation: &Conversation) -> Turn {
        Turn::start(
            conversation.id(),
            "Work on it",
            TurnOrigin::Brief,
            moment(1),
        )
        .unwrap()
    }

    fn finished(mut turn: Turn) -> Turn {
        turn.finish("The schematic is drawn.", TokenCounts::default(), moment(9))
            .unwrap();
        turn
    }

    impl Setting {
        fn event_of(&self, turn: &Turn) -> Option<ProjectEvent> {
            event_of_ended_turn(
                &self.project,
                &self.conversation,
                turn,
                Some(&self.engineer),
                Some(&self.task),
            )
        }

        fn agent(&self) -> Agent {
            Agent::Specialist {
                member_id: Some(self.engineer.id()),
                name: "Hardware engineer".to_owned(),
            }
        }

        fn task_of_event(&self) -> TaskOfEvent {
            TaskOfEvent {
                task_id: self.task.id(),
                reference: "SH-4".to_owned(),
                title: "Draw the schematic".to_owned(),
            }
        }
    }

    #[test]
    fn a_finished_turn_on_a_task_finishes_the_task() {
        let setting = setting();
        let event = setting
            .event_of(&finished(running(&setting.conversation)))
            .unwrap();
        assert_eq!(event.project_id, setting.project.id());
        assert_eq!(event.project_name, "Smart sensor node");
        assert_eq!(event.at, moment(9));
        assert_eq!(
            event.happening,
            Happening::TaskFinished {
                conversation_id: setting.conversation.id(),
                specialist: setting.agent(),
                task: setting.task_of_event(),
                report: "The schematic is drawn.".to_owned(),
            }
        );
    }

    #[test]
    fn a_turn_that_asks_waits_for_the_manager() {
        let setting = setting();
        let mut turn = running(&setting.conversation);
        let option = |label: &str| DecisionOption {
            label: label.to_owned(),
            consequence: String::new(),
        };
        turn.ask(
            DecisionRequest::new(
                "Which regulator?",
                "",
                vec![option("Linear"), option("Switching")],
                None,
            )
            .unwrap(),
        )
        .unwrap();
        let event = setting.event_of(&finished(turn)).unwrap();
        assert_eq!(
            event.happening,
            Happening::WaitsForManager {
                conversation_id: setting.conversation.id(),
                agent: setting.agent(),
                task: Some(setting.task_of_event()),
                waits_for: WaitsFor::Decision {
                    question: "Which regulator?".to_owned()
                },
            }
        );
    }

    #[test]
    fn the_advisor_waits_with_its_questions() {
        let setting = setting();
        let conversation = Conversation::start_with_advisor(
            setting.project.id(),
            None,
            None,
            moment(1),
            ComputerId::generate(),
        );
        let mut turn = running(&conversation);
        let question = |text: &str| InterviewQuestion {
            question: text.to_owned(),
            reason: String::new(),
            options: Vec::new(),
            recommended: None,
        };
        turn.ask_interview(
            Interview::new(vec![question("Who uses it?"), question("By when?")]).unwrap(),
        )
        .unwrap();
        let event =
            event_of_ended_turn(&setting.project, &conversation, &finished(turn), None, None)
                .unwrap();
        assert_eq!(
            event.happening,
            Happening::WaitsForManager {
                conversation_id: conversation.id(),
                agent: Agent::Advisor,
                task: None,
                waits_for: WaitsFor::Answers {
                    first_question: "Who uses it?".to_owned(),
                    questions: 2,
                },
            }
        );
    }

    #[test]
    fn a_failed_turn_stops_the_specialist_and_one_the_manager_stopped_is_no_event() {
        let setting = setting();
        let mut failed = running(&setting.conversation);
        failed
            .fail("The program ran out of time.".to_owned(), None, moment(5))
            .unwrap();
        assert_eq!(
            setting.event_of(&failed).unwrap().happening,
            Happening::AgentStopped {
                conversation_id: setting.conversation.id(),
                agent: setting.agent(),
                task: Some(setting.task_of_event()),
                reason: "The program ran out of time.".to_owned(),
            }
        );

        let mut stopped = running(&setting.conversation);
        stopped.stop(None, moment(5)).unwrap();
        assert_eq!(setting.event_of(&stopped), None);
        assert_eq!(setting.event_of(&running(&setting.conversation)), None);
    }

    #[test]
    fn an_answer_to_a_question_of_the_manager_is_no_event() {
        let setting = setting();
        let question = Conversation::start(
            setting.project.id(),
            setting.engineer.id(),
            None,
            moment(1),
            ComputerId::generate(),
        );
        let turn = finished(running(&question));
        assert_eq!(
            event_of_ended_turn(
                &setting.project,
                &question,
                &turn,
                Some(&setting.engineer),
                None
            ),
            None
        );
    }

    #[test]
    fn a_specialist_that_left_the_team_is_named_as_one() {
        let setting = setting();
        let event = event_of_ended_turn(
            &setting.project,
            &setting.conversation,
            &finished(running(&setting.conversation)),
            None,
            Some(&setting.task),
        )
        .unwrap();
        let Happening::TaskFinished { specialist, .. } = event.happening else {
            panic!("{event:?}");
        };
        assert_eq!(specialist.name(), "A specialist");
        assert_eq!(Agent::Advisor.name(), "The Advisor");
    }
}
