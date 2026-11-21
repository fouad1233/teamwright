//! Grouping the tasks of a view: by status, assignee, priority, area, tag or
//! sprint
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! A view keeps one [`Grouping`]. [`group_tasks`] is the rule that puts the
//! tasks of a project into groups, in a fixed order, so that a board draws
//! them as lanes and a list as sections without deciding anything itself.

use std::collections::BTreeSet;

use crate::area::Area;
use crate::ids::{AreaId, MemberId, SprintId, StatusId, TagId, TaskId};
use crate::macros::text_enum;
use crate::member::Member;
use crate::sprint::{Sprint, SprintState};
use crate::tag::Tag;
use crate::task::{Assignee, Priority, Task};
use crate::workflow::Workflow;

/// What a view groups its tasks by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Grouping {
    /// One group for each status of the workflow. This is how a board has
    /// always been drawn.
    #[default]
    Status,
    /// One group for the manager, one for each member of the team, and one
    /// for the tasks given to nobody.
    Assignee,
    /// One group for each priority, the highest first.
    Priority,
    /// One group for each area of the project.
    Area,
    /// One group for each tag, and one for the tasks without a tag. A task
    /// with several tags is in the group of each of them.
    Tag,
    /// One group for each sprint, and the backlog for the tasks in none.
    Sprint,
}

text_enum!(Grouping, kind = "grouping", {
    Status => "status",
    Assignee => "assignee",
    Priority => "priority",
    Area => "area",
    Tag => "tag",
    Sprint => "sprint",
});

/// What one group stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupKey {
    /// A status of the workflow.
    Status(StatusId),
    /// The tasks given to the manager.
    Manager,
    /// The tasks given to a member of the team.
    Member(MemberId),
    /// The tasks given to nobody.
    Nobody,
    /// A priority.
    Priority(Priority),
    /// An area.
    Area(AreaId),
    /// The tasks of an area that no longer exists. A task always has an
    /// area, so this group is there only when data was damaged.
    NoArea,
    /// A tag.
    Tag(TagId),
    /// The tasks that carry no tag.
    NoTag,
    /// A sprint.
    Sprint(SprintId),
    /// The tasks that are in no sprint.
    Backlog,
}

/// One group: what it stands for, and the tasks in it, in their order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGroup {
    /// What the group stands for.
    pub key: GroupKey,
    /// The tasks in it. Only tasks that are not subtasks: a subtask is shown
    /// with its parent.
    pub task_ids: Vec<TaskId>,
}

/// What the groups are made from besides the tasks.
#[derive(Debug, Clone, Copy)]
pub struct GroupingContext<'a> {
    /// The workflow of the project, for the order of the statuses.
    pub workflow: &'a Workflow,
    /// The members of the team, in the order of the team.
    pub members: &'a [Member],
    /// The areas of the project, in any order.
    pub areas: &'a [Area],
    /// The sprints of the project, in the order they are listed in: the
    /// active one, the planned ones, then the completed ones.
    pub sprints: &'a [Sprint],
    /// The tags of the project, in any order.
    pub tags: &'a [Tag],
}

