//! The reader of earlier conversations, on small made-up files in the shape
//! Claude Code writes: one JSON object on each line.

use std::fmt::Write as _;

use chrono::TimeZone;
use tempfile::TempDir;

use super::*;

const FOLDER: &str = "-home-ada-weather";

/// A typed request of the user.
fn typed(minute: u32, words: &str) -> String {
    serde_json::json!({
        "type": "user",
        "timestamp": format!("2026-10-03T09:{minute:02}:00.000Z"),
        "cwd": "/home/ada/weather",
        "isSidechain": false,
        "message": { "role": "user", "content": words },
    })
    .to_string()
}

/// An answer of Claude Code in words, with tool calls that write files.
fn answered(minute: u32, words: &str, files: &[&str]) -> String {
    let mut content = vec![
        serde_json::json!({ "type": "thinking", "thinking": "private thoughts" }),
        serde_json::json!({ "type": "text", "text": words }),
    ];
    content.extend(files.iter().map(|file| {
        serde_json::json!({
            "type": "tool_use",
            "name": "Write",
            "input": { "file_path": file, "content": "SECRET FILE CONTENT" },
        })
    }));
    serde_json::json!({
        "type": "assistant",
        "timestamp": format!("2026-10-03T09:{minute:02}:30.000Z"),
        "cwd": "/home/ada/weather",
        "isSidechain": false,
        "message": { "role": "assistant", "content": content },
    })
    .to_string()
}

/// What a tool answered, which Claude Code writes as a line of the user.
fn tool_result(minute: u32) -> String {
    serde_json::json!({
        "type": "user",
        "timestamp": format!("2026-10-03T09:{minute:02}:40.000Z"),
        "cwd": "/home/ada/weather",
        "toolUseResult": { "stdout": "TOKEN=from-a-tool" },
        "message": { "role": "user", "content": [
            { "type": "tool_result", "tool_use_id": "x", "content": "TOKEN=from-a-tool" }
        ] },
    })
    .to_string()
}

struct Disk {
    // Kept so that the folder lives as long as the test.
    _root: TempDir,
    projects: PathBuf,
}

fn disk() -> Disk {
    let root = TempDir::new().unwrap();
    let projects = root.path().join("projects");
    fs::create_dir_all(projects.join(FOLDER)).unwrap();
    Disk {
        _root: root,
        projects,
    }
}

impl Disk {
    fn write(&self, folder: &str, session: &str, lines: &[String]) {
        let folder = self.projects.join(folder);
        fs::create_dir_all(&folder).unwrap();
        let mut text = lines.join("\n");
        text.push('\n');
        fs::write(folder.join(format!("{session}.jsonl")), text).unwrap();
    }

    fn reader(&self) -> EarlierConversationsOnDisk {
        EarlierConversationsOnDisk::in_folder(self.projects.clone())
    }

    fn read(&self, source: &str) -> EarlierConversation {
        self.reader().read(source).unwrap().unwrap()
    }
}

fn at(minute: u32, second: u32) -> Timestamp {
    chrono::Utc
        .with_ymd_and_hms(2026, 10, 3, 9, minute, second)
        .unwrap()
}

#[test]
fn a_computer_where_claude_code_was_never_used_has_no_conversations() {
    let root = TempDir::new().unwrap();
    let reader = EarlierConversationsOnDisk::in_folder(root.path().join("nothing-here"));

    assert_eq!(reader.list().unwrap(), []);
    assert_eq!(reader.read(&format!("{FOLDER}/abc")).unwrap(), None);
}

