//! The areas of a project: the parts of it that each have a job of their
//! own, such as the board, the firmware and the enclosure of a device
//! ([decision 0022](../../../docs/decisions/0022-areas-inside-a-project.md)).
//!
//! An area has a name and a place among the areas of its project. Every
//! task is in exactly one area. The rules here concern one area, or one
//! name. The rules that look at the other areas or at the tasks of the
//! project, such as "no two areas with one name" or "the last area stays",
//! are use cases.

use uuid::Uuid;

use crate::ids::{AreaId, ProjectId};
use crate::text::AreaName;

/// The plain data of an area, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaSnapshot {
    /// The identifier of the area.
    pub id: AreaId,
    /// The project the area belongs to.
    pub project_id: ProjectId,
    /// Its name.
    pub name: AreaName,
    /// Its place among the areas of its project. Areas are listed by it,
    /// the smallest first.
    pub position: u32,
}

/// A part of a project with a job of its own. A project has one or more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    id: AreaId,
    project_id: ProjectId,
    name: AreaName,
    position: u32,
}

impl Area {
    /// The highest attempt [`Area::general_again`] knows an identifier for.
    pub const LAST_GENERAL_ATTEMPT: u8 = 8;

    /// The area a new project starts with: "General", in the first place.
    ///
    /// Its identifier follows from the project (see [`Area::starting_id`]),
    /// so two computers that each give a project its starting area give it
    /// the same one.
    #[must_use]
    pub fn starting(project_id: ProjectId) -> Self {
        Self {
            id: Self::starting_id(project_id),
            project_id,
            name: AreaName::general(),
            position: 0,
        }
    }

    /// The identifier of the starting area of a project: the identifier of
    /// the project with its version digit replaced by 0.
    ///
    /// Identifiers the program generates have the version digit 7, so a
    /// starting identifier never equals one of them, and two projects never
    /// share one.
    #[must_use]
    pub fn starting_id(project_id: ProjectId) -> AreaId {
        Self::id_with_digit(project_id, 0)
    }

    /// The area "General" a project gets again when it is left with no
    /// areas: two computers that each removed other areas can leave it so.
    /// Every computer gives it the same area, with an identifier that
    /// follows from the project: the identifier of its starting area at the
    /// first attempt, and, because a deletion is final, one with the version
    /// digit 8, 9, ... up to 15 at the later attempts, once the identifier
    /// before was deleted too. Nothing beyond the last attempt.
    #[must_use]
    pub fn general_again(project_id: ProjectId, attempt: u8) -> Option<Self> {
        let id = match attempt {
            0 => Self::starting_id(project_id),
            1..=Self::LAST_GENERAL_ATTEMPT => Self::id_with_digit(project_id, 7 + attempt),
            _ => return None,
        };
        Some(Self {
            id,
            project_id,
            name: AreaName::general(),
            position: 0,
        })
    }

    /// The identifier of a project with its version digit replaced.
    fn id_with_digit(project_id: ProjectId, digit: u8) -> AreaId {
        let mut bytes = *project_id.as_uuid().as_bytes();
        bytes[6] = (bytes[6] & 0x0f) | (digit << 4);
        AreaId::from_uuid(Uuid::from_bytes(bytes))
    }

    /// Adds an area to a project at a place.
    #[must_use]
    pub fn add(project_id: ProjectId, name: AreaName, position: u32) -> Self {
        Self {
            id: AreaId::generate(),
            project_id,
            name,
            position,
        }
    }

    /// Recreates an area from stored data.
    #[must_use]
    pub fn restore(snapshot: AreaSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            position: snapshot.position,
        }
    }

    /// The identifier of the area.
    #[must_use]
    pub const fn id(&self) -> AreaId {
        self.id
    }

    /// The project the area belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name of the area.
    #[must_use]
    pub const fn name(&self) -> &AreaName {
        &self.name
    }

    /// The place of the area among the areas of its project.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// Gives the area another name.
    pub fn rename(&mut self, name: AreaName) {
        self.name = name;
    }

    /// Puts the area at another place among the areas of its project.
    pub fn move_to(&mut self, position: u32) {
        self.position = position;
    }
}

impl AreaName {
    /// The name of the area every project starts with.
    pub const GENERAL: &'static str = "General";

    /// The name of the area every project starts with: "General".
    #[must_use]
    pub fn general() -> Self {
        Self::from_trusted(Self::GENERAL)
    }

