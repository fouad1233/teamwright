//! [`ClaudeCode`]: the agent runtime on the user's installed Claude Code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use teamwright_application::{
    AgentEvent, AgentRuntime, AgentRuntimeError, ConnectorStart, Installation, ProgramVersion,
    RunningTurn, SessionId, SignIn, TeamTool, TurnKit, TurnOutcome, TurnRequest,
};

use crate::arguments::{
    FIRST_VERSION_WITH_DESKTOP, desktop_arguments, sign_in_arguments, turn_arguments_with,
    version_arguments,
};
use crate::commands::environment_for_commands;
use crate::connector_launcher::{SecretsFile, is_base_variable, make_private_folder};
use crate::decode::Decoder;
use crate::environment::Environment;
use crate::kit::{ConnectorLauncher, ConnectorStarts};
use crate::locate::Locator;
use crate::memory_file::MemoryFile;
use crate::process::{command, run_briefly, start_turn};
use crate::sign_in::parse_sign_in;
use crate::team_bus::TeamBus;

/// Variables Claude Code itself may need to reach its service, which stay
/// in the environment of a turn even when a connector names them, besides
/// those every program needs ([`is_base_variable`]). A connector that names
/// one gets it from the launcher too.
const KEPT_FOR_THE_PROGRAM: &[&str] = &[
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
];

/// How long a question such as the version or the sign-in state may take.
const QUESTION_TIME_LIMIT: Duration = Duration::from_secs(30);

/// How long to wait for the program to hand a session to the desktop app.
/// If it is still running after that, it is left to finish on its own.
const DESKTOP_TIME_LIMIT: Duration = Duration::from_secs(20);

/// Runs team members' sessions on the user's installed Claude Code.
#[derive(Debug, Clone)]
pub struct ClaudeCode {
    environment: Environment,
    program: Option<PathBuf>,
    team_bus: Option<TeamBus>,
    connector_launcher: Option<ConnectorLauncher>,
}

impl Default for ClaudeCode {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeCode {
    /// Uses the program found on the search path or in the documented
    /// places, and starts it with the environment of this process, without
    /// the marks of a running Claude Code session.
    #[must_use]
    pub fn new() -> Self {
        Self {
            environment: Environment::inherited(),
            program: None,
            team_bus: None,
            connector_launcher: None,
        }
    }

    /// Starts the program with exactly this environment. The program is
    /// also searched for with its search path and home folder.
    #[must_use]
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    /// Uses the program at this place instead of searching for it.
    #[must_use]
    pub fn with_program(mut self, program: impl Into<PathBuf>) -> Self {
        self.program = Some(program.into());
        self
    }

    /// Says how the team bus is started: the program that serves it and
    /// its arguments, to which the tools of a turn are added. A turn with
    /// team tools is then given the bus. Without it, such a turn runs
    /// without them.
    #[must_use]
    pub fn with_team_bus(mut self, program: impl Into<PathBuf>, arguments: Vec<String>) -> Self {
        self.team_bus = Some(TeamBus {
            program: program.into(),
            arguments,
        });
        self
    }

    /// Says how the programs of connectors are started: through this
    /// program and these arguments, in the mode of the
    /// [`connector_launcher`](crate::connector_launcher), in `folder`, an
    /// empty folder of Teamwright's own, with the values of their variables
    /// in a file of `secrets_folder` while the turn runs. Neither folder may
    /// be one a session reads. Without it, a turn given a connector that is
    /// a program is refused.
    #[must_use]
    pub fn with_connector_launcher(
        mut self,
        program: impl Into<PathBuf>,
        arguments: Vec<String>,
        folder: impl Into<PathBuf>,
        secrets_folder: impl Into<PathBuf>,
    ) -> Self {
        self.connector_launcher = Some(ConnectorLauncher {
            program: program.into(),
            arguments,
            folder: folder.into(),
            secrets_folder: secrets_folder.into(),
        });
        self
    }

