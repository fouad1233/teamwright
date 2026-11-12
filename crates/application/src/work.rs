//! The work of AI specialists: starting it on a task, asking a specialist
//! something, following what it does, going on with it, and stopping it.
//! And the work of the Advisor, which plans a project with the manager
//! ([`advisor`]).
//!
//! A specialist is a session of the agent program ([`AgentRuntime`]). Every
//! request to it is a turn of a [`Conversation`]. The runtime reports what
//! happens during a turn as events, on a thread of its own; this module
//! records each event in the turn, stores the turn, and tells a
//! [`WorkListener`].

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use teamwright_domain::{
    Assignee, ComputerId, Conversation, ConversationId, DecisionOption, DecisionRequest,
    DomainError, Interview, InterviewQuestion, Member, MemberId, MemberKind, MemberName, ModelName,
    Project, ProjectId, Proposal, ProposedMember, ProposedOwner, ProposedTask, Rank,
    StatusCategory, Task, TaskId, TaskTitle, Timestamp, Turn, TurnId, TurnOrigin, TurnState,
};

use crate::agent_runtime::{
    self, AgentEvent, AgentRuntime, AgentRuntimeError, ManagerInterview, ManagerQuestion,
    PermissionProfile, PlanProposal, PlannedOwner, ProgramVersion, RunningTurn, SessionChoice,
    SessionId, SignIn, TeamTool, TurnFailure, TurnOutcome, TurnRequest,
};
use crate::error::ApplicationError;
use crate::places::{PlacesService, REFERENCES_FOLDER};
use crate::ports::{
    Clock, ConversationRepository, FileSystem, MemberRepository, PlacesRepository,
    PlanningRepository, ProjectFolders, ProjectRepository, TaskRepository, WorkListener,
};

mod advisor;

pub use advisor::{
    ApprovedMember, ApprovedOwner, ApprovedProposal, ApprovedTask, answers_in_words,
    instructions_for_advisor,
};

/// The failure text of a turn whose program said the user is not signed in.
const NOT_SIGNED_IN: &str =
    "Claude Code is not signed in. Open a terminal, run \"claude auth login\", and sign in.";

/// The failure text of a turn the program reported as failed without saying
/// why.
const COULD_NOT_FINISH: &str = "The specialist could not finish.";

/// The failure text of a turn that was still running when Teamwright was
/// closed.
const CLOSED_WHILE_WORKING: &str = "Teamwright was closed while the specialist was working.";

/// What a conversation without a task is about, in messages.
const A_QUESTION: &str = "a question";

/// Who a specialist that has left the team is, in messages.
const A_SPECIALIST: &str = "A specialist";

/// Who the Advisor is, in messages.
const THE_ADVISOR: &str = "The Advisor";

/// What the Advisor works on, in messages.
const A_PLAN: &str = "a plan";

/// Whether the program that runs the specialists can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentProgramState {
    /// It is installed and the user is signed in to it.
    Ready,
    /// It was not found.
    NotInstalled,
    /// It is installed, and the user has to sign in to it.
    SignedOut,
    /// It is there but could not be asked.
    Unavailable,
}

impl AgentProgramState {
    /// The stable text form of the state.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotInstalled => "not_installed",
            Self::SignedOut => "signed_out",
            Self::Unavailable => "unavailable",
        }
    }
}

/// What is known about the program that runs the specialists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProgram {
    /// Whether it can be used.
    pub state: AgentProgramState,
    /// Its version, if it is installed.
    pub version: Option<ProgramVersion>,
    /// How the user is signed in, as the program names it.
    pub sign_in_method: Option<String>,
    /// The kind of subscription, if the program says.
    pub subscription: Option<String>,
    /// What went wrong, in words, when the state is
    /// [`AgentProgramState::Unavailable`].
    pub problem: Option<String>,
}

impl AgentProgram {
    /// The program, when asking it failed.
    fn unusable(error: &AgentRuntimeError, version: Option<ProgramVersion>) -> Self {
        let (state, problem) = match error {
            AgentRuntimeError::NotInstalled => (AgentProgramState::NotInstalled, None),
            other => (AgentProgramState::Unavailable, Some(other.to_string())),
        };
        Self {
            state,
            version,
            sign_in_method: None,
            subscription: None,
            problem,
        }
    }
}

/// A conversation together with all of its turns, the oldest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationWithTurns {
    /// The conversation.
    pub conversation: Conversation,
    /// Its turns, the oldest first.
    pub turns: Vec<Turn>,
}

/// Everything the work of the specialists needs from the outside world.
pub struct WorkPorts {
    /// Where projects are stored.
    pub projects: Arc<dyn ProjectRepository>,
    /// Where tasks are stored.
    pub tasks: Arc<dyn TaskRepository>,
    /// Where the members of the teams are stored.
    pub members: Arc<dyn MemberRepository>,
    /// Where conversations and their turns are stored.
    pub conversations: Arc<dyn ConversationRepository>,
    /// Where what an approved proposal creates is stored, in one write.
    pub planning: Arc<dyn PlanningRepository>,
    /// The folder Teamwright gives each project.
    pub folders: Arc<dyn ProjectFolders>,
    /// Where the folders the manager chose for each project are stored.
    pub places: Arc<dyn PlacesRepository>,
    /// The files and folders of the computer.
    pub file_system: Arc<dyn FileSystem>,
    /// The program that runs the specialists.
    pub runtime: Arc<dyn AgentRuntime>,
    /// Who is told about every change to a conversation.
    pub listener: Arc<dyn WorkListener>,
    /// The time.
    pub clock: Arc<dyn Clock>,
    /// The identity of this computer. A conversation started here has it as
    /// its home, and only the turns of conversations at home here are run,
    /// answered, stopped or ended here.
    pub this_computer: ComputerId,
}

/// The use cases of the work of AI specialists.
///
/// The service keeps its parts in an [`Arc`], so that the listener it hands
/// to the runtime for each turn can reach them from the runtime's thread.
#[derive(Clone)]
pub struct WorkService {
    inner: Arc<Inner>,
}

/// The parts of the service, shared with the runtime's threads.
struct Inner {
    ports: WorkPorts,
    /// The folders of the projects: where a turn works, and what it may
    /// read besides.
    places: PlacesService,
    /// The turns this program has started and that have not ended yet, so
    /// that they can be stopped. Whether a specialist is busy is read from
    /// storage instead, which also knows the turns of earlier runs.
    running: Mutex<HashMap<TurnId, Box<dyn RunningTurn>>>,
    /// Held while the conversations are read and changed: by a use case
    /// that starts or stops a turn, and by the handling of one event. Each
    /// of them reads a turn, changes it and stores it, and none of them may
    /// see the others half done.
    work: Mutex<()>,
}

impl WorkService {
    /// Creates the service.
    #[must_use]
    pub fn new(ports: WorkPorts) -> Self {
        let places = PlacesService::new(
            Arc::clone(&ports.projects),
            Arc::clone(&ports.tasks),
            Arc::clone(&ports.members),
            Arc::clone(&ports.conversations),
            Arc::clone(&ports.places),
            Arc::clone(&ports.folders),
            Arc::clone(&ports.file_system),
        );
        Self {
            inner: Arc::new(Inner {
                ports,
                places,
                running: Mutex::new(HashMap::new()),
                work: Mutex::new(()),
            }),
        }
    }

    /// Says whether the program that runs the specialists can be used. It
    /// never fails: what went wrong is part of the answer.
    #[must_use]
    pub fn program(&self) -> AgentProgram {
        let runtime = &self.inner.ports.runtime;
        let version = match runtime.installation() {
            Ok(installation) => installation.version,
            Err(error) => return AgentProgram::unusable(&error, None),
        };
        match runtime.sign_in() {
            Ok(SignIn::SignedIn {
                method,
                subscription,
                ..
            }) => AgentProgram {
                state: AgentProgramState::Ready,
                version: Some(version),
                sign_in_method: Some(method),
                subscription,
                problem: None,
            },
            Ok(SignIn::SignedOut) => AgentProgram {
                state: AgentProgramState::SignedOut,
                version: Some(version),
                sign_in_method: None,
                subscription: None,
                problem: None,
            },
            Err(error) => AgentProgram::unusable(&error, Some(version)),
        }
    }

