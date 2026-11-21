//! The command lines Claude Code is started with.
//!
//! Every option used here was checked against `claude --help` of Claude
//! Code 2.1.322. The prompt is never an argument: it goes to standard input,
//! which has no length limit and needs no quoting on any system.

use std::ffi::OsString;
use std::path::PathBuf;

use teamwright_application::{
    AgentRuntimeError, Effort, PermissionMode, PermissionProfile, ProgramVersion, SessionChoice,
    SessionId, TurnRequest,
};

use crate::kit::{SESSION_SETTINGS, connector_entry, skill_rule};
use crate::team_bus::{SERVER_NAME, TeamBus, name_in_session};

/// The settings that refuse reads outside the working folders, also for
/// commands. The key is documented for Claude Code 2.1.257 and later. A
/// key the program does not know could make it ignore the whole settings,
/// so nothing else is put here.
pub const READS_BLOCKED_SETTINGS: &str =
    r#"{"permissions":{"blockReadsOutsideWorkingDirectories":true}}"#;

/// The first version that can open a session in the Claude desktop app.
pub const FIRST_VERSION_WITH_DESKTOP: ProgramVersion = ProgramVersion::new(2, 1, 285);

/// The arguments that ask for the version.
pub fn version_arguments() -> Vec<OsString> {
    vec!["--version".into()]
}

/// The arguments that ask whether the user is signed in, answered as JSON.
pub fn sign_in_arguments() -> Vec<OsString> {
    ["auth", "status", "--json"].map(OsString::from).to_vec()
}

/// The arguments that open a session in the Claude desktop app.
pub fn desktop_arguments(session: &SessionId) -> Vec<OsString> {
    vec![
        "--desktop".into(),
        "--resume".into(),
        session.as_str().into(),
    ]
}

