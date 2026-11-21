//! The team bus: the tool server through which a session reaches the
//! manager.
//!
//! Claude Code starts it for a turn as an MCP server over standard input
//! and output: one JSON-RPC 2.0 message per line. It offers the tools the
//! turn was given ([`TeamTool`]):
//!
//! - `ask_manager`, which a specialist calls when a choice is the
//!   manager's to make;
//! - `interview`, with which the Advisor asks what it needs to know;
//! - `propose_plan`, with which the Advisor proposes a team and a plan;
//! - `publish_handoff`, with which a specialist hands the result of its
//!   task over to whoever builds on it;
//! - `send_message`, with which a specialist writes to a teammate.
//!
//! The server only checks a call and answers the session: a question,
//! interview or proposal ends the turn, a handoff or a message does not.
//! What was said reaches Teamwright through the output of the turn, where
//! the decoder finds the call and checks it with the same functions:
//! [`question_from`], [`interview_from`], [`proposal_from`],
//! [`handoff_from`] and [`message_from`]. A handoff is also checked against
//! the disk, which only the server does: the decoder reports a handoff or a
//! message only once the server has taken it.
//!
//! The tools are named to the server as arguments after the ones it is
//! configured with ([`bus_arguments`], [`tools_from_arguments`]). The
//! server keeps no state beyond one turn: Claude Code starts it with the
//! turn and leaves it when the turn ends.

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use teamwright_application::agent_runtime::Priority;
use teamwright_application::{
    Effort, InterviewQuestionAsked, ManagerInterview, ManagerQuestion, PlanProposal, PlannedMember,
    PlannedOwner, PlannedTask, PublishedHandoff, QuestionOption, SentMessage, TeamTool,
};
use teamwright_domain::{
    HandoffContent, MemberName, MessageKind, ModelName, PathInFolder, TaskTitle, TeamMessage,
};

/// The name of the tool server, as Claude Code is told it.
pub const SERVER_NAME: &str = "teamwright";

/// The name of the tool to ask the manager to decide.
pub const ASK_MANAGER: &str = "ask_manager";

/// The name of the tool with which the Advisor asks its questions.
pub const INTERVIEW: &str = "interview";

/// The name of the tool with which the Advisor proposes a team and a plan.
pub const PROPOSE_PLAN: &str = "propose_plan";

/// The name of the tool with which a specialist hands a result over.
pub const PUBLISH_HANDOFF: &str = "publish_handoff";

/// The name of the tool with which a specialist writes to a teammate.
pub const SEND_MESSAGE: &str = "send_message";

/// What Claude Code puts in front of the name of a tool of this server.
const PREFIX_IN_SESSION: &str = "mcp__teamwright__";

/// The name of `ask_manager` inside a session of Claude Code.
pub const ASK_MANAGER_IN_SESSION: &str = "mcp__teamwright__ask_manager";

/// The name of `interview` inside a session of Claude Code.
pub const INTERVIEW_IN_SESSION: &str = "mcp__teamwright__interview";

/// The name of `propose_plan` inside a session of Claude Code.
pub const PROPOSE_PLAN_IN_SESSION: &str = "mcp__teamwright__propose_plan";

/// The name of `publish_handoff` inside a session of Claude Code.
pub const PUBLISH_HANDOFF_IN_SESSION: &str = "mcp__teamwright__publish_handoff";

/// The name of `send_message` inside a session of Claude Code.
pub const SEND_MESSAGE_IN_SESSION: &str = "mcp__teamwright__send_message";

/// The argument before the name of a member of the team.
const MEMBER_ARGUMENT: &str = "--member";

/// The argument before the name of a teammate a message may go to.
const TEAMMATE_ARGUMENT: &str = "--teammate";

/// The argument before the folder the files of a handoff are in.
const FOLDER_ARGUMENT: &str = "--folder";

/// What a session is told when its handoff has been taken. It goes on.
pub const HANDOFF_TAKEN: &str = "Your handoff is recorded for whoever builds on this task. Go \
                                 on, and end your turn with your report to the manager.";

/// What a session is told when its message has been taken. It goes on.
pub const MESSAGE_TAKEN: &str = "Your message is on its way. The teammate reads it when it \
                                 next starts to work, so do not wait for an answer in this turn.";

/// What a session is told when it hands over a second time in one turn.
const HANDOFF_ALREADY_TAKEN: &str = "You have published a handoff in this turn already, and \
                                     only one is taken. Say what changed in your report.";

/// What a session is told when it has sent as many messages as a turn may.
const TOO_MANY_MESSAGES: &str = "You have sent as many messages as one turn may. Say the rest \
                                 in your report.";

/// The most messages a session sends in one turn.
const MOST_MESSAGES: usize = 5;

/// What a session is told when its question has been taken.
pub const QUESTION_TAKEN: &str = "Your question is with the manager. End your turn now with one \
                                  sentence that says you are waiting for the decision.";

/// What the Advisor is told when its questions have been taken.
pub const INTERVIEW_TAKEN: &str = "Your questions are with the manager. End your turn now with \
                                   one sentence that says you are waiting for the answers.";

/// What the Advisor is told when its proposal has been taken.
pub const PROPOSAL_TAKEN: &str = "Your proposal is with the manager. End your turn now with one \
                                  or two sentences that say what you proposed.";

/// What a session is told when it asks or proposes a second time in one
/// turn.
const ALREADY_TAKEN: &str = "You have already asked the manager or proposed in this turn, and \
                             only one is taken. End your turn now with one sentence that says \
                             what you are waiting for.";

/// What the tool `ask_manager` tells the model it is for.
const ASK_MANAGER_DESCRIPTION: &str = "Ask the manager of the project to decide. Use it when a \
     choice changes what is built, what it costs or when it is ready: such a choice is the \
     manager's, so do not guess. Give the question, why it matters, two to four options with \
     what each leads to, and the one you recommend. After calling it, end your turn: the answer \
     comes as the next message.";

/// What the tool `interview` tells the model it is for.
const INTERVIEW_DESCRIPTION: &str = "Ask the manager what you need to know before you can \
     propose a team and a plan. Ask only questions whose answer changes the team or the plan, \
     one to five at a time. For each, say why you ask. Where you can, offer two to four options \
     with what each leads to, and recommend one. A question without options is answered in \
     words. After calling it, end your turn: the answers come as your next message.";

/// What the tool `propose_plan` tells the model it is for.
const PROPOSE_PLAN_DESCRIPTION: &str = "Propose the team and the plan of the project to the \
     manager, who can change them and approves them. Nothing is created before the manager \
     approves. If the project has members or tasks already, propose only the members and the \
     tasks that are to be added. After calling it, end your turn.";

/// What the tool `publish_handoff` tells the model it is for.
const PUBLISH_HANDOFF_DESCRIPTION: &str = "Hand the result of your task over to whoever builds \
     on it. Teammates do not read your conversation: this is what they get. Give a short summary \
     of what you did and what it means for them, the interface contract they may rely on (if \
     there is one), and the files that hold the result, as paths inside the folder of the \
     project. Every file must exist. Call it once, when your work is done, before your report; \
     your turn goes on.";

/// What the tool `send_message` tells the model it is for.
const SEND_MESSAGE_DESCRIPTION: &str = "Send a question or a note to one teammate, by name. \
     Use it when a teammate must know something, or when only a teammate can answer. It \
     reaches the teammate at the start of its next turn, so do not wait for an answer now: \
     your turn goes on. A choice that is the manager's goes to ask_manager instead.";

/// The fewest and the most options a question to decide may have.
const FEWEST_OPTIONS: usize = 2;
const MOST_OPTIONS: usize = 4;

/// The most questions of an interview.
const MOST_QUESTIONS: usize = 5;

/// The most members and tasks of a proposal.
const MOST_MEMBERS: usize = 8;
const MOST_TASKS: usize = 40;

/// The owner of a task that is the manager.
const MANAGER: &str = "manager";

/// The JSON-RPC error for a method the server does not have.
const METHOD_NOT_FOUND: i64 = -32_601;

/// The JSON-RPC error for a call with parameters the server cannot use.
const INVALID_PARAMS: i64 = -32_602;

/// The protocol version answered to a client that names none.
const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";

/// How Claude Code starts the tool server: a program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamBus {
    /// The program that serves the bus.
    pub(crate) program: PathBuf,
    /// The arguments it is started with, before the tools of a turn.
    pub(crate) arguments: Vec<String>,
}

impl TeamBus {
    /// The configuration of tool servers that Claude Code is given with
    /// `--mcp-config`: this one server, under its name, offering `tools`.
    pub(crate) fn configuration(&self, tools: &[TeamTool]) -> String {
        let mut arguments = self.arguments.clone();
        arguments.extend(bus_arguments(tools));
        json!({
            "mcpServers": {
                SERVER_NAME: {
                    "command": self.program.to_string_lossy(),
                    "args": arguments,
                }
            }
        })
        .to_string()
    }
}

/// The name of a tool, as the server offers it.
#[must_use]
pub const fn name_of(tool: &TeamTool) -> &'static str {
    match tool {
        TeamTool::AskManager => ASK_MANAGER,
        TeamTool::Interview => INTERVIEW,
        TeamTool::ProposePlan { .. } => PROPOSE_PLAN,
        TeamTool::PublishHandoff { .. } => PUBLISH_HANDOFF,
        TeamTool::SendMessage { .. } => SEND_MESSAGE,
    }
}

