//! Planning with the Advisor, as `demoAdvisor.test.ts` describes it for the
//! demo of the interface: the same cases, under the same names, with the
//! same codes and messages. The demo's Advisor is scripted; here its turns
//! are played through the scripted runtime.

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{TimeDelta, TimeZone};
use teamwright_domain::{
    Color, ComputerId, ConversationId, DomainError, Effort, InterviewAnswer, MemberKind, Priority,
    Project, Proposal, ProposedOwner, Timestamp, TurnOrigin, TurnState,
};

use super::*;
use crate::agent_runtime::{
    AgentEvent, InterviewQuestionAsked, ManagerInterview, ModelUsage, PermissionProfile,
    PlanProposal, PlannedMember, PlannedOwner, PlannedTask, QuestionOption, SessionId,
    SessionStarted, TeamTool, TokenCounts, TurnOutcome,
};
use crate::in_memory::InMemorySprintRepository;
use crate::in_memory::{
    FixedProjectFolders, InMemoryConversationRepository, InMemoryMemberRepository,
    InMemoryPlacesRepository, InMemoryPlanningRepository, InMemoryProjectRepository,
    InMemoryTaskRepository, ManualClock, PretendFileSystem, RecordingWorkListener, ScriptedRuntime,
};
use crate::ports::Clock;
use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
use crate::tasks::{NewTask, TaskService};
use crate::team::{NewMember, TeamService};
use crate::work::{WorkPorts, WorkService};

const SESSION: &str = "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10";
const HARDWARE: &str = "Hardware engineer";
const FIRMWARE: &str = "Firmware engineer";
const MECHANICAL: &str = "Mechanical engineer";

/// The services on shared storage, a scripted runtime, and a project with
/// an engineering workflow: to do, in progress, in revision, in test, done.
struct Planning {
    projects: ProjectService,
    tasks: TaskService,
    team: TeamService,
    work: WorkService,
    runtime: Arc<ScriptedRuntime>,
    listener: Arc<RecordingWorkListener>,
    clock: Arc<ManualClock>,
    project: Project,
}

fn start_of_day() -> Timestamp {
    chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap()
}

fn planning() -> Planning {
    let projects_store = Arc::new(InMemoryProjectRepository::default());
    let tasks_store = Arc::new(InMemoryTaskRepository::default());
    let members_store = Arc::new(InMemoryMemberRepository::default());
    let conversations_store = Arc::new(InMemoryConversationRepository::default());
    let places = Arc::new(InMemoryPlacesRepository::default());
    let runtime = Arc::new(ScriptedRuntime::new());
    let listener = Arc::new(RecordingWorkListener::default());
    let clock = Arc::new(ManualClock::starting_at(start_of_day()));
    let projects = ProjectService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        places.clone(),
        Arc::new(InMemorySprintRepository::new(tasks_store.clone())),
        Arc::new(crate::in_memory::InMemoryViewRepository::default()),
        clock.clone(),
    );
    let tasks = TaskService::new(
        projects_store.clone(),
        tasks_store.clone(),
        members_store.clone(),
        conversations_store.clone(),
        Arc::new(InMemorySprintRepository::new(tasks_store.clone())),
        clock.clone(),
    );
    let team = TeamService::new(
        projects_store.clone(),
        members_store.clone(),
        tasks_store.clone(),
        conversations_store.clone(),
        Arc::new(crate::in_memory::InMemoryViewRepository::default()),
        clock.clone(),
    );
    let work = WorkService::new(WorkPorts {
        projects: projects_store.clone(),
        tasks: tasks_store.clone(),
        members: members_store.clone(),
        conversations: conversations_store.clone(),
        planning: Arc::new(InMemoryPlanningRepository::new(
            projects_store,
            members_store,
            tasks_store,
            conversations_store,
        )),
        folders: Arc::new(FixedProjectFolders::inside("projects")),
        places,
        file_system: Arc::new(PretendFileSystem::default()),
        runtime: runtime.clone(),
        listener: listener.clone(),
        clock: clock.clone(),
        this_computer: ComputerId::generate(),
    });
    let project = projects
        .create(NewProject {
            name: "Air quality node".to_owned(),
            workflow: WorkflowTemplate::Engineering,
            ..NewProject::default()
        })
        .unwrap();
    Planning {
        projects,
        tasks,
        team,
        work,
        runtime,
        listener,
        clock,
        project,
    }
}

/// The three questions the Advisor of the demo asks.
fn questions() -> ManagerInterview {
    let option = |label: &str, consequence: &str| QuestionOption {
        label: label.to_owned(),
        consequence: consequence.to_owned(),
    };
    ManagerInterview {
        questions: vec![
            InterviewQuestionAsked {
                question: "What matters most for the first version?".to_owned(),
                reason: "It decides where the team spends its care, and which tasks come first."
                    .to_owned(),
                options: vec![
                    option(
                        "A working prototype soon",
                        "The plan takes the shortest way.",
                    ),
                    option(
                        "The lowest cost for each unit",
                        "Parts are chosen by price.",
                    ),
                    option("The smallest size", "The board is designed tightly."),
                ],
                recommended: Some(0),
            },
            InterviewQuestionAsked {
                question: "Who does the work that needs hands?".to_owned(),
                reason: "A specialist cannot order parts, solder or print.".to_owned(),
                options: vec![
                    option("I do it myself", "Ordering and testing become your tasks."),
                    option("Leave it out of the plan", "The plan ends with files."),
                ],
                recommended: Some(0),
            },
            InterviewQuestionAsked {
                question: "Is anything already decided that must not change?".to_owned(),
                reason: "The team builds around what is fixed.".to_owned(),
                options: Vec::new(),
                recommended: None,
            },
        ],
    }
}

