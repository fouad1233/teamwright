//! How the state of each kind of thing merges when two computers changed it
//! ([decision 0017](../../../docs/decisions/0017-sync-through-a-folder.md),
//! section 3).
//!
//! A logged state is a JSON object. Its keys are grouped into fields that
//! merge on their own: the keys of one field always travel together, so a
//! rule between them (a status and the moment of completion, a start and a
//! due date) cannot break. A key that no field names is a field of its own,
//! which is how a key written by a newer version of the program is kept.
//!
//! Every field keeps the stamp of the write that last set it. The merged
//! state of each thing, with those stamps, is kept in the table
//! `merged_states`; [`MergedState`] is one row of it.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, named_params};
use serde_json::{Map, Value};

use crate::error::DatabaseError;
use crate::operations::Entity;

/// Which of two values a field keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rule {
    /// The value with the later stamp wins.
    Later,
    /// The larger value wins, whatever its stamp: a counter that only grows,
    /// or the moment of the last change.
    Larger,
}

/// A field that merges on its own: one or several keys of the logged state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Field {
    /// The name of the field, under which its stamp is kept.
    pub(crate) name: &'static str,
    /// The keys of the logged state that belong to it.
    pub(crate) keys: &'static [&'static str],
    /// Which of two values it keeps.
    pub(crate) rule: Rule,
}

/// How the states of one kind of thing merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Merging {
    /// Field by field. Keys that no field names are fields of their own.
    ByField(&'static [Field]),
    /// The whole state is one field: the later one wins. Used for the
    /// things only one computer writes, the turns and their parts.
    Whole,
    /// The state never leaves this computer, and a state from elsewhere is
    /// never applied. On this computer it is kept like a whole.
    NeverLeaves,
}

/// The name of the one field of a thing that merges whole.
pub(crate) const WHOLE: &str = "whole";

