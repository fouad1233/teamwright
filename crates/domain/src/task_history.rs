//! The history of a task: what changed from one version of it to the next.
//!
//! Nothing is recorded for the history as such. Storage keeps every version
//! a task has had, in its log of operations, and the history is read off
//! those versions by comparing each one with the one before it.

use crate::field::FieldValue;
use crate::ids::{FieldId, SprintId, StatusId, TagId, TaskId};
use crate::sprint::StoryPoints;
use crate::task::{Assignee, Priority, Schedule, Task};
use crate::text::{ChecklistText, TaskTitle};
use crate::time::Timestamp;

/// One change in the life of a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskChange {
    /// When it happened.
    pub at: Timestamp,
    /// What happened.
    pub kind: TaskChangeKind,
}

/// What happened to a task in one change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskChangeKind {
    /// The task was opened.
    Created,
    /// It got another title.
    Renamed {
        /// The title before.
        from: TaskTitle,
        /// The title since.
        to: TaskTitle,
    },
    /// Its description changed. A description is long, so the history says
    /// only that it changed.
    Described,
    /// It went to another status. A status may have been removed since.
    Moved {
        /// The status before.
        from: StatusId,
        /// The status since.
        to: StatusId,
    },
    /// It was made a part of another task, or a task of its own. The task
    /// it was or is a part of may have been removed since.
    MadePartOf {
        /// The task it was a part of before, or nothing.
        from: Option<TaskId>,
        /// The task it is a part of since, or nothing.
        to: Option<TaskId>,
    },
    /// It was given to someone else, or to nobody.
    Assigned {
        /// Who was responsible before.
        from: Option<Assignee>,
        /// Who is responsible since.
        to: Option<Assignee>,
    },
    /// Its priority changed.
    Prioritized {
        /// The priority before.
        from: Priority,
        /// The priority since.
        to: Priority,
    },
    /// Its estimate changed. The numbers are minutes.
    Estimated {
        /// The estimate before.
        from: Option<u32>,
        /// The estimate since.
        to: Option<u32>,
    },
    /// It was planned into another sprint, or back into the backlog. A
    /// sprint may have been deleted since.
    Planned {
        /// The sprint it was in before, or nothing for the backlog.
        from: Option<SprintId>,
        /// The sprint it is in since, or nothing for the backlog.
        to: Option<SprintId>,
    },
    /// Its story points changed, or it got or lost them.
    Pointed {
        /// The points before.
        from: Option<StoryPoints>,
        /// The points since.
        to: Option<StoryPoints>,
    },
    /// Its dates changed.
    Rescheduled {
        /// The dates before.
        from: Schedule,
        /// The dates since.
        to: Schedule,
    },
    /// It began to wait for another task, which may have been removed
    /// since.
    NowWaitsFor {
        /// The task it waits for.
        blocker: TaskId,
    },
    /// It stopped waiting for another task.
    NoLongerWaitsFor {
        /// The task it waited for.
        blocker: TaskId,
    },
    /// A tag was put on it. The tag may have been removed since.
    Tagged {
        /// The tag.
        tag: TagId,
    },
    /// A tag was taken off it, by hand or because the tag was removed.
    Untagged {
        /// The tag.
        tag: TagId,
    },
    /// An item of its checklist was ticked: it is done.
    Ticked {
        /// The text of the item when it was ticked.
        item: ChecklistText,
    },
    /// An item of its checklist that was done is not done any more.
    Unticked {
        /// The text of the item when it was unticked.
        item: ChecklistText,
    },
    /// The value of a custom field was set, changed or cleared: by hand, or
    /// because the field or the choice it held was removed. The field may
    /// have been removed since.
    FieldValueChanged {
        /// The field.
        field: FieldId,
        /// The value before, if it held one.
        from: Option<FieldValue>,
        /// The value since, if it holds one.
        to: Option<FieldValue>,
    },
}

