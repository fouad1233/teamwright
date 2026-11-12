//! Tests of the process handling, against a stand-in for Claude Code.
//!
//! The stand-in is this test program itself. Started with the variable
//! `TEAMWRIGHT_STAND_IN` set, it plays Claude Code instead of running the
//! tests: it answers the questions for the version and the sign-in, and
//! plays a turn in headless mode. Because this file has its own `main`
//! instead of the usual test harness, nothing else is written to standard
//! output while it plays.
//!
//! The build server has no Claude Code, so these tests are what proves that
//! processes are started, fed, read and stopped correctly.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use std::io::{Read, Write};
use std::panic;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use teamwright_application::{
    AgentEvent, AgentRuntime, AgentRuntimeError, PermissionProfile, ProgramVersion, SessionChoice,
    SessionId, SignIn, TurnFailure, TurnOutcome, TurnRequest,
};
use teamwright_runtime::{ClaudeCode, Environment};

/// Set to the part the stand-in plays.
const ROLE: &str = "TEAMWRIGHT_STAND_IN";
/// Set to the version the stand-in reports, if not the usual one.
const VERSION: &str = "TEAMWRIGHT_STAND_IN_VERSION";
/// A variable the tests give the stand-in, to see that it arrives.
const PROBE: &str = "TEAMWRIGHT_PROBE";

