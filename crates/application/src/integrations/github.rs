//! GitHub, through the manager's own command line program `gh`: which pull
//! requests and issues are about which tasks.
//!
//! Teamwright keeps no token. It asks the program the manager signed in to,
//! the way the specialists run on the manager's own agent program. Nothing
//! is read in the background: a reading is kept in memory with its age,
//! and the manager asks for a new one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use teamwright_domain::{Project, ProjectId, RepositoryName, Task, TaskId, Timestamp};

use super::IntegrationStore;
use crate::error::ApplicationError;
use crate::places::PlacesService;
use crate::ports::{Clock, ProjectRepository, TaskRepository};
use crate::work::reference_of;

/// Where a pull request or an issue stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemState {
    /// Open, and ready to be looked at.
    Open,
    /// A pull request that is open as a draft.
    Draft,
    /// A pull request whose changes were merged.
    Merged,
    /// Closed without being merged, or an issue that was closed.
    Closed,
}

/// Whether an item is a pull request or an issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A pull request.
    PullRequest,
    /// An issue.
    Issue,
}

/// A pull request or an issue of a repository, as `gh` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubItem {
    /// A pull request or an issue.
    pub kind: ItemKind,
    /// Its number in the repository.
    pub number: u64,
    /// Its title.
    pub title: String,
    /// Its body, which is searched for keys of pull requests only.
    pub body: String,
    /// The name of the branch of a pull request; empty for an issue.
    pub branch: String,
    /// Where it stands.
    pub state: ItemState,
    /// Its page on GitHub.
    pub url: String,
}

impl GitHubItem {
    /// Whether the item is about a task with the given reference, such as
    /// `SH-4`: the reference stands in its title or in the name of its
    /// branch, and for a pull request also in its body.
    #[must_use]
    pub fn is_about(&self, reference: &str) -> bool {
        mentions(&self.title, reference)
            || mentions(&self.branch, reference)
            || (self.kind == ItemKind::PullRequest && mentions(&self.body, reference))
    }
}

/// Whether a text names a task reference, such as `SH-4`, as a whole: `SH-4`
/// is not in `SH-41` nor in `XSH-4`. Case does not matter, because names of
/// branches are often written small (`sh-4-schematic`).
#[must_use]
pub fn mentions(text: &str, reference: &str) -> bool {
    if reference.is_empty() {
        return false;
    }
    let text = text.to_ascii_lowercase();
    let reference = reference.to_ascii_lowercase();
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(found) = text[start..].find(&reference) {
        let at = start + found;
        let end = at + reference.len();
        let before_is_free = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        let after_is_free = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_is_free && after_is_free {
            return true;
        }
        start = at + 1;
        while !text.is_char_boundary(start) {
            start += 1;
        }
    }
    false
}

/// Whether `gh` is there and signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitHubSignIn {
    /// The program was found nowhere.
    NotInstalled,
    /// The program is there, and not signed in.
    NotSignedIn {
        /// Where it is.
        program: PathBuf,
    },
    /// The program is there and signed in.
    SignedIn {
        /// Where it is.
        program: PathBuf,
        /// The account it is signed in as, if it said.
        account: Option<String>,
    },
}

/// The manager's GitHub command line program, as the use cases need it.
/// Every call has a time limit; a failure is said in plain words, and never
/// repeats a token.
pub trait GitHubProgram: Send + Sync {
    /// Finds the program, the one the manager named first, and asks it
    /// whether it is signed in.
    fn sign_in(&self, named: Option<&Path>) -> GitHubSignIn;

    /// The address of the remote `origin` of a git folder, if it is one.
    fn remote_of(&self, folder: &Path) -> Option<String>;

    /// Checks that the repository exists and the manager may see it.
    ///
    /// # Errors
    ///
    /// Why it could not be seen, in plain words.
    fn check_repository(&self, program: &Path, repository: &RepositoryName) -> Result<(), String>;

