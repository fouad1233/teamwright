//! The named colors of the interface.

use crate::macros::text_enum;

/// A color, by name.
///
/// A status has one, and so has every member of a team. The domain only says
/// which color. The interface decides the exact shade for its light and its
/// dark theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    /// A neutral gray, usual for work that has not started.
    Gray,
    /// Blue, usual for work under way.
    Blue,
    /// Purple.
    Purple,
    /// Amber, usual for work that waits for a check.
    Amber,
    /// Green, usual for finished work.
    Green,
    /// Red.
    Red,
    /// Pink.
    Pink,
    /// Teal.
    Teal,
}

text_enum!(Color, kind = "color", {
    Gray => "gray",
    Blue => "blue",
    Purple => "purple",
    Amber => "amber",
    Green => "green",
    Red => "red",
    Pink => "pink",
    Teal => "teal",
});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DomainError;

    #[test]
    fn every_color_survives_being_written_as_text_and_read_back() {
        for color in Color::ALL {
            assert_eq!(color.as_str().parse::<Color>().unwrap(), *color);
        }
    }

    #[test]
    fn a_color_that_does_not_exist_is_rejected_by_name() {
        let error = "mauve".parse::<Color>().unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownValue {
                kind: "color",
                value: "mauve".to_owned()
            }
        );
    }
}
