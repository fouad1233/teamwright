//! The interface to the program that runs a team member's session.
//!
//! A team member is a session of an agent program, today the user's own
//! Claude Code ([decision 0005]). The application does not start that program
//! itself. It asks an [`AgentRuntime`] to: say whether the program is
//! installed, say whether the user is signed in to it, run one turn of a
//! session, and open a finished session in the program's desktop app.
//!
//! Everything here is plain data. How the data is turned into command-line
//! arguments and how the program's output is decoded is the business of the
//! crate that implements the trait.
//!
//! [decision 0005]: ../../../docs/decisions/0005-drive-the-users-claude-code.md

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use teamwright_domain::Timestamp;

/// Runs the sessions of team members on an agent program.
pub trait AgentRuntime: Send + Sync {
    /// Finds the installed program and its version.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::NotInstalled`] if the program cannot be
    /// found, and another error if it cannot be started or its answer is not
    /// understood.
    fn installation(&self) -> Result<Installation, AgentRuntimeError>;

    /// Asks the program whether the user is signed in.
    ///
    /// The program is asked through its own command. No credential is read,
    /// kept or passed on.
    ///
    /// # Errors
    ///
    /// Returns an error if the program cannot be found or started, or if its
    /// answer is not understood.
    fn sign_in(&self) -> Result<SignIn, AgentRuntimeError>;

    /// Starts one turn of a session and returns at once.
    ///
    /// The listener is called on another thread, once for each event, in the
    /// order the events happen. Its last call is always
    /// [`AgentEvent::TurnEnded`], also when the turn fails or is stopped.
    ///
    /// # Errors
    ///
    /// Returns an error if the request is not valid or the program cannot be
    /// found or started. Once the turn has started, failures are reported
    /// through [`AgentEvent::TurnEnded`] instead.
    fn start_turn(
        &self,
        request: TurnRequest,
        listener: Box<dyn FnMut(AgentEvent) + Send>,
    ) -> Result<Box<dyn RunningTurn>, AgentRuntimeError>;

    /// Opens a session in the agent program's desktop app, so that the
    /// manager can look at it there.
    ///
    /// The working folder is the one the session's turns ran in.
    ///
    /// # Errors
    ///
    /// Returns an error if the program cannot be found, if its version cannot
    /// open sessions in the desktop app, or if it reports a failure.
    fn open_in_desktop_app(
        &self,
        session: &SessionId,
        working_folder: &Path,
    ) -> Result<(), AgentRuntimeError>;
}

/// A turn that has been started and may still be running.
pub trait RunningTurn: Send + Sync {
    /// Stops the turn. Stopping a turn that has already ended does nothing.
    ///
    /// The listener still receives [`AgentEvent::TurnEnded`], with
    /// [`TurnFailure::Stopped`] unless the turn had finished on its own.
    fn stop(&self);

    /// Whether the turn has ended and its last event has been delivered.
    fn is_finished(&self) -> bool;

    /// Waits until the turn has ended and returns how it ended.
    ///
    /// # Errors
    ///
    /// Returns an error only if the thread that followed the turn failed,
    /// which is a bug.
    fn wait(self: Box<Self>) -> Result<TurnOutcome, AgentRuntimeError>;
}

/// Why the agent program could not be used.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentRuntimeError {
    /// The program is not installed in any of the places that were searched.
    #[error("Claude Code was not found on this computer")]
    NotInstalled,

    /// The program was found but could not be started or did not finish.
    #[error("Claude Code could not be run: {message}")]
    CouldNotRun {
        /// What went wrong, for people.
        message: String,
    },

    /// The program answered, but not in a way that is understood.
    #[error("Claude Code gave an answer that was not understood: {message}")]
    UnexpectedAnswer {
        /// What was expected, for people. It never repeats the answer, which
        /// may contain personal details.
        message: String,
    },

    /// The request cannot be carried out as it is.
    #[error("the request is not valid: {message}")]
    InvalidRequest {
        /// What is wrong with it, for people.
        message: String,
    },

    /// The installed version is too old for what was asked.
    #[error("Claude Code {installed} cannot do this; version {needed} or later is needed")]
    VersionTooOld {
        /// The version that is installed.
        installed: ProgramVersion,
        /// The first version that can do it.
        needed: ProgramVersion,
    },
}

