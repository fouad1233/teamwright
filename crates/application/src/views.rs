//! Use cases for the views of a project: listing, adding, renaming,
//! filtering, giving content to, moving and removing them
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! A new project gets its six views from
//! [`ProjectService`](crate::ProjectService), which also removes them with
//! the project. A member who leaves the team is forgotten by the views that
//! showed only their tasks, in [`TeamService`](crate::TeamService).

use std::sync::Arc;

use teamwright_domain::{
    FieldFilter, FieldId, FieldKind, FieldValue, Grouping, ProjectId, TagId, View, ViewAssignee,
    ViewFilter, ViewId, ViewKind, ViewName,
};

use crate::error::ApplicationError;
use crate::ports::{
    FieldRepository, MemberRepository, ProjectRepository, TagRepository, ViewRepository,
};

/// What is needed to add a view to a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewView {
    /// The project the view belongs to.
    pub project_id: ProjectId,
    /// Its kind.
    pub kind: ViewKind,
    /// The name, before it is tidied. Left out or empty, the view is named
    /// after its kind: "Board", or "Board 2" if the project has a view
    /// called "Board" already.
    pub name: Option<String>,
}

/// What a view asks of a task made inside it, so that the task stays in the
/// view instead of vanishing the moment it is made. Only a definite value is
/// asked for: a tag, and the choice of a choice field or "yes" of a yes-or-no
/// field. A filter for "no", which also shows tasks that hold nothing, asks
/// for nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewTaskDemands {
    /// The tag the new task carries.
    pub tag: Option<TagId>,
    /// The value the new task holds for a custom field.
    pub field_value: Option<(FieldId, FieldValue)>,
}

impl NewTaskDemands {
    /// What a filter asks of the tasks made in a view that has it.
    #[must_use]
    pub fn of_filter(filter: &ViewFilter) -> Self {
        let field_value = match filter.field {
            Some(FieldFilter::Choice { field, choice }) => {
                Some((field, FieldValue::Choice(choice)))
            }
            Some(FieldFilter::YesNo { field, yes: true }) => Some((field, FieldValue::YesNo(true))),
            Some(FieldFilter::YesNo { yes: false, .. }) | None => None,
        };
        Self {
            tag: filter.tag,
            field_value,
        }
    }
}

/// The use cases about the views of a project.
#[derive(Clone)]
pub struct ViewService {
    projects: Arc<dyn ProjectRepository>,
    members: Arc<dyn MemberRepository>,
    views: Arc<dyn ViewRepository>,
    tags: Arc<dyn TagRepository>,
    fields: Arc<dyn FieldRepository>,
}