/// The arguments that run one turn of a session in headless mode.
///
/// `readable_folders` are the request's readable folders in their full
/// form. `team_bus` is how the tool server is started, if the turn is to
/// have it; it then offers the team tools of the request, and the session
/// may use each of them.
///
/// # Errors
///
/// Returns [`AgentRuntimeError::InvalidRequest`] if the prompt is empty, or
/// if a model, a folder or a tool rule is empty or starts with `-`, where
/// Claude Code would take it for an option.
pub fn turn_arguments(
    request: &TurnRequest,
    readable_folders: &[PathBuf],
    team_bus: Option<&TeamBus>,
) -> Result<Vec<OsString>, AgentRuntimeError> {
    if request.prompt.trim().is_empty() {
        return Err(invalid("the prompt is empty"));
    }

    // `--verbose` is required by Claude Code for `stream-json` output in
    // print mode. Without it the program refuses to start.
    let mut arguments: Vec<OsString> = ["-p", "--output-format", "stream-json", "--verbose"]
        .map(OsString::from)
        .to_vec();

    match &request.session {
        SessionChoice::New => {}
        SessionChoice::NewWithId(id) => push_pair(&mut arguments, "--session-id", id.as_str()),
        SessionChoice::Resume(id) => push_pair(&mut arguments, "--resume", id.as_str()),
    }

    if let Some(model) = &request.model {
        check_value("model", model)?;
        push_pair(&mut arguments, "--model", model);
    }
    if let Some(effort) = request.effort {
        push_pair(&mut arguments, "--effort", effort_name(effort));
    }
    if let Some(instructions) = &request.extra_instructions {
        // Joined to the option with `=` so that instructions starting with
        // a dash cannot be read as an option of their own.
        arguments.push(format!("--append-system-prompt={instructions}").into());
    }

    let permissions = &request.permissions;
    push_pair(
        &mut arguments,
        "--permission-mode",
        permission_mode_name(permissions.mode),
    );
    if permissions.deny_instead_of_asking {
        push_pair(&mut arguments, "--permission-prompts", "none");
    }
    // Restricted mode confines the file tools to the working folder and
    // ignores the user's own settings. A version of the program that does
    // not know the option refuses to start, which is the safe way to fail.
    if permissions.confined_to_working_folder {
        arguments.push("--restricted".into());
        // No setting source at all: not the user's own skills, commands,
        // agents and settings, and not those of the folders the session
        // works in (decision 0039). Joined with `=`, because the list is
        // empty; the program forwards it in this form itself.
        arguments.push("--setting-sources=".into());
    }
    // One argument of settings, JSON, which restricted mode still reads:
    // those of every confined session (decision 0039) and, for a turn that
    // may run commands, the block on reads outside the folders (decision
    // 0038). One argument, so that neither replaces the other.
    if let Some(settings) = settings_of(permissions) {
        push_pair(&mut arguments, "--settings", &settings);
    }
    if permissions.only_given_tool_servers {
        arguments.push("--strict-mcp-config".into());
    }

    let mut allowed_tools = permissions.allowed_tools.clone();
    let mut disallowed_tools = permissions.disallowed_tools.clone();
    // Each skill is a plugin of its own for this session. Its folder may be
    // read, for the files the skill points to, and not written.
    let mut skill_folders = Vec::new();
    for skill in &request.kit.skills {
        check_value("skill", &skill.name)?;
        let folder = skill.folder.to_string_lossy().into_owned();
        check_value("skill folder", &folder)?;
        push_pair(&mut arguments, "--plugin-dir", &folder);
        allowed_tools.push(skill_rule(&skill.name));
        skill_folders.push(skill.folder.clone());
    }
    // The configuration is one argument, JSON, as the program reads a
    // configuration given in place of a file: the team bus and the
    // connectors, each under its own name.
    let mut servers = serde_json::Map::new();
    if let Some(team_bus) = team_bus {
        servers.insert(SERVER_NAME.to_owned(), team_bus.server(&request.team_tools));
        allowed_tools.extend(request.team_tools.iter().map(name_in_session));
    }
    for connector in &request.kit.connectors {
        let (entry, tools) = connector_entry(connector)?;
        if servers.insert(connector.name.clone(), entry).is_some() {
            return Err(invalid(&format!(
                "the connector {} is given twice",
                connector.name
            )));
        }
        // A connector reaches outside the folder: only the tools the
        // manager gave are allowed, one by one. Its other tools are denied
        // when the session tries them, as nobody can be asked.
        allowed_tools.extend(tools);
    }
    if !servers.is_empty() {
        push_pair(
            &mut arguments,
            "--mcp-config",
            &serde_json::json!({ "mcpServers": servers }).to_string(),
        );
    }
    for folder in readable_folders.iter().chain(&skill_folders) {
        let folder = folder.to_string_lossy();
        check_value("folder", &folder)?;
        push_pair(&mut arguments, "--add-dir", &folder);
        // A rule about a path uses forward slashes on every system. A
        // session given this rule read a text file and a PDF in such a
        // folder and was refused when it tried to write there.
        disallowed_tools.push(format!(
            "Edit({}/**)",
            folder.replace('\\', "/").trim_end_matches('/')
        ));
    }

    // The lists come last. Each takes every following argument up to the
    // next option, and an empty list must not be given at all: `--tools`
    // with nothing after it would mean "no tools".
    if let Some(tools) = &permissions.built_in_tools {
        push_list(&mut arguments, "--tools", tools)?;
    }
    push_list(&mut arguments, "--allowedTools", &allowed_tools)?;
    push_list(&mut arguments, "--disallowedTools", &disallowed_tools)?;

    Ok(arguments)
}

/// The settings a session is started with, as one JSON object, or nothing.
fn settings_of(permissions: &PermissionProfile) -> Option<String> {
    match (
        permissions.confined_to_working_folder,
        permissions.reads_blocked_outside_working_folders,
    ) {
        (false, false) => None,
        (true, false) => Some(SESSION_SETTINGS.to_owned()),
        (false, true) => Some(READS_BLOCKED_SETTINGS.to_owned()),
        // Both objects are constants of one level whose last character
        // closes them, so the second's keys join the first's.
        (true, true) => Some(format!(
            "{},{}",
            SESSION_SETTINGS.trim_end_matches('}'),
            READS_BLOCKED_SETTINGS.trim_start_matches('{')
        )),
    }
}

fn push_pair(arguments: &mut Vec<OsString>, option: &str, value: &str) {
    arguments.push(option.into());
    arguments.push(value.into());
}

fn push_list(
    arguments: &mut Vec<OsString>,
    option: &str,
    values: &[String],
) -> Result<(), AgentRuntimeError> {
    if values.is_empty() {
        return Ok(());
    }
    arguments.push(option.into());
    for value in values {
        check_value("tool rule", value)?;
        arguments.push(value.into());
    }
    Ok(())
}

