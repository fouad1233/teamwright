//! Stores the custom fields of the projects with their choices
//! ([decision 0035](../../../docs/decisions/0035-custom-fields.md)). The
//! values the tasks hold are stored with each task (see `tasks`); how a
//! value is written down is here.

use std::collections::BTreeMap;
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use teamwright_application::{FieldRepository, StorageError};
use teamwright_domain::{
    ChoiceName, DomainError, Field, FieldChoice, FieldId, FieldName, FieldNumber, FieldSnapshot,
    FieldText, FieldUnit, FieldValue, ProjectId, Rank,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::merge::MergedState;
use crate::operations::{Entity, OperationRecorder};

/// The start of the key of a choice in the logged state of a field:
/// `choice:<identifier of the choice>`, the choice as an object, or null
/// once it was removed.
pub(crate) const CHOICE_KEY: &str = "choice:";

/// The columns of the `fields` table that every query reads.
const FIELD_COLUMNS: &str = "id, project_id, name, kind, unit, position";

// The row is updated in place, never replaced: replacing it would clear its
// values from the tasks through the cascade of `task_field_values`.
const UPSERT_FIELD: &str = "
    INSERT INTO fields (id, project_id, name, kind, unit, position)
    VALUES (:id, :project_id, :name, :kind, :unit, :position)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        kind = excluded.kind,
        unit = excluded.unit,
        position = excluded.position";

/// A field as the database and the operation log keep it: plain text and
/// numbers, and one key for each choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldRecord {
    id: String,
    project_id: String,
    name: String,
    kind: String,
    unit: Option<String>,
    position: u32,
    /// The choices, one key each: see [`CHOICE_KEY`]. Read from a state,
    /// this also holds the keys a newer version wrote, which are passed
    /// over.
    #[serde(flatten)]
    parts: BTreeMap<String, Value>,
}

/// A choice as the operation log keeps it, under its key in the state of
/// its field.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChoiceRecord {
    name: String,
    color: String,
    rank: f64,
}

impl FieldRecord {
    /// Takes the stored form of a field.
    fn of(field: &Field) -> Self {
        Self {
            id: field.id().to_string(),
            project_id: field.project_id().to_string(),
            name: field.name().to_string(),
            kind: field.kind().to_string(),
            unit: field.unit().map(ToString::to_string),
            position: field.position(),
            parts: field
                .choices()
                .iter()
                .map(|choice| {
                    (
                        format!("{CHOICE_KEY}{}", choice.id()),
                        choice_value(
                            choice.name().as_str(),
                            choice.color().as_str(),
                            choice.rank().value(),
                        ),
                    )
                })
                .collect(),
        }
    }

    /// Reads the columns of a `fields` row. The choices live in another
    /// table and are added by the caller.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            kind: row.get("kind")?,
            unit: row.get("unit")?,
            position: row.get("position")?,
            parts: BTreeMap::new(),
        })
    }

    /// Rebuilds the field, checking the stored values against the rules.
    fn into_field(self) -> Result<Field, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("field", &self.id, &reason);
        let mut choices = Vec::new();
        for (key, value) in &self.parts {
            let Some(choice_id) = key.strip_prefix(CHOICE_KEY) else {
                continue;
            };
            if value.is_null() {
                continue;
            }
            let choice: ChoiceRecord = serde_json::from_value(value.clone()).map_err(|reason| {
                DatabaseError::damaged("a choice of a field", &value.to_string(), reason)
            })?;
            choices.push(FieldChoice::restore(
                codec::parse(choice_id)?,
                ChoiceName::parse(&choice.name).map_err(broken)?,
                codec::parse(&choice.color)?,
                Rank::from_value(choice.rank),
            ));
        }
        Ok(Field::restore(FieldSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: FieldName::parse(&self.name).map_err(broken)?,
            kind: codec::parse(&self.kind)?,
            unit: self
                .unit
                .as_deref()
                .map(FieldUnit::parse)
                .transpose()
                .map_err(broken)?,
            position: self.position,
            choices,
        }))
    }
}

/// A choice as the log keeps it.
fn choice_value(name: &str, color: &str, rank: f64) -> Value {
    json!({ "name": name, "color": color, "rank": rank })
}

/// A value of a field as the log and the table `task_field_values` keep
/// it: an object with one key that says its kind.
pub(crate) fn value_to_json(value: &FieldValue) -> Value {
    match value {
        FieldValue::Text(text) => json!({ "text": text.as_str() }),
        FieldValue::Number(number) => json!({ "number": number.value() }),
        FieldValue::Date(date) => json!({ "date": codec::date_to_text(*date) }),
        FieldValue::YesNo(yes) => json!({ "yes": yes }),
        FieldValue::Choice(choice) => json!({ "choice": choice.to_string() }),
        FieldValue::Link(address) => json!({ "link": address.as_str() }),
    }
}

