//! The views of a project: the ways the manager looks at its tasks
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! A view has a kind, a name and a filter, and a place among the views of
//! its project. The rules here concern one view, or one name. The rules
//! that look at the other views of the project, such as "one backlog in a
//! project" or "no two views with one name", are use cases.

use uuid::Uuid;

use crate::error::DomainError;
use crate::ids::{MemberId, ProjectId, ViewId};
use crate::macros::text_enum;
use crate::text::ViewName;

/// The ways the tasks of a project can be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewKind {
    /// The tasks in columns, one for each status.
    Board,
    /// The tasks that are in no sprint, and the sprints. A project has at
    /// most one.
    Backlog,
    /// The tasks in a table.
    List,
    /// The tasks on a time line.
    Gantt,
    /// The tasks on the days of a month.
    Calendar,
    /// Figures about the whole project. A project has at most one.
    Dashboard,
}

text_enum!(ViewKind, kind = "view kind", {
    Board => "board",
    Backlog => "backlog",
    List => "list",
    Gantt => "gantt",
    Calendar => "calendar",
    Dashboard => "dashboard",
});

impl ViewKind {
    /// The name a view of this kind gets when the manager gives it none.
    #[must_use]
    pub fn default_name(self) -> ViewName {
        ViewName::from_trusted(match self {
            Self::Board => "Board",
            Self::Backlog => "Backlog",
            Self::List => "List",
            Self::Gantt => "Gantt",
            Self::Calendar => "Calendar",
            Self::Dashboard => "Dashboard",
        })
    }

    /// Whether a project has at most one view of this kind: a backlog and a
    /// dashboard.
    #[must_use]
    pub const fn is_single(self) -> bool {
        matches!(self, Self::Backlog | Self::Dashboard)
    }

    /// Whether a view of this kind can be filtered. A backlog and a
    /// dashboard show every task.
    #[must_use]
    pub const fn can_be_filtered(self) -> bool {
        !self.is_single()
    }

    /// A digit of its own for each kind, used in the identifiers of the
    /// views a project starts with.
    const fn digit(self) -> u8 {
        match self {
            Self::Board => 1,
            Self::Backlog => 2,
            Self::List => 3,
            Self::Gantt => 4,
            Self::Calendar => 5,
            Self::Dashboard => 6,
        }
    }
}

/// Whose tasks a view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewAssignee {
    /// Everybody's, and the tasks given to nobody.
    #[default]
    Anyone,
    /// The tasks given to the manager.
    Manager,
    /// The tasks given to one member of the team.
    Member(MemberId),
    /// The tasks given to nobody.
    Nobody,
}

/// Which tasks a view shows. A task is shown if it passes the filter or one
/// of its subtasks does. Only a board, a list, a Gantt chart and a calendar
/// can be filtered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ViewFilter {
    /// Whose tasks are shown.
    pub assignee: ViewAssignee,
    /// Whether tasks in a status of the category done are left out.
    pub hides_finished: bool,
}

impl ViewFilter {
    /// The filter that shows every task.
    pub const EVERY_TASK: Self = Self {
        assignee: ViewAssignee::Anyone,
        hides_finished: false,
    };
}

/// The plain data of a view, used to recreate one that was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSnapshot {
    /// The identifier of the view.
    pub id: ViewId,
    /// The project the view belongs to.
    pub project_id: ProjectId,
    /// Its kind.
    pub kind: ViewKind,
    /// Its name.
    pub name: ViewName,
    /// Which tasks it shows.
    pub filter: ViewFilter,
    /// Its place among the views of its project. Views are listed by it,
    /// the smallest first.
    pub position: u32,
}

/// One way the manager looks at the tasks of a project: a kind with a name
/// and a filter of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    id: ViewId,
    project_id: ProjectId,
    kind: ViewKind,
    name: ViewName,
    filter: ViewFilter,
    position: u32,
}

impl View {
    /// The six views a new project starts with: one of each kind, in the
    /// order of [`ViewKind::ALL`], each named after its kind and showing
    /// every task.
    ///
    /// Their identifiers follow from the project and the kind (see
    /// [`View::starting_id`]), so two computers that each give a project its
    /// starting views give it the same ones.
    #[must_use]
    pub fn starting(project_id: ProjectId) -> Vec<Self> {
        (0_u32..)
            .zip(ViewKind::ALL)
            .map(|(position, kind)| Self {
                id: Self::starting_id(project_id, *kind),
                project_id,
                kind: *kind,
                name: kind.default_name(),
                filter: ViewFilter::EVERY_TASK,
                position,
            })
            .collect()
    }

