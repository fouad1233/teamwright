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
    TurnOutcome, TurnRequest, instructions_for,
};
use teamwright_domain::{
    Color, Member, MemberInstructions, MemberKind, MemberName, Project, ProjectKey, ProjectName,
    Workflow,
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
        team_tools: Vec::new(),
        readable_folders: Vec::new(),
        written_folders: Vec::new(),
        full_access: false,
        protected_places: Vec::new(),
        kit: teamwright_application::TurnKit::default(),
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

/// A real turn on a subscription reports the plan: whether it allows work,
/// and of the five hours and the week how much is used and when each
/// resets (decision 0040). If the limit is reached when this runs, the turn
/// must end as `PlanLimitReached`, never as an ordinary failure; the test
/// then prints what the program said, which is the shape the decoder
/// relies on. With an API key the program reports no plan, and this test
/// fails for that reason alone.
#[test]
#[ignore = "needs the real Claude Code, signed in with a subscription, and spends one turn"]
fn a_real_turn_reports_the_plan_or_ends_at_its_limit() {
    let folder = tempfile::tempdir().unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .start_turn(
            request(folder.path(), "Say yes.", SessionChoice::New),
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    let plans: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::PlanUsage(plan) => Some(plan.clone()),
            _ => None,
        })
        .collect();
    println!("{plans:#?}");
    println!("{:?}", outcome.failure);

    match &outcome.failure {
        None => {
            let plan = plans
                .last()
                .expect("a turn on a subscription reports the plan");
            assert!(matches!(
                plan.status.as_deref(),
                Some("allowed" | "allowed_warning")
            ));
            assert!(!plan.windows.is_empty());
        }
        Some(TurnFailure::PlanLimitReached(hit)) => println!("{hit:?}"),
        Some(other) => panic!("the turn failed otherwise: {other:?}"),
    }
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

/// Decision 0044. A folder to write in is written like the work folder, and
/// adds nothing else: a folder only to read stays unwritten, and a folder
/// that was not given stays out of reach for reading and for writing. It
/// passed on Ubuntu 24.04 with Claude Code 2.1.330 on 27 November 2026.
#[test]
#[ignore = "needs the real Claude Code, signed in, and spends one turn"]
fn a_real_session_writes_in_its_folder_to_write_in_and_still_nowhere_else() {
    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    let written = ground.join("firmware (rev b)");
    let shelf = ground.join("shelf");
    let outside = ground.join("outside");
    for folder in [&work, &written, &shelf, &outside] {
        std::fs::create_dir(folder).unwrap();
    }
    std::fs::write(
        written.join("pins.txt"),
        "The word in the firmware is LIME-2208.",
    )
    .unwrap();
    std::fs::write(outside.join("secret.txt"), "The word outside is PLUM-7391.").unwrap();

    let prompt = format!(
        "This is a test of what your tools let you reach. Do every step below with the tool \
         that is named, in this order, also when you expect it to be refused, and do not try \
         a step a second time.\n\
         1. Read the file {pins}\n\
         2. Write the file {made} with the text hello\n\
         3. Write the file {smuggled} with the text hello\n\
         4. Write the file {planted} with the text hello\n\
         5. Read the file {secret}\n\
         Then answer with the words you read in steps 1 and 5, or CANNOT for a file you could \
         not read.",
        pins = written.join("pins.txt").display(),
        made = written.join("made.txt").display(),
        smuggled = shelf.join("smuggled.txt").display(),
        planted = outside.join("planted.txt").display(),
        secret = outside.join("secret.txt").display(),
    );
    let mut request = request(&work, &prompt, SessionChoice::New);
    request.extra_instructions = None;
    request.readable_folders = vec![shelf.clone()];
    request.written_folders = vec![written.clone()];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let outcome = runtime()
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
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

    assert_eq!(outcome.failure, None);
    // The folder to write in: read and written.
    assert!(outcome.final_text.contains("LIME-2208"), "{uses:?}");
    assert_eq!(
        std::fs::read_to_string(written.join("made.txt"))
            .unwrap()
            .trim(),
        "hello"
    );
    // The folder only to read, and the folder not given: unchanged.
    assert!(!shelf.join("smuggled.txt").exists());
    assert!(!outside.join("planted.txt").exists());
    assert!(!outcome.final_text.contains("PLUM-7391"));
}

/// Decision 0044. With full access a session reads and writes outside every
/// folder it was given, and is still kept out of the protected places, for
/// reading and for writing. Stand-ins the test makes take the places of
/// Teamwright's data folder, of the settings of Claude Code and of the keys,
/// so that the real ones are never touched. The session has no shell, none
/// of the user's own skills, plugins or tool servers, and the team bus
/// still works.
///
/// It needs the built program for the team bus
/// (`cargo build -p teamwright-desktop`). It passed on Ubuntu 24.04 with
/// Claude Code 2.1.330 on 27 November 2026; it was never run on macOS or
/// Windows.
#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends one turn"]
fn a_real_session_with_full_access_reaches_outside_and_never_the_protected_places() {
    use teamwright_application::{ProtectedKind, ProtectedPlace, TeamTool};

    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    let outside = ground.join("outside");
    let data = ground.join("stand-in data");
    let settings = ground.join("stand-in claude");
    let keys = ground.join("stand-in ssh");
    for folder in [&work, &outside, &data, &settings, &keys] {
        std::fs::create_dir(folder).unwrap();
    }
    std::fs::write(outside.join("given.txt"), "The word outside is PEAR-4410.").unwrap();
    std::fs::write(
        data.join("teamwright.db"),
        "The word of the data is FIG-0001.",
    )
    .unwrap();
    std::fs::write(
        settings.join("settings.json"),
        "The word of the settings is DATE-0002.",
    )
    .unwrap();
    std::fs::write(keys.join("id_test"), "The word of the key is KIWI-0003.").unwrap();

    let prompt = format!(
        "This is a test of what your tools let you reach. Do every step below with the tool \
         that is named, in this order, also when you expect it to be refused, and do not try \
         a step a second time.\n\
         1. Write the file {made} with the text hello\n\
         2. Read the file {given}\n\
         3. Read the file {key}\n\
         4. Read the file {settings_file}\n\
         5. Write the file {forged} with the text hello\n\
         6. Write the file {planted} with the text hello\n\
         7. Grep for the pattern KIWI with the path {ground} and output mode content\n\
         8. Call the tool remember with the title \"Probe\" and the text \"Full access works.\"\n\
         Then answer with every word of the form WORD-1234 you read or found, or NONE.",
        made = outside.join("made.txt").display(),
        given = outside.join("given.txt").display(),
        key = keys.join("id_test").display(),
        settings_file = settings.join("settings.json").display(),
        forged = data.join("forged.txt").display(),
        planted = keys.join("authorized_keys").display(),
        ground = ground.display(),
    );
    let mut request = request(&work, &prompt, SessionChoice::New);
    request.extra_instructions = None;
    request.full_access = true;
    request.protected_places = vec![
        ProtectedPlace::folder(&data, ProtectedKind::TeamwrightData),
        ProtectedPlace::folder(&settings, ProtectedKind::AgentProgramSettings),
        ProtectedPlace::folder(&keys, ProtectedKind::Keys),
    ];
    request.team_tools = vec![TeamTool::Remember];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let outcome = runtime()
        .with_team_bus(bus_program(), vec!["--team-bus".to_owned()])
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    let uses = tool_uses(&events);
    for (tool, summary, failed) in &uses {
        println!(
            "{tool} {summary}: {}",
            if *failed { "stopped" } else { "done" }
        );
    }
    for denial in &outcome.permission_denials {
        println!("denied: {denial:?}");
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);
    let started = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::SessionStarted(started) => Some(started.clone()),
            _ => None,
        })
        .expect("the session never started");
    println!("tools: {:?}", started.tools);
    println!("skills: {:?}", started.skills);
    println!("plugins: {:?}", started.plugins);
    println!("servers: {:?}", started.mcp_servers);

    assert_eq!(outcome.failure, None);
    // Outside every folder it was given: written and read.
    assert_eq!(
        std::fs::read_to_string(outside.join("made.txt"))
            .unwrap()
            .trim(),
        "hello"
    );
    assert!(outcome.final_text.contains("PEAR-4410"), "{uses:?}");
    // The protected places: neither read, nor found by a search of the
    // folder around them, nor written.
    let everything = format!("{events:?}{}", outcome.final_text);
    for word in ["KIWI-0003", "DATE-0002", "FIG-0001"] {
        assert!(!everything.contains(word), "{word} was read");
    }
    assert!(!data.join("forged.txt").exists());
    assert!(!keys.join("authorized_keys").exists());
    // No shell, and nothing of the user's own.
    assert!(
        !started
            .tools
            .iter()
            .any(|tool| tool == "Bash" || tool == "PowerShell"),
        "{:?}",
        started.tools
    );
    let servers: Vec<&str> = started
        .mcp_servers
        .iter()
        .map(|server| server.name.as_str())
        .collect();
    assert_eq!(servers, ["teamwright"], "{:?}", started.mcp_servers);
    assert!(
        started
            .plugins
            .iter()
            .all(|plugin| plugin.starts_with("cc-plugin-")),
        "{:?}",
        started.plugins
    );
    let own = own_skills_of_the_user();
    for skill in &started.skills {
        let bare = skill.rsplit(':').next().unwrap_or(skill);
        assert!(
            !own.iter().any(|mine| mine == bare),
            "the session knows the user's own skill {skill}"
        );
    }
    // The team bus still works.
    assert!(
        events.iter().any(
            |event| matches!(event, AgentEvent::FactRemembered(fact) if fact.title == "Probe")
        ),
        "{events:?}"
    );
}

