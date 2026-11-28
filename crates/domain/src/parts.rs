//! Which task is a part of which, when tasks are regrouped.
//!
//! A task may be a part of another task, at any depth, as a subtask is a
//! part of a task on the board. Tasks that exist are regrouped in two ways:
//! the manager moves one by hand, under another task or out from under
//! one; and the Advisor proposes new tasks that may be parts of tasks that
//! exist, together with moves of tasks that exist. Both keep the rules of
//! subtasks, written here once:
//!
//! - No task is a part of itself, directly or further down.
//! - A task with parts starts only once its parts are finished, so it waits
//!   for them. A part may therefore not wait for the task it is a part of,
//!   directly or through other tasks: none of them could ever start.
//!
//! The rules work on any kind of key, as [`DependencyGraph`] does: the
//! identifiers of the tasks of a project, or the references the Advisor
//! reads, such as `IOT-3`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::dependencies::DependencyGraph;
use crate::ids::TaskId;
use crate::task::Task;

/// A task a plan names: one that exists, by its key, or one the plan
/// proposes, by its place in the plan from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TaskInPlan<Key = TaskId> {
    /// A task the project has.
    Existing(Key),
    /// A new task of the plan, by its place from 0.
    Proposed(usize),
}

/// The tasks of a project as far as parts go: for each, the task it is a
/// part of and the tasks it waits for.
#[derive(Debug, Clone)]
pub struct Outline<Key = TaskId> {
    /// For each task, the task it is a part of, if any.
    wholes: HashMap<Key, Option<Key>>,
    /// For each task, the tasks it waits for.
    waits: HashMap<Key, Vec<Key>>,
}

impl Outline<TaskId> {
    /// The outline of a set of tasks, usually all the tasks of a project.
    pub fn of_tasks<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Self {
        Self::new(tasks.into_iter().map(|task| {
            (
                task.id(),
                task.parent_id(),
                task.blocked_by().iter().copied().collect(),
            )
        }))
    }
}

impl<Key: Copy + Eq + Hash + Ord> Outline<Key> {
    /// Builds the outline from each task with the task it is a part of and
    /// the tasks it waits for.
    pub fn new(tasks: impl IntoIterator<Item = (Key, Option<Key>, Vec<Key>)>) -> Self {
        let mut wholes = HashMap::new();
        let mut waits = HashMap::new();
        for (task, whole, waits_for) in tasks {
            wholes.insert(task, whole);
            waits.insert(task, waits_for);
        }
        Self { wholes, waits }
    }

    /// Whether the task is one of the outline.
    #[must_use]
    pub fn contains(&self, task: Key) -> bool {
        self.wholes.contains_key(&task)
    }

    /// The task a task of the outline is a part of, if any.
    #[must_use]
    pub fn whole_of(&self, task: Key) -> Option<Key> {
        self.wholes.get(&task).copied().flatten()
    }
}

/// A new task of a regrouping: what it is a part of, and the other new
/// tasks it waits for, by their places from 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewPart<Key = TaskId> {
    /// The task it is a part of, or nothing for a task of its own.
    pub part_of: Option<TaskInPlan<Key>>,
    /// The new tasks it waits for, by their places from 0.
    pub waits_for: Vec<usize>,
}

/// A task that exists, moved under another task or made a task of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartMove<Key = TaskId> {
    /// The task that is moved. Its own parts go with it.
    pub task: Key,
    /// The task it becomes a part of, or nothing for a task of its own.
    pub part_of: Option<TaskInPlan<Key>>,
}

/// What breaks the rules of parts in a regrouping, by the keys of the tasks
/// it is about. Each caller says it in the words its reader knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegroupingProblem<Key = TaskId> {
    /// A task the regrouping names is not one of the outline.
    NotThere(Key),
    /// A new task names a place that is not a new task of the regrouping.
    NoSuchPlace(usize),
    /// A task that exists is moved twice.
    MovedTwice(Key),
    /// `task` would be a part of `whole`, which is the task itself or lies
    /// below it already, as its part or further down.
    PartOfItsOwnPart {
        /// The task that would be a part.
        task: TaskInPlan<Key>,
        /// The task it would be a part of.
        whole: TaskInPlan<Key>,
    },
    /// `task` would be a part of `whole`, which it waits for, directly or
    /// through other tasks. The whole waits for its parts, so neither
    /// could start.
    PartWaitsForWhole {
        /// The task that would be a part.
        task: TaskInPlan<Key>,
        /// The task it would be a part of.
        whole: TaskInPlan<Key>,
    },
}

