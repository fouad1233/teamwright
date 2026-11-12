//! Projects.

use std::fmt;

use crate::color::Color;
use crate::error::DomainError;
use crate::ids::{ProjectId, StatusId};
use crate::text::{ProjectName, StatusName};
use crate::time::Timestamp;
use crate::workflow::{Status, StatusCategory, Workflow};

/// The short code that prefixes the task numbers of a project, as in
/// `IOT-12`.
///
/// A key has 2 to 6 characters, starts with a letter and contains only
/// capital letters and digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectKey(String);

impl ProjectKey {
    /// The smallest number of characters a key can have.
    pub const MIN_LENGTH: usize = 2;
    /// The largest number of characters a key can have.
    pub const MAX_LENGTH: usize = 6;
    /// The number of characters of a key that is suggested from a name
    /// without usable initials.
    const SUGGESTED_PREFIX_LENGTH: usize = 3;
    /// The key suggested for a name that contains no letter at all.
    const FALLBACK: &'static str = "PRJ";
    /// The highest number that is appended to a key to make it unique
    /// (`IOT2` … `IOT9999`).
    const LAST_SUFFIX: u32 = 9999;

    /// Checks a key. Lower-case letters are accepted and stored as capitals.
    ///
    /// # Errors
    ///
    /// Returns an error if the text does not have the form of a key.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let key = raw.trim().to_ascii_uppercase();
        let has_valid_length = (Self::MIN_LENGTH..=Self::MAX_LENGTH).contains(&key.len());
        let starts_with_letter = key.starts_with(|first: char| first.is_ascii_alphabetic());
        let has_only_letters_and_digits = key.chars().all(|c| c.is_ascii_alphanumeric());

        if has_valid_length && starts_with_letter && has_only_letters_and_digits {
            Ok(Self(key))
        } else {
            Err(DomainError::InvalidProjectKey {
                value: raw.to_owned(),
            })
        }
    }

    /// Suggests a key for a project name.
    ///
    /// The initials of the words are used when there are at least two
    /// ("Smart Sensor Node" gives `SSN`). Otherwise the first letters of the
    /// name are used ("Thermostat" gives `THE`).
    #[must_use]
    pub fn suggest(name: &ProjectName) -> Self {
        // A key starts with a letter, so anything before the first letter of
        // the name cannot contribute to it.
        let from_first_letter = name
            .as_str()
            .trim_start_matches(|c: char| !c.is_ascii_alphabetic());

        let initials: String = from_first_letter
            .split_whitespace()
            .filter_map(|word| word.chars().find(char::is_ascii_alphanumeric))
            .take(Self::MAX_LENGTH)
            .collect();
        let first_letters: String = from_first_letter
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(Self::SUGGESTED_PREFIX_LENGTH)
            .collect();

        Self::parse(&initials)
            .or_else(|_| Self::parse(&first_letters))
            .unwrap_or_else(|_| Self(Self::FALLBACK.to_owned()))
    }

    /// Returns the key as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The key with a number appended, the first one that `is_taken` does
    /// not refuse: `IOT2`, `IOT3`, … A key that ends in a number already
    /// is counted on (`IOT2` gives `IOT3`). Letters are dropped from the end
    /// as needed to stay within the length limit. The key itself comes back
    /// if it is free, and also, unchanged, in the unlikely case that every
    /// number up to 9999 is taken.
    #[must_use]
    pub fn first_free(&self, is_taken: impl Fn(&Self) -> bool) -> Self {
        if !is_taken(self) {
            return self.clone();
        }
        let without_number = self.0.trim_end_matches(|c: char| c.is_ascii_digit());
        let stem = if without_number.len() >= Self::MIN_LENGTH {
            without_number
        } else {
            self.0.as_str()
        };
        (2..=Self::LAST_SUFFIX)
            .filter_map(|number| {
                let suffix = number.to_string();
                let letters_kept = Self::MAX_LENGTH.saturating_sub(suffix.len());
                let stem: String = stem.chars().take(letters_kept).collect();
                Self::parse(&format!("{stem}{suffix}")).ok()
            })
            .find(|candidate| !is_taken(candidate))
            .unwrap_or_else(|| self.clone())
    }
}