/// Puts the tasks of a project into groups.
///
/// - Only tasks that are not subtasks are grouped. A subtask goes with its
///   parent, whatever the parent is grouped by.
/// - Within a group, tasks are in the order of their statuses in the
///   workflow, then by rank, then by number. Grouped by status that is the
///   order of a column of the board.
/// - The groups come in a fixed order. Statuses: as in the workflow.
///   Assignee: the manager, the members in the order of the team, nobody.
///   Priority: urgent, high, normal, low. Area: in the order of the areas.
///   Tag: by name, whatever the capitals, then the tasks without a tag.
///   Sprint: in the order the sprints are listed in, then the backlog.
/// - A group that nothing stands in is still returned when a card can be
///   dropped on it: every status, the manager, every member, nobody, every
///   priority, every area, every tag, "no tag", every sprint that is not
///   completed, and the backlog. A completed sprint, and the group of a
///   removed area, are returned only with tasks.
/// - A task of a member, tag or sprint that is not known (it was removed
///   elsewhere and this computer has not caught up) counts as given to
///   nobody, as without a tag, or as in the backlog.
#[must_use]
pub fn group_tasks(
    grouping: Grouping,
    context: &GroupingContext<'_>,
    tasks: &[Task],
) -> Vec<TaskGroup> {
    let mut roots: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.parent_id().is_none())
        .collect();
    let status_place = |task: &Task| {
        context
            .workflow
            .statuses()
            .iter()
            .position(|status| status.id() == task.status_id())
            .unwrap_or(usize::MAX)
    };
    roots.sort_by_key(|task| (status_place(task), task.rank(), task.number(), task.id()));

    let group = |key: GroupKey, belongs: &dyn Fn(&Task) -> bool| TaskGroup {
        key,
        task_ids: roots
            .iter()
            .filter(|task| belongs(task))
            .map(|task| task.id())
            .collect(),
    };

    match grouping {
        Grouping::Status => context
            .workflow
            .statuses()
            .iter()
            .map(|status| {
                let id = status.id();
                group(GroupKey::Status(id), &|task| task.status_id() == id)
            })
            .collect(),
        Grouping::Assignee => {
            let known: BTreeSet<MemberId> = context.members.iter().map(Member::id).collect();
            let mut groups = vec![group(GroupKey::Manager, &|task| {
                task.assignee() == Some(Assignee::Manager)
            })];
            groups.extend(context.members.iter().map(|member| {
                let id = member.id();
                group(GroupKey::Member(id), &|task| {
                    task.assignee() == Some(Assignee::Member(id))
                })
            }));
            groups.push(group(GroupKey::Nobody, &|task| match task.assignee() {
                None => true,
                Some(Assignee::Manager) => false,
                Some(Assignee::Member(id)) => !known.contains(&id),
            }));
            groups
        }
        Grouping::Priority => [
            Priority::Urgent,
            Priority::High,
            Priority::Normal,
            Priority::Low,
        ]
        .into_iter()
        .map(|priority| {
            group(GroupKey::Priority(priority), &|task| {
                task.priority() == priority
            })
        })
        .collect(),
        Grouping::Area => {
            let mut areas: Vec<&Area> = context.areas.iter().collect();
            areas.sort_by_key(|area| (area.position(), area.id()));
            let known: BTreeSet<AreaId> = areas.iter().map(|area| area.id()).collect();
            let mut groups: Vec<TaskGroup> = areas
                .iter()
                .map(|area| {
                    let id = area.id();
                    group(GroupKey::Area(id), &|task| task.area_id() == id)
                })
                .collect();
            let strays = group(GroupKey::NoArea, &|task| !known.contains(&task.area_id()));
            if !strays.task_ids.is_empty() {
                groups.push(strays);
            }
            groups
        }
        Grouping::Tag => {
            let mut tags: Vec<&Tag> = context.tags.iter().collect();
            tags.sort_by_key(|tag| (tag.name().as_str().to_lowercase(), tag.id()));
            let known: BTreeSet<TagId> = tags.iter().map(|tag| tag.id()).collect();
            let mut groups: Vec<TaskGroup> = tags
                .iter()
                .map(|tag| {
                    let id = tag.id();
                    group(GroupKey::Tag(id), &|task| task.tags().contains(&id))
                })
                .collect();
            groups.push(group(GroupKey::NoTag, &|task| {
                task.tags().iter().all(|id| !known.contains(id))
            }));
            groups
        }
        Grouping::Sprint => {
            let known: BTreeSet<SprintId> = context.sprints.iter().map(Sprint::id).collect();
            let mut groups: Vec<TaskGroup> = context
                .sprints
                .iter()
                .map(|sprint| {
                    let id = sprint.id();
                    let sprint_group =
                        group(GroupKey::Sprint(id), &|task| task.sprint_id() == Some(id));
                    (sprint.state(), sprint_group)
                })
                .filter(|(state, sprint_group)| {
                    *state != SprintState::Completed || !sprint_group.task_ids.is_empty()
                })
                .map(|(_, sprint_group)| sprint_group)
                .collect();
            groups.push(group(GroupKey::Backlog, &|task| {
                task.sprint_id().is_none_or(|id| !known.contains(&id))
            }));
            groups
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::color::Color;
    use crate::ids::ProjectId;
    use crate::member::MemberKind;
    use crate::sprint::{SprintDates, SprintSummary};
    use crate::task::{Rank, TaskDraft};
    use crate::text::{AreaName, MemberName, SprintName, TagName, TaskTitle};
    use crate::time::{CalendarDate, Timestamp};

    fn moment(hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 21, hour, 0, 0)
            .unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    /// A project with the standard workflow, and what grouping needs.
    struct Fixture {
        project: ProjectId,
        workflow: Workflow,
        members: Vec<Member>,
        areas: Vec<Area>,
        sprints: Vec<Sprint>,
        tags: Vec<Tag>,
        tasks: Vec<Task>,
    }

    impl Fixture {
        fn new() -> Self {
            let project = ProjectId::generate();
            Self {
                project,
                workflow: Workflow::standard(),
                members: vec![member(project, "Zoe", 9), member(project, "Adam", 10)],
                areas: vec![
                    Area::add(project, AreaName::parse("Firmware").unwrap(), 1),
                    Area::add(project, AreaName::parse("Hardware").unwrap(), 0),
                ],
                sprints: Vec::new(),
                tags: vec![
                    Tag::add(project, TagName::parse("emc").unwrap(), Color::Blue),
                    Tag::add(project, TagName::parse("Battery").unwrap(), Color::Green),
                ],
                tasks: Vec::new(),
            }
        }

        fn add_task(&mut self, title: &str, status_index: usize, area: usize) -> TaskId {
            self.add_task_under(title, status_index, area, None)
        }

        fn add_task_under(
            &mut self,
            title: &str,
            status_index: usize,
            area: usize,
            parent_id: Option<TaskId>,
        ) -> TaskId {
            let status = self.workflow.statuses()[status_index].clone();
            let number = u32::try_from(self.tasks.len()).unwrap() + 1;
            let mut draft = TaskDraft::titled(TaskTitle::parse(title).unwrap());
            draft.parent_id = parent_id;
            let task = Task::create(
                self.project,
                self.areas[area].id(),
                number,
                &status,
                Rank::from_value(f64::from(number)),
                draft,
                moment(9),
            );
            let id = task.id();
            self.tasks.push(task);
            id
        }

        fn task_mut(&mut self, id: TaskId) -> &mut Task {
            self.tasks.iter_mut().find(|task| task.id() == id).unwrap()
        }

        fn groups(&self, grouping: Grouping) -> Vec<TaskGroup> {
            group_tasks(
                grouping,
                &GroupingContext {
                    workflow: &self.workflow,
                    members: &self.members,
                    areas: &self.areas,
                    sprints: &self.sprints,
                    tags: &self.tags,
                },
                &self.tasks,
            )
        }

        fn planned_sprint(&self, number: u32) -> Sprint {
            Sprint::plan(
                self.project,
                SprintName::numbered(number),
                "",
                SprintDates::default(),
                moment(9),
            )
        }
    }

    fn member(project: ProjectId, name: &str, hour: u32) -> Member {
        Member::create(
            project,
            MemberName::parse(name).unwrap(),
            MemberKind::Agent,
            String::new(),
            Color::Teal,
            moment(hour),
        )
    }

    fn keys(groups: &[TaskGroup]) -> Vec<GroupKey> {
        groups.iter().map(|group| group.key).collect()
    }

    fn sizes(groups: &[TaskGroup]) -> Vec<usize> {
        groups.iter().map(|group| group.task_ids.len()).collect()
    }

    #[test]
    fn the_text_of_a_grouping_is_stable_and_read_back() {
        for grouping in Grouping::ALL {
            assert_eq!(grouping.as_str().parse::<Grouping>().unwrap(), *grouping);
        }
        assert_eq!(Grouping::default(), Grouping::Status);
        assert!("owner".parse::<Grouping>().is_err());
    }

    #[test]
    fn grouped_by_status_every_status_is_a_group_in_the_order_of_the_workflow() {
        let mut fixture = Fixture::new();
        let first = fixture.add_task("Later", 0, 0);
        let second = fixture.add_task("Sooner", 0, 0);
        let last_status = fixture.workflow.statuses().len() - 1;
        let done = fixture.add_task("Finished", last_status, 0);

        let groups = fixture.groups(Grouping::Status);

        let statuses: Vec<GroupKey> = fixture
            .workflow
            .statuses()
            .iter()
            .map(|status| GroupKey::Status(status.id()))
            .collect();
        assert_eq!(keys(&groups), statuses);
        assert_eq!(groups[0].task_ids, [first, second]);
        assert_eq!(groups[last_status].task_ids, [done]);
        assert_eq!(groups[1].task_ids, []);
    }

    #[test]
    fn a_subtask_is_not_grouped_on_its_own() {
        let mut fixture = Fixture::new();
        let parent = fixture.add_task("Parent", 0, 0);
        fixture.add_task_under("Child", 0, 0, Some(parent));

        let groups = fixture.groups(Grouping::Status);

        assert_eq!(groups[0].task_ids, [parent]);
    }

    #[test]
    fn grouped_by_assignee_the_manager_comes_first_the_team_in_its_order_and_nobody_last() {
        let mut fixture = Fixture::new();
        let zoe = fixture.members[0].id();
        let adam = fixture.members[1].id();
        let mine = fixture.add_task("Mine", 0, 0);
        let hers = fixture.add_task("Hers", 0, 0);
        let nobody = fixture.add_task("Nobody's", 0, 0);
        fixture
            .task_mut(mine)
            .assign(Some(Assignee::Manager), moment(10));
        fixture
            .task_mut(hers)
            .assign(Some(Assignee::Member(zoe)), moment(10));

        let groups = fixture.groups(Grouping::Assignee);

        assert_eq!(
            keys(&groups),
            [
                GroupKey::Manager,
                GroupKey::Member(zoe),
                GroupKey::Member(adam),
                GroupKey::Nobody
            ]
        );
        assert_eq!(sizes(&groups), [1, 1, 0, 1]);
        assert_eq!(groups[3].task_ids, [nobody]);
    }

    #[test]
    fn a_task_of_a_member_who_is_not_known_counts_as_given_to_nobody() {
        let mut fixture = Fixture::new();
        let task = fixture.add_task("Orphan", 0, 0);
        fixture
            .task_mut(task)
            .assign(Some(Assignee::Member(MemberId::generate())), moment(10));

        let groups = fixture.groups(Grouping::Assignee);

        assert_eq!(groups.last().unwrap().task_ids, [task]);
    }

    #[test]
    fn grouped_by_priority_the_highest_comes_first_and_every_priority_is_a_group() {
        let mut fixture = Fixture::new();
        let urgent = fixture.add_task("Fire", 0, 0);
        fixture.add_task("Usual", 0, 0);
        fixture
            .task_mut(urgent)
            .prioritize(Priority::Urgent, moment(10));

        let groups = fixture.groups(Grouping::Priority);

        assert_eq!(
            keys(&groups),
            [
                GroupKey::Priority(Priority::Urgent),
                GroupKey::Priority(Priority::High),
                GroupKey::Priority(Priority::Normal),
                GroupKey::Priority(Priority::Low),
            ]
        );
        assert_eq!(sizes(&groups), [1, 0, 1, 0]);
    }

    #[test]
    fn grouped_by_area_the_areas_come_in_their_own_order() {
        let mut fixture = Fixture::new();
        let in_firmware = fixture.add_task("Driver", 0, 0);
        let in_hardware = fixture.add_task("Board", 0, 1);
        let hardware = fixture.areas[1].id();
        let firmware = fixture.areas[0].id();

        let groups = fixture.groups(Grouping::Area);

        assert_eq!(
            keys(&groups),
            [GroupKey::Area(hardware), GroupKey::Area(firmware)]
        );
        assert_eq!(groups[0].task_ids, [in_hardware]);
        assert_eq!(groups[1].task_ids, [in_firmware]);
    }

    #[test]
    fn a_task_of_a_removed_area_is_shown_under_no_area_and_the_group_is_otherwise_absent() {
        let mut fixture = Fixture::new();
        let task = fixture.add_task("Lost", 0, 0);
        assert!(!keys(&fixture.groups(Grouping::Area)).contains(&GroupKey::NoArea));
        fixture.areas.remove(0);

        let groups = fixture.groups(Grouping::Area);

        assert_eq!(groups.last().unwrap().key, GroupKey::NoArea);
        assert_eq!(groups.last().unwrap().task_ids, [task]);
    }

    #[test]
    fn grouped_by_tag_the_tags_come_by_name_and_a_task_shows_under_each_of_its_tags() {
        let mut fixture = Fixture::new();
        let emc = fixture.tags[0].id();
        let battery = fixture.tags[1].id();
        let both = fixture.add_task("Both", 0, 0);
        let plain = fixture.add_task("Plain", 0, 0);
        fixture.task_mut(both).put_tag(emc, moment(10));
        fixture.task_mut(both).put_tag(battery, moment(10));

        let groups = fixture.groups(Grouping::Tag);

        // "Battery" before "emc", whatever the capitals.
        assert_eq!(
            keys(&groups),
            [GroupKey::Tag(battery), GroupKey::Tag(emc), GroupKey::NoTag]
        );
        assert_eq!(groups[0].task_ids, [both]);
        assert_eq!(groups[1].task_ids, [both]);
        assert_eq!(groups[2].task_ids, [plain]);
    }

    #[test]
    fn grouped_by_sprint_the_listed_order_holds_and_the_backlog_is_last() {
        let mut fixture = Fixture::new();
        let mut active = fixture.planned_sprint(1);
        active
            .start("Sprint 1", "", day(10), day(23), moment(10))
            .unwrap();
        let planned = fixture.planned_sprint(2);
        let (active_id, planned_id) = (active.id(), planned.id());
        fixture.sprints = vec![active, planned];
        let in_sprint = fixture.add_task("In the sprint", 0, 0);
        let loose = fixture.add_task("Loose", 0, 0);
        fixture
            .task_mut(in_sprint)
            .plan_into(Some(active_id), moment(10));

        let groups = fixture.groups(Grouping::Sprint);

        assert_eq!(
            keys(&groups),
            [
                GroupKey::Sprint(active_id),
                GroupKey::Sprint(planned_id),
                GroupKey::Backlog
            ]
        );
        assert_eq!(groups[0].task_ids, [in_sprint]);
        assert_eq!(groups[2].task_ids, [loose]);
    }

    #[test]
    fn a_completed_sprint_is_a_group_only_while_it_has_tasks() {
        let mut fixture = Fixture::new();
        let mut finished = fixture.planned_sprint(1);
        finished
            .start("Sprint 1", "", day(10), day(20), moment(10))
            .unwrap();
        finished
            .complete(SprintSummary::of(&[]), moment(11))
            .unwrap();
        let finished_id = finished.id();
        fixture.sprints = vec![finished];
        let task = fixture.add_task("Left behind", 0, 0);

        assert_eq!(keys(&fixture.groups(Grouping::Sprint)), [GroupKey::Backlog]);

        fixture
            .task_mut(task)
            .plan_into(Some(finished_id), moment(12));
        let groups = fixture.groups(Grouping::Sprint);
        assert_eq!(
            keys(&groups),
            [GroupKey::Sprint(finished_id), GroupKey::Backlog]
        );
        assert_eq!(groups[0].task_ids, [task]);
    }
}