/// Reads a value of a field as [`value_to_json`] writes it, checking it
/// against the rules of the domain.
pub(crate) fn value_from_json(value: &Value) -> Result<FieldValue, DatabaseError> {
    let damaged = || {
        DatabaseError::damaged(
            "a value of a field",
            &value.to_string(),
            "it is not one of the kinds of value",
        )
    };
    let broken = |reason: DomainError| {
        DatabaseError::damaged("a value of a field", &value.to_string(), reason)
    };
    let Some((kind, inner)) = value.as_object().and_then(|object| object.iter().next()) else {
        return Err(damaged());
    };
    match (kind.as_str(), inner) {
        ("text", Value::String(text)) => {
            FieldText::parse(text).map(FieldValue::Text).map_err(broken)
        }
        ("number", Value::Number(number)) => number
            .as_f64()
            .ok_or_else(damaged)
            .and_then(|number| FieldNumber::new(number).map_err(broken))
            .map(FieldValue::Number),
        ("date", Value::String(date)) => Ok(FieldValue::Date(codec::date_from_text(date)?)),
        ("yes", Value::Bool(yes)) => Ok(FieldValue::YesNo(*yes)),
        ("choice", Value::String(choice)) => Ok(FieldValue::Choice(codec::parse(choice)?)),
        ("link", Value::String(address)) => FieldValue::link(address).map_err(broken),
        _ => Err(damaged()),
    }
}

/// Stores fields in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteFieldRepository {
    database: Arc<Database>,
}

impl SqliteFieldRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl FieldRepository for SqliteFieldRepository {
    fn save_all(&self, fields: &[Field]) -> Result<(), StorageError> {
        if fields.is_empty() {
            return Ok(());
        }
        self.database.write(|transaction, operations| {
            for field in fields {
                store_field(transaction, operations, field)?;
            }
            Ok(())
        })
    }

    fn find(&self, id: FieldId) -> Result<Option<Field>, StorageError> {
        self.database
            .read(|connection| find_field(connection, &id.to_string()))
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Field>, StorageError> {
        self.database
            .read(|connection| fields_of_project(connection, &project_id.to_string()))
    }

    fn delete(&self, id: FieldId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            if remove_field(transaction, &id)? {
                operations.record_delete(Entity::Field, &id)?;
            }
            Ok(())
        })
    }
}

/// Reads one field inside a transaction.
pub(crate) fn find_field(
    connection: &Connection,
    id: &str,
) -> Result<Option<Field>, DatabaseError> {
    let query = format!("SELECT {FIELD_COLUMNS} FROM fields WHERE id = :id");
    let Some(mut record) = connection
        .query_row(&query, named_params! { ":id": id }, FieldRecord::from_row)
        .optional()?
    else {
        return Ok(None);
    };
    record.parts = choices_in(connection, "field.id = :id", id)?
        .remove(&record.id)
        .unwrap_or_default();
    record.into_field().map(Some)
}

