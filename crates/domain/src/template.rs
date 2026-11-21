//! The templates a new project can start from.
//!
//! A template decides what a project starts with: the statuses of its
//! workflow with their categories, colors and work-in-progress limits, the
//! views it starts with and their order, its areas, and whether a first
//! sprint is planned. It never decides a team: the Advisor proposes one.
//!
//! A template is a starting point only
//! ([decision 0033](../../../docs/decisions/0033-work-in-progress-limits-and-templates.md)).
//! The project it makes does not remember it, and everything it made can be
//! changed afterwards like anything the manager made by hand.
//!
//! The set is knowledge of the product, written here in code, like the
//! ready-made specialists.

use crate::color::Color;
use crate::text::{AreaName, StatusName};
use crate::view::ViewKind;
use crate::work_in_progress::WorkInProgressLimit;
use crate::workflow::{Status, StatusCategory, Workflow};

/// One status a template gives a workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemplateStatus {
    /// Its name.
    pub name: &'static str,
    /// What it means for the progress of a task.
    pub category: StatusCategory,
    /// The color it is shown in.
    pub color: Color,
    /// Its work-in-progress limit, if it has one.
    pub limit: Option<u16>,
}

/// What a new project can start from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectTemplate {
    /// What names the template in the program, such as `kanban`. It never
    /// changes once released.
    pub key: &'static str,
    /// What the manager reads, such as "Kanban".
    pub name: &'static str,
    /// What it is for, in two sentences for the manager.
    pub description: &'static str,
    /// The statuses of the workflow, in board order.
    pub statuses: &'static [TemplateStatus],
    /// The views the project starts with, in the order of its row of views.
    pub views: &'static [ViewKind],
    /// The names of its areas, in their order. The first takes the place of
    /// the area "General" every project otherwise starts with.
    pub areas: &'static [&'static str],
    /// Whether a first sprint is planned, empty and without dates.
    pub plans_first_sprint: bool,
}

/// A status of a template, written briefly.
const fn status(
    name: &'static str,
    category: StatusCategory,
    color: Color,
    limit: Option<u16>,
) -> TemplateStatus {
    TemplateStatus {
        name,
        category,
        color,
        limit,
    }
}

/// The views of a project that is not planned in sprints first.
const STARTING_VIEWS: &[ViewKind] = &ViewKind::STARTING;

/// The plain workflow: to do, in progress, in review, done.
pub const STANDARD: ProjectTemplate = ProjectTemplate {
    key: "standard",
    name: "Standard",
    description: "Four plain steps from to do to done, and every view a project starts with. \
                  Choose it when the project does not yet say how it wants to be run.",
    statuses: &[
        status("To do", StatusCategory::Todo, Color::Gray, None),
        status("In progress", StatusCategory::Active, Color::Blue, None),
        status("In review", StatusCategory::Review, Color::Amber, None),
        status("Done", StatusCategory::Done, Color::Green, None),
    ],
    views: STARTING_VIEWS,
    areas: &[AreaName::GENERAL],
    plans_first_sprint: false,
};

/// Work that is reworked and tested before it is accepted.
pub const ENGINEERING: ProjectTemplate = ProjectTemplate {
    key: "engineering",
    name: "Engineering",
    description: "Work that goes back for revision and is tested before it is accepted. \
                  Choose it for code or design that is checked by test rather than by eye.",
    statuses: &[
        status("To do", StatusCategory::Todo, Color::Gray, None),
        status("In progress", StatusCategory::Active, Color::Blue, None),
        status("In revision", StatusCategory::Active, Color::Purple, None),
        status("In test", StatusCategory::Review, Color::Amber, None),
        status("Done", StatusCategory::Done, Color::Green, None),
    ],
    views: STARTING_VIEWS,
    areas: &[AreaName::GENERAL],
    plans_first_sprint: false,
};

