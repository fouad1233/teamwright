//! Teamwright's ways out to other services
//! ([decision 0027](../../../docs/decisions/0027-integrations.md)):
//! GitHub through the manager's own command line program `gh`.
//!
//! This crate implements ports the application crate declares. It is the
//! only crate that knows the options and the output of `gh`.

mod command;
mod github;
mod locate;

pub use github::GhCommandLine;
pub use locate::GhLocator;
