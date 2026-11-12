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
    AgentEvent, McpServer, ModelUsage, PermissionDenial, PlanUsage, PlanWindow, PlanWindowKind,
    SessionId, SessionStarted, TokenCounts, TurnFailure, TurnOutcome,
};

/// The longest summary of a tool's input, in characters.
const SUMMARY_LENGTH: usize = 120;

/// How Claude Code names the failure of a turn because nobody is signed in.
const NOT_SIGNED_IN: &str = "authentication_failed";

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
    last_text: String,
    failure_kind: Option<String>,
    result: Option<Map<String, Value>>,
}

impl Decoder {
    /// A decoder for a new turn.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
            Some("rate_limit_event") => decode_plan_usage(&event).into_iter().collect(),
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
                    ProcessEnd::Exited { code, error_lines } => TurnFailure::ProgramEnded {
                        message: describe_early_end(code, &error_lines),
                    },
                };
                TurnOutcome {
                    session_id: self.session_id,
                    failure: Some(failure),
                    final_text: self.last_text,
                    usage: Vec::new(),
                    estimated_cost_usd: None,
                    duration: None,
                    model_calls: None,
                    permission_denials: Vec::new(),
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
        }
    }

    fn reported_failure(&self, result: &Map<String, Value>) -> TurnFailure {
        if self.failure_kind.as_deref() == Some(NOT_SIGNED_IN) {
            return TurnFailure::NotSignedIn;
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

    fn decode_user(&self, event: &Map<String, Value>) -> Vec<AgentEvent> {
        content_blocks(event)
            .filter(|block| text(block, "type") == Some("tool_result"))
            .filter_map(|block| {
                let tool_use_id = text(block, "tool_use_id")?;
                Some(AgentEvent::ToolResult {
                    tool_use_id: tool_use_id.to_owned(),
                    tool: self.tool_names.get(tool_use_id).cloned(),
                    failed: block.get("is_error").and_then(Value::as_bool) == Some(true),
                })
            })
            .collect()
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

fn decode_plan_usage(event: &Map<String, Value>) -> Option<AgentEvent> {
    let info = event.get("rate_limit_info")?.as_object()?;
    let window = |kind: &str, details: &Map<String, Value>| PlanWindow {
        kind: match kind {
            "five_hour" => PlanWindowKind::FiveHours,
            "seven_day" => PlanWindowKind::Week,
            other => PlanWindowKind::Other(other.to_owned()),
        },
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
    (!windows.is_empty()).then_some(AgentEvent::PlanUsage(PlanUsage { windows }))
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
    use super::*;

    const TEXT_REPLY: &str = include_str!("../tests/recordings/text_reply.jsonl");
    const FILE_WRITTEN: &str = include_str!("../tests/recordings/file_written.jsonl");
    const RESUMED_WITH_DENIALS: &str =
        include_str!("../tests/recordings/resumed_with_denials.jsonl");
    const NOT_SIGNED_IN_RECORDING: &str = include_str!("../tests/recordings/not_signed_in.jsonl");

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
}