/// A continuous flow with limits on the work under way.
pub const KANBAN: ProjectTemplate = ProjectTemplate {
    key: "kanban",
    name: "Kanban",
    description: "A continuous flow of work without sprints, with a limit on how much stands \
                  in each step at once. Choose it when work arrives all the time and should \
                  be finished before more is started.",
    statuses: &[
        status("Backlog", StatusCategory::Todo, Color::Gray, None),
        status("Ready", StatusCategory::Todo, Color::Teal, Some(5)),
        status("In progress", StatusCategory::Active, Color::Blue, Some(3)),
        status("Review", StatusCategory::Review, Color::Amber, Some(2)),
        status("Done", StatusCategory::Done, Color::Green, None),
    ],
    views: &[
        ViewKind::Board,
        ViewKind::List,
        ViewKind::Calendar,
        ViewKind::Dashboard,
    ],
    areas: &[AreaName::GENERAL],
    plans_first_sprint: false,
};

/// Work planned in sprints from a backlog.
pub const SCRUM: ProjectTemplate = ProjectTemplate {
    key: "scrum",
    name: "Scrum",
    description: "Work planned from a backlog into sprints of one to four weeks, with the \
                  backlog first and a first sprint ready to fill. Choose it when the work \
                  is reviewed and planned again at a steady beat.",
    statuses: &[
        status("To do", StatusCategory::Todo, Color::Gray, None),
        status("In progress", StatusCategory::Active, Color::Blue, None),
        status("In review", StatusCategory::Review, Color::Amber, None),
        status("Done", StatusCategory::Done, Color::Green, None),
    ],
    views: &[
        ViewKind::Backlog,
        ViewKind::Board,
        ViewKind::List,
        ViewKind::Dashboard,
    ],
    areas: &[AreaName::GENERAL],
    plans_first_sprint: true,
};

/// A device built from electronics, firmware and an enclosure.
pub const HARDWARE_PRODUCT: ProjectTemplate = ProjectTemplate {
    key: "hardware",
    name: "Hardware product",
    description: "A device is designed, reviewed, ordered, built and tested, in areas for its \
                  electronics, its firmware and its enclosure. Choose it when the work ends \
                  in something that can be held in the hand.",
    statuses: &[
        status("To do", StatusCategory::Todo, Color::Gray, None),
        status("Design", StatusCategory::Active, Color::Blue, Some(4)),
        status(
            "Design review",
            StatusCategory::Review,
            Color::Amber,
            Some(2),
        ),
        status("Ordered", StatusCategory::Active, Color::Purple, None),
        status("Build", StatusCategory::Active, Color::Teal, Some(3)),
        status("Test", StatusCategory::Review, Color::Pink, Some(2)),
        status("Done", StatusCategory::Done, Color::Green, None),
    ],
    views: &[
        ViewKind::Board,
        ViewKind::Gantt,
        ViewKind::List,
        ViewKind::Calendar,
        ViewKind::Dashboard,
    ],
    areas: &["Electronics", "Firmware", "Enclosure"],
    plans_first_sprint: false,
};

impl ProjectTemplate {
    /// Every template, in the order the manager is shown them: the plain
    /// one first, then the two ways of running work, then the kinds of
    /// project.
    pub const ALL: &'static [Self] = &[STANDARD, KANBAN, SCRUM, HARDWARE_PRODUCT, ENGINEERING];

