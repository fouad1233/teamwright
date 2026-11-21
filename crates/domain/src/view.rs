//! The views of a project: the ways the manager looks at its tasks
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! A view has a kind, a name and a filter, and a place among the views of
//! its project. The rules here concern one view, or one name. The rules
//! that look at the other views of the project, such as "one backlog in a
//! project" or "no two views with one name", are use cases.

use uuid::Uuid;

use crate::error::DomainError;
use crate::ids::{MemberId, ProjectId, TagId, ViewId};
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
    /// The tasks as bars on a line of time, in groups.
    Timeline,
    /// The tasks in a table with a column for every field.
    Table,
    /// How much work each member of the team has, day by day.
    Workload,
    /// The tasks by the people they are given to. Such a view is named
    /// "People": "Team" is the place of a project where its members are kept.
    Team,
    /// Everything that happened in the project, newest first.
    Activity,
    /// The tasks as a tree of ideas.
    MindMap,
    /// The tasks as pins on a map.
    Map,
    /// Notes and arrows on an open surface.
    Whiteboard,
    /// A conversation about the project.
    Chat,
    /// A page of text.
    Doc,
    /// Another page shown inside the project.
    Embed,
    /// Questions whose answers become tasks.
    Form,
    /// The tasks as pictures.
    Gallery,
    /// Figures and charts the manager chooses.
    Report,
    /// What lies in the folders of the project on this computer.
    Files,
}

