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
        let arguments = turn_arguments(&request)?;
        let working_folder = full_folder(&request.working_folder)?;
        let program = self.program()?;
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

/// The full, long form of a folder that exists.
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
    let resolved = std::fs::canonicalize(folder).map_err(|_| not_there())?;
    if !resolved.is_dir() {
        return Err(not_there());
    }
    Ok(without_verbatim_prefix(resolved))
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
}
