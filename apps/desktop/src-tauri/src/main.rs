//! The Teamwright desktop application.
//!
//! This crate is the shell: it opens the window, builds the services, and
//! translates between the interface and the use cases. It contains no rules
//! of its own. A command reads its arguments, calls one use case and turns
//! the result into data the interface understands.

// Without this, a release build on Windows would open a console window next
// to the application window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dto;
mod error;
mod state;

use std::process::ExitCode;

use tauri::{Manager, RunEvent};

use crate::state::AppState;

fn main() -> ExitCode {
    let application = tauri::Builder::default()
        .setup(|app| {
            let state = AppState::open(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::rename_project,
            commands::projects::describe_project,
            commands::projects::archive_project,
            commands::projects::list_archived_projects,
            commands::projects::restore_project,
            commands::projects::delete_project,
            commands::projects::add_status,
            commands::projects::change_status,
            commands::projects::move_status,
            commands::projects::remove_status,
            commands::tasks::list_tasks,
            commands::tasks::create_task,
            commands::tasks::rename_task,
            commands::tasks::describe_task,
            commands::tasks::prioritize_task,
            commands::tasks::assign_task,
            commands::tasks::estimate_task,
            commands::tasks::reschedule_task,
            commands::tasks::move_task,
            commands::tasks::add_task_dependency,
            commands::tasks::remove_task_dependency,
            commands::tasks::delete_task,
            commands::team::list_members,
            commands::team::add_member,
            commands::team::rename_member,
            commands::team::set_member_responsibility,
            commands::team::remove_member,
            commands::work::agent_program,
            commands::work::list_conversations,
            commands::work::start_work,
            commands::work::reply,
            commands::work::ask,
            commands::work::stop_work,
            commands::work::open_in_claude,
        ])
        .build(tauri::generate_context!());

    match application {
        Ok(application) => {
            application.run(|app, event| {
                if matches!(event, RunEvent::Exit) {
                    stop_the_specialists(app);
                }
            });
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Teamwright could not start: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Stops the specialists that are still working when the program closes.
/// Nobody would follow what they do afterwards.
fn stop_the_specialists(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        // The window is gone, so a failure has nobody to be shown to. The
        // turns it left as running are ended at the next start.
        let _ = state.work.close();
    }
}
