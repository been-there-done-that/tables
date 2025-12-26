mod migrations;
mod connection;
mod credential_manager;
mod connection_manager;

use tauri::Emitter;
use std::{path::PathBuf, sync::{Arc, Mutex}, time::SystemTime};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Manager, State, PhysicalPosition, PhysicalSize, Size};

use connection::{Connection as DatabaseConnection, SecureCredentials, ConnectionInfo};
use connection_manager::{ConnectionManager, ConnectionManagerState};

#[derive(Clone, Debug, Serialize)]
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

#[derive(Debug)]
pub struct DatabaseState {
    pub conn: Arc<Mutex<Connection>>,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn load_theme(row: &rusqlite::Row<'_>) -> Result<Theme, rusqlite::Error> {
    Ok(Theme {
        id: row.get(0)?,
        name: row.get(1)?,
        author: row.get(2)?,
        description: row.get(3)?,
        theme_data: row.get(4)?,
        is_builtin: row.get::<_, i64>(5)? != 0,
        is_active: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn fetch_active_theme(conn: &Connection) -> Result<Option<Theme>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at
             FROM themes
             WHERE is_active = 1
             LIMIT 1",
        )
        .map_err(|e| format!("Failed to prepare active query: {e}"))?;
    stmt.query_row([], load_theme)
        .optional()
        .map_err(|e| format!("Failed to fetch active theme: {e}"))
}

#[tauri::command]
fn get_all_themes(state: State<'_, DatabaseState>) -> Result<Vec<Theme>, String> {
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

#[tauri::command]
fn get_active_theme(state: State<'_, DatabaseState>) -> Result<Option<Theme>, String> {
    let conn = state
        .conn
        .lock()
        .map_err(|e| format!("Failed to lock database: {e}"))?;
    fetch_active_theme(&conn)
}

#[tauri::command]
fn set_active_theme(
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
            params![theme_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check theme: {e}"))?;
    if exists.is_none() {
        return Err(format!("Theme {} not found", theme_id));
    }

    tx.execute("UPDATE themes SET is_active = 0", [])
        .map_err(|e| format!("Failed to clear active flag: {e}"))?;
    tx.execute(
        "UPDATE themes SET is_active = 1, updated_at = ?2 WHERE id = ?1",
        params![theme_id, now_ts()],
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

// Connection management commands
#[tauri::command]
async fn create_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.create_connection(connection, credentials)
}

#[tauri::command]
async fn get_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(DatabaseConnection, SecureCredentials), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_connection(&id)
}

#[tauri::command]
async fn get_connection_metadata(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<DatabaseConnection, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_connection_metadata(&id)
}

#[tauri::command]
async fn list_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.list_connections()
}

#[tauri::command]
async fn update_connection(
    connection: DatabaseConnection,
    credentials: Option<SecureCredentials>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.update_connection(connection, credentials)
}

#[tauri::command]
async fn delete_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.delete_connection(&id)
}

#[tauri::command]
async fn test_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection(&connection, &credentials)
}

#[tauri::command]
async fn get_favorite_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_favorite_connections()
}

#[tauri::command]
async fn search_connections(
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.search_connections(&query)
}

#[tauri::command]
async fn update_connection_stats(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.update_connection_stats(&id)
}

#[tauri::command]
async fn check_keyring_available(
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<bool, String> {
    Ok(conn_state.credential_manager.is_available())
}

fn init_connection(db_path: &PathBuf) -> Result<Connection, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create data directory: {e}"))?;
    }
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database {}: {e}", db_path.display()))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL: {e}"))?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| format!("Failed to enable foreign keys: {e}"))?;
    migrations::apply(&conn, now_ts)?;
    Ok(conn)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
            let db_path = app_data_dir.join("tables.db");
            let conn = init_connection(&db_path)?;
            app.manage(DatabaseState {
                conn: Arc::new(Mutex::new(conn)),
            });

            // Initialize connection manager state
            app.manage(ConnectionManagerState::new());

            // Dynamically size the main window to ~100% of the current monitor and center it.
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let screen_size = monitor.size();
                    let width = (screen_size.width as f64 * 1.0) as u32;
                    let height = (screen_size.height as f64 * 1.0) as u32;
                    let x = (screen_size.width as i32 - width as i32) / 2;
                    let y = (screen_size.height as i32 - height as i32) / 2;

                    let _ = window.set_size(Size::Physical(PhysicalSize { width, height }));
                    let _ =
                        window.set_position(tauri::Position::Physical(PhysicalPosition { x, y }));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_themes,
            get_active_theme,
            set_active_theme,
            create_connection,
            get_connection,
            get_connection_metadata,
            list_connections,
            update_connection,
            delete_connection,
            test_connection,
            get_favorite_connections,
            search_connections,
            update_connection_stats,
            check_keyring_available
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