    /// Lists the conversations of a project with their turns, the oldest
    /// conversation first.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<ConversationWithTurns>, ApplicationError> {
        let ports = &self.inner.ports;
        let project = self.inner.require_project(project_id)?;
        ports
            .conversations
            .list_for_project(project.id())?
            .into_iter()
            .map(|conversation| {
                let turns = ports.conversations.turns_of(conversation.id())?;
                Ok(ConversationWithTurns {
                    conversation,
                    turns,
                })
            })
            .collect()
    }

    /// Starts the work on a task: opens a conversation with the specialist
    /// the task is given to, and sends it the brief of the task.
    ///
    /// A task that waits for work, or for a check, moves to the first status
    /// in which work is under way.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if it is not given to an AI
    /// specialist, if work on it has started before, if the specialist is
    /// working on something else, if the program cannot start the turn, or
    /// if storage fails.
    pub fn start_work(&self, task_id: TaskId) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let task = self.inner.require_task(task_id)?;
        let project = self.inner.require_project(task.project_id())?;
        let member = match task.assignee() {
            Some(Assignee::Member(member_id)) => ports.members.find(member_id)?,
            Some(Assignee::Manager) | None => None,
        }
        .filter(|member| member.kind() == MemberKind::Agent)
        .ok_or(ApplicationError::TaskNotGivenToSpecialist)?;

        let already_started = ports
            .conversations
            .list_for_project(project.id())?
            .iter()
            .any(|conversation| conversation.task_id() == Some(task.id()));
        if already_started {
            return Err(ApplicationError::WorkAlreadyStarted {
                task: reference_of(&project, &task),
            });
        }
        self.inner.guard().refuse_while_busy(&member)?;

        let now = ports.clock.now();
        let brief = brief_for(
            &project,
            &task,
            &ports.tasks.list_for_project(project.id())?,
        );
        let conversation = Conversation::start(
            project.id(),
            member.id(),
            Some(task.id()),
            now,
            ports.this_computer,
        );
        let turn = Turn::start(conversation.id(), &brief, TurnOrigin::Brief, now)?;
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: true,
                earlier_turns: Vec::new(),
                answered_turn: None,
                turn,
                prompt: None,
            },
            Worker::Specialist(&member),
            &project,
        )
    }

    /// Goes on with a conversation: the specialist gets the manager's
    /// message in the same session.
    ///
    /// A task the conversation is about that waits for work, or for a check,
    /// moves to the first status in which work is under way.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if its specialist
    /// has left the team, if the message is empty, if the specialist is
    /// working on something, if it waits for the answer to a question
    /// (`decision_open`), if the program cannot start the turn, or if
    /// storage fails.
    ///
    /// In a conversation with the Advisor, the message goes to the
    /// Advisor: a wish for a change of its proposal, or another round of
    /// planning. It is refused while the Advisor waits for the answers to
    /// its questions (`interview_open`) and while it works on the project
    /// (`advisor_busy`).
    ///
    /// A conversation that runs on another computer is refused
    /// (`runs_elsewhere`): its session is there.
    pub fn reply(
        &self,
        conversation_id: ConversationId,
        message: &str,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        if conversation.is_with_advisor() {
            return self.reply_to_advisor(conversation, message);
        }
        let member = self.inner.specialist_of(&conversation)?;
        let turn = Turn::start(
            conversation.id(),
            message,
            TurnOrigin::Message,
            ports.clock.now(),
        )?;
        self.inner.guard().refuse_while_busy(&member)?;
        let earlier_turns = ports.conversations.turns_of(conversation.id())?;
        if open_decision_of(&earlier_turns).is_some() {
            return Err(ApplicationError::DecisionOpen {
                name: member.name().to_string(),
            });
        }

        let project = self.inner.require_project(conversation.project_id())?;
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: false,
                earlier_turns,
                answered_turn: None,
                turn,
                prompt: None,
            },
            Worker::Specialist(&member),
            &project,
        )
    }

    /// Answers the question the specialist of a conversation asked: with
    /// one of its options (counted from 0), with words of the manager's
    /// own, or with both. The specialist goes on in the same session, with
    /// the answer as the request of a new turn.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if no question is
    /// open in it (`no_decision_open`), if its specialist has left the team,
    /// if the answer does not fit the question (`rule_broken`), if the
    /// specialist is working on something, if the conversation runs on
    /// another computer (`runs_elsewhere`), if the program cannot start the
    /// turn, or if storage fails.
    pub fn answer_decision(
        &self,
        conversation_id: ConversationId,
        option: Option<usize>,
        note: &str,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        let mut earlier_turns = ports.conversations.turns_of(conversation.id())?;
        if open_decision_of(&earlier_turns).is_none() {
            return Err(ApplicationError::NoDecisionOpen);
        }
        let member = self.inner.specialist_of(&conversation)?;
        let now = ports.clock.now();
        let asked = earlier_turns
            .last_mut()
            .ok_or(ApplicationError::NoDecisionOpen)?;
        asked.answer_decision(option, note, now)?;
        self.inner.guard().refuse_while_busy(&member)?;

        let words = asked
            .decision()
            .and_then(|decision| {
                let answer = decision.answer()?;
                let chosen = answer
                    .option
                    .and_then(|place| decision.options().get(place));
                Some(answer_in_words(
                    chosen.map(|option| option.label.as_str()),
                    &answer.note,
                ))
            })
            .ok_or(ApplicationError::NoDecisionOpen)?;
        let answered_turn = asked.clone();
        let turn = Turn::start(conversation.id(), &words, TurnOrigin::Decision, now)?;
        let project = self.inner.require_project(conversation.project_id())?;
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: false,
                earlier_turns,
                answered_turn: Some(answered_turn),
                turn,
                prompt: None,
            },
            Worker::Specialist(&member),
            &project,
        )
    }

    /// Lists the conversations that wait for the manager, in every project
    /// that is not archived: a specialist waits for a decision, or the
    /// Advisor waits for the answers to its questions or for a word about
    /// its proposal. The one that has waited longest, by the end of the
    /// turn that asked, comes first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn waiting_for_manager(&self) -> Result<Vec<ConversationWithTurns>, ApplicationError> {
        let ports = &self.inner.ports;
        let mut waiting = Vec::new();
        for project in ports.projects.list()? {
            if project.is_archived() {
                continue;
            }
            for conversation in ports.conversations.list_for_project(project.id())? {
                let turns = ports.conversations.turns_of(conversation.id())?;
                let waits = open_decision_of(&turns).is_some()
                    || open_interview_of(&turns).is_some()
                    || open_proposal_of(&turns).is_some();
                if waits {
                    waiting.push(ConversationWithTurns {
                        conversation,
                        turns,
                    });
                }
            }
        }
        waiting.sort_by_key(asked_at);
        Ok(waiting)
    }

    /// Asks a specialist something: opens a conversation that is not about
    /// a task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member, if it is not an AI
    /// specialist, if the message is empty, if the specialist is working on
    /// something, if the program cannot start the turn, or if storage fails.
    pub fn ask(
        &self,
        member_id: MemberId,
        message: &str,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let member = self.inner.require_member(member_id)?;
        if member.kind() != MemberKind::Agent {
            return Err(ApplicationError::NotASpecialist {
                name: member.name().to_string(),
            });
        }
        let now = ports.clock.now();
        let conversation = Conversation::start(
            member.project_id(),
            member.id(),
            None,
            now,
            ports.this_computer,
        );
        let turn = Turn::start(conversation.id(), message, TurnOrigin::Message, now)?;
        self.inner.guard().refuse_while_busy(&member)?;

        let project = self.inner.require_project(member.project_id())?;
        self.begin_turn(
            NewTurn {
                conversation,
                is_new_conversation: true,
                earlier_turns: Vec::new(),
                answered_turn: None,
                turn,
                prompt: None,
            },
            Worker::Specialist(&member),
            &project,
        )
    }

    /// Stops the turn that is running in a conversation. It ends as stopped
    /// by the manager, and a task it was about stays where it is. Stopping a
    /// conversation in which nothing runs changes nothing.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if it runs on
    /// another computer (`runs_elsewhere`), or if storage fails.
    pub fn stop(
        &self,
        conversation_id: ConversationId,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let _work = self.inner.hold_work();
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        let mut turns = ports.conversations.turns_of(conversation.id())?;

        if let Some(turn) = turns.iter_mut().find(|turn| turn.is_running()) {
            turn.stop(None, ports.clock.now())?;
            ports.conversations.save_turn(turn)?;
            // The turn is recorded as stopped before the program is told,
            // so the event the program sends when it has ended finds the
            // turn ended and changes nothing.
            if let Some(handle) = lock(&self.inner.running).remove(&turn.id()) {
                handle.stop();
            }
            ports.listener.conversation_changed(&conversation, &turns);
        }
        Ok(ConversationWithTurns {
            conversation,
            turns,
        })
    }

    /// Opens the session of a conversation in the agent program's desktop
    /// app, so that the manager can look at it there.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such conversation, if it runs on
    /// another computer (`runs_elsewhere`), if the program has not started
    /// its session yet, if the program cannot open it, or if storage fails.
    pub fn open_in_desktop_app(
        &self,
        conversation_id: ConversationId,
    ) -> Result<(), ApplicationError> {
        let ports = &self.inner.ports;
        let conversation = self.inner.require_conversation_here(conversation_id)?;
        let session = conversation
            .session_id()
            .ok_or(ApplicationError::SessionNotStarted)?;
        let session = SessionId::parse(session)?;
        let project = self.inner.require_project(conversation.project_id())?;
        let folder = self.inner.places.work_folder_of(&project)?;
        ports.runtime.open_in_desktop_app(&session, &folder)?;
        Ok(())
    }

    /// Finds a conversation with its turns, the oldest first, if it is
    /// stored.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn conversation(
        &self,
        id: ConversationId,
    ) -> Result<Option<ConversationWithTurns>, ApplicationError> {
        let ports = &self.inner.ports;
        let Some(conversation) = ports.conversations.find(id)? else {
            return Ok(None);
        };
        let turns = ports.conversations.turns_of(id)?;
        Ok(Some(ConversationWithTurns {
            conversation,
            turns,
        }))
    }

    /// Stops the programs of turns that another computer deleted while
    /// they ran here: their records are gone, so nobody would follow what
    /// they do. A turn this run of the program does not run is passed over.
    pub fn stop_turns(&self, turns: &[TurnId]) {
        let stopped: Vec<_> = {
            let mut running = lock(&self.inner.running);
            turns.iter().filter_map(|id| running.remove(id)).collect()
        };
        for handle in stopped {
            handle.stop();
        }
    }

    /// Ends the turns that storage still calls running although nothing
    /// runs them: they were cut off when Teamwright was closed. Each ends
    /// as failed, with a text that says so. Call it once, when the program
    /// starts.
    ///
    /// A turn of a conversation whose home is another computer is left
    /// alone: that computer runs it, and only it ends it.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn recover(&self) -> Result<(), ApplicationError> {
        let _work = self.inner.hold_work();
        self.inner.end_turns_nothing_runs()
    }

    /// Stops every turn that is running here, because Teamwright is
    /// closing: a specialist must not go on working with nobody to follow
    /// it. Each turn ends as failed, with a text that says so. Call it once,
    /// when the program closes. The turns of other computers go on.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails. The specialists are stopped even
    /// then, and [`Self::recover`] ends their turns at the next start.
    pub fn close(&self) -> Result<(), ApplicationError> {
        let _work = self.inner.hold_work();
        // The programs are stopped before anything is recorded, so that
        // they stop whatever happens to the record. What they still say
        // waits for the lock on the work and then finds its turn ended.
        let stopped: Vec<_> = lock(&self.inner.running).drain().collect();
        for (_, handle) in &stopped {
            handle.stop();
        }
        self.inner.end_turns_nothing_runs()
    }

    /// Starts a turn on the agent program and stores it.
    ///
    /// The program is asked first, so that nothing is stored if it cannot
    /// start. Its events wait for the lock on the work, which the caller
    /// holds, so they find the turn stored.
    fn begin_turn(
        &self,
        new_turn: NewTurn,
        worker: Worker<'_>,
        project: &Project,
    ) -> Result<ConversationWithTurns, ApplicationError> {
        let ports = &self.inner.ports;
        let NewTurn {
            conversation,
            is_new_conversation,
            earlier_turns: mut turns,
            answered_turn,
            turn,
            prompt,
        } = new_turn;

        let session = match conversation.session_id() {
            None => SessionChoice::New,
            Some(session_id) => SessionChoice::Resume(SessionId::parse(session_id)?),
        };
        // A reference folder that is gone from the disk is left out, so
        // that the turn can still start.
        let readable_folders = self.inner.places.existing_reference_folders(project)?;
        let (model, effort, instructions, permissions, team_tools) = match worker {
            Worker::Specialist(member) => (
                member.model().map(ToString::to_string),
                member.effort(),
                instructions_for(member, project, &readable_folders),
                PermissionProfile::specialist(),
                vec![TeamTool::AskManager],
            ),
            Worker::Advisor => {
                let profile = conversation.advisor_profile().cloned().unwrap_or_default();
                let mut team = ports.members.list_for_project(project.id())?;
                team.sort_by_key(|member| (member.joined_at(), member.id()));
                let mut tasks = ports.tasks.list_for_project(project.id())?;
                tasks.sort_by_key(Task::number);
                let names = team
                    .iter()
                    .map(|member| member.name().to_string())
                    .collect();
                (
                    profile.model.map(|model| model.to_string()),
                    profile.effort,
                    instructions_for_advisor(project, &team, &tasks, &readable_folders),
                    PermissionProfile::advisor(),
                    vec![TeamTool::Interview, TeamTool::ProposePlan { team: names }],
                )
            }
        };
        let request = TurnRequest {
            working_folder: self.inner.places.work_folder_of(project)?,
            prompt: prompt.unwrap_or_else(|| turn.request().to_owned()),
            session,
            model,
            effort,
            extra_instructions: Some(instructions),
            permissions,
            team_tools,
            readable_folders,
        };
        let handle = ports
            .runtime
            .start_turn(request, self.listener_for(conversation.id(), turn.id()))?;

        if let Err(error) = self.inner.store_new_turn(
            &conversation,
            is_new_conversation,
            answered_turn.as_ref(),
            &turn,
        ) {
            // The program must not work on something nobody can see. If the
            // turn was stored, the event of its end records it as stopped.
            handle.stop();
            return Err(error);
        }
        lock(&self.inner.running).insert(turn.id(), handle);
        turns.push(turn);
        ports.listener.conversation_changed(&conversation, &turns);
        Ok(ConversationWithTurns {
            conversation,
            turns,
        })
    }

    /// The listener the runtime calls with the events of one turn.
    ///
    /// It holds the service weakly: a turn that outlives the service has
    /// nobody left to record its events.
    fn listener_for(
        &self,
        conversation_id: ConversationId,
        turn_id: TurnId,
    ) -> Box<dyn FnMut(AgentEvent) + Send> {
        let inner = Arc::downgrade(&self.inner);
        Box::new(move |event| {
            if let Some(inner) = Weak::upgrade(&inner) {
                inner.follow(conversation_id, turn_id, event);
            }
        })
    }
}

/// Who a turn is for.
#[derive(Clone, Copy)]
enum Worker<'member> {
    /// An AI specialist of the team.
    Specialist(&'member Member),
    /// The Advisor, which plans the project.
    Advisor,
}

/// A turn about to start, with the conversation it belongs to.
struct NewTurn {
    conversation: Conversation,
    /// Whether the conversation is new and has to be stored too.
    is_new_conversation: bool,
    earlier_turns: Vec<Turn>,
    /// The earlier turn whose question or interview the new turn answers,
    /// with the answer recorded. It is stored only if the new turn starts.
    answered_turn: Option<Turn>,
    turn: Turn,
    /// What the worker is told, where that is more than the request of the
    /// turn: the request with what happened in the program since the worker
    /// last spoke, which its session did not see.
    prompt: Option<String>,
}

