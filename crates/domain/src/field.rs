//! The custom fields of a project: properties of its tasks the manager
//! defines, such as "Supplier", "Unit cost" or "Datasheet"
//! ([decision 0035](../../../docs/decisions/0035-custom-fields.md)).
//!
//! A field belongs to one project and has a name, a kind and a place among
//! the fields of the project. A number field may have a unit; a choice
//! field has its choices, each a name and a color, like a tag. A task holds
//! at most one value for each field of its project.
//!
//! The rules here concern one field, or one value. The rules that look at
//! the other fields of the project, such as "no two fields with one name",
//! are use cases.

use std::cmp::Ordering;

use crate::color::Color;
use crate::error::DomainError;
use crate::ids::{ChoiceId, FieldId, ProjectId};
use crate::integration::WebAddress;
use crate::macros::text_enum;
use crate::tag::Tag;
use crate::task::Rank;
use crate::text::{ChoiceName, FieldName, FieldText, FieldUnit};
use crate::time::CalendarDate;

/// What a custom field holds. It is chosen when the field is made and never
/// changes: the values the tasks hold would have to be converted, and most
/// conversions (a date to a number, a text to a choice) lose something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FieldKind {
    /// One line of text, such as a part number.
    Text,
    /// A number, with a unit shown after it if the field has one.
    Number,
    /// A day on the calendar.
    Date,
    /// Yes or no.
    YesNo,
    /// One of a list of choices the manager writes.
    Choice,
    /// A web address that starts with `https://`.
    Link,
}

text_enum!(FieldKind, kind = "field kind", {
    Text => "text",
    Number => "number",
    Date => "date",
    YesNo => "yes_no",
    Choice => "choice",
    Link => "link",
});

impl FieldKind {
    /// What a field of this kind holds, in words: "a number", "a date".
    #[must_use]
    pub const fn in_words(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Number => "number",
            Self::Date => "date",
            Self::YesNo => "yes or no",
            Self::Choice => "choice",
            Self::Link => "web address",
        }
    }
}

/// A number held by a field: an ordinary number, never infinite and never
/// "not a number", at most a thousand billion either side of zero, so that
/// it can be compared, sorted and written to a spreadsheet as it is.
#[derive(Debug, Clone, Copy)]
pub struct FieldNumber(f64);

impl FieldNumber {
    /// The largest number a field holds, either side of zero. Far beyond
    /// any cost or measure of a project, and still exact to the cent.
    pub const LARGEST: f64 = 1e12;

    /// Checks a number.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::FieldNumberOutOfRange`] for a number that is
    /// not ordinary or lies beyond [`Self::LARGEST`].
    pub fn new(value: f64) -> Result<Self, DomainError> {
        if value.is_finite() && value.abs() <= Self::LARGEST {
            // Adding zero turns a negative zero into plain zero, so that
            // equal numbers always compare equal.
            Ok(Self(value + 0.0))
        } else {
            Err(DomainError::FieldNumberOutOfRange)
        }
    }

    /// The number.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl PartialEq for FieldNumber {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FieldNumber {}

impl PartialOrd for FieldNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FieldNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The value a task holds for one field. Each part is checked when it is
/// made: a text is one line of at most 200 characters, a number is
/// ordinary, a link starts with `https://` and names a host. Whether the
/// value suits the field (its kind, its choices) is checked by
/// [`Field::check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// One line of text.
    Text(FieldText),
    /// A number.
    Number(FieldNumber),
    /// A day.
    Date(CalendarDate),
    /// Yes (true) or no (false).
    YesNo(bool),
    /// One choice of the field.
    Choice(ChoiceId),
    /// A web address.
    Link(WebAddress),
}

impl FieldValue {
    /// Reads a web address for a link field. Only an address that starts
    /// with `https://` and names a host is a link, so that what the
    /// interface opens is always a page on the web.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::InvalidWebAddress`] for anything else.
    pub fn link(text: &str) -> Result<Self, DomainError> {
        WebAddress::parse(text, "a link").map(Self::Link)
    }

