//! Use cases for the areas of a project: listing, adding, renaming, moving
//! and removing them, and moving a task into another one
//! ([decision 0022](../../../docs/decisions/0022-areas-inside-a-project.md)).
//!
//! A new project gets its area "General" from
//! [`ProjectService`](crate::ProjectService), which also removes the areas
//! with the project. A new task goes into an area in
//! [`TaskService`](crate::TaskService), and the tasks of an approved plan of
//! the Advisor go into the first area in
//! [`WorkService`](crate::WorkService).

use std::sync::Arc;

use teamwright_domain::{Area, AreaId, AreaName, Project, ProjectId, Task, TaskId};

use crate::error::ApplicationError;
use crate::ports::{AreaRepository, Clock, ProjectRepository, TaskRepository};
use crate::tasks::{reference, subtree_of};

/// The use cases about the areas of a project.
#[derive(Clone)]
pub struct AreaService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    areas: Arc<dyn AreaRepository>,
    clock: Arc<dyn Clock>,
}

impl AreaService {
    /// Creates the service. The tasks are needed because every task is in
    /// an area: removing an area moves its tasks, and a task can be moved
    /// into another area.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        areas: Arc<dyn AreaRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            areas,
            clock,
        }
    }

    /// Lists the areas of a project in the order the manager gave them. A
    /// project has at least one.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Area>, ApplicationError> {
        self.require_project(project_id)?;
        areas_of(self.areas.as_ref(), project_id)
    }

    /// Adds an area at the end of the areas of a project. Its name is
    /// tidied, has at most 40 characters and is not the name of another
    /// area of the project, whatever the capitals.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name is empty
    /// or too long, if another area of the project has it
    /// (`area_name_taken`), or if storage fails.
    pub fn add(&self, project_id: ProjectId, name: &str) -> Result<Area, ApplicationError> {
        let project = self.require_project(project_id)?;
        let areas = areas_of(self.areas.as_ref(), project.id())?;
        let name = AreaName::parse(name)?;
        refuse_name_in_use(&areas, &name, None)?;
        let position = areas
            .iter()
            .map(|area| area.position().saturating_add(1))
            .max()
            .unwrap_or(0);
        let area = Area::add(project.id(), name, position);
        self.areas.save_all(std::slice::from_ref(&area))?;
        Ok(area)
    }

    /// Renames an area, by the same rules as a name given when it is added.
    /// An area may change the capitals of its own name. It keeps its place.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such area, if the name is empty, too
    /// long or taken, or if storage fails.
    pub fn rename(&self, id: AreaId, name: &str) -> Result<Area, ApplicationError> {
        let mut area = self.require_area(id)?;
        let name = AreaName::parse(name)?;
        let areas = areas_of(self.areas.as_ref(), area.project_id())?;
        refuse_name_in_use(&areas, &name, Some(id))?;
        area.rename(name);
        self.areas.save_all(std::slice::from_ref(&area))?;
        Ok(area)
    }

    /// Moves an area to another place among the areas of its project. Place
    /// 0 is the first; a place past the end is the last, and one before the
    /// start the first. Answers with all the areas of the project in their
    /// new order.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such area or if storage fails.
    pub fn move_to(&self, id: AreaId, index: i64) -> Result<Vec<Area>, ApplicationError> {
        let area = self.require_area(id)?;
        let mut areas: Vec<Area> = areas_of(self.areas.as_ref(), area.project_id())?
            .into_iter()
            .filter(|other| other.id() != id)
            .collect();
        let place = usize::try_from(index.max(0))
            .unwrap_or(usize::MAX)
            .min(areas.len());
        areas.insert(place, area);

        // Every area gets its place in the new order; only those whose place
        // changed are stored.
        let mut changed = Vec::new();
        for (position, area) in (0_u32..).zip(areas.iter_mut()) {
            if area.position() != position {
                area.move_to(position);
                changed.push(area.clone());
            }
        }
        self.areas.save_all(&changed)?;
        Ok(areas)
    }

    /// Removes an area. The last area of a project stays. If tasks are in
    /// it, `move_tasks_to` names another area of the same project for them,
    /// where they keep their status and their place. Without tasks,
    /// `move_tasks_to` is not looked at.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such area, if it is the last of its
    /// project (`last_area`), if it holds tasks and no other area is named
    /// for them (`area_in_use`), if the area named does not exist or belongs
    /// to another project, or if storage fails.
    pub fn remove(
        &self,
        id: AreaId,
        move_tasks_to: Option<AreaId>,
    ) -> Result<(), ApplicationError> {
        let area = self.require_area(id)?;
        if self.areas.list_for_project(area.project_id())?.len() <= 1 {
            return Err(ApplicationError::LastArea {
                name: area.name().to_string(),
            });
        }
        let mut in_area: Vec<Task> = self
            .tasks
            .list_for_project(area.project_id())?
            .into_iter()
            .filter(|task| task.area_id() == id)
            .collect();
        if !in_area.is_empty() {
            let Some(target) = move_tasks_to.filter(|target| *target != id) else {
                return Err(ApplicationError::AreaInUse {
                    name: area.name().to_string(),
                    count: in_area.len(),
                });
            };
            let target = self.require_area(target)?;
            if target.project_id() != area.project_id() {
                return Err(ApplicationError::TasksToAreaOfAnotherProject);
            }
            let now = self.clock.now();
            for task in &mut in_area {
                task.put_into_area(target.id(), now);
            }
            // The tasks are moved first. If removing the area then fails, it
            // is still there and empty, which is harmless.
            self.tasks.save_all(&in_area)?;
        }
        self.areas.delete(id)?;
        Ok(())
    }

    /// Moves a task, with its subtasks, into another area of its project. A
    /// subtask is not moved by itself: it goes where the task it is a part
    /// of goes. Answers with the task; a task that is in the area already is
    /// not changed.
    ///
    /// # Errors
    ///
    /// Returns an error if the task or the area does not exist, if the task
    /// is a subtask (`subtask_moved_alone`), if the area belongs to another
    /// project, or if storage fails.
    pub fn move_task(&self, task_id: TaskId, area_id: AreaId) -> Result<Task, ApplicationError> {
        let task = self.require_task(task_id)?;
        let project = self.require_project(task.project_id())?;
        let area = self.require_area(area_id)?;
        if let Some(parent) = task
            .parent_id()
            .map(|parent_id| self.tasks.find(parent_id))
            .transpose()?
            .flatten()
        {
            return Err(ApplicationError::SubtaskMovedAlone {
                task: reference(&project, &task),
                parent: reference(&project, &parent),
            });
        }
        if area.project_id() != task.project_id() {
            return Err(ApplicationError::AreaOfAnotherProject);
        }

        let now = self.clock.now();
        let tasks_of_project = self.tasks.list_for_project(project.id())?;
        let moving = subtree_of(task_id, &tasks_of_project);
        let moved: Vec<Task> = tasks_of_project
            .into_iter()
            .filter(|other| moving.contains(&other.id()) && other.area_id() != area_id)
            .map(|mut other| {
                other.put_into_area(area_id, now);
                other
            })
            .collect();
        self.tasks.save_all(&moved)?;
        Ok(moved
            .into_iter()
            .find(|moved| moved.id() == task_id)
            .unwrap_or(task))
    }

    /// Loads a project, or reports that it does not exist.
    fn require_project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Loads an area, or reports that it does not exist.
    fn require_area(&self, id: AreaId) -> Result<Area, ApplicationError> {
        require_area(self.areas.as_ref(), id)
    }

    /// Loads a task, or reports that it does not exist.
    fn require_task(&self, id: TaskId) -> Result<Task, ApplicationError> {
        self.tasks
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "task",
                id: id.to_string(),
            })
    }
}

