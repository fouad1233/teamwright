//! The command lines Claude Code is started with.
//!
//! Every option used here was checked against `claude --help` of Claude
//! Code 2.1.322. The prompt is never an argument: it goes to standard input,
//! which has no length limit and needs no quoting on any system.

use std::ffi::OsString;

use teamwright_application::{
    AgentRuntimeError, Effort, PermissionMode, ProgramVersion, SessionChoice, SessionId,
    TurnRequest,
};

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
/// # Errors
///
/// Returns [`AgentRuntimeError::InvalidRequest`] if the prompt is empty, or
/// if a model or a tool rule is empty or starts with `-`, where Claude Code
/// would take it for an option.
pub fn turn_arguments(request: &TurnRequest) -> Result<Vec<OsString>, AgentRuntimeError> {
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

    // The lists come last. Each takes every following argument up to the
    // next option, and an empty list must not be given at all: `--tools`
    // with nothing after it would mean "no tools".
    if let Some(tools) = &permissions.built_in_tools {
        push_list(&mut arguments, "--tools", tools)?;
    }
    push_list(&mut arguments, "--allowedTools", &permissions.allowed_tools)?;
    push_list(
        &mut arguments,
        "--disallowedTools",
        &permissions.disallowed_tools,
    )?;

    Ok(arguments)
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

    use teamwright_application::PermissionProfile;

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
        }
    }

    fn text(arguments: &[OsString]) -> Vec<String> {
        arguments
            .iter()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn a_specialists_new_turn_has_exactly_these_arguments() {
        let arguments = text(&turn_arguments(&request()).unwrap());
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

    #[test]
    fn the_prompt_is_never_an_argument() {
        let arguments = text(&turn_arguments(&request()).unwrap());
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
        let arguments = text(&turn_arguments(&resumed).unwrap());
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--resume", id.as_str()])
        );

        let mut chosen = request();
        chosen.session = SessionChoice::NewWithId(id.clone());
        let arguments = text(&turn_arguments(&chosen).unwrap());
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
        let arguments = text(&turn_arguments(&detailed).unwrap());
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
        };
        let arguments = text(&turn_arguments(&open).unwrap());
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
        assert!(turn_arguments(&sneaky).is_err());

        let mut sneaky = request();
        sneaky.model = Some("-x".to_owned());
        assert!(turn_arguments(&sneaky).is_err());

        let mut empty = request();
        empty.prompt = "  ".to_owned();
        assert!(turn_arguments(&empty).is_err());
    }

    #[test]
    fn the_desktop_app_is_asked_to_resume_the_session() {
        let id = SessionId::parse("abc").unwrap();
        assert_eq!(
            text(&desktop_arguments(&id)),
            ["--desktop", "--resume", "abc"]
        );
    }
}
