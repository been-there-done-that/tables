use tauri::{State, AppHandle, Emitter, Manager};
use serde::{Deserialize, Serialize};
use log::{debug, error, info, warn};
use crate::{DatabaseState, ConnectionManager, ConnectionManagerState};
use std::time::Instant;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// State for tracking running queries per connection
pub struct QueryExecutionState {
    /// Active queries: connection_id → CancellationToken
    pub active_queries: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl Default for QueryExecutionState {
    fn default() -> Self {
        Self {
            active_queries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

/// Cancel an active query for a connection
#[tauri::command]
pub async fn cancel_query(
    connection_id: String,
    query_state: State<'_, QueryExecutionState>,
    app: AppHandle,
) -> Result<bool, String> {
    info!("[cancel_query] Attempting to cancel query for connection: {}", connection_id);
    
    let mut queries = query_state.active_queries.lock().await;
    if let Some(token) = queries.remove(&connection_id) {
        token.cancel();
        info!("[cancel_query] Successfully cancelled query for connection: {}", connection_id);
        
        // Emit cancellation event
        let _ = app.emit("query-cancelled", serde_json::json!({
            "connectionId": connection_id,
            "timestamp": crate::now_ts() * 1000
        }));
        
        Ok(true)
    } else {
        debug!("[cancel_query] No active query to cancel for connection: {}", connection_id);
        Ok(false)
    }
}

/// Result of a general query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub rows: Vec<serde_json::Value>,
    pub columns: Vec<ColumnInfo>,
    pub affected_rows: Option<u64>,
    pub duration_ms: u64,
}

/// Result of a table preview query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePreviewResult {
    pub rows: Vec<serde_json::Value>,
    pub columns: Vec<ColumnInfo>,
    pub total: Option<i64>,  // Only present when fetchTotal=true
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryLogEntry {
    pub id: Option<i64>, // nullable for running events
    pub timestamp: i64,
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
    pub database: String,
    pub query: String,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<i64>, // nullable for running
    pub status: String,
    pub error: Option<String>,
    #[serde(rename = "rows")]
    pub row_count: Option<i64>,
}

/// Fetches recent query logs
#[tauri::command]
pub fn fetch_query_logs(
    limit: i64,
    connection_id: Option<String>,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<QueryLogEntry>, String> {
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, connection_id, database_name, query_text, duration_ms, status, error_message, row_count 
         FROM query_logs 
         WHERE (?1 IS NULL OR connection_id = ?1)
         ORDER BY timestamp DESC 
         LIMIT ?2",
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(rusqlite::params![connection_id, limit], |row| {
        Ok(QueryLogEntry {
            id: Some(row.get(0)?),
            timestamp: row.get(1)?,
            correlation_id: "historical".to_string(), // Historical logs don't have correlation_id stored yet
            connection_id: row.get(2)?,
            database: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            query: row.get(4)?,
            duration_ms: Some(row.get(5)?),
            status: row.get(6)?,
            error: row.get(7)?,
            row_count: row.get(8)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut logs = Vec::new();
    for row in rows {
        logs.push(row.map_err(|e| e.to_string())?);
    }
    
    logs.reverse();

    Ok(logs)
}

/// Clears query logs for a specific connection
#[tauri::command]
pub fn clear_query_logs(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<(), String> {
    debug!("Clearing query logs for connection: {}", connection_id);
    let conn = db_state.conn.lock().map_err(|e| e.to_string())?;
    
    conn.execute(
        "DELETE FROM query_logs WHERE connection_id = ?1",
        rusqlite::params![connection_id],
    ).map_err(|e| {
        error!("Failed to clear query logs for {}: {}", connection_id, e);
        format!("Failed to clear query logs: {}", e)
    })?;

    Ok(())
}

/// Column metadata from query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub column_type: String,
}

/// Fetches a preview of table data using SELECT * with LIMIT/OFFSET
/// Supports both PostgreSQL and SQLite engines
/// Uses connection pooling via get_or_create_adapter for connection reuse
#[tauri::command]
pub async fn fetch_table_preview(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    offset: i64,
    limit: i64,
    where_clause: Option<String>,
    order_by_clause: Option<String>,
    fetch_total: Option<bool>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    query_state: State<'_, QueryExecutionState>,
    app: AppHandle,
) -> Result<TablePreviewResult, String> {
    info!(
        "Fetching table preview for {}.{}.{} (offset={}, limit={}, where={:?}, order={:?})",
        database, schema, table_name, offset, limit, where_clause, order_by_clause
    );
    
    let correlation_id = uuid::Uuid::new_v4().to_string();
    let start = std::time::Instant::now();
    
    // Create cancellation token and register it
    let cancel_token = CancellationToken::new();
    {
        let mut queries = query_state.active_queries.lock().await;
        // Cancel any existing query for this connection
        if let Some(old_token) = queries.remove(&connection_id) {
            old_token.cancel();
        }
        queries.insert(connection_id.clone(), cancel_token.clone());
    }
    
    // Cleanup function to remove token when done
    let cleanup_token = || async {
        let mut queries = query_state.active_queries.lock().await;
        queries.remove(&connection_id);
    };
    
    // Build WHERE and ORDER BY clauses
    let where_part = match &where_clause {
        Some(w) if !w.trim().is_empty() => format!(" WHERE {}", w),
        _ => String::new(),
    };
    let order_part = match &order_by_clause {
        Some(o) if !o.trim().is_empty() => format!(" ORDER BY {}", o),
        _ => String::new(),
    };

    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    // Get connection metadata to determine engine type (doesn't fetch credentials)
    let connection = match manager.get_connection_metadata(&connection_id) {
        Ok(res) => res,
        Err(e) => {
            cleanup_token().await;
            return Err(e);
        }
    };

    // Use connection pooling - get or create adapter
    let adapter = match manager.get_or_create_adapter(&connection_id).await {
        Ok(a) => a,
        Err(e) => {
            cleanup_token().await;
            error!("Failed to get adapter: {}", e);
            return Err(e);
        }
    };

    // Ensure we're connected to the right database
    if let Err(e) = adapter.ensure_database(Some(&database)).await {
        cleanup_token().await;
        error!("Failed to ensure database: {}", e);
        return Err(format!("Failed to switch to database '{}': {}", database, e));
    }

    let should_fetch_total = fetch_total.unwrap_or(false);

    // Build query based on engine
    let (data_query, count_query) = match connection.engine.as_str() {
        "postgres" | "postgresql" => {
            let dq = format!(
                "SELECT * FROM \"{}\".\"{}\"{}{}  LIMIT {} OFFSET {}",
                schema, table_name, where_part, order_part, limit, offset
            );
            let cq = format!("SELECT COUNT(*) FROM \"{}\".\"{}\"{}",  schema, table_name, where_part);
            (dq, cq)
        }
        "sqlite" => {
            let dq = format!(
                "SELECT * FROM \"{}\"{}{}  LIMIT {} OFFSET {}",
                table_name, where_part, order_part, limit, offset
            );
            let cq = format!("SELECT COUNT(*) FROM \"{}\"{}",  table_name, where_part);
            (dq, cq)
        }
        _ => {
            cleanup_token().await;
            return Err(format!("Unsupported engine: {}", connection.engine));
        }
    };

    emit_query_start(&app, &correlation_id, &connection_id, &database, &data_query);

    // Check if already cancelled before starting query
    if cancel_token.is_cancelled() {
        cleanup_token().await;
        return Err("Query cancelled".to_string());
    }

    // Fetch total count if requested
    let total: Option<i64> = if should_fetch_total {
        match adapter.query(&count_query).await {
            Ok(result) => {
                result.rows.first()
                    .and_then(|row| row.get("count").or_else(|| row.as_object()?.values().next()))
                    .and_then(|v| v.as_i64())
            }
            Err(e) => {
                warn!("Failed to fetch count: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Check cancellation before main query
    if cancel_token.is_cancelled() {
        cleanup_token().await;
        let duration = start.elapsed().as_millis() as u64;
        log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "cancelled", Some("Query cancelled by user"), None);
        return Err("Query cancelled".to_string());
    }

    // Execute main query with cancellation support using tokio::select!
    let query_future = adapter.query(&data_query);
    let cancel_future = cancel_token.cancelled();
    
    let result = tokio::select! {
        query_result = query_future => query_result,
        _ = cancel_future => {
            cleanup_token().await;
            let duration = start.elapsed().as_millis() as u64;
            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "cancelled", Some("Query cancelled by user"), None);
            return Err("Query cancelled".to_string());
        }
    };
    
    let duration = start.elapsed().as_millis() as u64;
    cleanup_token().await;

    match result {
        Ok(query_result) => {
            let columns: Vec<ColumnInfo> = query_result.columns.iter().map(|c| {
                ColumnInfo {
                    name: c.name.clone(),
                    column_type: c.column_type.clone(),
                }
            }).collect();

            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "success", None, Some(query_result.rows.len()));

            Ok(TablePreviewResult {
                rows: query_result.rows,
                columns,
                total,
                duration_ms: duration,
            })
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "error", Some(&err_msg), None);
            Err(err_msg)
        }
    }
}



/// Executes a generic SQL query
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    database: String,
    schema: String, // Context, used for search path or logging
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: AppHandle,
) -> Result<QueryResult, String> {
    info!("Executing query on {}: {}", database, query);
    
    let correlation_id = uuid::Uuid::new_v4().to_string();
    emit_query_start(&app, &correlation_id, &connection_id, &database, &query);

    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let (connection, credentials) = match manager.get_connection(&connection_id) {
        Ok(res) => res,
        Err(e) => {
             log_query_end(&app, &correlation_id, &connection_id, &database, &query, 0, "error", Some(&e), None);
             return Err(e);
        }
    };

    let config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| {
            let err_msg = format!("Failed to parse connection config: {}", e);
            log_query_end(&app, &correlation_id, &connection_id, &database, &query, 0, "error", Some(&err_msg), None);
            err_msg
        })?;

    let start = Instant::now();
    let result = match connection.engine.as_str() {
        "postgres" | "postgresql" => {
            execute_postgres_query(&config, &credentials, &database, &schema, &query).await
        }
        "sqlite" => {
            execute_sqlite_query(&config, &query)
        }
        _ => Err(format!("Engine '{}' is not supported for query execution", connection.engine)),
    };
    let duration = start.elapsed().as_millis() as u64;

    let status = if result.is_ok() { "success" } else { "error" };
    let error = result.as_ref().err().map(|e| e.as_str());
    let row_count = result.as_ref().map(|r| r.rows.len()).ok();

    log_query_end(&app, &correlation_id, &connection_id, &database, &query, duration, status, error, row_count);

    result
}

// ... helper functions (fetch_postgres_preview, fetch_sqlite_preview, etc.) ...
// WE NEED TO INCLUDE THE HELPERS HERE OR THE FILE WILL BE INCOMPLETE
// Since I'm using write_to_file, I must include EVERYTHING.

async fn fetch_postgres_preview(
    config: &serde_json::Value,
    credentials: &crate::connection::SecureCredentials,
    database: &str,
    schema: &str,
    table_name: &str,
    offset: i64,
    limit: i64,
    where_clause: Option<&str>,
    order_by_clause: Option<&str>,
    fetch_total: bool,
) -> Result<TablePreviewResult, String> {
    let db = config.get("db").ok_or("Missing 'db' config")?;
    let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
    let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
    let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
    let password = credentials.password.as_ref()
        .map(|p| p.expose().to_string())
        .or_else(|| db.get("password").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();

    let tls_enabled = config.get("tls")
        .and_then(|t| t.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database);

    let client: tokio_postgres::Client = if tls_enabled {
        debug!("Table preview with TLS enabled");
        let tls_connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
        let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
            .map_err(|e| {
                error!("Postgres TLS connection failed: {:?}", e);
                format!("Connection error: {}", e)
            })?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });
        client
    } else {
        debug!("Table preview without TLS");
        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| {
                error!("Postgres connection failed: {:?}", e);
                format!("Connection error: {}", e)
            })?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });
        client
    };

    // Build WHERE clause for queries
    let where_part = match where_clause {
        Some(w) if !w.trim().is_empty() => format!(" WHERE {}", w),
        _ => String::new(),
    };

    // Get total count only if requested
    let total: Option<i64> = if fetch_total {
        let count_query = format!(
            "SELECT COUNT(*) FROM \"{}\".\"{}\"{}",
            schema, table_name, where_part
        );
        let count_row = client.query_one(&count_query, &[]).await
            .map_err(|e| crate::pg_utils::format_postgres_error(&e))?;
        Some(count_row.get(0))
    } else {
        None
    };

    // Build ORDER BY clause
    let order_part = match order_by_clause {
        Some(o) if !o.trim().is_empty() => format!(" ORDER BY {}", o),
        _ => String::new(),
    };

    // Fetch rows with limit/offset
    let data_query = format!(
        "SELECT * FROM \"{}\".\"{}\"{}{}  LIMIT {} OFFSET {}",
        schema, table_name, where_part, order_part, limit, offset
    );
    let rows = client.query(&data_query, &[]).await
        .map_err(|e| crate::pg_utils::format_postgres_error(&e))?;

    // Extract column info from the first row or query
    let columns: Vec<ColumnInfo> = if !rows.is_empty() {
        rows[0].columns().iter().map(|col| {
            ColumnInfo {
                name: col.name().to_string(),
                column_type: format!("{:?}", col.type_()),
            }
        }).collect()
    } else {
        // Query column info from information_schema if no rows
        let cols_query = format!(
            "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position"
        );
        let col_rows = client.query(&cols_query, &[&schema, &table_name]).await
            .map_err(|e| crate::pg_utils::format_postgres_error(&e))?;
        col_rows.iter().map(|row| {
            ColumnInfo {
                name: row.get(0),
                column_type: row.get(1),
            }
        }).collect()
    };

    // Convert rows to JSON
    let json_rows: Vec<serde_json::Value> = rows.iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let value = postgres_value_to_json(row, i);
            obj.insert(col.name().to_string(), value);
        }
        serde_json::Value::Object(obj)
    }).collect();

    Ok(TablePreviewResult {
        rows: json_rows,
        columns,
        total,
        duration_ms: 0, // Set by caller
    })
}

