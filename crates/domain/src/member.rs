//! The members of a project's team.

use crate::color::Color;
use crate::error::DomainError;
use crate::ids::{MemberId, ProjectId};
use crate::macros::text_enum;
use crate::text::{MemberName, ModelName};
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

/// How hard the model of a specialist thinks. More effort uses more tokens.
///
/// The values are ordered from the least effort to the most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Effort {
    /// The least effort.
    Low,
    /// Moderate effort.
    Medium,
    /// High effort.
    High,
    /// More than high effort.
    ExtraHigh,
    /// The most effort.
    Max,
}

text_enum!(Effort, kind = "effort", {
    Low => "low",
    Medium => "medium",
    High => "high",
    ExtraHigh => "xhigh",
    Max => "max",
});

/// Which model an AI specialist runs on and how hard it thinks.
///
/// Either part may be absent: the program that runs the specialist then
/// uses its own default. A person always has the empty profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct MemberProfile {
    /// The model, or the program's default if absent.
    pub model: Option<ModelName>,
    /// The effort, or the program's default if absent.
    pub effort: Option<Effort>,
}

impl MemberProfile {
    /// Whether the profile leaves both the model and the effort to the
    /// program.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.model.is_none() && self.effort.is_none()
    }
}

/// What an AI specialist is told about how it works, in the manager's
/// words: what it cares about, how it checks its work, what it must never
/// do.
///
/// The instructions reach every turn of the specialist after the fixed
/// instructions of Teamwright, and never instead of them. They may be
/// empty. Line breaks inside are kept, because instructions are often a
/// list; whitespace at both ends is removed, and every line break is
/// written as a single `\n`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct MemberInstructions(String);

impl MemberInstructions {
    /// The largest number of characters instructions can have: about a
    /// page, or a thousand tokens on every turn of the specialist.
    pub const MAX_LENGTH: usize = 4_000;

    /// Checks and tidies instructions.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::TextTooLong`] if the tidied text is longer
    /// than [`Self::MAX_LENGTH`] characters.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let text = raw.replace("\r\n", "\n").replace('\r', "\n");
        let text = text.trim();
        let actual_length = text.chars().count();
        if actual_length > Self::MAX_LENGTH {
            return Err(DomainError::TextTooLong {
                field: "instructions of a specialist",
                max_length: Self::MAX_LENGTH,
                actual_length,
            });
        }
        Ok(Self(text.to_owned()))
    }

    /// The instructions as text. Empty if there are none.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether there are no instructions.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

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
    /// The model and the effort of an AI specialist. Empty for a person.
    pub profile: MemberProfile,
    /// The instructions of an AI specialist. Empty for a person.
    pub instructions: MemberInstructions,
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
    profile: MemberProfile,
    instructions: MemberInstructions,
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

    /// Puts someone on the team of a project. The member starts with the
    /// empty profile and without instructions.
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
            profile: MemberProfile::default(),
            instructions: MemberInstructions::default(),
            color,
            joined_at: now,
            updated_at: now,
        }
    }

    /// Recreates a member from stored data.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ProfileOfPerson`] if the data describes a
    /// person with a model or an effort, and
    /// [`DomainError::InstructionsOfPerson`] if it describes a person with
    /// instructions.
    pub fn restore(snapshot: MemberSnapshot) -> Result<Self, DomainError> {
        refuse_profile_of_person(snapshot.kind, &snapshot.profile)?;
        refuse_instructions_of_person(snapshot.kind, &snapshot.instructions)?;
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            kind: snapshot.kind,
            responsibility: snapshot.responsibility,
            profile: snapshot.profile,
            instructions: snapshot.instructions,
            color: snapshot.color,
            joined_at: snapshot.joined_at,
            updated_at: snapshot.updated_at,
        })
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

    /// The model and the effort of the member. Empty for a person.
    #[must_use]
    pub const fn profile(&self) -> &MemberProfile {
        &self.profile
    }

    /// The model the member runs on, or `None` for the program's default.
    #[must_use]
    pub const fn model(&self) -> Option<&ModelName> {
        self.profile.model.as_ref()
    }

    /// How hard the member's model thinks, or `None` for the program's
    /// default.
    #[must_use]
    pub const fn effort(&self) -> Option<Effort> {
        self.profile.effort
    }

    /// The instructions of an AI specialist, in the manager's words. Empty
    /// for a person, and for a specialist that has none.
    #[must_use]
    pub const fn instructions(&self) -> &MemberInstructions {
        &self.instructions
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

    /// Replaces the model and the effort of the member.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ProfileOfPerson`] if the member is a person
    /// and the profile is not empty. Nothing is changed then.
    pub fn set_profile(
        &mut self,
        profile: MemberProfile,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        refuse_profile_of_person(self.kind, &profile)?;
        self.profile = profile;
        self.updated_at = now;
        Ok(())
    }

    /// Replaces the instructions of the member.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InstructionsOfPerson`] if the member is a
    /// person and the instructions are not empty. Nothing is changed then.
    pub fn set_instructions(
        &mut self,
        instructions: MemberInstructions,
        now: Timestamp,
    ) -> Result<(), DomainError> {
        refuse_instructions_of_person(self.kind, &instructions)?;
        self.instructions = instructions;
        self.updated_at = now;
        Ok(())
    }
}