    /// The kind of field that holds such a value.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        match self {
            Self::Text(_) => FieldKind::Text,
            Self::Number(_) => FieldKind::Number,
            Self::Date(_) => FieldKind::Date,
            Self::YesNo(_) => FieldKind::YesNo,
            Self::Choice(_) => FieldKind::Choice,
            Self::Link(_) => FieldKind::Link,
        }
    }
}

/// One choice of a choice field: a name and a color, like a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChoice {
    id: ChoiceId,
    name: ChoiceName,
    color: Color,
    rank: Rank,
}

impl FieldChoice {
    /// Recreates a choice from stored data.
    #[must_use]
    pub const fn restore(id: ChoiceId, name: ChoiceName, color: Color, rank: Rank) -> Self {
        Self {
            id,
            name,
            color,
            rank,
        }
    }

    /// The identifier of the choice.
    #[must_use]
    pub const fn id(&self) -> ChoiceId {
        self.id
    }

    /// Its name.
    #[must_use]
    pub const fn name(&self) -> &ChoiceName {
        &self.name
    }

    /// Its color.
    #[must_use]
    pub const fn color(&self) -> Color {
        self.color
    }

    /// Its place among the choices of its field: they are listed by it, the
    /// smallest first. A new choice comes last.
    #[must_use]
    pub const fn rank(&self) -> Rank {
        self.rank
    }
}

/// The plain data of a field, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSnapshot {
    /// The identifier of the field.
    pub id: FieldId,
    /// The project the field belongs to.
    pub project_id: ProjectId,
    /// Its name.
    pub name: FieldName,
    /// What it holds.
    pub kind: FieldKind,
    /// The unit after its numbers, for a number field that has one.
    pub unit: Option<FieldUnit>,
    /// Its place among the fields of its project, the smallest first.
    pub position: u32,
    /// Its choices, in any order, for a choice field.
    pub choices: Vec<FieldChoice>,
}

/// A property of the tasks of a project that the manager defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    id: FieldId,
    project_id: ProjectId,
    name: FieldName,
    kind: FieldKind,
    unit: Option<FieldUnit>,
    position: u32,
    choices: Vec<FieldChoice>,
}

impl Field {
    /// The most fields a project may have. Each is a column of the table
    /// and a line of the panel of a task; beyond this neither is read.
    pub const MOST_IN_A_PROJECT: usize = 30;

    /// The most choices a field may have, like a short menu.
    pub const MOST_CHOICES: usize = 50;

    /// Makes a field in a project, at a place among its fields, without a
    /// unit and without choices.
    #[must_use]
    pub fn add(project_id: ProjectId, name: FieldName, kind: FieldKind, position: u32) -> Self {
        Self {
            id: FieldId::generate(),
            project_id,
            name,
            kind,
            unit: None,
            position,
            choices: Vec::new(),
        }
    }

