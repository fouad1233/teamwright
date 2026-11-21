//! Reads the list of releases from a listener started on this computer, as
//! GitHub would answer it. Nothing leaves the computer.

#![allow(
    clippy::unwrap_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

use teamwright_application::ReleaseSource;
use teamwright_integrations::GitHubReleases;

/// Starts a listener that takes one request, answers with `status` and
/// `body`, and hands over the request line and the headers it got.
fn listen_once(status: u16, body: &'static str) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end().to_owned();
            if line.is_empty() {
                break;
            }
            lines.push(line);
        }
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 {status} Answer\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();
        lines
    });
    (address, handle)
}

#[test]
fn the_list_is_asked_for_without_anything_of_the_manager_and_read() {
    let (api, listener) = listen_once(
        200,
        r#"[{"tag_name":"v0.2.0","prerelease":true,"draft":false},{"tag_name":"v0.1.0","prerelease":true}]"#,
    );
    let source = GitHubReleases::of_repository(&api, "fouad1233/teamwright");

    let releases = source.published_releases().unwrap();

    assert_eq!(releases.len(), 2);
    assert_eq!(releases[0].version.text(), "0.2.0");
    let request = listener.join().unwrap();
    assert_eq!(
        request[0],
        "GET /repos/fouad1233/teamwright/releases?per_page=30 HTTP/1.1"
    );
    let names: Vec<String> = request[1..]
        .iter()
        .map(|line| line.split(':').next().unwrap().to_ascii_lowercase())
        .collect();
    // Only these: no token, no cookie, nothing that says who asks.
    for name in &names {
        assert!(
            ["host", "accept", "user-agent", "accept-encoding"].contains(&name.as_str()),
            "unexpected header {name}"
        );
    }
    let agent = request
        .iter()
        .find(|line| line.to_ascii_lowercase().starts_with("user-agent:"))
        .unwrap();
    assert!(agent.contains("Teamwright/0.1.0"), "{agent}");
}

#[test]
fn an_answer_with_another_status_is_only_a_failure_to_look() {
    let (api, listener) = listen_once(403, r#"{"message":"API rate limit exceeded"}"#);
    let source = GitHubReleases::of_repository(&api, "fouad1233/teamwright");
    let failure = source.published_releases().unwrap_err();
    assert_eq!(failure, "the list answered with status 403");
    listener.join().unwrap();
}

#[test]
fn nobody_listening_is_only_a_failure_to_look() {
    // A port that was free a moment ago and is closed now.
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let source = GitHubReleases::of_repository(&address, "fouad1233/teamwright");
    assert!(source.published_releases().is_err());
}

#[test]
fn an_answer_that_is_not_a_list_is_only_a_failure_to_look() {
    let (api, listener) = listen_once(200, "<html>a page of a hotel network</html>");
    let source = GitHubReleases::of_repository(&api, "fouad1233/teamwright");
    assert!(source.published_releases().is_err());
    listener.join().unwrap();
}
