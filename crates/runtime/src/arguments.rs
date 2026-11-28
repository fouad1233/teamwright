//! The command lines Claude Code is started with.
//!
//! Every option used here was checked against `claude --help` of Claude
//! Code 2.1.322. The prompt is never an argument: it goes to standard input,
//! which has no length limit and needs no quoting on any system.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use teamwright_application::{
    AgentRuntimeError, Effort, PermissionMode, PermissionProfile, ProgramVersion, ProtectedPlace,
    SessionChoice, SessionId, TurnRequest,
};

use crate::kit::{ConnectorStarts, SESSION_SETTINGS, connector_entry, skill_rule};
use crate::team_bus::{SERVER_NAME, TeamBus, name_in_session};

/// The settings of a turn that may run commands (decision 0038): reads
/// outside the working folders are refused also for commands, and every
/// command runs in the sandbox of Claude Code, which fails closed.
///
/// Each key was read in the documentation at <https://code.claude.com/docs>
/// (`settings-reference`, `sandboxing`) and found in the installed program,
/// Claude Code 2.1.289, on 21 November 2026. None of it has been seen
/// working in a run: the program was not signed in.
///
/// - `permissions.blockReadsOutsideWorkingDirectories` (2.1.257): with the
///   sandbox on, it also denies sandboxed commands the home folder and the
///   other roots of user files (`/home`, `/Users`, `/mnt`, ...), and opens
///   again only the working folders. Teamwright's data and the sync folder
///   are outside them.
/// - `sandbox.enabled`: commands run in the sandbox (bubblewrap on Linux and
///   WSL2, Seatbelt on macOS).
/// - `sandbox.failIfUnavailable`: if the sandbox cannot start, the program
///   does not start, instead of running commands without it.
/// - `sandbox.autoAllowBashIfSandboxed`: false. It defaults to true before
///   2.1.324, which would run every sandboxed command without a rule; false
///   keeps the allowed rules the only commands that run.
/// - `sandbox.allowUnsandboxedCommands`: false, so a command refused by the
///   sandbox is never tried again outside it.
/// - `sandbox.network.strictAllowlist`: a host that is not allowed is denied
///   at once instead of asked for. The allowed hosts are the web domains
///   the manager allowed, which the program adds itself; with none, a
///   command reaches no network.
/// - `sandbox.filesystem.denyWrite`: the folders the session may only read.
///   Without it, a command could write in every folder given with
///   `--add-dir`.
fn command_settings(read_only_folders: &[String]) -> serde_json::Value {
    serde_json::json!({
        "permissions": { "blockReadsOutsideWorkingDirectories": true },
        "sandbox": {
            "enabled": true,
            "failIfUnavailable": true,
            "autoAllowBashIfSandboxed": false,
            "allowUnsandboxedCommands": false,
            "network": { "strictAllowlist": true },
            "filesystem": { "denyWrite": read_only_folders },
        },
    })
}

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

/// The arguments that run one turn of a session in headless mode, without
/// a launcher for connectors: what the tests of the other arguments use.
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
#[cfg(test)]
pub fn turn_arguments(
    request: &TurnRequest,
    readable_folders: &[PathBuf],
    team_bus: Option<&TeamBus>,
) -> Result<Vec<OsString>, AgentRuntimeError> {
    turn_arguments_with(
        request,
        readable_folders,
        team_bus,
        &ConnectorStarts::default(),
    )
}