    /// This name with the first number from 2 on for which `is_free` holds,
    /// such as "Firmware 2". The name is shortened if the number would not
    /// fit otherwise.
    #[must_use]
    pub fn numbered(base: &str, is_free: &impl Fn(&str) -> bool) -> Self {
        let mut number: u32 = 2;
        loop {
            let suffix = format!(" {number}");
            let room = Self::MAX_LENGTH.saturating_sub(suffix.chars().count());
            let stem: String = base.chars().take(room).collect();
            let candidate = format!("{}{suffix}", stem.trim_end());
            if is_free(&candidate) {
                // A trimmed stem and a number are never empty, never
                // multi-line and never too long.
                return Self::parse(&candidate).unwrap_or_else(|_| Self::general());
            }
            number = number.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_of_the_starting_area_follows_the_rules() {
        assert_eq!(
            AreaName::parse(AreaName::GENERAL).unwrap(),
            AreaName::general()
        );
    }

    #[test]
    fn a_new_project_starts_with_the_area_general_in_the_first_place() {
        let project = ProjectId::generate();

        let area = Area::starting(project);

        assert_eq!(area.name().as_str(), "General");
        assert_eq!(area.position(), 0);
        assert_eq!(area.project_id(), project);
    }

    #[test]
    fn the_starting_area_of_a_project_has_the_same_identifier_every_time() {
        let project = ProjectId::generate();
        let other = ProjectId::generate();

        assert_eq!(Area::starting(project).id(), Area::starting(project).id());
        assert_ne!(Area::starting(project).id(), Area::starting(other).id());
        assert_ne!(Area::starting(project).id(), AreaId::generate());
    }

    #[test]
    fn a_starting_identifier_is_the_project_with_the_digit_0() {
        let project: ProjectId = "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10".parse().unwrap();

        let id = Area::starting_id(project);

        assert_eq!(id.to_string(), "0196f3a2-7c1e-0d53-9b6f-2f1a4c8e9d10");
    }

    #[test]
    fn general_given_again_has_an_identifier_of_its_own_at_each_attempt() {
        let project = ProjectId::generate();

        let ids: Vec<AreaId> = (0..=Area::LAST_GENERAL_ATTEMPT)
            .filter_map(|attempt| Area::general_again(project, attempt))
            .map(|area| area.id())
            .collect();

        assert_eq!(ids.len(), 9);
        assert_eq!(ids[0], Area::starting_id(project));
        let mut distinct = ids.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len());
        assert!(
            ids.iter()
                .all(|id| *id != AreaId::from_uuid(project.as_uuid()))
        );
        assert_eq!(
            Area::general_again(project, Area::LAST_GENERAL_ATTEMPT + 1),
            None
        );
        let again = Area::general_again(project, 3).unwrap();
        assert_eq!(again.name().as_str(), "General");
        assert_eq!(Area::general_again(project, 3), Some(again));
    }

    #[test]
    fn an_area_is_renamed_and_moved() {
        let project = ProjectId::generate();
        let mut area = Area::add(project, AreaName::parse("Firmware").unwrap(), 1);

        area.rename(AreaName::parse("Software").unwrap());
        area.move_to(3);

        assert_eq!(area.name().as_str(), "Software");
        assert_eq!(area.position(), 3);
    }

    #[test]
    fn a_name_is_numbered_with_the_first_free_number_whatever_the_capitals() {
        let taken = ["firmware", "firmware 2", "firmware 4"];

        let name = AreaName::numbered("Firmware", &|candidate: &str| {
            !taken.contains(&candidate.to_lowercase().as_str())
        });

        assert_eq!(name.as_str(), "Firmware 3");
    }

    #[test]
    fn a_long_name_is_shortened_to_make_room_for_its_number() {
        let long = "x".repeat(AreaName::MAX_LENGTH);

        let name = AreaName::numbered(&long, &|_| true);

        assert_eq!(name.as_str().chars().count(), AreaName::MAX_LENGTH);
        assert!(name.as_str().ends_with(" 2"));
    }

    #[test]
    fn an_area_name_is_tidied_and_has_at_most_40_characters() {
        assert_eq!(
            AreaName::parse("  Power \n  supply ").unwrap().as_str(),
            "Power supply"
        );
        assert_eq!(
            AreaName::parse(" \n ").unwrap_err().to_string(),
            "the area name must not be empty"
        );
        assert_eq!(
            AreaName::parse(&"x".repeat(41)).unwrap_err().to_string(),
            "the area name can have at most 40 characters, but this one has 41"
        );
        assert!(AreaName::parse(&"x".repeat(40)).is_ok());
    }
}
