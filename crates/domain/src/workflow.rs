//! Workflows: the statuses a task moves through.

use std::collections::HashSet;

use crate::error::DomainError;
use crate::ids::StatusId;
use crate::macros::text_enum;
use crate::text::StatusName;

/// What a status means for the progress of a task, whatever it is called.
///
/// A manager is free to name statuses ("In test", "Waiting for parts"). The
/// program still has to know whether work has started or finished, and the
/// category tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusCategory {
    /// Work has not started.
    Todo,
    /// Work is under way.
    Active,
    /// Work is finished and waits to be checked or accepted.
    Review,
    /// Work is accepted. Nothing more will happen to the task.
    Done,
}

text_enum!(StatusCategory, kind = "status category", {
    Todo => "todo",
    Active => "active",
    Review => "review",
    Done => "done",
});

/// The color of a status, by name.
///
/// The domain only says which color. The interface decides the exact shade
/// for its light and its dark theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusColor {
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

text_enum!(StatusColor, kind = "status color", {
    Gray => "gray",
    Blue => "blue",
    Purple => "purple",
    Amber => "amber",
    Green => "green",
    Red => "red",
    Pink => "pink",
    Teal => "teal",
});

/// One step of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    id: StatusId,
    name: StatusName,
    category: StatusCategory,
    color: StatusColor,
}

impl Status {
    /// Creates a status with a new identifier.
    #[must_use]
    pub fn new(name: StatusName, category: StatusCategory, color: StatusColor) -> Self {
        Self::restore(StatusId::generate(), name, category, color)
    }

    /// Recreates a status that already has an identifier, for example one
    /// read from storage.
    #[must_use]
    pub const fn restore(
        id: StatusId,
        name: StatusName,
        category: StatusCategory,
        color: StatusColor,
    ) -> Self {
        Self {
            id,
            name,
            category,
            color,
        }
    }

    /// The identifier of this status.
    #[must_use]
    pub const fn id(&self) -> StatusId {
        self.id
    }

    /// The name the manager sees.
    #[must_use]
    pub const fn name(&self) -> &StatusName {
        &self.name
    }

    /// What the status means for the progress of a task.
    #[must_use]
    pub const fn category(&self) -> StatusCategory {
        self.category
    }

    /// The color the status is shown in.
    #[must_use]
    pub const fn color(&self) -> StatusColor {
        self.color
    }
}

/// The ordered list of statuses that the tasks of a project move through.
///
/// A workflow always has at least one status in which work has not started
/// and at least one in which it is done, and no two of its statuses share a
/// name. Everything that schedules or reports on tasks relies on this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workflow {
    statuses: Vec<Status>,
}

impl Workflow {
    /// The categories no workflow can do without.
    const REQUIRED_CATEGORIES: [StatusCategory; 2] = [StatusCategory::Todo, StatusCategory::Done];

    /// Creates a workflow from statuses in the order they should appear.
    ///
    /// # Errors
    ///
    /// Returns an error if no status is in the "todo" category, if none is in
    /// the "done" category, or if two statuses have the same name. Names are
    /// compared without regard to upper and lower case.
    pub fn new(statuses: Vec<Status>) -> Result<Self, DomainError> {
        for category in Self::REQUIRED_CATEGORIES {
            if !statuses.iter().any(|status| status.category == category) {
                return Err(DomainError::MissingStatusCategory { category });
            }
        }

        let mut names_seen = HashSet::new();
        for status in &statuses {
            if !names_seen.insert(status.name.as_str().to_lowercase()) {
                return Err(DomainError::DuplicateStatusName {
                    name: status.name.to_string(),
                });
            }
        }

        Ok(Self { statuses })
    }

    /// The default workflow: to do, in progress, in review, done.
    #[must_use]
    pub fn standard() -> Self {
        Self::from_template(&[
            ("To do", StatusCategory::Todo, StatusColor::Gray),
            ("In progress", StatusCategory::Active, StatusColor::Blue),
            ("In review", StatusCategory::Review, StatusColor::Amber),
            ("Done", StatusCategory::Done, StatusColor::Green),
        ])
    }

    /// A workflow for engineering work that is reworked and tested before it
    /// is accepted: to do, in progress, in revision, in test, done.
    #[must_use]
    pub fn engineering() -> Self {
        Self::from_template(&[
            ("To do", StatusCategory::Todo, StatusColor::Gray),
            ("In progress", StatusCategory::Active, StatusColor::Blue),
            ("In revision", StatusCategory::Active, StatusColor::Purple),
            ("In test", StatusCategory::Review, StatusColor::Amber),
            ("Done", StatusCategory::Done, StatusColor::Green),
        ])
    }