/// Three answers that fit the three questions.
fn answers() -> Vec<InterviewAnswer> {
    vec![
        InterviewAnswer {
            option: Some(0),
            note: String::new(),
        },
        InterviewAnswer {
            option: Some(1),
            note: "  I have no printer.  ".to_owned(),
        },
        InterviewAnswer {
            option: None,
            note: "The microcontroller is an ESP32-C3.".to_owned(),
        },
    ]
}

/// The team and the plan the Advisor of the demo proposes.
fn plan() -> PlanProposal {
    let member = |name: &str, responsibility: &str, model: Option<&str>, effort| PlannedMember {
        name: name.to_owned(),
        responsibility: responsibility.to_owned(),
        reason: format!("The project needs a {}.", name.to_lowercase()),
        model: model.map(str::to_owned),
        effort,
    };
    let task = |title: &str,
                description: &str,
                owner: Option<&str>,
                minutes,
                priority,
                waits: &[usize]| {
        PlannedTask {
            title: title.to_owned(),
            description: description.to_owned(),
            owner: owner.map_or(PlannedOwner::Manager, |name| {
                PlannedOwner::Member(name.to_owned())
            }),
            estimate_minutes: Some(minutes),
            priority,
            waits_for: waits.to_vec(),
        }
    };
    let normal = Priority::Normal;
    let high = Priority::High;
    PlanProposal {
        summary: "A small connected device that measures the air and shows it on a screen."
            .to_owned(),
        members: vec![
            member(
                HARDWARE,
                "The schematic, the parts and the board.",
                None,
                Some(Effort::High),
            ),
            member(
                FIRMWARE,
                "The software on the device.",
                Some("sonnet"),
                None,
            ),
            member(MECHANICAL, "The enclosure.", Some("sonnet"), None),
        ],
        tasks: vec![
            task(
                "Agree on what the device must do",
                "A short list.",
                None,
                120,
                high,
                &[],
            ),
            task(
                "Choose the microcontroller and the sensors",
                "A list of parts.",
                Some(HARDWARE),
                240,
                high,
                &[0],
            ),
            task(
                "Define the pin map",
                "Which pin goes to which part, with the addresses on the bus.",
                Some(HARDWARE),
                120,
                normal,
                &[1],
            ),
            task(
                "Draw the schematic",
                "The schematic.",
                Some(HARDWARE),
                480,
                normal,
                &[2],
            ),
            task(
                "Route the board",
                "The layout.",
                Some(HARDWARE),
                600,
                normal,
                &[3],
            ),
            task(
                "Define the outline of the board",
                "The size.",
                Some(HARDWARE),
                60,
                normal,
                &[4],
            ),
            task(
                "Write the sensor drivers",
                "Code that reads every sensor.",
                Some(FIRMWARE),
                480,
                normal,
                &[2],
            ),
            task(
                "Write the screens of the display",
                "What the screen shows.",
                Some(FIRMWARE),
                300,
                normal,
                &[6],
            ),
            task(
                "Write the Wi-Fi setup",
                "How the device joins a network.",
                Some(FIRMWARE),
                360,
                normal,
                &[6],
            ),
            task(
                "Design the enclosure",
                "The enclosure.",
                Some(MECHANICAL),
                480,
                normal,
                &[5],
            ),
            task(
                "Order the parts for three prototypes",
                "Ordered.",
                None,
                60,
                high,
                &[4],
            ),
            task(
                "Assemble and test the prototype",
                "One device.",
                None,
                360,
                normal,
                &[7, 8, 9, 10],
            ),
        ],
    }
}

/// A plan without its last member, its tasks and what waited for them.
fn without_last_member(plan: &PlanProposal) -> PlanProposal {
    let gone = plan.members.last().map(|member| member.name.to_lowercase());
    let stays: Vec<bool> = plan
        .tasks
        .iter()
        .map(|task| !matches!(&task.owner, PlannedOwner::Member(name) if Some(name.to_lowercase()) == gone))
        .collect();
    let mut new_place = HashMap::new();
    for (place, keeps) in stays.iter().enumerate() {
        if *keeps {
            new_place.insert(place, new_place.len());
        }
    }
    PlanProposal {
        summary: plan.summary.clone(),
        members: plan.members[..plan.members.len() - 1].to_vec(),
        tasks: plan
            .tasks
            .iter()
            .enumerate()
            .filter(|(place, _)| stays[*place])
            .map(|(_, task)| PlannedTask {
                waits_for: task
                    .waits_for
                    .iter()
                    .filter_map(|place| new_place.get(place).copied())
                    .collect(),
                ..task.clone()
            })
            .collect(),
    }
}

/// What the manager approves when nothing of a proposal is changed.
fn unchanged(proposal: &Proposal) -> ApprovedProposal {
    ApprovedProposal {
        members: proposal
            .members()
            .iter()
            .map(|member| ApprovedMember {
                name: member.name.to_string(),
                responsibility: member.responsibility.clone(),
                reason: member.reason.clone(),
                model: member.model.as_ref().map(ToString::to_string),
                effort: member.effort,
            })
            .collect(),
        tasks: proposal
            .tasks()
            .iter()
            .map(|task| ApprovedTask {
                title: task.title.to_string(),
                description: task.description.clone(),
                owner: task.owner.as_ref().map(|owner| match owner {
                    ProposedOwner::Manager => ApprovedOwner::Manager,
                    ProposedOwner::Member(name) => ApprovedOwner::Member(name.to_string()),
                }),
                estimate_minutes: task.estimate_minutes,
                priority: task.priority,
                waits_for: task.waits_for.clone(),
            })
            .collect(),
    }
}

