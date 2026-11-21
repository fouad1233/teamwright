//! Use cases for the custom fields of a project: listing, adding, renaming,
//! reordering and removing them, changing the choices of a choice field,
//! and setting and clearing the values of tasks
//! ([decision 0035](../../../docs/decisions/0035-custom-fields.md)).
//!
//! A field belongs to one project, and a task holds at most one value for
//! each field of its project. Removing a field clears its values from every
//! task first, and removing a choice clears the tasks that held it, so that
//! the history of each task says so. A view filtered by either shows the
//! tasks whatever they hold again. Deleting a project removes its fields in
//! storage, with the project.

use std::sync::Arc;

use teamwright_domain::{
    ChoiceId, ChoiceName, Color, Field, FieldId, FieldKind, FieldName, FieldUnit, FieldValue,
    ProjectId, Task, TaskId, View,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, FieldRepository, ProjectRepository, TaskRepository, ViewRepository};

/// The use cases about the custom fields of a project.
#[derive(Clone)]
pub struct FieldService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    views: Arc<dyn ViewRepository>,
    fields: Arc<dyn FieldRepository>,
    clock: Arc<dyn Clock>,
}

impl FieldService {
    /// Creates the service. The tasks are needed because they hold the
    /// values, and the views because a view may be filtered by a field.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        views: Arc<dyn ViewRepository>,
        fields: Arc<dyn FieldRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            tasks,
            views,
            fields,
            clock,
        }
    }

    /// Lists the fields of a project in their order.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Field>, ApplicationError> {
        self.require_project(project_id)?;
        fields_of(self.fields.as_ref(), project_id)
    }

    /// Adds a field at the end of the fields of a project. Its name is
    /// tidied, has at most 40 characters and is not the name of another
    /// field of the project, whatever the capitals. A number field may have
    /// a unit; a field of another kind may not.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name or the
    /// unit is empty or too long, if another field has the name
    /// (`field_name_taken`), if the field is not a number field and a unit
    /// is given, if the project has as many fields as it may
    /// (`rule_broken`), or if storage fails.
    pub fn add(
        &self,
        project_id: ProjectId,
        name: &str,
        kind: FieldKind,
        unit: Option<&str>,
    ) -> Result<Field, ApplicationError> {
        self.require_project(project_id)?;
        let name = FieldName::parse(name)?;
        let unit = parse_unit(unit)?;
        let fields = fields_of(self.fields.as_ref(), project_id)?;
        refuse_name_in_use(&fields, &name, None)?;
        if fields.len() >= Field::MOST_IN_A_PROJECT {
            return Err(ApplicationError::TooManyFields {
                max: Field::MOST_IN_A_PROJECT,
            });
        }
        let position = fields
            .iter()
            .map(|field| field.position().saturating_add(1))
            .max()
            .unwrap_or(0);
        let mut field = Field::add(project_id, name, kind, position);
        field.set_unit(unit)?;
        self.fields.save_all(std::slice::from_ref(&field))?;
        Ok(field)
    }

    /// Renames a field, by the same rules as a name given when it is added.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field, if the name is empty,
    /// too long or taken, or if storage fails.
    pub fn rename(&self, id: FieldId, name: &str) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        let name = FieldName::parse(name)?;
        let fields = fields_of(self.fields.as_ref(), field.project_id())?;
        refuse_name_in_use(&fields, &name, Some(id))?;
        field.rename(name);
        self.save(field)
    }

    /// Gives a number field a unit, or takes it away with nothing or an
    /// empty text.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field, if the unit is too long,
    /// if the field does not hold numbers, or if storage fails.
    pub fn set_unit(&self, id: FieldId, unit: Option<&str>) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        field.set_unit(parse_unit(unit)?)?;
        self.save(field)
    }

    /// Moves a field to a place among the fields of its project. Index 0 is
    /// the first place; an index past the end means the last, one before
    /// the start the first. Answers with the fields of the project in their
    /// new order.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field or if storage fails.
    pub fn move_to(&self, id: FieldId, index: i64) -> Result<Vec<Field>, ApplicationError> {
        let field = self.require_field(id)?;
        let mut fields: Vec<Field> = fields_of(self.fields.as_ref(), field.project_id())?
            .into_iter()
            .filter(|other| other.id() != id)
            .collect();
        let place = usize::try_from(index.max(0))
            .unwrap_or(usize::MAX)
            .min(fields.len());
        fields.insert(place, field);

        // Every field gets its place in the new order; only those whose
        // place changed are stored.
        let mut changed = Vec::new();
        for (position, field) in (0_u32..).zip(fields.iter_mut()) {
            if field.position() != position {
                field.move_to(position);
                changed.push(field.clone());
            }
        }
        self.fields.save_all(&changed)?;
        Ok(fields)
    }

    /// Removes a field. Its values are cleared from every task first, so
    /// that the history of each task says so, and a view filtered by it
    /// shows the tasks whatever they hold again.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field or if storage fails.
    pub fn remove(&self, id: FieldId) -> Result<(), ApplicationError> {
        let field = self.require_field(id)?;
        let now = self.clock.now();
        let cleared: Vec<Task> = self
            .tasks
            .list_for_project(field.project_id())?
            .into_iter()
            .filter_map(|mut task| task.clear_field_value(id, now).then_some(task))
            .collect();
        self.tasks.save_all(&cleared)?;
        let views: Vec<View> = self
            .views
            .list_for_project(field.project_id())?
            .into_iter()
            .filter_map(|mut view| view.forget_field(id).then_some(view))
            .collect();
        self.views.save_all(&views)?;
        // The tasks and the views are changed first. If removing the field
        // then fails, it is still there and held by no task, which is
        // harmless.
        self.fields.delete(id)?;
        Ok(())
    }

    /// Adds a choice at the end of the choices of a choice field, with a
    /// color or the next one in turn. Answers with the field.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field, if it is not a choice
    /// field, if the name is empty, too long or taken by another choice
    /// (`choice_name_taken`), if the field has as many choices as it may
    /// (`rule_broken`), or if storage fails.
    pub fn add_choice(
        &self,
        id: FieldId,
        name: &str,
        color: Option<Color>,
    ) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        field.add_choice(ChoiceName::parse(name)?, color)?;
        self.save(field)
    }

    /// Renames a choice. Answers with the field.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field or choice, if the name is
    /// empty, too long or taken, or if storage fails.
    pub fn rename_choice(
        &self,
        id: FieldId,
        choice: ChoiceId,
        name: &str,
    ) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        field.rename_choice(choice, ChoiceName::parse(name)?)?;
        self.save(field)
    }

    /// Gives a choice another color. Answers with the field.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field or choice, or if storage
    /// fails.
    pub fn recolor_choice(
        &self,
        id: FieldId,
        choice: ChoiceId,
        color: Color,
    ) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        field.recolor_choice(choice, color)?;
        self.save(field)
    }

    /// Removes a choice. The tasks that held it are cleared first, so that
    /// the history of each task says so, and a view filtered by it shows
    /// the tasks whatever they hold again. Answers with the field.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such field or choice, or if storage
    /// fails.
    pub fn remove_choice(&self, id: FieldId, choice: ChoiceId) -> Result<Field, ApplicationError> {
        let mut field = self.require_field(id)?;
        field.remove_choice(choice)?;
        let now = self.clock.now();
        let held = FieldValue::Choice(choice);
        let cleared: Vec<Task> = self
            .tasks
            .list_for_project(field.project_id())?
            .into_iter()
            .filter_map(|mut task| {
                (task.field_values().get(&id) == Some(&held) && task.clear_field_value(id, now))
                    .then_some(task)
            })
            .collect();
        self.tasks.save_all(&cleared)?;
        let views: Vec<View> = self
            .views
            .list_for_project(field.project_id())?
            .into_iter()
            .filter_map(|mut view| view.forget_choice(choice).then_some(view))
            .collect();
        self.views.save_all(&views)?;
        self.save(field)
    }

    /// Gives a task a value for a field of its project, in place of the one
    /// it held. Answers with the task; a task that holds the value already
    /// is not changed.
    ///
    /// # Errors
    ///
    /// Returns an error if the task or the field does not exist, if the
    /// field belongs to another project (`different_projects`), if the
    /// value is of another kind or names a choice the field does not have,
    /// or if storage fails.
    pub fn set_value(
        &self,
        task_id: TaskId,
        field_id: FieldId,
        value: FieldValue,
    ) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(task_id)?;
        let field = self.require_field(field_id)?;
        if field.project_id() != task.project_id() {
            return Err(ApplicationError::FieldOfAnotherProject);
        }
        field.check(&value)?;
        if task.set_field_value(field_id, value, self.clock.now()) {
            self.tasks.save(&task)?;
        }
        Ok(task)
    }

    /// Clears the value of a field from a task. Answers with the task; a
    /// task without a value is not changed. The field need not exist any
    /// more.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task or if storage fails.
    pub fn clear_value(
        &self,
        task_id: TaskId,
        field_id: FieldId,
    ) -> Result<Task, ApplicationError> {
        let mut task = self.require_task(task_id)?;
        if task.clear_field_value(field_id, self.clock.now()) {
            self.tasks.save(&task)?;
        }
        Ok(task)
    }

    /// Stores a field and answers with it.
    fn save(&self, field: Field) -> Result<Field, ApplicationError> {
        self.fields.save_all(std::slice::from_ref(&field))?;
        Ok(field)
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

    /// Loads a field, or reports that it does not exist.
    fn require_field(&self, id: FieldId) -> Result<Field, ApplicationError> {
        self.fields
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "field",
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

/// The fields of a project in their order: by position, and by identifier
/// where two computers left two fields at one place.
pub(crate) fn fields_of(
    fields: &dyn FieldRepository,
    project_id: ProjectId,
) -> Result<Vec<Field>, ApplicationError> {
    let mut listed = fields.list_for_project(project_id)?;
    listed.sort_by_key(|field| (field.position(), field.id()));
    Ok(listed)
}

/// Reads a unit; nothing, or a text with nothing in it, is no unit.
fn parse_unit(unit: Option<&str>) -> Result<Option<FieldUnit>, ApplicationError> {
    match unit {
        Some(unit) if !unit.trim().is_empty() => Ok(Some(FieldUnit::parse(unit)?)),
        _ => Ok(None),
    }
}

/// Refuses a name that another field of the project has, whatever the
/// capitals. The field `except` may keep its own name.
fn refuse_name_in_use(
    fields: &[Field],
    name: &FieldName,
    except: Option<FieldId>,
) -> Result<(), ApplicationError> {
    let taken = fields
        .iter()
        .filter(|field| Some(field.id()) != except)
        .any(|field| field.name().is_same_as(name.as_str()));
    if taken {
        return Err(ApplicationError::FieldNameTaken {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