/// Reads the fields of a project inside a transaction, in no particular
/// order.
pub(crate) fn fields_of_project(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<Field>, DatabaseError> {
    let mut choices = choices_in(connection, "field.project_id = :id", project_id)?;
    let query = format!("SELECT {FIELD_COLUMNS} FROM fields WHERE project_id = :project_id");
    let mut statement = connection.prepare(&query)?;
    let records = statement
        .query_map(
            named_params! { ":project_id": project_id },
            FieldRecord::from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;
    records
        .into_iter()
        .map(|mut record| {
            record.parts = choices.remove(&record.id).unwrap_or_default();
            record.into_field()
        })
        .collect()
}

/// Writes one field with its choices and logs the change.
///
/// A choice the field no longer has is logged as such (null): a key left
/// out of a logged state keeps its value, so that each choice merges on its
/// own.
pub(crate) fn store_field(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    field: &Field,
) -> Result<(), DatabaseError> {
    let mut record = FieldRecord::of(field);
    write_rows(transaction, &record)?;
    if let Some(state) =
        MergedState::read(transaction, Entity::Field, &record.id)?.and_then(|merged| merged.state)
    {
        for (key, value) in state {
            if key.starts_with(CHOICE_KEY) && !value.is_null() && !record.parts.contains_key(&key) {
                record.parts.insert(key, Value::Null);
            }
        }
    }
    operations.record_upsert(Entity::Field, &record.id, &record)
}

/// Removes the row of a field, without logging it, and says whether there
/// was one. Its choices go with it, and so do the values the tasks still
/// hold of it.
pub(crate) fn remove_field(transaction: &Transaction<'_>, id: &str) -> Result<bool, DatabaseError> {
    let rows_removed = transaction.execute(
        "DELETE FROM fields WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(rows_removed > 0)
}

/// Rebuilds a field from a state the log keeps, checking it against the
/// rules of the domain.
pub(crate) fn field_from_state(state: &Map<String, Value>) -> Result<Field, DatabaseError> {
    let record: FieldRecord = codec::record_from_state("field", state)?;
    record.into_field()
}

/// Writes the rows of a field from a state the log keeps, without logging
/// anything. The state is first checked against the rules of the domain: a
/// state that breaks one is refused, and nothing is written.
pub(crate) fn write_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let field = field_from_state(state)?;
    write_rows(transaction, &FieldRecord::of(&field))
}

/// Writes the rows of a field and of its choices. Rows are added and
/// updated in place, and the rows of a choice removed are deleted.
fn write_rows(transaction: &Transaction<'_>, record: &FieldRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_FIELD,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":kind": record.kind,
            ":unit": record.unit,
            ":position": record.position,
        },
    )?;
    let mut kept = Vec::new();
    for (key, value) in &record.parts {
        let Some(choice_id) = key.strip_prefix(CHOICE_KEY) else {
            continue;
        };
        if value.is_null() {
            continue;
        }
        let choice: ChoiceRecord = serde_json::from_value(value.clone()).map_err(|reason| {
            DatabaseError::damaged("a choice of a field", &value.to_string(), reason)
        })?;
        transaction.execute(
            "INSERT INTO field_choices (id, field_id, name, color, rank)
             VALUES (:id, :field_id, :name, :color, :rank)
             ON CONFLICT (id) DO UPDATE SET
                 field_id = excluded.field_id,
                 name = excluded.name,
                 color = excluded.color,
                 rank = excluded.rank",
            named_params! {
                ":id": choice_id,
                ":field_id": record.id,
                ":name": choice.name,
                ":color": choice.color,
                ":rank": choice.rank,
            },
        )?;
        kept.push(choice_id.to_owned());
    }
    transaction.execute(
        "DELETE FROM field_choices WHERE field_id = :field_id
           AND id NOT IN (SELECT value FROM json_each(:kept))",
        named_params! { ":field_id": record.id, ":kept": Value::from(kept).to_string() },
    )?;
    Ok(())
}

/// Reads, for every field a condition on `field` picks, the keys of its
/// choices, as the log keeps them. The condition names one parameter,
/// `:id`.
fn choices_in(
    connection: &Connection,
    condition: &str,
    id: &str,
) -> Result<BTreeMap<String, BTreeMap<String, Value>>, DatabaseError> {
    let query = format!(
        "SELECT choice.field_id, choice.id, choice.name, choice.color, choice.rank
         FROM field_choices AS choice
         JOIN fields AS field ON field.id = choice.field_id
         WHERE {condition}"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(named_params! { ":id": id }, |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;
    let mut choices: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    for row in rows {
        let (field_id, choice_id, name, color, rank) = row?;
        choices.entry(field_id).or_default().insert(
            format!("{CHOICE_KEY}{choice_id}"),
            choice_value(&name, &color, rank),
        );
    }
    Ok(choices)
}

#[cfg(test)]
mod tests {
    use teamwright_domain::{ChoiceId, FieldKind};

    use super::*;

    #[test]
    fn every_kind_of_value_is_written_and_read_back() {
        let values = [
            FieldValue::Text(FieldText::parse("Two reels").unwrap()),
            FieldValue::Number(FieldNumber::new(-12.25).unwrap()),
            FieldValue::Date(teamwright_domain::CalendarDate::from_ymd_opt(2026, 11, 30).unwrap()),
            FieldValue::YesNo(false),
            FieldValue::Choice(ChoiceId::generate()),
            FieldValue::link("https://www.ti.com/lit/ds/tps62840.pdf").unwrap(),
        ];
        for value in values {
            let written = value_to_json(&value);
            assert_eq!(value_from_json(&written).unwrap(), value, "{written}");
        }
    }

    #[test]
    fn a_value_that_breaks_a_rule_or_is_of_no_kind_is_refused() {
        for wrong in [
            json!({ "link": "javascript:alert(1)" }),
            json!({ "number": "12" }),
            json!({ "colour": "red" }),
            json!("text"),
            json!({ "text": "" }),
        ] {
            assert!(value_from_json(&wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_field_reads_back_from_its_logged_state_with_its_choices_in_order() {
        let mut field = Field::add(
            ProjectId::generate(),
            FieldName::parse("Supplier").unwrap(),
            FieldKind::Choice,
            2,
        );
        for name in ["Mouser", "LCSC", "Digi-Key"] {
            field
                .add_choice(ChoiceName::parse(name).unwrap(), None)
                .unwrap();
        }
        let state = match serde_json::to_value(FieldRecord::of(&field)).unwrap() {
            Value::Object(state) => state,
            other => panic!("{other}"),
        };

        assert_eq!(field_from_state(&state).unwrap(), field);
    }
}