fn fetch_sqlite_preview(
    config: &serde_json::Value,
    table_name: &str,
    offset: i64,
    limit: i64,
    where_clause: Option<&str>,
    order_by_clause: Option<&str>,
    fetch_total: bool,
) -> Result<TablePreviewResult, String> {
    let sqlite_path = config.get("file")
        .and_then(|v| v.as_str())
        .ok_or("Missing SQLite file path in config")?;

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;

    // Build WHERE clause
    let where_part = match where_clause {
        Some(w) if !w.trim().is_empty() => format!(" WHERE {}", w),
        _ => String::new(),
    };

    // Get total count only if requested
    let total: Option<i64> = if fetch_total {
        let count_query = format!("SELECT COUNT(*) FROM \"{}\"{}",  table_name, where_part);
        conn.query_row(&count_query, [], |row| row.get(0))
            .map(Some)
            .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?
    } else {
        None
    };

    // Get column info
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table_name))
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;
    let columns: Vec<ColumnInfo> = stmt.query_map([], |row| {
        Ok(ColumnInfo {
            name: row.get(1)?,
            column_type: row.get(2)?,
        })
    }).map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?
      .filter_map(|r| r.ok())
      .collect();

    // Build ORDER BY clause
    let order_part = match order_by_clause {
        Some(o) if !o.trim().is_empty() => format!(" ORDER BY {}", o),
        _ => String::new(),
    };

    // Fetch rows
    let query = format!("SELECT * FROM \"{}\"{}{}  LIMIT {} OFFSET {}", table_name, where_part, order_part, limit, offset);
    let mut stmt = conn.prepare(&query)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;

    let column_names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        let mut obj = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = crate::sqlite_utils::sqlite_value_to_json(row, i);
            obj.insert(name.clone(), value);
        }
        Ok(serde_json::Value::Object(obj))
    }).map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?
      .filter_map(|r| r.ok())
      .collect();

    Ok(TablePreviewResult {
        rows,
        columns,
        total,
        duration_ms: 0, // Set by caller
    })
}