impl AgentRuntimeError {
    /// A short, stable name for the kind of error, for interfaces.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotInstalled => "agent_not_installed",
            Self::CouldNotRun { .. } => "agent_could_not_run",
            Self::UnexpectedAnswer { .. } => "agent_unexpected_answer",
            Self::InvalidRequest { .. } => "agent_invalid_request",
            Self::VersionTooOld { .. } => "agent_version_too_old",
        }
    }
}

/// The version of the installed program, such as 2.1.322.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl ProgramVersion {
    /// Makes a version from its three numbers.
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Reads the first version number in a text such as
    /// `2.1.322 (Claude Code)`.
    #[must_use]
    pub fn find_in(text: &str) -> Option<Self> {
        text.split(|character: char| !(character.is_ascii_digit() || character == '.'))
            .find_map(|word| {
                let mut numbers = word.split('.').map(str::parse::<u32>);
                let major = numbers.next()?.ok()?;
                let minor = numbers.next()?.ok()?;
                let patch = numbers.next()?.ok()?;
                Some(Self::new(major, minor, patch))
            })
    }

    /// The first number.
    #[must_use]
    pub const fn major(self) -> u32 {
        self.major
    }

    /// The second number.
    #[must_use]
    pub const fn minor(self) -> u32 {
        self.minor
    }

    /// The third number.
    #[must_use]
    pub const fn patch(self) -> u32 {
        self.patch
    }
}

impl fmt::Display for ProgramVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// An installed agent program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    /// Where the program is.
    pub program: PathBuf,
    /// Which version it is.
    pub version: ProgramVersion,
}

/// Whether the user is signed in to the agent program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignIn {
    /// The program can be used.
    SignedIn {
        /// How the user signed in, as the program names it, for example
        /// `claude.ai` for a subscription.
        method: String,
        /// Who serves the model, as the program names it, for example
        /// `firstParty`.
        provider: String,
        /// The kind of subscription, if the program says, for example `max`.
        subscription: Option<String>,
    },
    /// The user has to sign in to the program before a team can work.
    SignedOut,
}

/// The identifier of a session of the agent program.
///
/// For Claude Code it is a UUID. Only letters, digits, `-` and `_` are
/// accepted, so that an identifier can never be mistaken for an option on a
/// command line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// The longest identifier that is accepted.
    pub const MAX_LENGTH: usize = 128;

    /// Checks and wraps an identifier.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::InvalidRequest`] if the identifier is
    /// empty, too long, starts with `-`, or contains other characters than
    /// letters, digits, `-` and `_`.
    pub fn parse(text: &str) -> Result<Self, AgentRuntimeError> {
        let acceptable = !text.is_empty()
            && text.len() <= Self::MAX_LENGTH
            && !text.starts_with('-')
            && text
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || "-_".contains(character));
        if acceptable {
            Ok(Self(text.to_owned()))
        } else {
            Err(AgentRuntimeError::InvalidRequest {
                message: format!("\"{text}\" is not a session identifier"),
            })
        }
    }

    /// The identifier as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Which session a turn belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionChoice {
    /// Starts a new session. The program chooses its identifier and reports
    /// it in [`AgentEvent::SessionStarted`].
    New,
    /// Starts a new session with an identifier chosen beforehand, so that it
    /// can be recorded before the turn runs. It must be a UUID that no other
    /// session uses.
    NewWithId(SessionId),
    /// Continues an earlier session.
    Resume(SessionId),
}

/// How hard the model should think. It belongs to the domain, because a
/// specialist keeps one in its profile; it is named here too because a turn
/// request carries it.
pub use teamwright_domain::Effort;

/// How the agent program decides about tool use that no rule allows or
/// denies.
///
/// The program's mode that approves everything and the mode in which a model
/// judges whether an action is safe are left out on purpose: risky tool use
/// is never approved on the manager's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PermissionMode {
    /// Every tool use that no rule allows needs approval.
    Manual,
    /// Creating and editing files inside the working folder is approved;
    /// everything else that no rule allows needs approval.
    AcceptEdits,
    /// Everything that no rule allows is denied.
    DontAsk,
    /// The session may look but not change anything.
    Plan,
}