/// The arguments of a turn, as [`turn_arguments`], with how the connectors
/// that are programs are started: through the launcher, in an empty folder
/// of Teamwright's own (decision 0039, note of 21 November 2026). Without a
/// launcher, a turn given such a connector is refused.
///
/// # Errors
///
/// As [`turn_arguments`], and if a connector cannot be started safely.
pub fn turn_arguments_with(
    request: &TurnRequest,
    readable_folders: &[PathBuf],
    team_bus: Option<&TeamBus>,
    connector_starts: &ConnectorStarts<'_>,
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
    // may run commands, the block on reads outside the folders and the
    // sandbox (decision 0038). One argument, so that neither replaces the
    // other.
    let read_only_folders: Vec<String> = readable_folders
        .iter()
        .chain(request.kit.skills.iter().map(|skill| &skill.folder))
        .map(|folder| {
            folder
                .to_string_lossy()
                .replace('\\', "/")
                .trim_end_matches('/')
                .to_owned()
        })
        .collect();
    if let Some(settings) = settings_of(permissions, &read_only_folders) {
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
        let (entry, tools) = connector_entry(connector, connector_starts)?;
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
        disallowed_tools.push(format!("Edit({})", write_rule_path(&folder)));
    }
    // A folder to write in is a working folder like the first one: read and
    // written, with no rule against writing (decision 0044).
    for folder in &request.written_folders {
        let folder = folder.to_string_lossy();
        check_value("folder", &folder)?;
        push_pair(&mut arguments, "--add-dir", &folder);
    }
    // Full access: the root of every file system is a working folder, so
    // that the file tools reach wherever the user's account can. Restricted
    // mode stays on, and with it everything else it shuts out.
    if request.full_access {
        if permissions.commands_in_sandbox {
            return Err(invalid(
                "a turn with full access runs no command: its sandbox would reach the whole disk",
            ));
        }
        for root in roots_of_this_computer() {
            push_pair(&mut arguments, "--add-dir", &root.to_string_lossy());
        }
    }
    // The places no session may read or change: every one of them with
    // full access, and without it those that lie inside a folder the
    // session was given.
    let given: Vec<&Path> = std::iter::once(request.working_folder.as_path())
        .chain(readable_folders.iter().map(PathBuf::as_path))
        .chain(request.written_folders.iter().map(PathBuf::as_path))
        .collect();
    for place in &request.protected_places {
        if request.full_access || given.iter().any(|folder| place.path.starts_with(folder)) {
            disallowed_tools.extend(protected_rules(place));
        }
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

/// The roots of the file systems of this computer: `/` on Linux and macOS,
/// and on Windows every drive from `A:` to `Z:` that exists. A session with
/// full access is given each of them as a working folder.
fn roots_of_this_computer() -> Vec<PathBuf> {
    if cfg!(windows) {
        (b'A'..=b'Z')
            .map(|letter| PathBuf::from(format!("{}:\\", char::from(letter))))
            .filter(|drive| drive.is_dir())
            .collect()
    } else {
        vec![PathBuf::from("/")]
    }
}

/// The rules that keep a session out of a protected place: one against
/// reading it and one against changing it. The rule against reading also
/// stops the tools that write files, but not `NotebookEdit`, so the one
/// against changing is always given too.
fn protected_rules(place: &ProtectedPlace) -> [String; 2] {
    let path = rule_path(&place.path.to_string_lossy(), place.is_folder);
    [format!("Read({path})"), format!("Edit({path})")]
}

/// The path of a rule that forbids writing in a folder and everything
/// below it, as the rule writes it: `<folder>/**`.
///
/// Read and Edit rules use the syntax of gitignore, and the documentation
/// (<https://code.claude.com/docs/en/permissions>, "How path patterns
/// match") gives four kinds of start: `//path` is absolute from the root of
/// the file system, `~/path` from the home folder, `/path` from the place of
/// the settings (for a session started by Teamwright: its working folder),
/// and `path` from the current folder. So an absolute folder of a Unix
/// system, `/a/b`, is written `//a/b/**`. With one slash the rule forbids
/// writing in `<working folder>/a/b` and leaves the folder itself open: on
/// Linux a session wrote a file into a read-only folder that way.
///
/// A folder of Windows, which starts with a drive letter or two backslashes,
/// is written with forward slashes and no change at the start
/// (`C:/Users/sample/Documents/**`). That form was seen working on Windows
/// in a real run; the documentation names `//c/Users/...` for the same
/// folder. Windows has not been run again since.
///
/// Letters, digits and the marks `/ # ! - _ . :` are written as they are.
/// A star or a square bracket gets a backslash and a backslash gets four,
/// which a real run showed to work. Every other character, among them
/// spaces, commas, parentheses and the question mark, is written as `?`.
fn write_rule_path(folder: &str) -> String {
    rule_path(folder, true)
}

/// The path of a rule about a place: a folder with everything below it
/// (`<folder>/**`), or a single file (`<file>`), written as
/// [`write_rule_path`] says.
fn rule_path(folder: &str, is_folder: bool) -> String {
    let mut letters = folder.chars();
    let drive = matches!(
        (letters.next(), letters.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    );
    let windows_form = drive || folder.starts_with(r"\\");
    let slashed = if windows_form {
        folder.replace('\\', "/")
    } else {
        folder.to_owned()
    };
    let start = if windows_form { "" } else { "/" };
    let path = format!("{start}{}", slashed.trim_end_matches('/'));
    // Anything that a pattern, the parentheses of a rule or the list of
    // rules (split at spaces and commas) could read as more than itself
    // becomes `?`, which matches any one character. So the rule can only
    // forbid more than the folder, never less.
    let mut rule = String::new();
    for letter in path.chars() {
        match letter {
            '\\' => rule.push_str("\\\\\\\\"),
            '*' | '[' | ']' => {
                rule.push('\\');
                rule.push(letter);
            }
            '/' | '#' | '!' | '-' | '_' | '.' | ':' => {
                rule.push(letter);
            }
            letter if letter.is_alphanumeric() => rule.push(letter),
            _ => rule.push('?'),
        }
    }
    if is_folder {
        rule.push_str("/**");
    }
    rule
}

/// The settings a session is started with, as one JSON object, or nothing.
/// `read_only_folders` are the folders the session may read and not
/// write, in the form a sandbox path takes.
fn settings_of(permissions: &PermissionProfile, read_only_folders: &[String]) -> Option<String> {
    let mut extra = serde_json::Map::new();
    if permissions.commands_in_sandbox {
        if let serde_json::Value::Object(keys) = command_settings(read_only_folders) {
            extra = keys;
        }
    } else if permissions.reads_blocked_outside_working_folders {
        extra.insert(
            "permissions".to_owned(),
            serde_json::json!({ "blockReadsOutsideWorkingDirectories": true }),
        );
    }
    let extra = (!extra.is_empty()).then(|| serde_json::Value::Object(extra).to_string());
    match (permissions.confined_to_working_folder, extra) {
        (false, None) => None,
        (true, None) => Some(SESSION_SETTINGS.to_owned()),
        (false, Some(extra)) => Some(extra),
        // Both are objects of JSON whose last character closes them, and
        // their keys differ, so the second's keys join the first's.
        (true, Some(extra)) => Some(format!(
            "{},{}",
            SESSION_SETTINGS.trim_end_matches('}'),
            extra.trim_start_matches('{')
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
            written_folders: Vec::new(),
            full_access: false,
            protected_places: Vec::new(),
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
                tasks: Vec::new(),
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
                "Edit(//home/sample/shared/**)",
            ]
        );
    }

    #[test]
    fn a_rule_for_a_folder_of_unix_starts_with_two_slashes() {
        // One slash would mean "from the working folder of the session".
        assert_eq!(write_rule_path("/a/b"), "//a/b/**");
        assert_eq!(write_rule_path("/a/b/"), "//a/b/**");
        assert_eq!(
            write_rule_path("/home/sample/.teamwright/skills/0196f3a2"),
            "//home/sample/.teamwright/skills/0196f3a2/**"
        );
    }

    #[test]
    fn a_rule_for_a_folder_of_windows_keeps_its_drive_and_uses_forward_slashes() {
        assert_eq!(
            write_rule_path(r"C:\Users\sample\Documents\Teamwright\SSN"),
            "C:/Users/sample/Documents/Teamwright/SSN/**"
        );
        assert_eq!(write_rule_path(r"D:\a b\"), "D:/a?b/**");
        assert_eq!(
            write_rule_path(r"\\server\share\data"),
            "//server/share/data/**"
        );
    }

    #[test]
    fn a_rule_never_holds_what_a_pattern_or_the_list_of_rules_could_misread() {
        // Seen in real runs: each of these folders stayed read-only.
        let rule = |name: &str| write_rule_path(&format!("/t/{name}"));
        assert_eq!(rule("data sheets (old)"), "//t/data?sheets??old?/**");
        assert_eq!(rule("paren (a) b"), "//t/paren??a??b/**");
        assert_eq!(rule("comma, here"), "//t/comma??here/**");
        assert_eq!(rule("star *"), r"//t/star?\*/**");
        assert_eq!(rule("bracket [1]"), r"//t/bracket?\[1\]/**");
        assert_eq!(rule(r"back\slash"), r"//t/back\\\\slash/**");
        assert_eq!(rule("hash #1"), "//t/hash?#1/**");
        assert_eq!(rule("bang !x"), "//t/bang?!x/**");
        assert_eq!(rule("quest ?"), "//t/quest??/**");
        assert_eq!(rule("tab\tand trailing "), "//t/tab?and?trailing?/**");
        // No space, comma or parenthesis survives, whatever the name.
        let all = rule("a (b) , {c} | $ ^ + ' \"");
        assert!(!all.contains([' ', ',', '(', ')', '{', '|', '$', '^', '+', '\'', '"']));
        // The folder's own name keeps its letters, also outside ASCII.
        assert_eq!(rule("daten blätter"), "//t/daten?blätter/**");
    }

    #[test]
    fn a_folder_with_parentheses_is_one_rule_in_the_command_line() {
        let folders = [PathBuf::from("/tmp/data sheets (old)")];
        let arguments = text(&turn_arguments(&request(), &folders, None).unwrap());
        assert!(arguments.contains(&"Edit(//tmp/data?sheets??old?/**)".to_owned()));
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
                r#"{"disableAllHooks":true,"disableSkillShellExecution":true,"disableClaudeAiConnectors":true,"permissions":{"blockReadsOutsideWorkingDirectories":true},"sandbox":{"allowUnsandboxedCommands":false,"autoAllowBashIfSandboxed":false,"enabled":true,"failIfUnavailable":true,"filesystem":{"denyWrite":[]},"network":{"strictAllowlist":true}}}"#,
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
                "Bash(sed *)",
            ]
        );
    }

    #[test]
    fn the_settings_of_a_session_are_one_json_object_with_every_key() {
        let mut profile = PermissionProfile::specialist();
        profile.reads_blocked_outside_working_folders = true;
        profile.commands_in_sandbox = true;
        let folders = ["/home/sample/datasheets".to_owned()];
        let settings: serde_json::Value =
            serde_json::from_str(&settings_of(&profile, &folders).unwrap()).unwrap();
        assert_eq!(
            settings,
            serde_json::json!({
                "disableAllHooks": true,
                "disableSkillShellExecution": true,
                "disableClaudeAiConnectors": true,
                "permissions": { "blockReadsOutsideWorkingDirectories": true },
                "sandbox": {
                    "enabled": true,
                    "failIfUnavailable": true,
                    "autoAllowBashIfSandboxed": false,
                    "allowUnsandboxedCommands": false,
                    "network": { "strictAllowlist": true },
                    "filesystem": { "denyWrite": ["/home/sample/datasheets"] }
                }
            })
        );
        profile.confined_to_working_folder = false;
        let alone: serde_json::Value =
            serde_json::from_str(&settings_of(&profile, &folders).unwrap()).unwrap();
        assert_eq!(alone["sandbox"]["failIfUnavailable"], true);
        assert_eq!(alone.get("disableAllHooks"), None);
        profile.commands_in_sandbox = false;
        assert_eq!(
            settings_of(&profile, &folders).as_deref(),
            Some(r#"{"permissions":{"blockReadsOutsideWorkingDirectories":true}}"#)
        );
        profile.reads_blocked_outside_working_folders = false;
        assert_eq!(settings_of(&profile, &folders), None);
    }

    #[test]
    fn a_turn_that_may_run_a_command_runs_it_in_a_sandbox_that_fails_closed() {
        let mut allowed = granted(&[(PermissionKind::Command, "npm test")]);
        allowed.kit.skills = kit().skills;
        let arguments = text(
            &turn_arguments(&allowed, &[PathBuf::from("/home/sample/datasheets/")], None).unwrap(),
        );
        let settings: Vec<&String> = arguments
            .windows(2)
            .filter(|pair| pair[0] == "--settings")
            .map(|pair| &pair[1])
            .collect();
        let [settings] = settings.as_slice() else {
            panic!("one --settings, never two: {arguments:?}");
        };
        let settings: serde_json::Value = serde_json::from_str(settings).unwrap();
        let sandbox = &settings["sandbox"];
        assert_eq!(sandbox["enabled"], true);
        assert_eq!(sandbox["failIfUnavailable"], true);
        // Its default would approve every sandboxed command.
        assert_eq!(sandbox["autoAllowBashIfSandboxed"], false);
        assert_eq!(sandbox["allowUnsandboxedCommands"], false);
        assert_eq!(sandbox["network"]["strictAllowlist"], true);
        assert_eq!(sandbox.get("excludedCommands"), None);
        let skill_folder = allowed.kit.skills[0]
            .folder
            .to_string_lossy()
            .trim_end_matches('/')
            .to_owned();
        assert_eq!(
            sandbox["filesystem"]["denyWrite"],
            serde_json::json!(["/home/sample/datasheets", skill_folder])
        );
        assert_eq!(
            settings["permissions"]["blockReadsOutsideWorkingDirectories"],
            true
        );
    }

    #[test]
    fn a_turn_allowed_a_command_and_a_domain_gets_both_rules_and_both_tools() {
        let arguments = text(
            &turn_arguments(
                &granted(&[
                    (PermissionKind::Command, "cargo test"),
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
                "Bash(cargo test)",
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
            vec![(PermissionKind::Command, "npm run build")],
            vec![(PermissionKind::WebDomain, "api.example.com")],
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
            commands_in_sandbox: false,
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
            withheld_variables: Vec::new(),
        }
    }

    fn launcher() -> crate::kit::ConnectorLauncher {
        crate::kit::ConnectorLauncher {
            program: PathBuf::from(r"C:\Program Files\Teamwright\teamwright.exe"),
            arguments: vec!["--connector".to_owned()],
            folder: PathBuf::from(r"C:\Users\sample\AppData\Teamwright\connectors\starts"),
            secrets_folder: PathBuf::from(r"C:\Users\sample\AppData\Teamwright\connectors\secrets"),
        }
    }

    /// The arguments of a turn whose connectors are started through the
    /// launcher, with the file of secrets of the connector `parts`.
    fn kit_arguments(
        request: &TurnRequest,
        team_bus: Option<&TeamBus>,
    ) -> Result<Vec<OsString>, AgentRuntimeError> {
        let launcher = launcher();
        let starts = ConnectorStarts {
            launcher: Some(&launcher),
            secrets: std::collections::BTreeMap::from([(
                "parts".to_owned(),
                PathBuf::from(r"C:\Users\sample\AppData\Teamwright\connectors\secrets\t.json"),
            )]),
            folders: std::collections::BTreeMap::from([(
                "parts".to_owned(),
                PathBuf::from(r"C:\Users\sample\AppData\Teamwright\connectors\starts\s1"),
            )]),
        };
        turn_arguments_with(request, &[], team_bus, &starts)
    }

    #[test]
    fn a_specialist_with_a_kit_has_exactly_these_arguments() {
        let mut given = request();
        given.kit = kit();

        let arguments = text(&kit_arguments(&given, Some(&bus())).unwrap());

        let configuration = serde_json::json!({
            "mcpServers": {
                "teamwright": {
                    "command": r"C:\Program Files\Teamwright\teamwright.exe",
                    "args": ["--team-bus", "ask_manager"]
                },
                "parts": {
                    "command": r"C:\Program Files\Teamwright\teamwright.exe",
                    "args": [
                        "--connector",
                        "--folder",
                        r"C:\Users\sample\AppData\Teamwright\connectors\starts\s1",
                        "--secrets",
                        r"C:\Users\sample\AppData\Teamwright\connectors\secrets\t.json",
                        "--pass",
                        "PARTS_TOKEN",
                        "--",
                        "parts-server",
                        "--stdio"
                    ]
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
                "Edit(//home/sample/.teamwright/skills/0196f3a2/**)",
            ]
            .map(String::from),
        );
        assert_eq!(arguments, expected);
    }

    #[test]
    fn no_tool_of_a_connector_is_allowed_wholesale_and_no_secret_is_an_argument() {
        let mut given = request();
        given.kit = kit();
        let arguments = text(&kit_arguments(&given, None).unwrap());

        // No rule that allows a whole server, nor any tool by a pattern.
        assert!(!arguments.iter().any(|argument| {
            argument == "mcp__parts"
                || argument == "mcp__tickets"
                || (argument.starts_with("mcp__") && argument.contains('*'))
        }));
        // The secrets are named, never given. A program is handed them by
        // the launcher, from a file; an address in a header, by Claude Code
        // from its own environment.
        assert!(!arguments.join(" ").contains("${PARTS_TOKEN}"));
        assert!(arguments.join(" ").contains("${TICKETS_TOKEN}"));
        // Without the launcher, a program connector is not started at all.
        assert!(turn_arguments(&given, &[], None).is_err());
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
        assert!(kit_arguments(&given, Some(&bus())).is_err());

        let mut twice = request();
        twice.kit = kit();
        let first = twice.kit.connectors[0].clone();
        twice.kit.connectors.push(first);
        assert!(kit_arguments(&twice, None).is_err());

        let mut dashed = request();
        dashed.kit = kit();
        dashed.kit.skills[0].folder = PathBuf::from("--dangerously-skip-permissions");
        assert!(kit_arguments(&dashed, None).is_err());
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

    /// The protected places of a computer of Linux, as the desktop names
    /// them (decision 0044).
    fn protected() -> Vec<ProtectedPlace> {
        use teamwright_application::ProtectedKind;
        vec![
            ProtectedPlace::folder(
                "/home/sample/.local/share/dev.teamwright.app",
                ProtectedKind::TeamwrightData,
            ),
            ProtectedPlace::folder(
                "/home/sample/Dropbox/Teamwright sync",
                ProtectedKind::SyncFolder,
            ),
            ProtectedPlace::folder("/home/sample/.claude", ProtectedKind::AgentProgramSettings),
            ProtectedPlace::file(
                "/home/sample/.claude.json",
                ProtectedKind::AgentProgramSettings,
            ),
            ProtectedPlace::folder("/home/sample/.ssh", ProtectedKind::Keys),
            ProtectedPlace::folder("/home/sample/.gnupg", ProtectedKind::Keys),
        ]
    }

    #[cfg(unix)]
    #[test]
    fn a_turn_with_full_access_has_exactly_these_arguments() {
        let mut open = request();
        open.working_folder = PathBuf::from("/home/sample/Documents/Teamwright/SSN");
        open.full_access = true;
        open.protected_places = protected();

        let arguments = text(&turn_arguments(&open, &[], None).unwrap());

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
                "--add-dir",
                "/",
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
                "Read(//home/sample/.local/share/dev.teamwright.app/**)",
                "Edit(//home/sample/.local/share/dev.teamwright.app/**)",
                "Read(//home/sample/Dropbox/Teamwright?sync/**)",
                "Edit(//home/sample/Dropbox/Teamwright?sync/**)",
                "Read(//home/sample/.claude/**)",
                "Edit(//home/sample/.claude/**)",
                "Read(//home/sample/.claude.json)",
                "Edit(//home/sample/.claude.json)",
                "Read(//home/sample/.ssh/**)",
                "Edit(//home/sample/.ssh/**)",
                "Read(//home/sample/.gnupg/**)",
                "Edit(//home/sample/.gnupg/**)",
            ]
        );
    }

    #[test]
    fn full_access_keeps_everything_else_that_confines_a_session() {
        let mut open = request();
        open.full_access = true;
        open.protected_places = protected();
        let arguments = text(&turn_arguments(&open, &[], Some(&bus())).unwrap());

        for kept in ["--restricted", "--setting-sources=", "--strict-mcp-config"] {
            assert!(arguments.contains(&kept.to_owned()), "{kept}");
        }
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--settings", SESSION_SETTINGS])
        );
        for never in [
            "--dangerously-skip-permissions",
            "--allow-dangerously-skip-permissions",
            "bypassPermissions",
            "auto",
            "--permission-prompt-tool",
        ] {
            assert!(!arguments.contains(&never.to_owned()), "{never}");
        }
        // The shell stays out, and the team bus stays in.
        let tools_at = arguments.iter().position(|a| a == "--tools").unwrap();
        let allowed_at = arguments
            .iter()
            .position(|a| a == "--allowedTools")
            .unwrap();
        assert!(!arguments[tools_at..allowed_at].contains(&"Bash".to_owned()));
        assert!(arguments.contains(&"mcp__teamwright__ask_manager".to_owned()));
    }

    #[test]
    fn a_turn_with_full_access_and_a_command_is_refused() {
        let mut both = granted(&[(PermissionKind::Command, "npm test")]);
        both.full_access = true;
        assert!(turn_arguments(&both, &[], None).is_err());
        // A web domain is no command, and may go with full access.
        let mut domain = granted(&[(PermissionKind::WebDomain, "docs.rs")]);
        domain.full_access = true;
        assert!(turn_arguments(&domain, &[], None).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_to_write_in_is_added_and_never_denied() {
        let mut given = request();
        given.written_folders = vec![PathBuf::from("/home/sample/firmware (rev b)")];
        given.protected_places = protected();

        let arguments = text(
            &turn_arguments(&given, &[PathBuf::from("/home/sample/datasheets")], None).unwrap(),
        );

        let added: Vec<&String> = arguments
            .windows(2)
            .filter(|pair| pair[0] == "--add-dir")
            .map(|pair| &pair[1])
            .collect();
        assert_eq!(
            added,
            ["/home/sample/datasheets", "/home/sample/firmware (rev b)"]
        );
        let denied_at = arguments
            .iter()
            .position(|a| a == "--disallowedTools")
            .unwrap();
        // The reference folder is denied for writing; the folder to write
        // in is not; no protected place lies inside a folder of the turn,
        // so none is named; there is no root.
        assert_eq!(
            arguments[denied_at + 1..],
            ["Bash", "PowerShell", "Edit(//home/sample/datasheets/**)"]
        );
        assert!(!arguments.contains(&"/".to_owned()));
    }

    #[cfg(unix)]
    #[test]
    fn a_protected_place_inside_a_folder_of_the_turn_is_shut_also_without_full_access() {
        // The manager chose the home folder as the work folder: the keys
        // and the settings of Claude Code inside it stay closed.
        let mut home = request();
        home.working_folder = PathBuf::from("/home/sample");
        home.protected_places = protected();

        let arguments = text(&turn_arguments(&home, &[], None).unwrap());

        let denied_at = arguments
            .iter()
            .position(|a| a == "--disallowedTools")
            .unwrap();
        let denied = &arguments[denied_at + 1..];
        assert!(denied.contains(&"Read(//home/sample/.ssh/**)".to_owned()));
        assert!(denied.contains(&"Edit(//home/sample/.claude/**)".to_owned()));
        assert!(denied.contains(&"Read(//home/sample/.claude.json)".to_owned()));
        assert!(!arguments.contains(&"/".to_owned()));
        // A turn in a folder apart from them names none of them.
        let apart = text(&turn_arguments(&request(), &[], None).unwrap());
        assert!(!apart.iter().any(|argument| argument.contains(".ssh")));
    }

    #[test]
    fn a_rule_for_a_single_file_names_the_file_alone() {
        assert_eq!(
            rule_path("/home/sample/.claude.json", false),
            "//home/sample/.claude.json"
        );
        assert_eq!(
            rule_path("/home/sample/.claude", true),
            "//home/sample/.claude/**"
        );
        assert_eq!(
            rule_path(r"C:\Users\sample\.claude.json", false),
            "C:/Users/sample/.claude.json"
        );
    }
}
