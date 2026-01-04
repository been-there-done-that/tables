use tauri::State;
use crate::DatabaseState;
use crate::introspection::{Introspector, MetaTable, MetaDatabase, MetaSchema};
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
pub async fn get_databases(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaDatabase>, String> {
    debug!("Fetching cached databases for connection {}", connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_databases(&connection_id)
}

#[tauri::command]
pub async fn get_schemas(
    connection_id: String,
    database: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaSchema>, String> {
    debug!("Fetching cached schemas for {}.{} ", connection_id, database);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_schemas(&connection_id, &database)
}

#[tauri::command]
pub async fn get_tables_in_schema(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaTable>, String> {
    debug!("Fetching cached tables for {}.{}.{} ", connection_id, database, schema);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_tables_in_schema(&connection_id, &database, &schema)
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
#[tauri::command]
pub async fn introspect_database(
    connection_id: String,
    database_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: tauri::AppHandle,
) -> Result<MetaDatabase, String> {
    info!("Command: introspect_database for {} in connection {}", database_name, connection_id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    // 1. Get connection info
    let (connection, credentials) = manager.get_connection(&connection_id)?;
    
    // 2. Parse config
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    // 3. Inject password if available
    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
            }
        }
    }

    let introspector = Introspector::new(db_state.conn.clone());
    introspector.introspect_database(&connection_id, config, &database_name, &app).await
}

/// Progressive schema introspection with level-based event emission
/// Level 1: Databases
/// Level 2: Schemas  
/// Level 3: Tables + Columns
/// Level 4: FK + Indexes + Triggers
#[tauri::command]
pub async fn refresh_schema_progressive(
    connection_id: String,
    priority_database: Option<String>,
    priority_schema: Option<String>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Emitter;
    
    info!("Starting progressive schema refresh for connection {}", connection_id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    let (connection, credentials) = manager.get_connection(&connection_id)?;
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    // Inject password if available
    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
            }
        }
    }

    let introspector = Introspector::new(db_state.conn.clone());
    
    match connection.engine.as_str() {
        "postgres" | "postgresql" => {
            introspector.introspect_postgres_progressive(&connection_id, config, priority_database, priority_schema, &app).await?;
        },
        "sqlite" => {
            let sqlite_path = config.get("file")
                .and_then(|v| v.as_str())
                .ok_or("Missing SQLite file path in config")?;
            // For SQLite, we do all-at-once but still emit events for each level
            introspector.introspect_sqlite(&connection_id, sqlite_path)?;
            // Emit completion event for all levels
            for level in 1..=4 {
                let _ = app.emit("schema:level-complete", serde_json::json!({
                    "level": level,
                    "connection_id": &connection_id,
                }));
            }
        },
        _ => {
            return Err(format!("Engine '{}' not supported for progressive introspection", connection.engine));
        }
    }

    info!("Progressive schema refresh finished for connection {}", connection_id);
    Ok(())
}

#[tauri::command]
pub async fn refresh_schema_specific_progressive(
    connection_id: String,
    database_name: String,
    schema_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    info!("Starting specific schema refresh for {}.{} in connection {}", database_name, schema_name, connection_id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    let (connection, credentials) = manager.get_connection(&connection_id)?;
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
            }
        }
    }

    let introspector = Introspector::new(db_state.conn.clone());
    
    match connection.engine.as_str() {
        "postgres" | "postgresql" => {
            introspector.introspect_postgres_schema_progressive(&connection_id, config, &database_name, &schema_name, &app).await?;
        },
        _ => {
            return Err(format!("Engine '{}' not supported for specific schema refresh", connection.engine));
        }
    }

    Ok(())
}
