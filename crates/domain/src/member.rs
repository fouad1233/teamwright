//! The members of a project's team.

use crate::color::Color;
use crate::ids::{MemberId, ProjectId};
use crate::macros::text_enum;
use crate::text::MemberName;
use crate::time::Timestamp;

/// What kind of member someone is.
///
/// Both kinds are given tasks in the same way. They differ in who does the
/// work: an agent carries a task out itself, a person is named as the one
/// who is responsible for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKind {
    /// An AI specialist, such as a hardware designer.
    Agent,
    /// A person.
    Human,
}

text_enum!(MemberKind, kind = "kind of team member", {
    Agent => "agent",
    Human => "human",
});

/// The plain data of a team member, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberSnapshot {
    /// The identifier of the member.
    pub id: MemberId,
    /// The project whose team the member is on.
    pub project_id: ProjectId,
    /// What the member is called on the team.
    pub name: MemberName,
    /// Whether the member is an AI agent or a person.
    pub kind: MemberKind,
    /// What the member is on the team for. May be empty.
    pub responsibility: String,
    /// The color the member is shown in.
    pub color: Color,
    /// When the member joined the team.
    pub joined_at: Timestamp,
    /// When the member was last changed.
    pub updated_at: Timestamp,
}

/// Someone on the team of a project who can be given tasks: an AI agent or a
/// human teammate.
///
/// The manager is not a member. The manager owns the project and is always
/// there; a team is the help the manager brings in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    id: MemberId,
    project_id: ProjectId,
    name: MemberName,
    kind: MemberKind,
    responsibility: String,
    color: Color,
    joined_at: Timestamp,
    updated_at: Timestamp,
}

