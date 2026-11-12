//! Use cases for the team of a project.

use std::sync::Arc;

use teamwright_domain::{
    Assignee, Color, Effort, Member, MemberId, MemberKind, MemberName, MemberProfile, ModelName,
    Project, ProjectId, Task, Timestamp,
};

use crate::error::ApplicationError;
use crate::ports::{
    Clock, ConversationRepository, MemberRepository, ProjectRepository, TaskRepository,
};
use crate::work::WorkInProgress;

/// What is needed to put someone on a team.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMember {
    /// The project whose team the member joins.
    pub project_id: ProjectId,
    /// What the member is called on the team.
    pub name: String,
    /// Whether the member is an AI agent or a person.
    pub kind: MemberKind,
    /// What the member is on the team for. May be empty.
    pub responsibility: String,
    /// The model of an AI specialist. Absent, empty or only whitespace, it
    /// is left to the program. A person must not have one.
    pub model: Option<String>,
    /// The effort of an AI specialist. Absent, it is left to the program.
    /// A person must not have one.
    pub effort: Option<Effort>,
}

/// The use cases that put members on a team, change them and take them off.
#[derive(Clone)]
pub struct TeamService {
    projects: Arc<dyn ProjectRepository>,
    members: Arc<dyn MemberRepository>,
    tasks: Arc<dyn TaskRepository>,
    conversations: Arc<dyn ConversationRepository>,
    clock: Arc<dyn Clock>,
}

impl TeamService {
    /// Creates the service. The conversations are needed because a member
    /// that leaves leaves its conversations behind, and a member that is
    /// working cannot leave.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        members: Arc<dyn MemberRepository>,
        tasks: Arc<dyn TaskRepository>,
        conversations: Arc<dyn ConversationRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            projects,
            members,
            tasks,
            conversations,
            clock,
        }
    }

    /// Puts someone on the team of a project.
    ///
    /// The member gets a color that sets it apart from the others on the
    /// team.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the name is not
    /// valid, if another member of the team has the same name, if a person
    /// is given a model or an effort (`no_specialist`), if the name of the
    /// model is not valid, or if storage fails.
    pub fn add(&self, new_member: NewMember) -> Result<Member, ApplicationError> {
        let project = self.require_project(new_member.project_id)?;
        let name = MemberName::parse(&new_member.name)?;
        let team = self.members.list_for_project(project.id())?;
        refuse_name_in_use(&team, &name, None)?;
        let profile = profile_for(
            new_member.kind,
            &name,
            new_member.model.as_deref(),
            new_member.effort,
        )?;

        let colors_in_use: Vec<Color> = team.iter().map(Member::color).collect();
        let now = self.clock.now();
        let mut member = Member::create(
            project.id(),
            name,
            new_member.kind,
            new_member.responsibility.trim().to_owned(),
            Member::color_for_newcomer(&colors_in_use),
            now,
        );
        member.set_profile(profile, now)?;
        self.members.save(&member)?;
        Ok(member)
    }

    /// Lists the team of a project. The member who joined first comes first.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn list_for_project(&self, project_id: ProjectId) -> Result<Vec<Member>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let mut team = self.members.list_for_project(project.id())?;
        team.sort_by_key(|member| (member.joined_at(), member.id()));
        Ok(team)
    }

    /// Finds a member.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member or if storage fails.
    pub fn get(&self, id: MemberId) -> Result<Member, ApplicationError> {
        self.require_member(id)
    }

    /// Gives a member a new name.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member, if the name is not
    /// valid, if another member of the team has the same name, or if storage
    /// fails.
    pub fn rename(&self, id: MemberId, name: &str) -> Result<Member, ApplicationError> {
        let name = MemberName::parse(name)?;
        self.change(id, |member, team, now| {
            refuse_name_in_use(team, &name, Some(member.id()))?;
            member.rename(name, now);
            Ok(())
        })
    }

    /// Replaces what a member is on the team for.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member or if storage fails.
    pub fn set_responsibility(
        &self,
        id: MemberId,
        responsibility: &str,
    ) -> Result<Member, ApplicationError> {
        self.change(id, |member, _team, now| {
            member.set_responsibility(responsibility.trim().to_owned(), now);
            Ok(())
        })
    }

    /// Says which model an AI specialist runs on and how hard it thinks.
    /// A model that is absent, empty or only whitespace, and an absent
    /// effort, are left to the program.
    ///
    /// A person has neither. Giving a person the empty profile is allowed
    /// and changes nothing but the time of the last change.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member, if the member is a
    /// person and the profile is not empty (`no_specialist`), if the name of
    /// the model is not valid, or if storage fails.
    pub fn set_profile(
        &self,
        id: MemberId,
        model: Option<&str>,
        effort: Option<Effort>,
    ) -> Result<Member, ApplicationError> {
        self.change(id, |member, _team, now| {
            let profile = profile_for(member.kind(), member.name(), model, effort)?;
            member.set_profile(profile, now)?;
            Ok(())
        })
    }

    /// Takes a member off the team. The tasks that were given to the member
    /// are given to nobody. Its conversations stay on record, without it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member, if it is working on
    /// something, or if storage fails.
    pub fn remove(&self, id: MemberId) -> Result<(), ApplicationError> {
        let member = self.require_member(id)?;
        WorkInProgress {
            projects: self.projects.as_ref(),
            tasks: self.tasks.as_ref(),
            members: self.members.as_ref(),
            conversations: self.conversations.as_ref(),
        }
        .refuse_while_busy(&member)?;
        let now = self.clock.now();

        for mut conversation in self.conversations.list_for_project(member.project_id())? {
            if conversation.member_id() == Some(id) {
                conversation.forget_member();
                self.conversations.save_conversation(&conversation)?;
            }
        }
        let handed_back: Vec<Task> = self
            .tasks
            .list_for_project(member.project_id())?
            .into_iter()
            .filter(|task| task.assignee() == Some(Assignee::Member(id)))
            .map(|mut task| {
                task.assign(None, now);
                task
            })
            .collect();

        // The tasks are stored first. If removing the member then fails, the
        // member is still there without tasks, which is harmless. In the
        // other order a failure could leave tasks with an assignee that no
        // longer exists.
        self.tasks.save_all(&handed_back)?;
        self.members.delete(id)?;
        Ok(())
    }

    /// Loads a member and the team it is on, changes it and stores it.
    fn change(
        &self,
        id: MemberId,
        apply: impl FnOnce(&mut Member, &[Member], Timestamp) -> Result<(), ApplicationError>,
    ) -> Result<Member, ApplicationError> {
        let mut member = self.require_member(id)?;
        let team = self.members.list_for_project(member.project_id())?;
        apply(&mut member, &team, self.clock.now())?;
        self.members.save(&member)?;
        Ok(member)
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

    /// Loads a member, or reports that it does not exist.
    fn require_member(&self, id: MemberId) -> Result<Member, ApplicationError> {
        self.members
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "team member",
                id: id.to_string(),
            })
    }
}

