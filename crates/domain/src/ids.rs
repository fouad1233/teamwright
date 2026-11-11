//! Identifiers.
//!
//! Every kind of entity has its own identifier type, so a task identifier can
//! never be passed where a project identifier is expected. The compiler
//! catches the mix-up; no test has to.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::error::DomainError;

/// Declares an identifier type backed by a time-ordered UUID (version 7).
macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, kind = $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new, unique identifier.
            ///
            /// Identifiers created later sort after identifiers created
            /// earlier, so sorting by identifier is sorting by age.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }

            /// Wraps an existing UUID, for example one read from storage.
            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the underlying UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = DomainError;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(text).map(Self).map_err(|_| DomainError::InvalidIdentifier {
                    kind: $kind,
                    value: text.to_owned(),
                })
            }
        }
    };
}

define_id!(
    /// Identifies a project.
    ProjectId,
    kind = "project identifier"
);

define_id!(
    /// Identifies a status of a workflow.
    StatusId,
    kind = "status identifier"
);

define_id!(
    /// Identifies a task.
    TaskId,
    kind = "task identifier"
);

define_id!(
    /// Identifies a member of a project's team: an AI agent or a human teammate.
    MemberId,
    kind = "team member identifier"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_identifiers_are_unique() {
        assert_ne!(TaskId::generate(), TaskId::generate());
    }

    #[test]
    fn later_identifiers_sort_after_earlier_ones() {
        let earlier = TaskId::generate();
        let later = TaskId::generate();

        assert!(earlier < later);
    }

    #[test]
    fn an_identifier_survives_being_written_as_text_and_read_back() {
        let original = ProjectId::generate();

        let read_back: ProjectId = original.to_string().parse().unwrap();

        assert_eq!(read_back, original);
    }

    #[test]
    fn text_that_is_not_a_uuid_is_rejected_with_the_kind_of_identifier() {
        let error = "not-an-id".parse::<TaskId>().unwrap_err();

        assert_eq!(
            error,
            DomainError::InvalidIdentifier {
                kind: "task identifier",
                value: "not-an-id".to_owned()
            }
        );
    }
}
