mod migrations;
mod connection;
mod credentials;
mod crypto;
mod credential_manager;
mod connection_manager;
mod aws_profile_manager;
mod commands;
mod plugins;
mod configs;
mod metrics;
mod introspection;
mod db_test;
mod completion;
mod settings;
pub mod constants;
pub mod schema_types;
pub mod adapter;
pub mod adapters;
pub mod orchestrator;
pub mod adapter_registry;
#[cfg(test)]
mod dci_tests;

use tauri::{Manager, PhysicalPosition, PhysicalSize, Size, Emitter, Listener};
use std::{path::PathBuf, sync::{Arc, Mutex}, time::SystemTime, collections::{HashMap, HashSet}};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use log::{info, debug, warn, error, trace};
use commands::theme_commands::*;
use commands::connection_commands::*;
use commands::aws_commands::*;
use commands::redis_commands::*;
use commands::athena_commands::*;
use commands::window_commands::*;
use commands::introspection_commands::*;
use commands::test_commands::*;
use commands::completion_commands::*;
use commands::unified_introspection::*;
use commands::font_commands::*;
use commands::settings_commands::*;
use commands::debug_commands::*;
use plugins::{PluginDiscovery, get_available_plugins, enable_plugin, disable_plugin, get_plugin_info, initialize_all_plugins};
use credential_manager::CredentialManager;
use metrics::{MetricsRegistry, SystemMonitor, start_metrics_emitter};

// Re-export for command modules
pub use connection_manager::{ConnectionManager, ConnectionManagerState};
pub use metrics::MetricsRegistry as SharedMetricsRegistry; // Optional if commands need it

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
    trace!("Calculating current timestamp");
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn load_theme(row: &rusqlite::Row<'_>) -> Result<Theme, rusqlite::Error> {
    // debug!("Loading theme from database row");
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
    debug!("Fetching active theme from database");
    let mut stmt = conn
        .prepare(
            "SELECT id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at
             FROM themes
             WHERE is_active = 1
             LIMIT 1",
        )
        .map_err(|e| format!("Failed to prepare active query: {e}"))?;
    let theme = stmt.query_row([], load_theme)
        .optional()
        .map_err(|e| format!("Failed to fetch active theme: {e}"))?;
    if theme.is_some() {
        debug!("Active theme found");
    } else {
        debug!("No active theme found");
    }
    Ok(theme)
}

