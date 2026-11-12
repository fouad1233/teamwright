//! [`ClaudeCode`]: the agent runtime on the user's installed Claude Code.

use std::path::{Path, PathBuf};
use std::time::Duration;

use teamwright_application::{
    AgentEvent, AgentRuntime, AgentRuntimeError, Installation, ProgramVersion, RunningTurn,
    SessionId, SignIn, TurnRequest,
};

use crate::arguments::{
    FIRST_VERSION_WITH_DESKTOP, desktop_arguments, sign_in_arguments, turn_arguments,
    version_arguments,
};
use crate::environment::Environment;
use crate::locate::Locator;
use crate::process::{command, run_briefly, start_turn};
use crate::sign_in::parse_sign_in;
use crate::team_bus::TeamBus;

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
    /// its arguments. A turn that may ask the manager is then given the
    /// bus and its tool. Without it, such a turn runs without the tool.
    #[must_use]
    pub fn with_team_bus(mut self, program: impl Into<PathBuf>, arguments: Vec<String>) -> Self {
        self.team_bus = Some(TeamBus {
            program: program.into(),
            arguments,
        });
        self
    }

    /// The team bus a turn is given: the one this runtime knows, if the
    /// turn may ask the manager.
    fn team_bus_for(&self, request: &TurnRequest) -> Option<&TeamBus> {
        self.team_bus.as_ref().filter(|_| request.may_ask_manager)
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
        request: TurnRequest,
        listener: Box<dyn FnMut(AgentEvent) + Send>,
    ) -> Result<Box<dyn RunningTurn>, AgentRuntimeError> {
        let readable_folders = request
            .readable_folders
            .iter()
            .map(|folder| full_folder(folder))
            .collect::<Result<Vec<_>, _>>()?;
        let arguments = turn_arguments(&request, &readable_folders, self.team_bus_for(&request))?;
        let working_folder = full_folder(&request.working_folder)?;
        let program = self.program()?;
        refuse_a_script(&program)?;
        let turn = start_turn(
            command(
                &program,
                &arguments,
                Some(&working_folder),
                &self.environment,
            ),
            request.prompt,
            listener,
        )?;
        Ok(Box::new(turn))
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
    fn a_turn_gets_the_team_bus_only_if_it_may_ask_and_the_bus_is_known() {
        use teamwright_application::{PermissionProfile, SessionChoice};

        let mut request = TurnRequest {
            working_folder: PathBuf::from("work"),
            prompt: "Hello".to_owned(),
            session: SessionChoice::New,
            model: None,
            effort: None,
            extra_instructions: None,
            permissions: PermissionProfile::specialist(),
            may_ask_manager: true,
            readable_folders: Vec::new(),
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
        request.may_ask_manager = false;
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
            may_ask_manager: false,
            readable_folders: vec![folder.path().join("missing")],
        };

        let refusal = ClaudeCode::new()
            .with_program(folder.path().join("claude.exe"))
            .start_turn(request, Box::new(|_| {}))
            .err()
            .unwrap();

        assert_eq!(refusal.code(), "agent_invalid_request");
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
            may_ask_manager: true,
            readable_folders: Vec::new(),
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
