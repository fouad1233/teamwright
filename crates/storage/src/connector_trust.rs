//! What the manager confirmed on this computer of the connectors of the
//! libraries and of what each specialist is given of them, kept in `meta`
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md),
//! note of 22 November 2026).
//!
//! Like the theme and the choice about notifications it is a fact about
//! this computer: written without the recorder, so it is never logged,
//! never written to the sync folder, and nothing taken in from there can
//! change it. A connector is kept as the fingerprint of the definition the
//! manager confirmed, under `connector_confirmed:<connector>`; the tools a
//! specialist was confirmed as given, under
//! `connector_tools_confirmed:<member>`, as a JSON list of pairs of a
//! connector and a tool.

use std::collections::BTreeSet;
use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, named_params};
use teamwright_application::{ConnectorTrust, StorageError};
use teamwright_domain::{ConnectorId, MemberId, ToolName};

use crate::database::Database;
use crate::error::DatabaseError;

/// The beginning of the key of a confirmed definition.
const DEFINITION: &str = "connector_confirmed:";

/// The beginning of the key of the confirmed tools of a specialist.
const TOOLS: &str = "connector_tools_confirmed:";

/// What the manager confirmed on this computer, in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteConnectorTrust {
    database: Arc<Database>,
}

impl SqliteConnectorTrust {
    /// Creates a store that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

fn meta(connection: &Connection, key: &str) -> Result<Option<String>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT value FROM meta WHERE key = :key",
            named_params! { ":key": key },
            |row| row.get(0),
        )
        .optional()?)
}

fn set_meta(connection: &Connection, key: &str, value: &str) -> Result<(), DatabaseError> {
    connection.execute(
        "INSERT INTO meta (key, value) VALUES (:key, :value)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        named_params! { ":key": key, ":value": value },
    )?;
    Ok(())
}

fn clear_meta(connection: &Connection, key: &str) -> Result<(), DatabaseError> {
    connection.execute(
        "DELETE FROM meta WHERE key = :key",
        named_params! { ":key": key },
    )?;
    Ok(())
}

impl ConnectorTrust for SqliteConnectorTrust {
    fn confirmed_definition(&self, connector: ConnectorId) -> Result<Option<String>, StorageError> {
        self.database
            .read(|connection| meta(connection, &format!("{DEFINITION}{connector}")))
    }

    fn confirm_definition(
        &self,
        connector: ConnectorId,
        fingerprint: &str,
    ) -> Result<(), StorageError> {
        // Written without the recorder: what is confirmed here is not
        // logged.
        self.database.write(|transaction, _| {
            set_meta(
                transaction,
                &format!("{DEFINITION}{connector}"),
                fingerprint,
            )
        })
    }

    fn forget_definition(&self, connector: ConnectorId) -> Result<(), StorageError> {
        self.database
            .write(|transaction, _| clear_meta(transaction, &format!("{DEFINITION}{connector}")))
    }

    fn confirmed_tools(
        &self,
        member: MemberId,
    ) -> Result<BTreeSet<(ConnectorId, ToolName)>, StorageError> {
        let kept = self
            .database
            .read(|connection| meta(connection, &format!("{TOOLS}{member}")))?;
        let Some(kept) = kept else {
            return Ok(BTreeSet::new());
        };
        // What cannot be read confirms nothing: the tools then wait for
        // the manager again.
        let pairs: Vec<(String, String)> = serde_json::from_str(&kept).unwrap_or_default();
        Ok(pairs
            .iter()
            .filter_map(|(connector, tool)| {
                Some((
                    connector.parse::<ConnectorId>().ok()?,
                    ToolName::parse(tool).ok()?,
                ))
            })
            .collect())
    }

    fn confirm_tools(
        &self,
        member: MemberId,
        tools: &BTreeSet<(ConnectorId, ToolName)>,
    ) -> Result<(), StorageError> {
        let pairs: Vec<(String, String)> = tools
            .iter()
            .map(|(connector, tool)| (connector.to_string(), tool.to_string()))
            .collect();
        let value = serde_json::to_string(&pairs)
            .map_err(|error| StorageError::caused_by("the confirmed tools", error))?;
        self.database.write(|transaction, _| {
            let key = format!("{TOOLS}{member}");
            if tools.is_empty() {
                clear_meta(transaction, &key)
            } else {
                set_meta(transaction, &key, &value)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_confirmed_is_kept_and_never_logged() {
        let database = Arc::new(Database::open_in_memory().unwrap());
        let trust = SqliteConnectorTrust::new(database.clone());
        let connector = ConnectorId::generate();
        let member = MemberId::generate();
        assert_eq!(trust.confirmed_definition(connector).unwrap(), None);
        assert!(trust.confirmed_tools(member).unwrap().is_empty());

        trust.confirm_definition(connector, "abc").unwrap();
        assert_eq!(
            trust.confirmed_definition(connector).unwrap().as_deref(),
            Some("abc")
        );
        let tools = BTreeSet::from([
            (connector, ToolName::parse("search_parts").unwrap()),
            (connector, ToolName::parse("order_parts").unwrap()),
        ]);
        trust.confirm_tools(member, &tools).unwrap();
        assert_eq!(trust.confirmed_tools(member).unwrap(), tools);

        trust.forget_definition(connector).unwrap();
        assert_eq!(trust.confirmed_definition(connector).unwrap(), None);
        trust.confirm_tools(member, &BTreeSet::new()).unwrap();
        assert!(trust.confirmed_tools(member).unwrap().is_empty());
        assert!(database.operations_to_send(0).unwrap().is_empty());
    }
}