fn init_connection(db_path: &PathBuf) -> Result<Connection, String> {
    debug!("Initializing database connection at {}", db_path.display());
    if let Some(parent) = db_path.parent() {
        debug!("Ensuring parent directory exists: {}", parent.display());
        std::fs::create_dir_all(parent)
            .map_err(|e| {
                error!("Failed to create data directory {}: {}", parent.display(), e);
                format!("Failed to create data directory: {e}")
            })?;
    }
    debug!("Opening database connection");
    let mut conn = Connection::open(db_path)
        .map_err(|e| {
            error!("Failed to open database {}: {}", db_path.display(), e);
            format!("Failed to open database {}: {e}", db_path.display())
        })?;
    debug!("Enabling WAL journal mode");
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| {
            error!("Failed to enable WAL: {}", e);
            format!("Failed to enable WAL: {e}")
        })?;
    debug!("Setting busy timeout");
    conn.pragma_update(None, "busy_timeout", 5000)
        .map_err(|e| {
            error!("Failed to set busy timeout: {}", e);
            format!("Failed to set busy timeout: {e}")
        })?;
    debug!("Enabling foreign key constraints");
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| {
            error!("Failed to enable foreign keys: {}", e);
            format!("Failed to enable foreign keys: {e}")
        })?;
    debug!("Applying database migrations");
    migrations::apply(&mut conn, now_ts)?;
    info!("Database connection initialized successfully");
    Ok(conn)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger once for the Tauri side; default to debug for our crate if not set.
    tracing_log::LogTracer::init().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("tables=debug")))
        .try_init()
        .ok();

    info!("Starting Tables Tauri backend");

    debug!("Initializing Tauri application builder");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            debug!("Setting up Tauri application");
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| {
                    error!("Failed to resolve app data dir: {}", e);
                    format!("Failed to resolve app data dir: {e}")
                })?;
            let db_path = app_data_dir.join("tables.db");
            debug!("App data directory: {}", app_data_dir.display());
            debug!("Database path: {}", db_path.display());
            let conn = init_connection(&db_path)?;
            app.manage(DatabaseState {
                conn: Arc::new(Mutex::new(conn)),
            });

            debug!("Initializing credential manager");
            // Initialize CredentialManager
            let credential_manager = Arc::new(CredentialManager::new(&app_data_dir, app.state::<DatabaseState>().conn.clone())
                .expect("Failed to initialize credential manager"));

            debug!("Initializing connection manager");
            // Initialize connection manager state
            app.manage(ConnectionManagerState {
                credential_manager: credential_manager.clone(),
                active_connections: Arc::new(Mutex::new(HashMap::new())),
            });

            debug!("Initializing adapter registry");
            adapter_registry::init_builtins();

            debug!("Initializing completion engine");
            // Initialize completion state for SQL auto-completion
            app.manage(CompletionState::default());

            debug!("Initializing plugin system");
            // Initialize plugin system and discover all plugins
            let discovery = PluginDiscovery::new();
            debug!("Discovering builtin plugins");
            discovery.discover_builtin_plugins()
                .map_err(|e| {
                    error!("Failed to discover plugins: {}", e);
                    format!("Failed to discover plugins: {e}")
                })?;

            // Initialize all enabled plugins
            let init_results = discovery.initialize_plugins(app.handle());
            let mut failed_plugins = Vec::new();
            for result in init_results {
                if !result.success {
                    error!("Plugin initialization failed: {} - {}", result.plugin_name, result.message);
                    failed_plugins.push(format!("{} - {}", result.plugin_name, result.message));
                } else {
                    info!("Plugin initialized: {} ({} commands)", result.plugin_name, result.commands_registered);
                }
            }

            // Fail fast if critical plugins fail to initialize
            if !failed_plugins.is_empty() {
                warn!("Critical plugin initialization failures: {}", failed_plugins.join(", "));
                return Err(format!("Critical plugin initialization failures: {}", failed_plugins.join(", ")).into());
            }

            debug!("Configuring main window");
            // Dynamically size the main window to ~100% of the current monitor and center it.
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let screen_size = monitor.size();
                    let width = (screen_size.width as f64 * 1.0) as u32;
                    let height = (screen_size.height as f64 * 1.0) as u32;
                    let x = (screen_size.width as i32 - width as i32) / 2;
                    let y = (screen_size.height as i32 - height as i32) / 2;

                    debug!("Resizing window to {}x{} and centering", width, height);
                    let _ = window.set_size(Size::Physical(PhysicalSize { width, height }));
                    let _ =
                        window.set_position(tauri::Position::Physical(PhysicalPosition { x, y }));
                    
                    #[cfg(not(target_os = "macos"))]
                    {
                        debug!("Disabling decorations for main window on non-macOS");
                        let _ = window.set_decorations(false);
                    }
                }
            }

            
            // --- Metrics System Setup ---
            debug!("Initializing metrics system");
            let registry = Arc::new(MetricsRegistry::new());
            app.manage(registry.clone()); // Optional: Allow commands to access registry via state
            
            if constants::ENABLE_METRICS_EMISSION {
                // Start self-registering system monitor
                let monitor = SystemMonitor::new(&registry);
                monitor.run();

                // Start emitter thread
                start_metrics_emitter(app.handle().clone(), registry.clone());

                // "Welcome Push": Listen for new windows and immediately emit current snapshot
                let handle = app.handle().clone();
                let reg_clone = registry.clone();
                app.listen("window-created", move |_| {
                    info!("Window created, emitting welcome metrics snapshot");
                    let snapshot = reg_clone.snapshot();
                    if let Err(e) = handle.emit("metrics:snapshot", &snapshot) {
                        error!("Failed to emit welcome metrics snapshot: {}", e);
                    }
                });
            } else {
                debug!("Metrics emission disabled, skipping system monitor and emitter");
            }

            info!("Application setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                debug!("Window destroyed event for {}", label);
                
                // Cleanup active connection tracking for this window
                let app = window.app_handle();
                if let (Some(db_state), Some(conn_state)) = (app.try_state::<DatabaseState>(), app.try_state::<ConnectionManagerState>()) {
                    let manager = ConnectionManager::from_state(&db_state, &conn_state);
                    manager.remove_window_from_active(&label);
                    
                    // Emit global event with updated active IDs
                    let _ = app.emit("active-connections-changed", manager.get_active_connection_ids());
                }

                let _ = app.emit("window-destroyed", label);
            }
        })
        .invoke_handler(aggregate_plugin_commands!())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| match event {
            tauri::RunEvent::ExitRequested { .. } => {
                // Handle app exit if needed
            }
            _ => {}
        });
}
