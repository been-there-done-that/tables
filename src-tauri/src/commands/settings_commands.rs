use crate::DatabaseState;
use crate::settings::{self, AppSettings};
use tauri::{State, Window, Emitter};

#[tauri::command]
pub fn get_app_settings(state: State<'_, DatabaseState>) -> Result<AppSettings, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    settings::get_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_app_setting(
    state: State<'_, DatabaseState>,
    window: Window,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    settings::update_setting(&conn, &key, &value).map_err(|e| e.to_string())?;
    
    // Emit event to valid windows
    let _ = window.emit("settings-changed", (key, value));
    
    Ok(())
}