    /// The pull requests of a repository, open and closed, newest first.
    ///
    /// # Errors
    ///
    /// Why they could not be read, in plain words.
    fn pull_requests(
        &self,
        program: &Path,
        repository: &RepositoryName,
    ) -> Result<Vec<GitHubItem>, String>;

    /// The issues of a repository, open and closed, newest first.
    ///
    /// # Errors
    ///
    /// Why they could not be read, in plain words.
    fn issues(
        &self,
        program: &Path,
        repository: &RepositoryName,
    ) -> Result<Vec<GitHubItem>, String>;

    /// Opens an issue and answers with it.
    ///
    /// # Errors
    ///
    /// Why it could not be opened, in plain words.
    fn create_issue(
        &self,
        program: &Path,
        repository: &RepositoryName,
        title: &str,
        body: &str,
    ) -> Result<GitHubItem, String>;
}

/// The pull requests and issues of a repository as they were read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubReading {
    /// The repository.
    pub repository: RepositoryName,
    /// When it was read.
    pub read_at: Timestamp,
    /// The pull requests, then the issues.
    pub items: Vec<GitHubItem>,
}

/// GitHub as the integrations page shows it for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubOverview {
    /// Whether `gh` is there and signed in.
    pub sign_in: GitHubSignIn,
    /// The repository of the project, if it has one.
    pub repository: Option<RepositoryName>,
    /// Whether the repository was read from the work folder rather than
    /// named by the manager.
    pub is_from_work_folder: bool,
    /// When the repository was last read, if it was.
    pub read_at: Option<Timestamp>,
    /// How many pull requests are about a task of the project.
    pub linked_pull_requests: usize,
    /// How many issues are about a task of the project.
    pub linked_issues: usize,
    /// The pull requests and issues that are about no task, at most ten.
    pub unlinked: Vec<GitHubItem>,
    /// How many are about no task in all.
    pub unlinked_count: usize,
}

/// The most items about no task the overview lists.
const MOST_UNLINKED_SHOWN: usize = 10;

/// The pull requests and issues about one task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOnGitHub {
    /// The repository, if the project has one.
    pub repository: Option<RepositoryName>,
    /// When it was read, if it was.
    pub read_at: Option<Timestamp>,
    /// The items about the task: pull requests, then issues.
    pub items: Vec<GitHubItem>,
}

/// What the GitHub use cases need.
#[derive(Clone)]
pub struct GitHubPorts {
    /// The projects.
    pub projects: Arc<dyn ProjectRepository>,
    /// The tasks.
    pub tasks: Arc<dyn TaskRepository>,
    /// The folders of the projects, for the work folder.
    pub places: PlacesService,
    /// The settings of the integrations.
    pub store: Arc<dyn IntegrationStore>,
    /// The manager's `gh`.
    pub program: Arc<dyn GitHubProgram>,
    /// The time.
    pub clock: Arc<dyn Clock>,
}

/// The use cases of GitHub.
#[derive(Clone)]
pub struct GitHubService {
    ports: GitHubPorts,
    readings: Arc<Mutex<HashMap<ProjectId, GitHubReading>>>,
}

