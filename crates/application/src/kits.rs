//! The library of skills and connectors of a project, and what each
//! specialist is given of it
//! ([decision 0039](../../../docs/decisions/0039-skills-and-connectors-chosen-by-the-manager.md)).
//!
//! The library is data of the project: it is logged and reaches the other
//! computers. What belongs to one computer is not: the copy of a skill's
//! folder, which [`SkillShelf`] keeps on this computer, and the values of
//! the environment variables a connector needs, which come from the
//! environment Teamwright was started in. A turn is given only what this
//! computer has; the rest is shown to the manager as missing here.
//!
//! Nor is what the manager confirmed on this computer
//! ([`ConnectorTrust`]). A connector and what a specialist is given of it
//! arrive from other computers through the sync folder, which others may
//! write; so a connector is started here only if the manager confirmed, on
//! this computer, exactly its definition, and a specialist is given here
//! only the tools that were confirmed here (decision 0039, note of 22
//! November 2026).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use teamwright_domain::{
    Connector, ConnectorId, ConnectorName, ConnectorStart, DomainError, GivenConnector, Member,
    MemberId, MemberKind, MemberKit, ProjectId, SecretHeader, Skill, SkillFile, SkillId, SkillName,
    ToolName, VariableName,
};

use crate::agent_runtime::{KitConnector, KitSkill, TurnKit};
use crate::error::ApplicationError;
use crate::ports::{Clock, MemberRepository, ProjectRepository, StorageError};

/// The most files a skill's folder may hold.
pub const MAX_SKILL_FILES: usize = 200;

/// The most bytes a skill's folder may hold: 10 MiB.
pub const MAX_SKILL_BYTES: u64 = 10 * 1024 * 1024;

/// The name of the file that makes a folder a skill.
pub const SKILL_FILE: &str = "SKILL.md";

/// Stores the libraries of the projects and the kits of the specialists.
/// Every write is logged and reaches the other computers.
pub trait KitRepository: Send + Sync {
    /// The skills of the library of a project, in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn skills_of_project(&self, project_id: ProjectId) -> Result<Vec<Skill>, StorageError>;

    /// The connectors of the library of a project, in no particular order.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn connectors_of_project(&self, project_id: ProjectId) -> Result<Vec<Connector>, StorageError>;

    /// The kits of the specialists of a project, in no particular order. A
    /// specialist that was never given anything has none.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn kits_of_project(&self, project_id: ProjectId) -> Result<Vec<MemberKit>, StorageError>;

    /// The kit of one specialist, if it has one.
    ///
    /// # Errors
    ///
    /// Returns an error if stored data could not be read.
    fn kit_of(&self, member_id: MemberId) -> Result<Option<MemberKit>, StorageError>;

    /// Stores a skill of a library.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written.
    fn save_skill(&self, skill: &Skill) -> Result<(), StorageError>;

    /// Stores a connector of a library.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written.
    fn save_connector(&self, connector: &Connector) -> Result<(), StorageError>;

    /// Stores the kit of a specialist, replacing the one before.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written.
    fn save_kit(&self, kit: &MemberKit) -> Result<(), StorageError>;

    /// Removes a skill from its library and stores the kits that had it,
    /// without it, in one write. A skill that is not stored is ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written; then nothing is.
    fn remove_skill(&self, id: SkillId, kits: &[MemberKit]) -> Result<(), StorageError>;

    /// Removes a connector from its library and stores the kits that had
    /// it, without it, in one write.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written; then nothing is.
    fn remove_connector(&self, id: ConnectorId, kits: &[MemberKit]) -> Result<(), StorageError>;
}

/// Why a folder the manager chose cannot be looked at. It is the choice
/// that is wrong, or the permission of the folder; it is not a failure of
/// the storage of Teamwright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenFolderProblem {
    /// There is nothing at that path.
    Missing,
    /// There is something, but it is not a folder.
    NotAFolder,
    /// The folder, or something in it, is not allowed to be read.
    NotAllowed,
}

impl ChosenFolderProblem {
    /// What to tell the manager, in words they can act on.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Missing => "That folder does not exist.",
            Self::NotAFolder => "That is not a folder.",
            Self::NotAllowed => "Teamwright may not read that folder.",
        }
    }

    /// A code to decide by, stable.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Missing => "folder_missing",
            Self::NotAFolder => "not_a_folder",
            Self::NotAllowed => "folder_not_readable",
        }
    }
}

/// What a folder the manager points at holds, as far as the check of a
/// skill needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFolder {
    /// The name of the folder itself.
    pub folder_name: String,
    /// The text of its `SKILL.md`, if it has one directly inside.
    pub skill_text: Option<String>,
    /// How many files it holds, in all its folders. Counting stops a little
    /// past [`MAX_SKILL_FILES`].
    pub files: usize,
    /// How many bytes they have together.
    pub bytes: u64,
    /// Whether it holds a link (on Windows also a junction), which could
    /// lead outside it.
    pub has_links: bool,
}

/// The folder in which the agent program finds the manifest of a plugin.
pub const MANIFEST_FOLDER: &str = ".claude-plugin";

/// The manifest of a plugin, inside [`MANIFEST_FOLDER`].
pub const MANIFEST_FILE: &str = "plugin.json";

/// The folder of the skills of a plugin.
pub const SKILLS_FOLDER: &str = "skills";