    /// The identifier of the starting view of a kind in a project: the
    /// identifier of the project with its version digit replaced by a digit
    /// of the kind, from 1 to 6.
    ///
    /// Identifiers the program generates have the version digit 7, so a
    /// starting identifier never equals one of them, and two projects never
    /// share one.
    #[must_use]
    pub fn starting_id(project_id: ProjectId, kind: ViewKind) -> ViewId {
        let mut bytes = *project_id.as_uuid().as_bytes();
        bytes[6] = (bytes[6] & 0x0f) | (kind.digit() << 4);
        ViewId::from_uuid(Uuid::from_bytes(bytes))
    }

    /// Adds a view to a project at a place, showing every task.
    #[must_use]
    pub fn add(project_id: ProjectId, kind: ViewKind, name: ViewName, position: u32) -> Self {
        Self {
            id: ViewId::generate(),
            project_id,
            kind,
            name,
            filter: ViewFilter::EVERY_TASK,
            position,
        }
    }

    /// Recreates a view from stored data.
    #[must_use]
    pub fn restore(snapshot: ViewSnapshot) -> Self {
        Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            kind: snapshot.kind,
            name: snapshot.name,
            filter: snapshot.filter,
            position: snapshot.position,
        }
    }

    /// The identifier of the view.
    #[must_use]
    pub const fn id(&self) -> ViewId {
        self.id
    }

    /// The project the view belongs to.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// The kind of the view.
    #[must_use]
    pub const fn kind(&self) -> ViewKind {
        self.kind
    }

    /// The name of the view.
    #[must_use]
    pub const fn name(&self) -> &ViewName {
        &self.name
    }

    /// Which tasks the view shows.
    #[must_use]
    pub const fn filter(&self) -> ViewFilter {
        self.filter
    }

    /// The place of the view among the views of its project.
    #[must_use]
    pub const fn position(&self) -> u32 {
        self.position
    }

    /// Gives the view another name.
    pub fn rename(&mut self, name: ViewName) {
        self.name = name;
    }

    /// Replaces the filter of the view.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ViewNotFiltered`] for a backlog or a
    /// dashboard, which show every task. Nothing changes then.
    pub fn set_filter(&mut self, filter: ViewFilter) -> Result<(), DomainError> {
        if !self.kind.can_be_filtered() {
            return Err(DomainError::ViewNotFiltered {
                name: self.name.to_string(),
                kind: self.kind.as_str(),
            });
        }
        self.filter = filter;
        Ok(())
    }

    /// Puts the view at another place among the views of its project.
    pub fn move_to(&mut self, position: u32) {
        self.position = position;
    }

    /// Shows everybody's tasks again if the view showed only those of a
    /// member who left the team. Whether finished tasks are hidden stays as
    /// it was. Says whether anything changed.
    pub fn forget_member(&mut self, member_id: MemberId) -> bool {
        if self.filter.assignee == ViewAssignee::Member(member_id) {
            self.filter.assignee = ViewAssignee::Anyone;
            true
        } else {
            false
        }
    }
}