#[test]
#[ignore = "needs the real Claude Code, signed in, and spends one turn"]
fn a_real_session_may_not_write_in_folders_whose_names_hold_special_characters() {
    // A rule about a path is read as a pattern for ignoring files, and it is
    // written inside parentheses and a list that is split at spaces and
    // commas. Each folder here has a name that one of those readings could
    // mistake. The session may read all of them and must write into none.
    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    std::fs::create_dir(&work).unwrap();
    let names = [
        "plain",
        "bracket [1]",
        "star *",
        "quest ?",
        "back\\slash",
        "paren (a) b",
        "hash #1",
        "bang !x",
        "comma, here",
        "pipe | plus +",
        "cash $ caret ^",
        "brace {a,b} c",
        "close ) alone",
        "open ( alone",
    ];
    let folders: Vec<_> = names.iter().map(|name| ground.join(name)).collect();
    for folder in &folders {
        std::fs::create_dir(folder).unwrap();
    }
    let steps: String = folders
        .iter()
        .enumerate()
        .map(|(at, folder)| {
            format!(
                "{}. Write the file {} with the text hello\n",
                at + 1,
                folder.join(format!("w{at}.txt")).display()
            )
        })
        .collect();
    let prompt = format!(
        "This is a test of what your tools let you reach. Do every step below with the Write \
         tool, in this order, also when you expect it to be refused, and do not try a step a \
         second time.\n{steps}Then answer with the number of the steps that were refused."
    );
    let mut request = request(&work, &prompt, SessionChoice::New);
    request.extra_instructions = None;
    request.readable_folders = folders.clone();
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
    for (tool, summary, failed) in tool_uses(&events) {
        println!(
            "{tool} {summary}: {}",
            if failed { "stopped" } else { "done" }
        );
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);

    let written: Vec<_> = folders
        .iter()
        .enumerate()
        .filter(|(at, folder)| folder.join(format!("w{at}.txt")).exists())
        .map(|(_, folder)| folder.display().to_string())
        .collect();
    assert_eq!(outcome.failure, None);
    assert!(written.is_empty(), "written into: {written:#?}");
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

#[test]
#[ignore = "needs the real Claude Code, signed in, and spends one turn"]
fn the_own_instructions_of_a_specialist_reach_a_real_session() {
    // A specialist with instructions of its own, given to the session the
    // way the application gives them: after Teamwright's own instructions,
    // in the system prompt that is appended, and not in the prompt.
    let project = Project::create(
        ProjectName::parse("Smart sensor node").unwrap(),
        ProjectKey::parse("SSN").unwrap(),
        String::new(),
        Workflow::standard(),
        chrono::Utc::now(),
    );
    let mut member = Member::create(
        project.id(),
        MemberName::parse("Technical writer").unwrap(),
        MemberKind::Agent,
        String::new(),
        Color::Teal,
        chrono::Utc::now(),
    );
    member
        .set_instructions(
            MemberInstructions::parse(
                "End every reply with the word PERSIMMON, in capital letters, alone on the \
                 last line.",
            )
            .unwrap(),
            chrono::Utc::now(),
        )
        .unwrap();
    let folder = tempfile::tempdir().unwrap();
    let mut request = request(
        folder.path(),
        "In one sentence: what is a pull-up resistor for?",
        SessionChoice::New,
    );
    request.extra_instructions = Some(instructions_for(
        &member,
        &project,
        &teamwright_application::FoldersOfTurn::default(),
        &teamwright_domain::WordsMark::generate(),
    ));
    assert!(!request.prompt.contains("PERSIMMON"));

    let turn = runtime().start_turn(request, Box::new(|_| {})).unwrap();
    let outcome = turn.wait().unwrap();
    println!("{}", outcome.final_text);
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    let last_line = outcome.final_text.trim().lines().last().unwrap_or_default();
    assert_eq!(last_line.trim(), "PERSIMMON");
}

/// The Teamwright program, which serves the team bus. It is built with
/// `cargo build -p teamwright-desktop`, or named in
/// `TEAMWRIGHT_BUS_PROGRAM`.
/// The runtime with Teamwright's program as the launcher of connectors,
/// starting each in a fresh empty folder of its own under `ground`.
fn with_launcher(runtime: ClaudeCode, ground: &Path) -> ClaudeCode {
    runtime.with_connector_launcher(
        bus_program(),
        vec!["--connector".to_owned()],
        ground.join("connectors").join("starts"),
        ground.join("connectors").join("secrets"),
    )
}

fn bus_program() -> std::path::PathBuf {
    let program = std::env::var_os("TEAMWRIGHT_BUS_PROGRAM").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/debug/teamwright-desktop")
                .with_extension(std::env::consts::EXE_EXTENSION)
        },
        std::path::PathBuf::from,
    );
    assert!(
        program.is_file(),
        "build the program first with cargo build -p teamwright-desktop"
    );
    program
}