/// A skill whose folder this computer keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeptSkill {
    /// The folder the agent program is given: the manifest and
    /// `skills/<name>/`.
    pub folder: PathBuf,
    /// The name of the folder of the skill inside it, or empty if
    /// `skills/` does not hold exactly one entry.
    pub skill_folder_name: String,
    /// The text of the kept `SKILL.md`, read again for every turn; empty if
    /// it cannot be read.
    pub skill_text: String,
    /// What the copy holds, read again for every turn.
    pub layout: KeptLayout,
}

/// What the copy of a skill holds, as far as the check before a turn needs
/// it. The agent program loads more than skills from a plugin folder
/// (commands, agents, hooks, tool servers), so the copy must hold exactly
/// what Teamwright wrote and nothing else.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeptLayout {
    /// The names at the top of the copy.
    pub top: Vec<String>,
    /// The names inside [`MANIFEST_FOLDER`].
    pub manifest: Vec<String>,
    /// The names inside [`SKILLS_FOLDER`].
    pub skills: Vec<String>,
    /// How many files the folder of the skill holds. Counting stops a
    /// little past [`MAX_SKILL_FILES`].
    pub files: usize,
    /// How many bytes they have together.
    pub bytes: u64,
    /// Whether the copy holds a link anywhere, or is one.
    pub has_links: bool,
}

/// Checks that the copy of a skill holds exactly what Teamwright wrote:
/// the manifest and the folder of the skill, no link anywhere, and no more
/// than a skill may hold.
///
/// # Errors
///
/// Returns why not, in words.
pub fn check_kept_layout(layout: &KeptLayout, skill: &SkillName) -> Result<(), String> {
    if layout.has_links {
        return Err("its copy holds a link, which could lead outside it".to_owned());
    }
    let mut top = layout.top.clone();
    top.sort();
    if top != [MANIFEST_FOLDER, SKILLS_FOLDER] {
        let extra: Vec<&str> = top
            .iter()
            .map(String::as_str)
            .filter(|name| ![MANIFEST_FOLDER, SKILLS_FOLDER].contains(name))
            .collect();
        return Err(if extra.is_empty() {
            "its copy lacks the manifest or the folder of the skill".to_owned()
        } else {
            format!(
                "its copy holds more than Teamwright wrote: {}",
                shortened_list(&extra)
            )
        });
    }
    if layout.manifest != [MANIFEST_FILE] {
        return Err(format!(
            "the folder {MANIFEST_FOLDER} of its copy holds more or less than {MANIFEST_FILE}"
        ));
    }
    if layout.skills != [skill.as_str()] {
        return Err(format!(
            "the folder {SKILLS_FOLDER} of its copy holds more or less than the skill {skill}"
        ));
    }
    if layout.files > MAX_SKILL_FILES || layout.bytes > MAX_SKILL_BYTES {
        return Err(format!(
            "its copy is larger than a skill may be: at most {MAX_SKILL_FILES} files and {} MiB",
            MAX_SKILL_BYTES / 1024 / 1024
        ));
    }
    Ok(())
}

/// A few names, joined, for a message.
fn shortened_list(names: &[&str]) -> String {
    let mut shown: Vec<String> = names
        .iter()
        .take(3)
        .map(|name| name.chars().take(40).collect())
        .collect();
    if names.len() > 3 {
        shown.push(format!("and {} more", names.len() - 3));
    }
    shown.join(", ")
}

/// The disk and the environment of this computer, as far as skills and
/// connectors need them. Nothing it keeps reaches another computer.
pub trait SkillShelf: Send + Sync {
    /// Looks at a folder the manager chose as a skill.
    ///
    /// # Errors
    ///
    /// Returns the problem with the folder if it does not exist, is not a
    /// folder, or cannot be read.
    fn read_folder(&self, folder: &Path) -> Result<SkillFolder, ChosenFolderProblem>;

    /// Copies a skill's folder onto the shelf of this computer, in the form
    /// the agent program loads for one session, replacing an earlier copy.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder cannot be read or the copy written.
    fn keep(&self, from: &Path, skill: &Skill) -> Result<(), StorageError>;

    /// The copy of a skill this computer keeps, if it keeps one that can be
    /// read.
    fn kept(&self, skill: SkillId) -> Option<KeptSkill>;

    /// Removes the copy of a skill from this computer. A skill without one
    /// is ignored. The folder the manager chose is never touched.
    ///
    /// # Errors
    ///
    /// Returns an error if the copy cannot be removed.
    fn forget(&self, skill: SkillId) -> Result<(), StorageError>;

    /// Whether an environment variable is set, and not empty, in the
    /// environment Teamwright was started in. Its value is never read here.
    fn has_variable(&self, name: &VariableName) -> bool;
}

/// A skill of the library, with whether this computer has its folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySkill {
    /// The skill.
    pub skill: Skill,
    /// Whether this computer keeps a copy of its folder that still passes
    /// the check. Without one, no turn on this computer is given it.
    pub here: bool,
    /// Why the copy this computer keeps is not given to any turn, if it is
    /// there and fails the check: changed on the disk, holding more than
    /// Teamwright wrote, a link, or too much.
    pub problem: Option<String>,
}

/// What the manager confirmed on this computer, kept on this computer
/// alone: never logged, never written to the sync folder, so that nobody
/// who can write there can confirm anything (decision 0039, note of 22
/// November 2026).
///
/// - A connector is confirmed by the fingerprint of its whole definition
///   ([`Connector::fingerprint`]). Any later change, from anywhere, makes
///   it differ, and the connector waits for the manager again.
/// - What a specialist is given is confirmed tool by tool: the pairs of a
///   connector and one of its tools.
pub trait ConnectorTrust: Send + Sync {
    /// The fingerprint of the definition of a connector the manager
    /// confirmed on this computer, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be read.
    fn confirmed_definition(&self, connector: ConnectorId) -> Result<Option<String>, StorageError>;

