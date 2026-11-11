//! Stores the members of the teams.

use std::sync::Arc;

use rusqlite::{OptionalExtension, Row, named_params};
use serde::Serialize;
use teamwright_application::{MemberRepository, StorageError};
use teamwright_domain::{DomainError, Member, MemberId, MemberName, MemberSnapshot, ProjectId};

use crate::codec;
use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::Entity;

/// The columns of the `members` table that every query reads.
const MEMBER_COLUMNS: &str =
    "id, project_id, name, kind, responsibility, color, joined_at, updated_at";

// The row is updated in place, like every row of this database.
const UPSERT_MEMBER: &str = "
    INSERT INTO members (
        id, project_id, name, kind, responsibility, color, joined_at, updated_at
    )
    VALUES (
        :id, :project_id, :name, :kind, :responsibility, :color, :joined_at, :updated_at
    )
    ON CONFLICT (id) DO UPDATE SET
        project_id = excluded.project_id,
        name = excluded.name,
        kind = excluded.kind,
        responsibility = excluded.responsibility,
        color = excluded.color,
        joined_at = excluded.joined_at,
        updated_at = excluded.updated_at";

/// A member as the database and the operation log keep it: plain text.
#[derive(Debug, Serialize)]
struct MemberRecord {
    id: String,
    project_id: String,
    name: String,
    kind: String,
    responsibility: String,
    color: String,
    joined_at: String,
    updated_at: String,
}

impl MemberRecord {
    /// Takes the stored form of a member.
    fn of(member: &Member) -> Self {
        Self {
            id: member.id().to_string(),
            project_id: member.project_id().to_string(),
            name: member.name().to_string(),
            kind: member.kind().to_string(),
            responsibility: member.responsibility().to_owned(),
            color: member.color().to_string(),
            joined_at: codec::moment_to_text(member.joined_at()),
            updated_at: codec::moment_to_text(member.updated_at()),
        }
    }

    /// Reads the columns of a `members` row.
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            project_id: row.get("project_id")?,
            name: row.get("name")?,
            kind: row.get("kind")?,
            responsibility: row.get("responsibility")?,
            color: row.get("color")?,
            joined_at: row.get("joined_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Rebuilds the member, checking the stored values against the rules.
    fn into_member(self) -> Result<Member, DatabaseError> {
        let broken = |reason: DomainError| codec::damaged_record("team member", &self.id, &reason);

        Ok(Member::restore(MemberSnapshot {
            id: codec::parse(&self.id)?,
            project_id: codec::parse(&self.project_id)?,
            name: MemberName::parse(&self.name).map_err(broken)?,
            kind: codec::parse(&self.kind)?,
            responsibility: self.responsibility,
            color: codec::parse(&self.color)?,
            joined_at: codec::moment_from_text(&self.joined_at)?,
            updated_at: codec::moment_from_text(&self.updated_at)?,
        }))
    }
}

/// Stores the members of the teams in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteMemberRepository {
    database: Arc<Database>,
}

impl SqliteMemberRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl MemberRepository for SqliteMemberRepository {
    fn save(&self, member: &Member) -> Result<(), StorageError> {
        let record = MemberRecord::of(member);
        self.database.write(|transaction, operations| {
            transaction.execute(
                UPSERT_MEMBER,
                named_params! {
                    ":id": record.id,
                    ":project_id": record.project_id,
                    ":name": record.name,
                    ":kind": record.kind,
                    ":responsibility": record.responsibility,
                    ":color": record.color,
                    ":joined_at": record.joined_at,
                    ":updated_at": record.updated_at,
                },
            )?;
            operations.record_upsert(Entity::Member, &record.id, &record)
        })
    }

    fn find(&self, id: MemberId) -> Result<Option<Member>, StorageError> {
        self.database.read(|connection| {
            let query = format!("SELECT {MEMBER_COLUMNS} FROM members WHERE id = :id");
            connection
                .query_row(
                    &query,
                    named_params! { ":id": id.to_string() },
                    MemberRecord::from_row,
                )
                .optional()?
                .map(MemberRecord::into_member)
                .transpose()
        })
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Member>, StorageError> {
        self.database.read(|connection| {
            let query =
                format!("SELECT {MEMBER_COLUMNS} FROM members WHERE project_id = :project_id");
            let mut statement = connection.prepare(&query)?;
            let records = statement
                .query_map(
                    named_params! { ":project_id": project_id.to_string() },
                    MemberRecord::from_row,
                )?
                .collect::<Result<Vec<_>, _>>()?;

            records.into_iter().map(MemberRecord::into_member).collect()
        })
    }

    fn delete(&self, id: MemberId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            let id = id.to_string();
            let rows_removed = transaction.execute(
                "DELETE FROM members WHERE id = :id",
                named_params! { ":id": id },
            )?;
            if rows_removed > 0 {
                operations.record_delete(Entity::Member, &id)?;
            }
            Ok(())
        })
    }
}