/// What a session may do, as data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionProfile {
    /// The only built-in tools the session has, or all of them if absent.
    /// Names the program does not know are ignored by it. Tools from MCP
    /// servers are not affected.
    pub built_in_tools: Option<Vec<String>>,
    /// Tools, or tool rules such as `Bash(git status)`, that are used
    /// without asking.
    pub allowed_tools: Vec<String>,
    /// Tools, or tool rules, that are never used. A tool named here is
    /// removed from the session altogether.
    pub disallowed_tools: Vec<String>,
    /// What happens with tool use that no rule covers.
    pub mode: PermissionMode,
    /// Whether tool use that would need approval is denied at once. Without
    /// it, a turn would wait for an answer that nobody can give.
    pub deny_instead_of_asking: bool,
    /// Whether the session is confined to its working folder. Its file
    /// tools then reach nothing outside that folder, also not for reading,
    /// and the user's own settings do not apply to it, with the hooks,
    /// plugins and rules they bring.
    pub confined_to_working_folder: bool,
    /// Whether the session sees only the tool servers it is given, and none
    /// of those the user has set up for themself.
    pub only_given_tool_servers: bool,
}

impl PermissionProfile {
    /// The profile of a specialist in this milestone.
    ///
    /// A specialist may read and search, and create and edit files inside
    /// its working folder. It may not run shell commands. Everything else,
    /// such as reading or writing outside its folder, fetching from the web
    /// or using a connector, is denied without waiting.
    ///
    /// The session is confined to its working folder. Without that, the
    /// rule that lets it read without asking would let it read any file of
    /// the user, which a real run showed. Confined, it also leaves the
    /// user's own settings, hooks, plugins and tool servers alone: a
    /// specialist is a member of this team, not the user's own session.
    ///
    /// The built-in tools are limited to those it needs, so that tools which
    /// schedule work, start other sessions or publish something are not
    /// there at all. Skills stay usable, and so does the search for the
    /// tools of MCP servers.
    #[must_use]
    pub fn specialist() -> Self {
        Self {
            built_in_tools: Some(
                [
                    "Read",
                    "Glob",
                    "Grep",
                    "Edit",
                    "Write",
                    "NotebookEdit",
                    "Skill",
                    "ToolSearch",
                ]
                .map(String::from)
                .to_vec(),
            ),
            allowed_tools: ["Read", "Glob", "Grep"].map(String::from).to_vec(),
            disallowed_tools: ["Bash", "PowerShell"].map(String::from).to_vec(),
            mode: PermissionMode::AcceptEdits,
            deny_instead_of_asking: true,
            confined_to_working_folder: true,
            only_given_tool_servers: true,
        }
    }
}

/// What is needed to run one turn of a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnRequest {
    /// The folder the session works in. A resumed session works in the
    /// folder given here, not in the one it started in.
    pub working_folder: PathBuf,
    /// What the session is asked to do in this turn.
    pub prompt: String,
    /// Whether the turn starts a new session or continues one.
    pub session: SessionChoice,
    /// The model, by alias or full name. The program's default if absent.
    pub model: Option<String>,
    /// How hard the model should think. The program's default if absent.
    pub effort: Option<Effort>,
    /// Instructions added to the end of the program's own system prompt.
    pub extra_instructions: Option<String>,
    /// What the session may do.
    pub permissions: PermissionProfile,
}

/// Something that happened during a turn.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentEvent {
    /// The program has set up the session and is about to work.
    SessionStarted(SessionStarted),
    /// The assistant wrote text.
    Text {
        /// What it wrote.
        text: String,
    },
    /// The assistant uses a tool.
    ToolUse {
        /// The identifier of this use, which its result refers to.
        id: String,
        /// The name of the tool.
        tool: String,
        /// What the tool is used on, short and for people, for example the
        /// file that is read. May be empty.
        summary: String,
    },
    /// A tool has finished.
    ToolResult {
        /// The identifier of the use this is the result of.
        tool_use_id: String,
        /// The name of the tool, if its use was seen.
        tool: Option<String>,
        /// Whether the tool failed or was refused.
        failed: bool,
    },
    /// A tool use was denied because it needed approval.
    PermissionDenied(PermissionDenial),
    /// A request to the model failed and is tried again.
    Retrying {
        /// Which attempt this is, if reported.
        attempt: Option<u32>,
        /// How many attempts are made at most, if reported.
        max_attempts: Option<u32>,
        /// How long the program waits before the attempt, if reported.
        delay: Option<Duration>,
        /// Why the request failed, if reported.
        reason: Option<String>,
    },
    /// The program reported how much of the plan's limits is used.
    PlanUsage(PlanUsage),
    /// The program warned about something, for example its own setup.
    Warning {
        /// The warning, for people.
        message: String,
    },
    /// The turn is over. This is always the last event.
    TurnEnded(TurnOutcome),
}

