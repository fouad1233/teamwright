//! Sends to a listener started on this computer, as a web address would
//! get it: the headers, the body, the signature. Nothing leaves the
//! computer.

#![allow(
    clippy::unwrap_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread;

use teamwright_application::test_payload;
use teamwright_domain::ProjectId;
use teamwright_integrations::{SIGNATURE_HEADER, event_request, post, signature_of};

/// What the listener got: its headers, in small letters, and its body.
struct Received {
    headers: Vec<(String, String)>,
    body: String,
}

/// Starts a listener that takes one request, answers with `status`, and
/// hands over what it got.
fn listen_once(status: u16) -> (String, thread::JoinHandle<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/hook", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut headers = Vec::new();
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
        }
        let length: usize = headers
            .iter()
            .find(|(name, _)| name == "content-length")
            .map_or(0, |(_, value)| value.parse().unwrap());
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 {status} Answer\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream.flush().unwrap();
        Received {
            headers,
            body: String::from_utf8(body).unwrap(),
        }
    });
    (address, handle)
}

fn header<'received>(received: &'received Received, name: &str) -> Option<&'received str> {
    received
        .headers
        .iter()
        .find(|(each, _)| each == name)
        .map(|(_, value)| value.as_str())
}

#[test]
fn a_signed_event_reaches_a_listener_as_documented() {
    let (address, listener) = listen_once(200);
    let payload = test_payload(
        ProjectId::generate(),
        "SH",
        "Smart sensor node",
        chrono::Utc::now(),
    );
    let (body, headers) = event_request(&payload, Some("s3cret"));

    assert_eq!(post(&address, &body, &headers), Ok(200));

    let received = listener.join().unwrap();
    assert_eq!(received.body, body);
    assert_eq!(header(&received, "content-type"), Some("application/json"));
    assert_eq!(header(&received, "x-teamwright-event"), Some("test"));
    assert_eq!(header(&received, "x-teamwright-version"), Some("1"));
    assert_eq!(
        header(&received, &SIGNATURE_HEADER.to_ascii_lowercase()),
        Some(signature_of(received.body.as_bytes(), "s3cret").as_str())
    );
    let json: serde_json::Value = serde_json::from_str(&received.body).unwrap();
    assert_eq!(json["project"]["name"], "Smart sensor node");
}

#[test]
fn an_answer_that_is_not_a_success_is_its_status_and_no_listener_is_a_failure() {
    let (address, listener) = listen_once(410);
    assert_eq!(post(&address, "{}", &[]), Ok(410));
    listener.join().unwrap();

    // Nothing listens on a port that was just let go.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let refused = post(&format!("http://127.0.0.1:{port}/hook"), "{}", &[]).unwrap_err();
    assert_eq!(refused, "the address could not be reached");
}
