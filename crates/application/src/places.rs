//! The folders of a project: the work folder the specialists read and write
//! in, the reference folders they may only read, and the single files the
//! manager hands them, which are copied into the work folder
//! ([decision 0013](../../../docs/decisions/0013-the-folders-of-a-project.md)).
//!
//! The folders are facts about one computer. Teamwright never removes a file
//! or a folder of the manager: forgetting a folder leaves it where it is.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use teamwright_domain::{Project, ProjectId};

use crate::error::ApplicationError;
use crate::ports::{
    ConversationRepository, FileSystem, MemberRepository, PlacesRepository, ProjectFolders,
    ProjectRepository, TaskRepository,
};
use crate::work::WorkInProgress;

/// The folder inside the work folder that the files handed over by the
/// manager are copied into.
pub const REFERENCES_FOLDER: &str = "references";

/// The folders of a project, as the manager sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPlaces {
    /// The folder the specialists work in: the one the manager chose, or
    /// the one Teamwright gives the project.
    pub work_folder: PathBuf,
    /// Whether the manager chose the work folder.
    pub is_work_folder_chosen: bool,
    /// The folders the specialists may read and must not change, in the
    /// order they were added.
    pub reference_folders: Vec<PathBuf>,
}

/// What the manager has chosen for the folders of a project, as it is
/// stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChosenPlaces {
    /// The work folder the manager chose, or `None` for the one Teamwright
    /// gives the project.
    pub work_folder: Option<PathBuf>,
    /// The reference folders, in the order they were added, as the manager
    /// named them.
    pub reference_folders: Vec<PathBuf>,
}

/// The use cases of the folders of a project.
#[derive(Clone)]
pub struct PlacesService {
    projects: Arc<dyn ProjectRepository>,
    tasks: Arc<dyn TaskRepository>,
    members: Arc<dyn MemberRepository>,
    conversations: Arc<dyn ConversationRepository>,
    places: Arc<dyn PlacesRepository>,
    folders: Arc<dyn ProjectFolders>,
    file_system: Arc<dyn FileSystem>,
}

impl PlacesService {
    /// Creates the service. The tasks, the members and the conversations
    /// are needed because the work folder does not change while a
    /// specialist works in the project, and the error says who works on
    /// what.
    #[must_use]
    pub fn new(
        projects: Arc<dyn ProjectRepository>,
        tasks: Arc<dyn TaskRepository>,
        members: Arc<dyn MemberRepository>,
        conversations: Arc<dyn ConversationRepository>,
        places: Arc<dyn PlacesRepository>,
        folders: Arc<dyn ProjectFolders>,
        file_system: Arc<dyn FileSystem>,
    ) -> Self {
        Self {
            projects,
            tasks,
            members,
            conversations,
            places,
            folders,
            file_system,
        }
    }

    /// The folders of a project.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the folder
    /// Teamwright gives it cannot be made, or if storage fails.
    pub fn places(&self, project_id: ProjectId) -> Result<ProjectPlaces, ApplicationError> {
        let project = self.require_project(project_id)?;
        let chosen = self.places.find(project.id())?;
        self.places_of(&project, chosen)
    }

    /// Makes a folder the work folder of a project, or, with `None`, goes
    /// back to the folder Teamwright gives it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if a specialist works
    /// in it, if the folder is not named or does not exist, if it gets in
    /// the way of a reference folder, or if storage fails.
    pub fn set_work_folder(
        &self,
        project_id: ProjectId,
        folder: Option<&Path>,
    ) -> Result<ProjectPlaces, ApplicationError> {
        let project = self.require_project(project_id)?;
        self.refuse_while_worked_on(&project)?;
        let mut chosen = self.places.find(project.id())?;
        chosen.work_folder = match folder {
            None => None,
            Some(folder) => {
                let work_folder = self.require_folder(folder)?;
                for reference in &chosen.reference_folders {
                    let reference = Folder {
                        given: reference.clone(),
                        resolved: self.comparable(reference),
                    };
                    refuse_folders_that_overlap(&work_folder, &reference)?;
                }
                Some(work_folder.given)
            }
        };
        self.places.save(project.id(), &chosen)?;
        self.places_of(&project, chosen)
    }

