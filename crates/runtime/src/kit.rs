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
//!   `--mcp-config`.
//! - A connector started as a program is started through Teamwright's own
//!   program, as the [`connector_launcher`](crate::connector_launcher):
//!   in an empty folder of Teamwright's own, never the folder of the
//!   project, with only the variables it names. Their values are in a file
//!   of Teamwright's own for the one turn, never on a command line.
//! - A connector at an address is sent its secret in a header: Claude Code
//!   replaces `${NAME}` in the `headers` with the variable of its own
//!   environment, so the value never passes through Teamwright's
//!   arguments, database or log.
//! - Its tools are allowed one by one, as `mcp__<connector>__<tool>`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use teamwright_application::{AgentRuntimeError, ConnectorStart, KitConnector};
use teamwright_domain::{ConnectorName, ToolName};

use crate::connector_launcher::Launch;

/// How the programs of connectors are started on this computer: through
/// Teamwright's own program, in the mode of the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorLauncher {
    /// Teamwright's own program.
    pub program: PathBuf,
    /// The arguments that start it as the launcher, before what it starts.
    pub arguments: Vec<String>,
    /// The empty folder of Teamwright's own every connector starts in.
    pub folder: PathBuf,
    /// The folder of Teamwright's own that holds the values of the
    /// variables of the connectors of a turn while it runs.
    pub secrets_folder: PathBuf,
}