/// Convert a PostgreSQL row value to JSON using shared utilities
fn postgres_value_to_json(row: &tokio_postgres::Row, idx: usize) -> serde_json::Value {
    let col = &row.columns()[idx];
    crate::pg_utils::pg_value_to_json(row, idx, col)
}

async fn execute_postgres_query(
    config: &serde_json::Value,
    credentials: &crate::connection::SecureCredentials,
    database: &str,
    schema: &str,
    query: &str,
) -> Result<QueryResult, String> {
    let db = config.get("db").ok_or("Missing 'db' config")?;
    let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
    let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
    let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
    let password = credentials.password.as_ref()
        .map(|p| p.expose().to_string())
        .or_else(|| db.get("password").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();

    let tls_enabled = config.get("tls")
        .and_then(|t| t.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database);

    let client: tokio_postgres::Client = if tls_enabled {
        let tls_connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
        let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });
        client
    } else {
        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error: {}", e);
            }
        });
        client
    };

    if !schema.is_empty() && schema != "public" {
        let _ = client.execute(&format!("SET search_path TO \"{}\"", schema), &[]).await;
    }

    let rows = client.query(query, &[]).await
        .map_err(|e| format!("Query failed: {}", e))?;

    let columns: Vec<ColumnInfo> = if !rows.is_empty() {
        rows[0].columns().iter().map(|col| {
            ColumnInfo {
                name: col.name().to_string(),
                column_type: format!("{:?}", col.type_()),
            }
        }).collect()
    } else {
        vec![]
    };

    let json_rows: Vec<serde_json::Value> = rows.iter().map(|row| {
        let mut obj = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let value = postgres_value_to_json(row, i);
            obj.insert(col.name().to_string(), value);
        }
        serde_json::Value::Object(obj)
    }).collect();

    Ok(QueryResult {
        rows: json_rows,
        columns,
        affected_rows: None, // explicit query returns rows
        duration_ms: 0, // calculated by caller
    })
}