/// Declares a field whose value with the later stamp wins.
const fn later(name: &'static str, keys: &'static [&'static str]) -> Field {
    Field {
        name,
        keys,
        rule: Rule::Later,
    }
}

/// Declares a field whose larger value wins.
const fn larger(name: &'static str, keys: &'static [&'static str]) -> Field {
    Field {
        name,
        keys,
        rule: Rule::Larger,
    }
}

const PROJECT_FIELDS: &[Field] = &[
    later("key", &["key"]),
    later("name", &["name"]),
    later("description", &["description"]),
    later("archived", &["archived_at"]),
    // All statuses together: a workflow is one list, whose order and
    // whose "done" status are rules of the whole list.
    later("workflow", &["statuses"]),
    larger("next_task_number", &["next_task_number"]),
    larger("next_sprint_number", &["next_sprint_number"]),
];

const TASK_FIELDS: &[Field] = &[
    later("title", &["title"]),
    later("description", &["description"]),
    later("status", &["status_id", "completed_at"]),
    later("priority", &["priority"]),
    later("assignee", &["assignee_kind", "assignee_member_id"]),
    later("parent", &["parent_id"]),
    later("waits_for", &["blocked_by"]),
    later("estimate", &["estimate_minutes"]),
    later("schedule", &["start_on", "due_on"]),
    later("rank", &["rank"]),
    later("sprint", &["sprint_id"]),
    later("points", &["points"]),
    later("number", &["number"]),
    larger("updated_at", &["updated_at"]),
];

const MEMBER_FIELDS: &[Field] = &[
    later("name", &["name"]),
    later("responsibility", &["responsibility"]),
    later("profile", &["model", "effort"]),
    later("color", &["color"]),
    larger("updated_at", &["updated_at"]),
];

const SPRINT_FIELDS: &[Field] = &[
    later("name", &["name"]),
    later("goal", &["goal"]),
    later("dates", &["start_on", "end_on"]),
    // The state with its moments and its summary: a completed sprint has
    // all of them, a planned one none.
    later("state", &["state", "started_at", "completed_at", "summary"]),
];

const CONVERSATION_FIELDS: &[Field] = &[
    later("member", &["member_id"]),
    later("task", &["task_id"]),
    later("session", &["session_id"]),
    later(
        "advisor",
        &["with_advisor", "advisor_model", "advisor_effort"],
    ),
    later("home_computer", &["home_computer"]),
];

/// How the states of a kind of thing merge. A new kind of thing does not
/// compile until it says so here.
pub(crate) const fn merging_of(entity: Entity) -> Merging {
    match entity {
        Entity::Project => Merging::ByField(PROJECT_FIELDS),
        Entity::Task => Merging::ByField(TASK_FIELDS),
        Entity::Member => Merging::ByField(MEMBER_FIELDS),
        Entity::Sprint => Merging::ByField(SPRINT_FIELDS),
        Entity::Conversation => Merging::ByField(CONVERSATION_FIELDS),
        Entity::Turn
        | Entity::TurnStep
        | Entity::Decision
        | Entity::Interview
        | Entity::Proposal => Merging::Whole,
        Entity::ProjectPlaces => Merging::NeverLeaves,
    }
}

/// The fields of one state of a thing, each with the keys of the state that
/// belong to it. The fields the merging names come first, then one for each
/// key no field names.
pub(crate) fn fields_of(entity: Entity, keys: &[&str]) -> Vec<(String, Vec<String>, Rule)> {
    match merging_of(entity) {
        Merging::Whole | Merging::NeverLeaves => vec![(
            WHOLE.to_owned(),
            keys.iter().map(|key| (*key).to_owned()).collect(),
            Rule::Later,
        )],
        Merging::ByField(fields) => {
            let mut found: Vec<(String, Vec<String>, Rule)> = fields
                .iter()
                .map(|field| {
                    (
                        field.name.to_owned(),
                        field.keys.iter().map(|key| (*key).to_owned()).collect(),
                        field.rule,
                    )
                })
                .collect();
            for key in keys {
                let named = fields.iter().any(|field| field.keys.contains(key));
                if !named {
                    found.push(((*key).to_owned(), vec![(*key).to_owned()], Rule::Later));
                }
            }
            found
        }
    }
}

/// The stamps of the fields of a state, by the name of the field.
pub(crate) type Stamps = BTreeMap<String, String>;

/// The merged state of one thing, with the stamp of each of its fields, as
/// the table `merged_states` keeps it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MergedState {
    /// The merged state, including keys this version does not know. `None`
    /// for a thing that is only known by its deletion.
    pub(crate) state: Option<Map<String, Value>>,
    /// The stamp of every field of the state, by the name of the field.
    pub(crate) stamps: Stamps,
    /// The stamp of the deletion, if the thing is deleted.
    pub(crate) deleted: Option<String>,
    /// For each computer, by its identity, the latest change of the thing
    /// it had seen when it deleted the thing.
    pub(crate) deleted_seen: Stamps,
}

impl MergedState {
    /// The merged state of a thing that was just created with this state.
    pub(crate) fn created(entity: Entity, state: Map<String, Value>, stamp: &str) -> Self {
        let stamps = all_fields(entity, &state)
            .into_iter()
            .map(|(name, _, _)| (name, stamp.to_owned()))
            .collect();
        Self {
            state: Some(state),
            stamps,
            deleted: None,
            deleted_seen: Stamps::new(),
        }
    }

    /// The latest stamp of any field: the stamp of the last change.
    pub(crate) fn latest_stamp(&self) -> Option<&str> {
        self.stamps.values().map(String::as_str).max()
    }

    /// Reads the merged state of a thing, if it has one.
    pub(crate) fn read(
        connection: &Connection,
        entity: Entity,
        entity_id: &str,
    ) -> Result<Option<Self>, DatabaseError> {
        let row: Option<RawRow> = connection
            .query_row(
                "SELECT state, base_stamp, field_stamps, deleted_stamp, deleted_seen
                 FROM merged_states WHERE entity = :entity AND entity_id = :entity_id",
                named_params! { ":entity": entity.as_str(), ":entity_id": entity_id },
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((state, base_stamp, field_stamps, deleted, deleted_seen)) = row else {
            return Ok(None);
        };
        let state = state
            .as_deref()
            .map(|text| object_from_text(text, "a merged state"))
            .transpose()?;
        let older = field_stamps
            .as_deref()
            .map(stamps_from_text)
            .transpose()?
            .unwrap_or_default();
        let stamps = match &state {
            None => Stamps::new(),
            Some(state) => all_fields(entity, state)
                .into_iter()
                .map(|(name, _, _)| {
                    let stamp = older
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| base_stamp.clone());
                    (name, stamp)
                })
                .collect(),
        };
        Ok(Some(Self {
            state,
            stamps,
            deleted,
            deleted_seen: deleted_seen
                .as_deref()
                .map(stamps_from_text)
                .transpose()?
                .unwrap_or_default(),
        }))
    }

    /// Writes the merged state of a thing, replacing what was kept for it.
    ///
    /// The stamps are kept compactly: the latest one as the base, and only
    /// the fields whose stamp is older listed by name. Two computers that
    /// agree on a thing keep the very same row for it.
    pub(crate) fn write(
        &self,
        connection: &Connection,
        entity: Entity,
        entity_id: &str,
    ) -> Result<(), DatabaseError> {
        let base = self
            .latest_stamp()
            .or(self.deleted.as_deref())
            .unwrap_or_default()
            .to_owned();
        let older: Stamps = self
            .stamps
            .iter()
            .filter(|(_, stamp)| **stamp != base)
            .map(|(name, stamp)| (name.clone(), stamp.clone()))
            .collect();
        connection.execute(
            "INSERT INTO merged_states (
                 entity, entity_id, state, base_stamp, field_stamps, deleted_stamp, deleted_seen
             )
             VALUES (
                 :entity, :entity_id, :state, :base_stamp, :field_stamps, :deleted_stamp,
                 :deleted_seen
             )
             ON CONFLICT (entity, entity_id) DO UPDATE SET
                 state = excluded.state,
                 base_stamp = excluded.base_stamp,
                 field_stamps = excluded.field_stamps,
                 deleted_stamp = excluded.deleted_stamp,
                 deleted_seen = excluded.deleted_seen",
            named_params! {
                ":entity": entity.as_str(),
                ":entity_id": entity_id,
                ":state": self.state.as_ref().map(|state| Value::Object(state.clone()).to_string()),
                ":base_stamp": base,
                ":field_stamps": stamps_to_text(&older),
                ":deleted_stamp": self.deleted,
                ":deleted_seen": stamps_to_text(&self.deleted_seen),
            },
        )?;
        Ok(())
    }
}