    /// Writes, for each connector of the turn that is a program and is
    /// handed variables, their values to a file of its own, and makes the
    /// empty folder connectors start in. Nothing without a launcher: such
    /// a connector is then refused.
    fn prepare_connectors(
        &self,
        kit: &TurnKit,
    ) -> Result<Vec<(String, SecretsFile)>, AgentRuntimeError> {
        let Some(launcher) = &self.connector_launcher else {
            return Ok(Vec::new());
        };
        let could_not = |message: String| AgentRuntimeError::CouldNotRun { message };
        let has_programs = kit
            .connectors
            .iter()
            .any(|connector| matches!(connector.start, ConnectorStart::Program { .. }));
        if has_programs {
            make_private_folder(&launcher.folder).map_err(|error| {
                could_not(format!(
                    "the folder connectors start in could not be made: {error}"
                ))
            })?;
        }
        let mut files = Vec::new();
        for connector in &kit.connectors {
            let ConnectorStart::Program { variables, .. } = &connector.start else {
                continue;
            };
            if variables.is_empty() {
                continue;
            }
            let mut values = BTreeMap::new();
            for variable in variables {
                let value = self
                    .environment
                    .get(variable.as_str())
                    .and_then(|value| value.to_str())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        could_not(format!(
                            "the variable {variable} of the connector {} is not set as text on \
                             this computer",
                            connector.name
                        ))
                    })?;
                values.insert(variable.to_string(), value.to_owned());
            }
            let file =
                SecretsFile::write_in(&launcher.secrets_folder, &values).map_err(|error| {
                    could_not(format!(
                        "the variables of the connector {} could not be written for it: {error}",
                        connector.name
                    ))
                })?;
            files.push((connector.name.clone(), file));
        }
        Ok(files)
    }

    /// The team bus a turn is given: the one this runtime knows, if the
    /// turn has team tools.
    fn team_bus_for(&self, request: &TurnRequest) -> Option<&TeamBus> {
        self.team_bus
            .as_ref()
            .filter(|_| !request.team_tools.is_empty())
    }

    fn program(&self) -> Result<PathBuf, AgentRuntimeError> {
        match &self.program {
            Some(program) if program.is_file() => Ok(program.clone()),
            Some(_) => Err(AgentRuntimeError::NotInstalled),
            None => Locator::for_environment(&self.environment)
                .find()
                .ok_or(AgentRuntimeError::NotInstalled),
        }
    }

    fn version(&self, program: &Path) -> Result<ProgramVersion, AgentRuntimeError> {
        let finished = run_briefly(
            command(program, &version_arguments(), None, &self.environment),
            QUESTION_TIME_LIMIT,
            true,
        )?
        .ok_or_else(|| AgentRuntimeError::CouldNotRun {
            message: "it did not tell its version".to_owned(),
        })?;
        ProgramVersion::find_in(&finished.output).ok_or_else(|| {
            AgentRuntimeError::UnexpectedAnswer {
                message: "the version was not a version number".to_owned(),
            }
        })
    }
}

impl AgentRuntime for ClaudeCode {
    fn installation(&self) -> Result<Installation, AgentRuntimeError> {
        let program = self.program()?;
        let version = self.version(&program)?;
        Ok(Installation { program, version })
    }

    fn sign_in(&self) -> Result<SignIn, AgentRuntimeError> {
        let program = self.program()?;
        let finished = run_briefly(
            command(&program, &sign_in_arguments(), None, &self.environment),
            QUESTION_TIME_LIMIT,
            true,
        )?
        .ok_or_else(|| AgentRuntimeError::CouldNotRun {
            message: "it did not tell whether the user is signed in".to_owned(),
        })?;
        // Exit code 1 with an answer means "signed out"; any other failure
        // shows as an answer that cannot be read.
        parse_sign_in(&finished.output)
    }