fn execute_sqlite_query(
    config: &serde_json::Value,
    query: &str,
) -> Result<QueryResult, String> {
    let sqlite_path = config.get("file")
        .and_then(|v| v.as_str())
        .ok_or("Missing SQLite file path in config")?;

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;

    let mut stmt = conn.prepare(query)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;

    let column_count = stmt.column_count();
    let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
    
    // SQLite doesn't give types easily in dynamic query without PRAGMA or parsing
    // using placeholder types
    let columns: Vec<ColumnInfo> = column_names.iter().map(|name| ColumnInfo {
        name: name.clone(),
        column_type: "UNKNOWN".to_string()
    }).collect();

    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        let mut obj = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = crate::sqlite_utils::sqlite_value_to_json(row, i);
            obj.insert(name.clone(), value);
        }
        Ok(serde_json::Value::Object(obj))
    }).map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?
      .filter_map(|r| r.ok())
      .collect();

    Ok(QueryResult {
        rows,
        columns,
        affected_rows: None,
        duration_ms: 0,
    })
}

fn emit_query_start(
    app: &AppHandle,
    correlation_id: &str,
    connection_id: &str,
    database: &str,
    query: &str,
) {
    let timestamp = crate::now_ts() * 1000;
    
    let _ = app.emit("query-started", serde_json::json!({
        "timestamp": timestamp,
        "correlationId": correlation_id,
        "connectionId": connection_id,
        "database": database,
        "query": query,
        "status": "running"
    }));
}

fn log_query_end(
    app: &AppHandle,
    correlation_id: &str,
    connection_id: &str,
    database: &str,
    query: &str,
    duration_ms: u64,
    status: &str,
    error: Option<&str>,
    row_count: Option<usize>,
) {
    let timestamp = crate::now_ts() * 1000;
    
    // Emit event
    let _ = app.emit("query-log", serde_json::json!({
        "timestamp": timestamp, // End timestamp? or use original? Used for sorting...
        "correlationId": correlation_id,
        "connectionId": connection_id,
        "database": database,
        "query": query,
        "durationMs": duration_ms,
        "status": status,
        "error": error,
        "rows": row_count
    }));

    // Persist to internal DB
    if let Some(db_state) = app.try_state::<DatabaseState>() {
        if let Ok(conn) = db_state.conn.lock() {
            let _ = conn.execute(
                "INSERT INTO query_logs (connection_id, connection_name, database_name, timestamp, query_text, duration_ms, row_count, status, error_message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    connection_id,
                    "unknown", 
                    database,
                    timestamp,
                    query,
                    duration_ms as i64,
                    row_count.map(|r| r as i64),
                    status,
                    error
                ]
            );
        }
    }
}