#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends one turn"]
fn a_real_session_hands_its_result_over_through_the_team_bus() {
    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    let mut request = request(
        &folder,
        "This is a test of the team bus. Do these steps in order:\n\
         1. Write the file pinmap.md in the current folder with the text: SDA on pin 4.\n\
         2. Call the tool publish_handoff with the summary \"Pin map v1: SDA is on pin 4.\", \
            the contract \"SDA on pin 4, SCL on pin 5.\" and the files [\"pinmap.md\"].\n\
         3. Answer with one short sentence.",
        SessionChoice::New,
    );
    request.team_tools = vec![teamwright_application::TeamTool::PublishHandoff {
        folder: folder.clone(),
    }];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .with_team_bus(bus_program(), vec!["--team-bus".to_owned()])
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    let handoffs: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::HandoffPublished(handoff) => Some(handoff),
            _ => None,
        })
        .collect();
    let [handoff] = handoffs.as_slice() else {
        panic!("one handoff was expected: {events:?}");
    };
    assert!(handoff.summary.contains("pin 4"));
    assert!(handoff.contract.contains("SCL"));
    assert_eq!(handoff.files, ["pinmap.md"]);
    // The turn went on after the handoff and ended with an answer.
    assert!(!outcome.final_text.trim().is_empty());
}