/// The areas of a project in the order the manager gave them.
pub(crate) fn areas_of(
    areas: &dyn AreaRepository,
    project_id: ProjectId,
) -> Result<Vec<Area>, ApplicationError> {
    let mut listed = areas.list_for_project(project_id)?;
    sort_areas(&mut listed);
    Ok(listed)
}

/// Puts areas in the order the manager gave them. Two areas at the same
/// place, which two computers can leave behind, keep the order of their
/// identifiers.
pub(crate) fn sort_areas(areas: &mut [Area]) {
    areas.sort_by_key(|area| (area.position(), area.id()));
}

/// The first area of a project, which a task goes into when no area is
/// named.
pub(crate) fn first_area_of(
    areas: &dyn AreaRepository,
    project_id: ProjectId,
) -> Result<Area, ApplicationError> {
    areas_of(areas, project_id)?
        .into_iter()
        .next()
        .ok_or(ApplicationError::ProjectWithoutAreas)
}

/// Loads an area, or reports that it does not exist.
pub(crate) fn require_area(
    areas: &dyn AreaRepository,
    id: AreaId,
) -> Result<Area, ApplicationError> {
    areas.find(id)?.ok_or_else(|| ApplicationError::NotFound {
        kind: "area",
        id: id.to_string(),
    })
}

/// The area a new task goes into: the one it names, or the first area of
/// its project. A subtask goes into the area of its task: left out, it goes
/// there, and another area is refused.
pub(crate) fn area_for_new_task(
    areas: &dyn AreaRepository,
    project: &Project,
    area_id: Option<AreaId>,
    parent: Option<&Task>,
) -> Result<AreaId, ApplicationError> {
    let Some(area_id) = area_id else {
        return match parent {
            Some(parent) => Ok(parent.area_id()),
            None => Ok(first_area_of(areas, project.id())?.id()),
        };
    };
    let area = require_area(areas, area_id)?;
    if area.project_id() != project.id() {
        return Err(ApplicationError::AreaOfAnotherProject);
    }
    if let Some(parent) = parent
        && parent.area_id() != area.id()
    {
        return Err(ApplicationError::SubtaskInAnotherArea {
            parent: reference(project, parent),
        });
    }
    Ok(area.id())
}

/// Refuses a name that another area of the project has, whatever the
/// capitals. The area `except` may keep its own name.
fn refuse_name_in_use(
    areas: &[Area],
    name: &AreaName,
    except: Option<AreaId>,
) -> Result<(), ApplicationError> {
    let taken = areas
        .iter()
        .filter(|area| Some(area.id()) != except)
        .any(|area| area.name().is_same_as(name.as_str()));
    if taken {
        return Err(ApplicationError::AreaNameTaken {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
