//! Runs Teamwright's team members on the user's own Claude Code.
//!
//! This is the only crate that knows about Claude Code
//! ([decision 0005](../../../docs/decisions/0005-drive-the-users-claude-code.md)).
//! [`ClaudeCode`] implements the application's
//! [`AgentRuntime`](teamwright_application::AgentRuntime): it finds the
//! installed program, asks it whether the user is signed in, runs one turn
//! of a session in headless mode and reports what happens as events, and
//! opens a session in the Claude desktop app.
//! [`EarlierConversationsOnDisk`] implements
//! [`EarlierConversations`](teamwright_application::EarlierConversations):
//! it reads the conversations the user held with Claude Code before, from
//! where Claude Code keeps them, to make digests of them.
//!
//! Every turn is one process: `claude -p --output-format stream-json`, with
//! the prompt on standard input and one JSON object per line on standard
//! output. A later turn resumes the session by its identifier.
//!
//! A specialist and the Advisor reach the manager through the [`team_bus`]:
//! a small tool server that Claude Code starts for a turn, whose tools ask
//! the manager to decide, ask the questions of the Advisor, and propose a
//! team and a plan.
//!
//! The program of a connector is started through Teamwright's own program,
//! in the mode of the [`connector_launcher`]: in an empty folder of its
//! own, with only the variables the connector names.
//!
//! Teamwright never reads, keeps or passes on a credential. It asks the
//! program whether the user is signed in, and the program uses its own
//! sign-in.

mod arguments;
mod claude_code;
pub mod connector_launcher;
mod decode;
mod earlier;
mod environment;
mod kit;
mod locate;
pub mod memory_file;
mod plan_limit;
mod process;
mod sign_in;
pub mod team_bus;

pub use claude_code::{ClaudeCode, full_form};
pub use earlier::EarlierConversationsOnDisk;
pub use environment::Environment;
pub use locate::Locator;