/// The name of a tool inside a session of Claude Code, which puts the name
/// of its server in front of it.
#[must_use]
pub fn name_in_session(tool: &TeamTool) -> String {
    format!("{PREFIX_IN_SESSION}{}", name_of(tool))
}

/// The arguments that name the tools to the server: the name of each tool;
/// `--member <name>` for each member the project has when it may propose a
/// plan; `--folder <path>` for the folder of a handoff; and
/// `--teammate <name>` for each teammate a message may go to.
#[must_use]
pub fn bus_arguments(tools: &[TeamTool]) -> Vec<String> {
    let mut arguments = Vec::new();
    for tool in tools {
        arguments.push(name_of(tool).to_owned());
        match tool {
            TeamTool::ProposePlan { team } => {
                for name in team {
                    arguments.push(MEMBER_ARGUMENT.to_owned());
                    arguments.push(name.clone());
                }
            }
            TeamTool::PublishHandoff { folder } => {
                arguments.push(FOLDER_ARGUMENT.to_owned());
                arguments.push(folder.to_string_lossy().into_owned());
            }
            TeamTool::SendMessage { teammates } => {
                for name in teammates {
                    arguments.push(TEAMMATE_ARGUMENT.to_owned());
                    arguments.push(name.clone());
                }
            }
            TeamTool::AskManager | TeamTool::Interview => {}
        }
    }
    arguments
}

/// Reads the tools back from the arguments [`bus_arguments`] wrote.
///
/// # Errors
///
/// Returns what is wrong, in words, if an argument names no tool, if a
/// `--member`, `--teammate` or `--folder` has nothing after it, or if
/// `publish_handoff` has no folder.
pub fn tools_from_arguments(arguments: &[String]) -> Result<Vec<TeamTool>, String> {
    let mut tools = Vec::new();
    let mut team = Vec::new();
    let mut teammates = Vec::new();
    let mut folder = None;
    let mut given = arguments.iter();
    let value_after = |given: &mut std::slice::Iter<'_, String>, argument: &str| {
        given
            .next()
            .cloned()
            .ok_or_else(|| format!("{argument} needs a value after it"))
    };
    while let Some(argument) = given.next() {
        match argument.as_str() {
            ASK_MANAGER => tools.push(TeamTool::AskManager),
            INTERVIEW => tools.push(TeamTool::Interview),
            PROPOSE_PLAN => tools.push(TeamTool::ProposePlan { team: Vec::new() }),
            PUBLISH_HANDOFF => tools.push(TeamTool::PublishHandoff {
                folder: PathBuf::new(),
            }),
            SEND_MESSAGE => tools.push(TeamTool::SendMessage {
                teammates: Vec::new(),
            }),
            MEMBER_ARGUMENT => team.push(value_after(&mut given, MEMBER_ARGUMENT)?),
            TEAMMATE_ARGUMENT => teammates.push(value_after(&mut given, TEAMMATE_ARGUMENT)?),
            FOLDER_ARGUMENT => {
                folder = Some(PathBuf::from(value_after(&mut given, FOLDER_ARGUMENT)?))
            }
            other => return Err(format!("the team bus has no tool {other}")),
        }
    }
    for tool in &mut tools {
        match tool {
            TeamTool::ProposePlan { team: members } => members.clone_from(&team),
            TeamTool::SendMessage { teammates: names } => names.clone_from(&teammates),
            TeamTool::PublishHandoff { folder: place } => {
                *place = folder
                    .clone()
                    .ok_or_else(|| format!("{PUBLISH_HANDOFF} needs {FOLDER_ARGUMENT}"))?;
            }
            TeamTool::AskManager | TeamTool::Interview => {}
        }
    }
    Ok(tools)
}

/// Serves the tool server with these tools until the input ends.
///
/// Every line of input is one message. A line that is not JSON is passed
/// over, a notification is not answered, and every answer is one line,
/// written at once.
///
/// # Errors
///
/// Returns an error if the input cannot be read or the output cannot be
/// written.
pub fn serve(
    mut input: impl BufRead,
    mut output: impl Write,
    tools: Vec<TeamTool>,
) -> io::Result<()> {
    let mut bus = Bus::offering(tools);
    let mut line = Vec::new();
    loop {
        line.clear();
        if input.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        if let Some(answer) = bus.answer(&String::from_utf8_lossy(&line)) {
            output.write_all(answer.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

/// Reads and checks the arguments of a call of `ask_manager`.
///
/// The question must say something, there must be two to four options,
/// each with a label, and a recommended option must be one of them. The
/// model counts the recommended option from 1; the question that comes out
/// counts it from 0. Whitespace around the texts is removed.
///
/// # Errors
///
/// Returns what to change, in plain words, if the arguments are not
/// acceptable. The model reads it and tries again.
pub fn question_from(arguments: &Value) -> Result<ManagerQuestion, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("Give the question as an object with \"question\" and \"options\".".to_owned());
    };
    let question = optional_text(arguments, "question")?.unwrap_or_default();
    if question.is_empty() {
        return Err(
            "The question is empty. Write what the manager is to decide in \
                    \"question\"."
                .to_owned(),
        );
    }
    let reason = optional_text(arguments, "reason")?.unwrap_or_default();

    let Some(given) = arguments.get("options").and_then(Value::as_array) else {
        return Err(format!(
            "There are no options. Give {FEWEST_OPTIONS} to {MOST_OPTIONS} options in \
             \"options\", each with a \"label\" and a \"consequence\"."
        ));
    };
    if !(FEWEST_OPTIONS..=MOST_OPTIONS).contains(&given.len()) {
        return Err(format!(
            "There are {} options. Give {FEWEST_OPTIONS} to {MOST_OPTIONS} options in \
             \"options\".",
            given.len()
        ));
    }
    let options = given
        .iter()
        .enumerate()
        .map(|(index, option)| option_from(index + 1, option))
        .collect::<Result<Vec<_>, _>>()?;

    let recommended = match arguments.get("recommended") {
        None | Some(Value::Null) => None,
        Some(place) => Some(place_from(place, options.len()).ok_or_else(|| {
            format!(
                "\"recommended\" is the place of the option you recommend in the list, \
                 a whole number from 1 to {}.",
                options.len()
            )
        })?),
    };

    Ok(ManagerQuestion {
        question,
        reason,
        options,
        recommended,
    })
}

/// Reads and checks the arguments of a call of `interview`.
///
/// There must be one to five questions. Each says something and has two
/// to four options, each with a label, or none for a question answered in
/// words; a recommended option must be one of its options. The model
/// counts the recommended option from 1; the interview that comes out
/// counts it from 0. Whitespace around the texts is removed.
///
/// # Errors
///
/// Returns what to change, in plain words, if the arguments are not
/// acceptable. The model reads it and tries again.
pub fn interview_from(arguments: &Value) -> Result<ManagerInterview, String> {
    let questions = arguments
        .get("questions")
        .and_then(Value::as_array)
        .filter(|questions| (1..=MOST_QUESTIONS).contains(&questions.len()))
        .ok_or_else(|| "Ask one to five questions in \"questions\".".to_owned())?;
    let questions = questions
        .iter()
        .enumerate()
        .map(|(index, question)| interview_question_from(index + 1, question))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ManagerInterview { questions })
}

/// Reads one question of an interview; `place` counts from 1.
fn interview_question_from(
    place: usize,
    question: &Value,
) -> Result<InterviewQuestionAsked, String> {
    let Some(question) = question.as_object() else {
        return Err(format!(
            "Question {place} is not an object. Give each question a \"question\" and a \
             \"reason\"."
        ));
    };
    let text = optional_text(question, "question")?.unwrap_or_default();
    if text.is_empty() {
        return Err(format!("Question {place} is empty."));
    }
    let reason = optional_text(question, "reason")?.unwrap_or_default();
    let given = match question.get("options") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(options)) => options.clone(),
        Some(_) => return Err(format!("The options of question {place} must be a list.")),
    };
    if given.len() == 1 || given.len() > MOST_OPTIONS {
        return Err(format!(
            "Question {place} has {} options. Give two to four, or none.",
            given.len()
        ));
    }
    let options = given
        .iter()
        .map(|option| {
            let label = option
                .as_object()
                .map(|option| optional_text(option, "label"))
                .transpose()?
                .flatten()
                .unwrap_or_default();
            if label.is_empty() {
                return Err(format!("An option of question {place} has no label."));
            }
            let consequence = option
                .as_object()
                .map(|option| optional_text(option, "consequence"))
                .transpose()?
                .flatten()
                .unwrap_or_default();
            Ok(QuestionOption { label, consequence })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let recommended = match question.get("recommended") {
        None | Some(Value::Null) => None,
        Some(_) if options.is_empty() => {
            return Err(format!(
                "Question {place} has no options, so it recommends none. Leave \"recommended\" \
                 out."
            ));
        }
        Some(recommended) => Some(place_from(recommended, options.len()).ok_or_else(|| {
            format!(
                "\"recommended\" of question {place} is the place of an option, from 1 to {}.",
                options.len()
            )
        })?),
    };
    Ok(InterviewQuestionAsked {
        question: text,
        reason,
        options,
        recommended,
    })
}

/// Reads and checks the arguments of a call of `propose_plan`. `team` holds
/// the names of the members the project has already.
///
/// The summary says something. There are at most eight members, each with
/// a name that no other has and that nobody on the team has, whatever the
/// capitals, and that is not "manager"; a model is one word, an effort one
/// of low, medium, high, xhigh and max. There are one to forty tasks, each
/// with a title and an
/// owner: "manager", a proposed member or a member of the team, whatever
/// the capitals. A task waits only for other tasks of the list, counted
/// from 1 by the model and from 0 in the plan that comes out, and the
/// tasks do not wait for each other in a circle. An estimate in hours
/// becomes minutes, and the priority is normal unless it is said.
/// Whitespace around the texts is removed. Names and titles follow the
/// rules of members and tasks.
///
/// # Errors
///
/// Returns what to change, in plain words, if the arguments are not
/// acceptable. The model reads it and tries again.
pub fn proposal_from(arguments: &Value, team: &[String]) -> Result<PlanProposal, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err(
            "Give the proposal as an object with \"summary\", \"members\" and \"tasks\"."
                .to_owned(),
        );
    };
    let summary = optional_text(arguments, "summary")?.unwrap_or_default();
    if summary.is_empty() {
        return Err("The summary is empty.".to_owned());
    }

    let given_members = arguments
        .get("members")
        .and_then(Value::as_array)
        .filter(|members| members.len() <= MOST_MEMBERS)
        .ok_or_else(|| "Give the members as a list of at most eight.".to_owned())?;
    let on_the_team: HashSet<String> = team.iter().map(|name| name.trim().to_lowercase()).collect();
    let mut names = HashSet::new();
    let mut members = Vec::new();
    let mut there_already = Vec::new();
    for (index, member) in given_members.iter().enumerate() {
        let member = planned_member_from(index + 1, member)?;
        if !names.insert(member.name.to_lowercase()) {
            return Err(format!("Two members are called {}.", member.name));
        }
        if on_the_team.contains(&member.name.to_lowercase()) {
            there_already.push(member.name.clone());
        }
        members.push(member);
    }
    if !there_already.is_empty() {
        // A model that planned a project once tends to hand in the whole
        // plan again when it is asked for more. All the names are told at
        // once, so that one more try is enough.
        return Err(format!(
            "On the team already: {}. A proposal holds only what is to be added. Leave out the \
             members who are on the team and the tasks that exist, and propose again. A new \
             task may be given to a member who is on the team.",
            there_already.join(", ")
        ));
    }

    let given_tasks = arguments
        .get("tasks")
        .and_then(Value::as_array)
        .filter(|tasks| (1..=MOST_TASKS).contains(&tasks.len()))
        .ok_or_else(|| "Give one to forty tasks.".to_owned())?;
    let known: HashSet<String> = names.union(&on_the_team).cloned().collect();
    let tasks = given_tasks
        .iter()
        .enumerate()
        .map(|(index, task)| planned_task_from(index + 1, task, given_tasks.len(), &known))
        .collect::<Result<Vec<_>, _>>()?;
    if wait_in_circle(&tasks) {
        return Err(
            "The tasks wait for each other in a circle, so none of them could start. Let a task \
             wait only for tasks that do not wait for it."
                .to_owned(),
        );
    }
    Ok(PlanProposal {
        summary,
        members,
        tasks,
    })
}

