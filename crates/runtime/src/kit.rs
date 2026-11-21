//! How the skills and connectors of a turn reach Claude Code
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! Every option and setting used here was read in the help of Claude Code
//! 2.1.289 and its documentation of November 2026:
//!
//! - A skill is a folder in the form of a plugin (`.claude-plugin/` and
//!   `skills/<name>/`), loaded for one session with `--plugin-dir`. The
//!   session knows it as `<name>:<name>`.
//! - A connector is an entry of the configuration given with
//!   `--mcp-config`. Claude Code replaces `${NAME}` in the `env` of a
//!   program and in the `headers` of an address with the variable of its
//!   own environment, so the value of a secret never passes through
//!   Teamwright's arguments, database or log.
//! - Its tools are allowed one by one, as `mcp__<connector>__<tool>`.

use serde_json::{Map, Value, json};
use teamwright_application::{AgentRuntimeError, ConnectorStart, KitConnector};
use teamwright_domain::{ConnectorName, ToolName};

/// The settings every confined session is started with, through
/// `--settings`, which restricted mode still reads.
///
/// - `disableAllHooks`: no hook runs, from wherever it would come. A hook
///   runs a command, and a specialist runs no commands.
/// - `disableSkillShellExecution`: a skill cannot run a command when it is
///   used. The check of a skill refuses one that would; this holds even if
///   its copy was changed on the disk since.
/// - `disableClaudeAiConnectors`: the connectors of the manager's own
///   account at claude.ai are not fetched. They are the manager's, not the
///   team's.
pub const SESSION_SETTINGS: &str = r#"{"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true}"#;

/// The name under which a session may use a skill, in a rule of
/// `--allowedTools`: the plugin and the skill share the name.
pub fn skill_rule(name: &str) -> String {
    format!("Skill({name}:{name})")
}

/// The name of a tool of a connector inside a session.
pub fn tool_in_session(connector: &str, tool: &str) -> String {
    format!("mcp__{connector}__{tool}")
}

/// The entry of a connector in the configuration of tool servers, and the
/// tools of it the session may use, by their names in the session.
///
/// The name and the tools are checked again by the rules of the domain, so
/// that nothing reaches the command line that the library would refuse.
///
/// # Errors
///
/// Returns [`AgentRuntimeError::InvalidRequest`] if the name or a tool
/// breaks a rule, or if no tool is given.
pub fn connector_entry(
    connector: &KitConnector,
) -> Result<(Value, Vec<String>), AgentRuntimeError> {
    let invalid = |message: String| AgentRuntimeError::InvalidRequest { message };
    let name = ConnectorName::parse(&connector.name).map_err(|error| invalid(error.to_string()))?;
    if connector.tools.is_empty() {
        return Err(invalid(format!(
            "the connector {name} is given without any of its tools"
        )));
    }
    let tools = connector
        .tools
        .iter()
        .map(|tool| {
            ToolName::parse(tool)
                .map(|tool| tool_in_session(name.as_str(), tool.as_str()))
                .map_err(|error| invalid(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let entry = match &connector.start {
        ConnectorStart::Program {
            program,
            arguments,
            variables,
        } => {
            let mut entry = json!({ "command": program, "args": arguments });
            if !variables.is_empty() {
                let environment: Map<String, Value> = variables
                    .iter()
                    .map(|variable| {
                        (
                            variable.to_string(),
                            Value::String(format!("${{{variable}}}")),
                        )
                    })
                    .collect();
                entry["env"] = Value::Object(environment);
            }
            entry
        }
        ConnectorStart::Address { url, headers } => {
            let mut entry = json!({ "type": "http", "url": url });
            if !headers.is_empty() {
                let sent: Map<String, Value> = headers
                    .iter()
                    .map(|header| {
                        let secret = format!("${{{}}}", header.variable());
                        let value = if header.bearer() {
                            format!("Bearer {secret}")
                        } else {
                            secret
                        };
                        (header.name().to_owned(), Value::String(value))
                    })
                    .collect();
                entry["headers"] = Value::Object(sent);
            }
            entry
        }
    };
    Ok((entry, tools))
}

#[cfg(test)]
mod tests {
    use teamwright_domain::{SecretHeader, VariableName};

    use super::*;

    fn variable(name: &str) -> VariableName {
        VariableName::parse(name).unwrap()
    }

    #[test]
    fn a_program_is_handed_its_secrets_by_name_only() {
        let connector = KitConnector {
            name: "parts".to_owned(),
            start: ConnectorStart::program(
                "npx",
                vec!["-y".to_owned(), "parts-server".to_owned()],
                vec![variable("PARTS_TOKEN")],
            )
            .unwrap(),
            tools: vec!["search_parts".to_owned()],
        };

        let (entry, tools) = connector_entry(&connector).unwrap();

        assert_eq!(
            entry,
            json!({
                "command": "npx",
                "args": ["-y", "parts-server"],
                "env": { "PARTS_TOKEN": "${PARTS_TOKEN}" }
            })
        );
        assert_eq!(tools, ["mcp__parts__search_parts"]);
    }

    #[test]
    fn an_address_is_sent_its_secret_in_a_header() {
        let connector = KitConnector {
            name: "tickets".to_owned(),
            start: ConnectorStart::address(
                "https://tickets.example/mcp",
                vec![
                    SecretHeader::new("Authorization", variable("TICKETS_TOKEN"), true).unwrap(),
                    SecretHeader::new("X-Workspace", variable("TICKETS_SPACE"), false).unwrap(),
                ],
            )
            .unwrap(),
            tools: vec!["read_ticket".to_owned(), "list_tickets".to_owned()],
        };

        let (entry, tools) = connector_entry(&connector).unwrap();

        assert_eq!(
            entry,
            json!({
                "type": "http",
                "url": "https://tickets.example/mcp",
                "headers": {
                    "Authorization": "Bearer ${TICKETS_TOKEN}",
                    "X-Workspace": "${TICKETS_SPACE}"
                }
            })
        );
        assert_eq!(
            tools,
            ["mcp__tickets__read_ticket", "mcp__tickets__list_tickets"]
        );
    }

    #[test]
    fn a_connector_that_the_library_would_refuse_is_refused() {
        let start = ConnectorStart::program("server", Vec::new(), Vec::new()).unwrap();
        let mut connector = KitConnector {
            name: "teamwright".to_owned(),
            start,
            tools: vec!["search".to_owned()],
        };
        assert!(connector_entry(&connector).is_err());
        connector.name = "parts".to_owned();
        connector.tools = Vec::new();
        assert!(connector_entry(&connector).is_err());
        connector.tools = vec!["*".to_owned()];
        assert!(connector_entry(&connector).is_err());
    }
}