fn check_value(what: &str, value: &str) -> Result<(), AgentRuntimeError> {
    if value.trim().is_empty() {
        Err(invalid(&format!("a {what} is empty")))
    } else if value.starts_with('-') {
        Err(invalid(&format!(
            "the {what} \"{value}\" starts with a dash"
        )))
    } else {
        Ok(())
    }
}

fn invalid(message: &str) -> AgentRuntimeError {
    AgentRuntimeError::InvalidRequest {
        message: message.to_owned(),
    }
}

const fn effort_name(effort: Effort) -> &'static str {
    match effort {
        Effort::Low => "low",
        Effort::Medium => "medium",
        Effort::High => "high",
        Effort::ExtraHigh => "xhigh",
        Effort::Max => "max",
    }
}

const fn permission_mode_name(mode: PermissionMode) -> &'static str {
    match mode {
        PermissionMode::Manual => "manual",
        PermissionMode::AcceptEdits => "acceptEdits",
        PermissionMode::DontAsk => "dontAsk",
        PermissionMode::Plan => "plan",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use teamwright_application::{
        ConnectorStart, KitConnector, KitSkill, PermissionProfile, TeamTool, TurnKit,
    };
    use teamwright_domain::{PermissionKind, PermissionRule};
    use teamwright_domain::{SecretHeader, VariableName};

    use super::*;

    fn request() -> TurnRequest {
        TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Write the plan.".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: vec![TeamTool::AskManager],
            readable_folders: Vec::new(),
            kit: TurnKit::default(),
        }
    }

    fn bus() -> TeamBus {
        TeamBus {
            program: PathBuf::from(r"C:\Program Files\Teamwright\teamwright.exe"),
            arguments: vec!["--team-bus".to_owned()],
        }
    }

    #[test]
    fn a_turn_with_the_team_bus_is_given_the_server_and_may_use_its_tool() {
        let arguments = text(&turn_arguments(&request(), &[], Some(&bus())).unwrap());

        let configuration = arguments
            .windows(2)
            .find(|pair| pair[0] == "--mcp-config")
            .map(|pair| pair[1].clone())
            .unwrap();
        let configuration: serde_json::Value = serde_json::from_str(&configuration).unwrap();
        assert_eq!(
            configuration,
            serde_json::json!({
                "mcpServers": {
                    "teamwright": {
                        "command": r"C:\Program Files\Teamwright\teamwright.exe",
                        "args": ["--team-bus", "ask_manager"]
                    }
                }
            })
        );
        let allowed_at = arguments
            .iter()
            .position(|argument| argument == "--allowedTools")
            .unwrap();
        let denied_at = arguments
            .iter()
            .position(|argument| argument == "--disallowedTools")
            .unwrap();
        assert_eq!(
            arguments[allowed_at + 1..denied_at],
            ["Read", "Glob", "Grep", "mcp__teamwright__ask_manager"]
        );
        let configuration_at = arguments
            .iter()
            .position(|argument| argument == "--mcp-config")
            .unwrap();
        assert!(configuration_at < allowed_at);
    }

    #[test]
    fn the_advisor_is_given_its_two_tools_with_the_team_and_may_only_read() {
        let mut advisor = request();
        advisor.permissions = PermissionProfile::advisor();
        advisor.team_tools = vec![
            TeamTool::Interview,
            TeamTool::ProposePlan {
                team: vec!["Hardware engineer".to_owned()],
            },
        ];

        let arguments = text(&turn_arguments(&advisor, &[], Some(&bus())).unwrap());

        let configuration = arguments
            .windows(2)
            .find(|pair| pair[0] == "--mcp-config")
            .map(|pair| pair[1].clone())
            .unwrap();
        let configuration: serde_json::Value = serde_json::from_str(&configuration).unwrap();
        assert_eq!(
            configuration["mcpServers"]["teamwright"]["args"],
            serde_json::json!([
                "--team-bus",
                "interview",
                "propose_plan",
                "--member",
                "Hardware engineer"
            ])
        );
        let tools_at = arguments
            .iter()
            .position(|argument| argument == "--tools")
            .unwrap();
        let allowed_at = arguments
            .iter()
            .position(|argument| argument == "--allowedTools")
            .unwrap();
        let denied_at = arguments
            .iter()
            .position(|argument| argument == "--disallowedTools")
            .unwrap();
        assert_eq!(
            arguments[tools_at + 1..allowed_at],
            ["Read", "Glob", "Grep"]
        );
        assert_eq!(
            arguments[allowed_at + 1..denied_at],
            [
                "Read",
                "Glob",
                "Grep",
                "mcp__teamwright__interview",
                "mcp__teamwright__propose_plan"
            ]
        );
    }

    #[test]
    fn a_readable_folder_is_added_for_reading_and_denied_for_writing() {
        let folders = [
            PathBuf::from(r"C:\Users\sample\Documents\Teamwright\SSN"),
            PathBuf::from("/home/sample/shared/"),
        ];

        let arguments = text(&turn_arguments(&request(), &folders, None).unwrap());

        let tools_at = arguments
            .iter()
            .position(|argument| argument == "--tools")
            .unwrap();
        let added: Vec<(usize, &String)> = arguments
            .windows(2)
            .enumerate()
            .filter(|(_, pair)| pair[0] == "--add-dir")
            .map(|(at, pair)| (at, &pair[1]))
            .collect();
        assert_eq!(added.len(), 2);
        assert_eq!(added[0].1, r"C:\Users\sample\Documents\Teamwright\SSN");
        assert!(added.iter().all(|(at, _)| *at < tools_at));
        let denied_at = arguments
            .iter()
            .position(|argument| argument == "--disallowedTools")
            .unwrap();
        assert_eq!(
            arguments[denied_at + 1..],
            [
                "Bash",
                "PowerShell",
                "Edit(C:/Users/sample/Documents/Teamwright/SSN/**)",
                "Edit(/home/sample/shared/**)",
            ]
        );
    }

    fn text(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_specialists_new_turn_has_exactly_these_arguments() {
        let arguments = text(&turn_arguments(&request(), &[], None).unwrap());
        assert_eq!(
            arguments,
            [
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "acceptEdits",
                "--permission-prompts",
                "none",
                "--restricted",
                "--setting-sources=",
                "--settings",
                r#"{"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true}"#,
                "--strict-mcp-config",
                "--tools",
                "Read",
                "Glob",
                "Grep",
                "Edit",
                "Write",
                "NotebookEdit",
                "Skill",
                "ToolSearch",
                "--allowedTools",
                "Read",
                "Glob",
                "Grep",
                "--disallowedTools",
                "Bash",
                "PowerShell",
            ]
        );
    }

    fn granted(rules: &[(PermissionKind, &str)]) -> TurnRequest {
        let rules: Vec<PermissionRule> = rules
            .iter()
            .map(|(kind, text)| PermissionRule::new(*kind, text).unwrap())
            .collect();
        let mut granted = request();
        granted.permissions = PermissionProfile::specialist().with_granted(&rules);
        granted
    }

    #[test]
    fn a_turn_allowed_one_command_has_exactly_these_arguments() {
        let arguments = text(
            &turn_arguments(
                &granted(&[(PermissionKind::Command, "echo hello")]),
                &[],
                None,
            )
            .unwrap(),
        );
        assert_eq!(
            arguments,
            [
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "acceptEdits",
                "--permission-prompts",
                "none",
                "--restricted",
                "--setting-sources=",
                "--settings",
                r#"{"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true,"permissions":{"blockReadsOutsideWorkingDirectories":true}}"#,
                "--strict-mcp-config",
                "--tools",
                "Read",
                "Glob",
                "Grep",
                "Edit",
                "Write",
                "NotebookEdit",
                "Skill",
                "ToolSearch",
                "Bash",
                "--allowedTools",
                "Read",
                "Glob",
                "Grep",
                "Bash(echo hello)",
                "--disallowedTools",
                "PowerShell",
                "Bash(rm *)",
                "Bash(rmdir *)",
                "Bash(mv *)",
                "Bash(cp *)",
                "Bash(mkdir *)",
                "Bash(touch *)",
                "Bash(ln *)",
                "Bash(chmod *)",
                "Bash(chown *)",
                "Bash(chgrp *)",
                "Bash(dd *)",
                "Bash(tee *)",
                "Bash(truncate *)",
                "Bash(shred *)",
                "Bash(install *)",
            ]
        );
    }

    #[test]
    fn the_settings_of_a_session_are_one_json_object_with_every_key() {
        let mut profile = PermissionProfile::specialist();
        profile.reads_blocked_outside_working_folders = true;
        let settings: serde_json::Value =
            serde_json::from_str(&settings_of(&profile).unwrap()).unwrap();
        assert_eq!(settings["disableAllHooks"], true);
        assert_eq!(settings["disableClaudeAiConnectors"], true);
        assert_eq!(
            settings["permissions"]["blockReadsOutsideWorkingDirectories"],
            true
        );
        profile.confined_to_working_folder = false;
        assert_eq!(
            settings_of(&profile).as_deref(),
            Some(READS_BLOCKED_SETTINGS)
        );
        profile.reads_blocked_outside_working_folders = false;
        assert_eq!(settings_of(&profile), None);
    }

    #[test]
    fn a_turn_allowed_a_prefix_and_a_domain_gets_both_rules_and_both_tools() {
        let arguments = text(
            &turn_arguments(
                &granted(&[
                    (PermissionKind::CommandPrefix, "cargo test"),
                    (PermissionKind::WebDomain, "docs.rs"),
                ]),
                &[],
                None,
            )
            .unwrap(),
        );
        let tools_at = arguments.iter().position(|a| a == "--tools").unwrap();
        let allowed_at = arguments
            .iter()
            .position(|a| a == "--allowedTools")
            .unwrap();
        let denied_at = arguments
            .iter()
            .position(|a| a == "--disallowedTools")
            .unwrap();
        assert!(arguments[tools_at..allowed_at].contains(&"Bash".to_owned()));
        assert!(arguments[tools_at..allowed_at].contains(&"WebFetch".to_owned()));
        assert_eq!(
            arguments[allowed_at + 1..denied_at],
            [
                "Read",
                "Glob",
                "Grep",
                "Bash(cargo test *)",
                "WebFetch(domain:docs.rs)"
            ]
        );
    }

    #[test]
    fn a_turn_allowed_only_a_domain_has_no_shell_and_no_settings() {
        let arguments = text(
            &turn_arguments(
                &granted(&[(PermissionKind::WebDomain, "docs.rs")]),
                &[],
                None,
            )
            .unwrap(),
        );
        let settings: Vec<&String> = arguments
            .windows(2)
            .filter(|pair| pair[0] == "--settings")
            .map(|pair| &pair[1])
            .collect();
        assert_eq!(settings, [SESSION_SETTINGS]);
        let denied_at = arguments
            .iter()
            .position(|a| a == "--disallowedTools")
            .unwrap();
        assert_eq!(arguments[denied_at + 1..], ["Bash", "PowerShell"]);
        let tools_at = arguments.iter().position(|a| a == "--tools").unwrap();
        let allowed_at = arguments
            .iter()
            .position(|a| a == "--allowedTools")
            .unwrap();
        assert!(!arguments[tools_at..allowed_at].contains(&"Bash".to_owned()));
    }

    #[test]
    fn no_granted_turn_can_approve_everything_or_leave_its_folder() {
        for rules in [
            vec![(PermissionKind::Command, "echo hello")],
            vec![(PermissionKind::CommandPrefix, "npm run")],
            vec![(PermissionKind::WebDomain, "*.example.com")],
        ] {
            let arguments = text(&turn_arguments(&granted(&rules), &[], None).unwrap());
            for kept in ["--restricted", "--strict-mcp-config"] {
                assert!(arguments.contains(&kept.to_owned()));
            }
            assert!(
                arguments
                    .windows(2)
                    .any(|pair| pair == ["--permission-prompts", "none"])
            );
            assert!(
                arguments
                    .windows(2)
                    .any(|pair| pair == ["--permission-mode", "acceptEdits"])
            );
            for never in [
                "--dangerously-skip-permissions",
                "--allow-dangerously-skip-permissions",
                "--permission-prompt-tool",
                "bypassPermissions",
                "auto",
                "Bash",
                "Bash(*)",
                "WebFetch",
                "WebFetch(domain:*)",
            ] {
                let allowed_at = arguments
                    .iter()
                    .position(|a| a == "--allowedTools")
                    .unwrap();
                let denied_at = arguments
                    .iter()
                    .position(|a| a == "--disallowedTools")
                    .unwrap();
                assert!(
                    !arguments[allowed_at..denied_at].contains(&never.to_owned()),
                    "{never} is allowed"
                );
                if never.starts_with("--") || never == "bypassPermissions" || never == "auto" {
                    assert!(!arguments.contains(&never.to_owned()), "{never} is given");
                }
            }
        }
    }

    #[test]
    fn the_prompt_is_never_an_argument() {
        let arguments = text(&turn_arguments(&request(), &[], None).unwrap());
        assert!(
            !arguments
                .iter()
                .any(|argument| argument.contains("Write the plan"))
        );
    }

    #[test]
    fn a_resumed_turn_names_its_session() {
        let id = SessionId::parse("0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10").unwrap();
        let mut resumed = request();
        resumed.session = SessionChoice::Resume(id.clone());
        let arguments = text(&turn_arguments(&resumed, &[], None).unwrap());
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--resume", id.as_str()])
        );

        let mut chosen = request();
        chosen.session = SessionChoice::NewWithId(id.clone());
        let arguments = text(&turn_arguments(&chosen, &[], None).unwrap());
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--session-id", id.as_str()])
        );
    }

    #[test]
    fn model_effort_and_instructions_are_passed_on() {
        let mut detailed = request();
        detailed.model = Some("haiku".to_owned());
        detailed.effort = Some(Effort::ExtraHigh);
        detailed.extra_instructions = Some("-You are the tester.".to_owned());
        let arguments = text(&turn_arguments(&detailed, &[], None).unwrap());
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--model", "haiku"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--effort", "xhigh"])
        );
        assert!(arguments.contains(&"--append-system-prompt=-You are the tester.".to_owned()));
    }

    #[test]
    fn empty_lists_are_left_out_and_asking_is_allowed_when_wished() {
        let mut open = request();
        open.permissions = PermissionProfile {
            built_in_tools: None,
            allowed_tools: Vec::new(),
            disallowed_tools: Vec::new(),
            mode: PermissionMode::DontAsk,
            deny_instead_of_asking: false,
            confined_to_working_folder: false,
            only_given_tool_servers: false,
            reads_blocked_outside_working_folders: false,
        };
        let arguments = text(&turn_arguments(&open, &[], None).unwrap());
        assert_eq!(
            arguments,
            [
                "-p",
                "--output-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "dontAsk"
            ]
        );
    }

    #[test]
    fn values_that_would_be_read_as_options_are_refused() {
        let mut sneaky = request();
        sneaky.permissions.allowed_tools = vec!["--dangerously-skip-permissions".to_owned()];
        assert!(turn_arguments(&sneaky, &[], None).is_err());

        let mut sneaky = request();
        sneaky.model = Some("-x".to_owned());
        assert!(turn_arguments(&sneaky, &[], None).is_err());

        let mut empty = request();
        empty.prompt = "  ".to_owned();
        assert!(turn_arguments(&empty, &[], None).is_err());
    }

    #[test]
    fn the_desktop_app_is_asked_to_resume_the_session() {
        let id = SessionId::parse("abc").unwrap();
        assert_eq!(
            text(&desktop_arguments(&id)),
            ["--desktop", "--resume", "abc"]
        );
    }

    fn kit() -> TurnKit {
        TurnKit {
            skills: vec![KitSkill {
                name: "pcb-review".to_owned(),
                folder: PathBuf::from("/home/sample/.teamwright/skills/0196f3a2"),
            }],
            connectors: vec![
                KitConnector {
                    name: "parts".to_owned(),
                    start: ConnectorStart::program(
                        "parts-server",
                        vec!["--stdio".to_owned()],
                        vec![VariableName::parse("PARTS_TOKEN").unwrap()],
                    )
                    .unwrap(),
                    tools: vec!["search_parts".to_owned()],
                },
                KitConnector {
                    name: "tickets".to_owned(),
                    start: ConnectorStart::address(
                        "https://tickets.example/mcp",
                        vec![
                            SecretHeader::new(
                                "Authorization",
                                VariableName::parse("TICKETS_TOKEN").unwrap(),
                                true,
                            )
                            .unwrap(),
                        ],
                    )
                    .unwrap(),
                    tools: vec!["read_ticket".to_owned(), "list_tickets".to_owned()],
                },
            ],
        }
    }

    #[test]
    fn a_specialist_with_a_kit_has_exactly_these_arguments() {
        let mut given = request();
        given.kit = kit();

        let arguments = text(&turn_arguments(&given, &[], Some(&bus())).unwrap());

        let configuration = serde_json::json!({
            "mcpServers": {
                "teamwright": {
                    "command": r"C:\Program Files\Teamwright\teamwright.exe",
                    "args": ["--team-bus", "ask_manager"]
                },
                "parts": {
                    "command": "parts-server",
                    "args": ["--stdio"],
                    "env": { "PARTS_TOKEN": "${PARTS_TOKEN}" }
                },
                "tickets": {
                    "type": "http",
                    "url": "https://tickets.example/mcp",
                    "headers": { "Authorization": "Bearer ${TICKETS_TOKEN}" }
                }
            }
        });
        let configuration_at = arguments
            .iter()
            .position(|argument| argument == "--mcp-config")
            .unwrap();
        let given_configuration: serde_json::Value =
            serde_json::from_str(&arguments[configuration_at + 1]).unwrap();
        assert_eq!(given_configuration, configuration);
        let mut expected: Vec<String> = [
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "acceptEdits",
            "--permission-prompts",
            "none",
            "--restricted",
            "--setting-sources=",
            "--settings",
            r#"{"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true}"#,
            "--strict-mcp-config",
            "--plugin-dir",
            "/home/sample/.teamwright/skills/0196f3a2",
            "--mcp-config",
        ]
        .map(String::from)
        .to_vec();
        expected.push(arguments[configuration_at + 1].clone());
        expected.extend(
            [
                "--add-dir",
                "/home/sample/.teamwright/skills/0196f3a2",
                "--tools",
                "Read",
                "Glob",
                "Grep",
                "Edit",
                "Write",
                "NotebookEdit",
                "Skill",
                "ToolSearch",
                "--allowedTools",
                "Read",
                "Glob",
                "Grep",
                "Skill(pcb-review:pcb-review)",
                "mcp__teamwright__ask_manager",
                "mcp__parts__search_parts",
                "mcp__tickets__read_ticket",
                "mcp__tickets__list_tickets",
                "--disallowedTools",
                "Bash",
                "PowerShell",
                "Edit(/home/sample/.teamwright/skills/0196f3a2/**)",
            ]
            .map(String::from),
        );
        assert_eq!(arguments, expected);
    }

    #[test]
    fn no_tool_of_a_connector_is_allowed_wholesale_and_no_secret_is_an_argument() {
        let mut given = request();
        given.kit = kit();
        let arguments = text(&turn_arguments(&given, &[], None).unwrap());

        // No rule that allows a whole server, nor any tool by a pattern.
        assert!(!arguments.iter().any(|argument| {
            argument == "mcp__parts"
                || argument == "mcp__tickets"
                || (argument.starts_with("mcp__") && argument.contains('*'))
        }));
        // The secrets are named, never given: Claude Code reads them from
        // its own environment.
        assert!(arguments.join(" ").contains("${PARTS_TOKEN}"));
        // The team bus is not there without a bus; the connectors are.
        let configuration_at = arguments
            .iter()
            .position(|argument| argument == "--mcp-config")
            .unwrap();
        let configuration: serde_json::Value =
            serde_json::from_str(&arguments[configuration_at + 1]).unwrap();
        assert!(configuration["mcpServers"].get("teamwright").is_none());
        assert!(configuration["mcpServers"].get("parts").is_some());
    }

    #[test]
    fn a_connector_named_like_the_team_bus_or_given_twice_is_refused() {
        let mut given = request();
        given.kit = kit();
        given.kit.connectors[0].name = "teamwright".to_owned();
        assert!(turn_arguments(&given, &[], Some(&bus())).is_err());

        let mut twice = request();
        twice.kit = kit();
        let first = twice.kit.connectors[0].clone();
        twice.kit.connectors.push(first);
        assert!(turn_arguments(&twice, &[], None).is_err());

        let mut dashed = request();
        dashed.kit = kit();
        dashed.kit.skills[0].folder = PathBuf::from("--dangerously-skip-permissions");
        assert!(turn_arguments(&dashed, &[], None).is_err());
    }

    #[test]
    fn a_session_that_is_not_confined_is_not_given_the_settings_of_a_confined_one() {
        let mut open = request();
        open.permissions.confined_to_working_folder = false;
        let arguments = text(&turn_arguments(&open, &[], None).unwrap());
        assert!(
            !arguments
                .iter()
                .any(|argument| argument.starts_with("--setting"))
        );
    }
}