/// Reads one member of a proposal; `place` counts from 1.
fn planned_member_from(place: usize, member: &Value) -> Result<PlannedMember, String> {
    let needs_a_name = || format!("Member {place} needs a name, and \"manager\" is taken.");
    let Some(member) = member.as_object() else {
        return Err(needs_a_name());
    };
    let name = optional_text(member, "name")?.unwrap_or_default();
    if name.is_empty() || name.eq_ignore_ascii_case(MANAGER) {
        return Err(needs_a_name());
    }
    let name = MemberName::parse(&name)
        .map_err(|problem| format!("The name of member {place} cannot be used: {problem}."))?
        .to_string();
    let model = optional_text(member, "model")?
        .filter(|model| !model.is_empty())
        .map(|model| {
            ModelName::parse(&model)
                .map(|model| model.to_string())
                .map_err(|_| {
                    format!("The model of member {place} is one word, such as \"sonnet\".")
                })
        })
        .transpose()?;
    let effort = optional_text(member, "effort")?
        .filter(|effort| !effort.is_empty())
        .map(|effort| {
            effort.parse::<Effort>().map_err(|_| {
                format!("The effort of member {place} is one of low, medium, high, xhigh and max.")
            })
        })
        .transpose()?;
    Ok(PlannedMember {
        name,
        responsibility: optional_text(member, "responsibility")?.unwrap_or_default(),
        reason: optional_text(member, "reason")?.unwrap_or_default(),
        model,
        effort,
    })
}

/// Reads one task of a proposal; `place` counts from 1, as do the places
/// it waits for. `known` holds the names an owner may have, in small
/// letters.
fn planned_task_from(
    place: usize,
    task: &Value,
    count: usize,
    known: &HashSet<String>,
) -> Result<PlannedTask, String> {
    let Some(task) = task.as_object() else {
        return Err(format!("Task {place} has no title."));
    };
    let title = optional_text(task, "title")?.unwrap_or_default();
    if title.is_empty() {
        return Err(format!("Task {place} has no title."));
    }
    let title = TaskTitle::parse(&title)
        .map_err(|problem| format!("The title of task {place} cannot be used: {problem}."))?
        .to_string();

    let owner_text = optional_text(task, "owner")?.unwrap_or_default();
    let owner = if owner_text.eq_ignore_ascii_case(MANAGER) {
        PlannedOwner::Manager
    } else if known.contains(&owner_text.to_lowercase()) {
        PlannedOwner::Member(owner_text)
    } else {
        return Err(format!(
            "The owner of task {place}, \"{owner_text}\", is not a member. Use a name from \
             members, the name of a member of the team, or \"manager\"."
        ));
    };

    let estimate_minutes = match task.get("estimate_hours") {
        None | Some(Value::Null) => None,
        Some(hours) => Some(
            hours
                .as_f64()
                .filter(|hours| hours.is_finite() && *hours >= 0.0)
                .and_then(minutes_of)
                .ok_or_else(|| {
                    format!("The estimate of task {place} is a number of hours, such as 4.")
                })?,
        ),
    };

    let priority = match optional_text(task, "priority")?.filter(|text| !text.is_empty()) {
        None => Priority::Normal,
        Some(priority) => priority.parse::<Priority>().map_err(|_| {
            format!("The priority of task {place} is one of low, normal, high and urgent.")
        })?,
    };

    let mut waits_for = Vec::new();
    match task.get("waits_for") {
        None | Some(Value::Null) => {}
        Some(Value::Array(earlier)) => {
            for value in earlier {
                let other = place_from(value, count)
                    .filter(|other| *other + 1 != place)
                    .ok_or_else(|| {
                        format!(
                            "Task {place} waits for {value}, which is not another task of the list."
                        )
                    })?;
                if !waits_for.contains(&other) {
                    waits_for.push(other);
                }
            }
        }
        Some(_) => {
            return Err(format!(
                "\"waits_for\" of task {place} is a list of places of other tasks, counted from 1."
            ));
        }
    }

    Ok(PlannedTask {
        title,
        description: optional_text(task, "description")?.unwrap_or_default(),
        owner,
        estimate_minutes,
        priority,
        waits_for,
    })
}

/// A number of hours as whole minutes, if it is not too large.
fn minutes_of(hours: f64) -> Option<u32> {
    let minutes = (hours * 60.0).round();
    // The bounds are checked first, so the conversion cannot cut anything.
    (0.0..=f64::from(u32::MAX))
        .contains(&minutes)
        .then_some(minutes as u32)
}

/// Whether some tasks wait for each other in a circle. Every place a task
/// waits for is one of the tasks.
fn wait_in_circle(tasks: &[PlannedTask]) -> bool {
    // 0: not seen, 1: on the way, 2: clear. Meeting a task that is on the
    // way closes a circle.
    fn closes_circle(place: usize, tasks: &[PlannedTask], marks: &mut [u8]) -> bool {
        match marks.get(place) {
            Some(1) => return true,
            Some(0) => {}
            _ => return false,
        }
        marks[place] = 1;
        let found = tasks[place]
            .waits_for
            .iter()
            .any(|&earlier| closes_circle(earlier, tasks, marks));
        marks[place] = 2;
        found
    }
    let mut marks = vec![0_u8; tasks.len()];
    (0..tasks.len()).any(|place| closes_circle(place, tasks, &mut marks))
}

/// Reads a place in a list of `count`, counted from 1 by the model, as a
/// place counted from 0.
fn place_from(value: &Value, count: usize) -> Option<usize> {
    value
        .as_u64()
        .and_then(|place| usize::try_from(place).ok())
        .filter(|place| (1..=count).contains(place))
        .map(|place| place - 1)
}