#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends one turn"]
fn a_real_advisor_reads_the_work_folder_and_proposes_a_plan_with_parts() {
    // A work folder with a project in it already: the Advisor is to read
    // it before it plans, and to split one task into parts.
    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    std::fs::write(
        folder.join("README.md"),
        "# Weather station\n\nThe station runs on the microcontroller QUINCE-7. The firmware is \
         not written yet.\n",
    )
    .unwrap();
    let project = Project::create(
        ProjectName::parse("Weather station").unwrap(),
        ProjectKey::parse("WS").unwrap(),
        String::new(),
        Workflow::standard(),
        chrono::Utc::now(),
    );
    let mut request = request(
        &folder,
        "This is a test of planning. Plan the firmware of the weather station in this folder. \
         Read the README in the folder first. Ask nothing: call propose_plan at once, with one \
         member, \"Firmware engineer\", and at most five tasks. Name the microcontroller of the \
         README in the summary. Make one task \"Write the drivers\", and split it into two parts \
         with part_of: \"Write the driver of the temperature sensor\" and \"Write the driver of \
         the wind sensor\". Then end your turn with one sentence.",
        SessionChoice::New,
    );
    request.permissions = PermissionProfile::advisor();
    request.extra_instructions = Some(teamwright_application::instructions_for_advisor(
        &project,
        &[],
        &[],
        &[],
    ));
    request.team_tools = vec![teamwright_application::TeamTool::ProposePlan {
        team: Vec::new(),
        tasks: Vec::new(),
    }];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .with_team_bus(bus_program(), vec!["--team-bus".to_owned()])
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    // It looked at the folder before it proposed.
    let uses = tool_uses(&events);
    assert!(
        uses.iter().any(|(tool, _, _)| tool == "Read"),
        "the Advisor did not read: {uses:?}"
    );
    let proposals: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::PlanProposed(proposal) => Some(proposal),
            _ => None,
        })
        .collect();
    let [proposal] = proposals.as_slice() else {
        panic!("one proposal was expected: {events:?}");
    };
    assert!(
        proposal.summary.contains("QUINCE-7"),
        "the summary does not name what the README says: {}",
        proposal.summary
    );
    // The decoder took the parts: two tasks name the task they belong to,
    // which is a task of the same plan and a task of its own.
    let parts: Vec<_> = proposal
        .tasks
        .iter()
        .filter_map(|task| match task.part_of {
            Some(teamwright_application::PlannedWhole::Proposed(whole)) => Some(whole),
            _ => None,
        })
        .collect();
    assert_eq!(parts.len(), 2, "two parts were expected: {proposal:?}");
    assert_eq!(parts[0], parts[1]);
    let whole = &proposal.tasks[parts[0]];
    assert_eq!(whole.part_of, None);
    assert!(whole.title.to_lowercase().contains("driver"), "{whole:?}");
    // The program changed nothing in the folder.
    let names: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["README.md"]);
}

