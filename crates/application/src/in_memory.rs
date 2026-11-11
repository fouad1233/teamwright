//! Implementations of the ports that keep everything in memory.
//!
//! They exist for tests and demonstrations. Nothing is written to disk, and
//! everything is gone when the value is dropped.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::TimeDelta;
use teamwright_domain::{Member, MemberId, Project, ProjectId, Task, TaskId, Timestamp};

use crate::ports::{Clock, MemberRepository, ProjectRepository, StorageError, TaskRepository};

/// Locks a mutex, and keeps working even if another thread panicked while it
/// held the lock. The values guarded here are valid after every single
/// operation, so there is never anything to repair.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps projects in memory.
#[derive(Debug, Default)]
pub struct InMemoryProjectRepository {
    projects: Mutex<BTreeMap<ProjectId, Project>>,
}

impl ProjectRepository for InMemoryProjectRepository {
    fn save(&self, project: &Project) -> Result<(), StorageError> {
        lock(&self.projects).insert(project.id(), project.clone());
        Ok(())
    }

    fn find(&self, id: ProjectId) -> Result<Option<Project>, StorageError> {
        Ok(lock(&self.projects).get(&id).cloned())
    }

    fn list(&self) -> Result<Vec<Project>, StorageError> {
        // Identifiers are ordered by creation time, so walking the map in key
        // order lists the oldest project first.
        Ok(lock(&self.projects).values().cloned().collect())
    }
}

/// Keeps tasks in memory.
#[derive(Debug, Default)]
pub struct InMemoryTaskRepository {
    tasks: Mutex<BTreeMap<TaskId, Task>>,
}

impl TaskRepository for InMemoryTaskRepository {
    fn save(&self, task: &Task) -> Result<(), StorageError> {
        lock(&self.tasks).insert(task.id(), task.clone());
        Ok(())
    }

    fn save_all(&self, tasks: &[Task]) -> Result<(), StorageError> {
        let mut stored = lock(&self.tasks);
        for task in tasks {
            stored.insert(task.id(), task.clone());
        }
        Ok(())
    }

    fn find(&self, id: TaskId) -> Result<Option<Task>, StorageError> {
        Ok(lock(&self.tasks).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Task>, StorageError> {
        Ok(lock(&self.tasks)
            .values()
            .filter(|task| task.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, ids: &[TaskId]) -> Result<(), StorageError> {
        let mut stored = lock(&self.tasks);
        for id in ids {
            stored.remove(id);
        }
        Ok(())
    }
}

/// Keeps the members of the teams in memory.
#[derive(Debug, Default)]
pub struct InMemoryMemberRepository {
    members: Mutex<BTreeMap<MemberId, Member>>,
}

impl MemberRepository for InMemoryMemberRepository {
    fn save(&self, member: &Member) -> Result<(), StorageError> {
        lock(&self.members).insert(member.id(), member.clone());
        Ok(())
    }

    fn find(&self, id: MemberId) -> Result<Option<Member>, StorageError> {
        Ok(lock(&self.members).get(&id).cloned())
    }

    fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Member>, StorageError> {
        Ok(lock(&self.members)
            .values()
            .filter(|member| member.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn delete(&self, id: MemberId) -> Result<(), StorageError> {
        lock(&self.members).remove(&id);
        Ok(())
    }
}

/// A clock that shows the time it was last set to.
#[derive(Debug)]
pub struct ManualClock {
    now: Mutex<Timestamp>,
}

impl ManualClock {
    /// Creates a clock that shows `now` until it is changed.
    #[must_use]
    pub const fn starting_at(now: Timestamp) -> Self {
        Self {
            now: Mutex::new(now),
        }
    }

    /// Makes the clock show a different time.
    pub fn set(&self, now: Timestamp) {
        *lock(&self.now) = now;
    }

    /// Moves the clock forward.
    pub fn advance(&self, by: TimeDelta) {
        *lock(&self.now) += by;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        *lock(&self.now)
    }
}
