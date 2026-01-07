use tauri::{AppHandle, Manager, State};
use crate::DatabaseState;
use crate::connection::Connection;
use log::{info, error};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Simple response for open_internal_db - returns the path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalDbInfo {
    pub db_path: String,
    pub engine: String,
}

#[tauri::command]
pub async fn reset_app_state(state: State<'_, DatabaseState>) -> Result<(), String> {
    info!("Resetting application state...");
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    
    // Disable foreign keys to allow dropping tables out of order
    conn.pragma_update(None, "foreign_keys", "OFF").map_err(|e| e.to_string())?;

    // List of tables to clear
    let tables_to_clear = vec![
        "connections", 
        "settings", 
        "history", 
        "saved_queries",
        "themes",
        // Meta tables for introspection cache
        "meta_columns", "meta_tables", "meta_schemas", "meta_indexes", "meta_foreign_keys", "meta_index_columns"
    ];

    for table in tables_to_clear {
        // Use IF EXISTS to be safe
        let result = conn.execute(&format!("DELETE FROM {} WHERE 1=1", table), []);
        if let Err(e) = result {
            // Log but continue - table might not exist
            error!("Failed to clear table {} (may not exist): {}", table, e);
        }
    }
    
    // Re-enable FKs
    conn.pragma_update(None, "foreign_keys", "ON").map_err(|e| e.to_string())?;
    
    info!("Application state reset complete");
    Ok(())
}

#[tauri::command]
pub async fn open_internal_db(app: AppHandle) -> Result<InternalDbInfo, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("tables.db");
    
    let db_path_str = db_path.to_str().ok_or("Invalid database path")?.to_string();

    Ok(InternalDbInfo {
        db_path: db_path_str,
        engine: "sqlite".to_string(),
    })
}