    /// Keeps the fingerprint of the definition of a connector as the one
    /// the manager confirmed on this computer.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written.
    fn confirm_definition(
        &self,
        connector: ConnectorId,
        fingerprint: &str,
    ) -> Result<(), StorageError>;

    /// Forgets what was confirmed of a connector that left the library.
    ///
    /// # Errors
    ///
    /// Returns an error if it could not be written.
    fn forget_definition(&self, connector: ConnectorId) -> Result<(), StorageError>;

    /// The tools of connectors the manager confirmed, on this computer, as
    /// given to a specialist.
    ///
    /// # Errors
    ///
    /// Returns an error if they could not be read.
    fn confirmed_tools(
        &self,
        member: MemberId,
    ) -> Result<BTreeSet<(ConnectorId, ToolName)>, StorageError>;

    /// Keeps these tools, and no others, as the ones the manager confirmed
    /// on this computer as given to a specialist.
    ///
    /// # Errors
    ///
    /// Returns an error if they could not be written.
    fn confirm_tools(
        &self,
        member: MemberId,
        tools: &BTreeSet<(ConnectorId, ToolName)>,
    ) -> Result<(), StorageError>;
}

/// The tools a specialist was given of one connector that the manager has
/// not confirmed on this computer: given on another computer, or before
/// confirmations existed. No turn here is given them until the manager
/// confirms them here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingTools {
    /// The specialist.
    pub member_id: MemberId,
    /// The connector.
    pub connector_id: ConnectorId,
    /// The tools that wait, in the order of the kit.
    pub tools: Vec<ToolName>,
}

/// A connector of the library, with what this computer lacks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryConnector {
    /// The connector.
    pub connector: Connector,
    /// The fingerprint of its definition as it is now
    /// ([`Connector::fingerprint`]). The manager confirms it by sending it
    /// back, so that a definition that changed in between is not confirmed
    /// unseen.
    pub fingerprint: String,
    /// Whether the manager confirmed exactly this definition on this
    /// computer. Until then no turn here is given it: it arrived or
    /// changed through the sync folder, or was added before confirmations
    /// existed.
    pub confirmed: bool,
    /// The environment variables it needs that are not set here. While one
    /// is missing, no turn on this computer is given it.
    pub missing_variables: Vec<VariableName>,
    /// Why no turn is given it any more, if it breaks a rule that is newer
    /// than it, such as a program named by a relative path. The manager
    /// removes it and adds it again in the form the rule asks.
    pub problem: Option<String>,
}

/// The library of a project and the kits of its specialists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// The skills, by name.
    pub skills: Vec<LibrarySkill>,
    /// The connectors, by name.
    pub connectors: Vec<LibraryConnector>,
    /// What each specialist of the project was given, naming only skills
    /// and connectors the library has.
    pub kits: Vec<MemberKit>,
    /// The tools of the kits that wait for the manager's confirmation on
    /// this computer, by specialist and connector.
    pub waiting_tools: Vec<WaitingTools>,
}

/// A connector the manager describes, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewConnector {
    /// Its name.
    pub name: String,
    /// How it is started.
    pub start: NewConnectorStart,
    /// The names of its tools that the manager may give.
    pub tools: Vec<String>,
}

/// How a connector is started, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewConnectorStart {
    /// A program, with its arguments and the variables it is handed.
    Program {
        /// The program.
        program: String,
        /// Its arguments.
        arguments: Vec<String>,
        /// The names of the environment variables it is handed.
        variables: Vec<String>,
    },
    /// A web address, with the headers that carry its secrets.
    Address {
        /// The address.
        url: String,
        /// The headers.
        headers: Vec<NewHeader>,
    },
}

/// A header of a connector at a web address, before it is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewHeader {
    /// The name of the header.
    pub name: String,
    /// The environment variable that holds its secret.
    pub variable: String,
    /// Whether the value is `Bearer ` and the secret.
    pub bearer: bool,
}

/// What the manager gives one specialist.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KitChoice {
    /// The skills.
    pub skills: Vec<SkillId>,
    /// The connectors, each with the names of the tools given of it.
    pub connectors: Vec<(ConnectorId, Vec<String>)>,
}

/// The use cases of the library of a project and of the kits of its
/// specialists.
#[derive(Clone)]
pub struct KitService {
    kits: Arc<dyn KitRepository>,
    shelf: Arc<dyn SkillShelf>,
    trust: Arc<dyn ConnectorTrust>,
    projects: Arc<dyn ProjectRepository>,
    members: Arc<dyn MemberRepository>,
    clock: Arc<dyn Clock>,
}

