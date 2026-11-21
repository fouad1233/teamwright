//! Stores the libraries of skills and connectors of the projects, and the
//! kits of the specialists (version 27,
//! [decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! Every write is logged: a skill as the entity `skill`, a connector as
//! `connector` and a kit as `member_kit`, each with its whole state, and
//! each merges whole. No secret is ever written: a connector names the
//! environment variables that hold its secrets.

use std::sync::Arc;

use rusqlite::{Connection, OptionalExtension, Row, Transaction, named_params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use teamwright_application::{KitRepository, StorageError};
use teamwright_domain::{
    Connector, ConnectorId, ConnectorName, ConnectorSnapshot, ConnectorStart, DomainError,
    GivenConnector, MemberId, MemberKit, ProjectId, SecretHeader, Skill, SkillId, SkillName,
    SkillSnapshot, ToolName, VariableName,
};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

const SKILL_COLUMNS: &str = "id, project_id, name, description, added_at";
const CONNECTOR_COLUMNS: &str = "id, project_id, name, start, tools, added_at";
const KIT_COLUMNS: &str = "id, project_id, skills, connectors";

// The rows are updated in place, never replaced.
const UPSERT_SKILL: &str = "
    INSERT INTO skills (id, project_id, name, description, added_at)
    VALUES (:id, :project_id, :name, :description, :added_at)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        description = excluded.description,
        added_at = excluded.added_at";

const UPSERT_CONNECTOR: &str = "
    INSERT INTO connectors (id, project_id, name, start, tools, added_at)
    VALUES (:id, :project_id, :name, :start, :tools, :added_at)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        start = excluded.start,
        tools = excluded.tools,
        added_at = excluded.added_at";

const UPSERT_KIT: &str = "
    INSERT INTO member_kits (id, project_id, skills, connectors)
    VALUES (:id, :project_id, :skills, :connectors)
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        skills = excluded.skills,
        connectors = excluded.connectors";

/// A skill as the database and the log keep it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillRecord {
    id: String,
    project_id: String,
    name: String,
    description: String,
    added_at: String,
}

impl SkillRecord {
    fn of(skill: &Skill) -> Self {
        Self {
            id: skill.id().to_string(),
            project_id: skill.project_id().to_string(),
            name: skill.name().to_string(),
            description: skill.description().to_owned(),
            added_at: codec::moment_to_text(skill.added_at()),
        }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            added_at: row.get("added_at")?,
        })
    }

    fn into_skill(self) -> Result<Skill, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("skill", &self.id, &reason);
        Skill::restore(SkillSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: SkillName::parse(&self.name).map_err(broken)?,
            description: self.description.clone(),
            added_at: codec::moment_from_text(&self.added_at)?,
        })
        .map_err(broken)
    }
}

/// How a connector is started, as the log keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StartRecord {
    Program {
        program: String,
        arguments: Vec<String>,
        variables: Vec<String>,
    },
    Address {
        url: String,
        headers: Vec<HeaderRecord>,
    },
}

/// A header of a connector at a web address, as the log keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeaderRecord {
    name: String,
    variable: String,
    bearer: bool,
}

impl StartRecord {
    fn of(start: &ConnectorStart) -> Self {
        match start {
            ConnectorStart::Program {
                program,
                arguments,
                variables,
            } => Self::Program {
                program: program.clone(),
                arguments: arguments.clone(),
                variables: variables.iter().map(ToString::to_string).collect(),
            },
            ConnectorStart::Address { url, headers } => Self::Address {
                url: url.clone(),
                headers: headers
                    .iter()
                    .map(|header| HeaderRecord {
                        name: header.name().to_owned(),
                        variable: header.variable().to_string(),
                        bearer: header.bearer(),
                    })
                    .collect(),
            },
        }
    }

