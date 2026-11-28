//! Turning the JSON lines of Claude Code's headless mode into events.
//!
//! The format belongs to Claude Code and changes with its versions, so the
//! decoder is lenient on purpose: a line that is not JSON, an event of an
//! unknown type and a field that is missing or unknown are all passed over
//! without an error. Only what Teamwright uses is read. The tests at the end
//! run the decoder over real recordings in `tests/recordings/`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use serde_json::{Map, Value};
use teamwright_application::{
    AgentEvent, KnownTask, McpServer, ModelUsage, PermissionDenial, PlanLimitHit, PlanReset,
    PlanUsage, PlanWindow, PlanWindowKind, SessionContext, SessionId, SessionStarted, TokenCounts,
    TurnFailure, TurnOutcome,
};

use crate::plan_limit::limit_in_words;

use crate::team_bus::{
    ASK_MANAGER_IN_SESSION, ASK_PERMISSION_IN_SESSION, INTERVIEW_IN_SESSION,
    PROPOSE_PLAN_IN_SESSION, PUBLISH_HANDOFF_IN_SESSION, REMEMBER_IN_SESSION,
    SEND_MESSAGE_IN_SESSION, fact_from, handoff_from, interview_from, message_from,
    permission_from, proposal_from, question_from,
};

/// The longest summary of a tool's input, in characters.
const SUMMARY_LENGTH: usize = 120;

/// How Claude Code names the failure of a turn because nobody is signed in.
const NOT_SIGNED_IN: &str = "authentication_failed";

/// How Claude Code names the status of a plan that refuses work, in a
/// `rate_limit_event`.
const REJECTED: &str = "rejected";

/// How the process of a turn ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEnd {
    /// The process ended by itself.
    Exited {
        /// Its exit code, if it had one.
        code: Option<i32>,
        /// The last lines it wrote to standard error.
        error_lines: Vec<String>,
    },
    /// The process was stopped on request.
    Stopped,
}

/// Reads the lines of one turn, in order, and remembers what the end of the
/// turn needs.
#[derive(Debug, Default)]
pub struct Decoder {
    session_id: Option<SessionId>,
    working_folder: Option<String>,
    tool_names: HashMap<String, String>,
    /// The names of the members the project has, to which a proposed task
    /// may be given.
    team: Vec<String>,
    /// The tasks the project has, which a proposal may name and move.
    tasks: Vec<KnownTask>,
    /// The names of the teammates a message may go to.
    teammates: Vec<String>,
    /// Whether a command may be asked for with `ask_permission`, as the
    /// bus was told. Where it may not, a request for one is an ordinary use
    /// of a tool, which the bus refuses.
    commands_askable: bool,
    /// The handoffs and messages whose calls passed the check and wait for
    /// the answer of the team bus, by the identifier of the use. Only the
    /// bus knows whether the files of a handoff are there, so what it
    /// refused is reported as an ordinary use of a tool that failed.
    awaiting_answer: HashMap<String, Awaiting>,
    /// The use of the tool that asked the manager or proposed, once a
    /// question, an interview or a proposal was taken. A turn carries at
    /// most one. The result of the use is not reported.
    taken_use: Option<String>,
    last_text: String,
    failure_kind: Option<String>,
    /// The limit of the plan, if the program reported that the plan
    /// refuses work: a `rate_limit_event` whose status is `rejected`.
    plan_refused: Option<PlanLimitHit>,
    /// The last request the session itself made to the model, not one of
    /// a helper it started: what it held, and the model it went to.
    last_request: Option<LastRequest>,
    result: Option<Map<String, Value>>,
}

/// What the last request of a session sent the model.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LastRequest {
    /// The tokens it held: read, read from the cache and written to it.
    held: u64,
    /// The model it went to, by its full name.
    model: Option<String>,
}

/// A call of `publish_handoff` or `send_message` that waits for the answer
/// of the team bus.
#[derive(Debug, Clone)]
struct Awaiting {
    /// What is reported if the bus takes it.
    taken: AgentEvent,
    /// The name of the tool, reported if the bus refuses it.
    tool: String,
    /// What the tool was used on, reported if the bus refuses it.
    summary: String,
}

/// How Claude Code names a message it made up itself, which no model wrote
/// and which used no tokens.
const MADE_UP_MODEL: &str = "<synthetic>";

impl Decoder {
    /// A decoder for a new turn of a project without a team. The tests
    /// use it; a turn is always decoded for its team.
    #[cfg(test)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A decoder for a new turn of a project whose team has members of
    /// these names. A proposal may give a task to one of them.
    #[must_use]
    pub fn for_team(team: Vec<String>) -> Self {
        Self {
            team,
            ..Self::default()
        }
    }

    /// The same decoder, for a project that has these tasks. A proposal
    /// may make a new task a part of one of them, and move them.
    #[must_use]
    pub fn with_tasks(self, tasks: Vec<KnownTask>) -> Self {
        Self { tasks, ..self }
    }

    /// The same decoder, for a session that may write to teammates of
    /// these names.
    #[must_use]
    pub fn with_teammates(self, teammates: Vec<String>) -> Self {
        Self { teammates, ..self }
    }

    /// The same decoder, for a session in which a command may be asked
    /// for, or not. A new decoder takes none.
    #[must_use]
    pub fn with_commands_askable(self, commands_askable: bool) -> Self {
        Self {
            commands_askable,
            ..self
        }
    }