/// Tells what changed between two versions of a task, in the order the
/// history lists changes that happened at the same moment.
///
/// A change of the rank alone, which only moves a task up or down within
/// its status, is not part of the history.
#[must_use]
pub fn changes_between(before: &Task, after: &Task) -> Vec<TaskChange> {
    let mut kinds = Vec::new();

    if before.title() != after.title() {
        kinds.push(TaskChangeKind::Renamed {
            from: before.title().clone(),
            to: after.title().clone(),
        });
    }
    if before.description() != after.description() {
        kinds.push(TaskChangeKind::Described);
    }
    if before.status_id() != after.status_id() {
        kinds.push(TaskChangeKind::Moved {
            from: before.status_id(),
            to: after.status_id(),
        });
    }
    if before.parent_id() != after.parent_id() {
        kinds.push(TaskChangeKind::MadePartOf {
            from: before.parent_id(),
            to: after.parent_id(),
        });
    }
    if before.assignee() != after.assignee() {
        kinds.push(TaskChangeKind::Assigned {
            from: before.assignee(),
            to: after.assignee(),
        });
    }
    if before.priority() != after.priority() {
        kinds.push(TaskChangeKind::Prioritized {
            from: before.priority(),
            to: after.priority(),
        });
    }
    if before.estimate_minutes() != after.estimate_minutes() {
        kinds.push(TaskChangeKind::Estimated {
            from: before.estimate_minutes(),
            to: after.estimate_minutes(),
        });
    }
    if before.sprint_id() != after.sprint_id() {
        kinds.push(TaskChangeKind::Planned {
            from: before.sprint_id(),
            to: after.sprint_id(),
        });
    }
    if before.points() != after.points() {
        kinds.push(TaskChangeKind::Pointed {
            from: before.points(),
            to: after.points(),
        });
    }
    if before.schedule() != after.schedule() {
        kinds.push(TaskChangeKind::Rescheduled {
            from: before.schedule(),
            to: after.schedule(),
        });
    }
    kinds.extend(
        after
            .blocked_by()
            .difference(before.blocked_by())
            .map(|&blocker| TaskChangeKind::NowWaitsFor { blocker }),
    );
    kinds.extend(
        before
            .blocked_by()
            .difference(after.blocked_by())
            .map(|&blocker| TaskChangeKind::NoLongerWaitsFor { blocker }),
    );
    kinds.extend(
        after
            .tags()
            .difference(before.tags())
            .map(|&tag| TaskChangeKind::Tagged { tag }),
    );
    kinds.extend(
        before
            .tags()
            .difference(after.tags())
            .map(|&tag| TaskChangeKind::Untagged { tag }),
    );
    // Adding, renaming, moving and removing an item are not part of the
    // history; ticking it is, because that is work done.
    for item in after.checklist().items() {
        let Some(earlier) = before.checklist().item(item.id()) else {
            continue;
        };
        match (earlier.is_done(), item.is_done()) {
            (false, true) => kinds.push(TaskChangeKind::Ticked {
                item: item.text().clone(),
            }),
            (true, false) => kinds.push(TaskChangeKind::Unticked {
                item: item.text().clone(),
            }),
            _ => {}
        }
    }
    // The values of the fields, in the order of the identifiers of the
    // fields: a value set, changed or cleared.
    let fields: std::collections::BTreeSet<FieldId> = before
        .field_values()
        .keys()
        .chain(after.field_values().keys())
        .copied()
        .collect();
    for field in fields {
        let from = before.field_values().get(&field);
        let to = after.field_values().get(&field);
        if from != to {
            kinds.push(TaskChangeKind::FieldValueChanged {
                field,
                from: from.cloned(),
                to: to.cloned(),
            });
        }
    }

    let at = after.updated_at();
    kinds
        .into_iter()
        .map(|kind| TaskChange { at, kind })
        .collect()
}