impl fmt::Display for ProjectKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The plain data of a project, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    /// The identifier of the project.
    pub id: ProjectId,
    /// The prefix of its task numbers.
    pub key: ProjectKey,
    /// Its name.
    pub name: ProjectName,
    /// A free-form description. May be empty.
    pub description: String,
    /// The statuses its tasks move through.
    pub workflow: Workflow,
    /// The number the next new task will get.
    pub next_task_number: u32,
    /// The number the next new sprint will get.
    pub next_sprint_number: u32,
    /// When the project was created.
    pub created_at: Timestamp,
    /// When the project was archived, if it was.
    pub archived_at: Option<Timestamp>,
}

/// A piece of work with a goal, a workflow and tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    id: ProjectId,
    key: ProjectKey,
    name: ProjectName,
    description: String,
    workflow: Workflow,
    next_task_number: u32,
    next_sprint_number: u32,
    created_at: Timestamp,
    archived_at: Option<Timestamp>,
}

impl Project {
    /// The number of the first task of every project.
    const FIRST_TASK_NUMBER: u32 = 1;
    /// The number of the first sprint of every project.
    const FIRST_SPRINT_NUMBER: u32 = 1;

    /// Starts a new project.
    #[must_use]
    pub fn create(
        name: ProjectName,
        key: ProjectKey,
        description: String,
        workflow: Workflow,
        now: Timestamp,
    ) -> Self {
        Self {
            id: ProjectId::generate(),
            key,
            name,
            description,
            workflow,
            next_task_number: Self::FIRST_TASK_NUMBER,
            next_sprint_number: Self::FIRST_SPRINT_NUMBER,
            created_at: now,
            archived_at: None,
        }
    }

    /// Recreates a project from stored data.
    #[must_use]
    pub fn restore(snapshot: ProjectSnapshot) -> Self {
        Self {
            id: snapshot.id,
            key: snapshot.key,
            name: snapshot.name,
            description: snapshot.description,
            workflow: snapshot.workflow,
            next_task_number: snapshot.next_task_number.max(Self::FIRST_TASK_NUMBER),
            next_sprint_number: snapshot.next_sprint_number.max(Self::FIRST_SPRINT_NUMBER),
            created_at: snapshot.created_at,
            archived_at: snapshot.archived_at,
        }
    }

    /// The identifier of the project.
    #[must_use]
    pub const fn id(&self) -> ProjectId {
        self.id
    }

    /// The prefix of the project's task numbers.
    #[must_use]
    pub const fn key(&self) -> &ProjectKey {
        &self.key
    }

    /// The name of the project.
    #[must_use]
    pub const fn name(&self) -> &ProjectName {
        &self.name
    }

    /// The free-form description of the project. May be empty.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The statuses the project's tasks move through.
    #[must_use]
    pub const fn workflow(&self) -> &Workflow {
        &self.workflow
    }

    /// The number the next new task will get.
    #[must_use]
    pub const fn next_task_number(&self) -> u32 {
        self.next_task_number
    }

    /// The number the next new sprint will get.
    #[must_use]
    pub const fn next_sprint_number(&self) -> u32 {
        self.next_sprint_number
    }

    /// When the project was created.
    #[must_use]
    pub const fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// When the project was archived, if it was.
    #[must_use]
    pub const fn archived_at(&self) -> Option<Timestamp> {
        self.archived_at
    }

    /// Whether the project has been put away.
    #[must_use]
    pub const fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// Gives the project a new name.
    pub fn rename(&mut self, name: ProjectName) {
        self.name = name;
    }

    /// Gives the project another key. Only keeping two computers in step
    /// does this: when both gave one key to two projects at the same time,
    /// one of them gets another.
    pub fn rekey(&mut self, key: ProjectKey) {
        self.key = key;
    }

    /// Replaces the description of the project.
    pub fn describe(&mut self, description: String) {
        self.description = description;
    }

    /// Hands out the number for a new task and moves the counter on.
    ///
    /// Numbers are never reused, so `IOT-12` always means the same task.
    pub fn allocate_task_number(&mut self) -> u32 {
        let number = self.next_task_number;
        self.next_task_number = self.next_task_number.saturating_add(1);
        number
    }