/// A question a session asks the manager, because the answer is the
/// manager's to give. The session ends its turn after it has asked, and the
/// answer comes as the request of the next turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerQuestion {
    /// The question, not empty.
    pub question: String,
    /// Why it matters. May be empty.
    pub reason: String,
    /// The answers the session sees, two to four.
    pub options: Vec<QuestionOption>,
    /// The option the session recommends, by its place among the options,
    /// counted from 0.
    pub recommended: Option<usize>,
}

/// One of the answers a session offers the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionOption {
    /// The option in a few words, not empty.
    pub label: String,
    /// What choosing it leads to. May be empty.
    pub consequence: String,
}

/// What a session has at its start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStarted {
    /// The identifier of the session.
    pub session_id: SessionId,
    /// The model it runs on, by its full name.
    pub model: String,
    /// The folder it works in.
    pub working_folder: PathBuf,
    /// The permission mode the program reports, by the program's own name.
    pub permission_mode: String,
    /// The tools it can use.
    pub tools: Vec<String>,
    /// The skills it can use.
    pub skills: Vec<String>,
    /// The plugins that are loaded.
    pub plugins: Vec<String>,
    /// The MCP servers it knows, with whether they could be reached.
    pub mcp_servers: Vec<McpServer>,
    /// The version of the program, if it says.
    pub program_version: Option<String>,
}

/// An MCP server a session knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServer {
    /// Its name.
    pub name: String,
    /// Its state as the program names it, for example `connected`,
    /// `pending` or `needs-auth`.
    pub status: String,
}

/// A tool use that was denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDenial {
    /// The name of the tool.
    pub tool: String,
    /// The identifier of the tool use, if reported.
    pub tool_use_id: Option<String>,
}

/// How much of the plan's usage limits is used, as the program reports it.
///
/// It is only known when the program reports it. A window that is not
/// listed is unknown, never zero.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanUsage {
    /// The limit windows that were reported.
    pub windows: Vec<PlanWindow>,
}

/// One usage limit of the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanWindow {
    /// Which limit it is.
    pub kind: PlanWindowKind,
    /// How much of it is used, from 0 to 1, if reported.
    pub used: Option<f64>,
    /// When it starts again from nothing, if reported.
    pub resets_at: Option<Timestamp>,
    /// Whether the plan still allows work in this window, as the program
    /// names it, for example `allowed`.
    pub status: Option<String>,
}

/// Which usage limit of the plan a window is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlanWindowKind {
    /// The limit over five hours.
    FiveHours,
    /// The limit over seven days.
    Week,
    /// Another limit, by the program's name for it.
    Other(String),
}

/// How a turn ended.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnOutcome {
    /// The session the turn belonged to, if the program said.
    pub session_id: Option<SessionId>,
    /// Why the turn failed, or nothing if it did not.
    pub failure: Option<TurnFailure>,
    /// The assistant's last answer. May be empty.
    pub final_text: String,
    /// The tokens this turn used, per model.
    pub usage: Vec<ModelUsage>,
    /// The program's estimate of what this turn would cost at list prices,
    /// in US dollars. It is an estimate, and on a subscription nothing is
    /// paid per turn.
    pub estimated_cost_usd: Option<f64>,
    /// How long the turn took, if reported.
    pub duration: Option<Duration>,
    /// How many times the model was called in this turn, if reported.
    pub model_calls: Option<u32>,
    /// The tool uses that were denied.
    pub permission_denials: Vec<PermissionDenial>,
}

impl TurnOutcome {
    /// Whether the turn failed.
    #[must_use]
    pub const fn failed(&self) -> bool {
        self.failure.is_some()
    }

