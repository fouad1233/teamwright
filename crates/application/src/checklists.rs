//! Use cases for the checklist of a task: adding, renaming, ticking,
//! moving and removing its items
//! ([decision 0030](../../../docs/decisions/0030-tags-and-checklists.md)).
//!
//! The checklist is a part of its task. Every use case here loads the task,
//! lets the domain change its checklist, stores the task and answers with
//! it, so the interface gets the whole checklist back after each change.

use std::sync::Arc;

use teamwright_domain::{ChecklistItemId, ChecklistText, DomainError, Task, TaskId, Timestamp};

use crate::error::ApplicationError;
use crate::ports::{Clock, TaskRepository};

/// The use cases about the checklists of tasks.
#[derive(Clone)]
pub struct ChecklistService {
    tasks: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
}

impl ChecklistService {
    /// Creates the service.
    #[must_use]
    pub fn new(tasks: Arc<dyn TaskRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { tasks, clock }
    }

    /// Adds an item, not done, at the end of the checklist of a task. Its
    /// text is tidied and has at most 200 characters. Answers with the
    /// task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if the text is empty or
    /// too long, if the checklist is full (`rule_broken`), or if storage
    /// fails.
    pub fn add_item(&self, task_id: TaskId, text: &str) -> Result<Task, ApplicationError> {
        let text = ChecklistText::parse(text)?;
        self.change(task_id, |task, now| {
            task.add_checklist_item(text, now).map(|_| ())
        })
    }

    /// Gives an item of the checklist of a task another text, by the same
    /// rules. Answers with the task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or item (`not_found`), if
    /// the text is empty or too long, or if storage fails.
    pub fn rename_item(
        &self,
        task_id: TaskId,
        item_id: ChecklistItemId,
        text: &str,
    ) -> Result<Task, ApplicationError> {
        let text = ChecklistText::parse(text)?;
        self.change(task_id, |task, now| {
            task.rename_checklist_item(item_id, text, now)
        })
    }

    /// Marks an item of the checklist of a task done, or not done. Answers
    /// with the task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or item (`not_found`), or
    /// if storage fails.
    pub fn tick_item(
        &self,
        task_id: TaskId,
        item_id: ChecklistItemId,
        done: bool,
    ) -> Result<Task, ApplicationError> {
        self.change(task_id, |task, now| {
            task.tick_checklist_item(item_id, done, now)
        })
    }

    /// Moves an item of the checklist of a task to another place. Place 0
    /// is the first; a place past the end is the last, and one before the
    /// start the first. Answers with the task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or item (`not_found`), or
    /// if storage fails.
    pub fn move_item(
        &self,
        task_id: TaskId,
        item_id: ChecklistItemId,
        index: i64,
    ) -> Result<Task, ApplicationError> {
        let place = usize::try_from(index.max(0)).unwrap_or(usize::MAX);
        self.change(task_id, |task, now| {
            task.move_checklist_item(item_id, place, now)
        })
    }

    /// Removes an item from the checklist of a task. Answers with the task.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or item (`not_found`), or
    /// if storage fails.
    pub fn remove_item(
        &self,
        task_id: TaskId,
        item_id: ChecklistItemId,
    ) -> Result<Task, ApplicationError> {
        self.change(task_id, |task, now| {
            task.remove_checklist_item(item_id, now)
        })
    }

    /// Loads a task, changes it, and stores it if the change made a
    /// difference.
    fn change(
        &self,
        task_id: TaskId,
        change: impl FnOnce(&mut Task, Timestamp) -> Result<(), DomainError>,
    ) -> Result<Task, ApplicationError> {
        let mut task = self
            .tasks
            .find(task_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "task",
                id: task_id.to_string(),
            })?;
        let before = task.clone();
        change(&mut task, self.clock.now())?;
        if task != before {
            self.tasks.save(&task)?;
        }
        Ok(task)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;
    use teamwright_domain::changes_between;

    use super::*;
    use crate::in_memory::{
        InMemoryAreaRepository, InMemoryConversationRepository, InMemoryMemberRepository,
        InMemoryPlacesRepository, InMemoryProjectRepository, InMemorySprintRepository,
        InMemoryTaskRepository, InMemoryViewRepository, ManualClock,
    };
    use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
    use crate::tasks::{NewTask, TaskService};

    fn start_of_day() -> Timestamp {
        chrono::Utc.with_ymd_and_hms(2026, 11, 11, 9, 0, 0).unwrap()
    }