text_enum!(ViewKind, kind = "view kind", {
    Board => "board",
    Backlog => "backlog",
    List => "list",
    Gantt => "gantt",
    Calendar => "calendar",
    Dashboard => "dashboard",
    Timeline => "timeline",
    Table => "table",
    Workload => "workload",
    Team => "team",
    Activity => "activity",
    MindMap => "mind_map",
    Map => "map",
    Whiteboard => "whiteboard",
    Chat => "chat",
    Doc => "doc",
    Embed => "embed",
    Form => "form",
    Gallery => "gallery",
    Report => "report",
    Files => "files",
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
            Self::Timeline => "Timeline",
            Self::Table => "Table",
            Self::Workload => "Workload",
            Self::Team => "People",
            Self::Activity => "Activity",
            Self::MindMap => "Mind map",
            Self::Map => "Map",
            Self::Whiteboard => "Whiteboard",
            Self::Chat => "Chat",
            Self::Doc => "Doc",
            Self::Embed => "Embed",
            Self::Form => "Form",
            Self::Gallery => "Gallery",
            Self::Report => "Report",
            Self::Files => "Files",
        })
    }

    /// The kinds a new project gets one view of, in their order.
    pub const STARTING: [Self; 6] = [
        Self::Board,
        Self::Backlog,
        Self::List,
        Self::Gantt,
        Self::Calendar,
        Self::Dashboard,
    ];

    /// Whether a project has at most one view of this kind: a backlog and a
    /// dashboard.
    #[must_use]
    pub const fn is_single(self) -> bool {
        matches!(self, Self::Backlog | Self::Dashboard)
    }

    /// Whether a view of this kind can be filtered: one that shows some of
    /// the tasks of the project. A backlog and a dashboard show every task;
    /// an activity, a whiteboard, a chat, a document, an embedded page, a
    /// form, a report and the files of the project are not a set of tasks
    /// at all.
    #[must_use]
    pub const fn can_be_filtered(self) -> bool {
        match self {
            Self::Board
            | Self::List
            | Self::Gantt
            | Self::Calendar
            | Self::Timeline
            | Self::Table
            | Self::Workload
            | Self::Team
            | Self::MindMap
            | Self::Map
            | Self::Gallery => true,
            Self::Backlog
            | Self::Dashboard
            | Self::Activity
            | Self::Whiteboard
            | Self::Chat
            | Self::Doc
            | Self::Embed
            | Self::Form
            | Self::Report
            | Self::Files => false,
        }
    }

    /// A digit of its own for each kind a project starts with, used in the
    /// identifiers of its starting views. Nothing for the other kinds.
    const fn starting_digit(self) -> Option<u8> {
        match self {
            Self::Board => Some(1),
            Self::Backlog => Some(2),
            Self::List => Some(3),
            Self::Gantt => Some(4),
            Self::Calendar => Some(5),
            Self::Dashboard => Some(6),
            _ => None,
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
/// of its subtasks does. Only the kinds that show some of the tasks can be
/// filtered (see [`ViewKind::can_be_filtered`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ViewFilter {
    /// Whose tasks are shown.
    pub assignee: ViewAssignee,
    /// Whether tasks in a status of the category done are left out.
    pub hides_finished: bool,
    /// The tag the tasks carry, if only the tasks with one tag are shown.
    pub tag: Option<TagId>,
}

impl ViewFilter {
    /// The filter that shows every task.
    pub const EVERY_TASK: Self = Self {
        assignee: ViewAssignee::Anyone,
        hides_finished: false,
        tag: None,
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
    /// What it keeps of its own, if anything (see [`View::content`]).
    pub content: Option<String>,
}

/// One way the manager looks at the tasks of a project: a kind with a name
/// and a filter of its own, and content of its own for a kind that is a
/// page rather than a look at the tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    id: ViewId,
    project_id: ProjectId,
    kind: ViewKind,
    name: ViewName,
    filter: ViewFilter,
    position: u32,
    content: Option<String>,
}

impl View {
    /// The six views a new project starts with: one of each kind in
    /// [`ViewKind::STARTING`], in that order, each named after its kind and
    /// showing every task.
    ///
    /// Their identifiers follow from the project and the kind (see
    /// [`View::starting_id`]), so two computers that each give a project its
    /// starting views give it the same ones.
    #[must_use]
    pub fn starting(project_id: ProjectId) -> Vec<Self> {
        (0_u32..)
            .zip(ViewKind::STARTING)
            .filter_map(|(position, kind)| {
                Some(Self {
                    id: Self::starting_id(project_id, kind)?,
                    project_id,
                    kind,
                    name: kind.default_name(),
                    filter: ViewFilter::EVERY_TASK,
                    position,
                    content: None,
                })
            })
            .collect()
    }

    /// The most bytes the content of a view may have: 2 MiB.
    pub const MAX_CONTENT_BYTES: usize = 2 * 1024 * 1024;

    /// The identifier of the starting view of a kind in a project: the
    /// identifier of the project with its version digit replaced by a digit
    /// of the kind, from 1 to 6. Nothing for a kind a project does not
    /// start with.
    ///
    /// Identifiers the program generates have the version digit 7, so a
    /// starting identifier never equals one of them, and two projects never
    /// share one.
    #[must_use]
    pub fn starting_id(project_id: ProjectId, kind: ViewKind) -> Option<ViewId> {
        kind.starting_digit()
            .map(|digit| Self::id_with_digit(project_id, digit))
    }

    /// The identifier of the project with its version digit replaced.
    fn id_with_digit(project_id: ProjectId, digit: u8) -> ViewId {
        let mut bytes = *project_id.as_uuid().as_bytes();
        bytes[6] = (bytes[6] & 0x0f) | (digit << 4);
        ViewId::from_uuid(Uuid::from_bytes(bytes))
    }

    /// The highest attempt [`View::board_again`] knows an identifier for.
    pub const LAST_BOARD_ATTEMPT: u8 = 8;

    /// The board a project gets again when it is left with no views: two
    /// computers that each removed other views can leave it so. Every
    /// computer gives it the same board, with an identifier that follows
    /// from the project: the identifier of its starting board at the first
    /// attempt, and, because a deletion is final, one with the version
    /// digit 8, 9, ... up to 15 at the later attempts, once the identifier
    /// before was deleted too. Nothing beyond the last attempt.
    #[must_use]
    pub fn board_again(project_id: ProjectId, attempt: u8) -> Option<Self> {
        let id = match attempt {
            // The identifier of the starting board.
            0 => Self::id_with_digit(project_id, 1),
            1..=Self::LAST_BOARD_ATTEMPT => Self::id_with_digit(project_id, 7 + attempt),
            _ => return None,
        };
        Some(Self {
            id,
            project_id,
            kind: ViewKind::Board,
            name: ViewKind::Board.default_name(),
            filter: ViewFilter::EVERY_TASK,
            position: 0,
            content: None,
        })
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
            content: None,
        }
    }

    /// Recreates a view from stored data, checking its content against the
    /// rule of [`View::set_content`].
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ViewContentTooLong`] if the content is longer
    /// than a view may keep.
    pub fn restore(snapshot: ViewSnapshot) -> Result<Self, DomainError> {
        check_content(snapshot.content.as_deref())?;
        Ok(Self {
            id: snapshot.id,
            project_id: snapshot.project_id,
            kind: snapshot.kind,
            name: snapshot.name,
            filter: snapshot.filter,
            position: snapshot.position,
            content: snapshot.content,
        })
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

    /// What the view keeps of its own: the text of a document, the notes of
    /// a whiteboard, the fields of a form, and so on. The interface of each
    /// kind writes it and reads it back; the domain does not read it.
    /// Nothing for a view that never had any.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Replaces the content of the view, or clears it with nothing.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ViewContentTooLong`] if the content has more
    /// than [`View::MAX_CONTENT_BYTES`] bytes. Nothing changes then.
    pub fn set_content(&mut self, content: Option<String>) -> Result<(), DomainError> {
        check_content(content.as_deref())?;
        self.content = content;
        Ok(())
    }

    /// Gives the view another name.
    pub fn rename(&mut self, name: ViewName) {
        self.name = name;
    }

    /// Replaces the filter of the view.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::ViewNotFiltered`] for a kind that cannot be
    /// filtered, such as a backlog or a document. Nothing changes then.
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

    /// Shows the tasks with any tag again if the view showed only those with
    /// a tag that was removed. The rest of the filter stays as it was. Says
    /// whether anything changed.
    pub fn forget_tag(&mut self, tag_id: TagId) -> bool {
        if self.filter.tag == Some(tag_id) {
            self.filter.tag = None;
            true
        } else {
            false
        }
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

/// Refuses content longer than a view may keep.
fn check_content(content: Option<&str>) -> Result<(), DomainError> {
    let actual_bytes = content.map_or(0, str::len);
    if actual_bytes > View::MAX_CONTENT_BYTES {
        return Err(DomainError::ViewContentTooLong {
            max_bytes: View::MAX_CONTENT_BYTES,
            actual_bytes,
        });
    }
    Ok(())
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
        assert_eq!("mind_map".parse::<ViewKind>().unwrap(), ViewKind::MindMap);
        assert!("mindmap".parse::<ViewKind>().is_err());
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
    fn a_board_given_again_has_an_identifier_of_its_own_at_each_attempt() {
        let project = ProjectId::generate();

        let ids: Vec<ViewId> = (0..=View::LAST_BOARD_ATTEMPT)
            .filter_map(|attempt| View::board_again(project, attempt))
            .map(|board| board.id())
            .collect();

        assert_eq!(ids.len(), 9);
        assert_eq!(Some(ids[0]), View::starting_id(project, ViewKind::Board));
        let mut distinct = ids.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len());
        assert!(
            ids.iter()
                .all(|id| *id != ViewId::from_uuid(project.as_uuid()))
        );
        assert_eq!(
            View::board_again(project, View::LAST_BOARD_ATTEMPT + 1),
            None
        );
        assert_eq!(View::board_again(project, 3), View::board_again(project, 3));
    }

    #[test]
    fn a_starting_identifier_is_the_project_with_the_digit_of_the_kind() {
        let project: ProjectId = "0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10".parse().unwrap();

        let id = View::starting_id(project, ViewKind::Gantt);

        assert_eq!(
            id.map(|id| id.to_string()).as_deref(),
            Some("0196f3a2-7c1e-4d53-9b6f-2f1a4c8e9d10")
        );
        assert_eq!(View::starting_id(project, ViewKind::Timeline), None);
    }

    #[test]
    fn a_new_project_starts_with_none_of_the_newer_kinds() {
        let kinds: Vec<ViewKind> = View::starting(ProjectId::generate())
            .iter()
            .map(View::kind)
            .collect();

        assert_eq!(kinds, ViewKind::STARTING);
    }

    #[test]
    fn only_a_backlog_and_a_dashboard_are_single() {
        let single: Vec<ViewKind> = ViewKind::ALL
            .iter()
            .copied()
            .filter(|kind| kind.is_single())
            .collect();

        assert_eq!(single, [ViewKind::Backlog, ViewKind::Dashboard]);
    }

    #[test]
    fn the_kinds_that_show_some_of_the_tasks_can_be_filtered() {
        let filtered: Vec<&str> = ViewKind::ALL
            .iter()
            .filter(|kind| kind.can_be_filtered())
            .map(ViewKind::as_str)
            .collect();

        assert_eq!(
            filtered,
            [
                "board", "list", "gantt", "calendar", "timeline", "table", "workload", "team",
                "mind_map", "map", "gallery"
            ]
        );
    }

    #[test]
    fn a_document_cannot_be_filtered() {
        let mut doc = View::add(
            ProjectId::generate(),
            ViewKind::Doc,
            ViewKind::Doc.default_name(),
            6,
        );

        let result = doc.set_filter(ViewFilter {
            assignee: ViewAssignee::Nobody,
            hides_finished: false,
            tag: None,
        });

        assert_eq!(result.unwrap_err().code(), "rule_broken");
        assert_eq!(doc.filter(), ViewFilter::EVERY_TASK);
    }

    #[test]
    fn a_view_starts_without_content_and_keeps_what_it_is_given() {
        let mut doc = View::add(
            ProjectId::generate(),
            ViewKind::Doc,
            ViewKind::Doc.default_name(),
            6,
        );
        assert_eq!(doc.content(), None);

        doc.set_content(Some("{\"text\":\"Notes\"}".to_owned()))
            .unwrap();
        assert_eq!(doc.content(), Some("{\"text\":\"Notes\"}"));

        doc.set_content(None).unwrap();
        assert_eq!(doc.content(), None);
    }

    #[test]
    fn the_content_of_a_view_has_at_most_two_mebibytes() {
        let mut doc = View::add(
            ProjectId::generate(),
            ViewKind::Doc,
            ViewKind::Doc.default_name(),
            6,
        );

        doc.set_content(Some("x".repeat(View::MAX_CONTENT_BYTES)))
            .unwrap();
        let error = doc
            .set_content(Some("y".repeat(View::MAX_CONTENT_BYTES + 1)))
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the content of a view can have at most 2097152 bytes, but this one has 2097153"
        );
        assert_eq!(doc.content().map(str::len), Some(View::MAX_CONTENT_BYTES));
    }

    #[test]
    fn a_view_with_too_much_content_is_not_restored() {
        let doc = View::add(
            ProjectId::generate(),
            ViewKind::Doc,
            ViewKind::Doc.default_name(),
            6,
        );
        let snapshot = ViewSnapshot {
            id: doc.id(),
            project_id: doc.project_id(),
            kind: doc.kind(),
            name: doc.name().clone(),
            filter: doc.filter(),
            position: doc.position(),
            content: Some("z".repeat(View::MAX_CONTENT_BYTES + 1)),
        };

        assert!(View::restore(snapshot).is_err());
    }

    #[test]
    fn a_mind_map_is_named_with_a_space_and_written_with_a_line() {
        assert_eq!(ViewKind::MindMap.default_name().as_str(), "Mind map");
        assert_eq!(ViewKind::MindMap.as_str(), "mind_map");
    }

    #[test]
    fn a_view_of_the_people_is_not_named_like_the_place_of_the_team() {
        assert_eq!(ViewKind::Team.default_name().as_str(), "People");
        assert_eq!(ViewKind::Team.as_str(), "team");
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
            tag: Some(TagId::generate()),
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
                tag: None,
            })
            .unwrap();

        assert!(!board.forget_member(MemberId::generate()));
        assert!(board.forget_member(member));

        assert_eq!(
            board.filter(),
            ViewFilter {
                assignee: ViewAssignee::Anyone,
                hides_finished: true,
                tag: None,
            }
        );
    }

    #[test]
    fn a_view_forgets_a_tag_that_was_removed_and_keeps_the_rest_of_its_filter() {
        let mut board = View::starting(ProjectId::generate()).remove(0);
        let tag = TagId::generate();
        let filter = ViewFilter {
            assignee: ViewAssignee::Manager,
            hides_finished: true,
            tag: Some(tag),
        };
        board.set_filter(filter).unwrap();

        assert!(!board.forget_tag(TagId::generate()));
        assert!(board.forget_tag(tag));

        assert_eq!(
            board.filter(),
            ViewFilter {
                tag: None,
                ..filter
            }
        );
    }
}
