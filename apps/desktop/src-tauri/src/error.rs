//! The error a command reports to the interface.

use serde::Serialize;
use teamwright_application::ApplicationError;
use teamwright_domain::DomainError;

/// Why a command failed, in a form the interface can show.
///
/// `code` is stable and tells the interface what kind of problem it is.
/// `message` is written for the person using the program.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    /// The interface sent something that cannot be understood, such as a
    /// date that is not a date.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_request",
            message: message.into(),
        }
    }

    /// Something the computer offers could not be used, such as its file
    /// manager.
    pub fn not_available(message: impl Into<String>) -> Self {
        Self {
            code: "not_available",
            message: message.into(),
        }
    }
}

impl From<ApplicationError> for CommandError {
    fn from(error: ApplicationError) -> Self {
        Self {
            code: error.code(),
            message: error.to_string(),
        }
    }
}

impl From<DomainError> for CommandError {
    fn from(error: DomainError) -> Self {
        ApplicationError::from(error).into()
    }
}