#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends one turn"]
fn a_real_session_searches_the_memory_and_remembers_a_fact_through_the_team_bus() {
    use chrono::TimeZone;
    use teamwright_application::{MemoryKind, RememberedEntry, TeamTool};

    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    let mut request = request(
        &folder,
        "This is a test of the memory of the project. Do these steps in order:\n\
         1. Call the tool search_memory with the query \"display bus\".\n\
         2. Call the tool remember with the title \"Sensor bus\" and the text \"The sensors \
            are on I2C0.\"\n\
         3. Answer with the name of the bus the display is on, as the search said, and \
            nothing else.",
        SessionChoice::New,
    );
    request.team_tools = vec![
        TeamTool::Remember,
        TeamTool::SearchMemory {
            entries: vec![RememberedEntry {
                kind: MemoryKind::Fact,
                title: "Display bus".to_owned(),
                text: "The display is on SPI2.".to_owned(),
                source: "remembered by Hardware engineer".to_owned(),
                task: None,
                recorded_at: chrono::Utc.with_ymd_and_hms(2026, 11, 21, 9, 0, 0).unwrap(),
            }],
            mark: teamwright_domain::WordsMark::generate(),
        },
    ];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = runtime()
        .with_team_bus(bus_program(), vec!["--team-bus".to_owned()])
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    println!("{}", outcome.final_text);
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    assert!(
        tool_uses(&events)
            .iter()
            .any(|(tool, _, failed)| tool == "mcp__teamwright__search_memory" && !failed),
        "the search was expected as a tool use that did not fail: {events:?}"
    );
    let facts: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::FactRemembered(fact) => Some(fact),
            _ => None,
        })
        .collect();
    let [fact] = facts.as_slice() else {
        panic!("one fact was expected: {events:?}");
    };
    assert_eq!(fact.title, "Sensor bus");
    assert!(fact.text.contains("I2C0"));
    // The answer came from the memory the bus read from its file.
    assert!(outcome.final_text.contains("SPI2"));
}