    fn into_start(self) -> Result<ConnectorStart, DomainError> {
        match self {
            Self::Program {
                program,
                arguments,
                variables,
            } => ConnectorStart::program(
                &program,
                arguments,
                variables
                    .iter()
                    .map(|name| VariableName::parse(name))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Address { url, headers } => ConnectorStart::address(
                &url,
                headers
                    .iter()
                    .map(|header| {
                        SecretHeader::new(
                            &header.name,
                            VariableName::parse(&header.variable)?,
                            header.bearer,
                        )
                    })
                    .collect::<Result<_, _>>()?,
            ),
        }
    }
}

/// A connector as the log keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectorRecord {
    id: String,
    project_id: String,
    name: String,
    start: StartRecord,
    tools: Vec<String>,
    added_at: String,
}

impl ConnectorRecord {
    fn of(connector: &Connector) -> Self {
        Self {
            id: connector.id().to_string(),
            project_id: connector.project_id().to_string(),
            name: connector.name().to_string(),
            start: StartRecord::of(connector.start()),
            tools: connector.tools().iter().map(ToString::to_string).collect(),
            added_at: codec::moment_to_text(connector.added_at()),
        }
    }

    fn from_row(row: &Row<'_>) -> Result<Self, DatabaseError> {
        let id: String = row.get("id")?;
        let start: String = row.get("start")?;
        let tools: String = row.get("tools")?;
        Ok(Self {
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            start: serde_json::from_str(&start)
                .map_err(|reason| DatabaseError::damaged("the start of connector", &id, reason))?,
            tools: serde_json::from_str(&tools)
                .map_err(|reason| DatabaseError::damaged("the tools of connector", &id, reason))?,
            added_at: row.get("added_at")?,
            id,
        })
    }

    fn into_connector(self) -> Result<Connector, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("connector", &self.id, &reason);
        Connector::restore(ConnectorSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: ConnectorName::parse(&self.name).map_err(broken)?,
            start: self.start.clone().into_start().map_err(broken)?,
            tools: self
                .tools
                .iter()
                .map(|tool| ToolName::parse(tool))
                .collect::<Result<_, _>>()
                .map_err(broken)?,
            added_at: codec::moment_from_text(&self.added_at)?,
        })
        .map_err(broken)
    }
}

/// A connector given to a specialist, as the log keeps it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GivenRecord {
    connector_id: String,
    tools: Vec<String>,
}

/// A kit as the log keeps it. Its identifier is the member's.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct KitRecord {
    id: String,
    project_id: String,
    skills: Vec<String>,
    connectors: Vec<GivenRecord>,
}

impl KitRecord {
    fn of(kit: &MemberKit) -> Self {
        Self {
            id: kit.member_id().to_string(),
            project_id: kit.project_id().to_string(),
            skills: kit.skills().iter().map(ToString::to_string).collect(),
            connectors: kit
                .connectors()
                .iter()
                .map(|given| GivenRecord {
                    connector_id: given.connector_id().to_string(),
                    tools: given.tools().iter().map(ToString::to_string).collect(),
                })
                .collect(),
        }
    }

    fn from_row(row: &Row<'_>) -> Result<Self, DatabaseError> {
        let id: String = row.get("id")?;
        let skills: String = row.get("skills")?;
        let connectors: String = row.get("connectors")?;
        Ok(Self {
            project_id: row.get("project_id")?,
            skills: serde_json::from_str(&skills)
                .map_err(|reason| DatabaseError::damaged("the skills of kit", &id, reason))?,
            connectors: serde_json::from_str(&connectors)
                .map_err(|reason| DatabaseError::damaged("the connectors of kit", &id, reason))?,
            id,
        })
    }

    fn into_kit(self) -> Result<MemberKit, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("kit", &self.id, &reason);
        let member_id: MemberId = codec::parse(&self.id)?;
        let project_id: ProjectId = codec::parse(&self.project_id)?;
        let skills = self
            .skills
            .iter()
            .map(|id| codec::parse::<SkillId>(id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut connectors = Vec::new();
        for given in &self.connectors {
            let tools = given
                .tools
                .iter()
                .map(|tool| ToolName::parse(tool))
                .collect::<Result<_, _>>()
                .map_err(broken)?;
            connectors.push(
                GivenConnector::new(codec::parse::<ConnectorId>(&given.connector_id)?, tools)
                    .map_err(broken)?,
            );
        }
        MemberKit::new(member_id, project_id, skills, connectors).map_err(broken)
    }
}

/// Stores the libraries and the kits in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteKitRepository {
    database: Arc<Database>,
}