impl Member {
    /// The colors that are handed out to new members, in this order.
    ///
    /// Gray, amber and red are left out. In the interface they already mean
    /// something about a person: nobody, the manager, and trouble.
    pub const COLORS: &'static [Color] = &[
        Color::Teal,
        Color::Blue,
        Color::Purple,
        Color::Pink,
        Color::Green,
    ];

    /// Chooses the color for a newcomer to a team, given the colors of the
    /// members who are already on it.
    ///
    /// The newcomer gets the first color that nobody has. When every color
    /// is taken, the newcomer gets the one that the fewest members have.
    #[must_use]
    pub fn color_for_newcomer(colors_in_use: &[Color]) -> Color {
        Self::COLORS
            .iter()
            .copied()
            .min_by_key(|color| colors_in_use.iter().filter(|used| *used == color).count())
            .unwrap_or(Color::Teal)
    }

    /// Puts someone on the team of a project.
    #[must_use]
    pub fn create(
        project_id: ProjectId,
        name: MemberName,
        kind: MemberKind,
        responsibility: String,
        color: Color,
        now: Timestamp,
    ) -> Self {
        Self {
            id: MemberId::generate(),
            project_id,
            name,
            kind,
            responsibility,
            color,
            joined_at: now,
            updated_at: now,
        }
    }

    /// Recreates a member from stored data.
    #[must_use]
    pub fn restore(snapshot: MemberSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            kind: snapshot.kind,
            responsibility: snapshot.responsibility,
            color: snapshot.color,
            joined_at: snapshot.joined_at,
            updated_at: snapshot.updated_at,
        }
    }

    /// The identifier of the member.
    #[must_use]
    pub const fn id(&self) -> MemberId {
        self.id
    }

    /// The project whose team the member is on.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// What the member is called on the team.
    #[must_use]
    pub const fn name(&self) -> &MemberName {
        &self.name
    }

    /// Whether the member is an AI agent or a person.
    #[must_use]
    pub const fn kind(&self) -> MemberKind {
        self.kind
    }

    /// What the member is on the team for. May be empty.
    #[must_use]
    pub fn responsibility(&self) -> &str {
        &self.responsibility
    }

    /// The color the member is shown in.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// When the member joined the team.
    #[must_use]
    pub const fn joined_at(&self) -> Timestamp {
        self.joined_at
    }

    /// When the member was last changed.
    #[must_use]
    pub const fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// Gives the member a new name.
    pub fn rename(&mut self, name: MemberName, now: Timestamp) {
        self.name = name;
        self.updated_at = now;
    }

    /// Replaces what the member is on the team for.
    pub fn set_responsibility(&mut self, responsibility: String, now: Timestamp) {
        self.responsibility = responsibility;
        self.updated_at = now;
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn moment(hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 12, hour, 0, 0)
            .unwrap()
    }

    fn hardware_engineer() -> Member {
        Member::create(
            ProjectId::generate(),
            MemberName::parse("Hardware engineer").unwrap(),
            MemberKind::Agent,
            "The schematic, the parts and the board.".to_owned(),
            Color::Teal,
            moment(9),
        )
    }

    #[test]
    fn a_new_member_takes_everything_that_was_given_for_it() {
        let project_id = ProjectId::generate();

        let member = Member::create(
            project_id,
            MemberName::parse("Hardware engineer").unwrap(),
            MemberKind::Agent,
            "The schematic, the parts and the board.".to_owned(),
            Color::Blue,
            moment(9),
        );

        assert_eq!(member.project_id(), project_id);
        assert_eq!(member.name().as_str(), "Hardware engineer");
        assert_eq!(member.kind(), MemberKind::Agent);
        assert_eq!(
            member.responsibility(),
            "The schematic, the parts and the board."
        );
        assert_eq!(member.color(), Color::Blue);
        assert_eq!(member.joined_at(), moment(9));
        assert_eq!(member.updated_at(), moment(9));
    }

    #[test]
    fn two_new_members_never_share_an_identifier() {
        assert_ne!(hardware_engineer().id(), hardware_engineer().id());
    }

    #[test]
    fn a_change_to_a_member_records_when_it_was_made() {
        let mut member = hardware_engineer();

        member.rename(
            MemberName::parse("Electronics engineer").unwrap(),
            moment(10),
        );
        assert_eq!(member.name().as_str(), "Electronics engineer");
        assert_eq!(member.updated_at(), moment(10));

        member.set_responsibility("Everything on the board.".to_owned(), moment(11));
        assert_eq!(member.responsibility(), "Everything on the board.");
        assert_eq!(member.updated_at(), moment(11));

        assert_eq!(member.joined_at(), moment(9));
    }

    #[test]
    fn a_stored_member_is_recreated_as_it_was() {
        let snapshot = MemberSnapshot {
            id: MemberId::generate(),
            project_id: ProjectId::generate(),
            name: MemberName::parse("Dana").unwrap(),
            kind: MemberKind::Human,
            responsibility: String::new(),
            color: Color::Pink,
            joined_at: moment(9),
            updated_at: moment(12),
        };

        let member = Member::restore(snapshot.clone());

        assert_eq!(member.id(), snapshot.id);
        assert_eq!(member.project_id(), snapshot.project_id);
        assert_eq!(member.name(), &snapshot.name);
        assert_eq!(member.kind(), MemberKind::Human);
        assert_eq!(member.responsibility(), "");
        assert_eq!(member.color(), Color::Pink);
        assert_eq!(member.joined_at(), moment(9));
        assert_eq!(member.updated_at(), moment(12));
    }

    #[test]
    fn the_first_member_of_a_team_gets_the_first_color() {
        assert_eq!(Member::color_for_newcomer(&[]), Member::COLORS[0]);
    }

    #[test]
    fn a_newcomer_gets_a_color_that_nobody_on_the_team_has() {
        let mut colors_in_use = Vec::new();

        for _ in Member::COLORS {
            let color = Member::color_for_newcomer(&colors_in_use);
            assert!(!colors_in_use.contains(&color));
            colors_in_use.push(color);
        }
    }

    #[test]
    fn a_color_that_became_free_is_handed_out_again() {
        let colors_in_use = [Color::Teal, Color::Purple];

        assert_eq!(Member::color_for_newcomer(&colors_in_use), Color::Blue);
    }

    #[test]
    fn when_every_color_is_taken_a_newcomer_gets_the_one_used_least() {
        let mut colors_in_use = Member::COLORS.to_vec();
        colors_in_use.extend([Color::Teal, Color::Blue, Color::Pink, Color::Green]);

        assert_eq!(Member::color_for_newcomer(&colors_in_use), Color::Purple);
    }

    #[test]
    fn colors_that_already_mean_something_about_a_person_are_not_handed_out() {
        for reserved in [Color::Gray, Color::Amber, Color::Red] {
            assert!(!Member::COLORS.contains(&reserved));
        }
    }

    #[test]
    fn every_kind_of_member_survives_being_written_as_text_and_read_back() {
        for kind in MemberKind::ALL {
            assert_eq!(kind.as_str().parse::<MemberKind>().unwrap(), *kind);
        }
    }
}
