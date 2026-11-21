//! Use cases for the tags of a project: listing, adding, renaming,
//! recoloring and removing them, and putting them on tasks and taking them
//! off ([decision 0030](../../../docs/decisions/0030-tags-and-checklists.md)).
//!
//! A tag belongs to one project. Removing a tag takes it off every task
//! that carries it, and a view that showed only the tasks with it shows the
//! tasks with any tag again. Deleting a project removes its tags in
//! storage, with the project.

use std::sync::Arc;

use teamwright_domain::{Color, ProjectId, Tag, TagId, TagName, Task, TaskId, View};

use crate::error::ApplicationError;
use crate::ports::{Clock, ProjectRepository, TagRepository, TaskRepository, ViewRepository};

/// The use cases about the tags of a project.
#[derive(Clone)]
pub struct TagService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    views: Arc<dyn ViewRepository>,
    tags: Arc<dyn TagRepository>,
    clock: Arc<dyn Clock>,
}

impl TagService {
    /// Creates the service. The tasks are needed because tags are put on
    /// them and a removed tag is taken off them, and the views because a
    /// view may show only the tasks with one tag.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        views: Arc<dyn ViewRepository>,
        tags: Arc<dyn TagRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            views,
            tags,
            clock,
        }
    }

    /// Lists the tags of a project by name, whatever the capitals.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Tag>, ApplicationError> {
        self.require_project(project_id)?;
        tags_of(self.tags.as_ref(), project_id)
    }

    /// Adds a tag to a project. Its name is tidied, has at most 30
    /// characters and is not the name of another tag of the project,
    /// whatever the capitals. Without a color, it gets the next color in
    /// turn ([`Tag::color_in_turn`]).
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name is empty
    /// or too long, if another tag of the project has it
    /// (`tag_name_taken`), if the project has as many tags as it may
    /// (`rule_broken`), or if storage fails.
    pub fn add(
        &self,
        project_id: ProjectId,
        name: &str,
        color: Option<Color>,
    ) -> Result<Tag, ApplicationError> {
        self.require_project(project_id)?;
        let name = TagName::parse(name)?;
        let tags = tags_of(self.tags.as_ref(), project_id)?;
        refuse_name_in_use(&tags, &name, None)?;
        if tags.len() >= Tag::MOST_IN_A_PROJECT {
            return Err(ApplicationError::TooManyTags {
                max: Tag::MOST_IN_A_PROJECT,
            });
        }
        let color = color.unwrap_or_else(|| Tag::color_in_turn(tags.len()));
        let tag = Tag::add(project_id, name, color);
        self.tags.save_all(std::slice::from_ref(&tag))?;
        Ok(tag)
    }

    /// Renames a tag, by the same rules as a name given when it is added.
    /// A tag may change the capitals of its own name.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such tag, if the name is empty, too
    /// long or taken, or if storage fails.
    pub fn rename(&self, id: TagId, name: &str) -> Result<Tag, ApplicationError> {
        let mut tag = self.require_tag(id)?;
        let name = TagName::parse(name)?;
        let tags = tags_of(self.tags.as_ref(), tag.project_id())?;
        refuse_name_in_use(&tags, &name, Some(id))?;
        tag.rename(name);
        self.tags.save_all(std::slice::from_ref(&tag))?;
        Ok(tag)
    }

    /// Gives a tag another color.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such tag or if storage fails.
    pub fn recolor(&self, id: TagId, color: Color) -> Result<Tag, ApplicationError> {
        let mut tag = self.require_tag(id)?;
        tag.recolor(color);
        self.tags.save_all(std::slice::from_ref(&tag))?;
        Ok(tag)
    }

    /// Removes a tag. It is taken off every task that carries it first, so
    /// that the history of each task says so, and a view that showed only
    /// the tasks with it shows the tasks with any tag again.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such tag or if storage fails.
    pub fn remove(&self, id: TagId) -> Result<(), ApplicationError> {
        let tag = self.require_tag(id)?;
        let now = self.clock.now();
        let untagged: Vec<Task> = self
            .tasks
            .list_for_project(tag.project_id())?
            .into_iter()
            .filter_map(|mut task| task.take_tag_off(id, now).then_some(task))
            .collect();
        self.tasks.save_all(&untagged)?;
        let views: Vec<View> = self
            .views
            .list_for_project(tag.project_id())?
            .into_iter()
            .filter_map(|mut view| view.forget_tag(id).then_some(view))
            .collect();
        self.views.save_all(&views)?;
        // The tasks and the views are changed first. If removing the tag
        // then fails, it is still there and on no task, which is harmless.
        self.tags.delete(id)?;
        Ok(())
    }

    /// Puts a tag of its project on a task. Answers with the task; a task
    /// that carries the tag already is not changed.
    ///
    /// # Errors
    ///
    /// Returns an error if the task or the tag does not exist, if the tag
    /// belongs to another project (`different_projects`), or if storage
    /// fails.
    pub fn put_on(&self, task_id: TaskId, tag_id: TagId) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(task_id)?;
        let tag = self.require_tag(tag_id)?;
        if tag.project_id() != task.project_id() {
            return Err(ApplicationError::TagOfAnotherProject);
        }
        if task.put_tag(tag_id, self.clock.now()) {
            self.tasks.save(&task)?;
        }
        Ok(task)
    }

    /// Puts a tag on a task by its name, as the manager types it in the
    /// panel of the task: the tag of the project with that name, whatever
    /// the capitals, or a new tag with that name and the next color in
    /// turn. Answers with the task and the tag.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if the name is empty or
    /// too long, if a new tag would be one too many (`rule_broken`), or if
    /// storage fails.
    pub fn put_on_by_name(
        &self,
        task_id: TaskId,
        name: &str,
    ) -> Result<(Task, Tag), ApplicationError> {
        let task = self.require_task(task_id)?;
        let name = TagName::parse(name)?;
        let existing = tags_of(self.tags.as_ref(), task.project_id())?
            .into_iter()
            .find(|tag| tag.name().is_same_as(name.as_str()));
        let tag = match existing {
            Some(tag) => tag,
            None => self.add(task.project_id(), name.as_str(), None)?,
        };
        let task = self.put_on(task_id, tag.id())?;
        Ok((task, tag))
    }

    /// Takes a tag off a task. Answers with the task; a task that does not
    /// carry the tag is not changed. The tag need not exist any more.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn take_off(&self, task_id: TaskId, tag_id: TagId) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(task_id)?;
        if task.take_tag_off(tag_id, self.clock.now()) {
            self.tasks.save(&task)?;
        }
        Ok(task)
    }

    /// Loads a project, or reports that it does not exist.
    fn require_project(&self, id: ProjectId) -> Result<(), ApplicationError> {
        self.projects
            .find(id)?
            .map(|_| ())
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Loads a tag, or reports that it does not exist.
    fn require_tag(&self, id: TagId) -> Result<Tag, ApplicationError> {
        self.tags
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "tag",
                id: id.to_string(),
            })
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

/// The tags of a project by name, whatever the capitals. Two tags with one
/// name, which two computers can leave behind until they are repaired, keep
/// the order of their identifiers.
pub(crate) fn tags_of(
    tags: &dyn TagRepository,
    project_id: ProjectId,
) -> Result<Vec<Tag>, ApplicationError> {
    let mut listed = tags.list_for_project(project_id)?;
    listed.sort_by(|one, other| {
        one.name()
            .as_str()
            .to_lowercase()
            .cmp(&other.name().as_str().to_lowercase())
            .then_with(|| one.id().cmp(&other.id()))
    });
    Ok(listed)
}

/// Refuses a name that another tag of the project has, whatever the
/// capitals. The tag `except` may keep its own name.
fn refuse_name_in_use(
    tags: &[Tag],
    name: &TagName,
    except: Option<TagId>,
) -> Result<(), ApplicationError> {
    let taken = tags
        .iter()
        .filter(|tag| Some(tag.id()) != except)
        .any(|tag| tag.name().is_same_as(name.as_str()));
    if taken {
        return Err(ApplicationError::TagNameTaken {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