#[test]
fn reads_what_the_user_typed_the_files_written_and_the_last_answer() {
    let disk = disk();
    disk.write(
        FOLDER,
        "first",
        &[
            typed(0, "Build a weather station"),
            answered(1, "Here is a plan.", &["/home/ada/weather/plan.md"]),
            tool_result(1),
            typed(2, "Use a BME280"),
            answered(
                3,
                "Done, the driver is written.",
                &["/home/ada/weather/bme.c"],
            ),
        ],
    );

    let conversation = disk.read(&format!("{FOLDER}/first"));

    assert_eq!(conversation.source, format!("{FOLDER}/first"));
    assert_eq!(conversation.held_in, "/home/ada/weather");
    assert_eq!(
        conversation.requests,
        ["Build a weather station", "Use a BME280"]
    );
    assert_eq!(
        conversation.changed_files,
        ["/home/ada/weather/plan.md", "/home/ada/weather/bme.c"]
    );
    assert_eq!(
        conversation.last_answer.as_deref(),
        Some("Done, the driver is written.")
    );
    assert_eq!(conversation.started_at, Some(at(0, 0)));
    assert_eq!(conversation.ended_at, Some(at(3, 30)));
}

#[test]
fn never_reads_what_tools_answered_or_what_files_contain() {
    let disk = disk();
    disk.write(
        FOLDER,
        "tools",
        &[
            typed(0, "Write the file"),
            answered(1, "Written.", &["/home/ada/weather/a.txt"]),
            tool_result(2),
        ],
    );

    let read = format!("{:?}", disk.read(&format!("{FOLDER}/tools")));

    assert!(!read.contains("from-a-tool"));
    assert!(!read.contains("SECRET FILE CONTENT"));
    assert!(!read.contains("private thoughts"));
}

#[test]
fn lines_the_program_added_by_itself_are_not_requests() {
    let disk = disk();
    let meta = serde_json::json!({
        "type": "user", "isMeta": true,
        "message": { "content": "Caveat: the messages below were generated" },
    });
    let command = serde_json::json!({
        "type": "user",
        "message": { "content": "<command-name>/clear</command-name>" },
    });
    let notice = serde_json::json!({
        "type": "user", "origin": { "kind": "task-notification" },
        "message": { "content": "A task finished" },
    });
    let summary = serde_json::json!({
        "type": "user", "isCompactSummary": true,
        "message": { "content": "This session is being continued" },
    });
    let helper = serde_json::json!({
        "type": "user", "isSidechain": true,
        "message": { "content": "Search the code for the driver" },
    });
    let interrupted = serde_json::json!({
        "type": "user", "message": { "content": "[Request interrupted by user]" },
    });
    let with_reminder = serde_json::json!({
        "type": "user", "origin": { "kind": "human" },
        "message": { "content": [
            { "type": "text", "text": "Make it smaller" },
            { "type": "image", "source": {} },
        ] },
    });
    let reminder_appended = serde_json::json!({
        "type": "user",
        "message": { "content": "Add a battery\n<system-reminder>internal</system-reminder>" },
    });
    disk.write(
        FOLDER,
        "added",
        &[
            meta.to_string(),
            command.to_string(),
            notice.to_string(),
            summary.to_string(),
            helper.to_string(),
            interrupted.to_string(),
            with_reminder.to_string(),
            reminder_appended.to_string(),
        ],
    );

    let conversation = disk.read(&format!("{FOLDER}/added"));

    assert_eq!(conversation.requests, ["Make it smaller", "Add a battery"]);
}

#[test]
fn an_answer_of_a_helper_is_not_the_last_answer() {
    let disk = disk();
    let mut helper: Value = serde_json::from_str(&answered(5, "helper words", &[])).unwrap();
    helper["isSidechain"] = Value::Bool(true);
    disk.write(
        FOLDER,
        "helper",
        &[
            typed(0, "Go"),
            answered(1, "Main answer.", &[]),
            helper.to_string(),
        ],
    );

    let conversation = disk.read(&format!("{FOLDER}/helper"));

    assert_eq!(conversation.last_answer.as_deref(), Some("Main answer."));
}