/// Reads one option; `place` counts from 1, as the model does.
fn option_from(place: usize, option: &Value) -> Result<QuestionOption, String> {
    let Some(option) = option.as_object() else {
        return Err(format!(
            "Option {place} is not an object. Give each option a \"label\" and a \"consequence\"."
        ));
    };
    let label = optional_text(option, "label")?.unwrap_or_default();
    if label.is_empty() {
        return Err(format!(
            "Option {place} has no label. Give every option a short \"label\"."
        ));
    }
    let consequence = optional_text(option, "consequence")?.unwrap_or_default();
    Ok(QuestionOption { label, consequence })
}

/// Reads and checks the arguments of a call of `publish_handoff`.
///
/// The summary says something, the contract may be left out, and the files
/// are a list of paths inside the working folder: none absolute, none that
/// goes up with `..`, none named twice, at most fifty. Paths are written
/// with `/` between their parts. Whether the files exist is a question for
/// the disk, which [`missing_files`] asks.
///
/// # Errors
///
/// Returns what to change, in plain words, if the arguments are not
/// acceptable. The model reads it and tries again.
pub fn handoff_from(arguments: &Value) -> Result<PublishedHandoff, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err(
            "Give the handoff as an object with \"summary\", \"contract\" and \"files\"."
                .to_owned(),
        );
    };
    let summary = optional_text(arguments, "summary")?.unwrap_or_default();
    let contract = optional_text(arguments, "contract")?.unwrap_or_default();
    let files = match arguments.get("files") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(files)) => files
            .iter()
            .map(|file| {
                file.as_str().map(str::to_owned).ok_or_else(|| {
                    "Every file is a path as text, inside the folder of the project.".to_owned()
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => {
            return Err(
                "\"files\" is a list of paths inside the folder of the project.".to_owned(),
            );
        }
    };
    let content = HandoffContent::new(&summary, &contract, &files).map_err(|problem| {
        format!(
            "The handoff cannot be recorded: {problem}. Paths are relative to the folder of the \
             project, such as \"hardware/pinmap.md\"."
        )
    })?;
    Ok(PublishedHandoff {
        summary: content.summary().to_owned(),
        contract: content.contract().to_owned(),
        files: content.files().iter().map(PathInFolder::as_text).collect(),
    })
}

/// The files of a handoff that are not there: not inside `folder`, or not
/// on the disk at all. A link that leads out of the folder counts as not
/// there.
#[must_use]
pub fn missing_files(handoff: &PublishedHandoff, folder: &Path) -> Vec<String> {
    let Ok(folder) = folder.canonicalize() else {
        return handoff.files.clone();
    };
    handoff
        .files
        .iter()
        .filter(|file| {
            let inside = folder
                .join(file.as_str())
                .canonicalize()
                .is_ok_and(|place| place.starts_with(&folder));
            !inside
        })
        .cloned()
        .collect()
}

/// Reads and checks the arguments of a call of `send_message`. `teammates`
/// holds the names a message may go to.
///
/// The message goes to one of the teammates, whatever the capitals; its
/// kind is "question" or "note"; its text says something and is not too
/// long. Whitespace around the texts is removed.
///
/// # Errors
///
/// Returns what to change, in plain words, if the arguments are not
/// acceptable. The model reads it and tries again.
pub fn message_from(arguments: &Value, teammates: &[String]) -> Result<SentMessage, String> {
    let Some(arguments) = arguments.as_object() else {
        return Err("Give the message as an object with \"to\", \"kind\" and \"text\".".to_owned());
    };
    let to = optional_text(arguments, "to")?.unwrap_or_default();
    let Some(teammate) = teammates
        .iter()
        .find(|name| name.trim().eq_ignore_ascii_case(&to))
    else {
        return Err(if teammates.is_empty() {
            "You have no teammate to write to.".to_owned()
        } else {
            format!(
                "\"{to}\" is not a teammate. Write to one of: {}.",
                teammates.join(", ")
            )
        });
    };
    let kind = optional_text(arguments, "kind")?
        .unwrap_or_default()
        .parse::<MessageKind>()
        .map_err(|_| "\"kind\" is \"question\" or \"note\".".to_owned())?;
    let text = optional_text(arguments, "text")?.unwrap_or_default();
    if text.is_empty() {
        return Err("The message is empty. Write it in \"text\".".to_owned());
    }
    let length = text.chars().count();
    if length > TeamMessage::MAX_LENGTH {
        return Err(format!(
            "The message has {length} characters. Keep it under {}.",
            TeamMessage::MAX_LENGTH
        ));
    }
    Ok(SentMessage {
        to: teammate.trim().to_owned(),
        kind,
        text,
    })
}

/// Reads a field that is text if it is there, trimmed.
fn optional_text(object: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.trim().to_owned())),
        Some(_) => Err(format!("\"{key}\" must be text.")),
    }
}

/// The tool server of one turn.
#[derive(Debug, Default)]
struct Bus {
    /// The tools it offers.
    tools: Vec<TeamTool>,
    /// Whether a question, an interview or a proposal has been taken in
    /// this turn. A turn carries at most one, and nothing is taken after
    /// it: the session is to end its turn.
    taken: bool,
    /// Whether a handoff has been taken in this turn. A turn carries at
    /// most one.
    handed_over: bool,
    /// How many messages have been taken in this turn.
    messages_sent: usize,
}

impl Bus {
    /// A server that offers these tools.
    const fn offering(tools: Vec<TeamTool>) -> Self {
        Self {
            tools,
            taken: false,
            handed_over: false,
            messages_sent: 0,
        }
    }

    /// Answers one line of input, or returns nothing if the line asks for
    /// no answer: it is not JSON, it is a notification, or it is a response.
    fn answer(&mut self, line: &str) -> Option<String> {
        let Ok(Value::Object(message)) = serde_json::from_str::<Value>(line.trim()) else {
            return None;
        };
        // The identifier can be a number or a text. It goes back as it came.
        let id = message.get("id").filter(|id| !id.is_null())?.clone();
        let method = message.get("method").and_then(Value::as_str)?;
        let parameters = message.get("params");
        let outcome = match method {
            "initialize" => Ok(initialized(parameters)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({
                "tools": self.tools.iter().map(description_of).collect::<Vec<_>>()
            })),
            "tools/call" => self.call(parameters),
            other => Err((METHOD_NOT_FOUND, format!("the method {other} is not known"))),
        };
        let answer = match outcome {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": code, "message": message },
            }),
        };
        Some(answer.to_string())
    }

    /// Calls a tool.
    fn call(&mut self, parameters: Option<&Value>) -> Result<Value, (i64, String)> {
        let name = parameters
            .and_then(|parameters| parameters.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(tool) = self.tools.iter().find(|tool| name_of(tool) == name) else {
            return Err((INVALID_PARAMS, format!("there is no tool {name}")));
        };
        if self.taken {
            return Ok(tool_result(ALREADY_TAKEN, true));
        }
        let arguments = parameters
            .and_then(|parameters| parameters.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        match tool {
            TeamTool::PublishHandoff { folder } => {
                let folder = folder.clone();
                return Ok(self.hand_over(&arguments, &folder));
            }
            TeamTool::SendMessage { teammates } => {
                let teammates = teammates.clone();
                return Ok(self.send(&arguments, &teammates));
            }
            TeamTool::AskManager | TeamTool::Interview | TeamTool::ProposePlan { .. } => {}
        }
        let (checked, taken, not_sent) = match tool {
            TeamTool::AskManager => (
                question_from(&arguments).map(|_| ()),
                QUESTION_TAKEN,
                "The question was not sent to the manager.",
            ),
            TeamTool::Interview => (
                interview_from(&arguments).map(|_| ()),
                INTERVIEW_TAKEN,
                "The questions were not sent to the manager.",
            ),
            TeamTool::ProposePlan { team } => (
                proposal_from(&arguments, team).map(|_| ()),
                PROPOSAL_TAKEN,
                "The proposal was not sent to the manager.",
            ),
            // Answered above; they do not end the turn.
            TeamTool::PublishHandoff { .. } | TeamTool::SendMessage { .. } => return Ok(json!({})),
        };
        Ok(match checked {
            Ok(()) => {
                self.taken = true;
                tool_result(taken, false)
            }
            Err(problem) => tool_result(&format!("{not_sent} {problem}"), true),
        })
    }
}

impl Bus {
    /// Takes a handoff whose files are all inside `folder`, once a turn.
    fn hand_over(&mut self, arguments: &Value, folder: &Path) -> Value {
        const NOT_RECORDED: &str = "The handoff was not recorded.";
        if self.handed_over {
            return tool_result(HANDOFF_ALREADY_TAKEN, true);
        }
        let handoff = match handoff_from(arguments) {
            Ok(handoff) => handoff,
            Err(problem) => return tool_result(&format!("{NOT_RECORDED} {problem}"), true),
        };
        let missing = missing_files(&handoff, folder);
        if !missing.is_empty() {
            return tool_result(
                &format!(
                    "{NOT_RECORDED} These files are not in the folder of the project: {}. Write \
                     them first, or leave them out.",
                    missing.join(", ")
                ),
                true,
            );
        }
        self.handed_over = true;
        tool_result(HANDOFF_TAKEN, false)
    }

    /// Takes a message to a teammate, up to the most a turn may send.
    fn send(&mut self, arguments: &Value, teammates: &[String]) -> Value {
        if self.messages_sent >= MOST_MESSAGES {
            return tool_result(TOO_MANY_MESSAGES, true);
        }
        match message_from(arguments, teammates) {
            Ok(_) => {
                self.messages_sent += 1;
                tool_result(MESSAGE_TAKEN, false)
            }
            Err(problem) => tool_result(&format!("The message was not sent. {problem}"), true),
        }
    }
}

/// The answer to `initialize`: the client's protocol version, and a server
/// that has tools.
fn initialized(parameters: Option<&Value>) -> Value {
    let version = parameters
        .and_then(|parameters| parameters.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The result of a call of a tool: one text.
fn tool_result(text: &str, is_error: bool) -> Value {
    let mut result = json!({ "content": [{ "type": "text", "text": text }] });
    if is_error {
        result["isError"] = Value::Bool(true);
    }
    result
}

/// The description of a tool, with the schema of its arguments.
fn description_of(tool: &TeamTool) -> Value {
    match tool {
        TeamTool::AskManager => ask_manager_tool(),
        TeamTool::Interview => interview_tool(),
        TeamTool::ProposePlan { .. } => propose_plan_tool(),
        TeamTool::PublishHandoff { .. } => publish_handoff_tool(),
        TeamTool::SendMessage { teammates } => send_message_tool(teammates),
    }
}

/// The description of `publish_handoff`.
fn publish_handoff_tool() -> Value {
    json!({
        "name": PUBLISH_HANDOFF,
        "description": PUBLISH_HANDOFF_DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "required": ["summary"],
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "What you did and what it means for whoever builds on it, in \
                                    a few sentences."
                },
                "contract": {
                    "type": "string",
                    "description": "The interface others may rely on: names, pins, formats, \
                                    calls, limits. Leave it out if there is none."
                },
                "files": {
                    "type": "array",
                    "maxItems": HandoffContent::MOST_FILES,
                    "items": { "type": "string" },
                    "description": "The files that hold the result, as paths inside the folder \
                                    of the project, such as \"hardware/pinmap.md\"."
                }
            }
        }
    })
}