    /// Reads one line of output and returns the events it holds, often none.
    pub fn decode_line(&mut self, line: &str) -> Vec<AgentEvent> {
        let Ok(Value::Object(event)) = serde_json::from_str::<Value>(line.trim()) else {
            return Vec::new();
        };
        if self.session_id.is_none() {
            self.session_id = text(&event, "session_id").and_then(|id| SessionId::parse(id).ok());
        }
        match text(&event, "type") {
            Some("system") => {
                let decoded = self.decode_system(&event);
                if let Some(AgentEvent::SessionStarted(started)) = &decoded {
                    // Kept for the summaries of tool use, which show paths
                    // inside the working folder relative to it.
                    let folder = started.working_folder.to_string_lossy().into_owned();
                    self.working_folder = (!folder.is_empty()).then_some(folder);
                }
                decoded.into_iter().collect()
            }
            Some("assistant") => self.decode_assistant(&event),
            Some("user") => self.decode_user(&event),
            Some("rate_limit_event") => {
                if let Some(hit) = refused_by_plan(&event) {
                    self.plan_refused = Some(hit);
                }
                decode_plan_usage(&event).into_iter().collect()
            }
            Some("result") => {
                self.result = Some(event);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// How the turn ended, from what was read and how the process ended.
    #[must_use]
    pub fn finish(self, end: ProcessEnd) -> TurnOutcome {
        match self.result {
            Some(ref result) => self.outcome_from_result(result),
            None => {
                let failure = match end {
                    ProcessEnd::Stopped => TurnFailure::Stopped,
                    ProcessEnd::Exited { code, error_lines } => {
                        // A program that ended early may still have said
                        // that the plan's limit was reached.
                        let said = std::iter::once(self.last_text.as_str())
                            .chain(error_lines.iter().map(String::as_str))
                            .find_map(limit_in_words);
                        match self.plan_refused.clone().or(said) {
                            Some(hit) => TurnFailure::PlanLimitReached(hit),
                            None => TurnFailure::ProgramEnded {
                                message: describe_early_end(code, &error_lines),
                            },
                        }
                    }
                };
                // Without the last message the window is not known; what
                // the session held is.
                let context = self.last_request.map(|request| SessionContext {
                    held: request.held,
                    window: None,
                });
                TurnOutcome {
                    session_id: self.session_id,
                    failure: Some(failure),
                    final_text: self.last_text,
                    usage: Vec::new(),
                    estimated_cost_usd: None,
                    duration: None,
                    model_calls: None,
                    permission_denials: Vec::new(),
                    context,
                }
            }
        }
    }

    fn outcome_from_result(&self, result: &Map<String, Value>) -> TurnOutcome {
        let failure = if result.get("is_error").and_then(Value::as_bool) == Some(true) {
            Some(self.reported_failure(result))
        } else {
            None
        };

        // Since about 2.1.300 `modelUsage` adds up the whole session when a
        // session is resumed, and `model_usage_since_last_result` holds this
        // turn alone. Older versions only have the first.
        let per_model = result
            .get("model_usage_since_last_result")
            .or_else(|| result.get("modelUsage"))
            .and_then(Value::as_object);
        let usage = per_model.map(model_usage).unwrap_or_default();
        let estimated_cost_usd = per_model.and_then(|models| {
            let costs: Vec<f64> = models
                .values()
                .filter_map(|model| model.get("costUSD").and_then(Value::as_f64))
                .collect();
            (!costs.is_empty()).then(|| costs.iter().sum())
        });

        let permission_denials = result
            .get("permission_denials")
            .and_then(Value::as_array)
            .map(|denials| {
                denials
                    .iter()
                    .filter_map(Value::as_object)
                    .filter_map(|denial| {
                        Some(PermissionDenial {
                            tool: text(denial, "tool_name")?.to_owned(),
                            tool_use_id: text(denial, "tool_use_id").map(str::to_owned),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        TurnOutcome {
            session_id: text(result, "session_id")
                .and_then(|id| SessionId::parse(id).ok())
                .or_else(|| self.session_id.clone()),
            failure,
            final_text: text(result, "result")
                .map_or_else(|| self.last_text.clone(), str::to_owned),
            usage,
            estimated_cost_usd,
            duration: number(result, "duration_ms").map(Duration::from_millis),
            model_calls: number(result, "num_turns").and_then(|calls| u32::try_from(calls).ok()),
            permission_denials,
            context: self.context_at_the_end(result),
        }
    }

    /// How full the context of the session is after the turn: what its last
    /// request held, against the window the last message names for the
    /// model of that request. Both the usage of this turn and that of the
    /// whole session name the window.
    fn context_at_the_end(&self, result: &Map<String, Value>) -> Option<SessionContext> {
        let request = self.last_request.as_ref()?;
        let window = request.model.as_deref().and_then(|model| {
            ["model_usage_since_last_result", "modelUsage"]
                .iter()
                .filter_map(|key| result.get(*key).and_then(Value::as_object))
                .find_map(|models| {
                    models
                        .get(model)
                        .and_then(Value::as_object)
                        .and_then(|counts| number(counts, "contextWindow"))
                })
        });
        Some(SessionContext {
            held: request.held,
            window,
        })
    }

    /// Remembers the size of a request the session made, from the usage of
    /// the message the model answered with. The messages of a helper the
    /// session started run in a context of their own and are passed over.
    fn note_request(&mut self, event: &Map<String, Value>) {
        if event
            .get("parent_tool_use_id")
            .is_some_and(|parent| !parent.is_null())
        {
            return;
        }
        let Some(message) = event.get("message").and_then(Value::as_object) else {
            return;
        };
        let model = text(message, "model");
        if model == Some(MADE_UP_MODEL) {
            return;
        }
        let Some(usage) = message.get("usage").and_then(Value::as_object) else {
            return;
        };
        let held = [
            "input_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
        ]
        .iter()
        .filter_map(|key| number(usage, key))
        .fold(0_u64, u64::saturating_add);
        if held > 0 {
            self.last_request = Some(LastRequest {
                held,
                model: model.map(str::to_owned),
            });
        }
    }

    fn reported_failure(&self, result: &Map<String, Value>) -> TurnFailure {
        if self.failure_kind.as_deref() == Some(NOT_SIGNED_IN) {
            return TurnFailure::NotSignedIn;
        }
        if let Some(hit) = self.plan_limit_in(result) {
            return TurnFailure::PlanLimitReached(hit);
        }
        // A failed turn can still have the subtype "success"; the kind is
        // then best told by the assistant's error or the terminal reason.
        let kind = self
            .failure_kind
            .clone()
            .or_else(|| {
                text(result, "subtype")
                    .filter(|subtype| *subtype != "success")
                    .map(str::to_owned)
            })
            .or_else(|| text(result, "terminal_reason").map(str::to_owned));
        TurnFailure::Reported { kind }
    }

    /// The limit of the plan, if the turn failed because it was reached.
    /// What the program reported in fields comes first: a plan that
    /// refused work. Its sentence for people is read next, from the result
    /// and from the last text: "You've hit your 5-hour limit · resets
    /// 10:40pm (Europe/Istanbul)". A moment given as a number is better
    /// than one read from words, so the reset of the fields wins, and the
    /// words fill in what the fields left out.
    fn plan_limit_in(&self, result: &Map<String, Value>) -> Option<PlanLimitHit> {
        let said = text(result, "result")
            .and_then(limit_in_words)
            .or_else(|| limit_in_words(&self.last_text));
        match (self.plan_refused.clone(), said) {
            (Some(reported), Some(said)) => Some(PlanLimitHit {
                window: reported.window.or(said.window),
                resets: match reported.resets {
                    PlanReset::Unknown => said.resets,
                    known @ (PlanReset::At(_) | PlanReset::Said(_)) => known,
                },
            }),
            (reported, said) => reported.or(said),
        }
    }

    fn decode_system(&self, event: &Map<String, Value>) -> Option<AgentEvent> {
        match text(event, "subtype")? {
            "init" => self.decode_init(event),
            "api_retry" => Some(AgentEvent::Retrying {
                attempt: number(event, "attempt").and_then(|value| u32::try_from(value).ok()),
                max_attempts: number(event, "max_retries")
                    .and_then(|value| u32::try_from(value).ok()),
                delay: number(event, "retry_delay_ms").map(Duration::from_millis),
                reason: text(event, "error")
                    .map(str::to_owned)
                    .or_else(|| number(event, "error_status").map(|status| status.to_string())),
            }),
            "permission_denied" => Some(AgentEvent::PermissionDenied(PermissionDenial {
                tool: text(event, "tool_name")?.to_owned(),
                tool_use_id: text(event, "tool_use_id").map(str::to_owned),
            })),
            "informational" if matches!(text(event, "level"), Some("warning" | "error")) => {
                Some(AgentEvent::Warning {
                    message: text(event, "content")?.to_owned(),
                })
            }
            _ => None,
        }
    }

    fn decode_init(&self, event: &Map<String, Value>) -> Option<AgentEvent> {
        let session_id = SessionId::parse(text(event, "session_id")?).ok()?;
        Some(AgentEvent::SessionStarted(SessionStarted {
            session_id,
            model: text(event, "model").unwrap_or_default().to_owned(),
            working_folder: PathBuf::from(text(event, "cwd").unwrap_or_default()),
            permission_mode: text(event, "permissionMode").unwrap_or_default().to_owned(),
            tools: names(event.get("tools")),
            skills: names(event.get("skills")),
            plugins: names(event.get("plugins")),
            mcp_servers: event
                .get("mcp_servers")
                .and_then(Value::as_array)
                .map(|servers| {
                    servers
                        .iter()
                        .filter_map(Value::as_object)
                        .filter_map(|server| {
                            Some(McpServer {
                                name: text(server, "name")?.to_owned(),
                                status: text(server, "status").unwrap_or_default().to_owned(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            program_version: text(event, "claude_code_version").map(str::to_owned),
        }))
    }

    fn decode_assistant(&mut self, event: &Map<String, Value>) -> Vec<AgentEvent> {
        if let Some(kind) = text(event, "error") {
            self.failure_kind = Some(kind.to_owned());
        }
        self.note_request(event);
        let mut events = Vec::new();
        for block in content_blocks(event) {
            match text(block, "type") {
                Some("text") => {
                    let Some(written) = text(block, "text") else {
                        continue;
                    };
                    if written.trim().is_empty() {
                        continue;
                    }
                    self.last_text = written.to_owned();
                    events.push(AgentEvent::Text {
                        text: written.to_owned(),
                    });
                }
                Some("tool_use") => {
                    let (Some(id), Some(tool)) = (text(block, "id"), text(block, "name")) else {
                        continue;
                    };
                    if let Some(said) = self.said_to_team(tool, block.get("input")) {
                        // Reported when the bus has answered.
                        self.awaiting_answer.insert(
                            id.to_owned(),
                            Awaiting {
                                taken: said,
                                tool: tool.to_owned(),
                                summary: self.summarize(block.get("input")),
                            },
                        );
                        continue;
                    }
                    if let Some(taken) = self.taken_in(tool, block.get("input")) {
                        self.taken_use = Some(id.to_owned());
                        events.push(taken);
                        continue;
                    }
                    self.tool_names.insert(id.to_owned(), tool.to_owned());
                    events.push(AgentEvent::ToolUse {
                        id: id.to_owned(),
                        tool: tool.to_owned(),
                        summary: self.summarize(block.get("input")),
                    });
                }
                _ => {}
            }
        }
        events
    }

    /// What a use of a tool of the team bus asks the manager or proposes:
    /// the first good question, interview or proposal of the turn. One
    /// that does not pass the check, and any after the first, are ordinary
    /// uses of a tool, which the server refuses.
    fn taken_in(&self, tool: &str, input: Option<&Value>) -> Option<AgentEvent> {
        if self.taken_use.is_some() {
            return None;
        }
        let input = input?;
        match tool {
            ASK_MANAGER_IN_SESSION => question_from(input).ok().map(AgentEvent::ManagerAsked),
            INTERVIEW_IN_SESSION => interview_from(input).ok().map(AgentEvent::InterviewAsked),
            PROPOSE_PLAN_IN_SESSION => proposal_from(input, &self.team, &self.tasks)
                .ok()
                .map(AgentEvent::PlanProposed),
            ASK_PERMISSION_IN_SESSION => permission_from(input, self.commands_askable)
                .ok()
                .map(AgentEvent::PermissionAsked),
            _ => None,
        }
    }

    /// What a use of `publish_handoff` or `send_message` hands over or
    /// says, if it passes the check. Whether the bus takes it is known only
    /// from its answer.
    fn said_to_team(&self, tool: &str, input: Option<&Value>) -> Option<AgentEvent> {
        let input = input?;
        match tool {
            PUBLISH_HANDOFF_IN_SESSION => {
                handoff_from(input).ok().map(AgentEvent::HandoffPublished)
            }
            SEND_MESSAGE_IN_SESSION => message_from(input, &self.teammates)
                .ok()
                .map(AgentEvent::MessageSent),
            // The bus allows only a few facts in a turn, so a fact too is
            // reported only once the bus took it.
            REMEMBER_IN_SESSION => fact_from(input).ok().map(AgentEvent::FactRemembered),
            _ => None,
        }
    }

    fn decode_user(&mut self, event: &Map<String, Value>) -> Vec<AgentEvent> {
        let results: Vec<(String, bool)> = content_blocks(event)
            .filter(|block| text(block, "type") == Some("tool_result"))
            .filter_map(|block| {
                Some((
                    text(block, "tool_use_id")?.to_owned(),
                    block.get("is_error").and_then(Value::as_bool) == Some(true),
                ))
            })
            .collect();
        let mut events = Vec::new();
        for (tool_use_id, failed) in results {
            if self.taken_use.as_deref() == Some(tool_use_id.as_str()) {
                // What was taken was reported; what the server answered
                // is only for the session.
                continue;
            }
            if let Some(awaiting) = self.awaiting_answer.remove(&tool_use_id) {
                if failed {
                    // The bus refused it, and the session was told why: an
                    // ordinary use of a tool that failed.
                    self.tool_names
                        .insert(tool_use_id.clone(), awaiting.tool.clone());
                    events.push(AgentEvent::ToolUse {
                        id: tool_use_id.clone(),
                        tool: awaiting.tool.clone(),
                        summary: awaiting.summary,
                    });
                    events.push(AgentEvent::ToolResult {
                        tool_use_id,
                        tool: Some(awaiting.tool),
                        failed: true,
                    });
                } else {
                    events.push(awaiting.taken);
                }
                continue;
            }
            events.push(AgentEvent::ToolResult {
                tool: self.tool_names.get(&tool_use_id).cloned(),
                tool_use_id,
                failed,
            });
        }
        events
    }

    /// Says in a few words what a tool is used on: the file, the command,
    /// the pattern or the address. Paths inside the working folder are
    /// shown relative to it.
    fn summarize(&self, input: Option<&Value>) -> String {
        const KEYS: &[&str] = &[
            "file_path",
            "notebook_path",
            "command",
            "pattern",
            "url",
            "query",
            "skill",
            "path",
            "description",
            "prompt",
        ];
        let Some(input) = input.and_then(Value::as_object) else {
            return String::new();
        };
        let Some(value) = KEYS.iter().find_map(|key| text(input, key)) else {
            return String::new();
        };
        let value = self
            .working_folder
            .as_deref()
            .and_then(|folder| value.strip_prefix(folder))
            .map_or(value, |rest| rest.trim_start_matches(['/', '\\']));
        shorten(value.lines().next().unwrap_or_default())
    }
}

/// The limit of the plan, if a `rate_limit_event` says that the plan
/// refuses work: its status is `rejected`. The window that limits is named
/// by `rateLimitType`, and `resetsAt` is when it resets.
fn refused_by_plan(event: &Map<String, Value>) -> Option<PlanLimitHit> {
    let info = event.get("rate_limit_info")?.as_object()?;
    if text(info, "status") != Some(REJECTED) {
        return None;
    }
    Some(PlanLimitHit {
        window: text(info, "rateLimitType").map(window_kind),
        resets: info
            .get("resetsAt")
            .and_then(Value::as_i64)
            .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
            .map_or(PlanReset::Unknown, PlanReset::At),
    })
}

/// A window of the plan, by the program's name for it.
fn window_kind(name: &str) -> PlanWindowKind {
    match name {
        "five_hour" => PlanWindowKind::FiveHours,
        "seven_day" => PlanWindowKind::Week,
        other => PlanWindowKind::Other(other.to_owned()),
    }
}

fn decode_plan_usage(event: &Map<String, Value>) -> Option<AgentEvent> {
    let info = event.get("rate_limit_info")?.as_object()?;
    let window = |kind: &str, details: &Map<String, Value>| PlanWindow {
        kind: window_kind(kind),
        used: details.get("utilization").and_then(Value::as_f64),
        resets_at: details
            .get("resetsAt")
            .and_then(Value::as_i64)
            .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single()),
        status: text(details, "status").map(str::to_owned),
    };
    let windows: Vec<PlanWindow> = match info.get("unifiedWindows").and_then(Value::as_object) {
        Some(windows) => windows
            .iter()
            .filter_map(|(kind, details)| Some(window(kind, details.as_object()?)))
            .collect(),
        None => text(info, "rateLimitType")
            .map(|kind| vec![window(kind, info)])
            .unwrap_or_default(),
    };
    let status = text(info, "status").map(str::to_owned);
    (!windows.is_empty() || status.is_some())
        .then_some(AgentEvent::PlanUsage(PlanUsage { windows, status }))
}

fn model_usage(models: &Map<String, Value>) -> Vec<ModelUsage> {
    models
        .iter()
        .filter_map(|(model, counts)| {
            let counts = counts.as_object()?;
            Some(ModelUsage {
                model: model.clone(),
                tokens: TokenCounts {
                    input: number(counts, "inputTokens").unwrap_or(0),
                    output: number(counts, "outputTokens").unwrap_or(0),
                    cache_read: number(counts, "cacheReadInputTokens").unwrap_or(0),
                    cache_written: number(counts, "cacheCreationInputTokens").unwrap_or(0),
                },
            })
        })
        .collect()
}

fn describe_early_end(code: Option<i32>, error_lines: &[String]) -> String {
    let mut message = match code {
        Some(code) => {
            format!("Claude Code ended with exit code {code} before the turn was finished")
        }
        None => "Claude Code ended before the turn was finished".to_owned(),
    };
    let details: Vec<&str> = error_lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if !details.is_empty() {
        message.push_str(": ");
        message.push_str(&details.join(" / "));
    }
    message
}

fn content_blocks(event: &Map<String, Value>) -> impl Iterator<Item = &Map<String, Value>> {
    event
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
}

fn names(list: Option<&Value>) -> Vec<String> {
    list.and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| match item {
                    Value::String(name) => Some(name.clone()),
                    Value::Object(details) => text(details, "name").map(str::to_owned),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

fn number(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(Value::as_u64)
}

fn shorten(text: &str) -> String {
    if text.chars().count() <= SUMMARY_LENGTH {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(SUMMARY_LENGTH - 3).collect();
    short.push_str("...");
    short
}

#[cfg(test)]
mod tests {
    use teamwright_application::{
        ManagerInterview, ManagerQuestion, PlanProposal, PlannedOwner, PublishedHandoff,
        SentMessage,
    };

    use super::*;

    const TEXT_REPLY: &str = include_str!("../tests/recordings/text_reply.jsonl");
    const FILE_WRITTEN: &str = include_str!("../tests/recordings/file_written.jsonl");
    const RESUMED_WITH_DENIALS: &str =
        include_str!("../tests/recordings/resumed_with_denials.jsonl");
    const NOT_SIGNED_IN_RECORDING: &str = include_str!("../tests/recordings/not_signed_in.jsonl");
    const MANAGER_ASKED: &str = include_str!("../tests/recordings/manager_asked.jsonl");
    const INTERVIEW_ASKED: &str = include_str!("../tests/recordings/interview_asked.jsonl");
    const PLAN_PROPOSED: &str = include_str!("../tests/recordings/plan_proposed.jsonl");

    fn exited() -> ProcessEnd {
        ProcessEnd::Exited {
            code: Some(0),
            error_lines: Vec::new(),
        }
    }

    fn replay(recording: &str) -> (Vec<AgentEvent>, Decoder) {
        let mut decoder = Decoder::new();
        let events = recording
            .lines()
            .flat_map(|line| decoder.decode_line(line))
            .collect();
        (events, decoder)
    }

    fn tokens(input: u64, output: u64, cache_read: u64, cache_written: u64) -> TokenCounts {
        TokenCounts {
            input,
            output,
            cache_read,
            cache_written,
        }
    }

    #[test]
    fn a_text_reply_starts_the_session_answers_and_reports_usage() {
        let (events, decoder) = replay(TEXT_REPLY);

        let AgentEvent::SessionStarted(started) = &events[0] else {
            panic!("the first event is {:?}", events[0]);
        };
        assert_eq!(
            started.session_id.as_str(),
            "88402b56-c55e-46ab-94bd-6050c16e1417"
        );
        assert_eq!(started.model, "claude-haiku-5-5");
        assert_eq!(started.permission_mode, "acceptEdits");
        assert_eq!(
            started.working_folder,
            PathBuf::from(r"C:\Users\sample\teamwright\workspace")
        );
        assert!(started.tools.contains(&"Read".to_owned()));
        assert_eq!(started.skills, ["sample-skill", "review", "init"]);
        assert_eq!(started.plugins, ["sample-plugin"]);
        assert_eq!(
            started.mcp_servers[1],
            McpServer {
                name: "claude.ai Sample Connector".to_owned(),
                status: "needs-auth".to_owned()
            }
        );
        assert_eq!(started.program_version.as_deref(), Some("2.1.322"));

        assert!(events.contains(&AgentEvent::Text {
            text: "ready".to_owned()
        }));
        let plan = events
            .iter()
            .find_map(|event| match event {
                AgentEvent::PlanUsage(plan) => Some(plan),
                _ => None,
            })
            .unwrap();
        assert_eq!(plan.windows.len(), 2);
        let week = plan
            .windows
            .iter()
            .find(|window| window.kind == PlanWindowKind::Week)
            .unwrap();
        assert_eq!(week.used, Some(0.25));
        assert_eq!(week.status.as_deref(), Some("allowed"));
        assert!(week.resets_at.is_some());

        let outcome = decoder.finish(exited());
        assert_eq!(outcome.failure, None);
        assert_eq!(outcome.final_text, "ready");
        assert_eq!(
            outcome.usage,
            [ModelUsage {
                model: "claude-haiku-5-5".to_owned(),
                tokens: tokens(2, 4, 10_115, 11_325),
            }]
        );
        assert_eq!(outcome.estimated_cost_usd, Some(0.002_368_35));
        assert_eq!(outcome.duration, Some(Duration::from_millis(2174)));
        assert_eq!(outcome.model_calls, Some(1));
        assert!(outcome.permission_denials.is_empty());
        assert_eq!(
            outcome.session_id.map(|id| id.as_str().to_owned()),
            Some("88402b56-c55e-46ab-94bd-6050c16e1417".to_owned())
        );
    }

    #[test]
    fn writing_a_file_is_a_tool_use_with_a_short_summary_and_a_result() {
        let (events, decoder) = replay(FILE_WRITTEN);

        let tool_use = events
            .iter()
            .find_map(|event| match event {
                AgentEvent::ToolUse { id, tool, summary } => Some((id, tool, summary)),
                _ => None,
            })
            .unwrap();
        assert_eq!(tool_use.1, "Write");
        assert_eq!(tool_use.2, "notes.txt");
        assert!(events.contains(&AgentEvent::ToolResult {
            tool_use_id: tool_use.0.clone(),
            tool: Some("Write".to_owned()),
            failed: false,
        }));

        let outcome = decoder.finish(exited());
        assert!(!outcome.failed());
        assert!(outcome.final_text.starts_with("I created `notes.txt`"));
        assert_eq!(outcome.total_tokens(), tokens(4, 318, 30_924, 15_973));
        assert_eq!(outcome.model_calls, Some(2));
    }

    #[test]
    fn a_resumed_turn_reports_denials_and_only_its_own_tokens() {
        let (events, decoder) = replay(RESUMED_WITH_DENIALS);

        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolUse { tool, summary, .. } if tool == "Bash" && summary == "echo hello"
        )));
        // A tool that was taken away is not a denial: the program answers
        // that the tool does not exist, and the result is a failure.
        let failed_results = events
            .iter()
            .filter(|event| matches!(event, AgentEvent::ToolResult { failed: true, .. }))
            .count();
        assert_eq!(failed_results, 2);
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::PermissionDenied(PermissionDenial { tool, tool_use_id: Some(_) }) if tool == "Write"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::Warning { message } if message.contains("Git Bash")
        )));

        let outcome = decoder.finish(exited());
        assert!(!outcome.failed());
        // `modelUsage` says 8 input and 818 output tokens: the whole session.
        assert_eq!(outcome.total_tokens(), tokens(4, 500, 59_202, 7_591));
        assert_eq!(outcome.estimated_cost_usd, Some(0.002_360_62));
        assert_eq!(outcome.permission_denials.len(), 1);
        assert_eq!(outcome.permission_denials[0].tool, "Write");
    }

    #[test]
    fn a_turn_without_a_sign_in_fails_as_not_signed_in() {
        let (events, decoder) = replay(NOT_SIGNED_IN_RECORDING);
        assert!(events.contains(&AgentEvent::Text {
            text: "Not logged in · Please run /login".to_owned()
        }));
        let outcome = decoder.finish(ProcessEnd::Exited {
            code: Some(1),
            error_lines: Vec::new(),
        });
        assert_eq!(outcome.failure, Some(TurnFailure::NotSignedIn));
        assert!(outcome.usage.is_empty());
        assert_eq!(outcome.estimated_cost_usd, None);
    }

    #[test]
    fn a_question_to_the_manager_is_reported_as_such_and_not_as_a_tool_use() {
        let (events, decoder) = replay(MANAGER_ASKED);

        let AgentEvent::SessionStarted(started) = &events[0] else {
            panic!("the first event is {:?}", events[0]);
        };
        assert!(
            started
                .tools
                .contains(&"mcp__teamwright__ask_manager".to_owned())
        );
        assert_eq!(
            started.mcp_servers,
            [McpServer {
                name: "teamwright".to_owned(),
                status: "connected".to_owned()
            }]
        );

        let questions: Vec<&ManagerQuestion> = events
            .iter()
            .filter_map(|event| match event {
                AgentEvent::ManagerAsked(question) => Some(question),
                _ => None,
            })
            .collect();
        let [question] = questions.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(question.question, "How should the sensor node be powered?");
        assert!(question.reason.starts_with("The power source changes"));
        let labels: Vec<&str> = question
            .options
            .iter()
            .map(|option| option.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "Battery only",
                "Solar with battery",
                "Mains (wired) power",
                "Energy harvesting"
            ]
        );
        assert!(
            question.options[0]
                .consequence
                .starts_with("Lowest hardware cost")
        );
        // The model recommended the second option, counting from 1.
        assert_eq!(question.recommended, Some(1));
        assert!(!events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolUse { .. } | AgentEvent::ToolResult { .. }
        )));

        let outcome = decoder.finish(exited());
        assert!(!outcome.failed());
        assert!(
            outcome
                .final_text
                .starts_with("I'm waiting for the manager's decision")
        );
        assert_eq!(outcome.total_tokens(), tokens(4, 761, 7198, 7945));
    }

    #[test]
    fn an_interview_is_reported_as_such_and_not_as_a_tool_use() {
        let (events, decoder) = replay(INTERVIEW_ASKED);

        let AgentEvent::SessionStarted(started) = &events[0] else {
            panic!("the first event is {:?}", events[0]);
        };
        assert!(started.tools.contains(&INTERVIEW_IN_SESSION.to_owned()));
        assert!(started.tools.contains(&PROPOSE_PLAN_IN_SESSION.to_owned()));
        let interviews: Vec<&ManagerInterview> = events
            .iter()
            .filter_map(|event| match event {
                AgentEvent::InterviewAsked(interview) => Some(interview),
                _ => None,
            })
            .collect();
        let [interview] = interviews.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(interview.questions.len(), 5);
        let first = &interview.questions[0];
        assert!(!first.question.is_empty());
        assert!(!first.reason.is_empty());
        assert!((2..=4).contains(&first.options.len()));
        // Every recommendation counts from 0 and names an option.
        assert!(interview.questions.iter().all(|question| {
            question
                .recommended
                .is_none_or(|place| place < question.options.len())
        }));
        assert!(!events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolUse { tool, .. } if tool.starts_with("mcp__teamwright__")
        )));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, AgentEvent::ToolResult { tool: None, .. }))
        );

        let outcome = decoder.finish(exited());
        assert!(!outcome.failed());
        assert!(!outcome.final_text.is_empty());
    }

    #[test]
    fn a_proposal_is_reported_as_such_with_its_team_and_its_plan() {
        let (events, decoder) = replay(PLAN_PROPOSED);

        let proposals: Vec<&PlanProposal> = events
            .iter()
            .filter_map(|event| match event {
                AgentEvent::PlanProposed(proposal) => Some(proposal),
                _ => None,
            })
            .collect();
        let [proposal] = proposals.as_slice() else {
            panic!("{events:?}");
        };
        assert!(!proposal.summary.is_empty());
        assert_eq!(proposal.members.len(), 3);
        assert_eq!(proposal.tasks.len(), 16);
        // The work that needs hands is the manager's.
        assert!(
            proposal
                .tasks
                .iter()
                .any(|task| task.owner == PlannedOwner::Manager)
        );
        // Every task waits only for tasks before it in this plan, counted
        // from 0.
        assert!(proposal.tasks.iter().enumerate().all(|(place, task)| {
            task.waits_for
                .iter()
                .all(|earlier| *earlier < proposal.tasks.len() && *earlier != place)
        }));
        assert!(
            proposal
                .tasks
                .iter()
                .all(|task| task.estimate_minutes.is_some())
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            AgentEvent::ToolUse { tool, .. } if tool.starts_with("mcp__teamwright__")
        )));

        let outcome = decoder.finish(exited());
        assert!(!outcome.failed());
        assert!(!outcome.final_text.is_empty());
    }

    #[test]
    fn a_proposal_may_give_a_task_to_a_member_of_the_team() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use",
            "id":"toolu_sample1","name":"mcp__teamwright__propose_plan","input":{
            "summary":"More work.","members":[],
            "tasks":[{"title":"Check the board","owner":"Hardware engineer"}]}}]}}"#;

        let without_team = Decoder::new().decode_line(line);
        let with_team = Decoder::for_team(vec!["Hardware engineer".to_owned()]).decode_line(line);

        assert!(matches!(
            without_team.as_slice(),
            [AgentEvent::ToolUse { .. }]
        ));
        assert!(matches!(
            with_team.as_slice(),
            [AgentEvent::PlanProposed(_)]
        ));
    }

    #[test]
    fn a_proposal_may_move_the_tasks_the_project_has() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use",
            "id":"toolu_sample1","name":"mcp__teamwright__propose_plan","input":{
            "summary":"Modules.","members":[],
            "tasks":[{"title":"Module 0: Set up the project","owner":"manager"}],
            "moves":[{"task":"AQN-1","part_of":1}]}}]}}"#;
        let board = vec![KnownTask {
            reference: "AQN-1".to_owned(),
            part_of: None,
            waits_for: Vec::new(),
        }];

        let without_tasks = Decoder::for_team(Vec::new()).decode_line(line);
        let with_tasks = Decoder::for_team(Vec::new())
            .with_tasks(board)
            .decode_line(line);

        assert!(matches!(
            without_tasks.as_slice(),
            [AgentEvent::ToolUse { .. }]
        ));
        let [AgentEvent::PlanProposed(proposal)] = with_tasks.as_slice() else {
            panic!("the proposal was not taken: {with_tasks:?}");
        };
        assert_eq!(proposal.moves.len(), 1);
    }

    #[test]
    fn only_the_first_of_a_question_an_interview_and_a_proposal_is_reported() {
        let mut decoder = Decoder::new();
        let interview = decoder.decode_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_sample1",
                "name":"mcp__teamwright__interview","input":{"questions":[{"question":"Which?"}]}}]}}"#,
        );
        let question = decoder.decode_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_sample2",
                "name":"mcp__teamwright__ask_manager","input":{"question":"Which?",
                "options":[{"label":"A"},{"label":"B"}]}}]}}"#,
        );
        assert!(matches!(
            interview.as_slice(),
            [AgentEvent::InterviewAsked(_)]
        ));
        assert!(matches!(question.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    #[test]
    fn a_question_that_does_not_pass_the_check_is_an_ordinary_failed_tool_use() {
        let mut decoder = Decoder::new();
        let mut events = decoder.decode_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"toolu_sample1",
                "name":"mcp__teamwright__ask_manager","input":{"question":"Which?","options":[]}}]}}"#,
        );
        events.extend(decoder.decode_line(
            r#"{"type":"user","message":{"content":[{"type":"tool_result",
                "tool_use_id":"toolu_sample1","is_error":true,"content":"not sent"}]}}"#,
        ));
        assert_eq!(
            events,
            [
                AgentEvent::ToolUse {
                    id: "toolu_sample1".to_owned(),
                    tool: "mcp__teamwright__ask_manager".to_owned(),
                    summary: String::new(),
                },
                AgentEvent::ToolResult {
                    tool_use_id: "toolu_sample1".to_owned(),
                    tool: Some("mcp__teamwright__ask_manager".to_owned()),
                    failed: true,
                },
            ]
        );
    }

    #[test]
    fn only_the_first_good_question_of_a_turn_is_reported() {
        let mut decoder = Decoder::new();
        let ask = |id: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","id":"{id}",
                    "name":"mcp__teamwright__ask_manager","input":{{"question":"Which?",
                    "options":[{{"label":"A"}},{{"label":"B"}}]}}}}]}}}}"#
            )
        };
        let first = decoder.decode_line(&ask("toolu_sample1"));
        let second = decoder.decode_line(&ask("toolu_sample2"));
        assert!(matches!(first.as_slice(), [AgentEvent::ManagerAsked(_)]));
        assert!(matches!(second.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    fn context(held: u64, window: Option<u64>) -> Option<SessionContext> {
        Some(SessionContext { held, window })
    }

    #[test]
    fn the_context_is_the_size_of_the_last_request_against_the_window_of_its_model() {
        // The last message of the assistant read 2 tokens, 20,809 from the
        // cache, and wrote 5,279 to it. The window of the model is in the
        // usage by model of the last message.
        let (_, decoder) = replay(FILE_WRITTEN);
        assert_eq!(
            decoder.finish(exited()).context,
            context(2 + 20_809 + 5279, Some(1_000_000))
        );

        let (_, decoder) = replay(TEXT_REPLY);
        assert_eq!(
            decoder.finish(exited()).context,
            context(2 + 10_115 + 11_325, Some(1_000_000))
        );

        // A resumed session holds what was said before: more than this
        // turn read.
        let (_, decoder) = replay(RESUMED_WITH_DENIALS);
        assert_eq!(
            decoder.finish(exited()).context,
            context(2 + 33_114 + 565, Some(1_000_000))
        );

        let (_, decoder) = replay(PLAN_PROPOSED);
        assert_eq!(
            decoder.finish(exited()).context,
            context(2 + 8496 + 2275, Some(1_000_000))
        );
    }

    #[test]
    fn a_turn_that_reached_no_model_reports_no_context() {
        let (_, decoder) = replay(NOT_SIGNED_IN_RECORDING);
        assert_eq!(
            decoder
                .finish(ProcessEnd::Exited {
                    code: Some(1),
                    error_lines: Vec::new(),
                })
                .context,
            None
        );
    }

    #[test]
    fn a_helper_does_not_count_and_an_early_end_knows_no_window() {
        let mut decoder = Decoder::new();
        decoder.decode_line(
            r#"{"type":"assistant","parent_tool_use_id":null,"message":{"model":"claude-sample",
                "usage":{"input_tokens":3,"cache_read_input_tokens":40000,
                "cache_creation_input_tokens":1000,"output_tokens":20},"content":[]}}"#,
        );
        decoder.decode_line(
            r#"{"type":"assistant","parent_tool_use_id":"toolu_sample1","message":{
                "model":"claude-sample","usage":{"input_tokens":900,"output_tokens":5},
                "content":[]}}"#,
        );

        let outcome = decoder.finish(ProcessEnd::Stopped);

        assert_eq!(outcome.context, context(41_003, None));
    }

    #[test]
    fn a_window_the_program_does_not_name_stays_unknown() {
        let mut decoder = Decoder::new();
        decoder.decode_line(
            r#"{"type":"assistant","message":{"model":"claude-sample",
                "usage":{"input_tokens":10,"cache_read_input_tokens":90},"content":[]}}"#,
        );
        decoder.decode_line(
            r#"{"type":"result","subtype":"success","is_error":false,
                "modelUsage":{"claude-other":{"outputTokens":7,"contextWindow":200000}}}"#,
        );

        assert_eq!(decoder.finish(exited()).context, context(100, None));
    }

    #[test]
    fn lines_that_are_not_understood_are_passed_over() {
        let mut decoder = Decoder::new();
        for line in [
            "",
            "not json",
            "[1, 2]",
            r#"{"type":"something_new","value":1}"#,
            r#"{"type":"system","subtype":"something_new"}"#,
            r#"{"type":"system"}"#,
            r#"{"type":"system","subtype":"init"}"#,
            r#"{"type":"assistant"}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use"}]}}"#,
            r#"{"type":"user","message":{"content":"a plain prompt"}}"#,
            r#"{"type":"rate_limit_event","rate_limit_info":{}}"#,
        ] {
            assert_eq!(decoder.decode_line(line), Vec::new(), "{line}");
        }
    }

    #[test]
    fn missing_counts_are_zero_and_a_failure_keeps_its_kind() {
        let mut decoder = Decoder::new();
        decoder.decode_line(
            r#"{"type":"result","subtype":"error_max_turns","is_error":true,"session_id":"abc",
                "modelUsage":{"claude-sample":{"outputTokens":7}}}"#,
        );
        let outcome = decoder.finish(exited());
        assert_eq!(
            outcome.failure,
            Some(TurnFailure::Reported {
                kind: Some("error_max_turns".to_owned())
            })
        );
        assert_eq!(outcome.total_tokens(), tokens(0, 7, 0, 0));
        assert_eq!(outcome.estimated_cost_usd, None);
        assert_eq!(
            outcome.session_id.map(|id| id.to_string()),
            Some("abc".to_owned())
        );
    }

    #[test]
    fn a_retry_notice_is_reported() {
        // Built from the shape the headless documentation describes; no
        // retry happened during the recordings.
        let mut decoder = Decoder::new();
        let events = decoder.decode_line(
            r#"{"type":"system","subtype":"api_retry","attempt":2,"max_retries":10,
                "retry_delay_ms":1500,"error_status":529,"error":"overloaded","session_id":"abc"}"#,
        );
        assert_eq!(
            events,
            [AgentEvent::Retrying {
                attempt: Some(2),
                max_attempts: Some(10),
                delay: Some(Duration::from_millis(1500)),
                reason: Some("overloaded".to_owned()),
            }]
        );
    }

    #[test]
    fn a_plan_limit_without_windows_is_read_from_its_type() {
        let mut decoder = Decoder::new();
        let events = decoder.decode_line(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning",
                "rateLimitType":"seven_day","utilization":0.9}}"#,
        );
        let [AgentEvent::PlanUsage(plan)] = events.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(plan.windows[0].kind, PlanWindowKind::Week);
        assert_eq!(plan.windows[0].used, Some(0.9));
        assert_eq!(plan.windows[0].resets_at, None);
    }

    /// A turn that hit the limit of the plan, in the shape of tonight's
    /// eight turns: an API error of type `rate_limit`, HTTP 429, and the
    /// sentence of the program. Built from what was seen in the output of
    /// the agents and from the schema in the program; it is not a
    /// recording.
    fn limit_reached_lines(with_event: bool) -> Vec<String> {
        let mut lines = vec![
            r#"{"type":"system","subtype":"init","session_id":"0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d31","model":"claude-sample","cwd":"/work"}"#.to_owned(),
        ];
        if with_event {
            lines.push(r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1795030800,"rateLimitType":"five_hour","utilization":1.0},"session_id":"0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d31"}"#.to_owned());
        }
        lines.push(r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"You've hit your 5-hour limit · resets 10:40pm (Europe/Istanbul)"}]},"error":"rate_limit","is_api_error_message":true,"api_error_status":429,"session_id":"0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d31"}"#.to_owned());
        lines.push(r#"{"type":"result","subtype":"success","is_error":true,"terminal_reason":"api_error","api_error_status":429,"result":"You've hit your 5-hour limit · resets 10:40pm (Europe/Istanbul)","session_id":"0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d31","modelUsage":{}}"#.to_owned());
        lines
    }

    fn decoded(lines: &[String]) -> (Vec<AgentEvent>, TurnOutcome) {
        let mut decoder = Decoder::new();
        let events = lines
            .iter()
            .flat_map(|line| decoder.decode_line(line))
            .collect();
        (events, decoder.finish(exited()))
    }

    #[test]
    fn a_plan_that_refuses_work_ends_the_turn_at_its_limit_with_the_moment_of_the_fields() {
        let (events, outcome) = decoded(&limit_reached_lines(true));

        let Some(TurnFailure::PlanLimitReached(hit)) = outcome.failure else {
            panic!("{:?}", outcome.failure);
        };
        assert_eq!(hit.window, Some(PlanWindowKind::FiveHours));
        assert_eq!(
            hit.resets,
            PlanReset::At(Utc.timestamp_opt(1_795_030_800, 0).unwrap())
        );
        // The refusal is also reported as the standing of the plan.
        assert!(events.iter().any(|event| matches!(
            event,
            AgentEvent::PlanUsage(PlanUsage { status: Some(status), .. }) if status == "rejected"
        )));
    }

    #[test]
    fn without_the_event_the_limit_and_its_reset_are_read_from_the_words() {
        let (_, outcome) = decoded(&limit_reached_lines(false));

        let Some(TurnFailure::PlanLimitReached(hit)) = outcome.failure else {
            panic!("{:?}", outcome.failure);
        };
        assert_eq!(hit.window, Some(PlanWindowKind::FiveHours));
        let PlanReset::Said(said) = hit.resets else {
            panic!("{:?}", hit.resets);
        };
        assert_eq!(
            said.time,
            chrono::NaiveTime::from_hms_opt(22, 40, 0).unwrap()
        );
        assert_eq!(said.zone.as_deref(), Some("Europe/Istanbul"));
    }

    #[test]
    fn a_program_that_ends_after_saying_the_limit_is_at_the_limit() {
        let decoder = Decoder::new();
        let outcome = decoder.finish(ProcessEnd::Exited {
            code: Some(1),
            error_lines: vec!["You've hit your weekly limit".to_owned()],
        });
        let Some(TurnFailure::PlanLimitReached(hit)) = outcome.failure else {
            panic!("{:?}", outcome.failure);
        };
        assert_eq!(hit.window, Some(PlanWindowKind::Week));
        assert_eq!(hit.resets, PlanReset::Unknown);
    }

    #[test]
    fn an_ordinary_rate_limit_error_is_an_ordinary_failure() {
        let mut decoder = Decoder::new();
        for line in [
            r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"API Error: Rate limit reached for requests"}]},"error":"rate_limit","session_id":"abc"}"#,
            r#"{"type":"result","subtype":"success","is_error":true,"result":"API Error: Rate limit reached for requests","session_id":"abc"}"#,
        ] {
            decoder.decode_line(line);
        }
        assert_eq!(
            decoder.finish(exited()).failure,
            Some(TurnFailure::Reported {
                kind: Some("rate_limit".to_owned())
            })
        );
    }

    #[test]
    fn a_plan_that_allows_work_is_no_limit() {
        let (_, decoder) = replay(TEXT_REPLY);
        assert_eq!(decoder.finish(exited()).failure, None);

        let mut decoder = Decoder::new();
        let events = decoder.decode_line(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","rateLimitType":"five_hour","utilization":0.92,"resetsAt":1795030800}}"#,
        );
        let [AgentEvent::PlanUsage(plan)] = events.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(plan.status.as_deref(), Some("allowed_warning"));
        decoder.decode_line(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"Overloaded"}"#,
        );
        assert_eq!(
            decoder.finish(exited()).failure,
            Some(TurnFailure::Reported {
                kind: Some("error_during_execution".to_owned())
            })
        );
    }

    #[test]
    fn a_turn_that_ends_early_says_why() {
        let mut decoder = Decoder::new();
        decoder.decode_line(TEXT_REPLY.lines().next().unwrap());
        let outcome = decoder.finish(ProcessEnd::Exited {
            code: Some(3),
            error_lines: vec!["first".to_owned(), String::new(), "second".to_owned()],
        });
        assert_eq!(
            outcome.failure,
            Some(TurnFailure::ProgramEnded {
                message: "Claude Code ended with exit code 3 before the turn was finished: \
                          first / second"
                    .to_owned()
            })
        );
        assert!(outcome.session_id.is_some());

        let outcome = Decoder::new().finish(ProcessEnd::Stopped);
        assert_eq!(outcome.failure, Some(TurnFailure::Stopped));
    }

    #[test]
    fn long_summaries_are_shortened_and_keep_one_line() {
        let decoder = Decoder::new();
        let long = "x".repeat(500);
        let summary = decoder.summarize(Some(&serde_json::json!({ "command": long })));
        assert_eq!(summary.chars().count(), SUMMARY_LENGTH);
        let summary = decoder.summarize(Some(&serde_json::json!({ "command": "first\nsecond" })));
        assert_eq!(summary, "first");
        assert_eq!(
            decoder.summarize(Some(&serde_json::json!({ "other": 1 }))),
            ""
        );
    }

    /// The line of an assistant that calls a tool of the team bus.
    fn call_line(id: &str, tool: &str, input: &serde_json::Value) -> String {
        serde_json::json!({
            "type": "assistant",
            "message": { "content": [{ "type": "tool_use", "id": id, "name": tool, "input": input }] }
        })
        .to_string()
    }

    /// The line that carries the answer of the team bus to a call.
    fn answer_line(id: &str, is_error: bool) -> String {
        serde_json::json!({
            "type": "user",
            "message": { "content": [{
                "type": "tool_result", "tool_use_id": id, "is_error": is_error,
                "content": [{ "type": "text", "text": "answer" }]
            }] }
        })
        .to_string()
    }

    // The lines of the next two tests were written from the documentation
    // of Claude Code (the stream of `claude -p --output-format stream-json`,
    // `SDKPermissionDeniedMessage` and the `permission_denials` of the
    // result, in the reference of the Agent SDK for TypeScript). A real run
    // that asks for a permission has never been recorded: the program was
    // not signed in when this was written.

    #[test]
    fn a_request_for_a_permission_is_reported_as_such_and_not_as_a_tool_use() {
        let mut decoder = Decoder::new().with_commands_askable(true);
        let input = serde_json::json!({
            "kind": "command",
            "what": "echo hello",
            "reason": "To show the manager that commands run."
        });

        let events = decoder.decode_line(&call_line(
            "toolu_1",
            crate::team_bus::ASK_PERMISSION_IN_SESSION,
            &input,
        ));
        let [AgentEvent::PermissionAsked(asked)] = events.as_slice() else {
            panic!("{events:?}");
        };
        assert_eq!(asked.rule.tool_rule(), "Bash(echo hello)");
        assert_eq!(asked.reason, "To show the manager that commands run.");
        // The answer of the bus is not a step of its own.
        assert!(
            decoder
                .decode_line(&answer_line("toolu_1", false))
                .is_empty()
        );

        // A second request in the same turn is an ordinary tool use, which
        // the bus refuses.
        let again = decoder.decode_line(&call_line(
            "toolu_2",
            crate::team_bus::ASK_PERMISSION_IN_SESSION,
            &input,
        ));
        assert!(matches!(again.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    #[test]
    fn a_request_for_a_command_where_none_may_be_asked_for_is_an_ordinary_tool_use() {
        let mut decoder = Decoder::new();
        let events = decoder.decode_line(&call_line(
            "toolu_1",
            crate::team_bus::ASK_PERMISSION_IN_SESSION,
            &serde_json::json!({ "kind": "command", "what": "npm test", "reason": "Tests." }),
        ));
        assert!(matches!(events.as_slice(), [AgentEvent::ToolUse { .. }]));
        let domain = decoder.decode_line(&call_line(
            "toolu_2",
            crate::team_bus::ASK_PERMISSION_IN_SESSION,
            &serde_json::json!({ "kind": "web_domain", "what": "docs.rs", "reason": "Docs." }),
        ));
        assert!(matches!(
            domain.as_slice(),
            [AgentEvent::PermissionAsked(_)]
        ));
    }

    #[test]
    fn a_request_that_could_never_be_allowed_is_an_ordinary_tool_use() {
        let mut decoder = Decoder::new().with_commands_askable(true);
        let events = decoder.decode_line(&call_line(
            "toolu_1",
            crate::team_bus::ASK_PERMISSION_IN_SESSION,
            &serde_json::json!({ "kind": "command", "what": "sudo ls", "reason": "To look." }),
        ));
        assert!(matches!(events.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    #[test]
    fn a_command_denied_without_a_prompt_is_a_denial_of_the_shell() {
        let mut decoder = Decoder::new();
        let denied = serde_json::json!({
            "type": "system",
            "subtype": "permission_denied",
            "tool_name": "Bash",
            "tool_use_id": "toolu_3",
            "decision_reason_type": "mode",
            "decision_reason": "no approval surface in this session; permission request denied automatically",
            "message": "Permission for this tool use was denied.",
            "uuid": "00000000-0000-4000-8000-000000000009",
            "session_id": "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d13"
        });
        assert_eq!(
            decoder.decode_line(&denied.to_string()),
            [AgentEvent::PermissionDenied(PermissionDenial {
                tool: "Bash".to_owned(),
                tool_use_id: Some("toolu_3".to_owned()),
            })]
        );
    }

    #[test]
    fn a_handoff_is_reported_once_the_bus_has_taken_it() {
        let mut decoder = Decoder::new();
        let input = serde_json::json!({
            "summary": "Pin map v1.", "contract": "SPI2.", "files": ["hardware/pinmap.md"]
        });

        let at_the_call =
            decoder.decode_line(&call_line("toolu_1", PUBLISH_HANDOFF_IN_SESSION, &input));
        let at_the_answer = decoder.decode_line(&answer_line("toolu_1", false));

        assert!(at_the_call.is_empty());
        assert_eq!(
            at_the_answer,
            [AgentEvent::HandoffPublished(PublishedHandoff {
                summary: "Pin map v1.".to_owned(),
                contract: "SPI2.".to_owned(),
                files: vec!["hardware/pinmap.md".to_owned()],
            })]
        );
    }

    #[test]
    fn a_handoff_the_bus_refused_is_an_ordinary_failed_tool_use() {
        let mut decoder = Decoder::new();
        let input = serde_json::json!({ "summary": "Done.", "files": ["missing.md"] });

        decoder.decode_line(&call_line("toolu_1", PUBLISH_HANDOFF_IN_SESSION, &input));
        let at_the_answer = decoder.decode_line(&answer_line("toolu_1", true));
        let unchecked = decoder.decode_line(&call_line(
            "toolu_2",
            PUBLISH_HANDOFF_IN_SESSION,
            &serde_json::json!({ "summary": "" }),
        ));

        assert_eq!(
            at_the_answer,
            [
                AgentEvent::ToolUse {
                    id: "toolu_1".to_owned(),
                    tool: PUBLISH_HANDOFF_IN_SESSION.to_owned(),
                    summary: String::new(),
                },
                AgentEvent::ToolResult {
                    tool_use_id: "toolu_1".to_owned(),
                    tool: Some(PUBLISH_HANDOFF_IN_SESSION.to_owned()),
                    failed: true,
                }
            ]
        );
        assert!(matches!(unchecked.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    #[test]
    fn a_message_is_reported_to_a_teammate_of_the_turn_once_the_bus_has_taken_it() {
        let mut decoder =
            Decoder::for_team(Vec::new()).with_teammates(vec!["Firmware engineer".to_owned()]);
        let input = serde_json::json!({
            "to": "Firmware Engineer", "kind": "note", "text": "The pin map is ready."
        });

        decoder.decode_line(&call_line("toolu_1", SEND_MESSAGE_IN_SESSION, &input));
        let events = decoder.decode_line(&answer_line("toolu_1", false));
        let to_nobody =
            Decoder::new().decode_line(&call_line("toolu_2", SEND_MESSAGE_IN_SESSION, &input));

        assert_eq!(
            events,
            [AgentEvent::MessageSent(SentMessage {
                to: "Firmware engineer".to_owned(),
                kind: teamwright_application::MessageKind::Note,
                text: "The pin map is ready.".to_owned(),
            })]
        );
        assert!(matches!(to_nobody.as_slice(), [AgentEvent::ToolUse { .. }]));
    }

    #[test]
    fn a_fact_is_reported_once_the_bus_has_taken_it_and_a_search_is_an_ordinary_tool_use() {
        let mut decoder = Decoder::new();
        let fact = serde_json::json!({ "title": "Sensor bus", "text": "On I2C0." });

        let at_the_call = decoder.decode_line(&call_line(
            "toolu_1",
            crate::team_bus::REMEMBER_IN_SESSION,
            &fact,
        ));
        let at_the_answer = decoder.decode_line(&answer_line("toolu_1", false));
        decoder.decode_line(&call_line(
            "toolu_2",
            crate::team_bus::REMEMBER_IN_SESSION,
            &fact,
        ));
        let refused = decoder.decode_line(&answer_line("toolu_2", true));
        let searched = decoder.decode_line(&call_line(
            "toolu_3",
            crate::team_bus::SEARCH_MEMORY_IN_SESSION,
            &serde_json::json!({ "query": "display bus" }),
        ));

        assert!(at_the_call.is_empty());
        assert_eq!(
            at_the_answer,
            [AgentEvent::FactRemembered(
                teamwright_application::RememberedFact {
                    title: "Sensor bus".to_owned(),
                    text: "On I2C0.".to_owned(),
                }
            )]
        );
        assert!(matches!(
            refused.as_slice(),
            [
                AgentEvent::ToolUse { .. },
                AgentEvent::ToolResult { failed: true, .. }
            ]
        ));
        assert_eq!(
            searched,
            [AgentEvent::ToolUse {
                id: "toolu_3".to_owned(),
                tool: crate::team_bus::SEARCH_MEMORY_IN_SESSION.to_owned(),
                summary: "display bus".to_owned(),
            }]
        );
    }
}
