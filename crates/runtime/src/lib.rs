//! Runs Teamwright's team members on the user's own Claude Code.
//!
//! This is the only crate that knows about Claude Code
//! ([decision 0005](../../../docs/decisions/0005-drive-the-users-claude-code.md)).
//! [`ClaudeCode`] implements the application's
//! [`AgentRuntime`](teamwright_application::AgentRuntime): it finds the
//! installed program, asks it whether the user is signed in, runs one turn
//! of a session in headless mode and reports what happens as events, and
//! opens a session in the Claude desktop app.
//!
//! Every turn is one process: `claude -p --output-format stream-json`, with
//! the prompt on standard input and one JSON object per line on standard
//! output. A later turn resumes the session by its identifier.
//!
//! Teamwright never reads, keeps or passes on a credential. It asks the
//! program whether the user is signed in, and the program uses its own
//! sign-in.

mod arguments;
mod claude_code;
mod decode;
mod environment;
mod locate;
mod process;
mod sign_in;

pub use claude_code::ClaudeCode;
pub use environment::Environment;
pub use locate::Locator;
