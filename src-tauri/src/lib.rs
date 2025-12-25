use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{command, Manager, State};
use crate::migration_runner::initialize_database;

mod migration_runner;

// Theme data structures
#[derive(Debug, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub theme_data: String,
    pub is_builtin: bool,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

// Database state for Tauri
pub struct DatabaseState {
    pub conn: Mutex<Connection>,
}

impl DatabaseState {
    pub fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }
}

// Theme commands
#[command]
async fn get_all_themes(
    state: State<'_, DatabaseState>,
) -> Result<Vec<Theme>, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    let mut stmt = conn
        .prepare("SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at FROM themes ORDER BY name ASC")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let themes = stmt
        .query_map([], |row| {
            Ok(Theme {
                id: row.get(0)?,
                name: row.get(1)?,
                author: row.get(2)?,
                description: row.get(3)?,
                theme_data: row.get(4)?,
                is_builtin: row.get::<_, i32>(5)? != 0,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<SqlResult<Vec<_>>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(themes)
}

#[command]
async fn get_active_theme(
    state: State<'_, DatabaseState>,
) -> Result<Option<Theme>, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    let mut stmt = conn
        .prepare("SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at FROM themes WHERE is_active = 1 LIMIT 1")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let theme = stmt
        .query_row([], |row| {
            Ok(Theme {
                id: row.get(0)?,
                name: row.get(1)?,
                author: row.get(2)?,
                description: row.get(3)?,
                theme_data: row.get(4)?,
                is_builtin: row.get::<_, i32>(5)? != 0,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .optional()
        .map_err(|e| format!("Failed to execute query: {}", e))?;

    Ok(theme)
}

#[command]
async fn set_active_theme(
    state: State<'_, DatabaseState>,
    theme_id: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    // Begin transaction
    let tx = conn.unchecked_transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;

    // Clear all active flags
    tx.execute("UPDATE themes SET is_active = 0", [])
        .map_err(|e| format!("Failed to clear active themes: {}", e))?;

    // Set new active theme
    tx.execute(
        "UPDATE themes SET is_active = 1, updated_at = strftime('%s', 'now') WHERE id = ?",
        [theme_id],
    )
    .map_err(|e| format!("Failed to set active theme: {}", e))?;

    // Commit transaction
    tx.commit()
        .map_err(|e| format!("Failed to commit transaction: {}", e))?;

    Ok(())
}

#[command]
async fn create_theme(
    state: State<'_, DatabaseState>,
    theme_data: String,
) -> Result<Theme, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    // Parse theme to get metadata
    let parsed: serde_json::Value = serde_json::from_str(&theme_data)
        .map_err(|e| format!("Invalid theme JSON: {}", e))?;

    let id = parsed["id"]
        .as_str()
        .ok_or("Missing or invalid 'id' field".to_string())?
        .to_string();

    let name = parsed["name"]
        .as_str()
        .ok_or("Missing or invalid 'name' field".to_string())?
        .to_string();

    let author = parsed.get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = parsed.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Check if theme already exists
    let exists: i32 = conn.query_row(
        "SELECT COUNT(*) FROM themes WHERE id = ?",
        [&id],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to check if theme exists: {}", e))?;

    if exists > 0 {
        return Err(format!("Theme with ID '{}' already exists", id));
    }

    // Insert theme
    conn.execute(
        r#"
        INSERT INTO themes (id, name, author, description, theme_data, is_builtin, is_active)
        VALUES (?, ?, ?, ?, ?, 0, 0)
        "#,
        params![id, name, author, description, theme_data],
    )
    .map_err(|e| format!("Failed to insert theme: {}", e))?;

    // Return the created theme
    let mut stmt = conn
        .prepare("SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at FROM themes WHERE id = ?")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let theme = stmt
        .query_row([&id], |row| {
            Ok(Theme {
                id: row.get(0)?,
                name: row.get(1)?,
                author: row.get(2)?,
                description: row.get(3)?,
                theme_data: row.get(4)?,
                is_builtin: row.get::<_, i32>(5)? != 0,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to retrieve created theme: {}", e))?;

    Ok(theme)
}

#[command]
async fn update_theme(
    state: State<'_, DatabaseState>,
    theme_id: String,
    theme_data: String,
) -> Result<Theme, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    // Update theme
    conn.execute(
        r#"
        UPDATE themes
        SET theme_data = ?, updated_at = strftime('%s', 'now')
        WHERE id = ? AND is_builtin = 0
        "#,
        params![theme_data, theme_id],
    )
    .map_err(|e| format!("Failed to update theme: {}", e))?;

    // Check if theme was actually updated
    let rows_affected = conn.changes();
    if rows_affected == 0 {
        return Err("Theme not found or is built-in".to_string());
    }

    // Return updated theme
    let mut stmt = conn
        .prepare("SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at FROM themes WHERE id = ?")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let theme = stmt
        .query_row([&theme_id], |row| {
            Ok(Theme {
                id: row.get(0)?,
                name: row.get(1)?,
                author: row.get(2)?,
                description: row.get(3)?,
                theme_data: row.get(4)?,
                is_builtin: row.get::<_, i32>(5)? != 0,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to retrieve updated theme: {}", e))?;

    Ok(theme)
}

#[command]
async fn delete_theme(
    state: State<'_, DatabaseState>,
    theme_id: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    // Check if theme is active
    let is_active: i32 = conn.query_row(
        "SELECT is_active FROM themes WHERE id = ?",
        [&theme_id],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to check if theme is active: {}", e))?;

    if is_active != 0 {
        return Err("Cannot delete active theme".to_string());
    }

    // Delete theme (built-in themes are protected by is_builtin check)
    conn.execute(
        "DELETE FROM themes WHERE id = ? AND is_builtin = 0",
        [&theme_id],
    )
    .map_err(|e| format!("Failed to delete theme: {}", e))?;

    let rows_affected = conn.changes();
    if rows_affected == 0 {
        return Err("Theme not found or is built-in".to_string());
    }

    Ok(())
}

#[command]
async fn search_themes(
    state: State<'_, DatabaseState>,
    query: String,
) -> Result<Vec<Theme>, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;

    let pattern = format!("%{}%", query);

    let mut stmt = conn
        .prepare("SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at FROM themes WHERE name LIKE ? OR description LIKE ? ORDER BY name ASC")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let themes = stmt
        .query_map(params![pattern, pattern], |row| {
            Ok(Theme {
                id: row.get(0)?,
                name: row.get(1)?,
                author: row.get(2)?,
                description: row.get(3)?,
                theme_data: row.get(4)?,
                is_builtin: row.get::<_, i32>(5)? != 0,
                is_active: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Failed to execute query: {}", e))?
        .collect::<SqlResult<Vec<_>>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(themes)
}

#[command]
async fn export_themes(
    state: State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let themes = get_all_themes(state).await?
        .into_iter()
        .filter(|t| !t.is_builtin) // Only export user themes
        .map(|t| serde_json::json!({
            "id": t.id,
            "name": t.name,
            "author": t.author,
            "description": t.description,
            "theme_data": t.theme_data,
        }))
        .collect::<Vec<_>>();

    let export_data = serde_json::json!({
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "themes": themes
    });

    Ok(export_data)
}

#[command]
async fn import_themes(
    state: State<'_, DatabaseState>,
    import_data: serde_json::Value,
) -> Result<HashMap<String, String>, String> {
    let conn = state.conn.lock().map_err(|e| format!("Failed to lock database: {}", e))?;
    let conn = &*conn;
    let themes = import_data["themes"]
        .as_array()
        .ok_or("Invalid import data: missing themes array".to_string())?;

    let mut results = HashMap::new();

    for theme_value in themes {
        let theme_data = serde_json::to_string(theme_value)
            .map_err(|e| format!("Failed to serialize theme: {}", e))?;

        let theme_id = theme_value["id"]
            .as_str()
            .ok_or("Theme missing ID".to_string())?;

        // Try to insert/update theme
        match conn.execute(
            r#"
            INSERT INTO themes (id, name, theme_data, is_builtin, is_active)
            VALUES (?, ?, ?, 0, 0)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                theme_data = excluded.theme_data,
                updated_at = strftime('%s', 'now')
            "#,
            params![
                theme_value["id"].as_str().unwrap_or(""),
                theme_value["name"].as_str().unwrap_or("Unknown"),
                theme_data
            ],
        ) {
            Ok(_) => {
                results.insert(theme_id.to_string(), "success".to_string());
            }
            Err(e) => {
                results.insert(theme_id.to_string(), format!("error: {}", e));
            }
        }
    }

    Ok(results)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // Initialize database in app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to get app data directory: {}", e))?;

            let db_path = app_data_dir.join("tables.db");

            let conn = match initialize_database(&db_path) {
                Ok(conn) => conn,
                Err(e) => {
                    eprintln!("Failed to initialize database: {}", e);
                    std::process::exit(1);
                }
            };

            // Store database state
            app.manage(DatabaseState::new(conn));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_themes,
            get_active_theme,
            set_active_theme,
            create_theme,
            update_theme,
            delete_theme,
            search_themes,
            export_themes,
            import_themes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
