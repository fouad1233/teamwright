//! The commands the interface can call.
//!
//! Every command follows the same pattern: read the arguments, call one use
//! case, turn the result into data for the interface. Anything more belongs
//! in the application crate.

pub mod areas;
pub mod places;
pub mod projects;
pub mod sprints;
pub mod sync;
pub mod tasks;
pub mod team;
pub mod views;
pub mod work;
