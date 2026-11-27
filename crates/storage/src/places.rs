//! Stores the folders chosen for each project, and which projects have full
//! access.
//!
//! The folders are facts about this computer. They are logged like every
//! other change, as the entity `project_places`, but a folder on this
//! computer means nothing on another one: when computers synchronize, these
//! operations must not be applied elsewhere.
//!
//! Full access is a choice of this computer too, and is not logged at all:
//! it is kept in `meta`, written without the recorder, like the choice about
//! commands (decision 0044).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{OptionalExtension, Transaction, named_params};
use serde::Serialize;
use teamwright_application::{ChosenPlaces, PlacesRepository, StorageError};
use teamwright_domain::ProjectId;

use crate::database::Database;
use crate::error::DatabaseError;
use crate::operations::{Entity, OperationRecorder};

// The row is updated in place, like every row of this database.
const UPSERT_PLACES: &str = "
    INSERT INTO project_places (project_id, work_folder)
    VALUES (:project_id, :work_folder)
    ON CONFLICT (project_id) DO UPDATE SET
        work_folder = excluded.work_folder";

const UPSERT_REFERENCE_FOLDER: &str = "
    INSERT INTO project_reference_folders (project_id, position, folder)
    VALUES (:project_id, :position, :folder)
    ON CONFLICT (project_id, position) DO UPDATE SET
        folder = excluded.folder";

const UPSERT_WRITTEN_FOLDER: &str = "
    INSERT INTO project_written_folders (project_id, position, folder)
    VALUES (:project_id, :position, :folder)
    ON CONFLICT (project_id, position) DO UPDATE SET
        folder = excluded.folder";

/// The key in `meta` of the full access of a project, before its
/// identifier.
const FULL_ACCESS_KEY: &str = "full_access:";

/// The value of full access when switched on. Anything else, or nothing, is
/// off.
const ON: &str = "on";

/// The folders of a project as the database and the operation log keep
/// them: plain text.
#[derive(Debug, Serialize)]
struct PlacesRecord {
    project_id: String,
    /// The work folder the manager chose; `None` for the one Teamwright
    /// gives the project.
    work_folder: Option<String>,
    /// The reference folders, in order.
    reference_folders: Vec<String>,
    /// The folders to write in, in order.
    written_folders: Vec<String>,
}

impl PlacesRecord {
    /// Takes the stored form of the folders of a project.
    fn of(project_id: ProjectId, places: &ChosenPlaces) -> Self {
        Self {
            project_id: project_id.to_string(),
            work_folder: places.work_folder.as_deref().map(path_to_text),
            reference_folders: places
                .reference_folders
                .iter()
                .map(|folder| path_to_text(folder))
                .collect(),
            written_folders: places
                .written_folders
                .iter()
                .map(|folder| path_to_text(folder))
                .collect(),
        }
    }
}

/// The text a path is stored as. The paths come from the interface as text,
/// so nothing is lost.
fn path_to_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Stores the folders chosen for each project in the SQLite database.
#[derive(Debug, Clone)]
pub struct SqlitePlacesRepository {
    database: Arc<Database>,
}

impl SqlitePlacesRepository {
    /// Creates a repository that works on the given database.
    #[must_use]
    pub const fn new(database: Arc<Database>) -> Self {
        Self { database }
    }
}

impl PlacesRepository for SqlitePlacesRepository {
    fn find(&self, project_id: ProjectId) -> Result<ChosenPlaces, StorageError> {
        self.database.read(|connection| {
            let project_id = project_id.to_string();
            let stored: Option<Option<String>> = connection
                .query_row(
                    "SELECT work_folder FROM project_places WHERE project_id = :project_id",
                    named_params! { ":project_id": project_id },
                    |row| row.get(0),
                )
                .optional()?;
            let Some(work_folder) = stored else {
                return Ok(ChosenPlaces::default());
            };
            let folders_of = |table: &str| -> Result<Vec<PathBuf>, rusqlite::Error> {
                connection
                    .prepare(&format!(
                        "SELECT folder FROM {table}
                         WHERE project_id = :project_id
                         ORDER BY position"
                    ))?
                    .query_map(named_params! { ":project_id": project_id }, |row| {
                        row.get::<_, String>(0)
                    })?
                    .map(|folder| folder.map(PathBuf::from))
                    .collect()
            };
            Ok(ChosenPlaces {
                work_folder: work_folder.map(PathBuf::from),
                reference_folders: folders_of("project_reference_folders")?,
                written_folders: folders_of("project_written_folders")?,
            })
        })
    }

