use tauri::{State, Emitter};
use tauri::AppHandle;
use crate::DatabaseState;
use crate::introspection::{Introspector, MetaTable, MetaSchema};
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use log::{debug, info};

#[tauri::command]
pub async fn refresh_schema(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Refreshing schema for connection {}", connection_id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    // 1. Get connection info
    let (connection, _credentials) = manager.get_connection(&connection_id)?;
    
    if connection.engine != "sqlite" {
        return Err("Only SQLite is supported for introspection currently".to_string());
    }

    // 2. Parse config to get file path
    let config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;
    
    let sqlite_path = config.get("file")
        .and_then(|v| v.as_str())
        .ok_or("Missing SQLite file path in config")?;

    // 3. Run introspection (Hard Refresh)
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.introspect_sqlite(&connection_id, sqlite_path)?;

    info!("Schema refresh finished for connection {}", connection_id);
    Ok(())
}

#[tauri::command]
pub async fn get_schema(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaSchema>, String> {
    debug!("Fetching cached schema for connection {}", connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_schema(&connection_id)
}

#[tauri::command]
pub async fn get_schema_tables(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaTable>, String> {
    debug!("Fetching cached tables for connection {}", connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_tables(&connection_id)
}

#[tauri::command]
pub async fn get_schema_table_details(
    connection_id: String,
    schema: Option<String>,
    table_name: String,
    db_state: State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let schema = schema.unwrap_or_else(|| "main".to_string());
    debug!("Fetching cached details for table {}.{} in connection {}", schema, table_name, connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_table_details(&connection_id, &schema, &table_name)
}
