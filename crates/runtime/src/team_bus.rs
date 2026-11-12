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
//! - `propose_plan`, with which the Advisor proposes a team and a plan.
//!
//! The server only checks a call and tells the session to end its turn.
//! What was asked or proposed reaches Teamwright through the output of the
//! turn, where the decoder finds the call and checks it with the same
//! functions: [`question_from`], [`interview_from`] and [`proposal_from`].
//!
//! The tools are named to the server as arguments after the ones it is
//! configured with ([`bus_arguments`], [`tools_from_arguments`]). The
//! server keeps no state beyond one turn: Claude Code starts it with the
//! turn and leaves it when the turn ends.

use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use serde_json::{Map, Value, json};
use teamwright_application::agent_runtime::Priority;
use teamwright_application::{
    Effort, InterviewQuestionAsked, ManagerInterview, ManagerQuestion, PlanProposal, PlannedMember,
    PlannedOwner, PlannedTask, QuestionOption, TeamTool,
};
use teamwright_domain::{MemberName, ModelName, TaskTitle};

/// The name of the tool server, as Claude Code is told it.
pub const SERVER_NAME: &str = "teamwright";

/// The name of the tool to ask the manager to decide.
pub const ASK_MANAGER: &str = "ask_manager";

/// The name of the tool with which the Advisor asks its questions.
pub const INTERVIEW: &str = "interview";

/// The name of the tool with which the Advisor proposes a team and a plan.
pub const PROPOSE_PLAN: &str = "propose_plan";

/// What Claude Code puts in front of the name of a tool of this server.
const PREFIX_IN_SESSION: &str = "mcp__teamwright__";

/// The name of `ask_manager` inside a session of Claude Code.
pub const ASK_MANAGER_IN_SESSION: &str = "mcp__teamwright__ask_manager";

/// The name of `interview` inside a session of Claude Code.
pub const INTERVIEW_IN_SESSION: &str = "mcp__teamwright__interview";

/// The name of `propose_plan` inside a session of Claude Code.
pub const PROPOSE_PLAN_IN_SESSION: &str = "mcp__teamwright__propose_plan";

/// The argument before the name of a member of the team.
const MEMBER_ARGUMENT: &str = "--member";

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
     approves. After calling it, end your turn.";

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
    }
}

/// The name of a tool inside a session of Claude Code, which puts the name
/// of its server in front of it.
#[must_use]
pub fn name_in_session(tool: &TeamTool) -> String {
    format!("{PREFIX_IN_SESSION}{}", name_of(tool))
}

/// The arguments that name the tools to the server: the name of each tool,
/// and `--member <name>` for each member the project has when it may
/// propose a plan.
#[must_use]
pub fn bus_arguments(tools: &[TeamTool]) -> Vec<String> {
    let mut arguments = Vec::new();
    for tool in tools {
        arguments.push(name_of(tool).to_owned());
        if let TeamTool::ProposePlan { team } = tool {
            for name in team {
                arguments.push(MEMBER_ARGUMENT.to_owned());
                arguments.push(name.clone());
            }
        }
    }
    arguments
}

/// Reads the tools back from the arguments [`bus_arguments`] wrote.
///
/// # Errors
///
/// Returns what is wrong, in words, if an argument names no tool, or if a
/// `--member` has no name after it.
pub fn tools_from_arguments(arguments: &[String]) -> Result<Vec<TeamTool>, String> {
    let mut tools = Vec::new();
    let mut team = Vec::new();
    let mut given = arguments.iter();
    while let Some(argument) = given.next() {
        match argument.as_str() {
            ASK_MANAGER => tools.push(TeamTool::AskManager),
            INTERVIEW => tools.push(TeamTool::Interview),
            PROPOSE_PLAN => tools.push(TeamTool::ProposePlan { team: Vec::new() }),
            MEMBER_ARGUMENT => team.push(
                given
                    .next()
                    .ok_or_else(|| format!("{MEMBER_ARGUMENT} needs the name of a member"))?
                    .clone(),
            ),
            other => return Err(format!("the team bus has no tool {other}")),
        }
    }
    for tool in &mut tools {
        if let TeamTool::ProposePlan { team: members } = tool {
            members.clone_from(&team);
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
/// a name that no other has, whatever the capitals, and that is not
/// "manager"; a model is one word, an effort one of low, medium, high,
/// xhigh and max. There are one to forty tasks, each with a title and an
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
    let mut names = HashSet::new();
    let mut members = Vec::new();
    for (index, member) in given_members.iter().enumerate() {
        let member = planned_member_from(index + 1, member)?;
        if !names.insert(member.name.to_lowercase()) {
            return Err(format!("Two members are called {}.", member.name));
        }
        members.push(member);
    }

    let given_tasks = arguments
        .get("tasks")
        .and_then(Value::as_array)
        .filter(|tasks| (1..=MOST_TASKS).contains(&tasks.len()))
        .ok_or_else(|| "Give one to forty tasks.".to_owned())?;
    let known: HashSet<String> = names
        .iter()
        .cloned()
        .chain(team.iter().map(|name| name.trim().to_lowercase()))
        .collect();
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
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is a whole number between 0 and the largest u32"
    )]
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
    /// this turn. A turn carries at most one.
    taken: bool,
}

impl Bus {
    /// A server that offers these tools.
    const fn offering(tools: Vec<TeamTool>) -> Self {
        Self {
            tools,
            taken: false,
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
    }
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
/// it, with the efforts Teamwright knows and a model of any one word.
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
                    "description": "The AI specialists the project needs: the smallest team \
                                    that can do the job.",
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
}
