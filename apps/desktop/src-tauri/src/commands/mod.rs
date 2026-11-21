//! The commands the interface can call.
//!
//! Every command follows the same pattern: read the arguments, call one use
//! case, turn the result into data for the interface. Anything more belongs
//! in the application crate.

pub mod appearance;
pub mod areas;
pub mod checklists;
pub mod coordinator;
pub mod digests;
pub mod files;
pub mod github;
pub mod google_drive;
pub mod integrations;
pub mod kits;
pub mod memory;
pub mod notifications;
pub mod places;
pub mod project_report;
pub mod projects;
pub mod sprints;
pub mod sync;
pub mod tags;
pub mod tasks;
pub mod team;
pub mod teamwork;
pub mod views;
pub mod web;
pub mod work;
pub mod workbook;
