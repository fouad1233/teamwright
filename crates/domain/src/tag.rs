//! The tags of a project: short colored words the manager makes and puts on
//! tasks, such as "EMC" or "Waiting for parts"
//! ([decision 0030](../../../docs/decisions/0030-tags-and-checklists.md)).
//!
//! A tag belongs to one project and has a name and a color. A task carries
//! any number of the tags of its project. The rules here concern one tag,
//! or one name. The rules that look at the other tags of the project, such
//! as "no two tags with one name", are use cases.

use crate::color::Color;
use crate::ids::{ProjectId, TagId};
use crate::text::TagName;

/// The plain data of a tag, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagSnapshot {
    /// The identifier of the tag.
    pub id: TagId,
    /// The project the tag belongs to.
    pub project_id: ProjectId,
    /// Its name.
    pub name: TagName,
    /// Its color.
    pub color: Color,
}

/// A word the manager puts on tasks to group them across statuses and
/// areas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    id: TagId,
    project_id: ProjectId,
    name: TagName,
    color: Color,
}

impl Tag {
    /// The most tags a project may have. The tags are offered as one short
    /// list in the panel of a task; a list longer than this is no longer
    /// read.
    pub const MOST_IN_A_PROJECT: usize = 100;

    /// The colors a new tag gets in turn, when the manager names none. Gray
    /// comes last: it reads as "nothing special".
    pub const COLORS_IN_TURN: [Color; 8] = [
        Color::Blue,
        Color::Green,
        Color::Amber,
        Color::Purple,
        Color::Teal,
        Color::Pink,
        Color::Red,
        Color::Gray,
    ];

    /// Makes a tag in a project.
    #[must_use]
    pub fn add(project_id: ProjectId, name: TagName, color: Color) -> Self {
        Self {
            id: TagId::generate(),
            project_id,
            name,
            color,
        }
    }

    /// The color of a new tag when the manager names none, given how many
    /// tags the project has already: the colors take turns.
    #[must_use]
    pub const fn color_in_turn(tags_so_far: usize) -> Color {
        Self::COLORS_IN_TURN[tags_so_far % Self::COLORS_IN_TURN.len()]
    }

    /// Recreates a tag from stored data.
    #[must_use]
    pub fn restore(snapshot: TagSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            color: snapshot.color,
        }
    }

    /// The identifier of the tag.
    #[must_use]
    pub const fn id(&self) -> TagId {
        self.id
    }

    /// The project the tag belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name of the tag.
    #[must_use]
    pub const fn name(&self) -> &TagName {
        &self.name
    }

    /// The color of the tag.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Gives the tag another name.
    pub fn rename(&mut self, name: TagName) {
        self.name = name;
    }

    /// Gives the tag another color.
    pub const fn recolor(&mut self, color: Color) {
        self.color = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_renamed_and_recolored() {
        let project = ProjectId::generate();
        let mut tag = Tag::add(project, TagName::parse("EMC").unwrap(), Color::Blue);

        tag.rename(TagName::parse("Radio tests").unwrap());
        tag.recolor(Color::Red);

        assert_eq!(tag.name().as_str(), "Radio tests");
        assert_eq!(tag.color(), Color::Red);
        assert_eq!(tag.project_id(), project);
    }

    #[test]
    fn the_colors_of_new_tags_take_turns_and_start_again() {
        assert_eq!(Tag::color_in_turn(0), Color::Blue);
        assert_eq!(Tag::color_in_turn(1), Color::Green);
        assert_eq!(Tag::color_in_turn(7), Color::Gray);
        assert_eq!(Tag::color_in_turn(8), Color::Blue);
        let mut distinct = Tag::COLORS_IN_TURN.to_vec();
        distinct.sort_by_key(|color| color.as_str());
        distinct.dedup();
        assert_eq!(distinct.len(), Color::ALL.len(), "every color is used once");
    }

    #[test]
    fn a_tag_name_is_tidied_and_has_at_most_30_characters() {
        assert_eq!(
            TagName::parse("  Waiting \n for   parts ")
                .unwrap()
                .as_str(),
            "Waiting for parts"
        );
        assert_eq!(
            TagName::parse(" ").unwrap_err().to_string(),
            "the tag name must not be empty"
        );
        assert_eq!(
            TagName::parse(&"x".repeat(31)).unwrap_err().to_string(),
            "the tag name can have at most 30 characters, but this one has 31"
        );
        assert!(TagName::parse(&"x".repeat(30)).is_ok());
    }

    #[test]
    fn two_tag_names_are_the_same_whatever_the_capitals() {
        let name = TagName::parse("EMC").unwrap();

        assert!(name.is_same_as("emc"));
        assert!(!name.is_same_as("EMC 2"));
    }

    #[test]
    fn a_tag_name_is_numbered_with_the_first_free_number() {
        let taken = ["emc", "emc 2"];

        let name = TagName::numbered("EMC", &|candidate: &str| {
            !taken.contains(&candidate.to_lowercase().as_str())
        });

        assert_eq!(name.as_str(), "EMC 3");
        let long = TagName::numbered(&"x".repeat(TagName::MAX_LENGTH), &|_| true);
        assert_eq!(long.as_str().chars().count(), TagName::MAX_LENGTH);
        assert!(long.as_str().ends_with(" 2"));
    }
}