/// Checks a regrouping of the tasks of an outline: new tasks, which may be
/// parts of tasks that exist or of each other, and moves of tasks that
/// exist. A task that is moved takes its own parts with it.
///
/// Only what the regrouping changes is looked at. A circle the outline had
/// before is not the regrouping's to answer for.
///
/// # Errors
///
/// Returns the first problem found: a task that is not there, a place that
/// is not a new task, a task moved twice, a task that would be a part of
/// itself or of one of its own parts, or a part that would wait for the
/// task it is a part of.
pub fn check_regrouping<Key: Copy + Eq + Hash + Ord>(
    outline: &Outline<Key>,
    new_tasks: &[NewPart<Key>],
    moves: &[PartMove<Key>],
) -> Result<(), RegroupingProblem<Key>> {
    let named = |whole: Option<TaskInPlan<Key>>| match whole {
        Some(TaskInPlan::Existing(key)) if !outline.contains(key) => {
            Err(RegroupingProblem::NotThere(key))
        }
        Some(TaskInPlan::Proposed(place)) if place >= new_tasks.len() => {
            Err(RegroupingProblem::NoSuchPlace(place))
        }
        _ => Ok(()),
    };
    let mut moved: HashMap<Key, Option<TaskInPlan<Key>>> = HashMap::new();
    for one in moves {
        if !outline.contains(one.task) {
            return Err(RegroupingProblem::NotThere(one.task));
        }
        named(one.part_of)?;
        if moved.insert(one.task, one.part_of).is_some() {
            return Err(RegroupingProblem::MovedTwice(one.task));
        }
    }
    for new_task in new_tasks {
        named(new_task.part_of)?;
    }

    // What each task is a part of once the regrouping is made.
    let whole_of = |task: TaskInPlan<Key>| match task {
        TaskInPlan::Existing(key) => moved
            .get(&key)
            .copied()
            .unwrap_or_else(|| outline.whole_of(key).map(TaskInPlan::Existing)),
        TaskInPlan::Proposed(place) => new_tasks.get(place).and_then(|task| task.part_of),
    };
    // The tasks that get a new whole: the moved ones and the new parts.
    let regrouped: Vec<(TaskInPlan<Key>, TaskInPlan<Key>)> = moves
        .iter()
        .filter_map(|one| Some((TaskInPlan::Existing(one.task), one.part_of?)))
        .chain(
            new_tasks
                .iter()
                .enumerate()
                .filter_map(|(place, task)| Some((TaskInPlan::Proposed(place), task.part_of?))),
        )
        .collect();

    // No task lies below itself: from its new whole upwards, it is never
    // met again.
    for &(task, whole) in &regrouped {
        let mut seen = HashSet::new();
        let mut above = Some(whole);
        while let Some(current) = above {
            if current == task {
                return Err(RegroupingProblem::PartOfItsOwnPart { task, whole });
            }
            if !seen.insert(current) {
                break;
            }
            above = whole_of(current);
        }
    }

    // What waits for what as the outline stays: what each task waits for,
    // and each whole for its parts that are not moved; and what the new
    // tasks wait for. Each new link of a whole to its part is then added
    // one at a time, and none may close a circle.
    let mut graph = DependencyGraph::default();
    for (&task, waits) in &outline.waits {
        for &blocker in waits {
            graph.add_wait(TaskInPlan::Existing(task), TaskInPlan::Existing(blocker));
        }
    }
    for (&part, &whole) in &outline.wholes {
        if let Some(whole) = whole
            && !moved.contains_key(&part)
        {
            graph.add_wait(TaskInPlan::Existing(whole), TaskInPlan::Existing(part));
        }
    }
    for (place, task) in new_tasks.iter().enumerate() {
        for &earlier in &task.waits_for {
            graph.add_wait(TaskInPlan::Proposed(place), TaskInPlan::Proposed(earlier));
        }
    }
    for &(task, whole) in &regrouped {
        if graph.waits_for(task, whole) {
            return Err(RegroupingProblem::PartWaitsForWhole { task, whole });
        }
        graph.add_wait(whole, task);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A board, by references: 1 has the parts 2 and 3, 3 has the part 4,
    /// 5 is a task of its own that waits for 1, and 6 waits for 5.
    fn board() -> Outline<u32> {
        Outline::new([
            (1, None, vec![]),
            (2, Some(1), vec![]),
            (3, Some(1), vec![]),
            (4, Some(3), vec![]),
            (5, None, vec![1]),
            (6, None, vec![5]),
        ])
    }

    fn moving(task: u32, part_of: Option<TaskInPlan<u32>>) -> PartMove<u32> {
        PartMove { task, part_of }
    }

    fn under(key: u32) -> Option<TaskInPlan<u32>> {
        Some(TaskInPlan::Existing(key))
    }

    fn new_part(part_of: Option<TaskInPlan<u32>>, waits_for: Vec<usize>) -> NewPart<u32> {
        NewPart { part_of, waits_for }
    }

    #[test]
    fn a_task_can_be_moved_under_another_or_made_a_task_of_its_own() {
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(2, under(6))]),
            Ok(())
        );
        assert_eq!(check_regrouping(&board(), &[], &[moving(4, None)]), Ok(()));
        // A part may be a task its whole also waits for.
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(1, under(5))]),
            Ok(())
        );
        // A part may move to another task, with its own parts.
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(3, under(6))]),
            Ok(())
        );
    }

    #[test]
    fn a_task_is_not_a_part_of_itself_or_of_its_own_parts() {
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(1, under(1))]),
            Err(RegroupingProblem::PartOfItsOwnPart {
                task: TaskInPlan::Existing(1),
                whole: TaskInPlan::Existing(1),
            })
        );
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(1, under(4))]),
            Err(RegroupingProblem::PartOfItsOwnPart {
                task: TaskInPlan::Existing(1),
                whole: TaskInPlan::Existing(4),
            })
        );
    }

    #[test]
    fn a_part_does_not_wait_for_the_task_it_becomes_a_part_of() {
        // 6 waits for 5, which waits for 1: under 1, 1 would wait for it.
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(5, under(1))]),
            Err(RegroupingProblem::PartWaitsForWhole {
                task: TaskInPlan::Existing(5),
                whole: TaskInPlan::Existing(1),
            })
        );
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(6, under(3))]),
            Err(RegroupingProblem::PartWaitsForWhole {
                task: TaskInPlan::Existing(6),
                whole: TaskInPlan::Existing(3),
            })
        );
        // A task whose own part waits for the new whole: 2 is a part of 1
        // and waits for 3, so 1 under 3 would hold 3 up through 2.
        let board = Outline::new([(1, None, vec![]), (2, Some(1), vec![3]), (3, None, vec![])]);
        assert_eq!(
            check_regrouping(&board, &[], &[moving(1, under(3))]),
            Err(RegroupingProblem::PartWaitsForWhole {
                task: TaskInPlan::Existing(1),
                whole: TaskInPlan::Existing(3),
            })
        );
    }

    #[test]
    fn a_move_takes_the_old_link_to_its_whole_away() {
        // 2 waits for nothing; moved out of 1 and under 5, which waits for
        // 1, it no longer holds 1 up.
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(2, under(5))]),
            Ok(())
        );
        // Out of 1 and with 3 under 5 at once, 1 no longer waits for 3.
        let board = Outline::new([(1, None, vec![]), (3, Some(1), vec![]), (5, None, vec![1])]);
        assert_eq!(
            check_regrouping(&board, &[], &[moving(3, under(5))]),
            Ok(())
        );
    }

    #[test]
    fn new_tasks_may_be_parts_of_tasks_that_exist_and_take_them_in() {
        // A new module 0 holds 1 and 5; a new part of 6.
        let new_tasks = [new_part(None, vec![]), new_part(under(6), vec![])];
        let moves = [
            moving(1, Some(TaskInPlan::Proposed(0))),
            moving(5, Some(TaskInPlan::Proposed(0))),
        ];
        assert_eq!(check_regrouping(&board(), &new_tasks, &moves), Ok(()));
    }

    #[test]
    fn circles_through_new_tasks_are_found() {
        // New 0 is a part of 4; moving 1 under 0 puts 1 below itself.
        assert_eq!(
            check_regrouping(
                &board(),
                &[new_part(under(4), vec![])],
                &[moving(1, Some(TaskInPlan::Proposed(0)))],
            ),
            Err(RegroupingProblem::PartOfItsOwnPart {
                task: TaskInPlan::Existing(1),
                whole: TaskInPlan::Proposed(0),
            })
        );
        // New 0 is a part of 5, and 6 moves under 0. Then 5 waits for 0,
        // which waits for 6, which waits for 5.
        assert_eq!(
            check_regrouping(
                &board(),
                &[new_part(under(5), vec![])],
                &[moving(6, Some(TaskInPlan::Proposed(0)))],
            ),
            Err(RegroupingProblem::PartWaitsForWhole {
                task: TaskInPlan::Proposed(0),
                whole: TaskInPlan::Existing(5),
            })
        );
        // What new tasks wait for counts too: new 1 waits for new 0, which
        // takes 6 in; making new 1 a part of 6 would close a circle.
        assert_eq!(
            check_regrouping(
                &board(),
                &[new_part(None, vec![]), new_part(under(6), vec![0])],
                &[moving(6, Some(TaskInPlan::Proposed(0)))],
            ),
            Err(RegroupingProblem::PartWaitsForWhole {
                task: TaskInPlan::Proposed(1),
                whole: TaskInPlan::Existing(6),
            })
        );
    }

    #[test]
    fn what_is_named_must_be_there_and_moved_once() {
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(9, None)]),
            Err(RegroupingProblem::NotThere(9))
        );
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(2, under(9))]),
            Err(RegroupingProblem::NotThere(9))
        );
        assert_eq!(
            check_regrouping(&board(), &[new_part(under(9), vec![])], &[]),
            Err(RegroupingProblem::NotThere(9))
        );
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(2, Some(TaskInPlan::Proposed(0)))]),
            Err(RegroupingProblem::NoSuchPlace(0))
        );
        assert_eq!(
            check_regrouping(&board(), &[], &[moving(2, None), moving(2, under(5))]),
            Err(RegroupingProblem::MovedTwice(2))
        );
    }

    #[test]
    fn a_circle_the_board_had_before_is_not_held_against_a_regrouping() {
        // 7 and 8 wait for each other already, after a merge of two
        // computers that the next take-in repairs.
        let board = Outline::new([
            (1, None, vec![]),
            (2, None, vec![]),
            (7, None, vec![8]),
            (8, None, vec![7]),
        ]);
        assert_eq!(
            check_regrouping(&board, &[], &[moving(2, under(1))]),
            Ok(())
        );
    }
}