impl ViewName {
    /// The name a new view gets when the manager gives it none: the name of
    /// its kind, or that name with the first number from 2 on that none of
    /// the names taken has, whatever the capitals.
    #[must_use]
    pub fn for_new_view<'a>(kind: ViewKind, taken: impl IntoIterator<Item = &'a Self>) -> Self {
        let taken: Vec<String> = taken
            .into_iter()
            .map(|name| name.as_str().to_lowercase())
            .collect();
        let is_free = |candidate: &str| !taken.contains(&candidate.to_lowercase());
        let base = kind.default_name();
        if is_free(base.as_str()) {
            return base;
        }
        Self::numbered(base.as_str(), &is_free)
    }

    /// This name with the first number from 2 on for which `is_free` holds,
    /// such as "Board 2". The name is shortened if the number would not fit
    /// otherwise.
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
                return Self::parse(&candidate).unwrap_or_else(|_| Self::from_trusted("View"));
            }
            number = number.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_of_the_kinds_follow_the_rules() {
        for kind in ViewKind::ALL {
            let name = kind.default_name();
            assert_eq!(ViewName::parse(name.as_str()).unwrap(), name);
        }
    }

    #[test]
    fn a_kind_is_read_back_from_its_text() {
        for kind in ViewKind::ALL {
            assert_eq!(kind.as_str().parse::<ViewKind>().unwrap(), *kind);
        }
        assert!("timeline".parse::<ViewKind>().is_err());
    }

    #[test]
    fn a_new_project_starts_with_one_view_of_each_kind_in_order() {
        let project = ProjectId::generate();

        let views = View::starting(project);

        let names: Vec<&str> = views.iter().map(|view| view.name().as_str()).collect();
        assert_eq!(
            names,
            ["Board", "Backlog", "List", "Gantt", "Calendar", "Dashboard"]
        );
        let positions: Vec<u32> = views.iter().map(View::position).collect();
        assert_eq!(positions, [0, 1, 2, 3, 4, 5]);
        assert!(
            views
                .iter()
                .all(|view| view.filter() == ViewFilter::EVERY_TASK)
        );
    }

    #[test]
    fn the_starting_views_of_a_project_have_the_same_identifiers_every_time() {
        let project = ProjectId::generate();
        let other = ProjectId::generate();

        let ids: Vec<ViewId> = View::starting(project).iter().map(View::id).collect();
        let again: Vec<ViewId> = View::starting(project).iter().map(View::id).collect();
        let others: Vec<ViewId> = View::starting(other).iter().map(View::id).collect();

        assert_eq!(ids, again);
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6);
        assert!(ids.iter().all(|id| !others.contains(id)));
        assert!(ids.iter().all(|id| *id != ViewId::generate()));
    }

    #[test]
    fn a_starting_identifier_is_the_project_with_the_digit_of_the_kind() {
        let project: ProjectId = "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10".parse().unwrap();

        let id = View::starting_id(project, ViewKind::Gantt);

        assert_eq!(id.to_string(), "0196f3a2-7c1e-4d53-9b6f-2f1a4c8e9d10");
    }

    #[test]
    fn a_name_is_numbered_with_the_first_free_number_whatever_the_capitals() {
        let taken = [
            ViewName::parse("Gantt").unwrap(),
            ViewName::parse("gantt 2").unwrap(),
            ViewName::parse("Gantt 4").unwrap(),
        ];

        let name = ViewName::for_new_view(ViewKind::Gantt, &taken);

        assert_eq!(name.as_str(), "Gantt 3");
    }

    #[test]
    fn a_long_name_is_shortened_to_make_room_for_its_number() {
        let long = "x".repeat(ViewName::MAX_LENGTH);

        let name = ViewName::numbered(&long, &|_| true);

        assert_eq!(name.as_str().chars().count(), ViewName::MAX_LENGTH);
        assert!(name.as_str().ends_with(" 2"));
    }

    #[test]
    fn a_backlog_and_a_dashboard_cannot_be_filtered() {
        let project = ProjectId::generate();
        let mut views = View::starting(project);
        let filter = ViewFilter {
            assignee: ViewAssignee::Manager,
            hides_finished: true,
        };

        for view in &mut views {
            let result = view.set_filter(filter);
            if view.kind().can_be_filtered() {
                assert!(result.is_ok());
                assert_eq!(view.filter(), filter);
            } else {
                assert_eq!(result.unwrap_err().code(), "rule_broken");
                assert_eq!(view.filter(), ViewFilter::EVERY_TASK);
            }
        }
    }

    #[test]
    fn a_view_forgets_a_member_who_left_and_keeps_hiding_finished_tasks() {
        let project = ProjectId::generate();
        let member = MemberId::generate();
        let mut board = View::starting(project).remove(0);
        board
            .set_filter(ViewFilter {
                assignee: ViewAssignee::Member(member),
                hides_finished: true,
            })
            .unwrap();

        assert!(!board.forget_member(MemberId::generate()));
        assert!(board.forget_member(member));

        assert_eq!(
            board.filter(),
            ViewFilter {
                assignee: ViewAssignee::Anyone,
                hides_finished: true,
            }
        );
    }
}