impl KitService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        kits: Arc<dyn KitRepository>,
        shelf: Arc<dyn SkillShelf>,
        trust: Arc<dyn ConnectorTrust>,
        projects: Arc<dyn ProjectRepository>,
        members: Arc<dyn MemberRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            kits,
            shelf,
            trust,
            projects,
            members,
            clock,
        }
    }

    /// The library of a project, with what this computer has of it, and
    /// the kits of its specialists.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project or storage fails.
    pub fn library(&self, project_id: ProjectId) -> Result<Library, ApplicationError> {
        self.require_project(project_id)?;
        let mut skills = self.kits.skills_of_project(project_id)?;
        skills.sort_by(|first, second| first.name().cmp(second.name()));
        let mut connectors = self.kits.connectors_of_project(project_id)?;
        connectors.sort_by(|first, second| first.name().cmp(second.name()));
        let members: BTreeSet<MemberId> = self
            .members
            .list_for_project(project_id)?
            .iter()
            .map(Member::id)
            .collect();
        let kits: Vec<MemberKit> = self
            .kits
            .kits_of_project(project_id)?
            .into_iter()
            .filter(|kit| members.contains(&kit.member_id()))
            .map(|kit| known_part(&kit, &skills, &connectors))
            .collect();
        let mut waiting_tools = Vec::new();
        for kit in &kits {
            let confirmed = self.trust.confirmed_tools(kit.member_id())?;
            for given in kit.connectors() {
                let tools: Vec<ToolName> = given
                    .tools()
                    .iter()
                    .filter(|tool| !confirmed.contains(&(given.connector_id(), (*tool).clone())))
                    .cloned()
                    .collect();
                if !tools.is_empty() {
                    waiting_tools.push(WaitingTools {
                        member_id: kit.member_id(),
                        connector_id: given.connector_id(),
                        tools,
                    });
                }
            }
        }
        let mut library_connectors = Vec::with_capacity(connectors.len());
        for connector in connectors {
            let fingerprint = connector.fingerprint();
            library_connectors.push(LibraryConnector {
                confirmed: self.trust.confirmed_definition(connector.id())?.as_deref()
                    == Some(fingerprint.as_str()),
                fingerprint,
                missing_variables: missing_variables(self.shelf.as_ref(), &connector),
                problem: connector.start().problem(),
                connector,
            });
        }
        Ok(Library {
            skills: skills
                .into_iter()
                .map(|skill| {
                    let (here, problem) = match copy_here(self.shelf.as_ref(), &skill) {
                        CopyHere::Usable(_) => (true, None),
                        CopyHere::Missing => (false, None),
                        CopyHere::Refused(problem) => (false, Some(problem)),
                    };
                    LibrarySkill {
                        here,
                        problem,
                        skill,
                    }
                })
                .collect(),
            connectors: library_connectors,
            kits,
            waiting_tools,
        })
    }

    /// Adds the skill in a folder to the library of a project, and keeps a
    /// copy of the folder on this computer. The folder the manager chose is
    /// only read.
    ///
    /// A skill of the same name that this computer does not keep yet, added
    /// on another computer, is the one added again: the copy is taken here,
    /// and the specialists that have it keep it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project; if the folder does not
    /// exist, is not a folder, or may not be read (`folder_missing`,
    /// `not_a_folder`, `folder_not_readable`); if the folder is not a
    /// skill, holds a link, or is too large; if the library already has a
    /// skill of that name that this computer keeps; or if storage fails.
    pub fn add_skill(
        &self,
        project_id: ProjectId,
        folder: &Path,
    ) -> Result<Skill, ApplicationError> {
        self.require_project(project_id)?;
        let found = self
            .shelf
            .read_folder(folder)
            .map_err(ApplicationError::ChosenFolder)?;
        let Some(text) = &found.skill_text else {
            return Err(not_a_skill(&format!(
                "the folder {} has no {SKILL_FILE} directly inside",
                found.folder_name
            )));
        };
        if found.has_links {
            return Err(not_a_skill(
                "the folder holds a link, which could lead outside it",
            ));
        }
        if found.files > MAX_SKILL_FILES || found.bytes > MAX_SKILL_BYTES {
            return Err(not_a_skill(&format!(
                "the folder is larger than a skill may be: at most {MAX_SKILL_FILES} files and \
                 {} MiB",
                MAX_SKILL_BYTES / 1024 / 1024
            )));
        }
        let file = SkillFile::check(text, &found.folder_name)?;
        let existing = self
            .kits
            .skills_of_project(project_id)?
            .into_iter()
            .find(|skill| skill.name() == file.name());
        let skill = match existing {
            Some(skill)
                if matches!(copy_here(self.shelf.as_ref(), &skill), CopyHere::Usable(_)) =>
            {
                return Err(not_a_skill(&format!(
                    "the library already has the skill \"{}\"; remove it first to add it again",
                    skill.name()
                )));
            }
            Some(mut skill) => {
                skill.take_again(file)?;
                skill
            }
            None => Skill::add(project_id, file, self.clock.now()),
        };
        // The copy is made first: a skill that is stored has a copy here.
        self.shelf.keep(folder, &skill)?;
        if let Err(error) = self.kits.save_skill(&skill) {
            // Nothing refers to the copy yet; a failure to remove it only
            // leaves a folder nobody uses.
            let _ = self.shelf.forget(skill.id());
            return Err(error.into());
        }
        Ok(skill)
    }

    /// Removes a skill from the library, from every specialist that has
    /// it, and its copy from this computer. The folder the manager chose
    /// is never touched; copies on other computers go when they next open
    /// the library and find the skill gone.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such skill or storage fails.
    pub fn remove_skill(&self, project_id: ProjectId, id: SkillId) -> Result<(), ApplicationError> {
        let skills = self.kits.skills_of_project(project_id)?;
        if !skills.iter().any(|skill| skill.id() == id) {
            return Err(ApplicationError::NotFound {
                kind: "skill",
                id: id.to_string(),
            });
        }
        let kits: Vec<MemberKit> = self
            .kits
            .kits_of_project(project_id)?
            .into_iter()
            .filter_map(|mut kit| kit.without_skill(id).then_some(kit))
            .collect();
        self.kits.remove_skill(id, &kits)?;
        self.shelf.forget(id)?;
        Ok(())
    }

    /// Adds a connector to the library of a project. Added here, through
    /// the interface, it is confirmed on this computer as it is.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such project, if the connector
    /// breaks a rule, if the library has a connector of that name already,
    /// or if storage fails.
    pub fn add_connector(
        &self,
        project_id: ProjectId,
        new: NewConnector,
    ) -> Result<Connector, ApplicationError> {
        self.require_project(project_id)?;
        let name = ConnectorName::parse(&new.name)?;
        let start = match new.start {
            NewConnectorStart::Program {
                program,
                arguments,
                variables,
            } => ConnectorStart::program(
                &program,
                arguments,
                parse_all(&variables, VariableName::parse)?,
            )?,
            NewConnectorStart::Address { url, headers } => {
                let headers = headers
                    .iter()
                    .map(|header| {
                        SecretHeader::new(
                            &header.name,
                            VariableName::parse(&header.variable)?,
                            header.bearer,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ConnectorStart::address(&url, headers)?
            }
        };
        let tools = parse_all(&new.tools, ToolName::parse)?;
        if self
            .kits
            .connectors_of_project(project_id)?
            .iter()
            .any(|connector| connector.name() == &name)
        {
            return Err(DomainError::InvalidConnector {
                reason: format!("the library already has a connector called {name}"),
            }
            .into());
        }
        let connector = Connector::add(project_id, name, start, tools, self.clock.now())?;
        self.kits.save_connector(&connector)?;
        self.trust
            .confirm_definition(connector.id(), &connector.fingerprint())?;
        Ok(connector)
    }

    /// Removes a connector from the library and from every specialist
    /// that has it.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such connector or storage fails.
    pub fn remove_connector(
        &self,
        project_id: ProjectId,
        id: ConnectorId,
    ) -> Result<(), ApplicationError> {
        let connectors = self.kits.connectors_of_project(project_id)?;
        if !connectors.iter().any(|connector| connector.id() == id) {
            return Err(ApplicationError::NotFound {
                kind: "connector",
                id: id.to_string(),
            });
        }
        let kits: Vec<MemberKit> = self
            .kits
            .kits_of_project(project_id)?
            .into_iter()
            .filter_map(|mut kit| kit.without_connector(id).then_some(kit))
            .collect();
        self.kits.remove_connector(id, &kits)?;
        // Removing needs no confirmation; what was confirmed goes with it.
        self.trust.forget_definition(id)?;
        Ok(())
    }

    /// Confirms, on this computer, the definition of a connector that
    /// arrived or changed through the sync folder, so that turns here may
    /// be given it. `fingerprint` is the one the manager was shown
    /// ([`LibraryConnector::fingerprint`]): if the definition changed since,
    /// nothing is confirmed.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such connector, if it breaks a rule
    /// newer than it, if it changed since the manager looked at it, or if
    /// storage fails.
    pub fn confirm_connector(
        &self,
        project_id: ProjectId,
        id: ConnectorId,
        fingerprint: &str,
    ) -> Result<(), ApplicationError> {
        let connector = self
            .kits
            .connectors_of_project(project_id)?
            .into_iter()
            .find(|connector| connector.id() == id)
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "connector",
                id: id.to_string(),
            })?;
        if let Some(problem) = connector.start().problem() {
            return Err(DomainError::InvalidConnector { reason: problem }.into());
        }
        if connector.fingerprint() != fingerprint {
            return Err(DomainError::InvalidConnector {
                reason: format!(
                    "the connector {} changed since you looked at it; look at it again before \
                     you confirm it",
                    connector.name()
                ),
            }
            .into());
        }
        self.trust.confirm_definition(id, fingerprint)?;
        Ok(())
    }

    /// Confirms, on this computer, tools a specialist was given on another
    /// computer: of `seen`, the tools the manager was shown, those its kit
    /// still gives. What the kit gained since is not confirmed.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member or storage fails.
    pub fn confirm_kit(
        &self,
        member_id: MemberId,
        seen: &[(ConnectorId, Vec<String>)],
    ) -> Result<(), ApplicationError> {
        let member = self
            .members
            .find(member_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "member",
                id: member_id.to_string(),
            })?;
        let Some(kit) = self.kits.kit_of(member.id())? else {
            return Ok(());
        };
        let mut confirmed = self.trust.confirmed_tools(member_id)?;
        for given in kit.connectors() {
            let Some((_, tools)) = seen
                .iter()
                .find(|(connector_id, _)| *connector_id == given.connector_id())
            else {
                continue;
            };
            for tool in given.tools() {
                if tools.iter().any(|seen| seen == tool.as_str()) {
                    confirmed.insert((given.connector_id(), tool.clone()));
                }
            }
        }
        self.trust.confirm_tools(member_id, &confirmed)?;
        Ok(())
    }

    /// Gives a specialist these skills and connectors of the library of its
    /// project, and nothing else, and answers with its kit.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no such member; if it is a person, who
    /// runs no session; if a skill or a connector is not in the library of
    /// its project; if a tool is not one of its connector's; if the kit
    /// breaks a rule; or if storage fails.
    pub fn set_kit(
        &self,
        member_id: MemberId,
        choice: KitChoice,
    ) -> Result<MemberKit, ApplicationError> {
        let member = self
            .members
            .find(member_id)?
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "member",
                id: member_id.to_string(),
            })?;
        let project_id = member.project_id();
        if member.kind() != MemberKind::Agent
            && !(choice.skills.is_empty() && choice.connectors.is_empty())
        {
            return Err(DomainError::InvalidConnector {
                reason: "only an AI specialist is given skills and connectors".to_owned(),
            }
            .into());
        }
        let skills = self.kits.skills_of_project(project_id)?;
        for skill in &choice.skills {
            if !skills.iter().any(|known| known.id() == *skill) {
                return Err(ApplicationError::NotFound {
                    kind: "skill",
                    id: skill.to_string(),
                });
            }
        }
        let connectors = self.kits.connectors_of_project(project_id)?;
        let mut given = Vec::new();
        for (connector_id, tools) in choice.connectors {
            let connector = connectors
                .iter()
                .find(|known| known.id() == connector_id)
                .ok_or_else(|| ApplicationError::NotFound {
                    kind: "connector",
                    id: connector_id.to_string(),
                })?;
            let tools = parse_all(&tools, ToolName::parse)?;
            if let Some(unknown) = tools.iter().find(|tool| !connector.tools().contains(tool)) {
                return Err(DomainError::InvalidConnector {
                    reason: format!(
                        "the connector {} has no tool called {unknown}",
                        connector.name()
                    ),
                }
                .into());
            }
            given.push(GivenConnector::new(connector_id, tools)?);
        }
        let kit = MemberKit::new(member_id, project_id, choice.skills, given)?;
        self.kits.save_kit(&kit)?;
        // Chosen here, through the interface: what the kit gives is
        // confirmed on this computer, and nothing else.
        let confirmed = kit
            .connectors()
            .iter()
            .flat_map(|given| {
                given
                    .tools()
                    .iter()
                    .map(move |tool| (given.connector_id(), tool.clone()))
            })
            .collect();
        self.trust.confirm_tools(member_id, &confirmed)?;
        Ok(kit)
    }

    fn require_project(&self, id: ProjectId) -> Result<(), ApplicationError> {
        self.projects
            .find(id)?
            .map(|_| ())
            .ok_or_else(|| ApplicationError::NotFound {
                kind: "project",
                id: id.to_string(),
            })
    }
}