    fn save(&self, project_id: ProjectId, places: &ChosenPlaces) -> Result<(), StorageError> {
        let record = PlacesRecord::of(project_id, places);
        self.database.write(|transaction, operations| {
            transaction.execute(
                UPSERT_PLACES,
                named_params! {
                    ":project_id": record.project_id,
                    ":work_folder": record.work_folder,
                },
            )?;
            save_folders(
                transaction,
                &record.project_id,
                &record.reference_folders,
                UPSERT_REFERENCE_FOLDER,
                "project_reference_folders",
            )?;
            save_folders(
                transaction,
                &record.project_id,
                &record.written_folders,
                UPSERT_WRITTEN_FOLDER,
                "project_written_folders",
            )?;
            operations.record_upsert(Entity::ProjectPlaces, &record.project_id, &record)
        })
    }

    fn delete(&self, project_id: ProjectId) -> Result<(), StorageError> {
        self.database.write(|transaction, operations| {
            delete_places_of_project(transaction, operations, &project_id.to_string())
        })
    }

    fn has_full_access(&self, project_id: ProjectId) -> Result<bool, StorageError> {
        self.database.read(|connection| {
            let value: Option<String> = connection
                .query_row(
                    "SELECT value FROM meta WHERE key = :key",
                    named_params! { ":key": full_access_key(&project_id.to_string()) },
                    |row| row.get(0),
                )
                .optional()?;
            Ok(value.as_deref() == Some(ON))
        })
    }

    fn set_full_access(&self, project_id: ProjectId, on: bool) -> Result<(), StorageError> {
        // Written without the recorder: the choice is not logged, so it
        // never reaches the sync folder or another computer.
        self.database.write(|transaction, _| {
            let key = full_access_key(&project_id.to_string());
            if on {
                transaction.execute(
                    "INSERT INTO meta (key, value) VALUES (:key, :value)
                     ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                    named_params! { ":key": key, ":value": ON },
                )?;
            } else {
                transaction.execute(
                    "DELETE FROM meta WHERE key = :key",
                    named_params! { ":key": key },
                )?;
            }
            Ok(())
        })
    }
}

/// The key in `meta` of the full access of a project.
fn full_access_key(project_id: &str) -> String {
    format!("{FULL_ACCESS_KEY}{project_id}")
}

/// Writes one list of folders of a project into its table, position by
/// position, and removes the rows beyond its end.
fn save_folders(
    transaction: &Transaction<'_>,
    project_id: &str,
    folders: &[String],
    upsert: &str,
    table: &str,
) -> Result<(), DatabaseError> {
    for (position, folder) in (0_i64..).zip(folders) {
        transaction.execute(
            upsert,
            named_params! {
                ":project_id": project_id,
                ":position": position,
                ":folder": folder,
            },
        )?;
    }
    let kept = i64::try_from(folders.len())
        .map_err(|_| DatabaseError::Damaged("a project has too many folders".to_owned()))?;
    transaction.execute(
        &format!("DELETE FROM {table} WHERE project_id = :project_id AND position >= :kept"),
        named_params! { ":project_id": project_id, ":kept": kept },
    )?;
    Ok(())
}

/// Removes the folders chosen for a project, and logs the removal if there
/// were any. The folders themselves are not touched.
pub(crate) fn delete_places_of_project(
    transaction: &Transaction<'_>,
    operations: &mut OperationRecorder<'_>,
    project_id: &str,
) -> Result<(), DatabaseError> {
    // The reference folders and the folders to write in go with their row
    // through ON DELETE CASCADE; the logged state of the places contains
    // them, so its deletion covers them.
    let rows_removed = transaction.execute(
        "DELETE FROM project_places WHERE project_id = :project_id",
        named_params! { ":project_id": project_id },
    )?;
    // Full access is not logged, so its removal is not either. A project
    // deleted on another computer loses it here too.
    transaction.execute(
        "DELETE FROM meta WHERE key = :key",
        named_params! { ":key": full_access_key(project_id) },
    )?;
    if rows_removed > 0 {
        operations.record_delete(Entity::ProjectPlaces, project_id)?;
    }
    Ok(())
}
