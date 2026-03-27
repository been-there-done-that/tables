use tauri::State;
use crate::DatabaseState;
use crate::introspection::{Introspector, MetaTable, MetaDatabase, MetaSchema, MetaFunction, FunctionKind, MetaSequence, MetaConstraint, ConstraintKind};
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use log::{debug, info};
use crate::constants::ENABLE_INTROSPECTION_EVENTS;

pub(super) async fn pg_connect_for_commands(
    connection_id: &str,
    database: &str,
    db_state: &tauri::State<'_, DatabaseState>,
    conn_state: &tauri::State<'_, ConnectionManagerState>,
) -> Result<tokio_postgres::Client, String> {
    let manager = ConnectionManager::from_state(db_state, conn_state);
    let (connection, credentials) = manager.get_connection(connection_id)?;
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Config parse error: {}", e))?;

    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
            }
            // Override database to the requested one
            db_obj.insert("database".to_string(), serde_json::Value::String(database.to_string()));
        }
    }

    let db_config = config.get("db").ok_or("Missing db config")?;
    let host = db_config.get("host").and_then(|v| v.as_str()).unwrap_or("localhost");
    let port = db_config.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
    let user = db_config.get("username").and_then(|v| v.as_str()).unwrap_or("postgres");
    let pass = db_config.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let db = db_config.get("database").and_then(|v| v.as_str()).unwrap_or("postgres");
    let use_tls = config.get("tls").and_then(|t| t.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);

    let mut pg_config = tokio_postgres::Config::new();
    pg_config.host(host);
    pg_config.port(port);
    pg_config.user(user);
    pg_config.password(pass);
    pg_config.dbname(db);

    if use_tls {
        let tls = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("TLS error: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls);
        let (client, conn) = pg_config.connect(connector).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move { let _ = conn.await; });
        Ok(client)
    } else {
        let (client, conn) = pg_config.connect(tokio_postgres::NoTls).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move { let _ = conn.await; });
        Ok(client)
    }
}

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
            if ENABLE_INTROSPECTION_EVENTS {
                for level in 1..=4 {
                    let _ = app.emit("schema:level-complete", serde_json::json!({
                        "level": level,
                        "connection_id": &connection_id,
                    }));
                }
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

#[tauri::command]
pub async fn get_functions(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaFunction>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let rows = client.query(
        "SELECT
            p.proname AS name,
            l.lanname AS language,
            p.prokind AS kind,
            COALESCE(pg_get_function_result(p.oid), '') AS return_type,
            COALESCE(p.prosrc, pg_get_functiondef(p.oid)::text, '') AS definition,
            p.prosecdef AS security_definer,
            CASE p.provolatile
                WHEN 'v' THEN 'volatile'
                WHEN 's' THEN 'stable'
                WHEN 'i' THEN 'immutable'
                ELSE 'volatile'
            END AS volatility
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        JOIN pg_language l ON p.prolang = l.oid
        WHERE n.nspname = $1
          AND p.prokind IN ('f', 'p')
        ORDER BY p.proname",
        &[&schema],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let functions = rows.iter().map(|row| {
        let kind_char: i8 = row.get::<_, i8>(2);
        let kind = match kind_char as u8 as char {
            'p' => FunctionKind::Procedure,
            'a' => FunctionKind::Aggregate,
            'w' => FunctionKind::Window,
            _ => FunctionKind::Function,
        };
        MetaFunction {
            connection_id: connection_id.clone(),
            database: database.clone(),
            name: row.get(0),
            schema: schema.clone(),
            language: row.get(1),
            kind,
            return_type: row.get(3),
            definition: row.get(4),
            security_definer: row.get(5),
            volatility: row.get(6),
            arguments: vec![],
        }
    }).collect();

    Ok(functions)
}

#[tauri::command]
pub async fn get_sequences(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaSequence>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;

    let rows = client.query(
        "SELECT
            sequencename AS name,
            data_type,
            start_value,
            min_value,
            max_value,
            increment_by,
            cycle,
            cache_size,
            last_value
        FROM pg_sequences
        WHERE schemaname = $1
        ORDER BY sequencename",
        &[&schema],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let sequences = rows.iter().map(|row| MetaSequence {
        connection_id: connection_id.clone(),
        database: database.clone(),
        name: row.get(0),
        schema: schema.clone(),
        data_type: row.get(1),
        start_value: row.get(2),
        min_value: row.get(3),
        max_value: row.get(4),
        increment_by: row.get(5),
        cycle: row.get(6),
        cache_size: row.get(7),
        last_value: row.get(8),
    }).collect();

    Ok(sequences)
}

#[tauri::command]
pub async fn get_constraints(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<MetaConstraint>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);

    let rows = client.query(
        "SELECT
            c.conname AS name,
            c.contype AS kind,
            pg_get_constraintdef(c.oid) AS definition,
            ARRAY(
                SELECT a.attname
                FROM pg_attribute a
                WHERE a.attrelid = c.conrelid
                  AND a.attnum = ANY(c.conkey)
                  AND a.attnum > 0
                ORDER BY array_position(c.conkey, a.attnum)
            ) AS columns
        FROM pg_constraint c
        WHERE c.conrelid = $1::regclass
          AND c.contype IN ('c', 'u', 'x')
        ORDER BY c.conname",
        &[&qualified],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    let constraints = rows.iter().map(|row| {
        let kind_char: i8 = row.get::<_, i8>(1);
        let kind = match kind_char as u8 as char {
            'u' => ConstraintKind::Unique,
            'x' => ConstraintKind::Exclusion,
            _ => ConstraintKind::Check,
        };
        let columns: Vec<String> = row.get(3);
        MetaConstraint {
            connection_id: connection_id.clone(),
            database: database.clone(),
            schema: schema.clone(),
            table_name: table_name.clone(),
            name: row.get(0),
            kind,
            definition: row.get(2),
            columns,
        }
    }).collect();

    Ok(constraints)
}

#[tauri::command]
pub async fn get_index_details(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<crate::introspection::MetaIndex>, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);

    let rows = client.query(
        "SELECT
            i.relname AS index_name,
            am.amname AS index_type,
            ix.indisunique AS is_unique,
            ix.indisprimary AS is_primary,
            pg_get_indexdef(ix.indexrelid) AS definition,
            ARRAY(
                SELECT a.attname
                FROM pg_attribute a
                WHERE a.attrelid = t.oid
                  AND a.attnum = ANY(ix.indkey)
                  AND a.attnum > 0
                ORDER BY array_position(ix.indkey::smallint[], a.attnum)
            ) AS columns,
            pg_get_expr(ix.indpred, ix.indrelid) AS predicate
        FROM pg_index ix
        JOIN pg_class t ON t.oid = ix.indrelid
        JOIN pg_class i ON i.oid = ix.indexrelid
        JOIN pg_am am ON am.oid = i.relam
        WHERE t.oid = $1::regclass
        ORDER BY i.relname",
        &[&qualified],
    ).await.map_err(|e| format!("Query error: {}", e))?;

    // NOTE: Expression-based index columns (e.g. lower(name)) have attnum=0 in indkey
    // and are silently excluded from the columns array. The columns array will be
    // shorter than expected for such indexes. has_expressions is not tracked.
    let indexes = rows.iter().map(|row| {
        let columns: Vec<String> = row.get(5);
        crate::introspection::MetaIndex {
            connection_id: connection_id.clone(),
            database: database.clone(),
            schema: schema.clone(),
            table_name: table_name.clone(),
            index_name: row.get(0),
            is_unique: row.get(2),
            is_primary: row.get(3),
            index_type: row.get(1),
            columns,
            predicate: row.get(6),
            definition: row.get(4),
        }
    }).collect();

    Ok(indexes)
}