/// The skills and connectors a turn of a specialist is given: what it was
/// given of the library, as far as this computer has it. A skill whose
/// copy is missing here or no longer passes the check, a connector whose
/// variables are not all set here, a connector that breaks a rule newer
/// than it, a connector whose definition the manager has not confirmed on
/// this computer exactly as it is now, and a tool the manager has not
/// confirmed here as given to this specialist, are left out.
///
/// # Errors
///
/// Returns an error if storage fails.
pub fn turn_kit_of(
    kits: &dyn KitRepository,
    shelf: &dyn SkillShelf,
    trust: &dyn ConnectorTrust,
    member: &Member,
) -> Result<TurnKit, ApplicationError> {
    let Some(kit) = kits.kit_of(member.id())? else {
        return Ok(TurnKit::default());
    };
    if member.kind() != MemberKind::Agent || kit.is_empty() {
        return Ok(TurnKit::default());
    }
    let skills = kits.skills_of_project(member.project_id())?;
    let connectors = kits.connectors_of_project(member.project_id())?;
    let confirmed_tools = if kit.connectors().is_empty() {
        BTreeSet::new()
    } else {
        trust.confirmed_tools(member.id())?
    };
    let mut turn = TurnKit::default();
    for id in kit.skills() {
        let Some(skill) = skills.iter().find(|skill| skill.id() == *id) else {
            continue;
        };
        if let CopyHere::Usable(kept) = copy_here(shelf, skill) {
            turn.skills.push(KitSkill {
                name: skill.name().to_string(),
                folder: kept.folder,
            });
        }
    }
    for given in kit.connectors() {
        let Some(connector) = connectors
            .iter()
            .find(|connector| connector.id() == given.connector_id())
        else {
            continue;
        };
        if !missing_variables(shelf, connector).is_empty() || connector.start().problem().is_some()
        {
            continue;
        }
        // Started only as the manager confirmed it here, to the byte.
        if trust.confirmed_definition(connector.id())?.as_deref()
            != Some(connector.fingerprint().as_str())
        {
            continue;
        }
        let tools: Vec<String> = given
            .tools()
            .iter()
            .filter(|tool| connector.tools().contains(tool))
            .filter(|tool| confirmed_tools.contains(&(connector.id(), (*tool).clone())))
            .map(ToString::to_string)
            .collect();
        if tools.is_empty() {
            continue;
        }
        turn.connectors.push(KitConnector {
            name: connector.name().to_string(),
            start: connector.start().clone(),
            tools,
        });
    }
    Ok(turn)
}