    /// A checklist service, and a task to change, in a project of its own.
    fn workspace() -> (ChecklistService, TaskService, Task) {
        let projects_store = Arc::new(InMemoryProjectRepository::default());
        let tasks_store = Arc::new(InMemoryTaskRepository::default());
        let members_store = Arc::new(InMemoryMemberRepository::default());
        let conversations_store = Arc::new(InMemoryConversationRepository::default());
        let sprints_store = Arc::new(InMemorySprintRepository::new(tasks_store.clone()));
        let areas_store = Arc::new(InMemoryAreaRepository::default());
        let clock = Arc::new(ManualClock::starting_at(start_of_day()));
        let projects = ProjectService::new(
            projects_store.clone(),
            tasks_store.clone(),
            members_store.clone(),
            conversations_store.clone(),
            Arc::new(InMemoryPlacesRepository::default()),
            sprints_store.clone(),
            Arc::new(InMemoryViewRepository::default()),
            areas_store.clone(),
            clock.clone(),
        );
        let tasks = TaskService::new(
            projects_store,
            tasks_store.clone(),
            members_store,
            conversations_store,
            sprints_store,
            areas_store,
            clock.clone(),
        );
        let project = projects
            .create(NewProject {
                name: "Air quality node".to_owned(),
                workflow: WorkflowTemplate::Standard,
                ..NewProject::default()
            })
            .unwrap();
        let task = tasks
            .create(NewTask::titled(project.id(), "Assemble the first board"))
            .unwrap();
        (ChecklistService::new(tasks_store, clock), tasks, task)
    }

    fn texts(task: &Task) -> Vec<&str> {
        task.checklist()
            .items()
            .iter()
            .map(|item| item.text().as_str())
            .collect()
    }

    #[test]
    fn items_are_added_ticked_moved_renamed_and_removed_and_kept() {
        let (checklists, tasks, task) = workspace();

        checklists
            .add_item(task.id(), " Order the  stencil ")
            .unwrap();
        let task = checklists.add_item(task.id(), "Bake the board").unwrap();
        let [first, second] = [0, 1].map(|index| task.checklist().items()[index].id());
        checklists.tick_item(task.id(), first, true).unwrap();
        checklists.move_item(task.id(), second, -3).unwrap();
        checklists
            .rename_item(task.id(), second, "Bake it at 240 degrees")
            .unwrap();

        let stored = tasks.get(task.id()).unwrap();
        assert_eq!(
            texts(&stored),
            ["Bake it at 240 degrees", "Order the stencil"]
        );
        assert_eq!(stored.checklist().progress(), (1, 2));

        let stored = checklists.remove_item(task.id(), first).unwrap();
        assert_eq!(texts(&stored), ["Bake it at 240 degrees"]);
        assert_eq!(texts(&tasks.get(task.id()).unwrap()), texts(&stored));
    }

    #[test]
    fn a_tick_shows_in_the_history_of_the_task() {
        let (checklists, tasks, task) = workspace();
        let task = checklists.add_item(task.id(), "Order the stencil").unwrap();
        let item = task.checklist().items()[0].id();

        let ticked = checklists.tick_item(task.id(), item, true).unwrap();

        assert_eq!(changes_between(&task, &ticked).len(), 1);
        let history = tasks.history(task.id()).unwrap();
        assert!(matches!(
            history.last().map(|change| &change.kind),
            Some(teamwright_domain::TaskChangeKind::Ticked { item }) if item.as_str() == "Order the stencil"
        ));
    }

    #[test]
    fn an_empty_or_long_text_and_a_missing_item_are_refused() {
        let (checklists, _, task) = workspace();

        assert_eq!(
            checklists.add_item(task.id(), "  ").unwrap_err().code(),
            "rule_broken"
        );
        assert_eq!(
            checklists
                .add_item(task.id(), &"x".repeat(201))
                .unwrap_err()
                .code(),
            "rule_broken"
        );
        let missing = ChecklistItemId::generate();
        assert_eq!(
            checklists
                .tick_item(task.id(), missing, true)
                .unwrap_err()
                .code(),
            "not_found"
        );
        assert_eq!(
            checklists
                .add_item(TaskId::generate(), "Anything")
                .unwrap_err()
                .code(),
            "not_found"
        );
    }

    #[test]
    fn ticking_an_item_that_is_done_already_changes_nothing() {
        let (checklists, tasks, task) = workspace();
        let task = checklists.add_item(task.id(), "Order the stencil").unwrap();
        let item = task.checklist().items()[0].id();
        let ticked = checklists.tick_item(task.id(), item, true).unwrap();

        let again = checklists.tick_item(task.id(), item, true).unwrap();

        assert_eq!(again, ticked);
        assert_eq!(tasks.history(task.id()).unwrap().len(), 2);
    }
}