fn ended(final_text: &str) -> AgentEvent {
    AgentEvent::TurnEnded(TurnOutcome {
        session_id: Some(SessionId::parse(SESSION).unwrap()),
        failure: None,
        final_text: final_text.to_owned(),
        usage: vec![ModelUsage {
            model: "claude-sample-model".to_owned(),
            tokens: TokenCounts {
                input: 24,
                output: 1180,
                cache_read: 14_000,
                cache_written: 8200,
            },
        }],
        estimated_cost_usd: None,
        duration: None,
        model_calls: None,
        permission_denials: Vec::new(),
    })
}

fn latest(conversation: &ConversationWithTurns) -> &Turn {
    conversation.turns.last().unwrap()
}

fn proposal_of(conversation: &ConversationWithTurns) -> &Proposal {
    latest(conversation).proposal().unwrap()
}

impl Planning {
    /// Delivers an event to a turn, counted from 0, a second later.
    fn deliver(&self, turn: usize, event: AgentEvent) {
        self.clock.advance(TimeDelta::seconds(1));
        self.runtime.deliver(turn, event);
    }

    /// Plays the latest turn that was started to its end: it asks or
    /// proposes, and ends.
    fn play(&self, event: Option<AgentEvent>, words: &str) {
        let turn = self.runtime.requests().len() - 1;
        self.deliver(
            turn,
            AgentEvent::SessionStarted(SessionStarted {
                session_id: SessionId::parse(SESSION).unwrap(),
                model: "claude-sample-model".to_owned(),
                working_folder: PathBuf::from("projects/AQN"),
                permission_mode: "acceptEdits".to_owned(),
                tools: Vec::new(),
                skills: Vec::new(),
                plugins: Vec::new(),
                mcp_servers: Vec::new(),
                program_version: Some("2.1.322".to_owned()),
            }),
        );
        if let Some(event) = event {
            self.deliver(turn, event);
        }
        self.deliver(turn, ended(words));
    }

    fn advise(&self, brief: &str) -> Result<ConversationWithTurns, ApplicationError> {
        self.work.advise(self.project.id(), brief, None, None)
    }

    /// A conversation as it is now.
    fn now(&self, id: ConversationId) -> ConversationWithTurns {
        self.work
            .list_for_project(self.project.id())
            .unwrap()
            .into_iter()
            .find(|listed| listed.conversation.id() == id)
            .unwrap()
    }

    /// Starts a conversation with the Advisor and lets its first turn end
    /// with the interview.
    fn interviewed(&self) -> ConversationWithTurns {
        let conversation = self.advise("A small air sensor.").unwrap();
        self.play(
            Some(AgentEvent::InterviewAsked(questions())),
            "I have asked three questions and wait for your answers.",
        );
        self.now(conversation.conversation.id())
    }

    /// Goes on until the Advisor has proposed.
    fn proposed(&self) -> ConversationWithTurns {
        let conversation = self.interviewed();
        self.work
            .answer_interview(conversation.conversation.id(), answers())
            .unwrap();
        self.play(
            Some(AgentEvent::PlanProposed(plan())),
            "I propose a team of three and twelve tasks.",
        );
        self.now(conversation.conversation.id())
    }

    fn waiting(&self) -> Vec<ConversationId> {
        self.work
            .waiting_for_manager()
            .unwrap()
            .iter()
            .map(|waiting| waiting.conversation.id())
            .collect()
    }

    fn members(&self) -> Vec<Member> {
        self.team.list_for_project(self.project.id()).unwrap()
    }

    fn tasks(&self) -> Vec<Task> {
        self.tasks.list_for_project(self.project.id()).unwrap()
    }

    fn approve(
        &self,
        conversation: &ConversationWithTurns,
        approved: ApprovedProposal,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        self.work
            .approve_proposal(conversation.conversation.id(), approved)
    }
}

fn refusal(result: Result<ConversationWithTurns, ApplicationError>) -> (String, String) {
    let error = result.unwrap_err();
    (error.code().to_owned(), error.to_string())
}

// Starting

#[test]
fn opens_a_conversation_with_the_advisor_that_begins_with_what_the_manager_wants() {
    let planning = planning();

    let conversation = planning.advise("  A small air sensor.  ").unwrap();

    assert_eq!(
        conversation.conversation.project_id(),
        planning.project.id()
    );
    assert!(conversation.conversation.is_with_advisor());
    assert_eq!(conversation.conversation.member_id(), None);
    assert_eq!(conversation.conversation.task_id(), None);
    assert_eq!(conversation.turns.len(), 1);
    let turn = latest(&conversation);
    assert_eq!(turn.request(), "A small air sensor.");
    assert_eq!(turn.origin(), TurnOrigin::Message);
    assert_eq!(turn.state(), TurnState::Running);
}