impl ViewService {
    /// Creates the service. The members are needed because a filter may
    /// only name a member of the team of the project, the tags because it
    /// may only name a tag of the project, and the fields because it may
    /// only name a field of the project and one of its values.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        members: Arc<dyn MemberRepository>,
        views: Arc<dyn ViewRepository>,
        tags: Arc<dyn TagRepository>,
        fields: Arc<dyn FieldRepository>,
    ) -> Self {
        Self {
            projects,
            members,
            views,
            tags,
            fields,
        }
    }

    /// Lists the views of a project in the order the manager gave them.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<View>, ApplicationError> {
        self.require_project(project_id)?;
        self.views_of(project_id)
    }

    /// Adds a view at the end of the views of a project, showing every
    /// task. A name is tidied, has at most 40 characters and is not the
    /// name of another view of the project, whatever the capitals. Left out
    /// or empty, the view is named after its kind, numbered if that name is
    /// taken.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the project has a
    /// view of a kind it can have only one of already (`view_kind_taken`),
    /// if the name is too long or taken (`view_name_taken`), or if storage
    /// fails.
    pub fn add(&self, new_view: NewView) -> Result<View, ApplicationError> {
        self.require_project(new_view.project_id)?;
        let views = self.views_of(new_view.project_id)?;
        refuse_second_of_kind(&views, new_view.kind)?;
        let asked = new_view.name.unwrap_or_default();
        let name = if asked.trim().is_empty() {
            ViewName::for_new_view(new_view.kind, views.iter().map(View::name))
        } else {
            let name = ViewName::parse(&asked)?;
            refuse_name_in_use(&views, &name, None)?;
            name
        };
        let position = views
            .iter()
            .map(|view| view.position().saturating_add(1))
            .max()
            .unwrap_or(0);
        let view = View::add(new_view.project_id, new_view.kind, name, position);
        self.views.save_all(std::slice::from_ref(&view))?;
        Ok(view)
    }

    /// Renames a view, by the same rules as a name given when it is added.
    /// A view may change the capitals of its own name.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if the name is empty, too
    /// long or taken, or if storage fails.
    pub fn rename(&self, id: ViewId, name: &str) -> Result<View, ApplicationError> {
        let mut view = self.require_view(id)?;
        let name = ViewName::parse(name)?;
        let views = self.views_of(view.project_id())?;
        refuse_name_in_use(&views, &name, Some(id))?;
        view.rename(name);
        self.views.save_all(std::slice::from_ref(&view))?;
        Ok(view)
    }

    /// Replaces the filter of a view. Only the kinds that show some of the
    /// tasks can be filtered ([`ViewKind::can_be_filtered`]), and only by a
    /// member of the team of the project and a tag of the project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if its kind cannot be
    /// filtered, if the filter names a member who is not on the team
    /// (`member_not_on_team`), a tag that does not exist (`not_found`) or a
    /// tag of another project (`different_projects`), a field or a choice
    /// that does not exist (`not_found`), a field of another project
    /// (`different_projects`) or a field that is neither a choice nor yes
    /// or no (`rule_broken`), or if storage fails.
    pub fn set_filter(&self, id: ViewId, filter: ViewFilter) -> Result<View, ApplicationError> {
        let mut view = self.require_view(id)?;
        // The kind is checked before the member: a backlog is refused as a
        // backlog, whoever the filter names.
        view.set_filter(filter)?;
        if let ViewAssignee::Member(member_id) = filter.assignee {
            let on_team = self
                .members
                .find(member_id)?
                .is_some_and(|member| member.project_id() == view.project_id());
            if !on_team {
                return Err(ApplicationError::MemberNotOnTeam);
            }
        }
        if let Some(tag_id) = filter.tag {
            let tag = self
                .tags
                .find(tag_id)?
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "tag",
                    id: tag_id.to_string(),
                })?;
            if tag.project_id() != view.project_id() {
                return Err(ApplicationError::TagOfAnotherProject);
            }
        }
        if let Some(by_field) = filter.field {
            self.check_field_filter(&view, by_field)?;
        }
        self.views.save_all(std::slice::from_ref(&view))?;
        Ok(view)
    }

    /// Changes what a view groups its tasks by. Only a board and a list can
    /// be grouped by something other than status
    /// ([`ViewKind::can_be_grouped`]).
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if its kind cannot be
    /// grouped by what is asked (`rule_broken`), or if storage fails.
    pub fn set_grouping(&self, id: ViewId, grouping: Grouping) -> Result<View, ApplicationError> {
        let mut view = self.require_view(id)?;
        view.set_grouping(grouping)?;
        self.views.save_all(std::slice::from_ref(&view))?;
        Ok(view)
    }

    /// Replaces what a view keeps of its own, or clears it with nothing
    /// ([decision 0024](../../../docs/decisions/0024-a-view-keeps-content-of-its-own.md)).
    /// The content is text the interface of the kind writes and reads; it
    /// is not read here. Any kind of view may keep content.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if the content has more
    /// than 2 MiB (`rule_broken`), or if storage fails.
    pub fn set_content(
        &self,
        id: ViewId,
        content: Option<String>,
    ) -> Result<View, ApplicationError> {
        let mut view = self.require_view(id)?;
        view.set_content(content)?;
        self.views.save_all(std::slice::from_ref(&view))?;
        Ok(view)
    }

    /// Moves a view to another place among the views of its project. Place
    /// 0 is the first; a place past the end is the last, and one before the
    /// start the first. Answers with all the views of the project in their
    /// new order.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view or if storage fails.
    pub fn move_to(&self, id: ViewId, index: i64) -> Result<Vec<View>, ApplicationError> {
        let view = self.require_view(id)?;
        let mut views: Vec<View> = self
            .views_of(view.project_id())?
            .into_iter()
            .filter(|other| other.id() != id)
            .collect();
        let place = usize::try_from(index.max(0))
            .unwrap_or(usize::MAX)
            .min(views.len());
        views.insert(place, view);

        // Every view gets its place in the new order; only those whose place
        // changed are stored.
        let mut changed = Vec::new();
        for (position, view) in (0_u32..).zip(views.iter_mut()) {
            if view.position() != position {
                view.move_to(position);
                changed.push(view.clone());
            }
        }
        self.views.save_all(&changed)?;
        Ok(views)
    }

    /// Removes a view. The last view of a project cannot be removed.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if it is the last of its
    /// project (`last_view`), or if storage fails.
    pub fn remove(&self, id: ViewId) -> Result<(), ApplicationError> {
        let view = self.require_view(id)?;
        if self.views.list_for_project(view.project_id())?.len() <= 1 {
            return Err(ApplicationError::LastView {
                name: view.name().to_string(),
            });
        }
        self.views.delete(id)?;
        Ok(())
    }

    /// What a view asks of a task made inside it
    /// ([`NewTaskDemands`]), to be given to
    /// [`TaskService::create_in_view`](crate::TaskService::create_in_view).
    /// A view of a kind that cannot be filtered asks for nothing.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if it belongs to another
    /// project than the task will (`different_projects`), or if storage
    /// fails.
    pub fn demands_of_new_tasks(
        &self,
        id: ViewId,
        project_id: ProjectId,
    ) -> Result<NewTaskDemands, ApplicationError> {
        let view = self.require_view(id)?;
        if view.project_id() != project_id {
            return Err(ApplicationError::DifferentProjects);
        }
        if !view.kind().can_be_filtered() {
            return Ok(NewTaskDemands::default());
        }
        Ok(NewTaskDemands::of_filter(&view.filter()))
    }

    /// The views of a project in their order.
    fn views_of(&self, project_id: ProjectId) -> Result<Vec<View>, ApplicationError> {
        let mut views = self.views.list_for_project(project_id)?;
        sort_views(&mut views);
        Ok(views)
    }

    /// Makes sure that a project exists.
    fn require_project(&self, id: ProjectId) -> Result<(), ApplicationError> {
        self.projects
            .find(id)?
            .map(|_| ())
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    /// Checks that a filter by a custom field names a field of the project
    /// of the view, of a kind that filters (a choice, or yes or no), and,
    /// for a choice, one of its choices.
    fn check_field_filter(
        &self,
        view: &View,
        by_field: FieldFilter,
    ) -> Result<(), ApplicationError> {
        let field_id = by_field.field();
        let field = self
            .fields
            .find(field_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "field",
                id: field_id.to_string(),
            })?;
        if field.project_id() != view.project_id() {
            return Err(ApplicationError::FieldOfAnotherProject);
        }
        match (by_field, field.kind()) {
            (FieldFilter::Choice { choice, .. }, FieldKind::Choice) => {
                field.check(&FieldValue::Choice(choice))?;
                Ok(())
            }
            (FieldFilter::YesNo { .. }, FieldKind::YesNo) => Ok(()),
            _ => Err(ApplicationError::FieldNotFilterable),
        }
    }

    /// Loads a view, or reports that it does not exist.
    fn require_view(&self, id: ViewId) -> Result<View, ApplicationError> {
        self.views
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "view",
                id: id.to_string(),
            })
    }
}

/// Puts views in the order the manager gave them. Two views at the same
/// place, which two computers can leave behind, keep the order of their
/// identifiers.
pub(crate) fn sort_views(views: &mut [View]) {
    views.sort_by_key(|view| (view.position(), view.id()));
}

/// Refuses a second backlog or a second dashboard.
fn refuse_second_of_kind(views: &[View], kind: ViewKind) -> Result<(), ApplicationError> {
    if !kind.is_single() {
        return Ok(());
    }
    match views.iter().find(|view| view.kind() == kind) {
        Some(existing) => Err(ApplicationError::ViewKindTaken {
            kind: kind.as_str(),
            name: existing.name().to_string(),
        }),
        None => Ok(()),
    }
}

/// Refuses a name that another view of the project has, whatever the
/// capitals. The view `except` may keep its own name.
fn refuse_name_in_use(
    views: &[View],
    name: &ViewName,
    except: Option<ViewId>,
) -> Result<(), ApplicationError> {
    let taken = views
        .iter()
        .filter(|view| Some(view.id()) != except)
        .any(|view| view.name().is_same_as(name.as_str()));
    if taken {
        return Err(ApplicationError::ViewNameTaken {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