    /// Recreates a field from stored data. The choices are put in their
    /// order: by rank, and by identifier where ranks are equal, which two
    /// computers can leave behind.
    #[must_use]
    pub fn restore(snapshot: FieldSnapshot) -> Self {
        let mut choices = snapshot.choices;
        choices.sort_by_key(|choice| (choice.rank, choice.id));
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            name: snapshot.name,
            kind: snapshot.kind,
            unit: snapshot.unit,
            position: snapshot.position,
            choices,
        }
    }

    /// The identifier of the field.
    #[must_use]
    pub const fn id(&self) -> FieldId {
        self.id
    }

    /// The project the field belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The name of the field.
    #[must_use]
    pub const fn name(&self) -> &FieldName {
        &self.name
    }

    /// What the field holds.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        self.kind
    }

    /// The unit shown after its numbers, if it has one.
    #[must_use]
    pub const fn unit(&self) -> Option<&FieldUnit> {
        self.unit.as_ref()
    }

    /// The place of the field among the fields of its project.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// The choices of a choice field, in their order. Empty for the other
    /// kinds.
    #[must_use]
    pub fn choices(&self) -> &[FieldChoice] {
        &self.choices
    }

    /// The choice with an identifier, if the field has it.
    #[must_use]
    pub fn choice(&self, id: ChoiceId) -> Option<&FieldChoice> {
        self.choices.iter().find(|choice| choice.id == id)
    }

    /// Gives the field another name.
    pub fn rename(&mut self, name: FieldName) {
        self.name = name;
    }

    /// Puts the field at another place among the fields of its project.
    pub const fn move_to(&mut self, position: u32) {
        self.position = position;
    }

    /// Gives a number field a unit, or takes it away with nothing.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::UnitWithoutNumbers`] for a unit on a field of
    /// another kind. Taking the unit away is always allowed.
    pub fn set_unit(&mut self, unit: Option<FieldUnit>) -> Result<(), DomainError> {
        if unit.is_some() && self.kind != FieldKind::Number {
            return Err(DomainError::UnitWithoutNumbers);
        }
        self.unit = unit;
        Ok(())
    }

    /// Adds a choice at the end of the choices of a choice field. Without a
    /// color, it gets the next color in turn, as a tag does.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::FieldWithoutChoices`] for a field of another
    /// kind, [`DomainError::ChoiceNameTaken`] if another choice has the
    /// name, whatever the capitals, and [`DomainError::TooManyChoices`] if
    /// the field has as many as it may.
    pub fn add_choice(
        &mut self,
        name: ChoiceName,
        color: Option<Color>,
    ) -> Result<ChoiceId, DomainError> {
        self.require_choices()?;
        self.refuse_choice_name(&name, None)?;
        if self.choices.len() >= Self::MOST_CHOICES {
            return Err(DomainError::TooManyChoices {
                max: Self::MOST_CHOICES,
            });
        }
        let last = self.choices.last().map(|choice| choice.rank);
        let choice = FieldChoice {
            id: ChoiceId::generate(),
            name,
            color: color.unwrap_or_else(|| Tag::color_in_turn(self.choices.len())),
            rank: Rank::between(last, None),
        };
        let id = choice.id;
        self.choices.push(choice);
        Ok(id)
    }

    /// Gives a choice another name, by the same rule. A choice may change
    /// the capitals of its own name.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NoSuchChoice`] for a choice the field does not
    /// have and [`DomainError::ChoiceNameTaken`] for a name in use.
    pub fn rename_choice(&mut self, id: ChoiceId, name: ChoiceName) -> Result<(), DomainError> {
        self.refuse_choice_name(&name, Some(id))?;
        self.choice_mut(id)?.name = name;
        Ok(())
    }

    /// Gives a choice another color.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NoSuchChoice`] for a choice the field does not
    /// have.
    pub fn recolor_choice(&mut self, id: ChoiceId, color: Color) -> Result<(), DomainError> {
        self.choice_mut(id)?.color = color;
        Ok(())
    }

    /// Removes a choice. The tasks that held it are cleared by the use
    /// case, which knows them.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::NoSuchChoice`] for a choice the field does not
    /// have.
    pub fn remove_choice(&mut self, id: ChoiceId) -> Result<(), DomainError> {
        let before = self.choices.len();
        self.choices.retain(|choice| choice.id != id);
        if self.choices.len() == before {
            return Err(DomainError::NoSuchChoice);
        }
        Ok(())
    }

    /// Checks that a value suits the field: that it is of the kind of the
    /// field, and, for a choice, that it is one of its choices.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::FieldValueOfAnotherKind`] for a value of
    /// another kind and [`DomainError::NoSuchChoice`] for a choice the
    /// field does not have.
    pub fn check(&self, value: &FieldValue) -> Result<(), DomainError> {
        if value.kind() != self.kind {
            return Err(DomainError::FieldValueOfAnotherKind {
                field: self.name.to_string(),
                kind: self.kind.in_words(),
            });
        }
        if let FieldValue::Choice(choice) = value
            && self.choice(*choice).is_none()
        {
            return Err(DomainError::NoSuchChoice);
        }
        Ok(())
    }

    /// Refuses a field that has no choices.
    fn require_choices(&self) -> Result<(), DomainError> {
        if self.kind == FieldKind::Choice {
            Ok(())
        } else {
            Err(DomainError::FieldWithoutChoices {
                field: self.name.to_string(),
            })
        }
    }

    /// Refuses a name another choice has, whatever the capitals. The choice
    /// `except` may keep its own name.
    fn refuse_choice_name(
        &self,
        name: &ChoiceName,
        except: Option<ChoiceId>,
    ) -> Result<(), DomainError> {
        let taken = self
            .choices
            .iter()
            .filter(|choice| Some(choice.id) != except)
            .any(|choice| choice.name.is_same_as(name.as_str()));
        if taken {
            return Err(DomainError::ChoiceNameTaken {
                name: name.to_string(),
            });
        }
        Ok(())
    }

    /// The choice with an identifier, to change it.
    fn choice_mut(&mut self, id: ChoiceId) -> Result<&mut FieldChoice, DomainError> {
        self.require_choices()?;
        self.choices
            .iter_mut()
            .find(|choice| choice.id == id)
            .ok_or(DomainError::NoSuchChoice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(kind: FieldKind) -> Field {
        Field::add(
            ProjectId::generate(),
            FieldName::parse("Supplier").unwrap(),
            kind,
            0,
        )
    }

    fn choice(name: &str) -> ChoiceName {
        ChoiceName::parse(name).unwrap()
    }

    #[test]
    fn a_choice_field_gets_choices_in_turn_of_color_and_in_order() {
        let mut supplier = field(FieldKind::Choice);

        let mouser = supplier.add_choice(choice("Mouser"), None).unwrap();
        let digikey = supplier
            .add_choice(choice("Digi-Key"), Some(Color::Red))
            .unwrap();
        let lcsc = supplier.add_choice(choice("LCSC"), None).unwrap();

        let names: Vec<&str> = supplier
            .choices()
            .iter()
            .map(|choice| choice.name().as_str())
            .collect();
        assert_eq!(names, ["Mouser", "Digi-Key", "LCSC"]);
        assert_eq!(supplier.choice(mouser).unwrap().color(), Color::Blue);
        assert_eq!(supplier.choice(digikey).unwrap().color(), Color::Red);
        assert_eq!(supplier.choice(lcsc).unwrap().color(), Color::Amber);
    }

    #[test]
    fn two_choices_of_a_field_never_share_a_name() {
        let mut supplier = field(FieldKind::Choice);
        let mouser = supplier.add_choice(choice("Mouser"), None).unwrap();

        assert_eq!(
            supplier.add_choice(choice("MOUSER"), None),
            Err(DomainError::ChoiceNameTaken {
                name: "MOUSER".to_owned()
            })
        );
        supplier.rename_choice(mouser, choice("mouser")).unwrap();
        assert_eq!(supplier.choice(mouser).unwrap().name().as_str(), "mouser");
    }

    #[test]
    fn a_field_has_at_most_fifty_choices() {
        let mut supplier = field(FieldKind::Choice);
        for number in 0..Field::MOST_CHOICES {
            supplier
                .add_choice(choice(&format!("Supplier {number}")), None)
                .unwrap();
        }

        assert_eq!(
            supplier.add_choice(choice("One more"), None),
            Err(DomainError::TooManyChoices { max: 50 })
        );
    }

    #[test]
    fn only_a_choice_field_has_choices_and_only_a_number_field_a_unit() {
        let mut cost = field(FieldKind::Number);
        cost.set_unit(Some(FieldUnit::parse("€").unwrap())).unwrap();
        assert_eq!(cost.unit().unwrap().as_str(), "€");
        assert_eq!(
            cost.add_choice(choice("Cheap"), None).unwrap_err().code(),
            "rule_broken"
        );

        let mut ordered = field(FieldKind::YesNo);
        assert_eq!(
            ordered.set_unit(Some(FieldUnit::parse("mm").unwrap())),
            Err(DomainError::UnitWithoutNumbers)
        );
        ordered.set_unit(None).unwrap();
    }

    #[test]
    fn a_choice_is_recolored_and_removed() {
        let mut supplier = field(FieldKind::Choice);
        let mouser = supplier.add_choice(choice("Mouser"), None).unwrap();

        supplier.recolor_choice(mouser, Color::Teal).unwrap();
        assert_eq!(supplier.choice(mouser).unwrap().color(), Color::Teal);
        supplier.remove_choice(mouser).unwrap();

        assert!(supplier.choices().is_empty());
        assert_eq!(
            supplier.remove_choice(mouser),
            Err(DomainError::NoSuchChoice)
        );
        assert_eq!(
            supplier.remove_choice(mouser).unwrap_err().code(),
            "not_found"
        );
    }

    #[test]
    fn a_value_suits_a_field_of_its_kind_and_a_choice_its_own_field() {
        let mut supplier = field(FieldKind::Choice);
        let mouser = supplier.add_choice(choice("Mouser"), None).unwrap();

        assert!(supplier.check(&FieldValue::Choice(mouser)).is_ok());
        assert_eq!(
            supplier.check(&FieldValue::Choice(ChoiceId::generate())),
            Err(DomainError::NoSuchChoice)
        );
        assert_eq!(
            supplier.check(&FieldValue::YesNo(true)),
            Err(DomainError::FieldValueOfAnotherKind {
                field: "Supplier".to_owned(),
                kind: "choice"
            })
        );
        let cost = field(FieldKind::Number);
        assert!(
            cost.check(&FieldValue::Number(FieldNumber::new(12.5).unwrap()))
                .is_ok()
        );
    }

    #[test]
    fn a_number_is_ordinary_and_not_too_large() {
        assert_eq!(FieldNumber::new(-0.0).unwrap().value().to_bits(), 0);
        assert!(FieldNumber::new(1e12).is_ok());
        for wrong in [f64::NAN, f64::INFINITY, 1.5e12, -1e13] {
            assert_eq!(
                FieldNumber::new(wrong),
                Err(DomainError::FieldNumberOutOfRange),
                "{wrong}"
            );
        }
    }

    #[test]
    fn a_link_starts_with_https_and_names_a_host() {
        assert_eq!(
            FieldValue::link(" https://www.ti.com/lit/ds/tps62840.pdf ")
                .unwrap()
                .kind(),
            FieldKind::Link
        );
        for wrong in [
            "http://example.com",
            "javascript:alert(1)",
            "file:///etc",
            "www.ti.com",
        ] {
            assert_eq!(
                FieldValue::link(wrong),
                Err(DomainError::InvalidWebAddress { field: "a link" }),
                "{wrong}"
            );
        }
    }

    #[test]
    fn the_kinds_of_fields_read_back_from_their_text() {
        for kind in FieldKind::ALL {
            assert_eq!(kind.as_str().parse::<FieldKind>().unwrap(), *kind);
        }
        assert!("person".parse::<FieldKind>().is_err());
    }

    #[test]
    fn field_and_choice_names_are_numbered_with_the_first_free_number() {
        let taken = ["supplier", "supplier 2"];
        let name = FieldName::numbered("Supplier", &|candidate: &str| {
            !taken.contains(&candidate.to_lowercase().as_str())
        });
        assert_eq!(name.as_str(), "Supplier 3");
        let long = ChoiceName::numbered(&"x".repeat(ChoiceName::MAX_LENGTH), &|_| true);
        assert_eq!(long.as_str().chars().count(), ChoiceName::MAX_LENGTH);
        assert!(long.as_str().ends_with(" 2"));
    }
}
