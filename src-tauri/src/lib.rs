mod migrations;
mod connection;
mod credentials;
mod credential_manager;
mod connection_manager;
mod aws_profile_manager;
mod commands;
mod plugins;
mod configs;

use tauri::{Manager, PhysicalPosition, PhysicalSize, Size};
use std::{path::PathBuf, sync::{Arc, Mutex}, time::SystemTime};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use commands::theme_commands::*;
use commands::connection_commands::*;
use commands::aws_commands::*;
use commands::redis_commands::*;
use commands::athena_commands::*;
use commands::window_commands::*;
use plugins::{PluginDiscovery, get_available_plugins, enable_plugin, disable_plugin, get_plugin_info, initialize_all_plugins};

// Re-export for command modules
pub use connection_manager::ConnectionManagerState;

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

            // Initialize plugin system and discover all plugins
            let discovery = PluginDiscovery::new();
            discovery.discover_builtin_plugins()
                .map_err(|e| format!("Failed to discover plugins: {e}"))?;

            // Initialize all enabled plugins
            let init_results = discovery.initialize_plugins(app.handle());
            let mut failed_plugins = Vec::new();
            for result in init_results {
                if !result.success {
                    failed_plugins.push(format!("{} - {}", result.plugin_name, result.message));
                    eprintln!("Plugin initialization failed: {} - {}", result.plugin_name, result.message);
                } else {
                    println!("Plugin initialized: {} ({} commands)", result.plugin_name, result.commands_registered);
                }
            }

            // Fail fast if critical plugins fail to initialize
            if !failed_plugins.is_empty() {
                return Err(format!("Critical plugin initialization failures: {}", failed_plugins.join(", ")).into());
            }

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
        .invoke_handler(aggregate_plugin_commands!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
