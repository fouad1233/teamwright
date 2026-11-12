//! Tests against the real Claude Code. They are ignored by default.
//!
//! They need Claude Code installed and signed in, and the turns count
//! against the user's plan: each is one short prompt on the cheapest model.
//! Run them with
//!
//! ```text
//! cargo test -p teamwright-runtime --test real_program -- --ignored --nocapture --test-threads 1
//! ```
//!
//! The program is started with the environment of the test without any
//! `CLAUDE…` or `ANTHROPIC…` variable, so the tests also work from a terminal
//! inside a Claude Code session. `TEAMWRIGHT_REAL_MODEL` chooses another
//! model than `haiku`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use teamwright_application::{
    AgentEvent, AgentRuntime, PermissionProfile, SessionChoice, SessionId, SignIn, TurnFailure,
    TurnOutcome, TurnRequest,
};
use teamwright_runtime::{ClaudeCode, Environment};

fn runtime() -> ClaudeCode {
    ClaudeCode::new().with_environment(Environment::inherited_without_claude_variables())
}

fn request(folder: &Path, prompt: &str, session: SessionChoice) -> TurnRequest {
    TurnRequest {
        working_folder: folder.to_owned(),
        prompt: prompt.to_owned(),
        session,
        model: Some(std::env::var("TEAMWRIGHT_REAL_MODEL").unwrap_or_else(|_| "haiku".to_owned())),
        effort: None,
        extra_instructions: Some("Answer in one short sentence.".to_owned()),
        permissions: PermissionProfile::specialist(),
        may_ask_manager: false,
        readable_folders: Vec::new(),
    }
}

fn print_usage(outcome: &TurnOutcome) {
    for model in &outcome.usage {
        println!(
            "{}: {} in, {} out, {} cache read, {} cache written (estimated {:?} USD)",
            model.model,
            model.tokens.input,
            model.tokens.output,
            model.tokens.cache_read,
            model.tokens.cache_written,
            outcome.estimated_cost_usd,
        );
    }
}

#[test]
#[ignore = "needs the real Claude Code, signed in"]
fn the_installed_program_is_found_and_signed_in() {
    let installation = runtime().installation().unwrap();
    println!("{installation:?}");
    assert!(matches!(
        runtime().sign_in().unwrap(),
        SignIn::SignedIn { .. }
    ));
}

#[test]
#[ignore = "needs the real Claude Code, signed in, and spends one turn"]
fn a_real_turn_writes_a_file_and_reports_its_tokens() {
    let folder = tempfile::tempdir().unwrap();
    let id = SessionId::parse(&uuid::Uuid::now_v7().to_string()).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .start_turn(
            request(
                folder.path(),
                "Create a file named answer.txt in the current folder containing the word yes. \
                 Do nothing else.",
                SessionChoice::NewWithId(id.clone()),
            ),
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    assert_eq!(outcome.session_id, Some(id));
    assert!(outcome.total_tokens().output > 0);
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::ToolUse { tool, summary, .. } if tool == "Write" && summary == "answer.txt"
    )));
    let written = std::fs::read_to_string(folder.path().join("answer.txt")).unwrap();
    assert!(written.contains("yes"));
}