/// The environment variables named by any connector of the library of a
/// project, each once, in order. They hold the manager's secrets, so a
/// turn of the project starts without them, whatever it was given: a
/// specialist that may run a command could otherwise read them. The
/// runtime keeps only those Claude Code itself must have.
///
/// # Errors
///
/// Returns an error if storage fails.
pub fn variables_of_the_library(
    kits: &dyn KitRepository,
    project_id: ProjectId,
) -> Result<Vec<VariableName>, ApplicationError> {
    let named: BTreeSet<VariableName> = kits
        .connectors_of_project(project_id)?
        .iter()
        .flat_map(|connector| connector.start().variables().into_iter().cloned())
        .collect();
    Ok(named.into_iter().collect())
}

/// What this computer has of a skill.
enum CopyHere {
    /// No copy.
    Missing,
    /// A copy that fails the check, and why.
    Refused(String),
    /// A copy that passes it.
    Usable(KeptSkill),
}

/// The copy of a skill this computer keeps, checked: it must hold exactly
/// what Teamwright wrote, no link and no more than a skill may hold, and
/// its `SKILL.md` must still pass the check and name the same skill. The
/// copy is read again for every turn, so that a copy changed on the disk
/// is checked again.
fn copy_here(shelf: &dyn SkillShelf, skill: &Skill) -> CopyHere {
    let Some(kept) = shelf.kept(skill.id()) else {
        return CopyHere::Missing;
    };
    if let Err(problem) = check_kept_layout(&kept.layout, skill.name()) {
        return CopyHere::Refused(problem);
    }
    let file = match SkillFile::check(&kept.skill_text, &kept.skill_folder_name) {
        Ok(file) => file,
        Err(DomainError::NotASkill { reason }) => {
            return CopyHere::Refused(format!("its copy changed: {reason}"));
        }
        Err(other) => return CopyHere::Refused(other.to_string()),
    };
    if file.name() != skill.name() || kept.skill_folder_name != skill.name().as_str() {
        return CopyHere::Refused(format!("its copy is of another skill, {}", file.name()));
    }
    CopyHere::Usable(kept)
}