impl GitHubService {
    /// Creates the use cases, with nothing read yet.
    #[must_use]
    pub fn new(ports: GitHubPorts) -> Self {
        Self {
            ports,
            readings: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Whether `gh` is there and signed in.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn sign_in(&self) -> Result<GitHubSignIn, ApplicationError> {
        let named = self.ports.store.github_program()?;
        Ok(self.ports.program.sign_in(named.as_deref()))
    }

    /// GitHub for a project, from what was last read. Nothing is read.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, or if storage fails.
    pub fn overview(&self, project_id: ProjectId) -> Result<GitHubOverview, ApplicationError> {
        let project = self.project(project_id)?;
        let (repository, is_from_work_folder) = self.repository_of(&project)?;
        let reading = self
            .reading_of(project_id)
            .filter(|reading| Some(&reading.repository) == repository.as_ref());
        let references: Vec<String> = self
            .ports
            .tasks
            .list_for_project(project_id)?
            .iter()
            .map(|task| reference_of(&project, task))
            .collect();
        let items = reading
            .as_ref()
            .map_or(&[][..], |reading| &reading.items[..]);
        let is_linked =
            |item: &GitHubItem| references.iter().any(|reference| item.is_about(reference));
        let count = |kind: ItemKind| {
            items
                .iter()
                .filter(|item| item.kind == kind && is_linked(item))
                .count()
        };
        let unlinked: Vec<GitHubItem> = items
            .iter()
            .filter(|item| !is_linked(item))
            .cloned()
            .collect();
        Ok(GitHubOverview {
            sign_in: self.sign_in()?,
            repository,
            is_from_work_folder,
            read_at: reading.as_ref().map(|reading| reading.read_at),
            linked_pull_requests: count(ItemKind::PullRequest),
            linked_issues: count(ItemKind::Issue),
            unlinked_count: unlinked.len(),
            unlinked: unlinked.into_iter().take(MOST_UNLINKED_SHOWN).collect(),
        })
    }

    /// Reads the pull requests and issues of the project's repository
    /// again, and answers with the overview.
    ///
    /// # Errors
    ///
    /// [`ApplicationError::NoRepository`] without a repository;
    /// [`ApplicationError::GitHubUnavailable`] if `gh` is missing, not
    /// signed in, or failed; an error if there is no such project, or if
    /// storage fails.
    pub fn refresh(&self, project_id: ProjectId) -> Result<GitHubOverview, ApplicationError> {
        let project = self.project(project_id)?;
        self.read(&project)?;
        self.overview(project_id)
    }

    /// The pull requests and issues about a task. The repository is read
    /// once if it has not been read since the program started.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, if reading fails, or if
    /// storage fails. A project without a repository is no error: the
    /// answer has none.
    pub fn of_task(&self, task_id: TaskId) -> Result<TaskOnGitHub, ApplicationError> {
        let task = self.task(task_id)?;
        let project = self.project(task.project_id())?;
        let (repository, _) = self.repository_of(&project)?;
        let Some(repository) = repository else {
            return Ok(TaskOnGitHub {
                repository: None,
                read_at: None,
                items: Vec::new(),
            });
        };
        let reading = match self
            .reading_of(project.id())
            .filter(|reading| reading.repository == repository)
        {
            Some(reading) => reading,
            None => self.read(&project)?,
        };
        let reference = reference_of(&project, &task);
        Ok(TaskOnGitHub {
            repository: Some(repository),
            read_at: Some(reading.read_at),
            items: reading
                .items
                .into_iter()
                .filter(|item| item.is_about(&reference))
                .collect(),
        })
    }

    /// What an issue opened for a task says: its title, with the task's
    /// reference first, and its body. Nothing is sent.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such task, or if storage fails.
    pub fn issue_for_task(&self, task_id: TaskId) -> Result<(String, String), ApplicationError> {
        let task = self.task(task_id)?;
        let project = self.project(task.project_id())?;
        Ok(issue_of(&project, &task))
    }

    /// Opens an issue for a task in the project's repository, and keeps it
    /// with what was read.
    ///
    /// # Errors
    ///
    /// [`ApplicationError::NoRepository`] without a repository;
    /// [`ApplicationError::GitHubUnavailable`] if `gh` is missing, not
    /// signed in, or failed; an error if there is no such task, or if
    /// storage fails.
    pub fn open_issue(&self, task_id: TaskId) -> Result<GitHubItem, ApplicationError> {
        let task = self.task(task_id)?;
        let project = self.project(task.project_id())?;
        let (repository, _) = self.repository_of(&project)?;
        let repository = repository.ok_or(ApplicationError::NoRepository)?;
        let program = self.signed_in_program()?;
        let (title, body) = issue_of(&project, &task);
        let issue = self
            .ports
            .program
            .create_issue(&program, &repository, &title, &body)
            .map_err(|reason| ApplicationError::GitHubUnavailable { reason })?;
        let mut readings = self.readings.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(reading) = readings
            .get_mut(&project.id())
            .filter(|reading| reading.repository == repository)
        {
            reading.items.push(issue.clone());
        }
        Ok(issue)
    }

    fn read(&self, project: &Project) -> Result<GitHubReading, ApplicationError> {
        let (repository, _) = self.repository_of(project)?;
        let repository = repository.ok_or(ApplicationError::NoRepository)?;
        let program = self.signed_in_program()?;
        let unavailable = |reason| ApplicationError::GitHubUnavailable { reason };
        self.ports
            .program
            .check_repository(&program, &repository)
            .map_err(unavailable)?;
        let mut items = self
            .ports
            .program
            .pull_requests(&program, &repository)
            .map_err(unavailable)?;
        items.extend(
            self.ports
                .program
                .issues(&program, &repository)
                .map_err(unavailable)?,
        );
        let reading = GitHubReading {
            repository,
            read_at: self.ports.clock.now(),
            items,
        };
        self.readings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(project.id(), reading.clone());
        Ok(reading)
    }

    fn signed_in_program(&self) -> Result<PathBuf, ApplicationError> {
        match self.sign_in()? {
            GitHubSignIn::SignedIn { program, .. } => Ok(program),
            other => Err(ApplicationError::GitHubUnavailable {
                reason: sign_in_problem(&other).unwrap_or_default().to_owned(),
            }),
        }
    }

    /// The repository of a project: the one the manager named, or the one
    /// the work folder is a clone of. The second answer says which.
    fn repository_of(
        &self,
        project: &Project,
    ) -> Result<(Option<RepositoryName>, bool), ApplicationError> {
        if let Some(named) = self.ports.store.integrations(project.id())?.repository {
            return Ok((Some(named), false));
        }
        // A work folder that cannot be made or read has no repository.
        let from_folder = self
            .ports
            .places
            .work_folder_of(project)
            .ok()
            .and_then(|folder| self.ports.program.remote_of(&folder))
            .and_then(|remote| RepositoryName::of_remote(&remote));
        let is_from_folder = from_folder.is_some();
        Ok((from_folder, is_from_folder))
    }

    fn reading_of(&self, project_id: ProjectId) -> Option<GitHubReading> {
        self.readings
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&project_id)
            .cloned()
    }

    fn project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.ports
            .projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }

