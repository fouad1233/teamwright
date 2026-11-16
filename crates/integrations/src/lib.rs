//! Teamwright's ways out to other services
//! ([decision 0027](../../../docs/decisions/0027-integrations.md)):
//! GitHub through the manager's own command line program `gh`, and web
//! addresses (a Slack channel, and the addresses that listen for events)
//! over HTTPS.
//!
//! This crate implements ports the application crate declares. It is the
//! only crate that knows the options and the output of `gh`, and the shape
//! in which an event goes to a web address.

mod command;
mod github;
mod locate;
mod signature;
mod web;

pub use github::GhCommandLine;
pub use locate::GhLocator;
pub use signature::{hex, hmac_sha256, sha256};
pub use web::{
    HttpsSender, SENDER_HEADER, SIGNATURE_HEADER, VERSION_HEADER, channel_body, event_request,
    payload_json, post, signature_of,
};