#[test]
fn odd_lines_unknown_kinds_and_a_broken_last_line_are_skipped() {
    let disk = disk();
    let folder = disk.projects.join(FOLDER);
    let text = [
        "not json at all".to_owned(),
        "[1, 2, 3]".to_owned(),
        "42".to_owned(),
        String::new(),
        r#"{"type":"some-kind-from-a-later-version","payload":{"x":1}}"#.to_owned(),
        r#"{"type":"user","message":{"content":17}}"#.to_owned(),
        r#"{"type":"user"}"#.to_owned(),
        r#"{"type":"assistant","message":"just text"}"#.to_owned(),
        typed(0, "Build a weather station"),
        answered(1, "Fine.", &[]),
        // Claude Code was stopped while it wrote this line.
        r#"{"type":"user","message":{"content":"Half a requ"#.to_owned(),
    ]
    .join("\n");
    fs::write(folder.join("odd.jsonl"), text).unwrap();

    let conversation = disk.read(&format!("{FOLDER}/odd"));

    assert_eq!(conversation.requests, ["Build a weather station"]);
    assert_eq!(conversation.last_answer.as_deref(), Some("Fine."));
}

#[test]
fn a_line_with_broken_characters_is_read_with_them_replaced() {
    let disk = disk();
    let folder = disk.projects.join(FOLDER);
    let mut bytes = typed(0, "Build it").into_bytes();
    bytes.push(b'\n');
    bytes.extend_from_slice(b"{\"type\":\"user\",\"message\":{\"content\":\"caf\xff\"}}\n");
    fs::write(folder.join("bytes.jsonl"), bytes).unwrap();

    let conversation = disk.read(&format!("{FOLDER}/bytes"));

    assert_eq!(conversation.requests.len(), 2);
    assert!(conversation.requests[1].starts_with("caf"));
}

#[test]
fn an_empty_file_is_a_conversation_without_requests() {
    let disk = disk();
    fs::write(disk.projects.join(FOLDER).join("empty.jsonl"), "").unwrap();

    let conversation = disk.read(&format!("{FOLDER}/empty"));

    assert!(conversation.requests.is_empty());
    assert_eq!(
        conversation.held_in, FOLDER,
        "the folder name stands in for the folder"
    );
    let listed = disk.reader().list().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].first_request, None);
    assert!(listed[0].ended_at.is_some(), "when the file was written");
}

#[test]
fn a_very_long_conversation_is_read_through_and_its_requests_are_cut() {
    let disk = disk();
    let mut lines = Vec::new();
    for number in 0..3000 {
        lines.push(typed(number % 60, &format!("request {number}")));
        lines.push(answered(
            number % 60,
            &"words ".repeat(50),
            &[&format!("/home/ada/weather/file{number}.c")],
        ));
    }
    lines.push(typed(59, &"x".repeat(50_000)));
    disk.write(FOLDER, "long", &lines);

    let conversation = disk.read(&format!("{FOLDER}/long"));

    assert_eq!(conversation.requests.len(), 3001);
    assert_eq!(conversation.requests[2999], "request 2999");
    assert_eq!(
        conversation.requests[3000].chars().count(),
        REQUEST_CHARACTERS_READ
    );
    assert_eq!(conversation.changed_files.len(), MOST_FILES_READ);
}

#[test]
fn a_file_named_twice_is_listed_once() {
    let disk = disk();
    disk.write(
        FOLDER,
        "twice",
        &[
            typed(0, "Go"),
            answered(1, "One.", &["/a.c", "/b.c"]),
            answered(2, "Two.", &["/a.c"]),
        ],
    );

    assert_eq!(
        disk.read(&format!("{FOLDER}/twice")).changed_files,
        ["/a.c", "/b.c"]
    );
}

