//! The notices syncing keeps for the manager. They describe this computer's
//! view and are not logged.

use chrono::Utc;
use rusqlite::{Connection, named_params};
use teamwright_application::{NoticeKind, SyncNotice};

use crate::codec;
use crate::error::DatabaseError;

/// Keeps a notice.
pub(crate) fn add(
    connection: &Connection,
    kind: NoticeKind,
    project_id: Option<&str>,
    task_id: Option<&str>,
    message: &str,
) -> Result<(), DatabaseError> {
    connection.execute(
        "INSERT INTO notices (at, kind, project_id, task_id, message)
         VALUES (:at, :kind, :project_id, :task_id, :message)",
        named_params! {
            ":at": codec::moment_to_text(Utc::now()),
            ":kind": kind.as_str(),
            ":project_id": project_id,
            ":task_id": task_id,
            ":message": message,
        },
    )?;
    Ok(())
}

/// Every notice that is kept, the newest first.
pub(crate) fn all(connection: &Connection) -> Result<Vec<SyncNotice>, DatabaseError> {
    let mut statement = connection.prepare(
        "SELECT id, at, kind, project_id, task_id, message FROM notices ORDER BY id DESC",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(id, at, kind, project_id, task_id, message)| {
            Ok(SyncNotice {
                id,
                at: codec::moment_from_text(&at)?,
                kind: codec::parse(&kind)?,
                project_id: project_id.as_deref().map(codec::parse).transpose()?,
                task_id: task_id.as_deref().map(codec::parse).transpose()?,
                message,
            })
        })
        .collect()
}

/// Forgets a notice. A notice that is not kept is ignored.
pub(crate) fn dismiss(connection: &Connection, id: i64) -> Result<(), DatabaseError> {
    connection.execute(
        "DELETE FROM notices WHERE id = :id",
        named_params! { ":id": id },
    )?;
    Ok(())
}