    /// Hands out the number for a new sprint and moves the counter on.
    ///
    /// A sprint that is left without a name is called after its number.
    /// Numbers are never reused, so a deleted sprint still counts.
    pub fn allocate_sprint_number(&mut self) -> u32 {
        let number = self.next_sprint_number;
        self.next_sprint_number = self.next_sprint_number.saturating_add(1);
        number
    }

    /// Puts the project away. Archiving an archived project keeps the
    /// original time.
    pub fn archive(&mut self, now: Timestamp) {
        self.archived_at.get_or_insert(now);
    }

    /// Brings an archived project back.
    pub fn unarchive(&mut self) {
        self.archived_at = None;
    }

    /// Adds a status to the workflow, at the place [`Workflow::add_status`]
    /// chooses.
    ///
    /// # Errors
    ///
    /// Returns an error if another status already has the name.
    pub fn add_status(&mut self, status: Status) -> Result<(), DomainError> {
        self.workflow.add_status(status)
    }

    /// Gives a status of the workflow a new name, category and color.
    ///
    /// The tasks in the status are not touched here. If the status starts or
    /// stops being a "done" status, the caller completes or reopens them.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status, if another
    /// status already has the name, or if the workflow would be left without
    /// a "todo" or a "done" status.
    pub fn change_status(
        &mut self,
        id: StatusId,
        name: StatusName,
        category: StatusCategory,
        color: Color,
    ) -> Result<(), DomainError> {
        self.workflow.change_status(id, name, category, color)
    }

    /// Moves a status of the workflow to another place.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status.
    pub fn move_status(&mut self, id: StatusId, index: usize) -> Result<(), DomainError> {
        self.workflow.move_status(id, index)
    }

    /// Takes a status out of the workflow and returns it. The caller finds
    /// another status for the tasks that were in it.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status, or if it would be
    /// left without a "todo" or a "done" status.
    pub fn remove_status(&mut self, id: StatusId) -> Result<Status, DomainError> {
        self.workflow.remove_status(id)
    }
}

#[cfg(test)]
mod sync_tests {
    use super::*;

    #[test]
    fn a_taken_key_gets_the_first_free_number() {
        let key = ProjectKey::parse("SSN").unwrap();
        let taken = ["SSN", "SSN2"];

        let free = key.first_free(|candidate| taken.contains(&candidate.as_str()));

        assert_eq!(free.as_str(), "SSN3");
        assert_eq!(key.first_free(|_| false), key);
        let numbered = ProjectKey::parse("SSN2").unwrap();
        assert_eq!(
            numbered
                .first_free(|candidate| taken.contains(&candidate.as_str()))
                .as_str(),
            "SSN3"
        );
    }

    #[test]
    fn a_long_key_drops_letters_for_its_number() {
        let key = ProjectKey::parse("SENSOR").unwrap();

        assert_eq!(
            key.first_free(|candidate| candidate == &key).as_str(),
            "SENSO2"
        );
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn moment(day: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, day, 9, 0, 0)
            .unwrap()
    }

    fn project(name: &str) -> Project {
        let name = ProjectName::parse(name).unwrap();
        let key = ProjectKey::suggest(&name);
        Project::create(name, key, String::new(), Workflow::standard(), moment(11))
    }

    #[test]
    fn a_key_is_stored_in_capitals() {
        assert_eq!(ProjectKey::parse("iot").unwrap().as_str(), "IOT");
    }

    #[test]
    fn a_key_may_contain_digits_after_the_first_letter() {
        assert_eq!(ProjectKey::parse("V2").unwrap().as_str(), "V2");
    }

    #[test]
    fn keys_of_the_wrong_shape_are_rejected() {
        for invalid in ["", "A", "TOOLONG", "1AB", "A-B", "A B", "ÄB"] {
            assert_eq!(
                ProjectKey::parse(invalid),
                Err(DomainError::InvalidProjectKey {
                    value: invalid.to_owned()
                }),
                "'{invalid}' should not be accepted as a key"
            );
        }
    }