/// What a turn did with one tool: the tool, what it was used on, and
/// whether the use failed or was refused.
fn tool_uses(events: &[AgentEvent]) -> Vec<(String, String, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::ToolUse { id, tool, summary } => {
                let failed = events.iter().any(|result| {
                    matches!(
                        result,
                        AgentEvent::ToolResult { tool_use_id, failed: true, .. } if tool_use_id == id
                    )
                });
                Some((tool.clone(), summary.clone(), failed))
            }
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "needs the real Claude Code, signed in, and spends one turn"]
fn a_real_session_reaches_its_folders_and_nothing_else() {
    // Three folders side by side: the one the session works in, one it may
    // only read, and one it was not given. They are named in their full
    // form, so that a refusal is about the folder and not about how its
    // path is written.
    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    let shelf = ground.join("data sheets (old)");
    let outside = ground.join("outside");
    for folder in [&work, &shelf, &outside] {
        std::fs::create_dir(folder).unwrap();
    }
    std::fs::write(
        shelf.join("supply.txt"),
        "The word on the shelf is FIG-5150.",
    )
    .unwrap();
    std::fs::write(outside.join("secret.txt"), "The word outside is PLUM-7391.").unwrap();

    // The session is told that this is a test and asked to try every step.
    // A session that declines to try shows nothing about what the program
    // lets it do.
    let prompt = format!(
        "This is a test of what your tools let you reach. Do every step below with the tool \
         that is named, in this order, also when you expect it to be refused, and do not try \
         a step a second time.\n\
         1. Read the file {supply}\n\
         2. Glob with the pattern *.txt and the path {shelf}\n\
         3. Write the file {smuggled} with the text hello\n\
         4. Read the file {secret}\n\
         5. Write the file {planted} with the text hello\n\
         6. Write the file kept.txt in your current folder with the text hello\n\
         Then answer with the words you read in steps 1 and 4, or CANNOT for a file you \
         could not read.",
        supply = shelf.join("supply.txt").display(),
        shelf = shelf.display(),
        smuggled = shelf.join("smuggled.txt").display(),
        secret = outside.join("secret.txt").display(),
        planted = outside.join("planted.txt").display(),
    );
    let mut request = request(&work, &prompt, SessionChoice::New);
    request.extra_instructions = None;
    request.readable_folders = vec![shelf.clone()];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    let uses = tool_uses(&events);
    for (tool, summary, failed) in &uses {
        println!(
            "{tool} {summary}: {}",
            if *failed { "stopped" } else { "done" }
        );
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);
    let stopped = |tool: &str, file: &str| {
        uses.iter()
            .any(|(used, summary, failed)| used == tool && summary.contains(file) && *failed)
    };
    let done = |tool: &str, file: &str| {
        uses.iter()
            .any(|(used, summary, failed)| used == tool && summary.contains(file) && !*failed)
    };

    assert_eq!(outcome.failure, None);
    // The folder it may read: read and searched, and not written to.
    assert!(done("Read", "supply.txt"), "{uses:?}");
    assert!(outcome.final_text.contains("FIG-5150"));
    assert!(done("Glob", "*.txt"), "{uses:?}");
    assert!(stopped("Write", "smuggled.txt"), "{uses:?}");
    assert!(!shelf.join("smuggled.txt").exists());
    // The folder it was not given: neither read nor written to.
    assert!(stopped("Read", "secret.txt"), "{uses:?}");
    assert!(!outcome.final_text.contains("PLUM-7391"));
    assert!(stopped("Write", "planted.txt"), "{uses:?}");
    assert!(!outside.join("planted.txt").exists());
    // Its own folder: written to.
    assert!(work.join("kept.txt").is_file());
    // What was refused is reported.
    assert!(!outcome.permission_denials.is_empty());
}

#[test]
#[ignore = "needs the real Claude Code, signed in, and spends part of a turn"]
fn a_real_turn_can_be_stopped() {
    let folder = tempfile::tempdir().unwrap();
    let started = Arc::new(Mutex::new(false));
    let seen = Arc::clone(&started);
    let turn = runtime()
        .start_turn(
            request(
                folder.path(),
                "Count slowly from one to two hundred, one number per line.",
                SessionChoice::New,
            ),
            Box::new(move |event| {
                if matches!(event, AgentEvent::SessionStarted(_)) {
                    *seen.lock().unwrap() = true;
                }
            }),
        )
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(60);
    while !*started.lock().unwrap() {
        assert!(Instant::now() < deadline, "the session never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    let stopping = Instant::now();
    turn.stop();
    let outcome = turn.wait().unwrap();
    println!("stopped after {:?}: {outcome:?}", stopping.elapsed());
    assert_eq!(outcome.failure, Some(TurnFailure::Stopped));
    assert!(stopping.elapsed() < Duration::from_secs(10));
}
