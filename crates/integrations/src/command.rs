//! Runs a short command with a time limit. Arguments are handed over as
//! arguments, never through a shell, so the text of a task cannot become
//! a command.

use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// What a command that ended printed.
#[derive(Debug)]
pub struct Output {
    /// Whether it ended with success.
    pub succeeded: bool,
    /// Its standard output.
    pub stdout: String,
    /// Its standard error.
    pub stderr: String,
}

/// Why a command gave no output.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// It could not be started.
    NotStarted(String),
    /// It ran past its time limit and was stopped.
    TimedOut,
}

/// Runs `program` with `arguments` and waits at most `limit` for it.
pub fn run<I, S>(
    program: &Path,
    arguments: I,
    folder: Option<&Path>,
    limit: Duration,
) -> Result<Output, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // No question may wait for an answer nobody can give, and no colour
        // codes may get into what is read.
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
        .env("NO_COLOR", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    hide_console_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| Failure::NotStarted(error.to_string()))?;

    // The output is read while the command runs, so that a full pipe
    // cannot keep it from ending.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let read_out = thread::spawn(move || read_all(stdout.as_mut()));
    let read_err = thread::spawn(move || read_all(stderr.as_mut()));

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < limit => thread::sleep(Duration::from_millis(25)),
            _ => {
                // Already ended, or cannot be asked: either way it is
                // stopped.
                let _ = child.kill();
                let _ = child.wait();
                return Err(Failure::TimedOut);
            }
        }
    };
    Ok(Output {
        succeeded: status.success(),
        stdout: read_out.join().unwrap_or_default(),
        stderr: read_err.join().unwrap_or_default(),
    })
}

fn read_all(source: Option<&mut impl Read>) -> String {
    let mut bytes = Vec::new();
    if let Some(source) = source {
        let _ = source.read_to_end(&mut bytes);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    /// The process creation flag `CREATE_NO_WINDOW` of the Windows API.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_that_is_not_there_is_not_started() {
        let failure = run(
            Path::new("no-such-program-of-teamwright"),
            ["--version"],
            None,
            Duration::from_secs(5),
        )
        .unwrap_err();
        assert!(matches!(failure, Failure::NotStarted(_)));
    }
}
