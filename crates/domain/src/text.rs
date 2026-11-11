//! Short texts that name things.
//!
//! Each kind of name is its own type with its own length limit. A status name
//! cannot be used where a task title is expected, and every name in the
//! system is known to be non-empty, on a single line and free of stray
//! whitespace.

use std::fmt;

use crate::error::DomainError;

/// Declares a validated, single-line text type.
macro_rules! define_text {
    ($(#[$meta:meta])* $name:ident, field = $field:literal, max_length = $max_length:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            /// The largest number of characters this text can have.
            pub const MAX_LENGTH: usize = $max_length;

            /// Checks and tidies a text.
            ///
            /// Whitespace at both ends is removed, and line breaks and
            /// repeated spaces inside become single spaces.
            ///
            /// # Errors
            ///
            /// Returns an error if nothing is left after tidying, or if the
            /// text is longer than [`Self::MAX_LENGTH`] characters.
            pub fn parse(raw: &str) -> Result<Self, DomainError> {
                let text = join_words_with_single_spaces(raw);
                if text.is_empty() {
                    return Err(DomainError::EmptyText { field: $field });
                }
                let actual_length = text.chars().count();
                if actual_length > Self::MAX_LENGTH {
                    return Err(DomainError::TextTooLong {
                        field: $field,
                        max_length: Self::MAX_LENGTH,
                        actual_length,
                    });
                }
                Ok(Self(text))
            }

            /// Wraps a text that is written in this crate's own source code
            /// and is known to follow the rules. Tests parse every such text
            /// to prove it.
            #[allow(dead_code)]
            pub(crate) fn from_trusted(text: &'static str) -> Self {
                Self(text.to_owned())
            }

            /// Returns the text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

define_text!(
    /// The name of a project.
    ProjectName,
    field = "project name",
    max_length = 80
);

define_text!(
    /// The name of a status of a workflow, such as "In progress".
    StatusName,
    field = "status name",
    max_length = 40
);

define_text!(
    /// The title of a task.
    TaskTitle,
    field = "task title",
    max_length = 200
);

/// Removes surrounding whitespace and turns every run of whitespace inside
/// the text, line breaks included, into one space.
fn join_words_with_single_spaces(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrounding_whitespace_is_removed() {
        let title = TaskTitle::parse("  Design the power stage \n").unwrap();

        assert_eq!(title.as_str(), "Design the power stage");
    }

    #[test]
    fn line_breaks_and_repeated_spaces_become_single_spaces() {
        let title = TaskTitle::parse("Design\nthe   power\tstage").unwrap();

        assert_eq!(title.as_str(), "Design the power stage");
    }

    #[test]
    fn a_text_of_only_whitespace_is_rejected_as_empty() {
        let error = ProjectName::parse(" \n\t ").unwrap_err();

        assert_eq!(
            error,
            DomainError::EmptyText {
                field: "project name"
            }
        );
    }

    #[test]
    fn a_text_at_the_length_limit_is_accepted() {
        let longest_allowed = "s".repeat(StatusName::MAX_LENGTH);

        assert!(StatusName::parse(&longest_allowed).is_ok());
    }

    #[test]
    fn a_text_over_the_length_limit_is_rejected_with_both_lengths() {
        let too_long = "s".repeat(StatusName::MAX_LENGTH + 1);

        let error = StatusName::parse(&too_long).unwrap_err();

        assert_eq!(
            error,
            DomainError::TextTooLong {
                field: "status name",
                max_length: StatusName::MAX_LENGTH,
                actual_length: StatusName::MAX_LENGTH + 1,
            }
        );
    }

    #[test]
    fn length_is_counted_in_characters_not_bytes() {
        // Each of these letters takes two bytes in UTF-8.
        let longest_allowed = "ü".repeat(StatusName::MAX_LENGTH);

        assert!(StatusName::parse(&longest_allowed).is_ok());
    }
}