/// The description of `send_message`, with the names it may go to.
fn send_message_tool(teammates: &[String]) -> Value {
    json!({
        "name": SEND_MESSAGE,
        "description": SEND_MESSAGE_DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "required": ["to", "kind", "text"],
            "properties": {
                "to": {
                    "type": "string",
                    "enum": teammates,
                    "description": "The name of the teammate."
                },
                "kind": {
                    "type": "string",
                    "enum": ["question", "note"],
                    "description": "A question that wants an answer, or a note that only tells."
                },
                "text": {
                    "type": "string",
                    "description": "The message, short and complete: the teammate has not read \
                                    your conversation."
                }
            }
        }
    })
}

/// The description of `ask_manager`.
fn ask_manager_tool() -> Value {
    json!({
        "name": ASK_MANAGER,
        "description": ASK_MANAGER_DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "What the manager is to decide, as one question."
                },
                "reason": {
                    "type": "string",
                    "description": "Why it matters: what the answer changes."
                },
                "options": {
                    "type": "array",
                    "minItems": FEWEST_OPTIONS,
                    "maxItems": MOST_OPTIONS,
                    "description": "The answers you see, two to four.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "label": {
                                "type": "string",
                                "description": "The option in a few words."
                            },
                            "consequence": {
                                "type": "string",
                                "description": "What choosing it leads to."
                            }
                        },
                        "required": ["label"]
                    }
                },
                "recommended": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MOST_OPTIONS,
                    "description": "The option you recommend, by its place in the list, \
                                    counted from 1."
                }
            },
            "required": ["question", "options"]
        }
    })
}

/// The description of `interview`, as the stand-in that was tried had it.
fn interview_tool() -> Value {
    json!({
        "name": INTERVIEW,
        "description": INTERVIEW_DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "required": ["questions"],
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MOST_QUESTIONS,
                    "items": {
                        "type": "object",
                        "required": ["question"],
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The question, in one sentence."
                            },
                            "reason": {
                                "type": "string",
                                "description": "Why you ask: what the answer changes."
                            },
                            "options": {
                                "type": "array",
                                "maxItems": MOST_OPTIONS,
                                "description": "The answers you see, two to four, or none for \
                                                a question that is answered in words.",
                                "items": {
                                    "type": "object",
                                    "required": ["label"],
                                    "properties": {
                                        "label": {
                                            "type": "string",
                                            "description": "The option in a few words."
                                        },
                                        "consequence": {
                                            "type": "string",
                                            "description": "What choosing it leads to."
                                        }
                                    }
                                }
                            },
                            "recommended": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": MOST_OPTIONS,
                                "description": "The option you recommend, by its place in the \
                                                list, counted from 1."
                            }
                        }
                    }
                }
            }
        }
    })
}

