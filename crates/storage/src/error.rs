//! Errors inside the storage crate.

use teamwright_application::StorageError;

/// What can go wrong while working with the database.
///
/// This type stays inside the crate. At the boundary it becomes the
/// [`StorageError`] that the application understands.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DatabaseError {
    /// SQLite refused or failed an operation.
    #[error("the database reported an error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A stored value does not have the form this program writes.
    #[error("stored data is damaged: {0}")]
    Damaged(String),

    /// Something was to be written again that was deleted. A deletion is
    /// final: identifiers are never used twice.
    #[error("the {entity} {id} was deleted and cannot be written again")]
    Deleted {
        /// The kind of thing, as the log names it.
        entity: &'static str,
        /// Its identifier.
        id: String,
    },

    /// The file was written by a newer version of the program.
    #[error(
        "the database has format version {found}, but this program only understands versions \
         up to {supported}; it was probably created by a newer Teamwright"
    )]
    TooNew {
        /// The version found in the file.
        found: i64,
        /// The newest version this program can read.
        supported: usize,
    },
}

impl DatabaseError {
    /// Describes a stored value that could not be understood.
    pub(crate) fn damaged(what: &str, value: &str, reason: impl std::fmt::Display) -> Self {
        Self::Damaged(format!("{what} '{value}' cannot be read: {reason}"))
    }
}

impl From<DatabaseError> for StorageError {
    fn from(error: DatabaseError) -> Self {
        let message = error.to_string();
        match error {
            DatabaseError::Sqlite(source) => Self::caused_by(message, source),
            DatabaseError::Damaged(_)
            | DatabaseError::Deleted { .. }
            | DatabaseError::TooNew { .. } => Self::new(message),
        }
    }
}