impl SqliteKitRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl KitRepository for SqliteKitRepository {
    fn skills_of_project(&self, project_id: ProjectId) -> Result<Vec<Skill>, StorageError> {
        self.database.read(|connection| {
            let query =
                format!("SELECT {SKILL_COLUMNS} FROM skills WHERE project_id = :project_id");
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    SkillRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;
            records.into_iter().map(SkillRecord::into_skill).collect()
        })
    }

    fn connectors_of_project(&self, project_id: ProjectId) -> Result<Vec<Connector>, StorageError> {
        self.database.read(|connection| {
            let query = format!(
                "SELECT {CONNECTOR_COLUMNS} FROM connectors WHERE project_id = :project_id"
            );
            let mut statement = connection.prepare(&query)?;
            let mut rows =
                statement.query(named_params! { ":project_id": project_id.to_string() })?;
            let mut connectors = Vec::new();
            while let Some(row) = rows.next()? {
                connectors.push(ConnectorRecord::from_row(row)?.into_connector()?);
            }
            Ok(connectors)
        })
    }

    fn kits_of_project(&self, project_id: ProjectId) -> Result<Vec<MemberKit>, StorageError> {
        self.database.read(|connection| {
            let query =
                format!("SELECT {KIT_COLUMNS} FROM member_kits WHERE project_id = :project_id");
            let mut statement = connection.prepare(&query)?;
            let mut rows =
                statement.query(named_params! { ":project_id": project_id.to_string() })?;
            let mut kits = Vec::new();
            while let Some(row) = rows.next()? {
                kits.push(KitRecord::from_row(row)?.into_kit()?);
            }
            Ok(kits)
        })
    }

    fn kit_of(&self, member_id: MemberId) -> Result<Option<MemberKit>, StorageError> {
        self.database
            .read(|connection| kit_of(connection, &member_id.to_string()))
    }

    fn save_skill(&self, skill: &Skill) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| store_skill(transaction, operations, skill))
    }

    fn save_connector(&self, connector: &Connector) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| store_connector(transaction, operations, connector))
    }

    fn save_kit(&self, kit: &MemberKit) -> Result<(), StorageError> {
        self.database
            .write(|transaction, operations| store_kit(transaction, operations, kit))
    }

    fn remove_skill(&self, id: SkillId, kits: &[MemberKit]) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            for kit in kits {
                store_kit(transaction, operations, kit)?;
            }
            let id = id.to_string();
            if remove_row(transaction, "skills", &id)? {
                operations.record_delete(Entity::Skill, &id)?;
            }
            Ok(())
        })
    }

    fn remove_connector(&self, id: ConnectorId, kits: &[MemberKit]) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            for kit in kits {
                store_kit(transaction, operations, kit)?;
            }
            let id = id.to_string();
            if remove_row(transaction, "connectors", &id)? {
                operations.record_delete(Entity::Connector, &id)?;
            }
            Ok(())
        })
    }
}

fn kit_of(connection: &Connection, member_id: &str) -> Result<Option<MemberKit>, DatabaseError> {
    let query = format!("SELECT {KIT_COLUMNS} FROM member_kits WHERE id = :id");
    connection
        .query_row(&query, named_params! { ":id": member_id }, |row| {
            Ok(KitRecord::from_row(row))
        })
        .optional()?
        .transpose()?
        .map(KitRecord::into_kit)
        .transpose()
}

fn store_skill(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    skill: &Skill,
) -> Result<(), DatabaseError> {
    let record = SkillRecord::of(skill);
    write_skill(transaction, &record)?;
    operations.record_upsert(Entity::Skill, &record.id, &record)
}