    /// Lets the specialists of a project read a folder. A folder the
    /// project has already is kept once.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the folder is not
    /// named or does not exist, if it gets in the way of the work folder,
    /// or if storage fails.
    pub fn add_reference_folder(
        &self,
        project_id: ProjectId,
        folder: &Path,
    ) -> Result<ProjectPlaces, ApplicationError> {
        let project = self.require_project(project_id)?;
        let mut chosen = self.places.find(project.id())?;
        let reference = self.require_folder(folder)?;
        let work_folder = self.work_folder_in(&project, &chosen)?;
        let work_folder = Folder {
            resolved: self.comparable(&work_folder),
            given: work_folder,
        };
        refuse_folders_that_overlap(&work_folder, &reference)?;
        let is_known = chosen
            .reference_folders
            .iter()
            .any(|other| self.comparable(other) == reference.resolved);
        if !is_known {
            chosen.reference_folders.push(reference.given);
            self.places.save(project.id(), &chosen)?;
        }
        self.places_of(&project, chosen)
    }

    /// Takes a reference folder away from the specialists of a project. A
    /// folder the project does not have changes nothing. The folder itself
    /// is not touched.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or if storage fails.
    pub fn remove_reference_folder(
        &self,
        project_id: ProjectId,
        folder: &Path,
    ) -> Result<ProjectPlaces, ApplicationError> {
        let project = self.require_project(project_id)?;
        let mut chosen = self.places.find(project.id())?;
        let folder = tidy(folder);
        if !folder.as_os_str().is_empty() {
            // A folder that is gone from the disk can still be taken away:
            // it is then compared as it was named.
            let removed = self.comparable(&folder);
            let count_before = chosen.reference_folders.len();
            chosen
                .reference_folders
                .retain(|other| self.comparable(other) != removed);
            if chosen.reference_folders.len() != count_before {
                self.places.save(project.id(), &chosen)?;
            }
        }
        self.places_of(&project, chosen)
    }