    /// The tokens of all models together.
    #[must_use]
    pub fn total_tokens(&self) -> TokenCounts {
        self.usage
            .iter()
            .fold(TokenCounts::default(), |sum, model| sum.plus(model.tokens))
    }
}

/// Why a turn failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnFailure {
    /// The user is not signed in to the program.
    NotSignedIn,
    /// The program finished the turn and reported it as failed.
    Reported {
        /// The program's name for the kind of failure, if it gave one.
        kind: Option<String>,
    },
    /// The turn was stopped.
    Stopped,
    /// The program ended without finishing the turn.
    ProgramEnded {
        /// What is known about it, for people: the exit code and the last
        /// lines the program wrote about errors.
        message: String,
    },
}

/// The tokens one model used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelUsage {
    /// The model, by its full name.
    pub model: String,
    /// The tokens.
    pub tokens: TokenCounts,
}

/// Counts of tokens, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TokenCounts {
    /// Tokens read by the model that were not in the cache.
    pub input: u64,
    /// Tokens the model wrote, thinking included.
    pub output: u64,
    /// Tokens read from the cache.
    pub cache_read: u64,
    /// Tokens written to the cache.
    pub cache_written: u64,
}

impl TokenCounts {
    /// The sum of two counts, kind by kind.
    #[must_use]
    pub const fn plus(self, other: Self) -> Self {
        Self {
            input: self.input.saturating_add(other.input),
            output: self.output.saturating_add(other.output),
            cache_read: self.cache_read.saturating_add(other.cache_read),
            cache_written: self.cache_written.saturating_add(other.cache_written),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_found_in_the_programs_answer() {
        assert_eq!(
            ProgramVersion::find_in("2.1.322 (Claude Code)\n"),
            Some(ProgramVersion::new(2, 1, 322))
        );
        assert_eq!(ProgramVersion::find_in("Claude Code"), None);
        assert_eq!(ProgramVersion::find_in("version 1.2"), None);
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(ProgramVersion::new(2, 1, 285) < ProgramVersion::new(2, 1, 322));
        assert!(ProgramVersion::new(2, 2, 0) > ProgramVersion::new(2, 1, 999));
        assert_eq!(ProgramVersion::new(2, 1, 9).to_string(), "2.1.9");
    }

    #[test]
    fn a_session_identifier_cannot_look_like_an_option() {
        assert!(SessionId::parse("0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10").is_ok());
        assert!(SessionId::parse("").is_err());
        assert!(SessionId::parse("--dangerously-skip-permissions").is_err());
        assert!(SessionId::parse("a b").is_err());
        assert!(SessionId::parse(&"a".repeat(SessionId::MAX_LENGTH + 1)).is_err());
    }

    #[test]
    fn a_specialist_may_edit_but_not_run_commands_and_is_never_kept_waiting() {
        let profile = PermissionProfile::specialist();
        assert_eq!(profile.mode, PermissionMode::AcceptEdits);
        assert!(profile.deny_instead_of_asking);
        assert!(profile.disallowed_tools.iter().any(|tool| tool == "Bash"));
        assert!(!profile.allowed_tools.iter().any(|tool| tool == "Bash"));
        let built_in = profile.built_in_tools.unwrap_or_default();
        assert!(built_in.iter().any(|tool| tool == "Edit"));
        assert!(
            !built_in
                .iter()
                .any(|tool| tool == "Bash" || tool == "WebFetch")
        );
    }

    #[test]
    fn the_tokens_of_all_models_are_added_up() {
        let tokens = |input| TokenCounts {
            input,
            output: 1,
            cache_read: 2,
            cache_written: 3,
        };
        let outcome = TurnOutcome {
            session_id: None,
            failure: None,
            final_text: String::new(),
            usage: vec![
                ModelUsage {
                    model: "a".to_owned(),
                    tokens: tokens(10),
                },
                ModelUsage {
                    model: "b".to_owned(),
                    tokens: tokens(5),
                },
            ],
            estimated_cost_usd: None,
            duration: None,
            model_calls: None,
            permission_denials: Vec::new(),
        };
        assert_eq!(
            outcome.total_tokens(),
            TokenCounts {
                input: 15,
                output: 2,
                cache_read: 4,
                cache_written: 6
            }
        );
        assert!(!outcome.failed());
    }
}