    fn start_turn(
        &self,
        mut request: TurnRequest,
        listener: Box<dyn FnMut(AgentEvent) + Send>,
    ) -> Result<Box<dyn RunningTurn>, AgentRuntimeError> {
        // The folder of a skill is named in its full form, like a folder the
        // session may read, which it also is.
        for skill in &mut request.kit.skills {
            skill.folder = full_folder(&skill.folder)?;
        }
        let readable_folders = request
            .readable_folders
            .iter()
            .map(|folder| full_folder(folder))
            .collect::<Result<Vec<_>, _>>()?;
        // The memory a search of the turn reads is in a file of its own,
        // which lives as long as the turn (decision 0036).
        let memory = memory_file_for(&request)?;
        let team_bus = self.team_bus_for(&request).map(|bus| match &memory {
            Some(file) => bus.with_memory_file(file.path()),
            None => bus.clone(),
        });
        // The connectors that are programs are started through the launcher,
        // with the values of their variables in files that live as long as
        // the turn (decision 0039, note of 21 November 2026).
        let secrets = self.prepare_connectors(&request.kit)?;
        let starts = ConnectorStarts {
            launcher: self.connector_launcher.as_ref(),
            secrets: secrets
                .iter()
                .map(|(name, file)| (name.clone(), file.path().to_owned()))
                .collect(),
        };
        let arguments =
            turn_arguments_with(&request, &readable_folders, team_bus.as_ref(), &starts)?;
        let environment = environment_of_turn(&self.environment, &request)?;
        let working_folder = full_folder(&request.working_folder)?;
        let program = self.program()?;
        refuse_a_script(&program)?;
        // The decoder checks a proposal against the team the bus was told
        // of, and a message against the teammates, as the bus does.
        let team = request
            .team_tools
            .iter()
            .find_map(|tool| match tool {
                TeamTool::ProposePlan { team } => Some(team.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let teammates = request
            .team_tools
            .iter()
            .find_map(|tool| match tool {
                TeamTool::SendMessage { teammates } => Some(teammates.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let commands_askable = request
            .team_tools
            .iter()
            .any(|tool| matches!(tool, TeamTool::AskPermission { commands: true }));
        let turn = start_turn(
            command(&program, &arguments, Some(&working_folder), &environment),
            request.prompt,
            Decoder::for_team(team)
                .with_teammates(teammates)
                .with_commands_askable(commands_askable),
            listener,
        )?;
        Ok(if memory.is_none() && secrets.is_empty() {
            Box::new(turn)
        } else {
            Box::new(TurnWithFiles {
                turn: Box::new(turn),
                _memory: memory,
                _secrets: secrets,
            })
        })
    }

    fn why_no_commands(&self) -> Option<teamwright_domain::NoCommands> {
        crate::commands::why_no_commands(&self.environment)
    }

    fn open_in_desktop_app(
        &self,
        session: &SessionId,
        working_folder: &Path,
    ) -> Result<(), AgentRuntimeError> {
        let program = self.program()?;
        let installed = self.version(&program)?;
        if installed < FIRST_VERSION_WITH_DESKTOP {
            return Err(AgentRuntimeError::VersionTooOld {
                installed,
                needed: FIRST_VERSION_WITH_DESKTOP,
            });
        }
        let folder = full_folder(working_folder).ok();
        let finished = run_briefly(
            command(
                &program,
                &desktop_arguments(session),
                folder.as_deref(),
                &self.environment,
            ),
            DESKTOP_TIME_LIMIT,
            false,
        )?;
        match finished {
            Some(finished) if finished.code != Some(0) => Err(AgentRuntimeError::CouldNotRun {
                message: match finished.error_lines.last() {
                    Some(line) => format!("the desktop app could not be opened: {line}"),
                    None => "the desktop app could not be opened".to_owned(),
                },
            }),
            _ => Ok(()),
        }
    }
}

/// The file that holds the memory of the project for a turn that may
/// search it, written now. Nothing for a turn without `search_memory`.
fn memory_file_for(request: &TurnRequest) -> Result<Option<MemoryFile>, AgentRuntimeError> {
    let Some(entries) = request.team_tools.iter().find_map(|tool| match tool {
        TeamTool::SearchMemory { entries } => Some(entries),
        _ => None,
    }) else {
        return Ok(None);
    };
    MemoryFile::write(entries)
        .map(Some)
        .map_err(|error| AgentRuntimeError::CouldNotRun {
            message: format!("the memory of the project could not be written for it: {error}"),
        })
}

/// The environment a turn is started with. A turn that may run a command
/// gets the base set and nothing else, because a sandboxed command inherits
/// the environment of Claude Code (decision 0038, note of 22 November 2026);
/// it is refused if the manager signs in with a variable, which the turn
/// would lose or the command could read. Any other turn gets
/// [`turn_environment`].
fn environment_of_turn(
    base: &Environment,
    request: &TurnRequest,
) -> Result<Environment, AgentRuntimeError> {
    if !request.permissions.commands_in_sandbox {
        return Ok(turn_environment(base, &request.kit));
    }
    if crate::commands::sign_in_variable(base).is_some() {
        return Err(AgentRuntimeError::InvalidRequest {
            message: teamwright_domain::NoCommands::SignedInWithVariable
                .reason()
                .to_owned(),
        });
    }
    Ok(environment_for_commands(base))
}

/// The environment a turn is started with: Teamwright's own, without the
/// variables of the library of the project and of the connectors of the
/// turn. Kept are those a header of a connector of the turn needs, which
/// Claude Code fills in itself, and those Claude Code needs to start and
/// reach its service.
fn turn_environment(base: &Environment, kit: &TurnKit) -> Environment {
    let mut needed: BTreeSet<&str> = BTreeSet::new();
    let mut named: Vec<&str> = kit
        .withheld_variables
        .iter()
        .map(|variable| variable.as_str())
        .collect();
    for connector in &kit.connectors {
        match &connector.start {
            ConnectorStart::Address { .. } => needed.extend(
                connector
                    .start
                    .variables()
                    .into_iter()
                    .map(|variable| variable.as_str()),
            ),
            ConnectorStart::Program { variables, .. } => {
                named.extend(variables.iter().map(|variable| variable.as_str()));
            }
        }
    }
    let mut environment = base.clone();
    for name in named {
        let kept = needed.contains(name)
            || is_base_variable(name)
            || KEPT_FOR_THE_PROGRAM
                .iter()
                .any(|kept| kept.eq_ignore_ascii_case(name));
        if !kept {
            environment.remove(name);
        }
    }
    environment
}

/// A turn with files of its own: the memory its team bus reads, and the
/// values of the variables of its connectors. They are removed when the
/// turn is dropped, after it has ended.
struct TurnWithFiles {
    turn: Box<dyn RunningTurn>,
    _memory: Option<MemoryFile>,
    _secrets: Vec<(String, SecretsFile)>,
}

impl RunningTurn for TurnWithFiles {
    fn stop(&self) {
        self.turn.stop();
    }

    fn is_finished(&self) -> bool {
        self.turn.is_finished()
    }

    fn wait(self: Box<Self>) -> Result<TurnOutcome, AgentRuntimeError> {
        let Self {
            turn,
            _memory,
            _secrets,
        } = *self;
        turn.wait()
    }
}

/// Refuses to start a turn on a Claude Code that is a script.
///
/// Installed through npm, Claude Code on Windows is a script, `claude.cmd`.
/// Windows hands a script its arguments through the command prompt, and the
/// command prompt cannot take the instructions of a specialist as they are:
/// they have several lines. The start would fail with a message about batch
/// files. This says instead what is the matter and what to do about it.
///
/// The questions for the version and the sign-in have plain arguments and
/// work with a script, so such an installation is still found and named.
fn refuse_a_script(program: &Path) -> Result<(), AgentRuntimeError> {
    let is_script = cfg!(windows)
        && program
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
            });
    if is_script {
        return Err(AgentRuntimeError::CouldNotRun {
            message: "it is installed as a script, the way npm installs it, and a script \
                      cannot be given the instructions of a specialist. Install Claude Code \
                      with its own installer"
                .to_owned(),
        });
    }
    Ok(())
}

/// The full, long form of a folder that exists: the working folder, or a
/// folder the session may read.
///
/// Claude Code denies writing to a path that contains a Windows short name
/// such as `LONGNA~1`, as a guard against paths that dodge its rules. The
/// system's temporary folder is often given in that form, so the working
/// folder is resolved first: links are followed and short names expanded.
/// The `\\?\` prefix Windows adds to resolved paths is taken off again,
/// because programs that read the path back do not all understand it.
fn full_folder(folder: &Path) -> Result<PathBuf, AgentRuntimeError> {
    let not_there = || AgentRuntimeError::InvalidRequest {
        message: format!("the working folder {} does not exist", folder.display()),
    };
    let resolved = full_form(folder).map_err(|_| not_there())?;
    if !resolved.is_dir() {
        return Err(not_there());
    }
    Ok(resolved)
}

/// The full form of a path that exists, which is the form Claude Code knows
/// it in: links are followed, the short names of Windows are expanded, and
/// the `\\?\` prefix Windows adds to resolved paths is taken off again.
///
/// Claude Code compares paths as they are written. A folder that a session
/// was given in this form must be named to the session in this form too.
/// Named with a short name such as `LONGNA~1`, it counts as another folder,
/// outside what the session may reach, and reading it is refused.
///
/// # Errors
///
/// Returns an error if the path does not exist or cannot be reached.
pub fn full_form(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path).map(without_verbatim_prefix)
}

fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(share) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{share}"))
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        PathBuf::from(local)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use teamwright_application::PermissionProfile;

    use super::*;

    #[test]
    fn the_verbatim_prefix_is_taken_off() {
        assert_eq!(
            without_verbatim_prefix(PathBuf::from(r"\\?\C:\Users\sample\work")),
            PathBuf::from(r"C:\Users\sample\work")
        );
        assert_eq!(
            without_verbatim_prefix(PathBuf::from(r"\\?\UNC\server\share\work")),
            PathBuf::from(r"\\server\share\work")
        );
        assert_eq!(
            without_verbatim_prefix(PathBuf::from("/home/sample/work")),
            PathBuf::from("/home/sample/work")
        );
    }

    #[test]
    fn a_working_folder_is_given_in_its_full_form() {
        let folder = tempfile::tempdir().unwrap();
        let roundabout = folder.path().join("inner").join("..");
        std::fs::create_dir(folder.path().join("inner")).unwrap();
        let full = full_folder(&roundabout).unwrap();
        let text = full.to_string_lossy();
        assert!(!text.starts_with(r"\\?\"), "{text}");
        assert!(!text.contains(".."), "{text}");
        assert!(!text.contains('~'), "{text}");
        assert!(full.is_dir());
    }

    #[test]
    fn a_missing_working_folder_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let error = full_folder(&folder.path().join("missing")).unwrap_err();
        assert_eq!(error.code(), "agent_invalid_request");
    }

    #[test]
    fn a_turn_gets_the_team_bus_only_if_it_has_team_tools_and_the_bus_is_known() {
        use teamwright_application::{PermissionProfile, SessionChoice};

        let mut request = TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Hello".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: vec![TeamTool::AskManager],
            readable_folders: Vec::new(),
            kit: teamwright_application::TurnKit::default(),
        };
        let without_bus = ClaudeCode::new();
        let with_bus = ClaudeCode::new().with_team_bus("teamwright", vec!["--team-bus".to_owned()]);

        assert!(without_bus.team_bus_for(&request).is_none());
        assert_eq!(
            with_bus.team_bus_for(&request),
            Some(&TeamBus {
                program: PathBuf::from("teamwright"),
                arguments: vec!["--team-bus".to_owned()],
            })
        );
        request.team_tools = Vec::new();
        assert!(with_bus.team_bus_for(&request).is_none());
    }

    #[test]
    fn a_missing_readable_folder_is_refused_before_anything_starts() {
        use teamwright_application::{PermissionProfile, SessionChoice};

        let folder = tempfile::tempdir().unwrap();
        let request = TurnRequest {
            working_folder: folder.path().to_owned(),
            prompt: "Hello".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: Vec::new(),
            readable_folders: vec![folder.path().join("missing")],
            kit: teamwright_application::TurnKit::default(),
        };

        let refusal = ClaudeCode::new()
            .with_program(folder.path().join("claude.exe"))
            .start_turn(request, Box::new(|_| {}))
            .err()
            .unwrap();

        assert_eq!(refusal.code(), "agent_invalid_request");
    }

    #[test]
    fn a_turn_that_may_search_memory_gets_the_memory_in_a_file_and_names_it_to_the_bus() {
        use teamwright_application::{PermissionProfile, SessionChoice};

        let mut request = TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Hello".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: vec![TeamTool::AskManager],
            readable_folders: Vec::new(),
            kit: teamwright_application::TurnKit::default(),
        };
        assert!(memory_file_for(&request).unwrap().is_none());

        request.team_tools.push(TeamTool::SearchMemory {
            entries: Vec::new(),
        });
        let file = memory_file_for(&request).unwrap().unwrap();
        assert_eq!(
            crate::memory_file::entries_from(file.path()),
            Ok(Vec::new())
        );
        let bus = TeamBus {
            program: PathBuf::from("teamwright"),
            arguments: vec!["--team-bus".to_owned()],
        }
        .with_memory_file(file.path());
        assert_eq!(
            bus.arguments,
            [
                "--team-bus".to_owned(),
                "--memory".to_owned(),
                file.path().to_string_lossy().into_owned()
            ]
        );
    }

    fn connector(name: &str, start: ConnectorStart) -> teamwright_application::KitConnector {
        teamwright_application::KitConnector {
            name: name.to_owned(),
            start,
            tools: vec!["search".to_owned()],
        }
    }

    fn variable(name: &str) -> teamwright_domain::VariableName {
        teamwright_domain::VariableName::parse(name).unwrap()
    }

    #[test]
    fn a_turn_starts_without_the_secrets_of_the_library_but_with_what_claude_code_needs() {
        let base = Environment::empty()
            .with("PATH", "/usr/bin")
            .with("HTTPS_PROXY", "http://proxy:3128")
            .with("PARTS_TOKEN", "parts")
            .with("OTHER_TOKEN", "other")
            .with("TICKETS_TOKEN", "tickets")
            .with("UNRELATED", "kept");
        let kit = TurnKit {
            skills: Vec::new(),
            connectors: vec![
                connector(
                    "parts",
                    ConnectorStart::program("npx", Vec::new(), vec![variable("PARTS_TOKEN")])
                        .unwrap(),
                ),
                connector(
                    "tickets",
                    ConnectorStart::address(
                        "https://tickets.example/mcp",
                        vec![
                            teamwright_domain::SecretHeader::new(
                                "Authorization",
                                variable("TICKETS_TOKEN"),
                                true,
                            )
                            .unwrap(),
                        ],
                    )
                    .unwrap(),
                ),
            ],
            withheld_variables: [
                "PARTS_TOKEN",
                "OTHER_TOKEN",
                "TICKETS_TOKEN",
                "PATH",
                "HTTPS_PROXY",
            ]
            .map(variable)
            .to_vec(),
        };

        let environment = turn_environment(&base, &kit);

        // The program connector's secret reaches it through the launcher,
        // and another connector's not at all.
        assert_eq!(environment.get("PARTS_TOKEN"), None);
        assert_eq!(environment.get("OTHER_TOKEN"), None);
        // Claude Code fills in the header of an address itself.
        assert!(environment.get("TICKETS_TOKEN").is_some());
        // What Claude Code needs to start and reach its service stays.
        assert!(environment.get("PATH").is_some());
        assert!(environment.get("HTTPS_PROXY").is_some());
        // What no connector names is not touched by this rule.
        assert!(environment.get("UNRELATED").is_some());
    }

    #[test]
    fn a_turn_that_may_run_a_command_is_started_with_the_base_set_only() {
        let base = Environment::empty()
            .with("PATH", "/usr/bin")
            .with("HOME", "/home/manager")
            .with("GH_TOKEN", "secret")
            .with("UNRELATED", "dropped")
            .with("CLAUDECODE", "1");
        let mut request = TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Run the tests.".to_owned(),
            session: teamwright_application::SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: Vec::new(),
            readable_folders: Vec::new(),
            kit: TurnKit::default(),
        };
        request.permissions = PermissionProfile::specialist().with_granted(&[
            teamwright_domain::PermissionRule::new(
                teamwright_domain::PermissionKind::Command,
                "npm test",
            )
            .unwrap(),
        ]);
        let environment = environment_of_turn(&base, &request).unwrap();
        let names: Vec<String> = environment
            .iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["HOME", "PATH"]);

