//! Starting Claude Code, feeding it, reading it and stopping it.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use teamwright_application::{AgentEvent, AgentRuntimeError, RunningTurn, TurnOutcome};

use crate::decode::{Decoder, ProcessEnd};
use crate::environment::Environment;

/// How many of the last lines of standard error are kept for messages.
const ERROR_LINES_KEPT: usize = 10;

/// How often a process that has closed its output is asked whether it has
/// ended.
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// Prepares a command: the program, its arguments, its folder and exactly
/// the given environment. On Windows the program gets no console window,
/// because Teamwright is a windowed program and every turn would otherwise
/// flash one open.
pub fn command(
    program: &Path,
    arguments: &[OsString],
    folder: Option<&Path>,
    environment: &Environment,
) -> Command {
    let mut command = Command::new(program);
    command.args(arguments).env_clear().envs(environment.iter());
    if let Some(folder) = folder {
        command.current_dir(folder);
    }
    hide_console_window(&mut command);
    command
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

/// What a short-lived command printed.
#[derive(Debug)]
pub struct Finished {
    /// Its exit code, if it ended by itself and had one.
    pub code: Option<i32>,
    /// Everything it wrote to standard output.
    pub output: String,
    /// The last lines it wrote to standard error.
    pub error_lines: Vec<String>,
}

/// Runs a command that is expected to end soon, and collects what it
/// writes.
///
/// If it is still running after the time limit, it is ended when
/// `end_when_late` is set and left running otherwise; `Ok(None)` says that
/// it had not finished.
///
/// # Errors
///
/// Returns [`AgentRuntimeError::CouldNotRun`] if the program cannot be
/// started.
pub fn run_briefly(
    mut command: Command,
    time_limit: Duration,
    end_when_late: bool,
) -> Result<Option<Finished>, AgentRuntimeError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(could_not_start)?;

    let output = child.stdout.take().map(|mut stdout| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            // A read error leaves what was read so far, which is all there
            // is to have.
            let _ = stdout.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        })
    });
    let errors = child.stderr.take().map(collect_error_lines);

    let deadline = Instant::now() + time_limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(EXIT_POLL_INTERVAL),
            Ok(None) => {
                if end_when_late {
                    // It may have ended in the meantime; then there is
                    // nothing to end.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(AgentRuntimeError::CouldNotRun {
                        message: format!(
                            "it did not answer within {} seconds",
                            time_limit.as_secs()
                        ),
                    });
                }
                return Ok(None);
            }
            Err(error) => {
                return Err(AgentRuntimeError::CouldNotRun {
                    message: format!("its state could not be read: {error}"),
                });
            }
        }
    };

    Ok(Some(Finished {
        code: status.code(),
        output: output
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default(),
        error_lines: errors
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default(),
    }))
}

/// Starts the process of a turn, gives it the prompt and follows it on a
/// thread of its own. The decoder reads its output; it is made for the
/// turn, because what it accepts depends on the tools the turn was given.
///
/// # Errors
///
/// Returns [`AgentRuntimeError::CouldNotRun`] if the program cannot be
/// started.
pub fn start_turn(
    mut command: Command,
    prompt: String,
    mut decoder: Decoder,
    mut listener: Box<dyn FnMut(AgentEvent) + Send>,
) -> Result<TurnProcess, AgentRuntimeError> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(could_not_start)?;

    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let errors = child.stderr.take().map(collect_error_lines);

    // The prompt is written on a thread of its own: a long prompt could fill
    // the pipe while the program is already writing output that nobody
    // reads yet. Closing standard input tells the program the prompt is
    // complete.
    if let Some(mut stdin) = stdin {
        thread::spawn(move || {
            // If the program ended early, the failure shows in its output.
            let _ = stdin.write_all(prompt.as_bytes());
        });
    }

    let child = Arc::new(Mutex::new(child));
    let stopped = Arc::new(AtomicBool::new(false));
    let follower = {
        let child = Arc::clone(&child);
        let stopped = Arc::clone(&stopped);
        thread::spawn(move || {
            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout);
                let mut bytes = Vec::new();
                // Read as bytes: a line that is not valid UTF-8 must not end
                // the reading, or the program would block on a full pipe.
                while let Ok(read) = reader.read_until(b'\n', &mut bytes) {
                    if read == 0 {
                        break;
                    }
                    for event in decoder.decode_line(&String::from_utf8_lossy(&bytes)) {
                        listener(event);
                    }
                    bytes.clear();
                }
            }
            let code = wait_for_exit(&child);
            let error_lines = errors
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default();
            let end = if stopped.load(Ordering::SeqCst) {
                ProcessEnd::Stopped
            } else {
                ProcessEnd::Exited { code, error_lines }
            };
            let outcome = decoder.finish(end);
            listener(AgentEvent::TurnEnded(outcome.clone()));
            outcome
        })
    };

    Ok(TurnProcess {
        child,
        stopped,
        follower: Mutex::new(Some(follower)),
    })
}

/// A turn whose process is running or has run.
#[derive(Debug)]
pub struct TurnProcess {
    child: Arc<Mutex<Child>>,
    stopped: Arc<AtomicBool>,
    follower: Mutex<Option<JoinHandle<TurnOutcome>>>,
}

impl RunningTurn for TurnProcess {
    fn stop(&self) {
        if self.is_finished() {
            return;
        }
        self.stopped.store(true, Ordering::SeqCst);
        // Ending a process that has just ended fails harmlessly.
        let _ = lock(&self.child).kill();
    }

    fn is_finished(&self) -> bool {
        lock(&self.follower)
            .as_ref()
            .is_none_or(JoinHandle::is_finished)
    }

    fn wait(self: Box<Self>) -> Result<TurnOutcome, AgentRuntimeError> {
        let follower = lock(&self.follower).take();
        follower
            .and_then(|follower| follower.join().ok())
            .ok_or_else(|| AgentRuntimeError::CouldNotRun {
                message: "the thread that followed the turn failed".to_owned(),
            })
    }
}

/// Waits for a process whose output has ended. The lock is taken only for
/// each short question, so that `stop` is never kept waiting.
fn wait_for_exit(child: &Mutex<Child>) -> Option<i32> {
    loop {
        match lock(child).try_wait() {
            Ok(Some(status)) => return status.code(),
            Ok(None) => {}
            Err(_) => return None,
        }
        thread::sleep(EXIT_POLL_INTERVAL);
    }
}

fn collect_error_lines(stderr: ChildStderr) -> JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let mut kept = VecDeque::with_capacity(ERROR_LINES_KEPT);
        let mut reader = BufReader::new(stderr);
        let mut bytes = Vec::new();
        // Read as bytes: a message in the system's own code page must not
        // end the reading.
        while let Ok(read) = reader.read_until(b'\n', &mut bytes) {
            if read == 0 {
                break;
            }
            if kept.len() == ERROR_LINES_KEPT {
                kept.pop_front();
            }
            kept.push_back(String::from_utf8_lossy(&bytes).trim_end().to_owned());
            bytes.clear();
        }
        kept.into_iter().collect()
    })
}

fn could_not_start(error: std::io::Error) -> AgentRuntimeError {
    AgentRuntimeError::CouldNotRun {
        message: format!("the program could not be started: {error}"),
    }
}

/// Locks a mutex, and keeps working even if another thread panicked while it
/// held the lock: a process handle stays usable whatever happened.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