fn store_connector(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    connector: &Connector,
) -> Result<(), DatabaseError> {
    let record = ConnectorRecord::of(connector);
    write_connector(transaction, &record)?;
    operations.record_upsert(Entity::Connector, &record.id, &record)
}

fn store_kit(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    kit: &MemberKit,
) -> Result<(), DatabaseError> {
    let record = KitRecord::of(kit);
    write_kit(transaction, &record)?;
    operations.record_upsert(Entity::MemberKit, &record.id, &record)
}

fn write_skill(transaction: &Transaction<'_>, record: &SkillRecord) -> Result<(), DatabaseError> {
    transaction.execute(
        UPSERT_SKILL,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":description": record.description,
            ":added_at": record.added_at,
        },
    )?;
    Ok(())
}

fn write_connector(
    transaction: &Transaction<'_>,
    record: &ConnectorRecord,
) -> Result<(), DatabaseError> {
    let start = serde_json::to_string(&record.start)
        .map_err(|reason| DatabaseError::damaged("the start of connector", &record.id, reason))?;
    let tools = serde_json::to_string(&record.tools)
        .map_err(|reason| DatabaseError::damaged("the tools of connector", &record.id, reason))?;
    transaction.execute(
        UPSERT_CONNECTOR,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":name": record.name,
            ":start": start,
            ":tools": tools,
            ":added_at": record.added_at,
        },
    )?;
    Ok(())
}

fn write_kit(transaction: &Transaction<'_>, record: &KitRecord) -> Result<(), DatabaseError> {
    let skills = serde_json::to_string(&record.skills)
        .map_err(|reason| DatabaseError::damaged("the skills of kit", &record.id, reason))?;
    let connectors = serde_json::to_string(&record.connectors)
        .map_err(|reason| DatabaseError::damaged("the connectors of kit", &record.id, reason))?;
    transaction.execute(
        UPSERT_KIT,
        named_params! {
            ":id": record.id,
            ":project_id": record.project_id,
            ":skills": skills,
            ":connectors": connectors,
        },
    )?;
    Ok(())
}

/// Removes a row of one of the three tables, without logging it, and says
/// whether there was one.
fn remove_row(transaction: &Transaction<'_>, table: &str, id: &str) -> Result<bool, DatabaseError> {
    let removed = transaction.execute(
        &format!("DELETE FROM {table} WHERE id = :id"),
        named_params! { ":id": id },
    )?;
    Ok(removed > 0)
}

/// Writes the row of a skill from a state the log keeps, without logging.
/// The state is checked against the rules of the domain first.
pub(crate) fn write_skill_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: SkillRecord = codec::record_from_state("skill", state)?;
    write_skill(transaction, &SkillRecord::of(&record.into_skill()?))
}

/// Writes the row of a connector from a state the log keeps, without
/// logging. The state is checked against the rules of the domain first.
pub(crate) fn write_connector_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: ConnectorRecord = codec::record_from_state("connector", state)?;
    write_connector(transaction, &ConnectorRecord::of(&record.into_connector()?))
}

/// Writes the row of a kit from a state the log keeps, without logging.
/// The state is checked against the rules of the domain first.
pub(crate) fn write_kit_state(
    transaction: &Transaction<'_>,
    state: &Map<String, Value>,
) -> Result<(), DatabaseError> {
    let record: KitRecord = codec::record_from_state("member_kit", state)?;
    write_kit(transaction, &KitRecord::of(&record.into_kit()?))
}

/// Removes the row of a skill, without logging it.
pub(crate) fn remove_skill_row(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<(), DatabaseError> {
    remove_row(transaction, "skills", id).map(|_| ())
}

/// Removes the row of a connector, without logging it.
pub(crate) fn remove_connector_row(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<(), DatabaseError> {
    remove_row(transaction, "connectors", id).map(|_| ())
}

/// Removes the row of a kit, without logging it.
pub(crate) fn remove_kit_row(transaction: &Transaction<'_>, id: &str) -> Result<(), DatabaseError> {
    remove_row(transaction, "member_kits", id).map(|_| ())
}

#[cfg(test)]
mod tests;