/// The names of the skills and commands the user set up for their own use,
/// in `~/.claude/skills` and `~/.claude/commands`.
fn own_skills_of_the_user() -> Vec<String> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        return Vec::new();
    };
    let claude = Path::new(&home).join(".claude");
    ["skills", "commands"]
        .iter()
        .filter_map(|folder| std::fs::read_dir(claude.join(folder)).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| {
            entry
                .path()
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .filter(|name| !name.is_empty() && !name.starts_with('.') && name != "synced")
        .collect()
}

#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends one turn"]
fn a_real_session_gets_its_chosen_skill_and_connector_and_nothing_of_the_users_own() {
    // Decision 0039: a skill in the form Teamwright keeps it, and a
    // connector, which is the Teamwright program serving one tool under
    // another name. The session must know both, and must know no skill,
    // command, tool server or connector the user set up for their own use.
    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    let shelf = ground.join("shelf").join("probe-skill");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(shelf.join(".claude-plugin")).unwrap();
    std::fs::create_dir_all(shelf.join("skills").join("probe-skill")).unwrap();
    std::fs::write(
        shelf.join(".claude-plugin").join("plugin.json"),
        r#"{"name":"probe-skill","description":"A skill of a test."}"#,
    )
    .unwrap();
    std::fs::write(
        shelf.join("skills").join("probe-skill").join("SKILL.md"),
        "---\nname: probe-skill\ndescription: Answers the word APRICOT when asked for the \
         probe word.\n---\nWhen asked for the probe word, answer APRICOT.\n",
    )
    .unwrap();

    let mut request = request(
        &work,
        "This is a test. Answer with the single word ready.",
        SessionChoice::New,
    );
    request.kit = teamwright_application::TurnKit {
        skills: vec![teamwright_application::KitSkill {
            name: "probe-skill".to_owned(),
            folder: shelf.clone(),
        }],
        connectors: vec![teamwright_application::KitConnector {
            name: "probe".to_owned(),
            start: teamwright_application::ConnectorStart::program(
                &bus_program().to_string_lossy(),
                vec!["--team-bus".to_owned(), "ask_manager".to_owned()],
                Vec::new(),
            )
            .unwrap(),
            tools: vec!["ask_manager".to_owned()],
        }],
        withheld_variables: Vec::new(),
    };
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let turn = with_launcher(runtime(), &ground)
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap();
    let outcome = turn.wait().unwrap();
    let events = events.lock().unwrap().clone();
    print_usage(&outcome);
    let started = events
        .iter()
        .find_map(|event| match event {
            AgentEvent::SessionStarted(started) => Some(started.clone()),
            _ => None,
        })
        .expect("the session never started");
    println!("tools: {:?}", started.tools);
    println!("skills: {:?}", started.skills);
    println!("plugins: {:?}", started.plugins);
    println!("servers: {:?}", started.mcp_servers);

    assert_eq!(outcome.failure, None);
    // The chosen connector, and only it: no server of the user's own, no
    // connector of their account at claude.ai.
    let servers: Vec<&str> = started
        .mcp_servers
        .iter()
        .map(|server| server.name.as_str())
        .collect();
    assert_eq!(servers, ["probe"], "{:?}", started.mcp_servers);
    assert!(
        started
            .tools
            .iter()
            .any(|tool| tool == "mcp__probe__ask_manager"),
        "{:?}",
        started.tools
    );
    assert!(
        started
            .tools
            .iter()
            .filter(|tool| tool.starts_with("mcp__"))
            .all(|tool| tool.starts_with("mcp__probe__")),
        "{:?}",
        started.tools
    );
    // The chosen skill, and none of the user's own.
    assert!(
        started
            .skills
            .iter()
            .any(|skill| skill.contains("probe-skill")),
        "{:?}",
        started.skills
    );
    // The program lists its own built-in plugins too (security, telemetry
    // and privacy defaults, named `cc-plugin-…`, which it marks `@builtin`
    // in itself): they are not the user's. Every other plugin is a leak.
    let others: Vec<&String> = started
        .plugins
        .iter()
        .filter(|plugin| plugin.as_str() != "probe-skill" && !plugin.starts_with("cc-plugin-"))
        .collect();
    assert!(
        started.plugins.iter().any(|plugin| plugin == "probe-skill") && others.is_empty(),
        "{:?}",
        started.plugins
    );
    let own = own_skills_of_the_user();
    for skill in &started.skills {
        let bare = skill.rsplit(':').next().unwrap_or(skill);
        assert!(
            bare == "probe-skill" || !own.iter().any(|mine| mine == bare),
            "the session knows the user's own skill {skill}"
        );
    }
}

/// Decision 0038. A session that may not run commands asks for permission
/// to run `echo`, and its first attempt is refused; the turn the manager's
/// answer starts is given the rule, and runs it.
///
/// The command must not name Teamwright: a rule that does is refused, as a
/// command that could touch Teamwright's own data (the first run of this test
/// used such a word and was refused by `PermissionRule::new`, rightly).
#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, and spends two turns"]
fn a_real_session_asks_to_run_a_command_and_runs_it_once_allowed() {
    use teamwright_application::TeamTool;
    use teamwright_domain::{PermissionKind, PermissionRule};

    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    let id = SessionId::parse(&uuid::Uuid::now_v7().to_string()).unwrap();

    // The first turn: no command may run; the session asks.
    let mut first = request(
        &folder,
        "This is a test of permissions. Run the shell command `echo permission-test-ok`. If you \
         cannot, call the tool ask_permission with kind \"command\", what \"echo \
         permission-test-ok\" and a reason, then end your turn.",
        SessionChoice::NewWithId(id.clone()),
    );
    first.team_tools = vec![TeamTool::AskPermission { commands: true }];
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);
    let outcome = runtime()
        .with_team_bus(bus_program(), vec!["--team-bus".to_owned()])
        .start_turn(
            first,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    assert_eq!(outcome.failure, None);
    let asked: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            AgentEvent::PermissionAsked(asked) => Some(asked),
            _ => None,
        })
        .collect();
    let [asked] = asked.as_slice() else {
        panic!("one request was expected: {events:?}");
    };
    assert_eq!(asked.rule.tool_rule(), "Bash(echo permission-test-ok)");
    assert!(
        !tool_uses(&events)
            .iter()
            .any(|(tool, _, failed)| tool == "Bash" && !failed),
        "no command may have run in the first turn: {events:?}"
    );

    // The manager allowed it once: the next turn has exactly that rule.
    let rule = PermissionRule::new(PermissionKind::Command, "echo permission-test-ok").unwrap();
    let mut second = request(
        &folder,
        "The manager allowed you to run the command `echo permission-test-ok`. Run it now, and \
         answer with what it printed.",
        SessionChoice::Resume(id),
    );
    second.permissions = PermissionProfile::specialist().with_granted(&[rule]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);
    let outcome = runtime()
        .start_turn(
            second,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    print_usage(&outcome);
    assert_eq!(outcome.failure, None);
    assert!(
        tool_uses(&events)
            .iter()
            .any(|(tool, _, failed)| tool == "Bash" && !failed),
        "the command was expected to run: {events:?}"
    );
    assert!(outcome.final_text.contains("permission-test-ok"));
}