impl Inner {
    /// Takes the lock on the work. A panic of another thread while it held
    /// the lock leaves nothing to repair: everything it guards is in
    /// storage.
    fn hold_work(&self) -> MutexGuard<'_, ()> {
        lock(&self.work)
    }

    /// The checks that protect work in progress.
    fn guard(&self) -> WorkInProgress<'_> {
        WorkInProgress {
            projects: self.ports.projects.as_ref(),
            tasks: self.ports.tasks.as_ref(),
            members: self.ports.members.as_ref(),
            conversations: self.ports.conversations.as_ref(),
        }
    }

    /// Stores a turn that has just started, with its conversation if that
    /// is new and the turn whose question it answers if it answers one, and
    /// moves the task it is about into work.
    fn store_new_turn(
        &self,
        conversation: &Conversation,
        is_new_conversation: bool,
        answered_turn: Option<&Turn>,
        turn: &Turn,
    ) -> Result<(), ApplicationError> {
        if is_new_conversation {
            self.ports.conversations.save_conversation(conversation)?;
        }
        if let Some(answered_turn) = answered_turn {
            self.ports.conversations.save_turn(answered_turn)?;
        }
        self.ports.conversations.save_turn(turn)?;
        if let Some(task_id) = conversation.task_id() {
            self.move_task(
                task_id,
                &[StatusCategory::Todo, StatusCategory::Review],
                StatusCategory::Active,
            )?;
        }
        Ok(())
    }

    /// Ends the turns that storage calls running although this run of the
    /// program runs nothing for them: Teamwright was closed, or is closing,
    /// while they ran. Each ends as failed, with a text that says so. The
    /// turns of conversations whose home is another computer run there and
    /// are left alone. The caller holds the lock on the work.
    fn end_turns_nothing_runs(&self) -> Result<(), ApplicationError> {
        let ports = &self.ports;
        let now = ports.clock.now();
        for mut turn in ports.conversations.running_turns()? {
            if lock(&self.running).contains_key(&turn.id()) {
                // Started by this run of the program: it is really running.
                continue;
            }
            let Some(conversation) = ports.conversations.find(turn.conversation_id())? else {
                continue;
            };
            if !conversation.is_at_home_on(ports.this_computer) {
                continue;
            }
            turn.fail(CLOSED_WHILE_WORKING.to_owned(), None, now)?;
            ports.conversations.save_turn(&turn)?;
            let turns = ports.conversations.turns_of(conversation.id())?;
            ports.listener.conversation_changed(&conversation, &turns);
        }
        Ok(())
    }

    /// Records one event of a turn. Called on the runtime's thread.
    fn follow(&self, conversation_id: ConversationId, turn_id: TurnId, event: AgentEvent) {
        let _work = self.hold_work();
        // Nobody waits for this call, so a failure has nobody to be reported
        // to. The next event of the turn tries again; a turn whose end could
        // not be stored is ended by `recover` at the next start.
        let _ = self.record(conversation_id, turn_id, event);
    }

    /// Applies an event to its turn, stores the turn and tells the listener.
    fn record(
        &self,
        conversation_id: ConversationId,
        turn_id: TurnId,
        event: AgentEvent,
    ) -> Result<(), ApplicationError> {
        let ports = &self.ports;
        if matches!(event, AgentEvent::TurnEnded(_)) {
            lock(&self.running).remove(&turn_id);
        }
        let Some(mut conversation) = ports.conversations.find(conversation_id)? else {
            return Ok(());
        };
        let mut turns = ports.conversations.turns_of(conversation_id)?;
        let Some(index) = turns.iter().position(|turn| turn.id() == turn_id) else {
            return Ok(());
        };
        let turn = &mut turns[index];
        let now = ports.clock.now();

        let recorded = match event {
            AgentEvent::SessionStarted(started) => {
                if conversation.session_id() != Some(started.session_id.as_str()) {
                    conversation.remember_session(started.session_id.as_str().to_owned());
                    ports.conversations.save_conversation(&conversation)?;
                }
                turn.record_model(started.model)
            }
            AgentEvent::Text { text } => turn.say(text, now),
            AgentEvent::ToolUse { id, tool, summary } => turn.use_tool(id, tool, summary, now),
            AgentEvent::ToolResult {
                tool_use_id,
                failed,
                ..
            } => turn.finish_tool(&tool_use_id, failed),
            AgentEvent::PermissionDenied(denial) => turn.record_denial(denial.tool, now),
            AgentEvent::Retrying { reason, .. } => {
                turn.notice(retry_notice(reason.as_deref()), now)
            }
            AgentEvent::Warning { message } => turn.notice(message, now),
            // How much of the plan's limits is used is not shown yet. It
            // belongs with the budgets (decision 0008), which come later.
            AgentEvent::PlanUsage(_) => return Ok(()),
            AgentEvent::ManagerAsked(question) => {
                decision_from(question).and_then(|decision| turn.ask(decision))
            }
            AgentEvent::InterviewAsked(asked) => {
                interview_from(asked).and_then(|interview| turn.ask_interview(interview))
            }
            AgentEvent::PlanProposed(proposed) => {
                proposal_from(proposed).and_then(|proposal| turn.propose(proposal))
            }
            AgentEvent::TurnEnded(outcome) => end_turn(turn, &outcome, now),
        };
        if recorded.is_err() {
            // The turn ended before the event arrived (the manager stopped
            // it), or the event refers to a tool use the turn never saw.
            // Either way there is nothing to record.
            return Ok(());
        }

        // Work that ends in a question is not ready to be checked: the task
        // stays in work until the manager has decided and the specialist
        // has gone on.
        let ready_to_check = turn.state() == TurnState::Finished
            && turn.decision().is_none()
            && turn.interview().is_none()
            && turn.proposal().is_none();
        ports.conversations.save_turn(turn)?;
        if let Some(task_id) = conversation.task_id().filter(|_| ready_to_check) {
            self.move_task(task_id, &[StatusCategory::Active], StatusCategory::Review)?;
        }
        ports.listener.conversation_changed(&conversation, &turns);
        Ok(())
    }

    /// Moves a task whose status is of one of the categories `from` to the
    /// bottom of the first status of the category `to`. A task in a status
    /// of another category, or a workflow without a status of `to`, leaves
    /// the task where it is.
    fn move_task(
        &self,
        task_id: TaskId,
        from: &[StatusCategory],
        to: StatusCategory,
    ) -> Result<(), ApplicationError> {
        let ports = &self.ports;
        let Some(mut task) = ports.tasks.find(task_id)? else {
            return Ok(());
        };
        let project = self.require_project(task.project_id())?;
        let workflow = project.workflow();
        let is_to_move = workflow
            .status(task.status_id())
            .is_some_and(|status| from.contains(&status.category()));
        let Some(target) = workflow.first_in(to).filter(|_| is_to_move) else {
            return Ok(());
        };
        let last_rank = ports
            .tasks
            .list_for_project(project.id())?
            .iter()
            .filter(|other| other.status_id() == target.id() && other.id() != task.id())
            .map(Task::rank)
            .max();
        task.move_to(target, Rank::between(last_rank, None), ports.clock.now());
        ports.tasks.save(&task)?;
        Ok(())
    }

    fn require_project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.ports
            .projects
            .find(id)?
            .ok_or_else(|| not_found("project", id))
    }

    fn require_task(&self, id: TaskId) -> Result<Task, ApplicationError> {
        self.ports
            .tasks
            .find(id)?
            .ok_or_else(|| not_found("task", id))
    }

    fn require_member(&self, id: MemberId) -> Result<Member, ApplicationError> {
        self.ports
            .members
            .find(id)?
            .ok_or_else(|| not_found("team member", id))
    }

    /// The specialist of a conversation, unless it has left the team.
    fn specialist_of(&self, conversation: &Conversation) -> Result<Member, ApplicationError> {
        match conversation.member_id() {
            Some(member_id) => self.ports.members.find(member_id)?,
            None => None,
        }
        .ok_or(ApplicationError::SpecialistLeft)
    }

    fn require_conversation(&self, id: ConversationId) -> Result<Conversation, ApplicationError> {
        self.ports
            .conversations
            .find(id)?
            .ok_or_else(|| not_found("conversation", id))
    }

    /// The conversation, if its home is this computer. Only there does its
    /// session live, so only there can it be acted on.
    fn require_conversation_here(
        &self,
        id: ConversationId,
    ) -> Result<Conversation, ApplicationError> {
        let conversation = self.require_conversation(id)?;
        if conversation.is_at_home_on(self.ports.this_computer) {
            Ok(conversation)
        } else {
            Err(ApplicationError::ConversationRunsElsewhere {
                computer: conversation.home_computer(),
            })
        }
    }
}

/// The question that is open in a conversation: the one its latest turn
/// ended with, if the manager has not answered it yet.
fn open_decision_of(turns: &[Turn]) -> Option<&DecisionRequest> {
    turns
        .last()
        .filter(|latest| !latest.is_running())
        .and_then(Turn::decision)
        .filter(|decision| decision.is_open())
}

/// The interview that is open in a conversation: the one its latest turn
/// ended with, if the manager has not answered it yet.
fn open_interview_of(turns: &[Turn]) -> Option<&Interview> {
    turns
        .last()
        .filter(|latest| !latest.is_running())
        .and_then(Turn::interview)
        .filter(|interview| interview.is_open())
}

/// The proposal that is open in a conversation: the one its latest turn
/// ended with, if the manager has not approved it. A proposal of an
/// earlier turn is not open: the conversation has gone on since.
fn open_proposal_of(turns: &[Turn]) -> Option<&Proposal> {
    turns
        .last()
        .filter(|latest| !latest.is_running())
        .and_then(Turn::proposal)
        .filter(|proposal| proposal.is_open())
}

/// When the open question of a conversation was asked: when its latest turn
/// ended.
fn asked_at(conversation: &ConversationWithTurns) -> Timestamp {
    conversation
        .turns
        .last()
        .and_then(Turn::ended_at)
        .unwrap_or_else(|| conversation.conversation.started_at())
}

/// The answer of the manager as the specialist reads it: the option that
/// was chosen, and what the manager added in words.
fn answer_in_words(option: Option<&str>, note: &str) -> String {
    match option {
        None => note.to_owned(),
        Some(option) if note.is_empty() => format!("Decision: {option}"),
        Some(option) => format!("Decision: {option}\n\n{note}"),
    }
}

/// The question a specialist asked, as the domain keeps it.
fn decision_from(question: ManagerQuestion) -> Result<DecisionRequest, DomainError> {
    DecisionRequest::new(
        &question.question,
        &question.reason,
        question
            .options
            .into_iter()
            .map(|option| DecisionOption {
                label: option.label,
                consequence: option.consequence,
            })
            .collect(),
        question.recommended,
    )
}

/// The interview the Advisor asked, as the domain keeps it.
fn interview_from(asked: ManagerInterview) -> Result<Interview, DomainError> {
    Interview::new(
        asked
            .questions
            .into_iter()
            .map(|question| InterviewQuestion {
                question: question.question,
                reason: question.reason,
                options: question
                    .options
                    .into_iter()
                    .map(|option| DecisionOption {
                        label: option.label,
                        consequence: option.consequence,
                    })
                    .collect(),
                recommended: question.recommended,
            })
            .collect(),
    )
}