        // A turn without a command keeps what no connector names.
        request.permissions = PermissionProfile::specialist();
        let environment = environment_of_turn(&base, &request).unwrap();
        assert!(environment.get("UNRELATED").is_some());
    }

    #[test]
    fn a_turn_that_may_run_a_command_is_refused_when_signed_in_with_a_variable() {
        let base = Environment::empty()
            .with("PATH", "/usr/bin")
            .with("ANTHROPIC_API_KEY", "secret");
        let mut request = TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Run the tests.".to_owned(),
            session: teamwright_application::SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            team_tools: Vec::new(),
            readable_folders: Vec::new(),
            kit: TurnKit::default(),
        };
        request.permissions = PermissionProfile::specialist().with_granted(&[
            teamwright_domain::PermissionRule::new(
                teamwright_domain::PermissionKind::Command,
                "npm test",
            )
            .unwrap(),
        ]);
        assert!(matches!(
            environment_of_turn(&base, &request),
            Err(AgentRuntimeError::InvalidRequest { .. })
        ));
        request.permissions = PermissionProfile::specialist();
        assert!(environment_of_turn(&base, &request).is_ok());
    }

    #[test]
    fn the_values_of_a_programs_variables_are_written_for_the_turn_and_gone_after_it() {
        let ground = tempfile::tempdir().unwrap();
        let runtime = ClaudeCode::new()
            .with_environment(Environment::empty().with("PARTS_TOKEN", "t0ken"))
            .with_connector_launcher(
                "/opt/teamwright/teamwright",
                vec!["--connector".to_owned()],
                ground.path().join("start-here"),
                ground.path().join("secrets"),
            );
        let mut kit = TurnKit::default();
        kit.connectors.push(connector(
            "parts",
            ConnectorStart::program("npx", Vec::new(), vec![variable("PARTS_TOKEN")]).unwrap(),
        ));
        kit.connectors.push(connector(
            "plain",
            ConnectorStart::program("npx", Vec::new(), Vec::new()).unwrap(),
        ));

        let files = runtime.prepare_connectors(&kit).unwrap();

        assert!(ground.path().join("start-here").is_dir());
        let [(name, file)] = files.as_slice() else {
            panic!("one file was expected");
        };
        assert_eq!(name, "parts");
        let path = file.path().to_owned();
        assert!(path.starts_with(ground.path().join("secrets")));
        assert!(std::fs::read_to_string(&path).unwrap().contains("t0ken"));
        drop(files);
        assert!(!path.exists());

        // A variable that is not set is not started with an empty value.
        let unset = ClaudeCode::new()
            .with_environment(Environment::empty())
            .with_connector_launcher(
                "/opt/teamwright/teamwright",
                vec!["--connector".to_owned()],
                ground.path().join("start-here"),
                ground.path().join("secrets"),
            );
        assert!(unset.prepare_connectors(&kit).is_err());
    }

    #[test]
    fn a_program_that_is_not_a_script_is_not_refused() {
        assert!(refuse_a_script(Path::new("claude.exe")).is_ok());
        assert!(refuse_a_script(Path::new("claude")).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn a_turn_is_not_started_on_a_claude_code_that_is_a_script() {
        use teamwright_application::{PermissionProfile, SessionChoice};

        let folder = tempfile::tempdir().unwrap();
        let script = folder.path().join("claude.CMD");
        std::fs::write(&script, "@echo off\r\n").unwrap();
        let request = TurnRequest {
            working_folder: folder.path().to_owned(),
            prompt: "Hello".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: Some("Two\nlines.".to_owned()),
            permissions: PermissionProfile::specialist(),
            team_tools: vec![TeamTool::AskManager],
            readable_folders: Vec::new(),
            kit: teamwright_application::TurnKit::default(),
        };

        let refusal = ClaudeCode::new()
            .with_program(script)
            .start_turn(request, Box::new(|_| {}))
            .err()
            .unwrap();

        assert_eq!(refusal.code(), "agent_could_not_run");
        assert!(refusal.to_string().contains("installed as a script"));
    }
}