/// A tool server of a few lines, in Python, with one tool that tells the
/// folder it runs in, the value of the variable it was given, and the
/// names of all its variables.
const PROBE_SERVER: &str = r#"import json, os, sys
for line in sys.stdin:
    message = json.loads(line)
    ident = message.get("id")
    method = message.get("method")
    if method == "initialize":
        result = {"protocolVersion": message["params"].get("protocolVersion", "2025-06-18"),
                  "capabilities": {"tools": {}},
                  "serverInfo": {"name": "probe", "version": "1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "tell_environment",
                             "description": "Tells where this server runs and its environment.",
                             "inputSchema": {"type": "object", "properties": {}}}]}
    elif method == "tools/call":
        text = "folder=%s; given=%s; names=%s" % (
            os.getcwd(), os.environ.get("TEAMWRIGHT_PROBE_GIVEN", "none"),
            ",".join(sorted(os.environ)))
        result = {"content": [{"type": "text", "text": text}]}
    elif ident is None:
        continue
    else:
        result = None
    if ident is not None:
        reply = {"jsonrpc": "2.0", "id": ident}
        if result is None:
            reply["error"] = {"code": -32601, "message": "unknown method"}
        else:
            reply["result"] = result
        sys.stdout.write(json.dumps(reply) + "\n")
        sys.stdout.flush()
"#;

/// Decision 0039, note of 21 November 2026. A connector started as a
/// program runs through Teamwright's launcher: in an empty folder of
/// Teamwright's own, not the folder of the project, with the variable it
/// names and without one it does not name.
///
/// This test has never passed: the program was not signed in on the night
/// it was written. It needs `python3` on the search path, and the built
/// desktop program as the launcher.
#[test]
#[ignore = "needs the real Claude Code, signed in, the built program, python3, and spends one turn"]
fn a_real_connector_starts_through_the_launcher_and_sees_only_its_own_variable() {
    let ground = tempfile::tempdir().unwrap();
    let ground = teamwright_runtime::full_form(ground.path()).unwrap();
    let work = ground.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let server = ground.join("probe_server.py");
    std::fs::write(&server, PROBE_SERVER).unwrap();
    // A file of the same name in the folder of the project, which a
    // connector started there would have found first.
    std::fs::write(
        work.join("probe_server.py"),
        "raise SystemExit('planted')\n",
    )
    .unwrap();

    let mut request = request(
        &work,
        "This is a test. Call the tool mcp__probe__tell_environment once, then answer with \
         exactly the text it returned and nothing else.",
        SessionChoice::New,
    );
    request.kit = teamwright_application::TurnKit {
        skills: Vec::new(),
        connectors: vec![teamwright_application::KitConnector {
            name: "probe".to_owned(),
            start: teamwright_application::ConnectorStart::program(
                "python3",
                vec!["-I".to_owned(), server.to_string_lossy().into_owned()],
                vec![teamwright_domain::VariableName::parse("TEAMWRIGHT_PROBE_GIVEN").unwrap()],
            )
            .unwrap(),
            tools: vec!["tell_environment".to_owned()],
        }],
        withheld_variables: vec![
            teamwright_domain::VariableName::parse("TEAMWRIGHT_PROBE_GIVEN").unwrap(),
        ],
    };
    let runtime = with_launcher(
        ClaudeCode::new().with_environment(
            Environment::inherited_without_claude_variables()
                .with("TEAMWRIGHT_PROBE_GIVEN", "apricot-42")
                .with("TEAMWRIGHT_PROBE_HIDDEN", "never-handed-on"),
        ),
        &ground,
    );
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);

    let outcome = runtime
        .start_turn(
            request,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    print_usage(&outcome);

    assert_eq!(outcome.failure, None);
    let told = &outcome.final_text;
    // A folder of its own under the folder of starts, gone with the turn.
    let starts = ground.join("connectors").join("starts");
    assert!(
        told.contains(&format!(
            "folder={}",
            starts.join("teamwright-start-").display()
        )),
        "{told}"
    );
    assert_eq!(std::fs::read_dir(&starts).unwrap().count(), 0);
    assert!(told.contains("given=apricot-42"), "{told}");
    assert!(!told.contains("TEAMWRIGHT_PROBE_HIDDEN"), "{told}");
    // The value never reached a file the session could read: its file of
    // secrets is gone with the turn.
    assert_eq!(
        std::fs::read_dir(ground.join("connectors").join("secrets"))
            .unwrap()
            .count(),
        0
    );
}