/// A row of `merged_states` as it is read.
type RawRow = (
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Every field of a state: those of its kind that it has a key of, and one
/// for each key no field names.
pub(crate) fn all_fields(
    entity: Entity,
    state: &Map<String, Value>,
) -> Vec<(String, Vec<String>, Rule)> {
    let keys: Vec<&str> = state.keys().map(String::as_str).collect();
    fields_of(entity, &keys)
        .into_iter()
        .filter(|(_, field_keys, _)| field_keys.iter().any(|key| state.contains_key(key)))
        .collect()
}

/// Whether two states have the same value in the keys of a field. A key
/// that is missing counts as empty.
pub(crate) fn same_value(
    one: &Map<String, Value>,
    other: &Map<String, Value>,
    keys: &[String],
) -> bool {
    keys.iter()
        .all(|key| one.get(key).unwrap_or(&Value::Null) == other.get(key).unwrap_or(&Value::Null))
}

/// Writes stamps as the JSON the database keeps, or nothing if there are
/// none.
pub(crate) fn stamps_to_text(stamps: &Stamps) -> Option<String> {
    if stamps.is_empty() {
        return None;
    }
    let object: Map<String, Value> = stamps
        .iter()
        .map(|(name, stamp)| (name.clone(), Value::String(stamp.clone())))
        .collect();
    Some(Value::Object(object).to_string())
}

/// Reads stamps from the JSON the database keeps.
pub(crate) fn stamps_from_text(text: &str) -> Result<Stamps, DatabaseError> {
    object_from_text(text, "the stamps of the fields")?
        .into_iter()
        .map(|(name, stamp)| match stamp {
            Value::String(stamp) => Ok((name, stamp)),
            other => Err(DatabaseError::damaged(
                "the stamp of a field",
                &other.to_string(),
                "it is not text",
            )),
        })
        .collect()
}

/// Reads a JSON object.
pub(crate) fn object_from_text(
    text: &str,
    what: &str,
) -> Result<Map<String, Value>, DatabaseError> {
    match serde_json::from_str::<Value>(text) {
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(DatabaseError::damaged(
            what,
            text,
            "it is not a JSON object",
        )),
        Err(reason) => Err(DatabaseError::damaged(what, text, reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entity_says_how_it_merges() {
        for entity in Entity::ALL {
            let merging = merging_of(entity);
            if let Merging::ByField(fields) = merging {
                let mut names: Vec<&str> = fields.iter().map(|field| field.name).collect();
                names.sort_unstable();
                names.dedup();
                assert_eq!(names.len(), fields.len(), "{entity:?} names a field twice");
                let mut keys: Vec<&str> = fields
                    .iter()
                    .flat_map(|field| field.keys)
                    .copied()
                    .collect();
                let count = keys.len();
                keys.sort_unstable();
                keys.dedup();
                assert_eq!(keys.len(), count, "{entity:?} puts a key in two fields");
            }
        }
        assert_eq!(merging_of(Entity::ProjectPlaces), Merging::NeverLeaves);
        assert_eq!(merging_of(Entity::Turn), Merging::Whole);
    }

    #[test]
    fn a_key_no_field_names_is_a_field_of_its_own() {
        let fields = fields_of(Entity::Task, &["title", "colour_of_the_card"]);

        assert!(
            fields
                .iter()
                .any(|(name, keys, rule)| name == "colour_of_the_card"
                    && keys == &["colour_of_the_card".to_owned()]
                    && *rule == Rule::Later)
        );
        assert!(fields.iter().any(|(name, keys, _)| name == "status"
            && keys == &["status_id".to_owned(), "completed_at".to_owned()]));
    }

    #[test]
    fn a_turn_merges_whole() {
        let fields = fields_of(Entity::Turn, &["id", "state", "answer"]);

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].0, WHOLE);
        assert_eq!(fields[0].1.len(), 3);
    }
}
