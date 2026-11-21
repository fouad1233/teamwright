//! The permission requests of the specialists, as the manager sees them,
//! and the taking back of what was allowed (decision 0038).
//!
//! A request is recorded by the work of the specialists, from the stream
//! of a turn, and answered there too, because an answer starts a turn
//! ([`crate::WorkService::answer_permission`]). This service lists them,
//! and takes back a permission that was allowed for a task or for always.

use std::sync::Arc;

use teamwright_domain::{PermissionRequest, PermissionRequestId, ProjectId};

use crate::error::ApplicationError;
use crate::ports::{Clock, PermissionRepository, ProjectRepository};

/// The use cases of the permission requests.
#[derive(Clone)]
pub struct PermissionService {
    projects: Arc<dyn ProjectRepository>,
    permissions: Arc<dyn PermissionRepository>,
    clock: Arc<dyn Clock>,
}

impl PermissionService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        permissions: Arc<dyn PermissionRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            permissions,
            clock,
        }
    }

    /// The requests of a project, answered or not, the oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn requests_of_project(
        &self,
        project_id: ProjectId,
    ) -> Result<Vec<PermissionRequest>, ApplicationError> {
        if self.projects.find(project_id)?.is_none() {
            return Err(ApplicationError::NotFound {
                kind: "project",
                id: project_id.to_string(),
            });
        }
        Ok(self.permissions.requests_of_project(project_id)?)
    }

    /// The requests the manager has not answered, in every project that is
    /// not archived, the oldest first.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn open_requests(&self) -> Result<Vec<PermissionRequest>, ApplicationError> {
        let mut open = Vec::new();
        for project in self.projects.list()? {
            if project.is_archived() {
                continue;
            }
            open.extend(
                self.permissions
                    .requests_of_project(project.id())?
                    .into_iter()
                    .filter(PermissionRequest::is_open),
            );
        }
        open.sort_by_key(|request| (request.asked_at(), request.id()));
        Ok(open)
    }

    /// Takes back a permission that was allowed for a task or for always.
    /// Turns that start from now on are not given it; a turn that runs
    /// keeps what it was started with.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such request, if nothing of it
    /// stands (`rule_broken`), or if storage fails.
    pub fn withdraw(&self, id: PermissionRequestId) -> Result<PermissionRequest, ApplicationError> {
        let mut request =
            self.permissions
                .find_request(id)?
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "permission request",
                    id: id.to_string(),
                })?;
        request.withdraw(self.clock.now())?;
        self.permissions.save_request(&request)?;
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use teamwright_domain::{
        Answered, ComputerId, ConversationId, MemberId, PermissionKind, PermissionRule,
        PermissionScope, Project, ProjectKey, ProjectName, SaidIn, TurnId, Workflow,
    };

    use super::*;
    use crate::in_memory::{InMemoryPermissionRepository, InMemoryProjectRepository, ManualClock};

    struct Scene {
        service: PermissionService,
        permissions: Arc<InMemoryPermissionRepository>,
        projects: Arc<InMemoryProjectRepository>,
        project: Project,
    }

    fn scene() -> Scene {
        let clock = Arc::new(ManualClock::starting_at(
            Utc.with_ymd_and_hms(2026, 11, 21, 22, 0, 0).unwrap(),
        ));
        let projects = Arc::new(InMemoryProjectRepository::default());
        let permissions = Arc::new(InMemoryPermissionRepository::default());
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::engineering(),
            clock.now(),
        );
        projects.save(&project).unwrap();
        Scene {
            service: PermissionService::new(projects.clone(), permissions.clone(), clock),
            permissions,
            projects,
            project,
        }
    }

    fn asked(project: &Project) -> PermissionRequest {
        PermissionRequest::ask(
            project.id(),
            SaidIn {
                member_id: MemberId::generate(),
                conversation_id: ConversationId::generate(),
                turn_id: TurnId::generate(),
            },
            None,
            PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
            "To run the tests.",
            Utc.with_ymd_and_hms(2026, 11, 21, 21, 0, 0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn open_requests_are_listed_and_answered_ones_are_not() {
        let scene = scene();
        let open = asked(&scene.project);
        let mut answered = asked(&scene.project);
        answered
            .refuse(
                "",
                Answered {
                    at: Utc.with_ymd_and_hms(2026, 11, 21, 21, 5, 0).unwrap(),
                    computer: ComputerId::generate(),
                    next_turn: TurnId::generate(),
                },
            )
            .unwrap();
        scene.permissions.save_request(&open).unwrap();
        scene.permissions.save_request(&answered).unwrap();

        assert_eq!(scene.service.open_requests().unwrap(), [open]);
        assert_eq!(
            scene
                .service
                .requests_of_project(scene.project.id())
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn the_requests_of_an_archived_project_wait_for_nobody() {
        let scene = scene();
        scene
            .permissions
            .save_request(&asked(&scene.project))
            .unwrap();
        let mut project = scene.project.clone();
        project.archive(Utc.with_ymd_and_hms(2026, 11, 21, 22, 0, 0).unwrap());
        scene.projects.save(&project).unwrap();
        assert!(scene.service.open_requests().unwrap().is_empty());
    }

    #[test]
    fn a_standing_permission_is_taken_back_and_a_refused_one_cannot_be() {
        let scene = scene();
        let mut always = asked(&scene.project);
        always
            .allow(
                PermissionScope::Always,
                PermissionRule::new(PermissionKind::Command, "npm test").unwrap(),
                "",
                Answered {
                    at: Utc.with_ymd_and_hms(2026, 11, 21, 21, 5, 0).unwrap(),
                    computer: ComputerId::generate(),
                    next_turn: TurnId::generate(),
                },
            )
            .unwrap();
        scene.permissions.save_request(&always).unwrap();

        let withdrawn = scene.service.withdraw(always.id()).unwrap();
        assert!(withdrawn.standing().is_none());
        assert!(scene.service.withdraw(always.id()).is_err());
        let open = asked(&scene.project);
        scene.permissions.save_request(&open).unwrap();
        assert!(scene.service.withdraw(open.id()).is_err());
    }
}