/// Whether the manager gave a specialist a connector that is handed a
/// secret in an environment variable, whether or not this computer has the
/// variable now. Such a specialist is given no command (decision 0038): a
/// command could read the secret, wherever the program that runs the turn
/// keeps it.
///
/// # Errors
///
/// Returns an error if storage fails.
pub fn kit_names_a_secret(
    kits: &dyn KitRepository,
    member: &Member,
) -> Result<bool, ApplicationError> {
    let Some(kit) = kits.kit_of(member.id())? else {
        return Ok(false);
    };
    if kit.connectors().is_empty() {
        return Ok(false);
    }
    let connectors = kits.connectors_of_project(member.project_id())?;
    Ok(kit.connectors().iter().any(|given| {
        connectors
            .iter()
            .find(|connector| connector.id() == given.connector_id())
            .is_some_and(|connector| !connector.start().variables().is_empty())
    }))
}

/// The variables of a connector that are not set on this computer.
fn missing_variables(shelf: &dyn SkillShelf, connector: &Connector) -> Vec<VariableName> {
    connector
        .start()
        .variables()
        .into_iter()
        .filter(|variable| !shelf.has_variable(variable))
        .cloned()
        .collect()
}

/// A kit with only the skills and connectors the library has, and of each
/// connector only the tools it lists. What another computer removed while
/// this one gave it is dropped here, without a write.
fn known_part(kit: &MemberKit, skills: &[Skill], connectors: &[Connector]) -> MemberKit {
    let known_skills = kit
        .skills()
        .iter()
        .copied()
        .filter(|id| skills.iter().any(|skill| skill.id() == *id))
        .collect();
    let known_connectors = kit
        .connectors()
        .iter()
        .filter_map(|given| {
            let connector = connectors
                .iter()
                .find(|connector| connector.id() == given.connector_id())?;
            let tools = given
                .tools()
                .iter()
                .filter(|tool| connector.tools().contains(tool))
                .cloned()
                .collect();
            GivenConnector::new(given.connector_id(), tools).ok()
        })
        .collect();
    MemberKit::new(
        kit.member_id(),
        kit.project_id(),
        known_skills,
        known_connectors,
    )
    .unwrap_or_else(|_| MemberKit::empty(kit.member_id(), kit.project_id()))
}

fn parse_all<T>(
    texts: &[String],
    parse: impl Fn(&str) -> Result<T, DomainError>,
) -> Result<Vec<T>, DomainError> {
    texts
        .iter()
        .filter(|text| !text.trim().is_empty())
        .map(|text| parse(text))
        .collect()
}