#[test]
fn lists_the_conversation_with_the_others_of_the_project() {
    let planning = planning();
    let conversation = planning.advise("A small air sensor.").unwrap();

    let listed = planning
        .work
        .list_for_project(planning.project.id())
        .unwrap();

    let ids: Vec<ConversationId> = listed.iter().map(|other| other.conversation.id()).collect();
    assert_eq!(ids, [conversation.conversation.id()]);
}

#[test]
fn refuses_an_empty_brief() {
    let planning = planning();

    let (code, message) = refusal(planning.advise("   "));

    assert_eq!(code, "rule_broken");
    assert_eq!(message, "the message must not be empty");
}

#[test]
fn refuses_a_model_whose_name_is_not_one_word() {
    let planning = planning();

    let (code, _) = refusal(planning.work.advise(
        planning.project.id(),
        "A small air sensor.",
        Some("two words"),
        None,
    ));

    assert_eq!(code, "rule_broken");
    assert!(
        planning
            .work
            .list_for_project(planning.project.id())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn refuses_a_project_that_does_not_exist() {
    let planning = planning();

    let (code, _) = refusal(planning.work.advise(
        ProjectId::generate(),
        "A small air sensor.",
        None,
        None,
    ));

    assert_eq!(code, "not_found");
}

#[test]
fn refuses_a_second_conversation_while_the_advisor_is_working_on_the_project() {
    let planning = planning();
    planning.advise("A small air sensor.").unwrap();

    let (code, message) = refusal(planning.advise("And a weather station."));

    assert_eq!(code, "advisor_busy");
    assert_eq!(message, "the Advisor is still working on this project");
}

#[test]
fn lets_the_advisor_work_on_two_projects_at_once() {
    let planning = planning();
    let other = planning
        .projects
        .create(NewProject {
            name: "Weather station".to_owned(),
            workflow: WorkflowTemplate::Standard,
            ..NewProject::default()
        })
        .unwrap();
    planning.advise("A small air sensor.").unwrap();

    let second = planning
        .work
        .advise(other.id(), "A weather station.", None, None)
        .unwrap();

    assert_eq!(latest(&second).state(), TurnState::Running);
}

#[test]
fn can_be_stopped_like_the_work_of_a_specialist() {
    let planning = planning();
    let conversation = planning.advise("A small air sensor.").unwrap();

    let stopped = planning.work.stop(conversation.conversation.id()).unwrap();

    assert_eq!(latest(&stopped).state(), TurnState::Stopped);
    assert_eq!(latest(&stopped).interview(), None);
    assert!(planning.waiting().is_empty());
}

#[test]
fn runs_in_the_work_folder_with_the_tools_of_the_advisor_on_the_model_of_the_conversation() {
    let planning = planning();
    planning
        .team
        .add(NewMember {
            project_id: planning.project.id(),
            name: HARDWARE.to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();

    planning
        .work
        .advise(
            planning.project.id(),
            "A small air sensor.",
            Some("opus"),
            Some(Effort::High),
        )
        .unwrap();

    let request = &planning.runtime.requests()[0];
    assert_eq!(
        request.working_folder,
        PathBuf::from("projects").join("AQN")
    );
    assert_eq!(request.permissions, PermissionProfile::advisor());
    assert_eq!(
        request.team_tools,
        [
            TeamTool::Interview,
            TeamTool::ProposePlan {
                team: vec![HARDWARE.to_owned()]
            }
        ]
    );
    assert_eq!(request.model.as_deref(), Some("opus"));
    assert_eq!(request.effort, Some(Effort::High));
    let instructions = request.extra_instructions.as_deref().unwrap();
    assert!(instructions.starts_with("You are the Advisor of the project \"Air quality node\""));
    assert!(instructions.contains("- Hardware engineer (AI specialist)"));
}

// The interview

#[test]
fn ends_the_first_turn_with_questions_some_with_options_one_for_words() {
    let planning = planning();

    let conversation = planning.interviewed();

    let turn = latest(&conversation);
    assert_eq!(turn.state(), TurnState::Finished);
    assert_eq!(turn.decision(), None);
    assert_eq!(turn.proposal(), None);
    let interview = turn.interview().unwrap();
    let counts: Vec<usize> = interview
        .questions()
        .iter()
        .map(|question| question.options.len())
        .collect();
    assert_eq!(counts, [3, 2, 0]);
    assert_eq!(interview.questions()[0].recommended, Some(0));
    assert_eq!(interview.questions()[2].recommended, None);
    assert_eq!(interview.answers(), None);
    assert_eq!(interview.answered_at(), None);
}

#[test]
fn counts_open_questions_among_what_waits_for_the_manager() {
    let planning = planning();
    let conversation = planning.interviewed();

    assert_eq!(planning.waiting(), [conversation.conversation.id()]);
}

#[test]
fn takes_one_answer_for_each_question_and_sends_the_advisor_on_with_them() {
    let planning = planning();
    let conversation = planning.interviewed();
    planning.clock.advance(TimeDelta::seconds(60));

    let answered = planning
        .work
        .answer_interview(conversation.conversation.id(), answers())
        .unwrap();

    assert_eq!(answered.turns.len(), 2);
    let interview = answered.turns[0].interview().unwrap();
    assert_eq!(
        interview.answers().unwrap(),
        [
            InterviewAnswer {
                option: Some(0),
                note: String::new()
            },
            InterviewAnswer {
                option: Some(1),
                note: "I have no printer.".to_owned()
            },
            InterviewAnswer {
                option: None,
                note: "The microcontroller is an ESP32-C3.".to_owned()
            },
        ]
    );
    assert_eq!(interview.answered_at(), Some(planning.clock.now()));
    assert_eq!(latest(&answered).origin(), TurnOrigin::Interview);
    assert_eq!(latest(&answered).state(), TurnState::Running);
    assert_eq!(
        latest(&answered).request(),
        [
            "The manager answered your questions.",
            "",
            "1. What matters most for the first version?",
            "   Answer: A working prototype soon",
            "",
            "2. Who does the work that needs hands?",
            "   Answer: Leave it out of the plan",
            "   The manager adds: I have no printer.",
            "",
            "3. Is anything already decided that must not change?",
            "   Answer: The microcontroller is an ESP32-C3.",
        ]
        .join("\n")
    );
    assert!(planning.waiting().is_empty());
}

#[test]
fn refuses_too_few_or_too_many_answers() {
    let planning = planning();
    let conversation = planning.interviewed();
    let mut more = answers();
    more.push(InterviewAnswer {
        option: None,
        note: "More.".to_owned(),
    });

    for given in [answers()[..2].to_vec(), more] {
        let (code, message) = refusal(
            planning
                .work
                .answer_interview(conversation.conversation.id(), given),
        );
        assert_eq!(code, "rule_broken");
        assert_eq!(message, "every question needs an answer");
    }
}

#[test]
fn refuses_an_option_that_a_question_does_not_have() {
    let planning = planning();
    let conversation = planning.interviewed();
    let mut given = answers();
    given[1] = InterviewAnswer {
        option: Some(2),
        note: String::new(),
    };
    given[2].note = "Nothing.".to_owned();

    let (code, message) = refusal(
        planning
            .work
            .answer_interview(conversation.conversation.id(), given),
    );

    assert_eq!(code, "rule_broken");
    assert_eq!(message, "question 2 has no such option");
}

#[test]
fn refuses_an_answer_that_names_no_option_and_says_nothing() {
    let planning = planning();
    let conversation = planning.interviewed();
    let given = vec![
        InterviewAnswer {
            option: Some(0),
            note: String::new(),
        },
        InterviewAnswer {
            option: Some(0),
            note: String::new(),
        },
        InterviewAnswer {
            option: None,
            note: "   ".to_owned(),
        },
    ];

    let (code, message) = refusal(
        planning
            .work
            .answer_interview(conversation.conversation.id(), given),
    );

    assert_eq!(code, "rule_broken");
    assert_eq!(
        message,
        "the answer to question 3 names one of the options or says something in words"
    );
    let now = planning.now(conversation.conversation.id());
    assert_eq!(latest(&now).interview().unwrap().answers(), None);
}

#[test]
fn refuses_an_ordinary_message_while_the_questions_are_open() {
    let planning = planning();
    let conversation = planning.interviewed();

    let (code, message) = refusal(
        planning
            .work
            .reply(conversation.conversation.id(), "Just propose something."),
    );

    assert_eq!(code, "interview_open");
    assert_eq!(
        message,
        "the Advisor waits for your answers: answer its questions first"
    );
}

#[test]
fn refuses_answers_where_nothing_was_asked_or_was_answered_already() {
    let planning = planning();
    let conversation = planning.interviewed();
    planning
        .work
        .answer_interview(conversation.conversation.id(), answers())
        .unwrap();
    planning.play(None, "I am thinking.");

    let (again_code, again_message) = refusal(
        planning
            .work
            .answer_interview(conversation.conversation.id(), answers()),
    );
    let (nowhere, _) = refusal(
        planning
            .work
            .answer_interview(ConversationId::generate(), answers()),
    );

    assert_eq!(again_code, "no_interview_open");
    assert_eq!(again_message, "no questions are open in this conversation");
    assert_eq!(nowhere, "not_found");
}

// The proposal

#[test]
fn ends_the_turn_after_the_answers_with_a_team_and_a_plan_that_are_not_approved() {
    let planning = planning();

    let conversation = planning.proposed();

    let turn = latest(&conversation);
    assert_eq!(turn.state(), TurnState::Finished);
    assert_eq!(turn.interview(), None);
    let proposal = proposal_of(&conversation);
    assert_eq!(proposal.approved_at(), None);
    assert!(proposal.summary().contains("measures the air"));
    let names: Vec<String> = proposal
        .members()
        .iter()
        .map(|member| member.name.to_string())
        .collect();
    assert_eq!(names, [HARDWARE, FIRMWARE, MECHANICAL]);
    assert_eq!(proposal.members()[0].model, None);
    assert_eq!(proposal.members()[0].effort, Some(Effort::High));
    assert_eq!(proposal.tasks().len(), 12);
    let first = &proposal.tasks()[0];
    assert_eq!(first.title.as_str(), "Agree on what the device must do");
    assert_eq!(first.owner, Some(ProposedOwner::Manager));
    assert_eq!(first.estimate_minutes, Some(120));
    assert_eq!(first.priority, Priority::High);
    assert!(first.waits_for.is_empty());
    assert_eq!(proposal.tasks()[11].waits_for, [7, 8, 9, 10]);
}

#[test]
fn creates_nothing_before_the_manager_approves() {
    let planning = planning();

    planning.proposed();

    assert!(planning.members().is_empty());
    assert!(planning.tasks().is_empty());
}

#[test]
fn counts_a_proposal_that_is_not_approved_among_what_waits_for_the_manager() {
    let planning = planning();
    let conversation = planning.proposed();

    assert_eq!(planning.waiting(), [conversation.conversation.id()]);
}

#[test]
fn adds_the_members_to_the_team_and_creates_the_tasks_when_it_is_approved() {
    let planning = planning();
    let conversation = planning.proposed();
    planning.clock.advance(TimeDelta::seconds(60));

    let approved = planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let team = planning.members();
    let described: Vec<(String, MemberKind, Color)> = team
        .iter()
        .map(|member| (member.name().to_string(), member.kind(), member.color()))
        .collect();
    assert_eq!(
        described,
        [
            (HARDWARE.to_owned(), MemberKind::Agent, Color::Teal),
            (FIRMWARE.to_owned(), MemberKind::Agent, Color::Blue),
            (MECHANICAL.to_owned(), MemberKind::Agent, Color::Purple),
        ]
    );
    assert_eq!(
        team[0].responsibility(),
        "The schematic, the parts and the board."
    );
    assert_eq!(team[0].model(), None);
    assert_eq!(team[0].effort(), Some(Effort::High));
    assert_eq!(
        team[1].model().map(ToString::to_string).as_deref(),
        Some("sonnet")
    );
    assert_eq!(team[1].effort(), None);

    let tasks = planning.tasks();
    let numbers: Vec<u32> = tasks.iter().map(Task::number).collect();
    assert_eq!(numbers, (1..=12).collect::<Vec<_>>());
    let first_status = planning.project.workflow().statuses()[0].id();
    assert!(tasks.iter().all(|task| task.status_id() == first_status));
    // The plan lands in the backlog, without points.
    assert!(
        tasks
            .iter()
            .all(|task| task.sprint_id().is_none() && task.points().is_none())
    );
    assert_eq!(
        tasks[0].title().as_str(),
        "Agree on what the device must do"
    );
    assert_eq!(tasks[0].assignee(), Some(Assignee::Manager));
    assert_eq!(tasks[0].estimate_minutes(), Some(120));
    assert_eq!(tasks[0].priority(), Priority::High);
    assert!(tasks[0].blocked_by().is_empty());
    assert_eq!(tasks[1].assignee(), Some(Assignee::Member(team[0].id())));
    assert_eq!(
        tasks[1].blocked_by().iter().copied().collect::<Vec<_>>(),
        [tasks[0].id()]
    );
    assert_eq!(tasks[9].assignee(), Some(Assignee::Member(team[2].id())));
    let mut last_waits_for: Vec<_> = tasks[11].blocked_by().iter().copied().collect();
    last_waits_for.sort();
    let mut expected = vec![tasks[7].id(), tasks[8].id(), tasks[9].id(), tasks[10].id()];
    expected.sort();
    assert_eq!(last_waits_for, expected);
    assert!(
        tasks[2]
            .description()
            .contains("Which pin goes to which part")
    );

    assert_eq!(
        proposal_of(&approved).approved_at(),
        Some(planning.clock.now())
    );
    assert!(planning.waiting().is_empty());
}

#[test]
fn tells_its_listeners_when_a_proposal_is_approved() {
    let planning = planning();
    let conversation = planning.proposed();
    planning.listener.forget();

    planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let heard: Vec<Option<Timestamp>> = planning
        .listener
        .changes()
        .iter()
        .map(|(_, turns)| {
            turns
                .last()
                .and_then(Turn::proposal)
                .and_then(Proposal::approved_at)
        })
        .collect();
    assert_eq!(heard, [Some(planning.clock.now())]);
}

#[test]
fn approves_what_the_manager_left_of_it_and_keeps_that_on_record() {
    let planning = planning();
    let conversation = planning.proposed();
    let smaller = without_last_member(&plan());
    let mut approved = unchanged(proposal_of(&conversation));
    approved.members.pop();
    approved.tasks = unchanged(&proposal_from(smaller).unwrap()).tasks;

    let recorded = planning.approve(&conversation, approved).unwrap();

    assert_eq!(planning.members().len(), 2);
    let tasks = planning.tasks();
    assert_eq!(tasks.len(), 11);
    // The last task waited for the enclosure too. That task is gone.
    let mut waits: Vec<_> = tasks[10].blocked_by().iter().copied().collect();
    waits.sort();
    let mut expected = vec![tasks[7].id(), tasks[8].id(), tasks[9].id()];
    expected.sort();
    assert_eq!(waits, expected);
    assert_eq!(proposal_of(&recorded).members().len(), 2);
    assert_eq!(proposal_of(&recorded).tasks().len(), 11);
}

#[test]
fn gives_a_task_to_the_member_the_team_already_has_and_does_not_add_that_member_again() {
    let planning = planning();
    let there = planning
        .team
        .add(NewMember {
            project_id: planning.project.id(),
            name: "hardware engineer".to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        })
        .unwrap();
    let conversation = planning.proposed();

    planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let names: Vec<String> = planning
        .members()
        .iter()
        .map(|member| member.name().to_string())
        .collect();
    assert_eq!(names, ["hardware engineer", FIRMWARE, MECHANICAL]);
    assert_eq!(
        planning.tasks()[1].assignee(),
        Some(Assignee::Member(there.id()))
    );
}

#[test]
fn adds_to_the_tasks_the_project_already_has() {
    let planning = planning();
    planning
        .tasks
        .create(NewTask::titled(
            planning.project.id(),
            "Something that was there",
        ))
        .unwrap();
    let conversation = planning.proposed();

    planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let tasks = planning.tasks();
    assert_eq!(tasks.len(), 13);
    assert_eq!(tasks[0].title().as_str(), "Something that was there");
    assert_eq!(tasks[12].number(), 13);
}

#[test]
fn refuses_two_members_of_the_same_name() {
    let planning = planning();
    let conversation = planning.proposed();
    let mut changed = unchanged(proposal_of(&conversation));
    changed.members[1].name = " hardware ENGINEER ".to_owned();

    let (code, message) = refusal(planning.approve(&conversation, changed));

    assert_eq!(code, "rule_broken");
    assert_eq!(
        message,
        "two members of the proposal are called \"hardware ENGINEER\""
    );
}

#[test]
fn refuses_a_task_that_is_given_to_somebody_who_is_not_there() {
    let planning = planning();
    let conversation = planning.proposed();
    let mut changed = unchanged(proposal_of(&conversation));
    changed.members.pop();

    let (code, message) = refusal(planning.approve(&conversation, changed));

    assert_eq!(code, "rule_broken");
    assert_eq!(
        message,
        "\"Design the enclosure\" is given to \"Mechanical engineer\", who is neither on the \
         team nor in the proposal"
    );
}

#[test]
fn refuses_a_task_that_waits_for_itself_or_for_a_task_that_is_not_there() {
    // A place that is not a whole number, or below 0, cannot reach the use
    // case: the shell reads such a place as one that is not there.
    let planning = planning();
    let conversation = planning.proposed();

    for waits_for in [vec![3], vec![12], vec![usize::MAX]] {
        let mut changed = unchanged(proposal_of(&conversation));
        changed.tasks[3].waits_for = waits_for;
        let (code, message) = refusal(planning.approve(&conversation, changed));
        assert_eq!(code, "rule_broken");
        assert_eq!(
            message,
            "\"Draw the schematic\" waits for a task that is not another task of the proposal"
        );
    }
}

#[test]
fn refuses_tasks_that_wait_for_each_other_in_a_circle() {
    let planning = planning();
    let conversation = planning.proposed();
    let mut changed = unchanged(proposal_of(&conversation));
    // The first task now waits for the last, which waits for it through others.
    changed.tasks[0].waits_for = vec![11];

    let (code, message) = refusal(planning.approve(&conversation, changed));

    assert_eq!(code, "rule_broken");
    assert_eq!(
        message,
        "the tasks of the proposal wait for each other in a circle"
    );
}

#[test]
fn refuses_a_task_without_a_title() {
    let planning = planning();
    let conversation = planning.proposed();
    let mut changed = unchanged(proposal_of(&conversation));
    changed.tasks[4].title = "   ".to_owned();

    let (code, _) = refusal(planning.approve(&conversation, changed));

    assert_eq!(code, "rule_broken");
}

#[test]
fn refuses_a_proposal_with_nothing_left_in_it() {
    let planning = planning();
    let conversation = planning.proposed();

    let (code, message) = refusal(planning.approve(&conversation, ApprovedProposal::default()));

    assert_eq!(code, "rule_broken");
    assert_eq!(message, "the proposal has no members and no tasks");
}

#[test]
fn creates_nothing_when_something_is_refused_and_leaves_the_proposal_open() {
    let planning = planning();
    let conversation = planning.proposed();
    let mut changed = unchanged(proposal_of(&conversation));
    changed.tasks[11].owner = Some(ApprovedOwner::Member("Nobody".to_owned()));

    refusal(planning.approve(&conversation, changed));

    assert!(planning.members().is_empty());
    assert!(planning.tasks().is_empty());
    let now = planning.now(conversation.conversation.id());
    assert_eq!(proposal_of(&now).approved_at(), None);
    assert_eq!(planning.waiting().len(), 1);
}

#[test]
fn refuses_to_approve_twice_and_where_nothing_was_proposed() {
    let planning = planning();
    let asked = planning.interviewed();
    let (nothing_yet_code, nothing_yet_message) =
        refusal(planning.approve(&asked, ApprovedProposal::default()));
    planning
        .work
        .answer_interview(asked.conversation.id(), answers())
        .unwrap();
    planning.play(Some(AgentEvent::PlanProposed(plan())), "I propose.");
    let approved = unchanged(proposal_of(&planning.now(asked.conversation.id())));
    planning.approve(&asked, approved.clone()).unwrap();

    let (twice, _) = refusal(planning.approve(&asked, approved));

    assert_eq!(nothing_yet_code, "no_proposal_open");
    assert_eq!(
        nothing_yet_message,
        "no proposal is open in this conversation"
    );
    assert_eq!(twice, "no_proposal_open");
    assert_eq!(planning.tasks().len(), 12);
}

#[test]
fn takes_a_message_of_the_manager_as_a_wish_for_a_change_and_proposes_again() {
    let planning = planning();
    let conversation = planning.proposed();
    let first = proposal_of(&conversation).clone();

    planning
        .work
        .reply(
            conversation.conversation.id(),
            "  Two specialists are enough.  ",
        )
        .unwrap();
    planning.play(
        Some(AgentEvent::PlanProposed(without_last_member(&plan()))),
        "Here is a smaller team.",
    );

    let changed = planning.now(conversation.conversation.id());
    assert_eq!(changed.turns.len(), 3);
    let turn = latest(&changed);
    assert_eq!(turn.request(), "Two specialists are enough.");
    assert_eq!(turn.origin(), TurnOrigin::Message);
    assert_eq!(turn.state(), TurnState::Finished);
    assert_eq!(proposal_of(&changed).members().len(), 2);
    // The proposal before stays on record as it was, and is not open any more.
    assert_eq!(changed.turns[1].proposal(), Some(&first));
    let approved = planning
        .approve(&changed, unchanged(proposal_of(&changed)))
        .unwrap();
    assert_eq!(planning.members().len(), 2);
    assert_eq!(
        approved.turns[1].proposal().and_then(Proposal::approved_at),
        None
    );
}

#[test]
fn lets_the_manager_start_another_round_after_a_proposal_was_approved() {
    let planning = planning();
    let conversation = planning.proposed();
    planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let again = planning
        .work
        .reply(conversation.conversation.id(), "Add a battery.")
        .unwrap();

    assert_eq!(latest(&again).state(), TurnState::Running);
}

#[test]
fn tells_the_advisor_that_its_proposal_was_approved_when_the_manager_writes_again() {
    let planning = planning();
    let conversation = planning.proposed();
    planning
        .approve(&conversation, unchanged(proposal_of(&conversation)))
        .unwrap();

    let again = planning
        .work
        .reply(conversation.conversation.id(), "Add a battery.")
        .unwrap();

    // The turn keeps the words of the manager. The Advisor reads more.
    assert_eq!(latest(&again).request(), "Add a battery.");
    let requests = planning.runtime.requests();
    let prompt = &requests.last().unwrap().prompt;
    assert!(prompt.starts_with("The manager approved your proposal"));
    assert!(prompt.contains("propose only what is to be added"));
    assert!(prompt.ends_with("The manager writes:\nAdd a battery."));
}

#[test]
fn sends_the_advisor_the_words_of_the_manager_alone_while_its_proposal_is_open() {
    let planning = planning();
    let conversation = planning.proposed();

    planning
        .work
        .reply(conversation.conversation.id(), "Leave out the enclosure.")
        .unwrap();

    let requests = planning.runtime.requests();
    assert_eq!(requests.last().unwrap().prompt, "Leave out the enclosure.");
}

// What happens to the record of the planning

#[test]
fn keeps_a_project_while_the_advisor_is_working_on_it() {
    let planning = planning();
    planning.advise("A small air sensor.").unwrap();

    let error = planning.projects.delete(planning.project.id()).unwrap_err();

    assert_eq!(error.code(), "work_in_progress");
    assert_eq!(
        error.to_string(),
        "The Advisor is still working on a plan; stop the work first"
    );
}

#[test]
fn goes_with_a_project_that_is_deleted() {
    let planning = planning();
    planning.proposed();

    planning.projects.delete(planning.project.id()).unwrap();

    assert!(planning.waiting().is_empty());
}

#[test]
fn does_not_wait_for_the_manager_in_a_project_that_is_put_away() {
    let planning = planning();
    planning.proposed();

    planning.projects.archive(planning.project.id()).unwrap();

    assert!(planning.waiting().is_empty());
}

// The answers in words and the instructions

#[test]
fn gives_the_words_of_the_manager_where_no_option_was_chosen() {
    let words = answers_in_words(
        &[InterviewQuestion {
            question: "What is fixed?".to_owned(),
            reason: String::new(),
            options: Vec::new(),
            recommended: None,
        }],
        &[InterviewAnswer {
            option: None,
            note: "Nothing.".to_owned(),
        }],
    );

    assert_eq!(
        words,
        "The manager answered your questions.\n\n1. What is fixed?\n   Answer: Nothing."
    );
}

#[test]
fn the_instructions_of_the_advisor_name_the_project_its_material_and_the_rules() {
    let planning = planning();

    let instructions = instructions_for_advisor(
        &planning.project,
        &[],
        &[],
        &[PathBuf::from("/sample/datasheets")],
    );

    assert!(instructions.contains("the project \"Air quality node\""));
    assert!(
        instructions
            .contains("Write their paths exactly as they stand here:\n    /sample/datasheets\n")
    );
    assert!(!instructions.contains("What the project has already"));
    assert!(instructions.contains(
        "- Do not ask what a specialist will work out as part of its task. Such a choice becomes \
         a task of the plan, and if it is the manager's to make, a task for the manager to \
         approve it."
    ));
    assert!(instructions.contains(
        "- Do not write the word \"recommended\" into an option. Name the option with \
         recommended."
    ));
}

#[test]
fn the_instructions_of_the_advisor_say_who_and_what_the_project_has_already() {
    let planning = planning();
    let engineer = planning
        .team
        .add(NewMember {
            project_id: planning.project.id(),
            name: HARDWARE.to_owned(),
            kind: MemberKind::Agent,
            responsibility: "The board.".to_owned(),
            model: None,
            effort: None,
        })
        .unwrap();
    let task = planning
        .tasks
        .create(NewTask {
            assignee: Some(Assignee::Member(engineer.id())),
            ..NewTask::titled(planning.project.id(), "Draw the schematic")
        })
        .unwrap();

    let instructions = instructions_for_advisor(&planning.project, &[engineer], &[task], &[]);

    assert!(instructions.contains("Propose only what is to be added"));
    assert!(instructions.contains("- Hardware engineer (AI specialist): The board."));
    assert!(
        instructions.contains("- AQN-1: Draw the schematic (To do, given to Hardware engineer)")
    );
}

/// Converts a plan of the runtime into the proposal the domain keeps.
fn proposal_from(plan: PlanProposal) -> Result<Proposal, DomainError> {
    super::super::proposal_from(plan)
}
