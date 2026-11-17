//! Commands for the look of the window on this computer
//! ([decision 0028](../../../../../docs/decisions/0028-themes.md)).

use tauri::State;
use teamwright_application::Theme;

use crate::error::CommandError;
use crate::state::AppState;

/// The name of the theme of this computer: `lamplight` if the manager never
/// chose one.
#[tauri::command]
pub async fn theme(state: State<'_, AppState>) -> Result<String, CommandError> {
    Ok(state.appearance.theme()?.name().to_owned())
}

/// Chooses the theme of this computer by its name, and returns the name.
/// An unknown name is refused (`invalid_request`).
#[tauri::command]
pub async fn choose_theme(
    state: State<'_, AppState>,
    name: String,
) -> Result<String, CommandError> {
    let theme = Theme::parse(&name)
        .ok_or_else(|| CommandError::invalid_request(format!("{name} is not a theme")))?;
    Ok(state.appearance.choose(theme)?.name().to_owned())
}
