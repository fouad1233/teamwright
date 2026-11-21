//! The groups of the tasks of a view: the lanes of a board and the sections
//! of a list
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! The rule that puts tasks into groups is [`group_tasks`] in the domain.
//! This use case gathers what it needs from storage: the tasks, the team,
//! the areas, the sprints and the tags of the project of the view.

use std::sync::Arc;

use teamwright_domain::{GroupingContext, TaskGroup, ViewId, group_tasks};

use crate::error::ApplicationError;
use crate::ports::{
    AreaRepository, MemberRepository, ProjectRepository, SprintRepository, TagRepository,
    TaskRepository, ViewRepository,
};
use crate::sprints::as_listed;

/// The use case that groups the tasks of a view.
#[derive(Clone)]
pub struct GroupingService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    members: Arc<dyn MemberRepository>,
    areas: Arc<dyn AreaRepository>,
    sprints: Arc<dyn SprintRepository>,
    tags: Arc<dyn TagRepository>,
    views: Arc<dyn ViewRepository>,
}

impl GroupingService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        members: Arc<dyn MemberRepository>,
        areas: Arc<dyn AreaRepository>,
        sprints: Arc<dyn SprintRepository>,
        tags: Arc<dyn TagRepository>,
        views: Arc<dyn ViewRepository>,
    ) -> Self {
        Self {
            projects,
            tasks,
            members,
            areas,
            sprints,
            tags,
            views,
        }
    }

    /// The groups of the tasks of a project as a view groups them, in the
    /// order of [`group_tasks`]. The view is asked only for what it groups
    /// by: the filter of the view is not applied here, because the board
    /// keeps every task and is told which get a card.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view or project, or if storage
    /// fails.
    pub fn groups_of_view(&self, view_id: ViewId) -> Result<Vec<TaskGroup>, ApplicationError> {
        let view = self
            .views
            .find(view_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "view",
                id: view_id.to_string(),
            })?;
        let project =
            self.projects
                .find(view.project_id())?
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "project",
                    id: view.project_id().to_string(),
                })?;
        let tasks = self.tasks.list_for_project(project.id())?;
        let mut members = self.members.list_for_project(project.id())?;
        members.sort_by_key(|member| (member.joined_at(), member.id()));
        let areas = self.areas.list_for_project(project.id())?;
        let mut sprints = self.sprints.list_for_project(project.id())?;
        sprints.sort_by(as_listed);
        let tags = self.tags.list_for_project(project.id())?;
        Ok(group_tasks(
            view.grouping(),
            &GroupingContext {
                workflow: project.workflow(),
                members: &members,
                areas: &areas,
                sprints: &sprints,
                tags: &tags,
            },
            &tasks,
        ))
    }
}

#[cfg(test)]
mod tests;
