//! Dependencies between tasks: which task waits for which.

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ids::TaskId;
use crate::task::Task;

/// The "waits for" relation among a set of tasks.
///
/// A single task can refuse to wait for itself, but only a view of all the
/// tasks can tell that "A waits for B" and "B waits for A" form a loop that
/// would leave both waiting forever. This type is that view.
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// For each task, the tasks it waits for.
    blockers: HashMap<TaskId, BTreeSet<TaskId>>,
}

impl DependencyGraph {
    /// Builds the graph of a set of tasks, usually all tasks of one project.
    pub fn from_tasks<'a>(tasks: impl IntoIterator<Item = &'a Task>) -> Self {
        let blockers = tasks
            .into_iter()
            .filter(|task| !task.blocked_by().is_empty())
            .map(|task| (task.id(), task.blocked_by().clone()))
            .collect();
        Self { blockers }
    }

    /// Tells whether making `task` wait for `blocker` would close a loop.
    ///
    /// That is the case when the blocker is the task itself, or when the
    /// blocker already waits for the task, directly or through other tasks.
    #[must_use]
    pub fn would_create_cycle(&self, task: TaskId, blocker: TaskId) -> bool {
        task == blocker || self.waits_for(blocker, task)
    }

    /// Tells whether `waiting` waits for `target`, directly or through other
    /// tasks.
    #[must_use]
    pub fn waits_for(&self, waiting: TaskId, target: TaskId) -> bool {
        let mut to_visit = vec![waiting];
        let mut visited = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if !visited.insert(current) {
                continue;
            }
            for &blocker in self.blockers.get(&current).into_iter().flatten() {
                if blocker == target {
                    return true;
                }
                to_visit.push(blocker);
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::ids::{AreaId, ProjectId};
    use crate::task::{Rank, TaskDraft};
    use crate::text::TaskTitle;
    use crate::time::Timestamp;
    use crate::workflow::Workflow;

    fn now() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    fn task(title: &str) -> Task {
        Task::create(
            ProjectId::generate(),
            AreaId::generate(),
            1,
            Workflow::standard().initial_status(),
            Rank::FIRST,
            TaskDraft::titled(TaskTitle::parse(title).unwrap()),
            now(),
        )
    }

    /// Hardware first, then firmware, then the enclosure:
    /// the enclosure waits for the firmware, which waits for the hardware.
    fn chain() -> (Task, Task, Task) {
        let hardware = task("Design the board");
        let mut firmware = task("Write the firmware");
        let mut enclosure = task("Design the enclosure");
        firmware.block_on(hardware.id(), now()).unwrap();
        enclosure.block_on(firmware.id(), now()).unwrap();
        (hardware, firmware, enclosure)
    }

    #[test]
    fn a_task_waits_for_its_direct_blocker() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        assert!(graph.waits_for(firmware.id(), hardware.id()));
    }

    #[test]
    fn a_task_waits_for_the_blockers_of_its_blockers() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        assert!(graph.waits_for(enclosure.id(), hardware.id()));
    }

    #[test]
    fn waiting_has_a_direction() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        assert!(!graph.waits_for(hardware.id(), enclosure.id()));
    }

    #[test]
    fn waiting_for_itself_would_be_a_cycle() {
        let hardware = task("Design the board");
        let graph = DependencyGraph::from_tasks([&hardware]);

        assert!(graph.would_create_cycle(hardware.id(), hardware.id()));
    }

    #[test]
    fn two_tasks_waiting_for_each_other_would_be_a_cycle() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        assert!(graph.would_create_cycle(hardware.id(), firmware.id()));
    }

    #[test]
    fn closing_a_longer_loop_would_be_a_cycle() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        assert!(graph.would_create_cycle(hardware.id(), enclosure.id()));
    }

    #[test]
    fn a_second_path_to_the_same_task_is_not_a_cycle() {
        let (hardware, firmware, enclosure) = chain();
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure]);

        // The enclosure already waits for the hardware through the firmware.
        // Waiting for it directly as well is redundant, not circular.
        assert!(!graph.would_create_cycle(enclosure.id(), hardware.id()));
    }

    #[test]
    fn unrelated_tasks_can_depend_on_each_other_in_either_direction() {
        let (hardware, firmware, enclosure) = chain();
        let manual = task("Write the user manual");
        let graph = DependencyGraph::from_tasks([&hardware, &firmware, &enclosure, &manual]);

        assert!(!graph.would_create_cycle(manual.id(), enclosure.id()));
        assert!(!graph.would_create_cycle(hardware.id(), manual.id()));
    }

    #[test]
    fn a_graph_that_already_contains_a_loop_is_still_searched_to_the_end() {
        let mut first = task("First");
        let mut second = task("Second");
        let outsider = task("Outsider");
        // A loop like this cannot be created through the rules, but stored
        // data could contain one. The search must still terminate.
        first.block_on(second.id(), now()).unwrap();
        second.block_on(first.id(), now()).unwrap();
        let graph = DependencyGraph::from_tasks([&first, &second, &outsider]);

        assert!(!graph.waits_for(first.id(), outsider.id()));
    }
}