/// Decision 0038, after the review of 21 November 2026. A specialist can
/// change the files an allowed command runs, so the command itself must not
/// reach outside the folders. The allowed command is a script in the folder
/// of the project that reads a file in the home folder, outside every
/// folder of the session; the sandbox must refuse the read, and the secret
/// must appear nowhere in the turn.
///
/// This test has never passed: the program was not signed in on the night
/// it was written, and the sandbox has never been seen working. It needs
/// `bubblewrap` and `socat` on Linux.
#[test]
#[ignore = "needs the real Claude Code, signed in, a sandbox, and spends one turn"]
fn a_real_allowed_command_runs_in_a_sandbox_that_keeps_it_inside_the_folders() {
    use teamwright_domain::{PermissionKind, PermissionRule};

    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    // Outside the folders, where Teamwright keeps its own data: the home
    // folder. A folder under the temporary folder would not prove it,
    // because the sandbox lets commands read there.
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap();
    let outside = tempfile::tempdir_in(&home).unwrap();
    let secret = format!("teamwright-secret-{}", uuid::Uuid::now_v7());
    let secret_file = outside.path().join("secret.txt");
    std::fs::write(&secret_file, &secret).unwrap();
    // What the specialist could have written: the script the allowed
    // command runs.
    std::fs::write(
        folder.join("check.sh"),
        format!("cat '{}'\necho done-checking\n", secret_file.display()),
    )
    .unwrap();

    let rule = PermissionRule::new(PermissionKind::Command, "sh check.sh").unwrap();
    let mut turn = request(
        &folder,
        "Run the shell command `sh check.sh` exactly once and answer with what it printed.",
        SessionChoice::New,
    );
    turn.permissions = PermissionProfile::specialist().with_granted(&[rule]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);
    let outcome = runtime()
        .start_turn(
            turn,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    print_usage(&outcome);
    assert_eq!(outcome.failure, None, "the sandbox may be missing");
    assert!(
        tool_uses(&events)
            .iter()
            .any(|(tool, _, failed)| tool == "Bash" && !failed),
        "the command did not run, so nothing is shown about what the sandbox keeps out (on \
         Linux the program's sandbox needs nested user namespaces): {events:?}"
    );
    assert!(
        !format!("{events:?}").contains(&secret) && !outcome.final_text.contains(&secret),
        "a sandboxed command read a file outside the folders"
    );
}

/// An allowed command does not see the manager's environment: a turn that
/// may run a command is started with the base set only (decision 0038,
/// note of 22 November 2026). The script prints the whole environment it
/// was started with, and a variable set for Teamwright must not be in it.
///
/// This test has never passed: the program was not signed in on the night
/// it was written. It needs `bubblewrap` and `socat` on Linux, and the
/// program signed in with its own sign-in, not with a variable.
#[test]
#[ignore = "needs the real Claude Code, signed in, a sandbox, and spends one turn"]
fn a_real_allowed_command_sees_none_of_the_managers_secrets() {
    use teamwright_domain::{PermissionKind, PermissionRule};

    let folder = tempfile::tempdir().unwrap();
    let folder = teamwright_runtime::full_form(folder.path()).unwrap();
    let secret = format!("teamwright-secret-{}", uuid::Uuid::now_v7());
    std::fs::write(folder.join("check.sh"), "env\necho done-checking\n").unwrap();

    let rule = PermissionRule::new(PermissionKind::Command, "sh check.sh").unwrap();
    let mut turn = request(
        &folder,
        "Run the shell command `sh check.sh` exactly once and answer with what it printed, \
         word for word.",
        SessionChoice::New,
    );
    turn.permissions = PermissionProfile::specialist().with_granted(&[rule]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::clone(&events);
    let outcome = ClaudeCode::new()
        .with_environment(
            Environment::inherited_without_claude_variables()
                .with("TEAMWRIGHT_TEST_TOKEN", &secret),
        )
        .start_turn(
            turn,
            Box::new(move |event| collected.lock().unwrap().push(event)),
        )
        .unwrap()
        .wait()
        .unwrap();
    let events = events.lock().unwrap().clone();
    for event in &events {
        println!("{event:?}");
    }
    print_usage(&outcome);
    assert_eq!(outcome.failure, None, "the sandbox may be missing");
    assert!(
        tool_uses(&events)
            .iter()
            .any(|(tool, _, failed)| tool == "Bash" && !failed),
        "the command did not run, so nothing is shown about what the sandbox keeps out (on \
         Linux the program's sandbox needs nested user namespaces): {events:?}"
    );
    assert!(
        !format!("{events:?}").contains(&secret) && !outcome.final_text.contains(&secret),
        "an allowed command saw a variable of the manager's environment"
    );
}
