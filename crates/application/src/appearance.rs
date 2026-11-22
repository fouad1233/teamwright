//! How the window looks on this computer: the theme the manager chose
//! ([decision 0028](../../../docs/decisions/0028-themes.md)). Like the
//! choice about notifications it is a fact about one computer: kept, but
//! not logged and never synced.

use std::sync::Arc;

use crate::error::ApplicationError;
use crate::ports::StorageError;

/// A theme of the window: a set of values for the design tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Night and paper, following the light or dark setting of the system.
    #[default]
    Lamplight,
    /// Light, a warm off-white for a bright room.
    Paper,
    /// Dark, warm and dim, with little blue, for the evening.
    Dusk,
    /// Dark neutral grey with medium contrast, for long sessions.
    Slate,
    /// Light, with the highest contrast.
    Clear,
    /// Dark, with statuses told apart without colour vision.
    Even,
    /// Light, with a pattern of flowers.
    Blossom,
    /// Dark, with a pattern of birds.
    Wing,
    /// Light, with a pattern of leaves.
    Fern,
    /// Dark, with a pattern of waves.
    Tide,
    /// Light, a white ground with pink for the accents.
    Rose,
    /// Light, a white ground with blue for the accents.
    Delft,
    /// Light, a white ground with red for the accents.
    Poppy,
}

impl Theme {
    /// Every theme, the default first.
    pub const ALL: [Self; 13] = [
        Self::Lamplight,
        Self::Paper,
        Self::Dusk,
        Self::Slate,
        Self::Clear,
        Self::Even,
        Self::Blossom,
        Self::Wing,
        Self::Fern,
        Self::Tide,
        Self::Rose,
        Self::Delft,
        Self::Poppy,
    ];

    /// The name the interface and the store use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lamplight => "lamplight",
            Self::Paper => "paper",
            Self::Dusk => "dusk",
            Self::Slate => "slate",
            Self::Clear => "clear",
            Self::Even => "even",
            Self::Blossom => "blossom",
            Self::Wing => "wing",
            Self::Fern => "fern",
            Self::Tide => "tide",
            Self::Rose => "rose",
            Self::Delft => "delft",
            Self::Poppy => "poppy",
        }
    }

    /// The theme of a name, or nothing if no theme has it.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.name() == name)
    }

    /// The theme of a name, or the default one if no theme has it, so that
    /// a name kept by a later version of the program does no harm.
    #[must_use]
    pub fn or_default(name: &str) -> Self {
        Self::parse(name).unwrap_or_default()
    }
}

/// Keeps the theme of this computer.
pub trait ThemeStore: Send + Sync {
    /// The theme, or the default one if the manager never chose one.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme could not be read.
    fn theme(&self) -> Result<Theme, StorageError>;

    /// Keeps a theme in place of the one before.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme could not be kept.
    fn set_theme(&self, theme: Theme) -> Result<(), StorageError>;
}

/// The use cases of the look of the window on this computer.
#[derive(Clone)]
pub struct AppearanceService {
    store: Arc<dyn ThemeStore>,
}

impl AppearanceService {
    /// Creates the service on a store of the theme.
    #[must_use]
    pub fn new(store: Arc<dyn ThemeStore>) -> Self {
        Self { store }
    }

    /// The theme of this computer.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme could not be read.
    pub fn theme(&self) -> Result<Theme, ApplicationError> {
        Ok(self.store.theme()?)
    }

    /// Chooses a theme for this computer, and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error if the theme could not be kept.
    pub fn choose(&self, theme: Theme) -> Result<Theme, ApplicationError> {
        self.store.set_theme(theme)?;
        Ok(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_memory::InMemoryThemeStore;

    #[test]
    fn every_name_reads_back_as_its_theme() {
        for theme in Theme::ALL {
            assert_eq!(Theme::parse(theme.name()), Some(theme));
        }
    }

    #[test]
    fn the_three_colour_and_white_themes_have_their_names() {
        assert_eq!(Theme::ALL.len(), 13);
        assert_eq!(Theme::parse("rose"), Some(Theme::Rose));
        assert_eq!(Theme::parse("delft"), Some(Theme::Delft));
        assert_eq!(Theme::parse("poppy"), Some(Theme::Poppy));
    }

    #[test]
    fn an_unknown_name_falls_back_to_the_default() {
        assert_eq!(Theme::parse("neon"), None);
        assert_eq!(Theme::or_default("neon"), Theme::Lamplight);
        assert_eq!(Theme::or_default(""), Theme::Lamplight);
    }

    #[test]
    fn a_chosen_theme_is_kept() {
        let service = AppearanceService::new(Arc::new(InMemoryThemeStore::default()));
        assert_eq!(service.theme().unwrap(), Theme::Lamplight);
        assert_eq!(service.choose(Theme::Fern).unwrap(), Theme::Fern);
        assert_eq!(service.theme().unwrap(), Theme::Fern);
    }
}