/// The plan the Advisor proposed, as the domain keeps it. Names and titles
/// follow the rules of members and tasks.
fn proposal_from(proposed: PlanProposal) -> Result<Proposal, DomainError> {
    let members = proposed
        .members
        .into_iter()
        .map(|member| {
            Ok(ProposedMember {
                name: MemberName::parse(&member.name)?,
                responsibility: member.responsibility,
                reason: member.reason,
                model: member.model.as_deref().map(ModelName::parse).transpose()?,
                effort: member.effort,
            })
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    let tasks = proposed
        .tasks
        .into_iter()
        .map(|task| {
            Ok(ProposedTask {
                title: TaskTitle::parse(&task.title)?,
                description: task.description,
                owner: Some(match task.owner {
                    PlannedOwner::Manager => ProposedOwner::Manager,
                    PlannedOwner::Member(name) => ProposedOwner::Member(MemberName::parse(&name)?),
                }),
                estimate_minutes: task.estimate_minutes,
                priority: task.priority,
                waits_for: task.waits_for,
            })
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    Proposal::new(&proposed.summary, members, tasks)
}

/// Ends a turn the way the program says it ended.
fn end_turn(
    turn: &mut Turn,
    outcome: &TurnOutcome,
    now: teamwright_domain::Timestamp,
) -> Result<(), DomainError> {
    let tokens = tokens_of(outcome.total_tokens());
    // A turn that failed may have used tokens before it did. They are
    // recorded when the program reported any.
    let used = (!outcome.usage.is_empty()).then_some(tokens);
    match &outcome.failure {
        None => turn.finish(&outcome.final_text, tokens, now),
        Some(TurnFailure::Stopped) => turn.stop(used, now),
        Some(failure) => turn.fail(failure_text(failure, &outcome.final_text), used, now),
    }
}

/// Says in words why a turn failed.
fn failure_text(failure: &TurnFailure, final_text: &str) -> String {
    match failure {
        TurnFailure::NotSignedIn => NOT_SIGNED_IN.to_owned(),
        TurnFailure::Reported { .. } if final_text.trim().is_empty() => COULD_NOT_FINISH.to_owned(),
        TurnFailure::Reported { .. } => final_text.trim().to_owned(),
        TurnFailure::Stopped => Turn::STOPPED_BY_THE_MANAGER.to_owned(),
        // The runtime already says this in words for people.
        TurnFailure::ProgramEnded { message } => message.clone(),
    }
}

/// The notice for a request to the model that is tried again.
fn retry_notice(reason: Option<&str>) -> String {
    let opening = "A request to the model failed and is being tried again";
    match reason {
        Some(reason) => format!("{opening}: {reason}"),
        None => format!("{opening}."),
    }
}

/// The tokens of the runtime, as the domain keeps them.
const fn tokens_of(tokens: agent_runtime::TokenCounts) -> teamwright_domain::TokenCounts {
    teamwright_domain::TokenCounts {
        input: tokens.input,
        output: tokens.output,
        cache_read: tokens.cache_read,
        cache_written: tokens.cache_written,
    }
}

/// Reports that something the request refers to does not exist.
fn not_found(kind: &'static str, id: impl ToString) -> ApplicationError {
    ApplicationError::NotFound {
        kind,
        id: id.to_string(),
    }
}

/// Locks a mutex, and keeps working even if another thread panicked while it
/// held the lock. What the mutexes here guard stays valid between single
/// operations.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The name a manager knows a task by, such as `IOT-12`.
pub(crate) fn reference_of(project: &Project, task: &Task) -> String {
    format!("{}-{}", project.key(), task.number())
}

/// The checks that keep work in progress from being taken away from under a
/// specialist. The services that remove things use them too.
pub(crate) struct WorkInProgress<'ports> {
    pub(crate) projects: &'ports dyn ProjectRepository,
    pub(crate) tasks: &'ports dyn TaskRepository,
    pub(crate) members: &'ports dyn MemberRepository,
    pub(crate) conversations: &'ports dyn ConversationRepository,
}

impl WorkInProgress<'_> {
    /// Refuses to go on while a turn runs in one of some conversations.
    pub(crate) fn refuse_while_worked_on(
        &self,
        among: &[Conversation],
    ) -> Result<(), ApplicationError> {
        let busy = self.conversations_at_work()?;
        let Some(conversation) = among
            .iter()
            .find(|conversation| busy.contains(&conversation.id()))
        else {
            return Ok(());
        };
        if conversation.is_with_advisor() {
            return Err(ApplicationError::WorkInProgress {
                who: THE_ADVISOR.to_owned(),
                work: A_PLAN.to_owned(),
            });
        }
        let who = match conversation.member_id() {
            Some(member_id) => self.members.find(member_id)?,
            None => None,
        }
        .map_or_else(
            || A_SPECIALIST.to_owned(),
            |member| member.name().to_string(),
        );
        Err(ApplicationError::WorkInProgress {
            who,
            work: self.work_of(conversation)?,
        })
    }

    /// Refuses to go on while a specialist works on something: it works on
    /// one thing at a time.
    pub(crate) fn refuse_while_busy(&self, member: &Member) -> Result<(), ApplicationError> {
        for conversation_id in self.conversations_at_work()? {
            let Some(conversation) = self.conversations.find(conversation_id)? else {
                continue;
            };
            if conversation.member_id() == Some(member.id()) {
                return Err(ApplicationError::MemberBusy {
                    name: member.name().to_string(),
                    work: self.work_of(&conversation)?,
                });
            }
        }
        Ok(())
    }

    /// Refuses to go on while the Advisor works in a project: one
    /// conversation with the Advisor runs in a project at a time.
    pub(crate) fn refuse_while_advisor_works(
        &self,
        project_id: ProjectId,
    ) -> Result<(), ApplicationError> {
        for conversation_id in self.conversations_at_work()? {
            let Some(conversation) = self.conversations.find(conversation_id)? else {
                continue;
            };
            if conversation.is_with_advisor() && conversation.project_id() == project_id {
                return Err(ApplicationError::AdvisorBusy);
            }
        }
        Ok(())
    }

    /// The conversations in which a turn is running.
    fn conversations_at_work(&self) -> Result<BTreeSet<ConversationId>, ApplicationError> {
        Ok(self
            .conversations
            .running_turns()?
            .iter()
            .map(Turn::conversation_id)
            .collect())
    }

    /// What a conversation is about, in words: a task as the manager knows
    /// it, or a question.
    fn work_of(&self, conversation: &Conversation) -> Result<String, ApplicationError> {
        let Some(task_id) = conversation.task_id() else {
            return Ok(A_QUESTION.to_owned());
        };
        let Some(task) = self.tasks.find(task_id)? else {
            return Ok(A_QUESTION.to_owned());
        };
        Ok(self.projects.find(task.project_id())?.map_or_else(
            || A_QUESTION.to_owned(),
            |project| reference_of(&project, &task),
        ))
    }
}

/// Writes the brief a specialist gets when it starts on a task: what the
/// task is, what it belongs to, what it builds on, and what the manager
/// expects back. `tasks` are the tasks of the project.
#[must_use]
pub fn brief_for(project: &Project, task: &Task, tasks: &[Task]) -> String {
    let status_name = |other: &Task| {
        project.workflow().status(other.status_id()).map_or_else(
            || "unknown status".to_owned(),
            |status| status.name().to_string(),
        )
    };
    let listed = |others: &[&Task]| -> Vec<String> {
        others
            .iter()
            .map(|other| {
                format!(
                    "- {}: {} ({})",
                    reference_of(project, other),
                    other.title(),
                    status_name(other)
                )
            })
            .collect()
    };

    let parent = task
        .parent_id()
        .and_then(|parent_id| tasks.iter().find(|other| other.id() == parent_id));
    let mut parts: Vec<&Task> = tasks
        .iter()
        .filter(|other| other.parent_id() == Some(task.id()))
        .collect();
    parts.sort_by_key(|other| other.number());
    let mut blockers: Vec<&Task> = tasks
        .iter()
        .filter(|other| task.blocked_by().contains(&other.id()))
        .collect();
    blockers.sort_by_key(|other| other.number());

    let mut paragraphs = vec![format!(
        "Your task is {}: {}",
        reference_of(project, task),
        task.title()
    )];
    let description = task.description().trim();
    if !description.is_empty() {
        paragraphs.push(description.to_owned());
    }
    if let Some(parent) = parent {
        paragraphs.push(format!(
            "It is a part of {}: {}",
            reference_of(project, parent),
            parent.title()
        ));
    }
    if !parts.is_empty() {
        let mut lines = vec!["Its parts are:".to_owned()];
        lines.extend(listed(&parts));
        paragraphs.push(lines.join("\n"));
    }
    if !blockers.is_empty() {
        let mut lines = vec!["It builds on these tasks:".to_owned()];
        lines.extend(listed(&blockers));
        paragraphs.push(lines.join("\n"));
    }
    if let Some(due_on) = task.schedule().due_on() {
        paragraphs.push(format!("It is due on {}.", due_on.format("%Y-%m-%d")));
    }
    paragraphs.push(
        [
            "When you are done, end with a short report for your manager:",
            "1. what you did,",
            "2. which files you created or changed,",
            "3. what you decided on your own, and why,",
            "4. what is still open.",
        ]
        .join("\n"),
    );
    paragraphs.join("\n\n")
}

/// Writes the instructions that are added to the agent program's own: who
/// the specialist is, and how the team works. `reference_folders` are the
/// folders the specialist may read and must not change.
#[must_use]
pub fn instructions_for(
    member: &Member,
    project: &Project,
    reference_folders: &[PathBuf],
) -> String {
    let mut lines = vec![format!(
        "You are {}, a member of the team that works on the project \"{}\" for a human \
         manager. The team is run with the program Teamwright.",
        member.name(),
        project.name()
    )];
    let responsibility = member.responsibility().trim();
    if !responsibility.is_empty() {
        lines.push(format!("What you are on the team for: {responsibility}"));
    }
    lines.extend(
        [
            "",
            "How the team works:",
            "- You work in the folder of the project, which is the folder you are in. Read what \
             is already there before you begin.",
            "- Put what you produce into files in that folder, with names that say what they \
             hold. Do not write anywhere else.",
        ]
        .map(str::to_owned),
    );
    if !reference_folders.is_empty() {
        // Each folder stands on a line of its own, because the name of a
        // folder may hold a comma.
        lines.push(
            "- Besides the folder of the project you may read the folders below. They hold \
             material the manager wants you to know, and you must not change anything in \
             them. Write their paths exactly as they stand here:"
                .to_owned(),
        );
        lines.extend(
            reference_folders
                .iter()
                .map(|folder| format!("    {}", folder.display())),
        );
    }
    lines.push(format!(
        "- Single files the manager handed you are copies in the folder {REFERENCES_FOLDER} \
         inside the folder of the project."
    ));
    lines.extend(
        [
            "- You cannot run commands and you cannot fetch anything from the internet. If the \
             task needs one of these, say so in your report instead of working around it.",
            "- Decide small things yourself, and write down what you decided. A decision that \
             changes what is built, what it costs or when it is ready is the manager's: do not \
             guess, but ask with the tool ask_manager (the question, why it matters, two to \
             four options with what each leads to, and the one you recommend), then end your \
             turn. The answer comes as your next message.",
            "- Write in plain words. Your manager reads your report first, and may read nothing \
             else.",
        ]
        .map(str::to_owned),
    );
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::{TimeDelta, TimeZone};
    use teamwright_domain::{
        CalendarDate, Effort, StatusId, StepContent, StepKind, Timestamp, ToolOutcome,
    };

    use super::*;
    use crate::agent_runtime::{ModelUsage, PermissionDenial, QuestionOption, SessionStarted};
    use crate::in_memory::InMemorySprintRepository;
    use crate::in_memory::{
        FixedProjectFolders, InMemoryConversationRepository, InMemoryMemberRepository,
        InMemoryPlacesRepository, InMemoryPlanningRepository, InMemoryProjectRepository,
        InMemoryTaskRepository, ManualClock, PretendFileSystem, RecordingWorkListener,
        ScriptedRuntime,
    };
    use crate::places::PlacesService;
    use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
    use crate::tasks::{NewTask, TaskService};
    use crate::team::{NewMember, TeamService};

    const SESSION: &str = "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10";
    const REPORT: &str = "Done: Draw the schematic.\n1. I drew it.";

    /// The services under test on shared storage, a scripted runtime, and a
    /// project with an engineering workflow (to do, in progress, in
    /// revision, in test, done), a specialist and a task given to it.
    struct Workspace {
        projects: ProjectService,
        tasks: TaskService,
        team: TeamService,
        work: WorkService,
        ports: Ports,
        project: Project,
        engineer: Member,
        schematic: Task,
    }

    /// The stand-ins behind the services, to build a second work service on
    /// the same storage.
    #[derive(Clone)]
    struct Ports {
        projects: Arc<InMemoryProjectRepository>,
        tasks: Arc<InMemoryTaskRepository>,
        members: Arc<InMemoryMemberRepository>,
        conversations: Arc<InMemoryConversationRepository>,
        places: Arc<InMemoryPlacesRepository>,
        file_system: Arc<PretendFileSystem>,
        runtime: Arc<ScriptedRuntime>,
        listener: Arc<RecordingWorkListener>,
        clock: Arc<ManualClock>,
        this_computer: ComputerId,
    }

    impl Ports {
        fn work_service(&self) -> WorkService {
            WorkService::new(WorkPorts {
                projects: self.projects.clone(),
                tasks: self.tasks.clone(),
                members: self.members.clone(),
                conversations: self.conversations.clone(),
                planning: Arc::new(InMemoryPlanningRepository::new(
                    self.projects.clone(),
                    self.members.clone(),
                    self.tasks.clone(),
                    self.conversations.clone(),
                )),
                folders: Arc::new(FixedProjectFolders::inside("projects")),
                places: self.places.clone(),
                file_system: self.file_system.clone(),
                runtime: self.runtime.clone(),
                listener: self.listener.clone(),
                clock: self.clock.clone(),
                this_computer: self.this_computer,
            })
        }

        fn places_service(&self) -> PlacesService {
            PlacesService::new(
                self.projects.clone(),
                self.tasks.clone(),
                self.members.clone(),
                self.conversations.clone(),
                self.places.clone(),
                Arc::new(FixedProjectFolders::inside("projects")),
                self.file_system.clone(),
            )
        }
    }

    fn start_of_day() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn workspace() -> Workspace {
        let ports = Ports {
            projects: Arc::new(InMemoryProjectRepository::default()),
            tasks: Arc::new(InMemoryTaskRepository::default()),
            members: Arc::new(InMemoryMemberRepository::default()),
            conversations: Arc::new(InMemoryConversationRepository::default()),
            places: Arc::new(InMemoryPlacesRepository::default()),
            file_system: Arc::new(PretendFileSystem::default()),
            runtime: Arc::new(ScriptedRuntime::new()),
            listener: Arc::new(RecordingWorkListener::default()),
            clock: Arc::new(ManualClock::starting_at(start_of_day())),
            this_computer: ComputerId::generate(),
        };
        let projects = ProjectService::new(
            ports.projects.clone(),
            ports.tasks.clone(),
            ports.members.clone(),
            ports.conversations.clone(),
            ports.places.clone(),
            Arc::new(InMemorySprintRepository::new(ports.tasks.clone())),
            ports.clock.clone(),
        );
        let tasks = TaskService::new(
            ports.projects.clone(),
            ports.tasks.clone(),
            ports.members.clone(),
            ports.conversations.clone(),
            Arc::new(InMemorySprintRepository::new(ports.tasks.clone())),
            ports.clock.clone(),
        );
        let team = TeamService::new(
            ports.projects.clone(),
            ports.members.clone(),
            ports.tasks.clone(),
            ports.conversations.clone(),
            ports.clock.clone(),
        );
        let work = ports.work_service();

        let project = projects
            .create(NewProject {
                name: "Smart sensor node".to_owned(),
                workflow: WorkflowTemplate::Engineering,
                ..NewProject::default()
            })
            .unwrap();
        let engineer = team
            .add(NewMember {
                project_id: project.id(),
                name: "Hardware engineer".to_owned(),
                kind: MemberKind::Agent,
                responsibility: "The schematic, the parts and the board.".to_owned(),
                model: None,
                effort: None,
            })
            .unwrap();
        let schematic = tasks
            .create(NewTask {
                assignee: Some(Assignee::Member(engineer.id())),
                ..NewTask::titled(project.id(), "Draw the schematic")
            })
            .unwrap();
        Workspace {
            projects,
            tasks,
            team,
            work,
            ports,
            project,
            engineer,
            schematic,
        }
    }

    impl Workspace {
        fn status_of(&self, category: StatusCategory) -> StatusId {
            self.project.workflow().first_in(category).unwrap().id()
        }

        fn stored_task(&self, id: TaskId) -> Task {
            self.tasks.get(id).unwrap()
        }

        fn only_conversation(&self) -> ConversationWithTurns {
            let mut conversations = self.work.list_for_project(self.project.id()).unwrap();
            assert_eq!(conversations.len(), 1);
            conversations.remove(0)
        }

        fn person(&self, name: &str) -> Member {
            self.team
                .add(NewMember {
                    project_id: self.project.id(),
                    name: name.to_owned(),
                    kind: MemberKind::Human,
                    responsibility: String::new(),
                    model: None,
                    effort: None,
                })
                .unwrap()
        }

        fn task_for(&self, title: &str, assignee: Option<Assignee>) -> Task {
            self.tasks
                .create(NewTask {
                    assignee,
                    ..NewTask::titled(self.project.id(), title)
                })
                .unwrap()
        }

        /// Delivers an event to a turn, counted from 0, a second later.
        fn deliver(&self, turn: usize, event: AgentEvent) {
            self.ports.clock.advance(TimeDelta::seconds(1));
            self.ports.runtime.deliver(turn, event);
        }

        /// Plays a whole turn that reads a file and reports.
        fn play_whole_turn(&self, turn: usize) {
            self.deliver(turn, session_started());
            self.deliver(
                turn,
                AgentEvent::Text {
                    text: "I will read the notes first.".to_owned(),
                },
            );
            self.deliver(
                turn,
                AgentEvent::ToolUse {
                    id: "read".to_owned(),
                    tool: "Read".to_owned(),
                    summary: "notes.md".to_owned(),
                },
            );
            self.deliver(
                turn,
                AgentEvent::ToolResult {
                    tool_use_id: "read".to_owned(),
                    tool: Some("Read".to_owned()),
                    failed: false,
                },
            );
            self.deliver(
                turn,
                AgentEvent::Text {
                    text: REPORT.to_owned(),
                },
            );
            self.deliver(turn, ended(None, REPORT));
        }
    }

    fn session_started() -> AgentEvent {
        AgentEvent::SessionStarted(SessionStarted {
            session_id: SessionId::parse(SESSION).unwrap(),
            model: "claude-opus-model".to_owned(),
            working_folder: PathBuf::from("projects/SSN"),
            permission_mode: "acceptEdits".to_owned(),
            tools: Vec::new(),
            skills: Vec::new(),
            plugins: Vec::new(),
            mcp_servers: Vec::new(),
            program_version: Some("2.1.322".to_owned()),
        })
    }

    fn runtime_tokens() -> agent_runtime::TokenCounts {
        agent_runtime::TokenCounts {
            input: 42,
            output: 880,
            cache_read: 21_500,
            cache_written: 6400,
        }
    }

    /// What the specialist reads when it waits for the manager.
    const WAITING: &str = "I am waiting for your decision.";

    /// The question a specialist asks before it works on a task whose
    /// title says that something is to be chosen, as in the demo.
    fn question_about(title: &str) -> ManagerQuestion {
        ManagerQuestion {
            question: format!("Which way should I take for \"{title}\"?"),
            reason: "It changes what is built.".to_owned(),
            options: vec![
                QuestionOption {
                    label: "The proven way".to_owned(),
                    consequence: "A larger board.".to_owned(),
                },
                QuestionOption {
                    label: "The compact way".to_owned(),
                    consequence: "A new part to test.".to_owned(),
                },
            ],
            recommended: Some(0),
        }
    }

    impl Workspace {
        /// Plays a whole turn, counted from 0, that asks the manager and
        /// ends.
        fn play_turn_that_asks(&self, turn: usize, title: &str) {
            self.deliver(turn, session_started());
            self.deliver(turn, AgentEvent::ManagerAsked(question_about(title)));
            self.deliver(
                turn,
                AgentEvent::Text {
                    text: WAITING.to_owned(),
                },
            );
            self.deliver(turn, ended(None, WAITING));
        }

        /// Starts a task given to the engineer and lets the specialist
        /// work on it until it has asked its question.
        fn asked_about(&self, title: &str) -> (Task, ConversationWithTurns) {
            let task = self.task_for(title, Some(Assignee::Member(self.engineer.id())));
            let started = self.work.start_work(task.id()).unwrap();
            let turn = self.ports.runtime.requests().len() - 1;
            self.play_turn_that_asks(turn, title);
            (task, self.conversation(started.conversation.id()))
        }

        /// Lets the specialist work on "Choose the battery" until it has
        /// asked its question.
        fn asked(&self) -> (Task, ConversationWithTurns) {
            self.asked_about("Choose the battery")
        }

        fn conversation(&self, id: ConversationId) -> ConversationWithTurns {
            self.work
                .list_for_project(self.project.id())
                .unwrap()
                .into_iter()
                .find(|listed| listed.conversation.id() == id)
                .unwrap()
        }

        fn waiting(&self) -> Vec<ConversationId> {
            self.work
                .waiting_for_manager()
                .unwrap()
                .iter()
                .map(|waiting| waiting.conversation.id())
                .collect()
        }
    }

    fn ended(failure: Option<TurnFailure>, final_text: &str) -> AgentEvent {
        AgentEvent::TurnEnded(TurnOutcome {
            session_id: Some(SessionId::parse(SESSION).unwrap()),
            failure,
            final_text: final_text.to_owned(),
            usage: vec![ModelUsage {
                model: "claude-opus-model".to_owned(),
                tokens: runtime_tokens(),
            }],
            estimated_cost_usd: None,
            duration: None,
            model_calls: None,
            permission_denials: Vec::new(),
        })
    }

    #[test]
    fn the_service_can_be_shared_between_threads() {
        // A desktop shell keeps the service in its state and calls it from
        // several threads, and the runtime's threads reach it through the
        // listener of each turn. The compiler proves it is allowed.
        fn shared<Service: Send + Sync + 'static>() {}
        shared::<WorkService>();
    }

    // The program

    #[test]
    fn the_program_is_ready_when_it_is_installed_and_signed_in() {
        let workspace = workspace();

        let program = workspace.work.program();

        assert_eq!(
            program,
            AgentProgram {
                state: AgentProgramState::Ready,
                version: Some(ProgramVersion::new(2, 1, 322)),
                sign_in_method: Some("claude.ai".to_owned()),
                subscription: Some("max".to_owned()),
                problem: None,
            }
        );
    }

    #[test]
    fn the_program_says_when_it_is_missing_signed_out_or_cannot_be_asked() {
        let workspace = workspace();
        let runtime = &workspace.ports.runtime;

        runtime.set_sign_in(Ok(SignIn::SignedOut));
        let signed_out = workspace.work.program();
        runtime.set_sign_in(Err(AgentRuntimeError::UnexpectedAnswer {
            message: "no sign-in state".to_owned(),
        }));
        let unclear = workspace.work.program();
        runtime.set_installation(Err(AgentRuntimeError::NotInstalled));
        let missing = workspace.work.program();
        runtime.set_installation(Err(AgentRuntimeError::CouldNotRun {
            message: "access denied".to_owned(),
        }));
        let broken = workspace.work.program();

        assert_eq!(signed_out.state, AgentProgramState::SignedOut);
        assert_eq!(signed_out.version, Some(ProgramVersion::new(2, 1, 322)));
        assert_eq!(unclear.state, AgentProgramState::Unavailable);
        assert_eq!(
            unclear.problem.as_deref(),
            Some("Claude Code gave an answer that was not understood: no sign-in state")
        );
        assert_eq!(missing.state, AgentProgramState::NotInstalled);
        assert_eq!(missing.version, None);
        assert_eq!(missing.problem, None);
        assert_eq!(broken.state, AgentProgramState::Unavailable);
        assert_eq!(
            broken.problem.as_deref(),
            Some("Claude Code could not be run: access denied")
        );
        assert_eq!(
            [signed_out.state, missing.state, broken.state].map(AgentProgramState::as_str),
            ["signed_out", "not_installed", "unavailable"]
        );
    }

    // Starting on a task

    #[test]
    fn starting_on_a_task_opens_a_conversation_and_sends_the_brief() {
        let workspace = workspace();

        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();

        let conversation = &started.conversation;
        assert_eq!(conversation.project_id(), workspace.project.id());
        assert_eq!(conversation.member_id(), Some(workspace.engineer.id()));
        assert_eq!(conversation.task_id(), Some(workspace.schematic.id()));
        assert_eq!(started.turns.len(), 1);
        let turn = &started.turns[0];
        let brief = brief_for(
            &workspace.project,
            &workspace.schematic,
            std::slice::from_ref(&workspace.schematic),
        );
        assert_eq!(turn.origin(), TurnOrigin::Brief);
        assert_eq!(turn.state(), TurnState::Running);
        assert_eq!(turn.request(), brief);
        assert_eq!(turn.answer(), "");
        assert_eq!(turn.tokens(), None);
        assert_eq!(workspace.only_conversation(), started);
        assert_eq!(workspace.ports.listener.changes().len(), 1);
    }

    #[test]
    fn a_turn_runs_in_the_folder_of_the_project_with_the_permissions_of_a_specialist() {
        let workspace = workspace();

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let request = &workspace.ports.runtime.requests()[0];
        assert_eq!(
            request.working_folder,
            PathBuf::from("projects").join("SSN")
        );
        assert_eq!(request.session, SessionChoice::New);
        assert_eq!(request.permissions, PermissionProfile::specialist());
        assert_eq!(request.team_tools, [TeamTool::AskManager]);
        assert!(request.readable_folders.is_empty());
        assert_eq!(request.model, None);
        assert_eq!(request.effort, None);
        assert_eq!(
            request.extra_instructions.as_deref(),
            Some(instructions_for(&workspace.engineer, &workspace.project, &[]).as_str())
        );
        assert!(
            request
                .prompt
                .starts_with("Your task is SSN-1: Draw the schematic")
        );
    }

    #[test]
    fn a_turn_works_in_the_chosen_work_folder_and_may_read_the_reference_folders_that_exist() {
        let workspace = workspace();
        for folder in ["/sample/work", "/sample/datasheets"] {
            workspace.ports.file_system.add_folder(folder);
        }
        workspace
            .ports
            .places
            .save(
                workspace.project.id(),
                &crate::places::ChosenPlaces {
                    work_folder: Some(PathBuf::from("/sample/work")),
                    reference_folders: vec![
                        PathBuf::from("/sample/datasheets"),
                        PathBuf::from("/sample/gone"),
                    ],
                },
            )
            .unwrap();

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let request = &workspace.ports.runtime.requests()[0];
        assert_eq!(request.working_folder, PathBuf::from("/sample/work"));
        assert_eq!(
            request.readable_folders,
            [PathBuf::from("/sample/datasheets")]
        );
        let instructions = request.extra_instructions.as_deref().unwrap();
        assert!(instructions.contains(
            "Write their paths exactly as they stand here:\n    /sample/datasheets\n- Single files"
        ));
        assert!(!instructions.contains("/sample/gone"));
        assert_eq!(
            workspace
                .ports
                .places_service()
                .work_folder_of(&workspace.project)
                .unwrap(),
            request.working_folder
        );
    }

    #[test]
    fn a_reference_folder_reaches_a_turn_in_its_full_form() {
        // The manager named the folder in another way than the computer
        // does: here with other capitals, on Windows often with a short
        // name. The program that runs the specialist compares paths as they
        // are written, so the turn must get the one full form.
        let workspace = workspace();
        workspace
            .ports
            .file_system
            .add_folder("/Sample/Data sheets");
        workspace
            .ports
            .places
            .save(
                workspace.project.id(),
                &crate::places::ChosenPlaces {
                    work_folder: None,
                    reference_folders: vec![PathBuf::from("/sample/data sheets")],
                },
            )
            .unwrap();

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let request = &workspace.ports.runtime.requests()[0];
        assert_eq!(
            request.readable_folders,
            [PathBuf::from("/Sample/Data sheets")]
        );
        let instructions = request.extra_instructions.as_deref().unwrap();
        assert!(instructions.contains("\n    /Sample/Data sheets\n"));
        assert!(!instructions.contains("/sample/data sheets"));
    }

    #[test]
    fn a_turn_runs_on_the_model_and_at_the_effort_of_the_specialist() {
        let workspace = workspace();
        workspace
            .team
            .set_profile(workspace.engineer.id(), Some("opus"), Some(Effort::High))
            .unwrap();

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let request = &workspace.ports.runtime.requests()[0];
        assert_eq!(request.model.as_deref(), Some("opus"));
        assert_eq!(request.effort, Some(Effort::High));
    }

    #[test]
    fn a_specialist_without_a_profile_leaves_the_model_and_the_effort_to_the_program() {
        let workspace = workspace();
        assert!(workspace.engineer.profile().is_empty());

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let request = &workspace.ports.runtime.requests()[0];
        assert_eq!(request.model, None);
        assert_eq!(request.effort, None);
    }

    #[test]
    fn starting_on_a_task_moves_it_to_the_first_status_in_which_work_is_under_way() {
        let workspace = workspace();

        workspace.work.start_work(workspace.schematic.id()).unwrap();

        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Active)
        );
    }

    #[test]
    fn work_starts_only_on_a_task_given_to_an_ai_specialist() {
        let workspace = workspace();
        let person = workspace.person("Test engineer");
        let mine = workspace.task_for("Order the parts", Some(Assignee::Manager));
        let nobodys = workspace.task_for("Unassigned", None);
        let persons = workspace.task_for("Test the prototype", Some(Assignee::Member(person.id())));

        for task in [mine, nobodys, persons] {
            let error = workspace.work.start_work(task.id()).unwrap_err();
            assert_eq!(error.code(), "no_specialist");
            assert_eq!(
                error.to_string(),
                "the task is not given to an AI specialist"
            );
        }
        assert!(workspace.ports.runtime.requests().is_empty());
    }

    #[test]
    fn work_on_a_task_starts_only_once() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        let error = workspace
            .work
            .start_work(workspace.schematic.id())
            .unwrap_err();

        assert_eq!(error.code(), "work_already_started");
        assert_eq!(error.to_string(), "work on SSN-1 has already started");
    }

    #[test]
    fn a_specialist_works_on_one_thing_at_a_time() {
        let workspace = workspace();
        let second = workspace.task_for(
            "Route the board",
            Some(Assignee::Member(workspace.engineer.id())),
        );
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let error = workspace.work.start_work(second.id()).unwrap_err();

        assert_eq!(error.code(), "member_busy");
        assert_eq!(
            error.to_string(),
            "Hardware engineer is still working on SSN-1"
        );
    }

    #[test]
    fn a_specialist_that_answers_a_question_is_busy_with_it() {
        let workspace = workspace();
        workspace
            .work
            .ask(workspace.engineer.id(), "Which regulator do you prefer?")
            .unwrap();

        let error = workspace
            .work
            .start_work(workspace.schematic.id())
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Hardware engineer is still working on a question"
        );
    }

    #[test]
    fn a_turn_the_program_could_not_start_leaves_nothing_behind() {
        let workspace = workspace();
        workspace
            .ports
            .runtime
            .refuse_next_turn(AgentRuntimeError::NotInstalled);

        let error = workspace
            .work
            .start_work(workspace.schematic.id())
            .unwrap_err();

        assert_eq!(error.code(), "agent_not_installed");
        assert!(
            workspace
                .work
                .list_for_project(workspace.project.id())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Todo)
        );
        assert!(workspace.work.start_work(workspace.schematic.id()).is_ok());
    }

    // Following the work

    #[test]
    fn every_event_is_stored_and_told_with_the_whole_conversation() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        let last_kinds: Vec<Option<StepKind>> = workspace
            .ports
            .listener
            .changes()
            .iter()
            .map(|(_, turns)| turns[0].steps().last().map(|step| step.content.kind()))
            .collect();
        assert_eq!(
            last_kinds,
            [
                None,
                None,
                Some(StepKind::Text),
                Some(StepKind::Tool),
                Some(StepKind::Tool),
                Some(StepKind::Text),
                Some(StepKind::Tool),
            ]
        );
        let (told, turns) = workspace.ports.listener.changes().pop().unwrap();
        let stored = workspace.only_conversation();
        assert_eq!(told, stored.conversation);
        assert_eq!(turns, stored.turns);
        assert_eq!(told.session_id(), Some(SESSION));
        assert_eq!(turns[0].model(), Some("claude-opus-model"));
    }

    #[test]
    fn a_tool_is_shown_as_running_until_it_is_done() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        let outcomes: Vec<ToolOutcome> = workspace
            .ports
            .listener
            .changes()
            .iter()
            .filter_map(|(_, turns)| {
                turns[0]
                    .steps()
                    .iter()
                    .find_map(|step| match &step.content {
                        StepContent::Tool {
                            use_id, outcome, ..
                        } if use_id == "read" => Some(*outcome),
                        _ => None,
                    })
            })
            .collect();
        assert_eq!(outcomes.first(), Some(&ToolOutcome::Running));
        assert_eq!(outcomes.last(), Some(&ToolOutcome::Done));
    }

    #[test]
    fn a_finished_turn_has_its_report_once_the_tokens_it_used_and_its_end() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(turn.state(), TurnState::Finished);
        assert_eq!(turn.answer(), REPORT);
        assert_eq!(turn.tokens(), Some(tokens_of(runtime_tokens())));
        assert_eq!(turn.ended_at(), Some(workspace.ports.clock.now()));
        let texts: Vec<&StepContent> = turn
            .steps()
            .iter()
            .map(|step| &step.content)
            .filter(|content| content.kind() == StepKind::Text)
            .collect();
        assert_eq!(texts.len(), 1);
    }

    #[test]
    fn a_finished_turn_moves_the_task_on_to_the_first_status_for_checking() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Review)
        );
    }

    #[test]
    fn without_a_status_for_checking_the_task_stays_where_work_is_under_way() {
        let workspace = workspace();
        workspace
            .projects
            .remove_status(
                workspace.project.id(),
                workspace.status_of(StatusCategory::Review),
                None,
            )
            .unwrap();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Active)
        );
    }

    #[test]
    fn a_task_the_manager_moved_on_meanwhile_stays_where_the_manager_put_it() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        let done = workspace.status_of(StatusCategory::Done);
        workspace
            .tasks
            .move_to(workspace.schematic.id(), done, 0)
            .unwrap();

        workspace.play_whole_turn(0);

        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            done
        );
    }

    #[test]
    fn retries_warnings_and_refused_tools_are_shown_and_plan_usage_is_not() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.deliver(
            0,
            AgentEvent::Retrying {
                attempt: Some(1),
                max_attempts: Some(10),
                delay: None,
                reason: None,
            },
        );
        workspace.deliver(
            0,
            AgentEvent::Retrying {
                attempt: Some(2),
                max_attempts: Some(10),
                delay: None,
                reason: Some("overloaded".to_owned()),
            },
        );
        workspace.deliver(
            0,
            AgentEvent::Warning {
                message: "A plugin could not be loaded.".to_owned(),
            },
        );
        workspace.deliver(
            0,
            AgentEvent::PermissionDenied(PermissionDenial {
                tool: "Bash".to_owned(),
                tool_use_id: None,
            }),
        );
        workspace.deliver(
            0,
            AgentEvent::PlanUsage(crate::agent_runtime::PlanUsage {
                windows: Vec::new(),
            }),
        );

        let steps: Vec<StepContent> = workspace.only_conversation().turns[0]
            .steps()
            .iter()
            .map(|step| step.content.clone())
            .collect();
        assert_eq!(
            steps,
            [
                StepContent::Notice {
                    message: "A request to the model failed and is being tried again.".to_owned()
                },
                StepContent::Notice {
                    message: "A request to the model failed and is being tried again: overloaded"
                        .to_owned()
                },
                StepContent::Notice {
                    message: "A plugin could not be loaded.".to_owned()
                },
                StepContent::Denied {
                    tool: "Bash".to_owned()
                },
            ]
        );
        assert_eq!(workspace.ports.listener.changes().len(), 5);
    }

    #[test]
    fn a_failed_turn_says_why_in_words_and_leaves_the_task_in_work() {
        let cases = [
            (
                TurnFailure::NotSignedIn,
                "Not logged in",
                "Claude Code is not signed in. Open a terminal, run \"claude auth login\", and \
                 sign in.",
            ),
            (
                TurnFailure::Reported { kind: None },
                "  The file is locked.  ",
                "The file is locked.",
            ),
            (
                TurnFailure::Reported {
                    kind: Some("error_during_execution".to_owned()),
                },
                "",
                "The specialist could not finish.",
            ),
            (
                TurnFailure::ProgramEnded {
                    message: "Claude Code ended with exit code 1 before the turn was finished"
                        .to_owned(),
                },
                "",
                "Claude Code ended with exit code 1 before the turn was finished",
            ),
        ];
        for (failure, final_text, expected) in cases {
            let workspace = workspace();
            workspace.work.start_work(workspace.schematic.id()).unwrap();

            workspace.deliver(0, ended(Some(failure), final_text));

            let turn = &workspace.only_conversation().turns[0];
            assert_eq!(turn.state(), TurnState::Failed);
            assert_eq!(turn.failure(), Some(expected));
            assert_eq!(turn.answer(), "");
            assert_eq!(
                workspace.stored_task(workspace.schematic.id()).status_id(),
                workspace.status_of(StatusCategory::Active)
            );
        }
    }

    #[test]
    fn events_after_the_end_of_a_turn_change_nothing() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);
        let before = workspace.only_conversation();
        let changes_before = workspace.ports.listener.changes().len();

        workspace.deliver(
            0,
            AgentEvent::Text {
                text: "Late.".to_owned(),
            },
        );
        workspace.deliver(0, ended(Some(TurnFailure::Stopped), ""));

        assert_eq!(workspace.only_conversation(), before);
        assert_eq!(workspace.ports.listener.changes().len(), changes_before);
    }

    // Stopping

    #[test]
    fn stopping_ends_the_running_turn_as_stopped_by_the_manager() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.deliver(0, session_started());

        let stopped = workspace.work.stop(started.conversation.id()).unwrap();
        workspace.deliver(0, ended(Some(TurnFailure::Stopped), ""));

        assert!(workspace.ports.runtime.was_stopped(0));
        assert_eq!(stopped.turns[0].state(), TurnState::Stopped);
        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(turn.state(), TurnState::Stopped);
        assert_eq!(turn.failure(), Some("Stopped by the manager."));
        assert_eq!(turn.answer(), "");
        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Active)
        );
    }

    #[test]
    fn a_turn_the_program_reports_as_stopped_ends_as_stopped() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.deliver(0, ended(Some(TurnFailure::Stopped), ""));

        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(turn.state(), TurnState::Stopped);
        assert_eq!(turn.tokens(), Some(tokens_of(runtime_tokens())));
    }

    #[test]
    fn stopping_a_conversation_in_which_nothing_runs_changes_nothing() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);
        let before = workspace.only_conversation();

        let same = workspace.work.stop(started.conversation.id()).unwrap();

        assert_eq!(same, before);
        assert_eq!(same.turns[0].state(), TurnState::Finished);
        assert!(!workspace.ports.runtime.was_stopped(0));
    }

    // Replying and asking

    #[test]
    fn a_reply_continues_the_session_with_the_words_of_the_manager() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        let replied = workspace
            .work
            .reply(started.conversation.id(), "  Use the smaller regulator.  ")
            .unwrap();

        assert_eq!(replied.conversation.id(), started.conversation.id());
        assert_eq!(replied.turns.len(), 2);
        let turn = &replied.turns[1];
        assert_eq!(turn.origin(), TurnOrigin::Message);
        assert_eq!(turn.request(), "Use the smaller regulator.");
        assert_eq!(turn.state(), TurnState::Running);
        let request = &workspace.ports.runtime.requests()[1];
        assert_eq!(
            request.session,
            SessionChoice::Resume(SessionId::parse(SESSION).unwrap())
        );
        assert_eq!(request.prompt, "Use the smaller regulator.");
    }

    #[test]
    fn a_reply_sends_a_task_that_waited_to_be_checked_back_to_work() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        workspace
            .work
            .reply(started.conversation.id(), "Use the smaller regulator.")
            .unwrap();

        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Active)
        );
    }

    #[test]
    fn an_empty_reply_is_refused() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        let error = workspace
            .work
            .reply(started.conversation.id(), "   ")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "the message must not be empty");
    }

    #[test]
    fn a_reply_while_the_specialist_works_is_refused() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();

        let error = workspace
            .work
            .reply(started.conversation.id(), "Faster, please.")
            .unwrap_err();

        assert_eq!(error.code(), "member_busy");
    }

    #[test]
    fn a_reply_to_a_specialist_that_left_the_team_is_refused() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);
        workspace.team.remove(workspace.engineer.id()).unwrap();

        let error = workspace
            .work
            .reply(started.conversation.id(), "Are you there?")
            .unwrap_err();

        assert_eq!(error.code(), "no_specialist");
        assert_eq!(
            error.to_string(),
            "the specialist of this conversation has left the team"
        );
    }

    #[test]
    fn asking_a_specialist_opens_a_conversation_that_is_not_about_a_task() {
        let workspace = workspace();

        let asked = workspace
            .work
            .ask(workspace.engineer.id(), "Which regulator do you prefer?")
            .unwrap();

        assert_eq!(
            asked.conversation.member_id(),
            Some(workspace.engineer.id())
        );
        assert_eq!(asked.conversation.task_id(), None);
        assert_eq!(asked.turns[0].origin(), TurnOrigin::Message);
        assert_eq!(asked.turns[0].request(), "Which regulator do you prefer?");
        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Todo)
        );
    }

    #[test]
    fn only_an_ai_specialist_can_be_asked() {
        let workspace = workspace();
        let person = workspace.person("Test engineer");

        let error = workspace
            .work
            .ask(person.id(), "Are you there?")
            .unwrap_err();

        assert_eq!(error.code(), "no_specialist");
        assert_eq!(error.to_string(), "Test engineer is not an AI specialist");
    }

    // Asking the manager to decide

    #[test]
    fn ends_the_turn_with_a_question_instead_of_a_report_and_leaves_the_task_in_work() {
        let workspace = workspace();

        let (choice, conversation) = workspace.asked();

        let turn = &conversation.turns[0];
        assert_eq!(turn.state(), TurnState::Finished);
        let decision = turn.decision().unwrap();
        assert_eq!(
            decision.question(),
            "Which way should I take for \"Choose the battery\"?"
        );
        assert_eq!(decision.recommended(), Some(0));
        assert_eq!(decision.answer(), None);
        let labels: Vec<&str> = decision
            .options()
            .iter()
            .map(|option| option.label.as_str())
            .collect();
        assert_eq!(labels, ["The proven way", "The compact way"]);
        assert_eq!(
            workspace.stored_task(choice.id()).status_id(),
            workspace.status_of(StatusCategory::Active)
        );
    }

    #[test]
    fn asks_nothing_in_a_turn_that_ends_in_a_report() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        workspace.play_whole_turn(0);

        assert_eq!(workspace.only_conversation().turns[0].decision(), None);
    }

    #[test]
    fn lists_the_conversations_that_wait_for_the_manager_the_longest_wait_first() {
        let workspace = workspace();
        let writer = workspace
            .team
            .add(NewMember {
                project_id: workspace.project.id(),
                name: "Writer".to_owned(),
                kind: MemberKind::Agent,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap();
        let second = workspace.task_for("Select the paper", Some(Assignee::Member(writer.id())));
        let (_, first) = workspace.asked();
        workspace.ports.clock.advance(TimeDelta::seconds(60));
        let later = workspace.work.start_work(second.id()).unwrap();
        assert_eq!(workspace.waiting(), [first.conversation.id()]);

        workspace.play_turn_that_asks(1, "Select the paper");

        assert_eq!(
            workspace.waiting(),
            [first.conversation.id(), later.conversation.id()]
        );
    }

    #[test]
    fn does_not_list_a_conversation_of_a_project_that_is_put_away() {
        let workspace = workspace();
        workspace.asked();

        workspace.projects.archive(workspace.project.id()).unwrap();

        assert!(workspace.waiting().is_empty());
    }

    #[test]
    fn takes_an_option_as_the_answer_and_sends_the_specialist_on_with_it() {
        let workspace = workspace();
        let (choice, conversation) = workspace.asked();
        workspace.ports.clock.advance(TimeDelta::seconds(60));

        let answered = workspace
            .work
            .answer_decision(conversation.conversation.id(), Some(1), "")
            .unwrap();

        assert_eq!(answered.turns.len(), 2);
        assert_eq!(
            answered.turns[0]
                .decision()
                .and_then(DecisionRequest::answer),
            Some(&teamwright_domain::DecisionAnswer {
                option: Some(1),
                note: String::new(),
                at: workspace.ports.clock.now(),
            })
        );
        let turn = &answered.turns[1];
        assert_eq!(turn.origin(), TurnOrigin::Decision);
        assert_eq!(turn.state(), TurnState::Running);
        assert_eq!(turn.request(), "Decision: The compact way");
        assert_eq!(turn.decision(), None);
        assert_eq!(
            workspace.conversation(conversation.conversation.id()),
            answered
        );
        assert!(workspace.waiting().is_empty());
        let request = &workspace.ports.runtime.requests()[1];
        assert_eq!(request.prompt, "Decision: The compact way");
        assert_eq!(
            request.session,
            SessionChoice::Resume(SessionId::parse(SESSION).unwrap())
        );

        workspace.play_whole_turn(1);
        assert_eq!(
            workspace.stored_task(choice.id()).status_id(),
            workspace.status_of(StatusCategory::Review)
        );
    }

    #[test]
    fn takes_words_of_the_manager_with_an_option_or_instead_of_one() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();
        let with_both = workspace
            .work
            .answer_decision(
                conversation.conversation.id(),
                Some(0),
                "  But keep the connector.  ",
            )
            .unwrap();
        assert_eq!(
            with_both.turns[1].request(),
            "Decision: The proven way\n\nBut keep the connector."
        );
        let answer = with_both.turns[0]
            .decision()
            .and_then(DecisionRequest::answer)
            .unwrap();
        assert_eq!(answer.option, Some(0));
        assert_eq!(answer.note, "But keep the connector.");
        workspace.play_whole_turn(1);

        let (_, second) = workspace.asked_about("Decide on the color");
        let in_words = workspace
            .work
            .answer_decision(
                second.conversation.id(),
                None,
                "Neither. Use the gray we already have.",
            )
            .unwrap();
        assert_eq!(
            in_words.turns[1].request(),
            "Neither. Use the gray we already have."
        );
        assert_eq!(
            in_words.turns[0]
                .decision()
                .and_then(DecisionRequest::answer)
                .map(|answer| answer.option),
            Some(None)
        );
    }

    #[test]
    fn refuses_an_answer_that_names_no_option_and_says_nothing() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();

        let error = workspace
            .work
            .answer_decision(conversation.conversation.id(), None, "  ")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "an answer names one of the options or says something in words"
        );
    }

    #[test]
    fn refuses_an_option_that_the_question_does_not_have() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();

        let error = workspace
            .work
            .answer_decision(conversation.conversation.id(), Some(2), "")
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "the question has no such option");
        assert_eq!(workspace.waiting().len(), 1);
        assert_eq!(workspace.ports.runtime.requests().len(), 1);
    }

    #[test]
    fn refuses_an_ordinary_message_while_a_question_is_open() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();

        let error = workspace
            .work
            .reply(conversation.conversation.id(), "Go on.")
            .unwrap_err();

        assert_eq!(error.code(), "decision_open");
        assert_eq!(
            error.to_string(),
            "Hardware engineer waits for a decision: answer its question first"
        );
    }

    #[test]
    fn refuses_an_answer_where_nothing_was_asked_or_was_answered_already() {
        let workspace = workspace();
        let report = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);
        let nothing_asked = workspace
            .work
            .answer_decision(report.conversation.id(), Some(0), "")
            .unwrap_err();
        assert_eq!(nothing_asked.code(), "no_decision_open");
        assert_eq!(
            nothing_asked.to_string(),
            "no question is open in this conversation"
        );

        let (_, conversation) = workspace.asked();
        let id = conversation.conversation.id();
        workspace.work.answer_decision(id, Some(0), "").unwrap();
        workspace.play_whole_turn(2);
        let twice = workspace.work.answer_decision(id, Some(1), "").unwrap_err();
        assert_eq!(twice.code(), "no_decision_open");
    }

    #[test]
    fn lets_the_specialist_start_on_something_else_while_it_waits_for_a_decision() {
        let workspace = workspace();
        workspace.asked();

        let other = workspace.work.start_work(workspace.schematic.id()).unwrap();

        assert_eq!(other.turns[0].state(), TurnState::Running);
    }

    #[test]
    fn refuses_an_answer_while_the_specialist_works_on_something_else() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let error = workspace
            .work
            .answer_decision(conversation.conversation.id(), Some(0), "")
            .unwrap_err();

        assert_eq!(error.code(), "member_busy");
        assert_eq!(workspace.waiting(), [conversation.conversation.id()]);
    }

    #[test]
    fn refuses_an_answer_to_a_specialist_that_left_the_team() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();
        workspace.team.remove(workspace.engineer.id()).unwrap();

        let error = workspace
            .work
            .answer_decision(conversation.conversation.id(), Some(0), "")
            .unwrap_err();

        assert_eq!(error.code(), "no_specialist");
    }

    #[test]
    fn an_answer_the_program_could_not_act_on_is_not_kept() {
        let workspace = workspace();
        let (_, conversation) = workspace.asked();
        workspace
            .ports
            .runtime
            .refuse_next_turn(AgentRuntimeError::NotInstalled);

        let error = workspace
            .work
            .answer_decision(conversation.conversation.id(), Some(0), "")
            .unwrap_err();

        assert_eq!(error.code(), "agent_not_installed");
        assert_eq!(workspace.waiting(), [conversation.conversation.id()]);
    }

    #[test]
    fn a_second_question_in_one_turn_is_not_recorded() {
        let workspace = workspace();
        let choice = workspace.task_for(
            "Choose the battery",
            Some(Assignee::Member(workspace.engineer.id())),
        );
        workspace.work.start_work(choice.id()).unwrap();

        workspace.deliver(0, AgentEvent::ManagerAsked(question_about("First")));
        workspace.deliver(0, AgentEvent::ManagerAsked(question_about("Second")));

        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(
            turn.decision().map(DecisionRequest::question),
            Some("Which way should I take for \"First\"?")
        );
    }

    // What happens to the record of the work

    #[test]
    fn the_record_of_the_work_goes_with_a_task_that_is_deleted() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        workspace.tasks.delete(workspace.schematic.id()).unwrap();

        assert!(
            workspace
                .work
                .list_for_project(workspace.project.id())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn the_record_of_the_work_stays_without_a_specialist_that_leaves() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        workspace.team.remove(workspace.engineer.id()).unwrap();

        let kept = workspace.only_conversation();
        assert_eq!(kept.conversation.member_id(), None);
        assert_eq!(kept.turns.len(), 1);
    }

    #[test]
    fn a_specialist_stays_on_the_team_while_it_is_working() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let error = workspace.team.remove(workspace.engineer.id()).unwrap_err();

        assert_eq!(error.code(), "member_busy");
        assert!(workspace.team.get(workspace.engineer.id()).is_ok());
    }

    #[test]
    fn the_record_of_the_work_goes_with_a_project_that_is_deleted() {
        let workspace = workspace();
        let other = workspace
            .projects
            .create(NewProject {
                name: "Display driver".to_owned(),
                ..NewProject::default()
            })
            .unwrap();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.play_whole_turn(0);

        workspace.projects.delete(workspace.project.id()).unwrap();

        let error = workspace
            .work
            .list_for_project(workspace.project.id())
            .unwrap_err();
        assert_eq!(error.code(), "not_found");
        assert!(
            workspace
                .work
                .list_for_project(other.id())
                .unwrap()
                .is_empty()
        );
        assert!(
            workspace
                .ports
                .conversations
                .running_turns()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_task_and_its_project_are_kept_while_a_specialist_works_on_the_task() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();

        let errors = [
            workspace
                .tasks
                .delete(workspace.schematic.id())
                .unwrap_err(),
            workspace
                .projects
                .delete(workspace.project.id())
                .unwrap_err(),
            workspace
                .projects
                .archive(workspace.project.id())
                .unwrap_err(),
        ];

        assert_eq!(
            errors.each_ref().map(ApplicationError::code),
            ["work_in_progress"; 3]
        );
        assert_eq!(
            errors[0].to_string(),
            "Hardware engineer is still working on SSN-1; stop the work first"
        );
        assert!(workspace.tasks.get(workspace.schematic.id()).is_ok());
        assert!(
            !workspace
                .projects
                .get(workspace.project.id())
                .unwrap()
                .is_archived()
        );
    }

    #[test]
    fn a_task_whose_subtask_is_worked_on_is_kept() {
        let workspace = workspace();
        let board = workspace.task_for("Design the board", None);
        workspace
            .tasks
            .create(NewTask {
                parent_id: Some(board.id()),
                assignee: Some(Assignee::Member(workspace.engineer.id())),
                ..NewTask::titled(workspace.project.id(), "Power supply")
            })
            .and_then(|power| workspace.work.start_work(power.id()))
            .unwrap();

        let error = workspace.tasks.delete(board.id()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Hardware engineer is still working on SSN-3; stop the work first"
        );
    }

    #[test]
    fn a_task_can_go_once_the_work_on_it_has_been_stopped() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.work.stop(started.conversation.id()).unwrap();

        workspace.tasks.delete(workspace.schematic.id()).unwrap();

        assert!(
            workspace
                .work
                .list_for_project(workspace.project.id())
                .unwrap()
                .is_empty()
        );
    }

    // The desktop app, and starting again

    #[test]
    fn a_conversation_opens_in_the_desktop_app_once_its_session_has_started() {
        let workspace = workspace();
        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();
        let id = started.conversation.id();

        let too_early = workspace.work.open_in_desktop_app(id).unwrap_err();
        workspace.deliver(0, session_started());
        workspace.work.open_in_desktop_app(id).unwrap();

        assert_eq!(too_early.code(), "session_not_started");
        assert_eq!(
            workspace.ports.runtime.opened_sessions(),
            [(
                SessionId::parse(SESSION).unwrap(),
                PathBuf::from("projects").join("SSN")
            )]
        );
    }

    #[test]
    fn closing_the_program_stops_the_specialists_and_says_so_in_their_turns() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.deliver(0, session_started());

        workspace.work.close().unwrap();
        workspace.deliver(0, ended(Some(TurnFailure::Stopped), ""));

        assert!(workspace.ports.runtime.was_stopped(0));
        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(turn.state(), TurnState::Failed);
        assert_eq!(
            turn.failure(),
            Some("Teamwright was closed while the specialist was working."),
            "what the program says after it was stopped does not change the record"
        );
        assert_eq!(
            workspace.stored_task(workspace.schematic.id()).status_id(),
            workspace.status_of(StatusCategory::Active),
            "the task stays in work"
        );
    }

    #[test]
    fn turns_cut_off_by_closing_the_program_end_as_failed_at_the_next_start() {
        let workspace = workspace();
        workspace.work.start_work(workspace.schematic.id()).unwrap();
        workspace.work.recover().unwrap();
        assert_eq!(
            workspace.only_conversation().turns[0].state(),
            TurnState::Running,
            "a turn this run started is really running"
        );

        let after_a_restart = workspace.ports.work_service();
        after_a_restart.recover().unwrap();

        let turn = &workspace.only_conversation().turns[0];
        assert_eq!(turn.state(), TurnState::Failed);
        assert_eq!(
            turn.failure(),
            Some("Teamwright was closed while the specialist was working.")
        );
        assert!(
            workspace.team.remove(workspace.engineer.id()).is_ok(),
            "the specialist is no longer busy"
        );
    }

    // Conversations that run on another computer

    impl Workspace {
        /// Stores a conversation of the engineer that was started on
        /// another computer, as one that came over from it, with a turn
        /// that runs there.
        fn running_elsewhere(&self) -> Conversation {
            let conversation = Conversation::start(
                self.project.id(),
                self.engineer.id(),
                None,
                start_of_day(),
                ComputerId::generate(),
            );
            let turn = Turn::start(
                conversation.id(),
                "Check the regulator.",
                TurnOrigin::Message,
                start_of_day(),
            )
            .unwrap();
            self.ports
                .conversations
                .save_conversation(&conversation)
                .unwrap();
            self.ports.conversations.save_turn(&turn).unwrap();
            conversation
        }
    }

    #[test]
    fn a_conversation_started_here_has_this_computer_as_its_home() {
        let workspace = workspace();

        let started = workspace.work.start_work(workspace.schematic.id()).unwrap();

        assert_eq!(
            started.conversation.home_computer(),
            workspace.ports.this_computer
        );
    }

    #[test]
    fn starting_and_closing_leave_the_turns_of_another_computer_running() {
        let workspace = workspace();
        let elsewhere = workspace.running_elsewhere();

        workspace.ports.work_service().recover().unwrap();
        workspace.work.close().unwrap();

        let turns = workspace
            .ports
            .conversations
            .turns_of(elsewhere.id())
            .unwrap();
        assert_eq!(turns[0].state(), TurnState::Running);
    }

    #[test]
    fn a_conversation_of_another_computer_is_only_read_here() {
        let workspace = workspace();
        let elsewhere = workspace.running_elsewhere();

        let refused = [
            workspace.work.reply(elsewhere.id(), "Go on.").unwrap_err(),
            workspace
                .work
                .answer_decision(elsewhere.id(), Some(0), "")
                .unwrap_err(),
            workspace.work.stop(elsewhere.id()).unwrap_err(),
            workspace
                .work
                .open_in_desktop_app(elsewhere.id())
                .unwrap_err(),
        ];

        assert_eq!(
            refused.each_ref().map(ApplicationError::code),
            ["runs_elsewhere"; 4]
        );
        assert!(workspace.ports.runtime.requests().is_empty());
        let turns = workspace
            .ports
            .conversations
            .turns_of(elsewhere.id())
            .unwrap();
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].state(), TurnState::Running);
    }

    #[test]
    fn a_specialist_working_on_another_computer_stays_on_the_team() {
        let workspace = workspace();
        workspace.running_elsewhere();

        let error = workspace.team.remove(workspace.engineer.id()).unwrap_err();

        assert_eq!(error.code(), "member_busy");
    }

    // The brief and the instructions

    #[test]
    fn the_brief_names_the_task_and_what_the_manager_expects_back() {
        let workspace = workspace();

        assert_eq!(
            brief_for(
                &workspace.project,
                &workspace.schematic,
                std::slice::from_ref(&workspace.schematic)
            ),
            [
                "Your task is SSN-1: Draw the schematic",
                "",
                "When you are done, end with a short report for your manager:",
                "1. what you did,",
                "2. which files you created or changed,",
                "3. what you decided on your own, and why,",
                "4. what is still open.",
            ]
            .join("\n")
        );
    }

    #[test]
    fn the_brief_adds_details_parent_parts_what_it_builds_on_and_when_it_is_due() {
        let workspace = workspace();
        let project = workspace.project.id();
        let tasks = &workspace.tasks;
        // SSN-1 is the schematic of the workspace; these are SSN-2 to SSN-5.
        let board = tasks
            .create(NewTask::titled(project, "Design the board"))
            .unwrap();
        let pins = tasks
            .create(NewTask::titled(project, "Define the pin map"))
            .unwrap();
        let schematic = tasks
            .create(NewTask {
                description: "  Keep the board under 40 by 60 mm.  ".to_owned(),
                parent_id: Some(board.id()),
                due_on: CalendarDate::from_ymd_opt(2026, 11, 14),
                ..NewTask::titled(project, "Draw the schematic")
            })
            .unwrap();
        tasks
            .create(NewTask {
                parent_id: Some(schematic.id()),
                ..NewTask::titled(project, "Power supply")
            })
            .unwrap();
        let waiting = tasks.add_dependency(schematic.id(), pins.id()).unwrap();
        let all = tasks.list_for_project(project).unwrap();

        assert_eq!(
            brief_for(&workspace.project, &waiting, &all),
            [
                "Your task is SSN-4: Draw the schematic",
                "",
                "Keep the board under 40 by 60 mm.",
                "",
                "It is a part of SSN-2: Design the board",
                "",
                "Its parts are:",
                "- SSN-5: Power supply (To do)",
                "",
                "It builds on these tasks:",
                "- SSN-3: Define the pin map (To do)",
                "",
                "It is due on 2026-11-14.",
                "",
                "When you are done, end with a short report for your manager:",
                "1. what you did,",
                "2. which files you created or changed,",
                "3. what you decided on your own, and why,",
                "4. what is still open.",
            ]
            .join("\n")
        );
    }

    #[test]
    fn the_instructions_say_who_the_specialist_is_and_how_the_team_works() {
        let workspace = workspace();
        let without_responsibility = workspace
            .team
            .set_responsibility(workspace.engineer.id(), "")
            .unwrap();

        let instructions = instructions_for(&workspace.engineer, &workspace.project, &[]);
        let shorter = instructions_for(&without_responsibility, &workspace.project, &[]);

        let lines: Vec<&str> = instructions.lines().collect();
        assert_eq!(
            lines[..4],
            [
                "You are Hardware engineer, a member of the team that works on the project \
                 \"Smart sensor node\" for a human manager. The team is run with the program \
                 Teamwright.",
                "What you are on the team for: The schematic, the parts and the board.",
                "",
                "How the team works:",
            ]
        );
        assert_eq!(lines.len(), 10);
        assert_eq!(
            lines[6],
            "- Single files the manager handed you are copies in the folder references inside \
             the folder of the project."
        );
        assert!(lines[9].starts_with("- Write in plain words."));
        assert!(lines[8].contains("ask with the tool ask_manager"));
        assert!(lines[8].ends_with("The answer comes as your next message."));
        assert_eq!(shorter.lines().count(), 9);
        assert!(!instructions.contains("Besides the folder of the project"));
        assert!(!shorter.contains("What you are on the team for"));
    }
}