/// Refuses instructions for a person. Only an AI specialist follows them.
fn refuse_instructions_of_person(
    kind: MemberKind,
    instructions: &MemberInstructions,
) -> Result<(), DomainError> {
    if kind == MemberKind::Human && !instructions.is_empty() {
        return Err(DomainError::InstructionsOfPerson);
    }
    Ok(())
}

/// Refuses a model or an effort for a person. Only an AI specialist runs on
/// a model.
fn refuse_profile_of_person(kind: MemberKind, profile: &MemberProfile) -> Result<(), DomainError> {
    if kind == MemberKind::Human && !profile.is_empty() {
        return Err(DomainError::ProfileOfPerson);
    }
    Ok(())
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
            profile: MemberProfile::default(),
            instructions: MemberInstructions::default(),
            color: Color::Pink,
            joined_at: moment(9),
            updated_at: moment(12),
        };

        let member = Member::restore(snapshot.clone()).unwrap();

        assert_eq!(member.id(), snapshot.id);
        assert_eq!(member.project_id(), snapshot.project_id);
        assert_eq!(member.name(), &snapshot.name);
        assert_eq!(member.kind(), MemberKind::Human);
        assert_eq!(member.responsibility(), "");
        assert!(member.profile().is_empty());
        assert_eq!(member.color(), Color::Pink);
        assert_eq!(member.joined_at(), moment(9));
        assert_eq!(member.updated_at(), moment(12));
    }

    fn opus_at_high_effort() -> MemberProfile {
        MemberProfile {
            model: Some(ModelName::parse("opus").unwrap()),
            effort: Some(Effort::High),
        }
    }

    fn dana() -> Member {
        Member::create(
            ProjectId::generate(),
            MemberName::parse("Dana").unwrap(),
            MemberKind::Human,
            String::new(),
            Color::Pink,
            moment(9),
        )
    }

    #[test]
    fn a_new_member_leaves_the_model_and_the_effort_to_the_program() {
        let member = hardware_engineer();

        assert!(member.profile().is_empty());
        assert_eq!(member.model(), None);
        assert_eq!(member.effort(), None);
    }

    #[test]
    fn a_specialist_can_be_given_a_model_and_an_effort_and_lose_them_again() {
        let mut member = hardware_engineer();

        member
            .set_profile(opus_at_high_effort(), moment(10))
            .unwrap();
        assert_eq!(member.model().map(ModelName::as_str), Some("opus"));
        assert_eq!(member.effort(), Some(Effort::High));
        assert_eq!(member.updated_at(), moment(10));

        member
            .set_profile(MemberProfile::default(), moment(11))
            .unwrap();
        assert!(member.profile().is_empty());
        assert_eq!(member.updated_at(), moment(11));
    }

    #[test]
    fn a_person_has_neither_a_model_nor_an_effort() {
        let mut person = dana();

        let error = person
            .set_profile(opus_at_high_effort(), moment(10))
            .unwrap_err();

        assert_eq!(error, DomainError::ProfileOfPerson);
        assert!(person.profile().is_empty());
        assert_eq!(person.updated_at(), moment(9));
    }

    #[test]
    fn a_person_may_be_given_the_empty_profile() {
        let mut person = dana();

        person
            .set_profile(MemberProfile::default(), moment(10))
            .unwrap();

        assert_eq!(person.updated_at(), moment(10));
    }

    #[test]
    fn stored_data_of_a_person_with_a_profile_is_refused() {
        let snapshot = MemberSnapshot {
            id: MemberId::generate(),
            project_id: ProjectId::generate(),
            name: MemberName::parse("Dana").unwrap(),
            kind: MemberKind::Human,
            responsibility: String::new(),
            profile: opus_at_high_effort(),
            instructions: MemberInstructions::default(),
            color: Color::Pink,
            joined_at: moment(9),
            updated_at: moment(9),
        };

        assert_eq!(
            Member::restore(snapshot).unwrap_err(),
            DomainError::ProfileOfPerson
        );
    }

    fn never_guess_a_pin() -> MemberInstructions {
        MemberInstructions::parse("Never guess a pin.\nAsk instead.").unwrap()
    }

    #[test]
    fn a_new_member_has_no_instructions() {
        assert!(hardware_engineer().instructions().is_empty());
        assert_eq!(hardware_engineer().instructions().as_str(), "");
    }

    #[test]
    fn a_specialist_can_be_given_instructions_and_lose_them_again() {
        let mut member = hardware_engineer();

        member
            .set_instructions(never_guess_a_pin(), moment(10))
            .unwrap();
        assert_eq!(
            member.instructions().as_str(),
            "Never guess a pin.\nAsk instead."
        );
        assert_eq!(member.updated_at(), moment(10));

        member
            .set_instructions(MemberInstructions::default(), moment(11))
            .unwrap();
        assert!(member.instructions().is_empty());
        assert_eq!(member.updated_at(), moment(11));
    }

    #[test]
    fn a_person_has_no_instructions() {
        let mut person = dana();

        let error = person
            .set_instructions(never_guess_a_pin(), moment(10))
            .unwrap_err();

        assert_eq!(error, DomainError::InstructionsOfPerson);
        assert!(person.instructions().is_empty());
        assert_eq!(person.updated_at(), moment(9));
        person
            .set_instructions(MemberInstructions::default(), moment(10))
            .unwrap();
    }

    #[test]
    fn stored_data_of_a_person_with_instructions_is_refused() {
        let snapshot = MemberSnapshot {
            id: MemberId::generate(),
            project_id: ProjectId::generate(),
            name: MemberName::parse("Dana").unwrap(),
            kind: MemberKind::Human,
            responsibility: String::new(),
            profile: MemberProfile::default(),
            instructions: never_guess_a_pin(),
            color: Color::Pink,
            joined_at: moment(9),
            updated_at: moment(9),
        };

        assert_eq!(
            Member::restore(snapshot).unwrap_err(),
            DomainError::InstructionsOfPerson
        );
    }

    #[test]
    fn instructions_keep_their_lines_and_lose_the_space_around_them() {
        let instructions =
            MemberInstructions::parse("\r\n  - Check the units.\r\n- Say what you left out.\n\n")
                .unwrap();

        assert_eq!(
            instructions.as_str(),
            "- Check the units.\n- Say what you left out."
        );
        assert!(MemberInstructions::parse(" \n\t ").unwrap().is_empty());
    }

    #[test]
    fn instructions_have_a_limit_of_length_counted_in_characters() {
        let longest_allowed = "ü".repeat(MemberInstructions::MAX_LENGTH);
        let too_long = "ü".repeat(MemberInstructions::MAX_LENGTH + 1);

        assert!(MemberInstructions::parse(&longest_allowed).is_ok());
        assert_eq!(
            MemberInstructions::parse(&too_long).unwrap_err(),
            DomainError::TextTooLong {
                field: "instructions of a specialist",
                max_length: MemberInstructions::MAX_LENGTH,
                actual_length: MemberInstructions::MAX_LENGTH + 1,
            }
        );
    }

    #[test]
    fn every_effort_survives_being_written_as_text_and_read_back() {
        for effort in Effort::ALL {
            assert_eq!(effort.as_str().parse::<Effort>().unwrap(), *effort);
        }
        assert_eq!(Effort::ExtraHigh.as_str(), "xhigh");
    }

    #[test]
    fn efforts_are_ordered_from_the_least_to_the_most() {
        let mut sorted = Effort::ALL.to_vec();
        sorted.sort();

        assert_eq!(sorted, Effort::ALL);
        assert!(Effort::Low < Effort::Max);
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