fn main() {
    if let Ok(role) = std::env::var(ROLE) {
        std::process::exit(play_stand_in(&role));
    }

    let tests: &[(&str, fn())] = &[
        (
            "the_installation_has_a_place_and_a_version",
            the_installation_has_a_place_and_a_version,
        ),
        (
            "the_sign_in_state_comes_from_the_program",
            the_sign_in_state_comes_from_the_program,
        ),
        (
            "a_missing_program_is_reported_as_not_installed",
            a_missing_program_is_reported_as_not_installed,
        ),
        (
            "a_turn_reports_its_events_and_ends_last",
            a_turn_reports_its_events_and_ends_last,
        ),
        (
            "the_program_gets_the_prompt_the_folder_and_only_the_given_environment",
            the_program_gets_the_prompt_the_folder_and_only_the_given_environment,
        ),
        ("a_long_prompt_arrives_whole", a_long_prompt_arrives_whole),
        (
            "a_resumed_turn_names_its_session",
            a_resumed_turn_names_its_session,
        ),
        (
            "a_running_turn_can_be_stopped",
            a_running_turn_can_be_stopped,
        ),
        (
            "a_program_that_ends_early_is_reported_with_its_errors",
            a_program_that_ends_early_is_reported_with_its_errors,
        ),
        (
            "a_turn_in_a_missing_folder_is_refused",
            a_turn_in_a_missing_folder_is_refused,
        ),
        (
            "a_session_is_opened_in_the_desktop_app",
            a_session_is_opened_in_the_desktop_app,
        ),
        (
            "an_old_version_cannot_open_the_desktop_app",
            an_old_version_cannot_open_the_desktop_app,
        ),
        (
            "a_failure_to_open_the_desktop_app_is_reported",
            a_failure_to_open_the_desktop_app_is_reported,
        ),
    ];

    let mut failed = 0;
    for (name, test) in tests {
        let started = Instant::now();
        let result = panic::catch_unwind(test);
        let verdict = if result.is_ok() { "ok" } else { "FAILED" };
        println!(
            "test {name} ... {verdict} ({} ms)",
            started.elapsed().as_millis()
        );
        if result.is_err() {
            failed += 1;
        }
    }
    println!(
        "\ntest result: {}. {} passed; {failed} failed",
        if failed == 0 { "ok" } else { "FAILED" },
        tests.len() - failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// The stand-in
// ---------------------------------------------------------------------------

fn play_stand_in(role: &str) -> i32 {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| arguments.iter().any(|argument| argument == flag);

    if has("--version") {
        let version = std::env::var(VERSION).unwrap_or_else(|_| "2.1.322".to_owned());
        println!("{version} (Claude Code)");
        return 0;
    }
    if arguments.first().map(String::as_str) == Some("auth") {
        let signed_in = role != "signed-out";
        println!(
            "{}",
            json!({
                "loggedIn": signed_in,
                "authMethod": if signed_in { "claude.ai" } else { "none" },
                "apiProvider": "firstParty",
                "email": "sample@example.com",
                "subscriptionType": "pro",
            })
        );
        return if signed_in { 0 } else { 1 };
    }
    if has("--desktop") {
        if role == "desktop-fails" {
            eprintln!("The Claude desktop app is not installed");
            return 2;
        }
        return 0;
    }
    if has("-p") {
        return play_turn(role, &arguments);
    }
    eprintln!("the stand-in does not know these arguments: {arguments:?}");
    64
}

fn play_turn(role: &str, arguments: &[String]) -> i32 {
    let mut prompt = String::new();
    std::io::stdin().read_to_string(&mut prompt).unwrap();

    let value_after = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|index| arguments.get(index + 1))
            .cloned()
    };
    let session = value_after("--resume")
        .or_else(|| value_after("--session-id"))
        .unwrap_or_else(|| "stand-in-session".to_owned());
    let folder = std::env::current_dir().unwrap();

    let mut out = std::io::stdout().lock();
    let mut say = |event: Value| {
        writeln!(out, "{event}").unwrap();
        out.flush().unwrap();
    };
    say(json!({
        "type": "system", "subtype": "init", "session_id": session,
        "cwd": folder, "model": "claude-stand-in", "permissionMode": "acceptEdits",
        "tools": ["Read", "Write"], "skills": [], "plugins": [], "mcp_servers": [],
    }));

    match role {
        "hang" => {
            std::thread::sleep(Duration::from_secs(60));
            0
        }
        "crash" => {
            eprintln!("first problem");
            eprintln!("second problem");
            3
        }
        _ => {
            say(
                json!({"type": "assistant", "session_id": session, "message": {"content": [
                    {"type": "text", "text": "Working on it."},
                    {"type": "tool_use", "id": "toolu_1", "name": "Write",
                     "input": {"file_path": folder.join("notes.txt"), "content": "alpha"}},
                ]}}),
            );
            println!("a line that is not JSON");
            say(
                json!({"type": "user", "session_id": session, "message": {"content": [
                    {"type": "tool_result", "tool_use_id": "toolu_1", "content": "done"},
                ]}}),
            );
            say(json!({"type": "something_from_a_later_version", "session_id": session}));
            let report = json!({
                "prompt_length": prompt.chars().count(),
                "prompt_start": prompt.chars().take(40).collect::<String>(),
                "arguments": arguments,
                "folder": folder,
                "probe": std::env::var(PROBE).ok(),
                "path_given": std::env::var_os("PATH").is_some(),
            });
            say(json!({
                "type": "result", "subtype": "success", "is_error": false,
                "session_id": session, "result": report.to_string(), "num_turns": 2,
                "duration_ms": 5, "permission_denials": [],
                "model_usage_since_last_result": {"claude-stand-in": {
                    "inputTokens": 1, "outputTokens": 2, "cacheReadInputTokens": 3,
                    "cacheCreationInputTokens": 4, "costUSD": 0.5,
                }},
            }));
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn stand_in(role: &str) -> ClaudeCode {
    ClaudeCode::new()
        .with_program(std::env::current_exe().unwrap())
        .with_environment(Environment::empty().with(ROLE, role).with(PROBE, "given"))
}

fn request(folder: PathBuf, prompt: &str) -> TurnRequest {
    TurnRequest {
        working_folder: folder,
        prompt: prompt.to_owned(),
        session: SessionChoice::New,
        model: Some("haiku".to_owned()),
        effort: None,
        extra_instructions: None,
        permissions: PermissionProfile::specialist(),
        may_ask_manager: false,
        readable_folders: Vec::new(),
    }
}

/// Runs a turn to its end and returns its events and its outcome.
fn run(runtime: &ClaudeCode, request: TurnRequest) -> (Vec<AgentEvent>, TurnOutcome) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);
    let turn = runtime
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    (events, outcome)
}

fn report(outcome: &TurnOutcome) -> Value {
    serde_json::from_str(&outcome.final_text).unwrap()
}

// ---------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------

fn the_installation_has_a_place_and_a_version() {
    let installation = stand_in("turn").installation().unwrap();
    assert_eq!(installation.program, std::env::current_exe().unwrap());
    assert_eq!(installation.version, ProgramVersion::new(2, 1, 322));
}

fn the_sign_in_state_comes_from_the_program() {
    assert_eq!(
        stand_in("signed-in").sign_in().unwrap(),
        SignIn::SignedIn {
            method: "claude.ai".to_owned(),
            provider: "firstParty".to_owned(),
            subscription: Some("pro".to_owned()),
        }
    );
    assert_eq!(stand_in("signed-out").sign_in().unwrap(), SignIn::SignedOut);
}

fn a_missing_program_is_reported_as_not_installed() {
    let folder = tempfile::tempdir().unwrap();
    let nowhere = ClaudeCode::new().with_environment(
        Environment::empty()
            .with("PATH", folder.path())
            .with("HOME", folder.path())
            .with("USERPROFILE", folder.path()),
    );
    // Outside Windows a few system folders are searched as well; on a
    // computer with Claude Code installed there this test says nothing.
    if cfg!(windows) {
        assert_eq!(
            nowhere.installation().unwrap_err(),
            AgentRuntimeError::NotInstalled
        );
    }
    let missing = ClaudeCode::new().with_program(folder.path().join("claude"));
    assert_eq!(
        missing.sign_in().unwrap_err(),
        AgentRuntimeError::NotInstalled
    );
}

fn a_turn_reports_its_events_and_ends_last() {
    let folder = tempfile::tempdir().unwrap();
    let (events, outcome) = run(
        &stand_in("turn"),
        request(folder.path().to_owned(), "Hello"),
    );

    assert!(matches!(&events[0], AgentEvent::SessionStarted(started)
        if started.session_id.as_str() == "stand-in-session" && started.model == "claude-stand-in"));
    assert_eq!(
        events[1],
        AgentEvent::Text {
            text: "Working on it.".to_owned()
        }
    );
    assert_eq!(
        events[2],
        AgentEvent::ToolUse {
            id: "toolu_1".to_owned(),
            tool: "Write".to_owned(),
            summary: "notes.txt".to_owned(),
        }
    );
    assert_eq!(
        events[3],
        AgentEvent::ToolResult {
            tool_use_id: "toolu_1".to_owned(),
            tool: Some("Write".to_owned()),
            failed: false,
        }
    );
    assert_eq!(events.len(), 5);
    assert_eq!(events[4], AgentEvent::TurnEnded(outcome.clone()));

    assert_eq!(outcome.failure, None);
    assert_eq!(outcome.total_tokens().cache_written, 4);
    assert_eq!(outcome.estimated_cost_usd, Some(0.5));
    assert_eq!(outcome.model_calls, Some(2));
}

fn the_program_gets_the_prompt_the_folder_and_only_the_given_environment() {
    let folder = tempfile::tempdir().unwrap();
    let (_, outcome) = run(
        &stand_in("turn"),
        request(
            folder.path().to_owned(),
            "Write the plan, ünïcödé included.",
        ),
    );
    let report = report(&outcome);

    assert_eq!(report["prompt_start"], "Write the plan, ünïcödé included.");
    let arguments: Vec<&str> = report["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument.as_str().unwrap())
        .collect();
    assert_eq!(
        &arguments[..4],
        ["-p", "--output-format", "stream-json", "--verbose"]
    );
    assert!(!arguments.iter().any(|argument| argument.contains("plan")));
    assert_eq!(
        PathBuf::from(report["folder"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        folder.path().canonicalize().unwrap()
    );
    assert_eq!(report["probe"], "given");
    assert_eq!(report["path_given"], false);
}

fn a_long_prompt_arrives_whole() {
    let folder = tempfile::tempdir().unwrap();
    let prompt = "word ".repeat(200_000);
    let (_, outcome) = run(
        &stand_in("turn"),
        request(folder.path().to_owned(), &prompt),
    );
    assert_eq!(report(&outcome)["prompt_length"], 1_000_000);
}

fn a_resumed_turn_names_its_session() {
    let folder = tempfile::tempdir().unwrap();
    let mut resumed = request(folder.path().to_owned(), "Go on");
    let id = SessionId::parse("0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10").unwrap();
    resumed.session = SessionChoice::Resume(id.clone());
    let (events, outcome) = run(&stand_in("turn"), resumed);
    assert!(matches!(&events[0], AgentEvent::SessionStarted(started) if started.session_id == id));
    assert_eq!(outcome.session_id, Some(id));
}

fn a_running_turn_can_be_stopped() {
    let folder = tempfile::tempdir().unwrap();
    let started = Arc::new(Mutex::new(false));
    let seen = Arc::clone(&started);
    let turn = stand_in("hang")
        .start_turn(
            request(folder.path().to_owned(), "Take your time"),
            Box::new(move |event| {
                if matches!(event, AgentEvent::SessionStarted(_)) {
                    *seen.lock().unwrap() = true;
                }
            }),
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(20);
    while !*started.lock().unwrap() {
        assert!(Instant::now() < deadline, "the session never started");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!turn.is_finished());

    let stopping = Instant::now();
    turn.stop();
    turn.stop();
    let outcome = turn.wait().unwrap();
    assert_eq!(outcome.failure, Some(TurnFailure::Stopped));
    assert!(stopping.elapsed() < Duration::from_secs(10));
}

fn a_program_that_ends_early_is_reported_with_its_errors() {
    let folder = tempfile::tempdir().unwrap();
    let (events, outcome) = run(&stand_in("crash"), request(folder.path().to_owned(), "Hi"));
    assert!(matches!(events.last(), Some(AgentEvent::TurnEnded(_))));
    let Some(TurnFailure::ProgramEnded { message }) = outcome.failure else {
        panic!("{:?}", outcome.failure);
    };
    assert!(message.contains("exit code 3"), "{message}");
    assert!(
        message.contains("first problem / second problem"),
        "{message}"
    );
}

fn a_turn_in_a_missing_folder_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let result = stand_in("turn").start_turn(
        request(folder.path().join("missing"), "Hi"),
        Box::new(|_| {}),
    );
    assert_eq!(
        result.err().map(|error| error.code()),
        Some("agent_invalid_request")
    );
}

fn a_session_is_opened_in_the_desktop_app() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse("abc").unwrap();
    stand_in("desktop")
        .open_in_desktop_app(&id, folder.path())
        .unwrap();
}

fn an_old_version_cannot_open_the_desktop_app() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse("abc").unwrap();
    let old = ClaudeCode::new()
        .with_program(std::env::current_exe().unwrap())
        .with_environment(
            Environment::empty()
                .with(ROLE, "desktop")
                .with(VERSION, "2.1.284"),
        );
    assert_eq!(
        old.open_in_desktop_app(&id, folder.path()).unwrap_err(),
        AgentRuntimeError::VersionTooOld {
            installed: ProgramVersion::new(2, 1, 284),
            needed: ProgramVersion::new(2, 1, 285),
        }
    );
}

fn a_failure_to_open_the_desktop_app_is_reported() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse("abc").unwrap();
    let error = stand_in("desktop-fails")
        .open_in_desktop_app(&id, folder.path())
        .unwrap_err();
    assert!(error.to_string().contains("not installed"), "{error}");
}
