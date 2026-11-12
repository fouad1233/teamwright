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

use std::ffi::OsStr;
use std::process::ExitCode;
use std::{env, io};

use tauri::{Manager, RunEvent};

use crate::state::{AppState, TEAM_BUS_ARGUMENT};

fn main() -> ExitCode {
    if env::args_os().nth(1).as_deref() == Some(OsStr::new(TEAM_BUS_ARGUMENT)) {
        return serve_the_team_bus();
    }

    let application = tauri::Builder::default()
        // Lets the interface open the window of the operating system in
        // which the manager chooses a folder or files.
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::open(app.handle())?;
            app.manage(state);
            state::start_syncing_in_the_background(app.handle().clone());
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
            commands::places::project_places,
            commands::places::set_work_folder,
            commands::places::add_reference_folder,
            commands::places::remove_reference_folder,
            commands::places::add_reference_files,
            commands::places::show_work_folder,
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
            commands::tasks::list_task_history,
            commands::sprints::list_sprints,
            commands::sprints::create_sprint,
            commands::sprints::change_sprint,
            commands::sprints::delete_sprint,
            commands::sprints::start_sprint,
            commands::sprints::complete_sprint,
            commands::sprints::plan_task,
            commands::sprints::set_task_points,
            commands::sprints::sprint_report,
            commands::team::list_members,
            commands::team::add_member,
            commands::team::rename_member,
            commands::team::set_member_responsibility,
            commands::team::set_member_profile,
            commands::team::remove_member,
            commands::work::agent_program,
            commands::work::list_conversations,
            commands::work::start_work,
            commands::work::reply,
            commands::work::ask,
            commands::work::answer_decision,
            commands::work::advise,
            commands::work::answer_interview,
            commands::work::approve_proposal,
            commands::work::list_conversations_waiting,
            commands::work::stop_work,
            commands::work::open_in_claude,
            commands::sync::sync_state,
            commands::sync::choose_sync_folder,
            commands::sync::stop_syncing,
            commands::sync::sync_now,
            commands::sync::rename_this_computer,
            commands::sync::accept_computer,
            commands::sync::ignore_computer,
            commands::sync::sync_notices,
            commands::sync::dismiss_sync_notice,
            commands::sync::give_this_computer_a_new_identity,
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

/// Serves the team bus on standard input and output, for the one turn of a
/// specialist or of the Advisor that Claude Code started this program for.
/// The arguments after `--team-bus` name the tools it offers. No window is
/// opened and the database is not touched.
fn serve_the_team_bus() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(2).collect();
    let tools = match teamwright_runtime::team_bus::tools_from_arguments(&arguments) {
        Ok(tools) => tools,
        Err(problem) => {
            eprintln!("The team bus cannot start: {problem}");
            return ExitCode::FAILURE;
        }
    };
    let input = io::stdin().lock();
    let output = io::stdout().lock();
    match teamwright_runtime::team_bus::serve(input, output, tools) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("The team bus stopped: {error}");
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