/// Tidies the model and the effort a member is to have into a profile.
///
/// A model that is empty or only whitespace means none. A person is refused
/// before the name of the model is checked, so that a manager who gives a
/// person a model hears first that a person has none.
fn profile_for(
    kind: MemberKind,
    name: &MemberName,
    model: Option<&str>,
    effort: Option<Effort>,
) -> Result<MemberProfile, ApplicationError> {
    let model = model.map(str::trim).filter(|model| !model.is_empty());
    if kind != MemberKind::Agent && (model.is_some() || effort.is_some()) {
        return Err(ApplicationError::NotASpecialist {
            name: name.to_string(),
        });
    }
    Ok(MemberProfile {
        model: model.map(ModelName::parse).transpose()?,
        effort,
    })
}

/// Reports a name that another member of the team already has. Capital and
/// small letters count as the same, so "Tester" and "tester" cannot both be
/// on one team. `except` is the member that is being renamed, if any.
fn refuse_name_in_use(
    team: &[Member],
    name: &MemberName,
    except: Option<MemberId>,
) -> Result<(), ApplicationError> {
    let wanted = name.as_str().to_lowercase();
    let is_taken = team
        .iter()
        .filter(|member| Some(member.id()) != except)
        .any(|member| member.name().as_str().to_lowercase() == wanted);
    if is_taken {
        return Err(ApplicationError::MemberNameTaken {
            name: name.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, TimeZone};
    use teamwright_domain::DomainError;

    use super::*;
    use crate::in_memory::InMemorySprintRepository;
    use crate::in_memory::{
        InMemoryConversationRepository, InMemoryMemberRepository, InMemoryPlacesRepository,
        InMemoryProjectRepository, InMemoryTaskRepository, ManualClock,
    };
    use crate::projects::{NewProject, ProjectService};
    use crate::tasks::{NewTask, TaskService};

    /// The services under test, sharing storage and a clock that the test
    /// controls.
    struct Workspace {
        projects: ProjectService,
        team: TeamService,
        tasks: TaskService,
        clock: Arc<ManualClock>,
    }

    fn workspace() -> Workspace {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let members = Arc::new(InMemoryMemberRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        let conversations = Arc::new(InMemoryConversationRepository::default());
        let start_of_day = chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap();
        let clock = Arc::new(ManualClock::starting_at(start_of_day));
        Workspace {
            projects: ProjectService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(InMemoryPlacesRepository::default()),
                Arc::new(InMemorySprintRepository::new(tasks.clone())),
                clock.clone(),
            ),
            team: TeamService::new(
                projects.clone(),
                members.clone(),
                tasks.clone(),
                conversations.clone(),
                clock.clone(),
            ),
            tasks: TaskService::new(
                projects,
                tasks.clone(),
                members,
                conversations,
                Arc::new(InMemorySprintRepository::new(tasks.clone())),
                clock.clone(),
            ),
            clock,
        }
    }

    impl Workspace {
        fn project(&self, name: &str) -> Project {
            self.projects
                .create(NewProject {
                    name: name.to_owned(),
                    ..NewProject::default()
                })
                .unwrap()
        }

        /// Puts an agent on the team. A minute passes afterwards, so that
        /// members join at different times.
        fn agent(&self, project: &Project, name: &str) -> Member {
            let member = self.team.add(new_agent(project, name)).unwrap();
            self.clock.advance(TimeDelta::minutes(1));
            member
        }

        fn task_for(&self, project: &Project, title: &str, assignee: Option<Assignee>) -> Task {
            self.tasks
                .create(NewTask {
                    assignee,
                    ..NewTask::titled(project.id(), title)
                })
                .unwrap()
        }

        fn names_on_team(&self, project: &Project) -> Vec<String> {
            self.team
                .list_for_project(project.id())
                .unwrap()
                .iter()
                .map(|member| member.name().to_string())
                .collect()
        }
    }

    fn new_agent(project: &Project, name: &str) -> NewMember {
        NewMember {
            project_id: project.id(),
            name: name.to_owned(),
            kind: MemberKind::Agent,
            responsibility: String::new(),
            model: None,
            effort: None,
        }
    }

    #[test]
    fn a_member_joins_the_team_with_everything_that_was_given_for_it() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let member = workspace
            .team
            .add(NewMember {
                project_id: project.id(),
                name: "  Hardware engineer ".to_owned(),
                kind: MemberKind::Agent,
                responsibility: " The schematic, the parts and the board. \n".to_owned(),
                model: None,
                effort: None,
            })
            .unwrap();

        let stored = workspace.team.get(member.id()).unwrap();
        assert_eq!(stored, member);
        assert_eq!(stored.project_id(), project.id());
        assert_eq!(stored.name().as_str(), "Hardware engineer");
        assert_eq!(stored.kind(), MemberKind::Agent);
        assert_eq!(
            stored.responsibility(),
            "The schematic, the parts and the board."
        );
        assert_eq!(stored.joined_at(), workspace.clock.now());
    }

    #[test]
    fn a_person_can_be_on_a_team_next_to_agents() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        workspace.agent(&project, "Hardware engineer");

        let person = workspace
            .team
            .add(NewMember {
                kind: MemberKind::Human,
                ..new_agent(&project, "Dana")
            })
            .unwrap();

        assert_eq!(person.kind(), MemberKind::Human);
        assert_eq!(
            workspace.names_on_team(&project),
            ["Hardware engineer", "Dana"]
        );
    }

    #[test]
    fn the_team_is_listed_in_the_order_its_members_joined() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        workspace.agent(&project, "Hardware engineer");
        workspace.agent(&project, "Firmware engineer");
        workspace.agent(&project, "Mechanical engineer");

        assert_eq!(
            workspace.names_on_team(&project),
            [
                "Hardware engineer",
                "Firmware engineer",
                "Mechanical engineer"
            ]
        );
    }

    #[test]
    fn every_newcomer_gets_a_color_that_nobody_on_the_team_has() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let colors: Vec<Color> = ["Hardware engineer", "Firmware engineer", "Tester"]
            .iter()
            .map(|name| workspace.agent(&project, name).color())
            .collect();

        assert_eq!(colors, &Member::COLORS[..3]);
    }

    #[test]
    fn the_color_of_a_member_who_left_is_handed_out_again() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let first = workspace.agent(&project, "Hardware engineer");
        workspace.agent(&project, "Firmware engineer");

        workspace.team.remove(first.id()).unwrap();
        let newcomer = workspace.agent(&project, "Tester");

        assert_eq!(newcomer.color(), first.color());
    }

    #[test]
    fn each_project_has_a_team_of_its_own() {
        let workspace = workspace();
        let sensor = workspace.project("Smart sensor node");
        let film = workspace.project("Product film");

        workspace.agent(&sensor, "Hardware engineer");
        workspace.agent(&film, "Video editor");

        assert_eq!(workspace.names_on_team(&sensor), ["Hardware engineer"]);
        assert_eq!(workspace.names_on_team(&film), ["Video editor"]);
    }

    #[test]
    fn a_member_without_a_name_is_refused() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let error = workspace.team.add(new_agent(&project, "   ")).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::Rule(DomainError::EmptyText {
                field: "name of the team member"
            })
        ));
    }

    #[test]
    fn a_member_for_a_project_that_does_not_exist_is_refused() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let missing = NewMember {
            project_id: ProjectId::generate(),
            ..new_agent(&project, "Hardware engineer")
        };

        let error = workspace.team.add(missing).unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "project",
                ..
            }
        ));
    }

    #[test]
    fn two_members_of_one_team_cannot_have_the_same_name() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        workspace.agent(&project, "Tester");

        let error = workspace
            .team
            .add(new_agent(&project, "tester"))
            .unwrap_err();

        assert_eq!(error.code(), "member_name_taken");
        assert_eq!(
            error.to_string(),
            "the team already has a member called \"tester\""
        );
    }

    #[test]
    fn the_same_name_can_be_used_on_the_teams_of_two_projects() {
        let workspace = workspace();
        let sensor = workspace.project("Smart sensor node");
        let thermostat = workspace.project("Thermostat");
        workspace.agent(&sensor, "Hardware engineer");

        let second = workspace
            .team
            .add(new_agent(&thermostat, "Hardware engineer"));

        assert!(second.is_ok());
    }

    #[test]
    fn a_member_can_be_renamed_and_given_another_responsibility() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let member = workspace.agent(&project, "Hardware engineer");
        let later = workspace.clock.now();

        workspace
            .team
            .rename(member.id(), "Electronics engineer")
            .unwrap();
        workspace
            .team
            .set_responsibility(member.id(), "  Everything on the board. ")
            .unwrap();

        let stored = workspace.team.get(member.id()).unwrap();
        assert_eq!(stored.name().as_str(), "Electronics engineer");
        assert_eq!(stored.responsibility(), "Everything on the board.");
        assert_eq!(stored.updated_at(), later);
        assert_eq!(stored.joined_at(), member.joined_at());
        assert_eq!(stored.color(), member.color());
    }

    #[test]
    fn a_member_cannot_take_the_name_of_another_member() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        workspace.agent(&project, "Hardware engineer");
        let tester = workspace.agent(&project, "Tester");

        let error = workspace
            .team
            .rename(tester.id(), "HARDWARE ENGINEER")
            .unwrap_err();

        assert_eq!(error.code(), "member_name_taken");
        assert_eq!(
            workspace.team.get(tester.id()).unwrap().name().as_str(),
            "Tester"
        );
    }

    #[test]
    fn a_member_can_change_how_its_own_name_is_written() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let tester = workspace.agent(&project, "tester");

        let renamed = workspace.team.rename(tester.id(), "Tester").unwrap();

        assert_eq!(renamed.name().as_str(), "Tester");
    }

    #[test]
    fn leaves_the_model_and_the_effort_of_a_new_specialist_to_the_program() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let member = workspace.agent(&project, "Hardware engineer");

        assert_eq!(member.model(), None);
        assert_eq!(member.effort(), None);
    }

    #[test]
    fn puts_a_specialist_on_the_team_with_a_model_and_an_effort() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");

        let member = workspace
            .team
            .add(NewMember {
                model: Some(" haiku ".to_owned()),
                effort: Some(Effort::Low),
                ..new_agent(&project, "Tester")
            })
            .unwrap();

        assert_eq!(member.model().map(ModelName::as_str), Some("haiku"));
        assert_eq!(member.effort(), Some(Effort::Low));
        assert_eq!(workspace.team.get(member.id()).unwrap(), member);
    }

    #[test]
    fn gives_a_specialist_another_model_and_effort_and_takes_them_away_again() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let member = workspace.agent(&project, "Hardware engineer");

        let profiled = workspace
            .team
            .set_profile(member.id(), Some("opus"), Some(Effort::High))
            .unwrap();
        assert_eq!(profiled.model().map(ModelName::as_str), Some("opus"));
        assert_eq!(profiled.effort(), Some(Effort::High));
        assert_eq!(profiled.updated_at(), workspace.clock.now());
        assert_eq!(workspace.team.get(member.id()).unwrap(), profiled);

        let left = workspace
            .team
            .set_profile(member.id(), Some("  "), None)
            .unwrap();
        assert_eq!(left.model(), None);
        assert_eq!(left.effort(), None);
    }

    #[test]
    fn takes_the_full_name_of_a_model_as_it_is_and_refuses_one_with_a_space_in_it() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let member = workspace.agent(&project, "Hardware engineer");

        let named = workspace
            .team
            .set_profile(member.id(), Some("claude-sonnet-5-5"), None)
            .unwrap();
        assert_eq!(
            named.model().map(ModelName::as_str),
            Some("claude-sonnet-5-5")
        );

        let error = workspace
            .team
            .set_profile(member.id(), Some("the best one"), None)
            .unwrap_err();
        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the name of a model is one word of at most 80 characters"
        );
    }

    #[test]
    fn gives_a_person_neither_a_model_nor_an_effort() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let dana = workspace
            .team
            .add(NewMember {
                kind: MemberKind::Human,
                ..new_agent(&project, "Dana")
            })
            .unwrap();

        let with_model = workspace
            .team
            .set_profile(dana.id(), Some("sonnet"), None)
            .unwrap_err();
        assert_eq!(with_model.code(), "no_specialist");
        assert_eq!(with_model.to_string(), "Dana is not an AI specialist");
        let from_the_start = workspace
            .team
            .add(NewMember {
                kind: MemberKind::Human,
                effort: Some(Effort::High),
                ..new_agent(&project, "Eli")
            })
            .unwrap_err();
        assert_eq!(from_the_start.code(), "no_specialist");
        // An empty profile is what a person has, so setting it changes
        // nothing and is allowed.
        let unchanged = workspace.team.set_profile(dana.id(), None, None).unwrap();
        assert_eq!(unchanged.model(), None);
    }

    #[test]
    fn removing_a_member_gives_its_tasks_to_nobody_and_leaves_the_others_alone() {
        let workspace = workspace();
        let project = workspace.project("Smart sensor node");
        let hardware = workspace.agent(&project, "Hardware engineer");
        let firmware = workspace.agent(&project, "Firmware engineer");
        let of_hardware = Some(Assignee::Member(hardware.id()));
        let of_firmware = Some(Assignee::Member(firmware.id()));
        let schematic = workspace.task_for(&project, "Draw the schematic", of_hardware);
        let drivers = workspace.task_for(&project, "Write the drivers", of_firmware);
        let order = workspace.task_for(&project, "Order the parts", Some(Assignee::Manager));
        let later = workspace.clock.now();

        workspace.team.remove(hardware.id()).unwrap();

        let schematic = workspace.tasks.get(schematic.id()).unwrap();
        assert_eq!(schematic.assignee(), None);
        assert_eq!(schematic.updated_at(), later);
        assert_eq!(
            workspace.tasks.get(drivers.id()).unwrap().assignee(),
            of_firmware
        );
        assert_eq!(
            workspace.tasks.get(order.id()).unwrap().assignee(),
            Some(Assignee::Manager)
        );
        assert_eq!(workspace.names_on_team(&project), ["Firmware engineer"]);
    }

    #[test]
    fn changing_or_removing_a_member_that_does_not_exist_is_reported() {
        let workspace = workspace();
        let nobody = MemberId::generate();

        for error in [
            workspace.team.get(nobody).unwrap_err(),
            workspace.team.rename(nobody, "Tester").unwrap_err(),
            workspace.team.set_responsibility(nobody, "").unwrap_err(),
            workspace.team.set_profile(nobody, None, None).unwrap_err(),
            workspace.team.remove(nobody).unwrap_err(),
        ] {
            assert!(matches!(
                error,
                ApplicationError::NotFound {
                    kind: "team member",
                    ..
                }
            ));
        }
    }

    #[test]
    fn listing_the_team_of_a_project_that_does_not_exist_is_reported() {
        let workspace = workspace();

        let error = workspace
            .team
            .list_for_project(ProjectId::generate())
            .unwrap_err();

        assert!(matches!(
            error,
            ApplicationError::NotFound {
                kind: "project",
                ..
            }
        ));
    }
}
