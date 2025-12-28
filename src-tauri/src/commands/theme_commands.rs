use crate::{Theme, DatabaseState, load_theme, fetch_active_theme, now_ts};
use tauri::{AppHandle, State, Emitter};
use rusqlite::OptionalExtension;
use log::{info, debug, warn, error, trace};

/// Get all available themes
#[tauri::command]
pub fn get_all_themes(state: State<'_, DatabaseState>) -> Result<Vec<Theme>, String> {
    debug!("Getting all themes");
    let conn = state
        .conn
        .lock()
        .map_err(|e| {
            error!("Failed to lock database for getting themes: {}", e);
            format!("Failed to lock database: {e}")
        })?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at
             FROM themes
             ORDER BY name COLLATE NOCASE",
        )
        .map_err(|e| {
            error!("Failed to prepare query for themes: {}", e);
            format!("Failed to prepare query: {e}")
        })?;
    let rows = stmt
        .query_map([], load_theme)
        .map_err(|e| {
            error!("Failed to query themes: {}", e);
            format!("Failed to query themes: {e}")
        })?;

    let mut themes = Vec::new();
    for theme in rows {
        themes.push(theme.map_err(|e| {
            error!("Failed to read theme: {}", e);
            format!("Failed to read theme: {e}")
        })?);
    }
    debug!("Retrieved {} themes", themes.len());
    Ok(themes)
}

/// Get the currently active theme
#[tauri::command]
pub fn get_active_theme(state: State<'_, DatabaseState>) -> Result<Option<Theme>, String> {
    debug!("Getting active theme");
    let conn = state
        .conn
        .lock()
        .map_err(|e| {
            error!("Failed to lock database for getting active theme: {}", e);
            format!("Failed to lock database: {e}")
        })?;
    let theme = fetch_active_theme(&conn);
    if theme.is_ok() {
        debug!("Active theme retrieved");
    }
    theme
}

/// Set a theme as active
#[tauri::command]
pub fn set_active_theme(
    app: AppHandle,
    state: State<'_, DatabaseState>,
    theme_id: String,
) -> Result<(), String> {
    debug!("Setting active theme to '{}'", theme_id);
    let mut conn = state
        .conn
        .lock()
        .map_err(|e| {
            error!("Failed to lock database for setting active theme: {}", e);
            format!("Failed to lock database: {e}")
        })?;
    let tx = conn
        .transaction()
        .map_err(|e| {
            error!("Failed to open transaction for theme change: {}", e);
            format!("Failed to open transaction: {e}")
        })?;

    let exists: Option<String> = tx
        .query_row(
            "SELECT id FROM themes WHERE id = ?1",
            rusqlite::params![theme_id],
            |row| row.get(0),
        )
        .unwrap_or(None);
    if exists.is_none() {
        error!("Theme '{}' not found", theme_id);
        return Err(format!("Theme {} not found", theme_id));
    }
    trace!("Theme '{}' exists, proceeding with activation", theme_id);

    tx.execute("UPDATE themes SET is_active = 0", [])
        .map_err(|e| {
            error!("Failed to clear active flag: {}", e);
            format!("Failed to clear active flag: {e}")
        })?;
    tx.execute(
        "UPDATE themes SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![theme_id, now_ts()],
    )
    .map_err(|e| {
        error!("Failed to activate theme '{}': {}", theme_id, e);
        format!("Failed to activate theme: {e}")
    })?;
    tx.commit()
        .map_err(|e| {
            error!("Failed to commit theme change for '{}': {}", theme_id, e);
            format!("Failed to commit theme change: {e}")
        })?;

    debug!("Theme '{}' activated successfully", theme_id);
    // Broadcast change
    if let Ok(Some(theme)) = fetch_active_theme(&conn) {
        trace!("Broadcasting theme change event");
        let _ = app.emit("current-theme", theme);
    }
    Ok(())
}