    fn task(&self, id: TaskId) -> Result<Task, ApplicationError> {
        self.ports
            .tasks
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "task",
                id: id.to_string(),
            })
    }
}

/// What is missing for `gh` to be used, in plain words, or nothing if it is
/// signed in.
#[must_use]
pub const fn sign_in_problem(sign_in: &GitHubSignIn) -> Option<&'static str> {
    match sign_in {
        GitHubSignIn::NotInstalled => Some(
            "GitHub's command line program is not installed: get it from https://cli.github.com",
        ),
        GitHubSignIn::NotSignedIn { .. } => {
            Some("GitHub's command line program is not signed in: run `gh auth login`")
        }
        GitHubSignIn::SignedIn { .. } => None,
    }
}

/// The title and the body of an issue opened for a task.
fn issue_of(project: &Project, task: &Task) -> (String, String) {
    let reference = reference_of(project, task);
    let title = format!("{reference} {}", task.title().as_str());
    let description = task.description().trim();
    let body = if description.is_empty() {
        format!("Opened from Teamwright for {reference}.")
    } else {
        format!("{description}\n\nOpened from Teamwright for {reference}.")
    };
    (title, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: ItemKind, title: &str, branch: &str, body: &str) -> GitHubItem {
        GitHubItem {
            kind,
            number: 1,
            title: title.to_owned(),
            body: body.to_owned(),
            branch: branch.to_owned(),
            state: ItemState::Open,
            url: "https://github.com/octo-org/sensor-node/pull/1".to_owned(),
        }
    }

    #[test]
    fn a_reference_is_found_whole_only() {
        assert!(mentions("SH-4: draw the schematic", "SH-4"));
        assert!(mentions("Fix (SH-4)", "SH-4"));
        assert!(mentions("sh-4-schematic", "SH-4"));
        assert!(mentions("feature/SH-4", "SH-4"));
        assert!(mentions("SH-41 and SH-4", "SH-4"));
        assert!(!mentions("SH-41: route the board", "SH-4"));
        assert!(!mentions("XSH-4", "SH-4"));
        assert!(!mentions("SH-4a", "SH-4"));
        assert!(!mentions("SH 4", "SH-4"));
        assert!(!mentions("anything", ""));
        assert!(mentions("Über SH-4 ändern", "SH-4"));
    }

    #[test]
    fn a_pull_request_is_about_a_task_by_its_title_branch_or_body_and_an_issue_not_by_its_body() {
        let reference = "SH-4";
        assert!(item(ItemKind::PullRequest, "Schematic", "sh-4-schematic", "").is_about(reference));
        assert!(
            item(ItemKind::PullRequest, "Schematic", "main", "Closes SH-4").is_about(reference)
        );
        assert!(item(ItemKind::Issue, "SH-4 Schematic", "", "").is_about(reference));
        assert!(!item(ItemKind::Issue, "Schematic", "", "About SH-4").is_about(reference));
    }

    use teamwright_domain::{
        AreaId, ProjectKey, ProjectName, Rank, TaskDraft, TaskTitle, Workflow,
    };

    use crate::SystemClock;
    use crate::in_memory::{
        FixedProjectFolders, InMemoryConversationRepository, InMemoryMemberRepository,
        InMemoryPlacesRepository, InMemoryProjectRepository, InMemoryTaskRepository,
        PretendFileSystem,
    };
    use crate::integrations::tests::MemoryStore;
    use crate::integrations::{IntegrationSettings, ProjectIntegrations};

    /// A `gh` that is signed in and knows a few pull requests and issues.
    struct FakeGh {
        created: Mutex<Vec<(String, String)>>,
        reads: Mutex<usize>,
    }

    impl GitHubProgram for FakeGh {
        fn sign_in(&self, _named: Option<&Path>) -> GitHubSignIn {
            GitHubSignIn::SignedIn {
                program: PathBuf::from("gh"),
                account: Some("octo-cat".to_owned()),
            }
        }

        fn remote_of(&self, _folder: &Path) -> Option<String> {
            Some("git@github.com:octo-org/sensor-node.git".to_owned())
        }

        fn check_repository(&self, _: &Path, _: &RepositoryName) -> Result<(), String> {
            Ok(())
        }

        fn pull_requests(&self, _: &Path, _: &RepositoryName) -> Result<Vec<GitHubItem>, String> {
            *self.reads.lock().unwrap() += 1;
            Ok(vec![
                item(ItemKind::PullRequest, "Schematic", "sh-1-schematic", ""),
                item(ItemKind::PullRequest, "Tidy the readme", "readme", ""),
            ])
        }

        fn issues(&self, _: &Path, _: &RepositoryName) -> Result<Vec<GitHubItem>, String> {
            Ok(vec![item(ItemKind::Issue, "SH-1 is wrong", "", "")])
        }

        fn create_issue(
            &self,
            _: &Path,
            _: &RepositoryName,
            title: &str,
            body: &str,
        ) -> Result<GitHubItem, String> {
            self.created
                .lock()
                .unwrap()
                .push((title.to_owned(), body.to_owned()));
            Ok(item(ItemKind::Issue, title, "", ""))
        }
    }

    fn service() -> (
        GitHubService,
        IntegrationSettings,
        ProjectId,
        TaskId,
        Arc<FakeGh>,
    ) {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        let project = Project::create(
            ProjectName::parse("Smart sensor node").unwrap(),
            ProjectKey::parse("SH").unwrap(),
            String::new(),
            Workflow::standard(),
            chrono::Utc::now(),
        );
        projects.save(&project).unwrap();
        let task = Task::create(
            project.id(),
            AreaId::generate(),
            1,
            project.workflow().initial_status(),
            Rank::between(None, None),
            TaskDraft::titled(TaskTitle::parse("Draw the schematic").unwrap()),
            chrono::Utc::now(),
        );
        tasks.save(&task).unwrap();
        let store = Arc::new(MemoryStore::default());
        let gh = Arc::new(FakeGh {
            created: Mutex::new(Vec::new()),
            reads: Mutex::new(0),
        });
        let places = PlacesService::new(
            projects.clone(),
            tasks.clone(),
            Arc::new(InMemoryMemberRepository::default()),
            Arc::new(InMemoryConversationRepository::default()),
            Arc::new(InMemoryPlacesRepository::default()),
            Arc::new(FixedProjectFolders::inside("C:\\Projects")),
            Arc::new(PretendFileSystem::default()),
        );
        let service = GitHubService::new(GitHubPorts {
            projects: projects.clone(),
            tasks,
            places,
            store: store.clone(),
            program: gh.clone(),
            clock: Arc::new(SystemClock),
        });
        (
            service,
            IntegrationSettings::new(projects, store),
            project.id(),
            task.id(),
            gh,
        )
    }

    #[test]
    fn the_repository_comes_from_the_work_folder_unless_one_is_named() {
        let (service, settings, project, _, _) = service();
        let overview = service.overview(project).unwrap();
        assert_eq!(
            overview.repository.unwrap().to_string(),
            "octo-org/sensor-node"
        );
        assert!(overview.is_from_work_folder);
        assert_eq!(overview.read_at, None);

        settings
            .set_repository(project, Some("octo-org/firmware"))
            .unwrap();
        let named = service.overview(project).unwrap();
        assert_eq!(named.repository.unwrap().to_string(), "octo-org/firmware");
        assert!(!named.is_from_work_folder);
        assert_eq!(
            settings.of(project).unwrap(),
            ProjectIntegrations {
                repository: RepositoryName::parse("octo-org/firmware").ok(),
                ..ProjectIntegrations::default()
            }
        );
    }

    #[test]
    fn a_reading_counts_what_is_about_tasks_and_lists_the_rest() {
        let (service, _, project, task, gh) = service();
        let overview = service.refresh(project).unwrap();
        assert!(overview.read_at.is_some());
        assert_eq!(overview.linked_pull_requests, 1);
        assert_eq!(overview.linked_issues, 1);
        assert_eq!(overview.unlinked_count, 1);
        assert_eq!(overview.unlinked[0].title, "Tidy the readme");

        // The task finds its items in what was read, without reading again.
        let on_github = service.of_task(task).unwrap();
        assert_eq!(on_github.items.len(), 2);
        assert_eq!(*gh.reads.lock().unwrap(), 1);
    }

    #[test]
    fn an_issue_for_a_task_carries_its_reference_and_is_kept() {
        let (service, _, project, task, gh) = service();
        service.refresh(project).unwrap();
        let (title, body) = service.issue_for_task(task).unwrap();
        assert_eq!(title, "SH-1 Draw the schematic");
        assert_eq!(body, "Opened from Teamwright for SH-1.");
        service.open_issue(task).unwrap();
        assert_eq!(gh.created.lock().unwrap().len(), 1);
        assert_eq!(service.of_task(task).unwrap().items.len(), 3);
    }

    #[test]
    fn what_is_missing_is_said_plainly() {
        assert!(
            sign_in_problem(&GitHubSignIn::NotInstalled)
                .unwrap()
                .contains("cli.github.com")
        );
        assert!(
            sign_in_problem(&GitHubSignIn::NotSignedIn {
                program: PathBuf::from("gh")
            })
            .unwrap()
            .contains("gh auth login")
        );
        assert_eq!(
            sign_in_problem(&GitHubSignIn::SignedIn {
                program: PathBuf::from("gh"),
                account: None
            }),
            None
        );
    }
}