    #[test]
    fn a_key_is_suggested_from_the_initials_of_the_name() {
        let name = ProjectName::parse("Smart sensor node").unwrap();

        assert_eq!(ProjectKey::suggest(&name).as_str(), "SSN");
    }

    #[test]
    fn a_one_word_name_gives_its_first_three_letters() {
        let name = ProjectName::parse("Thermostat").unwrap();

        assert_eq!(ProjectKey::suggest(&name).as_str(), "THE");
    }

    #[test]
    fn a_suggested_key_skips_anything_before_the_first_letter() {
        let name = ProjectName::parse("3D printer case").unwrap();

        assert_eq!(ProjectKey::suggest(&name).as_str(), "DPC");
    }

    #[test]
    fn a_long_name_gives_a_key_of_at_most_the_maximum_length() {
        let name = ProjectName::parse("A very long name with a lot of words in it").unwrap();

        assert_eq!(ProjectKey::suggest(&name).as_str(), "AVLNWA");
    }

    #[test]
    fn a_name_without_letters_gets_the_fallback_key() {
        let name = ProjectName::parse("2026 – 11").unwrap();

        assert_eq!(ProjectKey::suggest(&name).as_str(), "PRJ");
    }

    #[test]
    fn task_numbers_start_at_one_and_are_never_handed_out_twice() {
        let mut project = project("Smart sensor node");

        let numbers = [
            project.allocate_task_number(),
            project.allocate_task_number(),
            project.allocate_task_number(),
        ];

        assert_eq!(numbers, [1, 2, 3]);
        assert_eq!(project.next_task_number(), 4);
    }

    #[test]
    fn sprint_numbers_start_at_one_and_are_never_handed_out_twice() {
        let mut project = project("Smart sensor node");

        let numbers = [
            project.allocate_sprint_number(),
            project.allocate_sprint_number(),
        ];

        assert_eq!(numbers, [1, 2]);
        assert_eq!(project.next_sprint_number(), 3);
        assert_eq!(project.next_task_number(), 1);
    }

    #[test]
    fn a_new_project_is_not_archived() {
        assert!(!project("Smart sensor node").is_archived());
    }

    #[test]
    fn archiving_twice_keeps_the_time_of_the_first_archiving() {
        let mut project = project("Smart sensor node");

        project.archive(moment(12));
        project.archive(moment(20));

        assert_eq!(project.archived_at(), Some(moment(12)));
    }

    #[test]
    fn an_archived_project_can_be_brought_back() {
        let mut project = project("Smart sensor node");
        project.archive(moment(12));

        project.unarchive();

        assert!(!project.is_archived());
    }

    #[test]
    fn the_statuses_of_a_project_can_be_added_changed_moved_and_removed() {
        let mut project = project("Smart sensor node");
        let blocked = Status::new(
            StatusName::parse("Blocked").unwrap(),
            StatusCategory::Active,
            Color::Red,
        );
        let blocked_id = blocked.id();

        project.add_status(blocked).unwrap();
        project
            .change_status(
                blocked_id,
                StatusName::parse("On hold").unwrap(),
                StatusCategory::Active,
                Color::Pink,
            )
            .unwrap();
        project.move_status(blocked_id, 0).unwrap();
        let first = project.workflow().statuses()[0].clone();
        let removed = project.remove_status(blocked_id).unwrap();

        assert_eq!(first.name().as_str(), "On hold");
        assert_eq!(first.color(), Color::Pink);
        assert_eq!(removed, first);
        assert_eq!(project.workflow().statuses().len(), 4);
        assert!(project.workflow().status(blocked_id).is_none());
    }

    #[test]
    fn a_restored_project_equals_the_one_that_was_stored() {
        let original = project("Smart sensor node");

        let restored = Project::restore(ProjectSnapshot {
            id: original.id(),
            key: original.key().clone(),
            name: original.name().clone(),
            description: original.description().to_owned(),
            workflow: original.workflow().clone(),
            next_task_number: original.next_task_number(),
            next_sprint_number: original.next_sprint_number(),
            created_at: original.created_at(),
            archived_at: original.archived_at(),
        });

        assert_eq!(restored, original);
    }
}