fn not_a_skill(reason: &str) -> ApplicationError {
    DomainError::NotASkill {
        reason: reason.to_owned(),
    }
    .into()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Keeps the libraries and the kits in memory, for tests.
#[derive(Debug, Default)]
pub struct InMemoryKitRepository {
    skills: Mutex<BTreeMap<SkillId, Skill>>,
    connectors: Mutex<BTreeMap<ConnectorId, Connector>>,
    kits: Mutex<BTreeMap<MemberId, MemberKit>>,
}

impl KitRepository for InMemoryKitRepository {
    fn skills_of_project(&self, project_id: ProjectId) -> Result<Vec<Skill>, StorageError> {
        Ok(lock(&self.skills)
            .values()
            .filter(|skill| skill.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn connectors_of_project(&self, project_id: ProjectId) -> Result<Vec<Connector>, StorageError> {
        Ok(lock(&self.connectors)
            .values()
            .filter(|connector| connector.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn kits_of_project(&self, project_id: ProjectId) -> Result<Vec<MemberKit>, StorageError> {
        Ok(lock(&self.kits)
            .values()
            .filter(|kit| kit.project_id() == project_id)
            .cloned()
            .collect())
    }

    fn kit_of(&self, member_id: MemberId) -> Result<Option<MemberKit>, StorageError> {
        Ok(lock(&self.kits).get(&member_id).cloned())
    }

    fn save_skill(&self, skill: &Skill) -> Result<(), StorageError> {
        lock(&self.skills).insert(skill.id(), skill.clone());
        Ok(())
    }

    fn save_connector(&self, connector: &Connector) -> Result<(), StorageError> {
        lock(&self.connectors).insert(connector.id(), connector.clone());
        Ok(())
    }

    fn save_kit(&self, kit: &MemberKit) -> Result<(), StorageError> {
        lock(&self.kits).insert(kit.member_id(), kit.clone());
        Ok(())
    }

    fn remove_skill(&self, id: SkillId, kits: &[MemberKit]) -> Result<(), StorageError> {
        for kit in kits {
            self.save_kit(kit)?;
        }
        lock(&self.skills).remove(&id);
        Ok(())
    }

    fn remove_connector(&self, id: ConnectorId, kits: &[MemberKit]) -> Result<(), StorageError> {
        for kit in kits {
            self.save_kit(kit)?;
        }
        lock(&self.connectors).remove(&id);
        Ok(())
    }
}

/// Keeps what the manager confirmed in memory, for tests.
#[derive(Debug, Default)]
pub struct InMemoryConnectorTrust {
    definitions: Mutex<BTreeMap<ConnectorId, String>>,
    tools: Mutex<BTreeMap<MemberId, BTreeSet<(ConnectorId, ToolName)>>>,
}

impl ConnectorTrust for InMemoryConnectorTrust {
    fn confirmed_definition(&self, connector: ConnectorId) -> Result<Option<String>, StorageError> {
        Ok(lock(&self.definitions).get(&connector).cloned())
    }

    fn confirm_definition(
        &self,
        connector: ConnectorId,
        fingerprint: &str,
    ) -> Result<(), StorageError> {
        lock(&self.definitions).insert(connector, fingerprint.to_owned());
        Ok(())
    }

    fn forget_definition(&self, connector: ConnectorId) -> Result<(), StorageError> {
        lock(&self.definitions).remove(&connector);
        Ok(())
    }

    fn confirmed_tools(
        &self,
        member: MemberId,
    ) -> Result<BTreeSet<(ConnectorId, ToolName)>, StorageError> {
        Ok(lock(&self.tools).get(&member).cloned().unwrap_or_default())
    }

    fn confirm_tools(
        &self,
        member: MemberId,
        tools: &BTreeSet<(ConnectorId, ToolName)>,
    ) -> Result<(), StorageError> {
        lock(&self.tools).insert(member, tools.clone());
        Ok(())
    }
}

/// A shelf that pretends, for tests: folders it is told about, the copies
/// it made, and the variables that are set.
#[derive(Debug, Default)]
pub struct PretendShelf {
    folders: Mutex<BTreeMap<PathBuf, SkillFolder>>,
    kept: Mutex<BTreeMap<SkillId, KeptSkill>>,
    variables: Mutex<BTreeSet<String>>,
}

impl PretendShelf {
    /// Makes a folder exist, with this `SKILL.md` and nothing else.
    pub fn put_folder(&self, path: impl Into<PathBuf>, skill_text: &str) {
        let path = path.into();
        let folder_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        lock(&self.folders).insert(
            path,
            SkillFolder {
                folder_name,
                skill_text: Some(skill_text.to_owned()),
                files: 1,
                bytes: skill_text.len() as u64,
                has_links: false,
            },
        );
    }

    /// Makes a folder exist as it is described.
    pub fn put_described_folder(&self, path: impl Into<PathBuf>, folder: SkillFolder) {
        lock(&self.folders).insert(path.into(), folder);
    }

    /// Sets an environment variable.
    pub fn set_variable(&self, name: &str) {
        lock(&self.variables).insert(name.to_owned());
    }

    /// Changes the kept copy of a skill, as an edit on the disk would.
    pub fn change_kept(&self, skill: SkillId, skill_text: &str) {
        if let Some(kept) = lock(&self.kept).get_mut(&skill) {
            skill_text.clone_into(&mut kept.skill_text);
        }
    }

    /// Changes what the kept copy of a skill holds, as a change on the disk
    /// would.
    pub fn change_kept_layout(&self, skill: SkillId, change: impl FnOnce(&mut KeptLayout)) {
        if let Some(kept) = lock(&self.kept).get_mut(&skill) {
            change(&mut kept.layout);
        }
    }

    /// Forgets every copy, as another computer that never kept them.
    pub fn clear_kept(&self) {
        lock(&self.kept).clear();
    }
}

impl SkillShelf for PretendShelf {
    fn read_folder(&self, folder: &Path) -> Result<SkillFolder, ChosenFolderProblem> {
        lock(&self.folders)
            .get(folder)
            .cloned()
            .ok_or(ChosenFolderProblem::Missing)
    }

    fn keep(&self, from: &Path, skill: &Skill) -> Result<(), StorageError> {
        let found = self.read_folder(from).map_err(|problem| {
            StorageError::new(format!("{}: {}", from.display(), problem.message()))
        })?;
        lock(&self.kept).insert(
            skill.id(),
            KeptSkill {
                folder: PathBuf::from("shelf").join(skill.id().to_string()),
                skill_folder_name: skill.name().to_string(),
                layout: KeptLayout {
                    top: vec![MANIFEST_FOLDER.to_owned(), SKILLS_FOLDER.to_owned()],
                    manifest: vec![MANIFEST_FILE.to_owned()],
                    skills: vec![skill.name().to_string()],
                    files: found.files,
                    bytes: found.bytes,
                    has_links: found.has_links,
                },
                skill_text: found.skill_text.unwrap_or_default(),
            },
        );
        Ok(())
    }

    fn kept(&self, skill: SkillId) -> Option<KeptSkill> {
        lock(&self.kept).get(&skill).cloned()
    }

    fn forget(&self, skill: SkillId) -> Result<(), StorageError> {
        lock(&self.kept).remove(&skill);
        Ok(())
    }

    fn has_variable(&self, name: &VariableName) -> bool {
        lock(&self.variables).contains(name.as_str())
    }
}

#[cfg(test)]
mod tests;
