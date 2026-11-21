#![allow(
    clippy::unwrap_used,
    reason = "a failed test is the right outcome of an unexpected value"
)]

use teamwright_domain::{ConnectorStart, ToolName};

use super::*;

fn connector() -> Connector {
    Connector::add(
        ProjectId::generate(),
        ConnectorName::parse("tickets").unwrap(),
        ConnectorStart::address(
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
        vec![ToolName::parse("read_ticket").unwrap()],
        chrono::Utc::now(),
    )
    .unwrap()
}

fn state_of(record: &impl Serialize) -> Map<String, Value> {
    match serde_json::to_value(record) {
        Ok(Value::Object(state)) => state,
        other => panic!("a record is logged as an object, not {other:?}"),
    }
}

#[test]
fn a_connector_is_logged_with_the_name_of_its_variable_and_read_back() {
    let connector = connector();
    let state = state_of(&ConnectorRecord::of(&connector));

    assert_eq!(
        state["start"],
        serde_json::json!({
            "kind": "address",
            "url": "https://tickets.example/mcp",
            "headers": [
                { "name": "Authorization", "variable": "TICKETS_TOKEN", "bearer": true }
            ]
        })
    );
    let read: ConnectorRecord = codec::record_from_state("connector", &state).unwrap();
    let mut read = read.into_connector().unwrap();
    // The moment is kept to the millisecond.
    assert_eq!(read.id(), connector.id());
    assert_eq!(read.start(), connector.start());
    read = Connector::restore(ConnectorSnapshot {
        id: read.id(),
        project_id: read.project_id(),
        name: read.name().clone(),
        start: read.start().clone(),
        tools: read.tools().to_vec(),
        added_at: connector.added_at(),
    })
    .unwrap();
    assert_eq!(read, connector);
}

#[test]
fn a_state_from_elsewhere_that_breaks_a_rule_is_refused() {
    let mut state = state_of(&ConnectorRecord::of(&connector()));
    state.insert(
        "start".to_owned(),
        serde_json::json!({
            "kind": "address",
            "url": "https://tickets.example/mcp?token=hunter2",
            "headers": []
        }),
    );
    let read: ConnectorRecord = codec::record_from_state("connector", &state).unwrap();
    let refusal = read.into_connector().unwrap_err().to_string();
    assert!(!refusal.contains("hunter2"), "{refusal}");

    let mut wholesale = state_of(&ConnectorRecord::of(&connector()));
    wholesale.insert("tools".to_owned(), serde_json::json!([]));
    let read: ConnectorRecord = codec::record_from_state("connector", &wholesale).unwrap();
    assert!(read.into_connector().is_err());
}

#[test]
fn a_kit_is_logged_with_its_tools_and_read_back() {
    let given = GivenConnector::new(
        ConnectorId::generate(),
        vec![ToolName::parse("read_ticket").unwrap()],
    )
    .unwrap();
    let kit = MemberKit::new(
        MemberId::generate(),
        ProjectId::generate(),
        vec![SkillId::generate()],
        vec![given],
    )
    .unwrap();
    let state = state_of(&KitRecord::of(&kit));
    assert_eq!(state["id"], Value::String(kit.member_id().to_string()));
    let read: KitRecord = codec::record_from_state("member_kit", &state).unwrap();
    assert_eq!(read.into_kit().unwrap(), kit);
}