/// The description of `propose_plan`, as the stand-in that was tried had
/// it, with the efforts Teamwright knows and a model of any one word. What
/// it says about members and tasks that are there already was added when a
/// real Advisor, asked for more, proposed its whole plan a second time.
fn propose_plan_tool() -> Value {
    json!({
        "name": PROPOSE_PLAN,
        "description": PROPOSE_PLAN_DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "required": ["summary", "members", "tasks"],
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "The project in your words, for the manager to check: what \
                                    is built, what done means, what is left out, the \
                                    constraints. A few short paragraphs or a list."
                },
                "members": {
                    "type": "array",
                    "maxItems": MOST_MEMBERS,
                    "description": "The AI specialists to add to the project: the smallest \
                                    team that can do the job. Nobody who is on the team \
                                    already. The list is empty if nobody is to be added.",
                    "items": {
                        "type": "object",
                        "required": ["name", "responsibility", "reason"],
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The discipline as a name, for example \
                                                \"Firmware engineer\". No two members have the \
                                                same name."
                            },
                            "responsibility": {
                                "type": "string",
                                "description": "What this member is responsible for, in one \
                                                sentence."
                            },
                            "reason": {
                                "type": "string",
                                "description": "Why the project needs this member, in one \
                                                sentence."
                            },
                            "model": {
                                "type": "string",
                                "description": "The model the member runs on, one word such \
                                                as haiku, sonnet or opus. Leave it out to use \
                                                the default of the program."
                            },
                            "effort": {
                                "type": "string",
                                "enum": ["low", "medium", "high", "xhigh", "max"],
                                "description": "How much the member thinks before it acts. \
                                                Leave it out for the default."
                            }
                        }
                    }
                },
                "tasks": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MOST_TASKS,
                    "description": "The tasks to create. No task that exists already.",
                    "items": {
                        "type": "object",
                        "required": ["title", "owner"],
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "What is done, starting with a verb."
                            },
                            "description": {
                                "type": "string",
                                "description": "What the result is, and how to tell that it \
                                                is done."
                            },
                            "owner": {
                                "type": "string",
                                "description": "The name of the member who does it, exactly \
                                                as in members or as a member of the team is \
                                                called, or \"manager\" for work only the \
                                                manager can do."
                            },
                            "estimate_hours": {
                                "type": "number",
                                "description": "The size of the task: the hours a skilled \
                                                person would need for it."
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "normal", "high", "urgent"]
                            },
                            "waits_for": {
                                "type": "array",
                                "items": { "type": "integer" },
                                "description": "The tasks that must be finished first, by \
                                                their places in this list, counted from 1."
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_arguments() -> Value {
        json!({
            "question": "  How should the sensor node be powered? ",
            "reason": "It changes what is built.",
            "options": [
                { "label": "Battery only", "consequence": "Cheapest." },
                { "label": " Solar with battery ", "consequence": "No battery swaps." },
                { "label": "Mains power" }
            ],
            "recommended": 2
        })
    }

    fn good_interview() -> Value {
        json!({
            "questions": [
                {
                    "question": " Which microcontroller? ",
                    "reason": "It decides the firmware.",
                    "options": [
                        { "label": "ESP32-C3", "consequence": "Wi-Fi on board." },
                        { "label": "RP2040" }
                    ],
                    "recommended": 1
                },
                { "question": "What is fixed?", "reason": "" }
            ]
        })
    }

    fn good_proposal() -> Value {
        json!({
            "summary": " A device that measures the air. ",
            "members": [
                {
                    "name": "Hardware engineer",
                    "responsibility": "The board.",
                    "reason": "It is a board.",
                    "model": "opus",
                    "effort": "xhigh"
                },
                { "name": "Firmware engineer", "responsibility": "", "reason": "" }
            ],
            "tasks": [
                { "title": "Agree on the device", "owner": "Manager", "estimate_hours": 2 },
                {
                    "title": "Draw the schematic",
                    "owner": "hardware engineer",
                    "estimate_hours": 1.5,
                    "priority": "high",
                    "waits_for": [1, 1]
                },
                { "title": "Write the drivers", "owner": "Firmware engineer", "waits_for": [2] },
                { "title": "Design the enclosure", "owner": "Mechanical engineer", "waits_for": [1] }
            ]
        })
    }

    fn team() -> Vec<String> {
        vec!["Mechanical engineer".to_owned()]
    }

    fn answer_of(bus: &mut Bus, message: &Value) -> Value {
        serde_json::from_str(&bus.answer(&message.to_string()).unwrap()).unwrap()
    }

    fn call_of(tool: &str, arguments: &Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": tool, "arguments": arguments }
        })
    }

    fn call(arguments: &Value) -> Value {
        call_of(ASK_MANAGER, arguments)
    }

    fn text_of(result: &Value) -> &str {
        result["content"][0]["text"].as_str().unwrap()
    }

    fn every_tool() -> Vec<TeamTool> {
        vec![
            TeamTool::AskManager,
            TeamTool::Interview,
            TeamTool::ProposePlan { team: team() },
        ]
    }

    #[test]
    fn a_good_question_is_read_with_its_recommendation_counted_from_0() {
        let question = question_from(&good_arguments()).unwrap();
        assert_eq!(
            question,
            ManagerQuestion {
                question: "How should the sensor node be powered?".to_owned(),
                reason: "It changes what is built.".to_owned(),
                options: vec![
                    QuestionOption {
                        label: "Battery only".to_owned(),
                        consequence: "Cheapest.".to_owned(),
                    },
                    QuestionOption {
                        label: "Solar with battery".to_owned(),
                        consequence: "No battery swaps.".to_owned(),
                    },
                    QuestionOption {
                        label: "Mains power".to_owned(),
                        consequence: String::new(),
                    },
                ],
                recommended: Some(1),
            }
        );
    }

    #[test]
    fn a_reason_and_a_recommendation_may_be_left_out() {
        let question = question_from(&json!({
            "question": "Which color?",
            "options": [{ "label": "Gray" }, { "label": "Black" }],
            "recommended": null
        }))
        .unwrap();
        assert_eq!(question.reason, "");
        assert_eq!(question.recommended, None);
    }

    #[test]
    fn a_question_that_cannot_be_answered_is_refused_with_what_to_change() {
        let refused = |change: &dyn Fn(&mut Value)| {
            let mut arguments = good_arguments();
            change(&mut arguments);
            question_from(&arguments).unwrap_err()
        };

        assert!(refused(&|arguments| arguments["question"] = json!("  ")).contains("\"question\""));
        assert!(
            refused(&|arguments| {
                arguments.as_object_mut().unwrap().remove("question");
            })
            .contains("The question is empty")
        );
        assert!(refused(&|arguments| arguments["question"] = json!(3)).contains("must be text"));
        assert!(refused(&|arguments| arguments["reason"] = json!([])).contains("\"reason\""));
        assert!(
            refused(&|arguments| {
                arguments.as_object_mut().unwrap().remove("options");
            })
            .contains("There are no options")
        );
        assert_eq!(
            refused(&|arguments| arguments["options"] = json!([{ "label": "Only" }])),
            "There are 1 options. Give 2 to 4 options in \"options\"."
        );
        assert!(
            refused(&|arguments| {
                arguments["options"] = json!([
                    { "label": "a" }, { "label": "b" }, { "label": "c" },
                    { "label": "d" }, { "label": "e" }
                ]);
            })
            .starts_with("There are 5 options")
        );
        assert_eq!(
            refused(&|arguments| arguments["options"][1]["label"] = json!(" ")),
            "Option 2 has no label. Give every option a short \"label\"."
        );
        assert!(
            refused(&|arguments| arguments["options"][0] = json!("Battery")).contains("Option 1")
        );
        assert!(
            refused(&|arguments| arguments["options"][0]["consequence"] = json!(1))
                .contains("\"consequence\"")
        );
        for wrong in [json!(0), json!(4), json!(-1), json!(1.5), json!("2")] {
            assert!(
                refused(&|arguments| arguments["recommended"] = wrong.clone())
                    .contains("a whole number from 1 to 3"),
                "{wrong}"
            );
        }
        assert!(question_from(&json!("a question")).is_err());
    }

    #[test]
    fn a_good_interview_is_read_tidied_with_its_recommendations_counted_from_0() {
        let interview = interview_from(&good_interview()).unwrap();

        assert_eq!(interview.questions.len(), 2);
        assert_eq!(interview.questions[0].question, "Which microcontroller?");
        assert_eq!(interview.questions[0].recommended, Some(0));
        assert_eq!(interview.questions[0].options[1].consequence, "");
        assert!(interview.questions[1].options.is_empty());
        assert_eq!(interview.questions[1].recommended, None);
    }

    #[test]
    fn an_interview_that_cannot_be_answered_is_refused_with_what_to_change() {
        let refused = |change: &dyn Fn(&mut Value)| {
            let mut arguments = good_interview();
            change(&mut arguments);
            interview_from(&arguments).unwrap_err()
        };

        assert_eq!(
            refused(&|arguments| arguments["questions"] = json!([])),
            "Ask one to five questions in \"questions\"."
        );
        assert_eq!(
            refused(&|arguments| arguments["questions"] =
                Value::Array(vec![json!({ "question": "A?" }); 6])),
            "Ask one to five questions in \"questions\"."
        );
        assert_eq!(
            refused(&|arguments| arguments["questions"][1]["question"] = json!(" ")),
            "Question 2 is empty."
        );
        assert_eq!(
            refused(&|arguments| arguments["questions"][0]["options"] = json!([{ "label": "A" }])),
            "Question 1 has 1 options. Give two to four, or none."
        );
        assert_eq!(
            refused(&|arguments| arguments["questions"][0]["options"][1]["label"] = json!("")),
            "An option of question 1 has no label."
        );
        for wrong in [json!(0), json!(3), json!(1.5)] {
            assert_eq!(
                refused(&|arguments| arguments["questions"][0]["recommended"] = wrong.clone()),
                "\"recommended\" of question 1 is the place of an option, from 1 to 2."
            );
        }
        assert!(
            refused(&|arguments| arguments["questions"][1]["recommended"] = json!(1))
                .contains("Leave \"recommended\" out")
        );
    }

    #[test]
    fn a_good_proposal_is_read_with_its_places_counted_from_0_and_hours_as_minutes() {
        let proposal = proposal_from(&good_proposal(), &team()).unwrap();

        assert_eq!(proposal.summary, "A device that measures the air.");
        assert_eq!(proposal.members[0].model.as_deref(), Some("opus"));
        assert_eq!(proposal.members[0].effort, Some(Effort::ExtraHigh));
        assert_eq!(proposal.members[1].model, None);
        let tasks = &proposal.tasks;
        assert_eq!(tasks[0].owner, PlannedOwner::Manager);
        assert_eq!(tasks[0].estimate_minutes, Some(120));
        assert_eq!(tasks[0].priority, Priority::Normal);
        assert_eq!(
            tasks[1].owner,
            PlannedOwner::Member("hardware engineer".to_owned())
        );
        assert_eq!(tasks[1].estimate_minutes, Some(90));
        assert_eq!(tasks[1].priority, Priority::High);
        assert_eq!(tasks[1].waits_for, [0]);
        assert_eq!(tasks[2].estimate_minutes, None);
        assert_eq!(tasks[2].waits_for, [1]);
        // A member the project has already may be given a task.
        assert_eq!(
            tasks[3].owner,
            PlannedOwner::Member("Mechanical engineer".to_owned())
        );
    }

    #[test]
    fn a_proposal_that_cannot_be_approved_is_refused_with_what_to_change() {
        let refused = |change: &dyn Fn(&mut Value)| {
            let mut arguments = good_proposal();
            change(&mut arguments);
            proposal_from(&arguments, &team()).unwrap_err()
        };

        assert_eq!(
            refused(&|arguments| arguments["summary"] = json!(" ")),
            "The summary is empty."
        );
        assert_eq!(
            refused(
                &|arguments| arguments["members"] = Value::Array(vec![json!({ "name": "A" }); 9])
            ),
            "Give the members as a list of at most eight."
        );
        assert_eq!(
            refused(&|arguments| arguments["members"][1]["name"] = json!("MANAGER")),
            "Member 2 needs a name, and \"manager\" is taken."
        );
        assert_eq!(
            refused(&|arguments| arguments["members"][1]["name"] = json!("hardware Engineer")),
            "Two members are called hardware Engineer."
        );
        assert_eq!(
            refused(&|arguments| arguments["members"][1]["name"] = json!("mechanical Engineer")),
            "On the team already: mechanical Engineer. A proposal holds only what is to be \
             added. Leave out the members who are on the team and the tasks that exist, and \
             propose again. A new task may be given to a member who is on the team."
        );
        assert!(
            refused(&|arguments| arguments["members"][1]["name"] = json!("x".repeat(61)))
                .starts_with("The name of member 2 cannot be used")
        );
        assert_eq!(
            refused(&|arguments| arguments["members"][0]["model"] = json!("two words")),
            "The model of member 1 is one word, such as \"sonnet\"."
        );
        assert_eq!(
            refused(&|arguments| arguments["members"][0]["effort"] = json!("huge")),
            "The effort of member 1 is one of low, medium, high, xhigh and max."
        );
        assert_eq!(
            refused(&|arguments| arguments["tasks"] = json!([])),
            "Give one to forty tasks."
        );
        assert_eq!(
            refused(&|arguments| arguments["tasks"][2]["title"] = json!("")),
            "Task 3 has no title."
        );
        assert_eq!(
            refused(&|arguments| arguments["tasks"][2]["owner"] = json!("Nobody")),
            "The owner of task 3, \"Nobody\", is not a member. Use a name from members, the name \
             of a member of the team, or \"manager\"."
        );
        for wrong in [json!(-1), json!("two")] {
            assert_eq!(
                refused(&|arguments| arguments["tasks"][0]["estimate_hours"] = wrong.clone()),
                "The estimate of task 1 is a number of hours, such as 4."
            );
        }
        assert_eq!(
            refused(&|arguments| arguments["tasks"][0]["priority"] = json!("soon")),
            "The priority of task 1 is one of low, normal, high and urgent."
        );
        for wrong in [json!([3]), json!([5]), json!([0]), json!([1.5])] {
            assert!(
                refused(&|arguments| arguments["tasks"][2]["waits_for"] = wrong.clone())
                    .ends_with("which is not another task of the list."),
                "{wrong}"
            );
        }
        assert!(
            refused(&|arguments| arguments["tasks"][0]["waits_for"] = json!([3]))
                .starts_with("The tasks wait for each other in a circle")
        );
    }

    #[test]
    fn the_tools_go_to_the_server_as_arguments_and_are_read_back() {
        let tools = every_tool();

        let arguments = bus_arguments(&tools);

        assert_eq!(
            arguments,
            [
                "ask_manager",
                "interview",
                "propose_plan",
                "--member",
                "Mechanical engineer"
            ]
        );
        assert_eq!(tools_from_arguments(&arguments), Ok(tools));
        assert!(tools_from_arguments(&["something_else".to_owned()]).is_err());
        assert!(tools_from_arguments(&["--member".to_owned()]).is_err());
        assert_eq!(name_in_session(&TeamTool::Interview), INTERVIEW_IN_SESSION);
        assert_eq!(
            name_in_session(&TeamTool::ProposePlan { team: Vec::new() }),
            PROPOSE_PLAN_IN_SESSION
        );
        assert_eq!(
            name_in_session(&TeamTool::AskManager),
            ASK_MANAGER_IN_SESSION
        );
    }

    #[test]
    fn the_server_introduces_itself_with_the_version_of_the_client() {
        let mut bus = Bus::offering(vec![TeamTool::AskManager]);
        let answer = answer_of(
            &mut bus,
            &json!({
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": { "name": "sample-client" }
                },
                "jsonrpc": "2.0",
                "id": 0
            }),
        );
        assert_eq!(
            answer,
            json!({
                "jsonrpc": "2.0",
                "id": 0,
                "result": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "teamwright", "version": env!("CARGO_PKG_VERSION") }
                }
            })
        );
    }

    #[test]
    fn the_server_offers_the_tool_to_ask_the_manager_when_it_is_given() {
        let mut bus = Bus::offering(vec![TeamTool::AskManager]);
        let answer = answer_of(
            &mut bus,
            &json!({ "method": "tools/list", "jsonrpc": "2.0", "id": 1 }),
        );
        let tools = answer["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "ask_manager");
        assert_eq!(
            tools[0]["inputSchema"]["required"],
            json!(["question", "options"])
        );
        let description = tools[0]["description"].as_str().unwrap();
        assert!(description.contains("what it costs"));
        assert!(description.contains("end your turn"));
    }

    #[test]
    fn the_server_offers_the_tools_of_the_advisor_when_they_are_given() {
        let mut bus = Bus::offering(vec![
            TeamTool::Interview,
            TeamTool::ProposePlan { team: team() },
        ]);
        let answer = answer_of(
            &mut bus,
            &json!({ "method": "tools/list", "jsonrpc": "2.0", "id": 1 }),
        );
        let tools = answer["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["interview", "propose_plan"]);
        assert_eq!(
            tools[1]["inputSchema"]["properties"]["members"]["items"]["properties"]["effort"]["enum"],
            json!(["low", "medium", "high", "xhigh", "max"])
        );
        let refused = answer_of(&mut bus, &call(&good_arguments()));
        assert_eq!(refused["error"]["code"], -32_602);
    }

    #[test]
    fn a_good_question_is_taken_once_and_the_session_is_told_to_end_its_turn() {
        let mut bus = Bus::offering(vec![TeamTool::AskManager]);

        let first = answer_of(&mut bus, &call(&good_arguments()));
        let second = answer_of(&mut bus, &call(&good_arguments()));

        assert_eq!(first["id"], 2);
        assert_eq!(text_of(&first["result"]), QUESTION_TAKEN);
        assert_eq!(first["result"].get("isError"), None);
        assert_eq!(second["result"]["isError"], true);
        assert!(text_of(&second["result"]).contains("already asked"));
    }

    #[test]
    fn an_interview_and_a_proposal_are_taken_with_the_answers_of_the_stand_in() {
        let mut interviewing = Bus::offering(every_tool());
        let mut proposing = Bus::offering(every_tool());

        let interview = answer_of(&mut interviewing, &call_of(INTERVIEW, &good_interview()));
        let proposal = answer_of(&mut proposing, &call_of(PROPOSE_PLAN, &good_proposal()));

        assert_eq!(
            text_of(&interview["result"]),
            "Your questions are with the manager. End your turn now with one sentence that says \
             you are waiting for the answers."
        );
        assert_eq!(
            text_of(&proposal["result"]),
            "Your proposal is with the manager. End your turn now with one or two sentences \
             that say what you proposed."
        );
    }

    #[test]
    fn after_one_is_taken_no_question_interview_or_proposal_is_taken_in_the_turn() {
        let mut bus = Bus::offering(every_tool());
        answer_of(&mut bus, &call_of(INTERVIEW, &good_interview()));

        for (tool, arguments) in [
            (INTERVIEW, good_interview()),
            (PROPOSE_PLAN, good_proposal()),
            (ASK_MANAGER, good_arguments()),
        ] {
            let again = answer_of(&mut bus, &call_of(tool, &arguments));
            assert_eq!(again["result"]["isError"], true, "{tool}");
        }
    }

    #[test]
    fn a_refused_call_says_what_to_change_and_may_be_made_again() {
        let mut bus = Bus::offering(every_tool());

        let refused = answer_of(&mut bus, &call(&json!({ "question": "Which?" })));
        let refused_plan = answer_of(
            &mut bus,
            &call_of(PROPOSE_PLAN, &json!({ "summary": "A plan." })),
        );
        let taken = answer_of(&mut bus, &call(&good_arguments()));

        assert_eq!(refused["result"]["isError"], true);
        assert!(text_of(&refused["result"]).starts_with("The question was not sent"));
        assert!(text_of(&refused_plan["result"]).starts_with("The proposal was not sent"));
        assert_eq!(text_of(&taken["result"]), QUESTION_TAKEN);
    }

    #[test]
    fn identifiers_go_back_as_they_came_and_unknown_methods_are_errors() {
        let mut bus = Bus::offering(vec![TeamTool::AskManager]);
        let probe = answer_of(
            &mut bus,
            &json!({
                "jsonrpc": "2.0",
                "id": "server-discover-probe-1",
                "method": "server/discover",
                "params": {}
            }),
        );
        assert_eq!(probe["id"], "server-discover-probe-1");
        assert_eq!(probe["error"]["code"], -32_601);

        let ping = answer_of(
            &mut bus,
            &json!({ "jsonrpc": "2.0", "id": 7, "method": "ping" }),
        );
        assert_eq!(ping["result"], json!({}));

        let other_tool = answer_of(
            &mut bus,
            &json!({
                "jsonrpc": "2.0", "id": 8, "method": "tools/call",
                "params": { "name": "something_else", "arguments": {} }
            }),
        );
        assert_eq!(other_tool["error"]["code"], -32_602);
    }

    #[test]
    fn notifications_responses_and_lines_that_are_not_json_get_no_answer() {
        let mut bus = Bus::default();
        for line in [
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":3,"result":{}}"#,
            "not json",
            "",
            "[1]",
        ] {
            assert_eq!(bus.answer(line), None, "{line}");
        }
    }

    #[test]
    fn the_server_answers_line_by_line_until_the_input_ends() {
        let input = [
            r#"{"jsonrpc":"2.0","id":"server-discover-probe-1","method":"server/discover"}"#,
            r#"{"method":"initialize","params":{"protocolVersion":"2025-11-25"},"jsonrpc":"2.0","id":0}"#,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            "garbage",
            r#"{"method":"tools/list","jsonrpc":"2.0","id":1}"#,
        ]
        .join("\r\n");
        let mut output = Vec::new();

        serve(input.as_bytes(), &mut output, vec![TeamTool::AskManager]).unwrap();

        let answers: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let ids: Vec<&Value> = answers.iter().map(|answer| &answer["id"]).collect();
        assert_eq!(
            ids,
            [&json!("server-discover-probe-1"), &json!(0), &json!(1)]
        );
    }

    // Handoffs and messages

    fn teammates() -> Vec<String> {
        vec!["Firmware engineer".to_owned(), "Tester".to_owned()]
    }

    fn good_handoff() -> Value {
        json!({
            "summary": " Pin map v1 is done. ",
            "contract": "Display on SPI2.",
            "files": ["hardware\\pinmap.md", "./notes.md"]
        })
    }

    fn good_message() -> Value {
        json!({ "to": "firmware engineer", "kind": "question", "text": " Which pins are free? " })
    }

    /// A folder with the files of [`good_handoff`] in it.
    fn folder_with_files() -> tempfile::TempDir {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir(folder.path().join("hardware")).unwrap();
        std::fs::write(folder.path().join("hardware").join("pinmap.md"), "pins").unwrap();
        std::fs::write(folder.path().join("notes.md"), "notes").unwrap();
        folder
    }

    #[test]
    fn a_good_handoff_is_read_tidied_with_its_paths_written_with_forward_slashes() {
        let handoff = handoff_from(&good_handoff()).unwrap();

        assert_eq!(
            handoff,
            PublishedHandoff {
                summary: "Pin map v1 is done.".to_owned(),
                contract: "Display on SPI2.".to_owned(),
                files: vec!["hardware/pinmap.md".to_owned(), "notes.md".to_owned()],
            }
        );
        let bare = handoff_from(&json!({ "summary": "Done." })).unwrap();
        assert!(bare.contract.is_empty() && bare.files.is_empty());
    }

    #[test]
    fn a_handoff_that_cannot_be_recorded_is_refused_with_what_to_change() {
        let refused = |change: &dyn Fn(&mut Value)| {
            let mut arguments = good_handoff();
            change(&mut arguments);
            handoff_from(&arguments).unwrap_err()
        };

        assert!(
            refused(&|arguments| arguments["summary"] = json!(" "))
                .contains("the summary of a handoff must not be empty")
        );
        assert!(
            refused(&|arguments| arguments["files"] = json!(["../outside.md"]))
                .contains("does not stay inside the folder")
        );
        assert!(
            refused(&|arguments| arguments["files"] = json!(["/etc/passwd"]))
                .contains("does not stay inside the folder")
        );
        assert!(
            refused(&|arguments| arguments["files"] = json!(["a.md", "a.md"]))
                .contains("named twice")
        );
        assert!(refused(&|arguments| arguments["files"] = json!([3])).contains("path as text"));
        assert!(refused(&|arguments| arguments["files"] = json!("a.md")).contains("is a list"));
        assert!(refused(&|arguments| arguments["contract"] = json!(1)).contains("must be text"));
        assert!(handoff_from(&json!("done")).is_err());
    }

    #[test]
    fn the_files_of_a_handoff_must_be_in_the_folder() {
        let folder = folder_with_files();
        let handoff = handoff_from(&good_handoff()).unwrap();

        assert!(missing_files(&handoff, folder.path()).is_empty());
        let mut with_missing = handoff.clone();
        with_missing.files.push("firmware/main.c".to_owned());
        assert_eq!(
            missing_files(&with_missing, folder.path()),
            ["firmware/main.c"]
        );
        assert_eq!(
            missing_files(&handoff, &folder.path().join("nowhere")),
            handoff.files
        );
    }

    #[test]
    fn a_good_message_goes_to_a_teammate_by_the_name_it_has() {
        let message = message_from(&good_message(), &teammates()).unwrap();

        assert_eq!(
            message,
            SentMessage {
                to: "Firmware engineer".to_owned(),
                kind: MessageKind::Question,
                text: "Which pins are free?".to_owned(),
            }
        );
    }

    #[test]
    fn a_message_that_cannot_be_sent_is_refused_with_what_to_change() {
        let refused = |change: &dyn Fn(&mut Value)| {
            let mut arguments = good_message();
            change(&mut arguments);
            message_from(&arguments, &teammates()).unwrap_err()
        };

        assert_eq!(
            refused(&|arguments| arguments["to"] = json!("Manager")),
            "\"Manager\" is not a teammate. Write to one of: Firmware engineer, Tester."
        );
        assert_eq!(
            refused(&|arguments| arguments["kind"] = json!("letter")),
            "\"kind\" is \"question\" or \"note\"."
        );
        assert_eq!(
            refused(&|arguments| arguments["text"] = json!("  ")),
            "The message is empty. Write it in \"text\"."
        );
        assert!(
            refused(&|arguments| arguments["text"] = json!("x".repeat(4_001)))
                .starts_with("The message has 4001 characters")
        );
        assert_eq!(
            message_from(&good_message(), &[]).unwrap_err(),
            "You have no teammate to write to."
        );
    }

    #[test]
    fn the_folder_and_the_teammates_go_to_the_server_as_arguments_and_are_read_back() {
        let tools = vec![
            TeamTool::AskManager,
            TeamTool::PublishHandoff {
                folder: PathBuf::from("/projects/SSN"),
            },
            TeamTool::SendMessage {
                teammates: teammates(),
            },
        ];

        let arguments = bus_arguments(&tools);

        assert_eq!(
            arguments,
            [
                "ask_manager",
                "publish_handoff",
                "--folder",
                "/projects/SSN",
                "send_message",
                "--teammate",
                "Firmware engineer",
                "--teammate",
                "Tester"
            ]
        );
        assert_eq!(tools_from_arguments(&arguments), Ok(tools));
        assert!(tools_from_arguments(&["publish_handoff".to_owned()]).is_err());
        assert!(tools_from_arguments(&["--teammate".to_owned()]).is_err());
        assert_eq!(
            name_in_session(&TeamTool::PublishHandoff {
                folder: PathBuf::new()
            }),
            PUBLISH_HANDOFF_IN_SESSION
        );
        assert_eq!(
            name_in_session(&TeamTool::SendMessage {
                teammates: Vec::new()
            }),
            SEND_MESSAGE_IN_SESSION
        );
    }

    #[test]
    fn the_server_offers_the_tools_of_a_specialist_with_the_names_of_its_teammates() {
        let mut bus = Bus::offering(vec![
            TeamTool::AskManager,
            TeamTool::PublishHandoff {
                folder: PathBuf::from("."),
            },
            TeamTool::SendMessage {
                teammates: teammates(),
            },
        ]);
        let answer = answer_of(
            &mut bus,
            &json!({ "method": "tools/list", "jsonrpc": "2.0", "id": 1 }),
        );
        let tools = answer["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["ask_manager", "publish_handoff", "send_message"]);
        assert_eq!(tools[1]["inputSchema"]["required"], json!(["summary"]));
        assert_eq!(
            tools[2]["inputSchema"]["properties"]["to"]["enum"],
            json!(["Firmware engineer", "Tester"])
        );
        assert!(
            tools[1]["description"]
                .as_str()
                .unwrap()
                .contains("your turn goes on")
        );
    }

    #[test]
    fn a_handoff_is_taken_once_and_the_session_goes_on_and_may_still_ask() {
        let folder = folder_with_files();
        let mut bus = Bus::offering(vec![
            TeamTool::AskManager,
            TeamTool::PublishHandoff {
                folder: folder.path().to_owned(),
            },
        ]);

        let first = answer_of(&mut bus, &call_of(PUBLISH_HANDOFF, &good_handoff()));
        let second = answer_of(&mut bus, &call_of(PUBLISH_HANDOFF, &good_handoff()));
        let asked = answer_of(&mut bus, &call(&good_arguments()));

        assert_eq!(text_of(&first["result"]), HANDOFF_TAKEN);
        assert_eq!(first["result"].get("isError"), None);
        assert_eq!(second["result"]["isError"], true);
        assert!(text_of(&second["result"]).contains("already"));
        assert_eq!(text_of(&asked["result"]), QUESTION_TAKEN);
    }

    #[test]
    fn a_handoff_with_a_file_that_is_not_there_is_refused_and_may_be_made_again() {
        let folder = folder_with_files();
        let mut bus = Bus::offering(vec![TeamTool::PublishHandoff {
            folder: folder.path().to_owned(),
        }]);
        let mut with_missing = good_handoff();
        with_missing["files"] = json!(["notes.md", "firmware/main.c"]);

        let refused = answer_of(&mut bus, &call_of(PUBLISH_HANDOFF, &with_missing));
        let taken = answer_of(&mut bus, &call_of(PUBLISH_HANDOFF, &good_handoff()));

        assert_eq!(refused["result"]["isError"], true);
        assert_eq!(
            text_of(&refused["result"]),
            "The handoff was not recorded. These files are not in the folder of the project: \
             firmware/main.c. Write them first, or leave them out."
        );
        assert_eq!(text_of(&taken["result"]), HANDOFF_TAKEN);
    }

    #[test]
    fn messages_are_taken_up_to_five_in_a_turn_and_never_after_a_question() {
        let mut bus = Bus::offering(vec![
            TeamTool::AskManager,
            TeamTool::SendMessage {
                teammates: teammates(),
            },
        ]);
        let refused = answer_of(
            &mut bus,
            &call_of(
                SEND_MESSAGE,
                &json!({ "to": "Nobody", "kind": "note", "text": "Hi" }),
            ),
        );
        assert!(text_of(&refused["result"]).starts_with("The message was not sent."));
        for _ in 0..5 {
            let taken = answer_of(&mut bus, &call_of(SEND_MESSAGE, &good_message()));
            assert_eq!(text_of(&taken["result"]), MESSAGE_TAKEN);
        }
        let sixth = answer_of(&mut bus, &call_of(SEND_MESSAGE, &good_message()));
        assert_eq!(text_of(&sixth["result"]), TOO_MANY_MESSAGES);

        let mut asking = Bus::offering(vec![
            TeamTool::AskManager,
            TeamTool::SendMessage {
                teammates: teammates(),
            },
        ]);
        answer_of(&mut asking, &call(&good_arguments()));
        let after = answer_of(&mut asking, &call_of(SEND_MESSAGE, &good_message()));
        assert_eq!(after["result"]["isError"], true);
    }
}
