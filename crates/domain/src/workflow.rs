//! Workflows: the statuses a task moves through.

use std::collections::HashSet;

use crate::color::Color;
use crate::error::DomainError;
use crate::ids::StatusId;
use crate::macros::text_enum;
use crate::text::StatusName;

/// What a status means for the progress of a task, whatever it is called.
///
/// A manager is free to name statuses ("In test", "Waiting for parts"). The
/// program still has to know whether work has started or finished, and the
/// category tells it.
///
/// Categories are ordered by how far along the work is: todo comes first,
/// then active, review and done. The order of the variants below is that
/// order, and comparisons rely on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

/// One step of a workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    id: StatusId,
    name: StatusName,
    category: StatusCategory,
    color: Color,
}

impl Status {
    /// Creates a status with a new identifier.
    #[must_use]
    pub fn new(name: StatusName, category: StatusCategory, color: Color) -> Self {
        Self::restore(StatusId::generate(), name, category, color)
    }

    /// Recreates a status that already has an identifier, for example one
    /// read from storage.
    #[must_use]
    pub const fn restore(
        id: StatusId,
        name: StatusName,
        category: StatusCategory,
        color: Color,
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
    pub const fn color(&self) -> Color {
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
        Self::require_categories(statuses.iter().map(Status::category))?;

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
            ("To do", StatusCategory::Todo, Color::Gray),
            ("In progress", StatusCategory::Active, Color::Blue),
            ("In review", StatusCategory::Review, Color::Amber),
            ("Done", StatusCategory::Done, Color::Green),
        ])
    }

    /// A workflow for engineering work that is reworked and tested before it
    /// is accepted: to do, in progress, in revision, in test, done.
    #[must_use]
    pub fn engineering() -> Self {
        Self::from_template(&[
            ("To do", StatusCategory::Todo, Color::Gray),
            ("In progress", StatusCategory::Active, Color::Blue),
            ("In revision", StatusCategory::Active, Color::Purple),
            ("In test", StatusCategory::Review, Color::Amber),
            ("Done", StatusCategory::Done, Color::Green),
        ])
    }

    /// Builds one of the workflows that ship with the program. The templates
    /// are written above and are known to follow the rules; a test passes
    /// each of them through [`Workflow::new`] to prove it.
    fn from_template(template: &[(&'static str, StatusCategory, Color)]) -> Self {
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

    /// Adds a status. It goes behind the last status that is as far along
    /// in the course of the work, or less far, so that a new step of the
    /// work stands with the other steps and not behind the statuses in which
    /// work is done.
    ///
    /// # Errors
    ///
    /// Returns an error if another status already has the name. Names are
    /// compared without regard to upper and lower case.
    pub fn add_status(&mut self, status: Status) -> Result<(), DomainError> {
        self.refuse_name_in_use(&status.name, None)?;
        let place = self.place_for_new_status(status.category);
        self.statuses.insert(place, status);
        Ok(())
    }

    /// Gives a status a new name, category and color in one step. The
    /// status keeps its place and its identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status, if another
    /// status already has the name, or if the change would leave the
    /// workflow without a "todo" or a "done" status.
    pub fn change_status(
        &mut self,
        id: StatusId,
        name: StatusName,
        category: StatusCategory,
        color: Color,
    ) -> Result<(), DomainError> {
        let position = self.position_of(id)?;
        self.refuse_name_in_use(&name, Some(id))?;
        Self::require_categories(self.statuses.iter().map(|status| {
            if status.id == id {
                category
            } else {
                status.category
            }
        }))?;

        let status = &mut self.statuses[position];
        status.name = name;
        status.category = category;
        status.color = color;
        Ok(())
    }

    /// Moves a status to a place among the other statuses. A place past the
    /// end means the last place.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status.
    pub fn move_status(&mut self, id: StatusId, index: usize) -> Result<(), DomainError> {
        let position = self.position_of(id)?;
        let status = self.statuses.remove(position);
        let index = index.min(self.statuses.len());
        self.statuses.insert(index, status);
        Ok(())
    }

    /// Takes a status out of the workflow and returns it.
    ///
    /// The workflow does not know about tasks. Whoever removes a status has
    /// to find another one for the tasks that are in it.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow has no such status, or if it would be
    /// left without a "todo" or a "done" status.
    pub fn remove_status(&mut self, id: StatusId) -> Result<Status, DomainError> {
        let position = self.position_of(id)?;
        Self::require_categories(
            self.statuses
                .iter()
                .filter(|status| status.id != id)
                .map(Status::category),
        )?;
        Ok(self.statuses.remove(position))
    }

    /// Checks that statuses of the given categories make a usable workflow:
    /// at least one in which work has not started and one in which it is
    /// done.
    fn require_categories(
        categories: impl Iterator<Item = StatusCategory> + Clone,
    ) -> Result<(), DomainError> {
        for category in Self::REQUIRED_CATEGORIES {
            if !categories.clone().any(|present| present == category) {
                return Err(DomainError::MissingStatusCategory { category });
            }
        }
        Ok(())
    }

    /// Refuses a name that a status of the workflow already has, except the
    /// status that is being changed, which may keep its own name.
    fn refuse_name_in_use(
        &self,
        name: &StatusName,
        changed: Option<StatusId>,
    ) -> Result<(), DomainError> {
        let wanted = name.as_str().to_lowercase();
        let in_use = self.statuses.iter().any(|status| {
            Some(status.id) != changed && status.name.as_str().to_lowercase() == wanted
        });
        if in_use {
            return Err(DomainError::DuplicateStatusName {
                name: name.to_string(),
            });
        }
        Ok(())
    }

    /// The index at which a new status of a category goes: behind the last
    /// status that is as far along, or less far.
    fn place_for_new_status(&self, category: StatusCategory) -> usize {
        self.statuses
            .iter()
            .rposition(|status| status.category <= category)
            .map_or(0, |position| position + 1)
    }

    /// The index of a status in the board order.
    fn position_of(&self, id: StatusId) -> Result<usize, DomainError> {
        self.statuses
            .iter()
            .position(|status| status.id == id)
            .ok_or(DomainError::UnknownStatus { status_id: id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(name: &str, category: StatusCategory) -> Status {
        Status::new(StatusName::parse(name).unwrap(), category, Color::Gray)
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

    fn names(workflow: &Workflow) -> Vec<&str> {
        workflow
            .statuses()
            .iter()
            .map(|status| status.name().as_str())
            .collect()
    }

    fn named<'workflow>(workflow: &'workflow Workflow, name: &str) -> &'workflow Status {
        workflow
            .statuses()
            .iter()
            .find(|status| status.name().as_str() == name)
            .unwrap()
    }

    fn id_of(workflow: &Workflow, name: &str) -> StatusId {
        named(workflow, name).id()
    }

    #[test]
    fn categories_are_ordered_by_how_far_along_the_work_is() {
        assert!(StatusCategory::Todo < StatusCategory::Active);
        assert!(StatusCategory::Active < StatusCategory::Review);
        assert!(StatusCategory::Review < StatusCategory::Done);
    }

    #[test]
    fn a_new_step_of_the_work_goes_behind_the_other_steps_in_front_of_review_and_done() {
        let mut workflow = Workflow::engineering();

        workflow
            .add_status(status("Waiting for parts", StatusCategory::Active))
            .unwrap();

        assert_eq!(
            names(&workflow),
            [
                "To do",
                "In progress",
                "In revision",
                "Waiting for parts",
                "In test",
                "Done"
            ]
        );
    }

    #[test]
    fn a_new_todo_status_goes_behind_the_other_todo_statuses() {
        let mut workflow = Workflow::engineering();

        workflow
            .add_status(status("Backlog", StatusCategory::Todo))
            .unwrap();

        assert_eq!(names(&workflow)[..3], ["To do", "Backlog", "In progress"]);
    }

    #[test]
    fn a_second_done_status_goes_to_the_end() {
        let mut workflow = Workflow::engineering();

        workflow
            .add_status(status("Cancelled", StatusCategory::Done))
            .unwrap();

        assert_eq!(names(&workflow).last(), Some(&"Cancelled"));
    }

    #[test]
    fn a_new_status_follows_the_last_status_not_further_along_wherever_it_stands() {
        // A manager may have put a todo status behind the done status.
        let mut workflow = Workflow::new(vec![
            status("Doing", StatusCategory::Active),
            status("Shipped", StatusCategory::Done),
            status("Ideas", StatusCategory::Todo),
        ])
        .unwrap();

        workflow
            .add_status(status("Waiting", StatusCategory::Active))
            .unwrap();

        assert_eq!(names(&workflow), ["Doing", "Shipped", "Ideas", "Waiting"]);
    }

    #[test]
    fn a_new_status_may_not_take_a_name_in_use_whatever_the_capitals() {
        let mut workflow = Workflow::engineering();

        let error = workflow
            .add_status(status("in TEST", StatusCategory::Review))
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::DuplicateStatusName {
                name: "in TEST".to_owned()
            }
        );
        assert_eq!(workflow.statuses().len(), 5);
    }

    #[test]
    fn changing_a_status_replaces_its_name_category_and_color_in_place() {
        let mut workflow = Workflow::engineering();
        let in_test = id_of(&workflow, "In test");

        workflow
            .change_status(
                in_test,
                StatusName::parse("On the bench").unwrap(),
                StatusCategory::Active,
                Color::Teal,
            )
            .unwrap();

        let changed = workflow.status(in_test).unwrap();
        assert_eq!(changed.name().as_str(), "On the bench");
        assert_eq!(changed.category(), StatusCategory::Active);
        assert_eq!(changed.color(), Color::Teal);
        assert_eq!(names(&workflow)[3], "On the bench");
    }

    #[test]
    fn a_changed_status_may_keep_its_own_name() {
        let mut workflow = Workflow::engineering();
        let in_test = id_of(&workflow, "In test");

        workflow
            .change_status(
                in_test,
                StatusName::parse("IN TEST").unwrap(),
                StatusCategory::Review,
                Color::Red,
            )
            .unwrap();

        assert_eq!(workflow.status(in_test).unwrap().color(), Color::Red);
    }

    #[test]
    fn a_changed_status_may_not_take_the_name_of_another() {
        let mut workflow = Workflow::engineering();
        let in_test = id_of(&workflow, "In test");

        let error = workflow
            .change_status(
                in_test,
                StatusName::parse("Done").unwrap(),
                StatusCategory::Review,
                Color::Amber,
            )
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::DuplicateStatusName {
                name: "Done".to_owned()
            }
        );
    }

    #[test]
    fn the_only_todo_and_the_only_done_status_keep_their_categories() {
        let mut workflow = Workflow::engineering();
        let original = workflow.clone();

        let todo_error = workflow
            .change_status(
                id_of(&original, "To do"),
                StatusName::parse("To do").unwrap(),
                StatusCategory::Active,
                Color::Gray,
            )
            .unwrap_err();
        let done_error = workflow
            .change_status(
                id_of(&original, "Done"),
                StatusName::parse("Done").unwrap(),
                StatusCategory::Review,
                Color::Green,
            )
            .unwrap_err();

        assert_eq!(
            todo_error,
            DomainError::MissingStatusCategory {
                category: StatusCategory::Todo
            }
        );
        assert_eq!(
            done_error,
            DomainError::MissingStatusCategory {
                category: StatusCategory::Done
            }
        );
        assert_eq!(workflow, original);
    }

    #[test]
    fn changing_a_status_the_workflow_lacks_is_an_error() {
        let mut workflow = Workflow::standard();
        let foreign_id = StatusId::generate();

        let error = workflow
            .change_status(
                foreign_id,
                StatusName::parse("Elsewhere").unwrap(),
                StatusCategory::Active,
                Color::Gray,
            )
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::UnknownStatus {
                status_id: foreign_id
            }
        );
    }

    #[test]
    fn a_status_moves_to_another_place() {
        let mut workflow = Workflow::engineering();

        workflow
            .move_status(id_of(&workflow, "In test"), 1)
            .unwrap();

        assert_eq!(
            names(&workflow),
            ["To do", "In test", "In progress", "In revision", "Done"]
        );
    }

    #[test]
    fn a_place_past_the_end_means_the_last_place() {
        let mut workflow = Workflow::engineering();

        workflow.move_status(id_of(&workflow, "To do"), 99).unwrap();

        assert_eq!(names(&workflow).last(), Some(&"To do"));
    }

    #[test]
    fn moving_a_status_the_workflow_lacks_is_an_error() {
        let mut workflow = Workflow::standard();

        assert!(matches!(
            workflow.move_status(StatusId::generate(), 0),
            Err(DomainError::UnknownStatus { .. })
        ));
    }

    #[test]
    fn a_removed_status_is_taken_out_and_returned() {
        let mut workflow = Workflow::engineering();
        let in_revision = id_of(&workflow, "In revision");

        let removed = workflow.remove_status(in_revision).unwrap();

        assert_eq!(removed.id(), in_revision);
        assert_eq!(
            names(&workflow),
            ["To do", "In progress", "In test", "Done"]
        );
    }

    #[test]
    fn the_only_todo_status_cannot_be_removed() {
        let mut workflow = Workflow::engineering();

        let error = workflow
            .remove_status(id_of(&workflow, "To do"))
            .unwrap_err();

        assert_eq!(
            error,
            DomainError::MissingStatusCategory {
                category: StatusCategory::Todo
            }
        );
        assert_eq!(workflow.statuses().len(), 5);
    }

    #[test]
    fn a_done_status_can_be_removed_while_another_remains() {
        let mut workflow = Workflow::engineering();
        workflow
            .add_status(status("Cancelled", StatusCategory::Done))
            .unwrap();

        workflow.remove_status(id_of(&workflow, "Done")).unwrap();

        assert_eq!(names(&workflow).last(), Some(&"Cancelled"));
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
