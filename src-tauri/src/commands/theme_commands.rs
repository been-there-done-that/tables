use crate::{Theme, DatabaseState, load_theme, fetch_active_theme, now_ts};
use tauri::{AppHandle, State, Emitter};
use rusqlite::OptionalExtension;

/// Get all available themes
#[tauri::command]
pub fn get_all_themes(state: State<'_, DatabaseState>) -> Result<Vec<Theme>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| format!("Failed to lock database: {e}"))?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at
             FROM themes
             ORDER BY name COLLATE NOCASE",
        )
        .map_err(|e| format!("Failed to prepare query: {e}"))?;
    let rows = stmt
        .query_map([], load_theme)
        .map_err(|e| format!("Failed to query themes: {e}"))?;

    let mut themes = Vec::new();
    for theme in rows {
        themes.push(theme.map_err(|e| format!("Failed to read theme: {e}"))?);
    }
    Ok(themes)
}

/// Get the currently active theme
#[tauri::command]
pub fn get_active_theme(state: State<'_, DatabaseState>) -> Result<Option<Theme>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| format!("Failed to lock database: {e}"))?;
    fetch_active_theme(&conn)
}

/// Set a theme as active
#[tauri::command]
pub fn set_active_theme(
    app: AppHandle,
    state: State<'_, DatabaseState>,
    theme_id: String,
) -> Result<(), String> {
    let mut conn = state
        .conn
        .lock()
        .map_err(|e| format!("Failed to lock database: {e}"))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to open transaction: {e}"))?;

    let exists: Option<String> = tx
        .query_row(
            "SELECT id FROM themes WHERE id = ?1",
            rusqlite::params![theme_id],
            |row| row.get(0),
        )
        .unwrap_or(None);
    if exists.is_none() {
        return Err(format!("Theme {} not found", theme_id));
    }

    tx.execute("UPDATE themes SET is_active = 0", [])
        .map_err(|e| format!("Failed to clear active flag: {e}"))?;
    tx.execute(
        "UPDATE themes SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        rusqlite::params![theme_id, now_ts()],
    )
    .map_err(|e| format!("Failed to activate theme: {e}"))?;
    tx.commit()
        .map_err(|e| format!("Failed to commit theme change: {e}"))?;

    // Broadcast change
    if let Ok(Some(theme)) = fetch_active_theme(&conn) {
        let _ = app.emit("current-theme", theme);
    }
    Ok(())
}
