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
    let (connection, _initial_creds) = manager.get_connection(&connection_id)?;
    
    // 2. Parse config
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    // 3. Dispatch based on engine
    let introspector = Introspector::new(db_state.conn.clone());

    match connection.engine.as_str() {
        "sqlite" => {
            let sqlite_path = config.get("file")
                .and_then(|v| v.as_str())
                .ok_or("Missing SQLite file path in config")?;
            introspector.introspect_sqlite(&connection_id, sqlite_path)?;
        },
        "postgres" | "postgresql" => {
            // Inject secure credentials (password) into config
             // ConnectionManager get_connection returns credentials with the connection, but get_connection was called above and returned `(connection, _credentials)`.
             // Actually, `_credentials` variable holds them.
            
             // Re-fetch credentials properly since I ignored them in line 18
             let (_, credentials) = manager.get_connection(&connection_id)?;
             
             if let Some(db) = config.get_mut("db") {
                 if let Some(db_obj) = db.as_object_mut() {
                     if let Some(password) = &credentials.password {
                         debug!("Injecting password from secure credentials into connection config for introspection");
                         db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
                     }
                 }
             }
             
            introspector.introspect_postgres(&connection_id, config).await?;
        },
        _ => {
            return Err(format!("Engine '{}' is not supported for introspection currently", connection.engine));
        }
    }

    info!("Schema refresh finished for connection {}", connection_id);
    Ok(())
}

#[tauri::command]
pub async fn get_schema(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaDatabase>, String> {
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
    database: Option<String>,
    schema: Option<String>,
    table_name: String,
    db_state: State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let database = database.unwrap_or_else(|| "main".to_string());
    let schema = schema.unwrap_or_else(|| "main".to_string());
    debug!("Fetching cached details for table {}.{}.{} in connection {}", database, schema, table_name, connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_table_details(&connection_id, &database, &schema, &table_name)
}
