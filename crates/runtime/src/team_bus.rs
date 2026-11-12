//! The team bus: the tool server through which a specialist reaches the
//! manager.
//!
//! Claude Code starts it for a turn as an MCP server over standard input
//! and output: one JSON-RPC 2.0 message per line. It has one tool,
//! `ask_manager`, which a session calls when a choice is the manager's to
//! make. The server only checks the question and tells the session to end
//! its turn. The question itself reaches Teamwright through the output of
//! the turn, where the decoder finds the call and checks it with the same
//! function, [`question_from`].
//!
//! The server keeps no state beyond one turn: Claude Code starts it with
//! the turn and leaves it when the turn ends.

use std::io::{self, BufRead, Write};

use serde_json::{Map, Value, json};
use teamwright_application::{ManagerQuestion, QuestionOption};

/// The name of the tool server, as Claude Code is told it.
pub const SERVER_NAME: &str = "teamwright";

/// The name of the tool, as the server offers it.
pub const ASK_MANAGER: &str = "ask_manager";

/// The name of the tool inside a session of Claude Code, which puts the
/// name of its server in front of it.
pub const ASK_MANAGER_IN_SESSION: &str = "mcp__teamwright__ask_manager";

/// What a session is told when its question has been taken.
pub const QUESTION_TAKEN: &str = "Your question is with the manager. End your turn now with one \
                                  sentence that says you are waiting for the decision.";

/// What a session is told when it asks a second time in one turn.
const ALREADY_ASKED: &str = "You have already asked the manager a question in this turn, and \
                             only one is taken. End your turn now with one sentence that says \
                             you are waiting for the decision.";

/// What the tool tells the model it is for.
const ASK_MANAGER_DESCRIPTION: &str = "Ask the manager of the project to decide. Use it when a \
     choice changes what is built, what it costs or when it is ready: such a choice is the \
     manager's, so do not guess. Give the question, why it matters, two to four options with \
     what each leads to, and the one you recommend. After calling it, end your turn: the answer \
     comes as the next message.";

/// The fewest and the most options a question may have.
const FEWEST_OPTIONS: usize = 2;
const MOST_OPTIONS: usize = 4;

/// The JSON-RPC error for a method the server does not have.
const METHOD_NOT_FOUND: i64 = -32_601;

/// The JSON-RPC error for a call with parameters the server cannot use.
const INVALID_PARAMS: i64 = -32_602;

/// The protocol version answered to a client that names none.
const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";

/// Serves the tool server until the input ends.
///
/// Every line of input is one message. A line that is not JSON is passed
/// over, a notification is not answered, and every answer is one line,
/// written at once.
///
/// # Errors
///
/// Returns an error if the input cannot be read or the output cannot be
/// written.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let mut bus = Bus::default();
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
        Some(place) => {
            let index = place
                .as_u64()
                .and_then(|place| usize::try_from(place).ok())
                .filter(|place| (1..=options.len()).contains(place))
                .ok_or_else(|| {
                    format!(
                        "\"recommended\" is the place of the option you recommend in the list, \
                         a whole number from 1 to {}.",
                        options.len()
                    )
                })?;
            Some(index - 1)
        }
    };

    Ok(ManagerQuestion {
        question,
        reason,
        options,
        recommended,
    })
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
    /// Whether a question has been taken in this turn.
    asked: bool,
}

impl Bus {
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
            "tools/list" => Ok(json!({ "tools": [ask_manager_tool()] })),
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
            .and_then(Value::as_str);
        if name != Some(ASK_MANAGER) {
            return Err((INVALID_PARAMS, format!("the only tool is {ASK_MANAGER}")));
        }
        if self.asked {
            return Ok(tool_result(ALREADY_ASKED, true));
        }
        let arguments = parameters
            .and_then(|parameters| parameters.get("arguments"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        Ok(match question_from(&arguments) {
            Ok(_) => {
                self.asked = true;
                tool_result(QUESTION_TAKEN, false)
            }
            Err(problem) => tool_result(
                &format!("The question was not sent to the manager. {problem}"),
                true,
            ),
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

/// The description of `ask_manager`, with the schema of its arguments.
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

    fn answer_of(bus: &mut Bus, message: &Value) -> Value {
        serde_json::from_str(&bus.answer(&message.to_string()).unwrap()).unwrap()
    }

    fn call(arguments: &Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "ask_manager", "arguments": arguments }
        })
    }

    fn text_of(result: &Value) -> &str {
        result["content"][0]["text"].as_str().unwrap()
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
    fn the_server_introduces_itself_with_the_version_of_the_client() {
        let mut bus = Bus::default();
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
    fn the_server_offers_one_tool_to_ask_the_manager() {
        let mut bus = Bus::default();
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
    fn a_good_question_is_taken_once_and_the_session_is_told_to_end_its_turn() {
        let mut bus = Bus::default();

        let first = answer_of(&mut bus, &call(&good_arguments()));
        let second = answer_of(&mut bus, &call(&good_arguments()));

        assert_eq!(first["id"], 2);
        assert_eq!(text_of(&first["result"]), QUESTION_TAKEN);
        assert_eq!(first["result"].get("isError"), None);
        assert_eq!(second["result"]["isError"], true);
        assert!(text_of(&second["result"]).contains("already asked"));
    }

    #[test]
    fn a_refused_question_says_what_to_change_and_may_be_asked_again() {
        let mut bus = Bus::default();

        let refused = answer_of(&mut bus, &call(&json!({ "question": "Which?" })));
        let taken = answer_of(&mut bus, &call(&good_arguments()));

        assert_eq!(refused["result"]["isError"], true);
        assert!(text_of(&refused["result"]).starts_with("The question was not sent"));
        assert_eq!(text_of(&taken["result"]), QUESTION_TAKEN);
    }

    #[test]
    fn identifiers_go_back_as_they_came_and_unknown_methods_are_errors() {
        let mut bus = Bus::default();
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

        serve(input.as_bytes(), &mut output).unwrap();

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