/// The history of a task from the versions it has had, the oldest first:
/// that it was opened, and then every change from one version to the next.
/// A task without versions has no history.
#[must_use]
pub fn history_of(versions: &[Task]) -> Vec<TaskChange> {
    let Some(first) = versions.first() else {
        return Vec::new();
    };
    let mut history = vec![TaskChange {
        at: first.created_at(),
        kind: TaskChangeKind::Created,
    }];
    for (before, after) in versions.iter().zip(versions.iter().skip(1)) {
        history.extend(changes_between(before, after));
    }
    history
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::ids::{AreaId, ProjectId};
    use crate::task::{Rank, TaskDraft};
    use crate::time::CalendarDate;
    use crate::workflow::{StatusCategory, Workflow};

    fn moment(hour: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 11, hour, 0, 0)
            .unwrap()
    }

    fn day(day_of_month: u32) -> CalendarDate {
        CalendarDate::from_ymd_opt(2026, 11, day_of_month).unwrap()
    }

    fn title(text: &str) -> TaskTitle {
        TaskTitle::parse(text).unwrap()
    }

    fn task_in(workflow: &Workflow) -> Task {
        Task::create(
            ProjectId::generate(),
            AreaId::generate(),
            1,
            workflow.initial_status(),
            Rank::FIRST,
            TaskDraft::titled(title("Draw the schematic")),
            moment(9),
        )
    }

    fn kinds(changes: &[TaskChange]) -> Vec<&TaskChangeKind> {
        changes.iter().map(|change| &change.kind).collect()
    }

    #[test]
    fn the_history_begins_with_the_task_being_opened() {
        let task = task_in(&Workflow::standard());

        let history = history_of(std::slice::from_ref(&task));

        assert_eq!(
            history,
            [TaskChange {
                at: moment(9),
                kind: TaskChangeKind::Created
            }]
        );
    }

    #[test]
    fn a_task_without_versions_has_no_history() {
        assert_eq!(history_of(&[]), []);
    }

    #[test]
    fn every_change_is_listed_with_the_moment_it_was_made_the_oldest_first() {
        let workflow = Workflow::standard();
        let opened = task_in(&workflow);
        let mut renamed = opened.clone();
        renamed.rename(title("Draw the whole schematic"), moment(10));
        let mut prioritized = renamed.clone();
        prioritized.prioritize(Priority::Urgent, moment(11));

        let history = history_of(&[opened, renamed, prioritized]);

        assert_eq!(
            history,
            [
                TaskChange {
                    at: moment(9),
                    kind: TaskChangeKind::Created
                },
                TaskChange {
                    at: moment(10),
                    kind: TaskChangeKind::Renamed {
                        from: title("Draw the schematic"),
                        to: title("Draw the whole schematic"),
                    }
                },
                TaskChange {
                    at: moment(11),
                    kind: TaskChangeKind::Prioritized {
                        from: Priority::Normal,
                        to: Priority::Urgent,
                    }
                },
            ]
        );
    }

    #[test]
    fn every_kind_of_change_is_told() {
        let workflow = Workflow::standard();
        let before = task_in(&workflow);
        let active = workflow
            .first_in(StatusCategory::Active)
            .expect("the standard workflow has a status for work under way");
        let blocker = crate::ids::TaskId::generate();
        let schedule = Schedule::new(Some(day(12)), Some(day(14))).unwrap();

        let change = |change: &dyn Fn(&mut Task)| {
            let mut after = before.clone();
            change(&mut after);
            changes_between(&before, &after)
        };

        assert_eq!(
            kinds(&change(
                &|task| task.describe("Two sheets.".to_owned(), moment(10))
            )),
            [&TaskChangeKind::Described]
        );
        let whole = crate::ids::TaskId::generate();
        assert_eq!(
            kinds(&change(&|task| task
                .set_parent(Some(whole), moment(10))
                .unwrap())),
            [&TaskChangeKind::MadePartOf {
                from: None,
                to: Some(whole)
            }]
        );
        assert_eq!(
            kinds(&change(&|task| task.move_to(
                active,
                Rank::FIRST,
                moment(10)
            ))),
            [&TaskChangeKind::Moved {
                from: before.status_id(),
                to: active.id()
            }]
        );
        assert_eq!(
            kinds(&change(
                &|task| task.assign(Some(Assignee::Manager), moment(10))
            )),
            [&TaskChangeKind::Assigned {
                from: None,
                to: Some(Assignee::Manager)
            }]
        );
        assert_eq!(
            kinds(&change(&|task| task.estimate(Some(90), moment(10)))),
            [&TaskChangeKind::Estimated {
                from: None,
                to: Some(90)
            }]
        );
        let sprint = crate::ids::SprintId::generate();
        let points = crate::sprint::StoryPoints::new(5).unwrap();
        assert_eq!(
            kinds(&change(&|task| task.plan_into(Some(sprint), moment(10)))),
            [&TaskChangeKind::Planned {
                from: None,
                to: Some(sprint)
            }]
        );
        assert_eq!(
            kinds(&change(&|task| task.set_points(Some(points), moment(10)))),
            [&TaskChangeKind::Pointed {
                from: None,
                to: Some(points)
            }]
        );
        assert_eq!(
            kinds(&change(&|task| task.reschedule(schedule, moment(10)))),
            [&TaskChangeKind::Rescheduled {
                from: Schedule::unscheduled(),
                to: schedule
            }]
        );
        assert_eq!(
            kinds(&change(&|task| task.block_on(blocker, moment(10)).unwrap())),
            [&TaskChangeKind::NowWaitsFor { blocker }]
        );
    }

    #[test]
    fn a_task_that_stops_waiting_says_for_which_task() {
        let mut before = task_in(&Workflow::standard());
        let blocker = crate::ids::TaskId::generate();
        before.block_on(blocker, moment(10)).unwrap();
        let mut after = before.clone();
        after.unblock_from(blocker, moment(11));

        assert_eq!(
            changes_between(&before, &after),
            [TaskChange {
                at: moment(11),
                kind: TaskChangeKind::NoLongerWaitsFor { blocker }
            }]
        );
    }

    #[test]
    fn a_tag_put_on_and_taken_off_is_told() {
        let before = task_in(&Workflow::standard());
        let kept = TagId::generate();
        let added = TagId::generate();
        let mut before_with_one = before.clone();
        before_with_one.put_tag(kept, moment(10));
        let mut after = before_with_one.clone();
        after.take_tag_off(kept, moment(11));
        after.put_tag(added, moment(11));

        assert_eq!(
            kinds(&changes_between(&before_with_one, &after)),
            [
                &TaskChangeKind::Tagged { tag: added },
                &TaskChangeKind::Untagged { tag: kept }
            ]
        );
    }

    #[test]
    fn a_value_set_changed_and_cleared_is_told() {
        let field = FieldId::generate();
        let empty = task_in(&Workflow::standard());
        let mut set = empty.clone();
        set.set_field_value(field, FieldValue::YesNo(false), moment(10));
        let mut changed = set.clone();
        changed.set_field_value(field, FieldValue::YesNo(true), moment(11));
        let mut cleared = changed.clone();
        cleared.clear_field_value(field, moment(12));

        assert_eq!(
            kinds(&changes_between(&empty, &set)),
            [&TaskChangeKind::FieldValueChanged {
                field,
                from: None,
                to: Some(FieldValue::YesNo(false))
            }]
        );
        assert_eq!(
            kinds(&changes_between(&set, &changed)),
            [&TaskChangeKind::FieldValueChanged {
                field,
                from: Some(FieldValue::YesNo(false)),
                to: Some(FieldValue::YesNo(true))
            }]
        );
        assert_eq!(
            kinds(&changes_between(&changed, &cleared)),
            [&TaskChangeKind::FieldValueChanged {
                field,
                from: Some(FieldValue::YesNo(true)),
                to: None
            }]
        );
        assert_eq!(changes_between(&cleared, &empty), []);
    }

    #[test]
    fn ticking_an_item_is_told_and_adding_or_moving_one_is_not() {
        let text = |words: &str| ChecklistText::parse(words).unwrap();
        let mut before = task_in(&Workflow::standard());
        let item = before
            .add_checklist_item(text("Order the stencil"), moment(10))
            .unwrap();
        let mut added = before.clone();
        added
            .add_checklist_item(text("Bake the board"), moment(11))
            .unwrap();
        let mut ticked = added.clone();
        ticked.tick_checklist_item(item, true, moment(12)).unwrap();
        ticked.move_checklist_item(item, 1, moment(12)).unwrap();
        let mut unticked = ticked.clone();
        unticked
            .tick_checklist_item(item, false, moment(13))
            .unwrap();

        assert_eq!(changes_between(&before, &added), []);
        assert_eq!(
            kinds(&changes_between(&added, &ticked)),
            [&TaskChangeKind::Ticked {
                item: text("Order the stencil")
            }]
        );
        assert_eq!(
            kinds(&changes_between(&ticked, &unticked)),
            [&TaskChangeKind::Unticked {
                item: text("Order the stencil")
            }]
        );
    }

    #[test]
    fn a_move_up_or_down_within_a_status_is_not_a_change() {
        let workflow = Workflow::standard();
        let before = task_in(&workflow);
        let mut after = before.clone();
        after.move_to(
            workflow.initial_status(),
            Rank::between(Some(Rank::FIRST), None),
            moment(10),
        );

        assert_eq!(changes_between(&before, &after), []);
    }

    #[test]
    fn several_changes_of_one_moment_come_in_a_fixed_order() {
        let workflow = Workflow::standard();
        let before = task_in(&workflow);
        let active = workflow
            .first_in(StatusCategory::Active)
            .expect("the standard workflow has a status for work under way");
        let mut after = before.clone();
        after.prioritize(Priority::High, moment(10));
        after.move_to(active, Rank::FIRST, moment(10));
        after.rename(title("Draw it"), moment(10));

        let changes = changes_between(&before, &after);

        assert!(matches!(
            kinds(&changes)[..],
            [
                TaskChangeKind::Renamed { .. },
                TaskChangeKind::Moved { .. },
                TaskChangeKind::Prioritized { .. }
            ]
        ));
        assert!(changes.iter().all(|change| change.at == moment(10)));
    }
}
