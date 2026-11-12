//! Use cases for the views of a project: listing, adding, renaming,
//! filtering, moving and removing them
//! ([decision 0021](../../../docs/decisions/0021-views-the-manager-can-add.md)).
//!
//! A new project gets its six views from
//! [`ProjectService`](crate::ProjectService), which also removes them with
//! the project. A member who leaves the team is forgotten by the views that
//! showed only their tasks, in [`TeamService`](crate::TeamService).

use std::sync::Arc;

use teamwright_domain::{ProjectId, View, ViewAssignee, ViewFilter, ViewId, ViewKind, ViewName};

use crate::error::ApplicationError;
use crate::ports::{MemberRepository, ProjectRepository, ViewRepository};

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

/// The use cases about the views of a project.
#[derive(Clone)]
pub struct ViewService {
    projects: Arc<dyn ProjectRepository>,
    members: Arc<dyn MemberRepository>,
    views: Arc<dyn ViewRepository>,
}

impl ViewService {
    /// Creates the service. The members are needed because a filter may
    /// only name a member of the team of the project.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        members: Arc<dyn MemberRepository>,
        views: Arc<dyn ViewRepository>,
    ) -> Self {
        Self {
            projects,
            members,
            views,
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

    /// Replaces the filter of a view. Only a board, a list, a Gantt chart
    /// and a calendar can be filtered, and only by a member of the team of
    /// the project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such view, if it is a backlog or a
    /// dashboard, if the filter names a member who is not on the team
    /// (`member_not_on_team`), or if storage fails.
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