    /// Builds one of the workflows that ship with the program. The templates
    /// are written above and are known to follow the rules; a test passes
    /// each of them through [`Workflow::new`] to prove it.
    fn from_template(template: &[(&'static str, StatusCategory, StatusColor)]) -> Self {
        let statuses = template
            .iter()
            .map(|&(name, category, color)| {
                Status::new(StatusName::from_trusted(name), category, color)
            })
            .collect();
        Self { statuses }
    }

    /// The statuses in the order they appear on a board.
    #[must_use]
    pub fn statuses(&self) -> &[Status] {
        &self.statuses
    }

    /// Looks a status up by its identifier.
    #[must_use]
    pub fn status(&self, id: StatusId) -> Option<&Status> {
        self.statuses.iter().find(|status| status.id == id)
    }

    /// Looks a status up by its identifier and treats its absence as an
    /// error.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status.
    pub fn require_status(&self, id: StatusId) -> Result<&Status, DomainError> {
        self.status(id)
            .ok_or(DomainError::UnknownStatus { status_id: id })
    }

    /// The first status of a category, in board order.
    #[must_use]
    pub fn first_in(&self, category: StatusCategory) -> Option<&Status> {
        self.statuses
            .iter()
            .find(|status| status.category == category)
    }

    /// The status a new task starts in: the first one in which work has not
    /// started.
    #[must_use]
    pub fn initial_status(&self) -> &Status {
        // Every workflow has a "todo" status, so the search always succeeds.
        // The first status is named as the alternative only to keep this
        // function free of a panic path.
        self.first_in(StatusCategory::Todo)
            .unwrap_or(&self.statuses[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(name: &str, category: StatusCategory) -> Status {
        Status::new(
            StatusName::parse(name).unwrap(),
            category,
            StatusColor::Gray,
        )
    }

    #[test]
    fn a_workflow_keeps_its_statuses_in_the_given_order() {
        let workflow = Workflow::new(vec![
            status("Backlog", StatusCategory::Todo),
            status("Doing", StatusCategory::Active),
            status("Shipped", StatusCategory::Done),
        ])
        .unwrap();

        let names: Vec<&str> = workflow
            .statuses()
            .iter()
            .map(|status| status.name().as_str())
            .collect();

        assert_eq!(names, ["Backlog", "Doing", "Shipped"]);
    }

    #[test]
    fn a_workflow_without_a_todo_status_is_rejected() {
        let error = Workflow::new(vec![
            status("Doing", StatusCategory::Active),
            status("Shipped", StatusCategory::Done),
        ])
        .unwrap_err();

        assert_eq!(
            error,
            DomainError::MissingStatusCategory {
                category: StatusCategory::Todo
            }
        );
    }

    #[test]
    fn a_workflow_without_a_done_status_is_rejected() {
        let error = Workflow::new(vec![
            status("Backlog", StatusCategory::Todo),
            status("Doing", StatusCategory::Active),
        ])
        .unwrap_err();

        assert_eq!(
            error,
            DomainError::MissingStatusCategory {
                category: StatusCategory::Done
            }
        );
    }

    #[test]
    fn two_statuses_whose_names_differ_only_in_case_are_rejected() {
        let error = Workflow::new(vec![
            status("Backlog", StatusCategory::Todo),
            status("BACKLOG", StatusCategory::Active),
            status("Shipped", StatusCategory::Done),
        ])
        .unwrap_err();

        assert_eq!(
            error,
            DomainError::DuplicateStatusName {
                name: "BACKLOG".to_owned()
            }
        );
    }

    #[test]
    fn every_built_in_workflow_follows_the_rules() {
        for template in [Workflow::standard(), Workflow::engineering()] {
            for status in template.statuses() {
                assert!(StatusName::parse(status.name().as_str()).is_ok());
            }
            assert!(Workflow::new(template.statuses().to_vec()).is_ok());
        }
    }

    #[test]
    fn a_new_task_starts_in_the_first_todo_status() {
        let workflow = Workflow::new(vec![
            status("Doing", StatusCategory::Active),
            status("Ideas", StatusCategory::Todo),
            status("Ready", StatusCategory::Todo),
            status("Shipped", StatusCategory::Done),
        ])
        .unwrap();

        assert_eq!(workflow.initial_status().name().as_str(), "Ideas");
    }

    #[test]
    fn a_status_is_found_by_its_identifier() {
        let workflow = Workflow::standard();
        let in_review = workflow.first_in(StatusCategory::Review).unwrap();

        assert_eq!(workflow.status(in_review.id()), Some(in_review));
    }

    #[test]
    fn requiring_a_status_the_workflow_lacks_is_an_error() {
        let workflow = Workflow::standard();
        let foreign_id = StatusId::generate();

        let error = workflow.require_status(foreign_id).unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownStatus {
                status_id: foreign_id
            }
        );
    }

    #[test]
    fn a_category_survives_being_written_as_text_and_read_back() {
        for &category in StatusCategory::ALL {
            assert_eq!(
                category.as_str().parse::<StatusCategory>().unwrap(),
                category
            );
        }
    }

    #[test]
    fn text_that_names_no_category_is_rejected() {
        let error = "paused".parse::<StatusCategory>().unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownValue {
                kind: "status category",
                value: "paused".to_owned()
            }
        );
    }
}
