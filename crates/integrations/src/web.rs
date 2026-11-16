//! Sends to web addresses: one line to a Slack channel, and events as
//! JSON in the shape documented in `docs/integrations/webhooks.md`.

use std::time::Duration;

use chrono::SecondsFormat;
use serde_json::{Value, json};
use teamwright_application::{
    Agent, EventPayload, PAYLOAD_VERSION, StatusOfEvent, TaskOfEvent, WebSender,
};
use teamwright_domain::WebAddress;

use crate::signature::{hex, hmac_sha256};

/// How long one sending may take, from the first try to reach the address
/// to the end of its answer.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The header that says what sent the body.
pub const SENDER_HEADER: &str = "X-Teamwright-Event";
/// The header that carries the version of the shape.
pub const VERSION_HEADER: &str = "X-Teamwright-Version";
/// The header that carries the signature, `sha256=` and the hexadecimal
/// HMAC-SHA256 of the body with the secret.
pub const SIGNATURE_HEADER: &str = "X-Teamwright-Signature";

/// Sends over HTTPS with a blocking client and a time limit.
#[derive(Debug, Default, Clone)]
pub struct HttpsSender;

fn status_of(status: Option<&str>) -> Value {
    status.map_or(Value::Null, |name| Value::String(name.to_owned()))
}

fn status_json(status: Option<&StatusOfEvent>) -> Value {
    status.map_or(
        Value::Null,
        |status| json!({ "name": status.name, "category": status.category.as_str() }),
    )
}

fn task_json(task: &TaskOfEvent) -> Value {
    json!({
        "id": task.task_id.to_string(),
        "key": task.reference,
        "title": task.title,
        "status": status_json(task.status.as_ref()),
        "assignee": task.assignee,
    })
}

fn agent_json(agent: &Agent) -> Value {
    match agent {
        Agent::Specialist { member_id, name } => json!({
            "kind": "specialist",
            "id": member_id.map(|id| id.to_string()),
            "name": name,
        }),
        Agent::Advisor => json!({ "kind": "advisor", "id": null, "name": agent.name() }),
    }
}

/// The JSON a web address gets of an event. Every field is always there;
/// one that does not apply is null.
#[must_use]
pub fn payload_json(payload: &EventPayload) -> Value {
    json!({
        "version": PAYLOAD_VERSION,
        "event": payload.event,
        "at": payload.at.to_rfc3339_opts(SecondsFormat::Millis, true),
        "project": {
            "id": payload.project_id.to_string(),
            "key": payload.project_key,
            "name": payload.project_name,
        },
        "task": payload.task.as_ref().map_or(Value::Null, task_json),
        "agent": payload.agent.as_ref().map_or(Value::Null, agent_json),
        "from_status": status_json(payload.from_status.as_ref()),
        "waits_for": status_of(payload.waits_for),
        "question": payload.question,
        "reason": payload.reason,
        "sprint": payload.sprint.as_ref().map_or(Value::Null, |sprint| json!({
            "id": sprint.sprint_id.to_string(),
            "name": sprint.name,
            "goal": sprint.goal,
            "unit": sprint.unit.map(|unit| unit.as_str()),
            "committed": sprint.committed,
            "completed": sprint.completed,
        })),
    })
}

/// The value of the signature header for a body and a secret.
#[must_use]
pub fn signature_of(body: &[u8], secret: &str) -> String {
    format!("sha256={}", hex(&hmac_sha256(secret.as_bytes(), body)))
}

/// Posts a body to an address and answers with the status of the answer.
/// The address is any address: the rule that it is https belongs to the
/// address the manager sets, so that a test can listen on this computer.
///
/// # Errors
///
/// Why nothing was answered, in words that do not repeat the address.
pub fn post(address: &str, body: &str, headers: &[(&str, String)]) -> Result<u16, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIME_LIMIT))
        .http_status_as_error(false)
        .user_agent(concat!("Teamwright/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut request = agent
        .post(address)
        .header("Content-Type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    match request.send(body) {
        Ok(response) => Ok(response.status().as_u16()),
        Err(ureq::Error::Timeout(_)) => Err(format!(
            "the address did not answer within {} seconds",
            TIME_LIMIT.as_secs()
        )),
        Err(ureq::Error::HostNotFound) => Err("the host of the address was not found".to_owned()),
        // The other errors say nothing the manager can act on, and some
        // repeat the address, which may be a secret.
        Err(_) => Err("the address could not be reached".to_owned()),
    }
}

/// The body of a line for a Slack channel. Slack reads `&`, `<` and `>` as
/// the start of a mention or a link, so they are written the way it asks,
/// and a title such as `<b>` stays text.
#[must_use]
pub fn channel_body(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    json!({ "text": escaped }).to_string()
}

/// The body and the headers of an event for an address with a secret.
#[must_use]
pub fn event_request(
    payload: &EventPayload,
    secret: Option<&str>,
) -> (String, Vec<(&'static str, String)>) {
    let body = payload_json(payload).to_string();
    let mut headers = vec![
        (SENDER_HEADER, payload.event.to_owned()),
        (VERSION_HEADER, PAYLOAD_VERSION.to_string()),
    ];
    if let Some(secret) = secret {
        headers.push((SIGNATURE_HEADER, signature_of(body.as_bytes(), secret)));
    }
    (body, headers)
}

impl WebSender for HttpsSender {
    fn send_to_channel(&self, address: &WebAddress, text: &str) -> Result<u16, String> {
        post(address.as_str(), &channel_body(text), &[])
    }

    fn send_event(
        &self,
        address: &WebAddress,
        secret: Option<&str>,
        payload: &EventPayload,
    ) -> Result<u16, String> {
        let (body, headers) = event_request(payload, secret);
        post(address.as_str(), &body, &headers)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_application::test_payload;
    use teamwright_domain::ProjectId;

    use super::*;

    #[test]
    fn a_test_event_has_every_field_and_nulls_where_nothing_applies() {
        let payload = test_payload(
            ProjectId::generate(),
            "SH",
            "Smart sensor node",
            chrono::Utc
                .with_ymd_and_hms(2026, 11, 16, 9, 30, 0)
                .unwrap(),
        );
        let json = payload_json(&payload);
        assert_eq!(json["version"], 1);
        assert_eq!(json["event"], "test");
        assert_eq!(json["at"], "2026-11-16T09:30:00.000Z");
        assert_eq!(json["project"]["key"], "SH");
        for field in [
            "task",
            "agent",
            "from_status",
            "waits_for",
            "question",
            "reason",
            "sprint",
        ] {
            assert!(json[field].is_null(), "{field}");
        }
    }

    #[test]
    fn a_line_for_a_channel_keeps_its_marks_as_text() {
        assert_eq!(
            channel_body("SH-4 <b> & \"more\""),
            r#"{"text":"SH-4 &lt;b&gt; &amp; \"more\""}"#
        );
    }

    #[test]
    fn a_signed_request_carries_the_signature_of_its_body() {
        let payload = test_payload(ProjectId::generate(), "SH", "Node", chrono::Utc::now());
        let (body, headers) = event_request(&payload, Some("s3cret"));
        let signature = headers
            .iter()
            .find(|(name, _)| *name == SIGNATURE_HEADER)
            .map(|(_, value)| value.clone())
            .unwrap();
        assert_eq!(signature, signature_of(body.as_bytes(), "s3cret"));
        assert!(signature.starts_with("sha256=") && signature.len() == 7 + 64);
        let (_, unsigned) = event_request(&payload, None);
        assert!(unsigned.iter().all(|(name, _)| *name != SIGNATURE_HEADER));
    }
}