    /// Copies files into the folder `references` inside the work folder of
    /// a project, and answers with the names they got there, in the order
    /// of the files.
    ///
    /// A file keeps its name if it is free. A name that is taken, by a file
    /// there already or by an earlier file of the same call, gets a number
    /// in front of its ending, as in `datasheet (2).pdf`. Capitals do not
    /// make a name different.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if a path has no file
    /// name (then nothing is copied), if a file cannot be read or copied
    /// (then the files before it are copied), or if storage fails.
    pub fn add_reference_files(
        &self,
        project_id: ProjectId,
        files: &[PathBuf],
    ) -> Result<Vec<String>, ApplicationError> {
        let project = self.require_project(project_id)?;
        let files = files
            .iter()
            .map(|file| {
                let file = tidy(file);
                let name = file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .ok_or(ApplicationError::FileWithoutName)?;
                Ok((file, name))
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;

        let folder = self.work_folder_of(&project)?.join(REFERENCES_FOLDER);
        let mut taken = self.file_system.file_names_in(&folder)?;
        let mut names = Vec::with_capacity(files.len());
        for (file, name) in files {
            let name = free_name(&name, &taken);
            self.file_system.copy_file(&file, &folder.join(&name))?;
            taken.push(name.clone());
            names.push(name);
        }
        Ok(names)
    }

    /// The folder the specialists of a project work in: the one the manager
    /// chose, or the one Teamwright gives the project. It is the working
    /// folder of every turn, and the folder the manager is shown.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder Teamwright gives the project cannot be
    /// made, or if storage fails.
    pub fn work_folder_of(&self, project: &Project) -> Result<PathBuf, ApplicationError> {
        let chosen = self.places.find(project.id())?;
        self.work_folder_in(project, &chosen)
    }

    /// The reference folders of a project that exist on the computer now,
    /// in the order they were added, each in its full form.
    ///
    /// The full form matters. The program that runs a specialist compares
    /// paths as they are written. A folder that is named to it in another
    /// way than the specialist names it later, for example with a short
    /// name of Windows, counts as a different folder, and reading it is
    /// refused.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    pub fn existing_reference_folders(
        &self,
        project: &Project,
    ) -> Result<Vec<PathBuf>, ApplicationError> {
        Ok(self
            .places
            .find(project.id())?
            .reference_folders
            .into_iter()
            .filter(|folder| self.file_system.is_folder(folder))
            .filter_map(|folder| self.file_system.resolve(&folder).ok())
            .collect())
    }

    /// The folders of a project as the manager sees them.
    fn places_of(
        &self,
        project: &Project,
        chosen: ChosenPlaces,
    ) -> Result<ProjectPlaces, ApplicationError> {
        Ok(ProjectPlaces {
            work_folder: self.work_folder_in(project, &chosen)?,
            is_work_folder_chosen: chosen.work_folder.is_some(),
            reference_folders: chosen.reference_folders,
        })
    }

    /// The work folder of a project with the given choice.
    fn work_folder_in(
        &self,
        project: &Project,
        chosen: &ChosenPlaces,
    ) -> Result<PathBuf, ApplicationError> {
        match &chosen.work_folder {
            Some(folder) => Ok(folder.clone()),
            None => Ok(self.folders.folder_of(project)?),
        }
    }

    /// Checks that a folder is named and exists, and answers with it as it
    /// was given, trimmed, and in its full form.
    fn require_folder(&self, folder: &Path) -> Result<Folder, ApplicationError> {
        let given = tidy(folder);
        if given.as_os_str().is_empty() {
            return Err(ApplicationError::FolderNotNamed);
        }
        if !self.file_system.is_folder(&given) {
            return Err(ApplicationError::FolderNotFound {
                folder: given.display().to_string(),
            });
        }
        let resolved = self.file_system.resolve(&given)?;
        Ok(Folder { given, resolved })
    }

    /// The form in which a folder is compared with another one: its full
    /// form if it exists, and otherwise the folder as it was named.
    fn comparable(&self, folder: &Path) -> PathBuf {
        self.file_system
            .resolve(folder)
            .unwrap_or_else(|_| folder.to_path_buf())
    }

    /// Refuses to go on while a specialist works in a project.
    fn refuse_while_worked_on(&self, project: &Project) -> Result<(), ApplicationError> {
        let conversations = self.conversations.list_for_project(project.id())?;
        WorkInProgress {
            projects: self.projects.as_ref(),
            tasks: self.tasks.as_ref(),
            members: self.members.as_ref(),
            conversations: self.conversations.as_ref(),
        }
        .refuse_while_worked_on(&conversations)
    }

    /// Loads a project, or reports that there is none.
    fn require_project(&self, id: ProjectId) -> Result<Project, ApplicationError> {
        self.projects
            .find(id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }
}

/// A folder as the manager named it, and in the form in which it is
/// compared.
struct Folder {
    given: PathBuf,
    resolved: PathBuf,
}

/// Refuses a work folder and a reference folder that get in each other's
/// way. The specialists write in the work folder and only read a reference
/// folder, so the one cannot be the other, lie inside it, or contain it.
fn refuse_folders_that_overlap(
    work_folder: &Folder,
    reference_folder: &Folder,
) -> Result<(), ApplicationError> {
    let work_text = || work_folder.given.display().to_string();
    let reference_text = || reference_folder.given.display().to_string();
    // Paths are compared part by part, so `/work` does not contain
    // `/workshop`.
    if reference_folder.resolved == work_folder.resolved {
        return Err(ApplicationError::WorkFolderIsReference {
            work_folder: work_text(),
        });
    }
    if reference_folder.resolved.starts_with(&work_folder.resolved) {
        return Err(ApplicationError::ReferenceInsideWorkFolder {
            reference_folder: reference_text(),
            work_folder: work_text(),
        });
    }
    if work_folder.resolved.starts_with(&reference_folder.resolved) {
        return Err(ApplicationError::WorkFolderInsideReference {
            work_folder: work_text(),
            reference_folder: reference_text(),
        });
    }
    Ok(())
}

/// A path without the spaces the manager may have typed around it.
fn tidy(path: &Path) -> PathBuf {
    path.to_str()
        .map_or_else(|| path.to_path_buf(), |text| PathBuf::from(text.trim()))
}

/// A name for a file that is copied to where the names in `taken` are in
/// use: the name itself if it is free, and otherwise the name with a number
/// in front of its ending, as in `datasheet (2).pdf`. Capitals do not make a
/// name different.
fn free_name(name: &str, taken: &[String]) -> String {
    let is_taken = |candidate: &str| {
        let candidate = candidate.to_lowercase();
        taken.iter().any(|other| other.to_lowercase() == candidate)
    };
    if !is_taken(name) {
        return name.to_owned();
    }
    // A name that starts with its only dot, such as `.gitignore`, has no
    // ending.
    let (stem, ending) = match name.rfind('.') {
        Some(dot) if dot > 0 => name.split_at(dot),
        _ => (name, ""),
    };
    (2_u64..)
        .map(|number| format!("{stem} ({number}){ending}"))
        .find(|candidate| !is_taken(candidate))
        // The numbers run out only after more names than any folder holds.
        .unwrap_or_else(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{Assignee, MemberKind};

    use super::*;
    use crate::in_memory::InMemorySprintRepository;
    use crate::in_memory::{
        FixedProjectFolders, InMemoryConversationRepository, InMemoryMemberRepository,
        InMemoryPlacesRepository, InMemoryPlanningRepository, InMemoryProjectRepository,
        InMemoryTaskRepository, ManualClock, PretendFileSystem, RecordingWorkListener,
        ScriptedRuntime,
    };
    use crate::projects::{NewProject, ProjectService, WorkflowTemplate};
    use crate::tasks::{NewTask, TaskService};
    use crate::team::{NewMember, TeamService};
    use crate::work::{WorkPorts, WorkService};

    /// The folder that holds the folders Teamwright gives the projects.
    const PROJECTS_FOLDER: &str = "/documents/Teamwright";

    /// The folders that exist on the computer these tests pretend to run
    /// on.
    const FOLDERS: [&str; 5] = [
        "/Work",
        "/Work/sensor-node",
        "/Work/sensor-node/docs",
        "/Datasheets",
        "/Datasheets/Bosch",
    ];

    /// The services under test on shared storage, the pretended file
    /// system, and a project "Smart sensor node" with the key SSN.
    struct Workspace {
        projects: ProjectService,
        places: PlacesService,
        tasks: TaskService,
        team: TeamService,
        work: WorkService,
        file_system: Arc<PretendFileSystem>,
        project: Project,
    }

    fn workspace() -> Workspace {
        let projects = Arc::new(InMemoryProjectRepository::default());
        let tasks = Arc::new(InMemoryTaskRepository::default());
        let members = Arc::new(InMemoryMemberRepository::default());
        let conversations = Arc::new(InMemoryConversationRepository::default());
        let places = Arc::new(InMemoryPlacesRepository::default());
        let folders = Arc::new(FixedProjectFolders::inside(PROJECTS_FOLDER));
        let file_system = Arc::new(PretendFileSystem::with_folders(FOLDERS));
        let clock = Arc::new(ManualClock::starting_at(
            chrono::Utc.with_ymd_and_hms(2026, 11, 12, 9, 0, 0).unwrap(),
        ));

        let area_store = Arc::new(crate::in_memory::InMemoryAreaRepository::default());
        let project_service = ProjectService::new(
            projects.clone(),
            tasks.clone(),
            members.clone(),
            conversations.clone(),
            places.clone(),
            Arc::new(InMemorySprintRepository::new(tasks.clone())),
            Arc::new(crate::in_memory::InMemoryViewRepository::default()),
            area_store.clone(),
            clock.clone(),
        );
        let project = project_service
            .create(NewProject {
                name: "Smart sensor node".to_owned(),
                workflow: WorkflowTemplate::Engineering,
                ..NewProject::default()
            })
            .unwrap();
        Workspace {
            places: PlacesService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                places.clone(),
                folders.clone(),
                file_system.clone(),
            ),
            tasks: TaskService::new(
                projects.clone(),
                tasks.clone(),
                members.clone(),
                conversations.clone(),
                Arc::new(InMemorySprintRepository::new(tasks.clone())),
                area_store.clone(),
                clock.clone(),
            ),
            team: TeamService::new(
                projects.clone(),
                members.clone(),
                tasks.clone(),
                conversations.clone(),
                Arc::new(crate::in_memory::InMemoryViewRepository::default()),
                clock.clone(),
            ),
            work: WorkService::new(WorkPorts {
                planning: Arc::new(InMemoryPlanningRepository::new(
                    projects.clone(),
                    members.clone(),
                    tasks.clone(),
                    conversations.clone(),
                )),
                projects,
                tasks,
                areas: area_store,
                members,
                conversations,
                folders,
                places,
                file_system: file_system.clone(),
                runtime: Arc::new(ScriptedRuntime::new()),
                listener: Arc::new(RecordingWorkListener::default()),
                clock,
                this_computer: teamwright_domain::ComputerId::generate(),
            }),
            file_system,
            project,
            projects: project_service,
        }
    }

    /// The folder Teamwright gives the project of the workspace.
    fn given_work_folder() -> PathBuf {
        PathBuf::from(PROJECTS_FOLDER).join("SSN")
    }

    fn path(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    impl Workspace {
        fn set_work_folder(&self, folder: &str) -> Result<ProjectPlaces, ApplicationError> {
            self.places
                .set_work_folder(self.project.id(), Some(Path::new(folder)))
        }

        fn add_reference_folder(&self, folder: &str) -> Result<ProjectPlaces, ApplicationError> {
            self.places
                .add_reference_folder(self.project.id(), Path::new(folder))
        }

        fn current(&self) -> ProjectPlaces {
            self.places.places(self.project.id()).unwrap()
        }
    }

    #[test]
    fn gives_every_project_a_work_folder_of_its_own_named_by_its_key() {
        let workspace = workspace();

        assert_eq!(
            workspace.current(),
            ProjectPlaces {
                work_folder: given_work_folder(),
                is_work_folder_chosen: false,
                reference_folders: Vec::new(),
            }
        );
    }

    #[test]
    fn lets_the_manager_choose_the_work_folder_and_go_back_to_the_one_teamwright_gives() {
        let workspace = workspace();

        let chosen = workspace.set_work_folder("  /Work/sensor-node ").unwrap();
        assert_eq!(chosen.work_folder, path("/Work/sensor-node"));
        assert!(chosen.is_work_folder_chosen);
        assert_eq!(
            workspace.places.work_folder_of(&workspace.project).unwrap(),
            path("/Work/sensor-node")
        );

        let back = workspace
            .places
            .set_work_folder(workspace.project.id(), None)
            .unwrap();
        assert_eq!(back.work_folder, given_work_folder());
        assert!(!back.is_work_folder_chosen);
    }

    #[test]
    fn refuses_a_work_folder_that_does_not_exist_and_one_without_a_name() {
        let workspace = workspace();

        let missing = workspace.set_work_folder("/Nowhere").unwrap_err();
        assert_eq!(missing.code(), "folder_not_found");
        assert_eq!(missing.to_string(), "the folder /Nowhere does not exist");

        let unnamed = workspace.set_work_folder("   ").unwrap_err();
        assert_eq!(unnamed.code(), "rule_broken");
        assert_eq!(unnamed.to_string(), "the folder must be named");
    }

    #[test]
    fn does_not_change_the_work_folder_while_a_specialist_works_in_the_project() {
        let workspace = workspace();
        let engineer = workspace
            .team
            .add(NewMember {
                project_id: workspace.project.id(),
                name: "Hardware engineer".to_owned(),
                kind: MemberKind::Agent,
                responsibility: String::new(),
                model: None,
                effort: None,
            })
            .unwrap();
        let task = workspace
            .tasks
            .create(NewTask {
                assignee: Some(Assignee::Member(engineer.id())),
                ..NewTask::titled(workspace.project.id(), "Draw the schematic")
            })
            .unwrap();
        workspace.work.start_work(task.id()).unwrap();

        let error = workspace.set_work_folder("/Work/sensor-node").unwrap_err();

        assert_eq!(error.code(), "work_in_progress");
        assert!(!workspace.current().is_work_folder_chosen);
    }

    #[test]
    fn lets_the_specialists_read_a_reference_folder_and_lists_it_once() {
        let workspace = workspace();

        workspace.add_reference_folder("/Datasheets").unwrap();
        let places = workspace.add_reference_folder("/datasheets/").unwrap();

        assert_eq!(places.reference_folders, [path("/Datasheets")]);
    }

    #[test]
    fn takes_a_reference_folder_away_again_and_does_nothing_for_one_it_does_not_have() {
        let workspace = workspace();
        workspace.add_reference_folder("/Datasheets").unwrap();

        let still = workspace
            .places
            .remove_reference_folder(workspace.project.id(), Path::new("/Work"))
            .unwrap();
        assert_eq!(still.reference_folders, [path("/Datasheets")]);
        let gone = workspace
            .places
            .remove_reference_folder(workspace.project.id(), Path::new("/Datasheets/"))
            .unwrap();
        assert!(gone.reference_folders.is_empty());
    }

    #[test]
    fn refuses_a_reference_folder_that_does_not_exist() {
        let workspace = workspace();

        let error = workspace.add_reference_folder("/Missing").unwrap_err();

        assert_eq!(error.code(), "folder_not_found");
    }

    #[test]
    fn refuses_a_reference_folder_that_is_the_work_folder_or_lies_inside_it() {
        let workspace = workspace();
        workspace.set_work_folder("/Work/sensor-node").unwrap();

        let inside = workspace
            .add_reference_folder("/Work/sensor-node/docs")
            .unwrap_err();
        assert_eq!(inside.code(), "rule_broken");
        assert_eq!(
            inside.to_string(),
            "/Work/sensor-node/docs lies inside the work folder /Work/sensor-node, where the \
             specialists read everything already"
        );
        let same = workspace
            .add_reference_folder("/work/SENSOR-NODE")
            .unwrap_err();
        assert_eq!(same.code(), "rule_broken");
        assert_eq!(
            same.to_string(),
            "the work folder and a reference folder cannot be the same folder: /Work/sensor-node"
        );
    }

    #[test]
    fn refuses_a_reference_folder_that_contains_the_work_folder() {
        let workspace = workspace();
        workspace.set_work_folder("/Work/sensor-node").unwrap();

        let error = workspace.add_reference_folder("/Work").unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(
            error.to_string(),
            "the work folder /Work/sensor-node lies inside the reference folder /Work, where the \
             specialists may not write"
        );
    }

    #[test]
    fn refuses_a_work_folder_that_gets_in_the_way_of_a_reference_folder() {
        let workspace = workspace();
        workspace.add_reference_folder("/Datasheets").unwrap();

        let inside = workspace.set_work_folder("/Datasheets/Bosch").unwrap_err();
        assert_eq!(inside.code(), "rule_broken");
        let same = workspace.set_work_folder("/Datasheets").unwrap_err();
        assert_eq!(same.code(), "rule_broken");
        assert!(!workspace.current().is_work_folder_chosen);
    }

    #[test]
    fn copies_reference_files_by_their_names_and_gives_a_name_that_is_taken_a_number() {
        let workspace = workspace();
        for file in [
            "/Downloads/BME680 datasheet.pdf",
            "/Datasheets/Bosch/bme680 DATASHEET.pdf",
            "/Datasheets/notes",
        ] {
            workspace.file_system.add_file(file);
        }

        let names = workspace
            .places
            .add_reference_files(
                workspace.project.id(),
                &[
                    path("/Downloads/BME680 datasheet.pdf"),
                    path("/Datasheets/Bosch/bme680 DATASHEET.pdf"),
                    path("/Datasheets/notes"),
                ],
            )
            .unwrap();
        let again = workspace
            .places
            .add_reference_files(workspace.project.id(), &[path("/Datasheets/notes")])
            .unwrap();

        assert_eq!(
            names,
            ["BME680 datasheet.pdf", "bme680 DATASHEET (2).pdf", "notes"]
        );
        assert_eq!(again, ["notes (2)"]);
        let references = given_work_folder().join(REFERENCES_FOLDER);
        assert!(
            workspace
                .file_system
                .files()
                .contains(&references.join("bme680 DATASHEET (2).pdf"))
        );
    }

    #[test]
    fn copies_reference_files_into_the_work_folder_the_manager_chose() {
        let workspace = workspace();
        workspace.file_system.add_file("/Datasheets/notes");
        workspace.set_work_folder("/Work/sensor-node").unwrap();

        workspace
            .places
            .add_reference_files(workspace.project.id(), &[path(" /Datasheets/notes ")])
            .unwrap();

        assert!(
            workspace.file_system.files().contains(
                &path("/Work/sensor-node")
                    .join(REFERENCES_FOLDER)
                    .join("notes")
            )
        );
    }

    #[test]
    fn refuses_a_file_without_a_name_and_copies_nothing() {
        let workspace = workspace();
        workspace.file_system.add_file("/Datasheets/notes");

        let error = workspace
            .places
            .add_reference_files(
                workspace.project.id(),
                &[path("/Datasheets/notes"), path("   ")],
            )
            .unwrap_err();

        assert_eq!(error.code(), "rule_broken");
        assert_eq!(error.to_string(), "a file to copy has no name");
        assert_eq!(workspace.file_system.files().len(), 1);
    }

    #[test]
    fn reports_a_file_that_cannot_be_read_by_its_name() {
        let workspace = workspace();

        let error = workspace
            .places
            .add_reference_files(workspace.project.id(), &[path("/Downloads/missing.pdf")])
            .unwrap_err();

        assert_eq!(error.code(), "storage_failed");
        assert!(error.to_string().contains("/Downloads/missing.pdf"));
    }

    #[test]
    fn forgets_the_folders_of_a_project_that_is_deleted() {
        let workspace = workspace();
        workspace.set_work_folder("/Work/sensor-node").unwrap();

        workspace.projects.delete(workspace.project.id()).unwrap();

        let error = workspace.places.places(workspace.project.id()).unwrap_err();
        assert_eq!(error.code(), "not_found");
        assert!(
            workspace
                .file_system
                .is_folder(Path::new("/Work/sensor-node"))
        );
    }

    #[test]
    fn knows_no_project_that_does_not_exist() {
        let workspace = workspace();

        let error = workspace.places.places(ProjectId::generate()).unwrap_err();

        assert_eq!(error.code(), "not_found");
    }

    #[test]
    fn names_only_the_reference_folders_that_still_exist() {
        let workspace = workspace();
        workspace.add_reference_folder("/Datasheets").unwrap();
        workspace.add_reference_folder("/Work").unwrap();
        workspace.file_system.add_folder("/Gone");
        workspace.add_reference_folder("/Gone").unwrap();
        let file_system_without_gone = PretendFileSystem::with_folders(FOLDERS);
        let places = PlacesService {
            file_system: Arc::new(file_system_without_gone),
            ..workspace.places.clone()
        };

        assert_eq!(
            places
                .existing_reference_folders(&workspace.project)
                .unwrap(),
            [path("/Datasheets"), path("/Work")]
        );
    }

    #[test]
    fn finds_a_free_name_by_counting_up_in_front_of_the_ending() {
        let taken = |names: &[&str]| {
            names
                .iter()
                .map(|name| (*name).to_owned())
                .collect::<Vec<_>>()
        };

        assert_eq!(free_name("sheet.pdf", &[]), "sheet.pdf");
        assert_eq!(
            free_name("sheet.pdf", &taken(&["Sheet.pdf"])),
            "sheet (2).pdf"
        );
        assert_eq!(
            free_name("sheet.pdf", &taken(&["sheet.pdf", "sheet (2).pdf"])),
            "sheet (3).pdf"
        );
        assert_eq!(
            free_name(".gitignore", &taken(&[".gitignore"])),
            ".gitignore (2)"
        );
    }
}