    /// Finds a template by its key.
    #[must_use]
    pub fn find(key: &str) -> Option<&'static Self> {
        Self::ALL.iter().find(|template| template.key == key)
    }

    /// A fresh workflow from the template, with new identifiers for its
    /// statuses.
    #[must_use]
    pub fn workflow(&self) -> Workflow {
        let statuses = self
            .statuses
            .iter()
            .map(|status| {
                let built = Status::new(
                    StatusName::from_trusted(status.name),
                    status.category,
                    status.color,
                );
                let limit = status.limit.map(WorkInProgressLimit::from_trusted);
                // A template never puts a limit on a done status; the test of
                // every template proves it. Should one, the status goes
                // without.
                built.clone().with_limit(limit).unwrap_or(built)
            })
            .collect();
        Workflow::from_trusted(statuses)
    }

    /// The names of the areas, checked. A template always has at least one;
    /// should it have none, the project gets "General".
    #[must_use]
    pub fn area_names(&self) -> Vec<AreaName> {
        let names: Vec<AreaName> = self
            .areas
            .iter()
            .filter_map(|name| AreaName::parse(name).ok())
            .collect();
        if names.is_empty() {
            vec![AreaName::general()]
        } else {
            names
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_template_makes_a_workflow_that_follows_the_rules() {
        for template in ProjectTemplate::ALL {
            let workflow = template.workflow();
            assert_eq!(workflow.statuses().len(), template.statuses.len());
            for status in workflow.statuses() {
                assert!(StatusName::parse(status.name().as_str()).is_ok());
            }
            assert!(
                Workflow::new(workflow.statuses().to_vec()).is_ok(),
                "the workflow of {} breaks a rule",
                template.name
            );
        }
    }

    #[test]
    fn every_limit_of_a_template_is_in_range_and_on_a_status_that_may_carry_one() {
        for template in ProjectTemplate::ALL {
            for status in template.statuses {
                if let Some(limit) = status.limit {
                    assert!(WorkInProgressLimit::new(i64::from(limit)).is_ok());
                    assert!(
                        WorkInProgressLimit::fits(status.category),
                        "{} of {} cannot carry a limit",
                        status.name,
                        template.name
                    );
                }
            }
            let built = template.workflow();
            let limits: Vec<Option<u16>> = built
                .statuses()
                .iter()
                .map(|status| status.limit().map(WorkInProgressLimit::value))
                .collect();
            let written: Vec<Option<u16>> = template.statuses.iter().map(|s| s.limit).collect();
            assert_eq!(limits, written);
        }
    }

    #[test]
    fn every_template_has_a_key_a_name_two_sentences_views_and_areas() {
        let mut keys = HashSet::new();
        for template in ProjectTemplate::ALL {
            assert!(keys.insert(template.key), "{} is used twice", template.key);
            assert!(!template.name.is_empty());
            assert_eq!(
                template.description.matches(". ").count() + 1,
                2,
                "the description of {} is not two sentences",
                template.name
            );
            assert!(template.description.ends_with('.'));
            assert!(!template.views.is_empty());
            let kinds: HashSet<ViewKind> = template.views.iter().copied().collect();
            assert_eq!(
                kinds.len(),
                template.views.len(),
                "{} repeats a view",
                template.name
            );
            assert!(
                template
                    .views
                    .iter()
                    .all(|kind| kind.starting_digit().is_some())
            );
            assert_eq!(template.area_names().len(), template.areas.len());
        }
    }

    #[test]
    fn the_standard_template_makes_what_every_project_made_before_templates() {
        assert_eq!(ProjectTemplate::ALL[0], STANDARD);
        assert_eq!(STANDARD.views, ViewKind::STARTING);
        assert_eq!(STANDARD.area_names(), [AreaName::general()]);
        let names = |workflow: &Workflow| -> Vec<String> {
            workflow
                .statuses()
                .iter()
                .map(|status| status.name().to_string())
                .collect()
        };
        assert_eq!(names(&STANDARD.workflow()), names(&Workflow::standard()));
        assert_eq!(
            names(&ENGINEERING.workflow()),
            names(&Workflow::engineering())
        );
    }

    #[test]
    fn kanban_limits_the_work_under_way_and_plans_no_sprint() {
        let workflow = KANBAN.workflow();
        let limited = workflow
            .statuses()
            .iter()
            .filter(|status| status.limit().is_some())
            .count();
        assert_eq!(limited, 3);
        const { assert!(!KANBAN.plans_first_sprint) };
        assert!(!KANBAN.views.contains(&ViewKind::Backlog));
    }

    #[test]
    fn scrum_puts_the_backlog_first_and_plans_a_sprint() {
        assert_eq!(SCRUM.views[0], ViewKind::Backlog);
        const { assert!(SCRUM.plans_first_sprint) };
    }

    #[test]
    fn a_hardware_product_has_an_area_for_each_part_of_the_device() {
        let names: Vec<String> = HARDWARE_PRODUCT
            .area_names()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(names, ["Electronics", "Firmware", "Enclosure"]);
    }

    #[test]
    fn a_template_is_found_by_its_key() {
        assert_eq!(ProjectTemplate::find("kanban"), Some(&KANBAN));
        assert_eq!(ProjectTemplate::find("waterfall"), None);
    }
}