#[test]
fn the_list_shows_folder_first_request_moments_title_and_size() {
    let disk = disk();
    let title = serde_json::json!({ "type": "ai-title", "aiTitle": "Weather station" });
    let custom = serde_json::json!({ "type": "custom-title", "customTitle": "My station" });
    disk.write(
        FOLDER,
        "listed",
        &[
            serde_json::json!({ "type": "queue-operation", "operation": "enqueue" }).to_string(),
            typed(0, "Build a weather station"),
            answered(1, "Plan.", &[]),
            title.to_string(),
            custom.to_string(),
            typed(7, "More"),
        ],
    );
    disk.write(
        "-home-ada-other",
        "elsewhere",
        &[typed(4, "Something else")],
    );

    let mut listed = disk.reader().list().unwrap();
    listed.sort_by(|one, other| one.source.cmp(&other.source));

    assert_eq!(listed.len(), 2);
    let found = &listed[1];
    assert_eq!(found.source, format!("{FOLDER}/listed"));
    assert_eq!(found.held_in, "/home/ada/weather");
    assert_eq!(
        found.first_request.as_deref(),
        Some("Build a weather station")
    );
    assert_eq!(found.title.as_deref(), Some("My station"));
    assert_eq!(found.started_at, Some(at(0, 0)));
    assert_eq!(found.ended_at, Some(at(7, 0)));
    assert!(found.size_bytes > 0);
}

#[test]
fn the_list_finds_the_end_of_a_large_file_from_its_tail() {
    let disk = disk();
    let mut lines = vec![typed(0, "Start")];
    let mut filler = String::new();
    while filler.len() < 400 * 1024 {
        let _ = write!(filler, "padding ");
    }
    lines.push(answered(1, &filler, &[]));
    lines.push(answered(2, &filler, &[]));
    lines.push(typed(45, "The end"));
    disk.write(FOLDER, "large", &lines);

    let listed = disk.reader().list().unwrap();

    assert_eq!(listed[0].first_request.as_deref(), Some("Start"));
    assert_eq!(listed[0].ended_at, Some(at(45, 0)));
}

#[test]
fn files_that_are_not_conversations_are_not_listed() {
    let disk = disk();
    let folder = disk.projects.join(FOLDER);
    fs::write(folder.join("notes.txt"), "hello").unwrap();
    fs::create_dir_all(folder.join("session-folder")).unwrap();
    fs::write(disk.projects.join("loose.jsonl"), typed(0, "x")).unwrap();

    assert_eq!(disk.reader().list().unwrap(), []);
}

#[test]
fn a_source_never_leads_out_of_the_projects_folder() {
    let disk = disk();
    disk.write(FOLDER, "real", &[typed(0, "Go")]);
    let reader = disk.reader();

    for source in [
        "../projects/-home-ada-weather/real",
        "-home-ada-weather/../real",
        "-home-ada-weather/..",
        "..//real",
        "/etc/passwd",
        "-home-ada-weather",
        "-home-ada-weather/sub/real",
        "-home-ada-weather\\real",
        "C:/real",
        "",
    ] {
        assert_eq!(reader.read(source).unwrap(), None, "{source}");
    }
    assert!(reader.read(&format!("{FOLDER}/real")).unwrap().is_some());
}

/// Reads the real conversations of this computer, to see that the reader
/// copes with what Claude Code really wrote and how long it takes. It
/// prints counts and times only, never what a conversation holds. Run it
/// with `cargo test -p teamwright-runtime --lib -- --ignored
/// reads_the_conversations_of_this_computer --nocapture`.
#[test]
#[ignore = "reads the conversations of the user of this computer"]
fn reads_the_conversations_of_this_computer() {
    let reader = EarlierConversationsOnDisk::of_this_computer();
    let started = std::time::Instant::now();
    let listed = reader.list().unwrap();
    let listing = started.elapsed();
    let with_request = listed
        .iter()
        .filter(|found| found.first_request.is_some())
        .count();
    println!(
        "listed {} conversations in {listing:?}, {with_request} with a first request",
        listed.len()
    );
    if let Some(largest) = listed.iter().max_by_key(|found| found.size_bytes) {
        let started = std::time::Instant::now();
        let read = reader.read(&largest.source).unwrap().unwrap();
        println!(
            "read the largest, {} bytes, in {:?}: {} requests, {} files, an answer: {}",
            largest.size_bytes,
            started.elapsed(),
            read.requests.len(),
            read.changed_files.len(),
            read.last_answer.is_some()
        );
    }
}