/// How the connectors of one turn are started: the launcher, if this
/// computer has one, and for each program connector that is handed
/// variables, the file that holds their values for this turn.
#[derive(Debug, Clone, Default)]
pub struct ConnectorStarts<'a> {
    /// The launcher. Without it, no connector started as a program can be
    /// given to a turn.
    pub launcher: Option<&'a ConnectorLauncher>,
    /// The file of secrets of each connector, by its name.
    pub secrets: BTreeMap<String, PathBuf>,
}

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
/// breaks a rule, or if no tool is given; for a program, also if there is
/// no launcher, if the program is named by a relative path, if a variable
/// has no file of secrets, or if a path of the launcher holds `$`, which
/// Claude Code could replace.
pub fn connector_entry(
    connector: &KitConnector,
    starts: &ConnectorStarts<'_>,
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
            ConnectorStart::check_program_name(program)
                .map_err(|error| invalid(error.to_string()))?;
            let launcher = starts.launcher.ok_or_else(|| {
                invalid(format!(
                    "the connector {name} is a program, and this Teamwright cannot start it in a \
                     folder of its own"
                ))
            })?;
            let secrets = if variables.is_empty() {
                None
            } else {
                Some(starts.secrets.get(name.as_str()).ok_or_else(|| {
                    invalid(format!(
                        "the variables of the connector {name} were not written for the turn"
                    ))
                })?)
            };
            for path in [&launcher.program, &launcher.folder]
                .into_iter()
                .chain(secrets)
            {
                refuse_a_dollar(path)?;
            }
            let launch = Launch {
                folder: launcher.folder.clone(),
                secrets: secrets.cloned(),
                variables: variables.iter().map(ToString::to_string).collect(),
                program: program.clone(),
                arguments: arguments.clone(),
            };
            let mut launched = launcher.arguments.clone();
            launched.extend(launch.to_arguments());
            // No `env`: the launcher hands on the variables itself, and
            // Claude Code then has nothing of the connector to replace.
            json!({
                "command": launcher.program.to_string_lossy(),
                "args": launched,
            })
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

/// Refuses a path with `$`, which Claude Code could replace with a variable
/// of its environment in the configuration of tool servers.
fn refuse_a_dollar(path: &Path) -> Result<(), AgentRuntimeError> {
    if path.to_string_lossy().contains('$') {
        return Err(AgentRuntimeError::InvalidRequest {
            message: format!(
                "the path {} holds $, which Claude Code could replace; a connector cannot be \
                 started from it",
                path.display()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use teamwright_domain::{SecretHeader, VariableName};

    use super::*;

    fn variable(name: &str) -> VariableName {
        VariableName::parse(name).unwrap()
    }

    fn launcher() -> ConnectorLauncher {
        ConnectorLauncher {
            program: PathBuf::from("/opt/teamwright/teamwright"),
            arguments: vec!["--connector".to_owned()],
            folder: PathBuf::from("/data/connectors/start-here"),
            secrets_folder: PathBuf::from("/data/connectors/secrets"),
        }
    }

    fn parts() -> KitConnector {
        KitConnector {
            name: "parts".to_owned(),
            start: ConnectorStart::program(
                "npx",
                vec!["-y".to_owned(), "parts-server".to_owned()],
                vec![variable("PARTS_TOKEN")],
            )
            .unwrap(),
            tools: vec!["search_parts".to_owned()],
        }
    }

    #[test]
    fn a_program_is_started_through_the_launcher_and_handed_its_secrets_by_file() {
        let launcher = launcher();
        let starts = ConnectorStarts {
            launcher: Some(&launcher),
            secrets: BTreeMap::from([(
                "parts".to_owned(),
                PathBuf::from("/data/connectors/secrets/turn-1.json"),
            )]),
        };

        let (entry, tools) = connector_entry(&parts(), &starts).unwrap();

        assert_eq!(
            entry,
            json!({
                "command": "/opt/teamwright/teamwright",
                "args": [
                    "--connector",
                    "--folder", "/data/connectors/start-here",
                    "--secrets", "/data/connectors/secrets/turn-1.json",
                    "--pass", "PARTS_TOKEN",
                    "--", "npx", "-y", "parts-server"
                ]
            })
        );
        assert_eq!(tools, ["mcp__parts__search_parts"]);
        // No value and no `${…}` of a variable: nothing for Claude Code to
        // replace from the environment of the turn.
        assert!(!entry.to_string().contains('$'));
        assert!(entry.get("env").is_none());
    }

    #[test]
    fn a_program_without_variables_names_no_file() {
        let launcher = launcher();
        let starts = ConnectorStarts {
            launcher: Some(&launcher),
            secrets: BTreeMap::new(),
        };
        let mut plain = parts();
        plain.start =
            ConnectorStart::program("/usr/local/bin/parts", Vec::new(), Vec::new()).unwrap();

        let (entry, _) = connector_entry(&plain, &starts).unwrap();

        assert_eq!(
            entry["args"],
            json!([
                "--connector",
                "--folder",
                "/data/connectors/start-here",
                "--",
                "/usr/local/bin/parts"
            ])
        );
    }

    #[test]
    fn a_program_is_refused_without_a_launcher_a_file_or_a_sound_path() {
        let launcher = launcher();
        // No launcher: the program would start in the folder of the
        // project, so it is not started at all.
        assert!(connector_entry(&parts(), &ConnectorStarts::default()).is_err());
        // Variables without their file.
        let starts = ConnectorStarts {
            launcher: Some(&launcher),
            secrets: BTreeMap::new(),
        };
        assert!(connector_entry(&parts(), &starts).is_err());
        // A program stored before the rule about relative paths.
        let mut relative = parts();
        relative.start =
            ConnectorStart::program_as_stored("./server", Vec::new(), Vec::new()).unwrap();
        assert!(connector_entry(&relative, &starts).is_err());
        // A path Claude Code could fill in from its environment.
        let mut dollar = launcher.clone();
        dollar.folder = PathBuf::from("/data/${HOME}/start-here");
        let starts = ConnectorStarts {
            launcher: Some(&dollar),
            secrets: BTreeMap::new(),
        };
        let mut plain = parts();
        plain.start = ConnectorStart::program("npx", Vec::new(), Vec::new()).unwrap();
        assert!(connector_entry(&plain, &starts).is_err());
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

        let (entry, tools) = connector_entry(&connector, &ConnectorStarts::default()).unwrap();

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
        let launcher = launcher();
        let starts = ConnectorStarts {
            launcher: Some(&launcher),
            secrets: BTreeMap::new(),
        };
        let start = ConnectorStart::program("server", Vec::new(), Vec::new()).unwrap();
        let mut connector = KitConnector {
            name: "teamwright".to_owned(),
            start,
            tools: vec!["search".to_owned()],
        };
        assert!(connector_entry(&connector, &starts).is_err());
        connector.name = "parts".to_owned();
        connector.tools = Vec::new();
        assert!(connector_entry(&connector, &starts).is_err());
        connector.tools = vec!["*".to_owned()];
        assert!(connector_entry(&connector, &starts).is_err());
    }
}
