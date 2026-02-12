
use tauri::{State, AppHandle, Emitter, Manager};
use serde::{Deserialize, Serialize};
use log::{debug, error, info, warn};
use crate::{DatabaseState, ConnectionManager, ConnectionManagerState};
use std::time::{Instant, Duration};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use crate::completion::parsing::parse_sql;
use crate::completion::ranges::find_all_statement_ranges;

/// Active query tracking with cancellation tokens
pub struct ActiveQuery {
    pub token: CancellationToken,
    pub pg_cancel: Option<tokio_postgres::CancelToken>,
}

/// State for tracking running queries per connection
pub struct QueryExecutionState {
    /// Active queries: connection_id → ActiveQuery
    pub active_queries: Arc<Mutex<HashMap<String, ActiveQuery>>>,
}

impl Default for QueryExecutionState {
    fn default() -> Self {
        Self {
            active_queries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

pub enum DBSession {
    Postgres(Arc<tokio_postgres::Client>),
    Sqlite(Arc<Mutex<rusqlite::Connection>>),
}

/// State for tracking active database sessions per editor window
#[derive(Clone)]
pub struct QuerySessionManager {
    /// Active sessions: (connection_id, session_id) → DBSession
    pub sessions: Arc<Mutex<HashMap<(String, String), DBSession>>>,
}

impl Default for QuerySessionManager {
    fn default() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl QuerySessionManager {
    pub async fn remove_sessions_for_window(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|(_, sid), _| sid != session_id);
    }
}

/// Cancel an active query for a connection
#[tauri::command]
pub async fn cancel_query(
    connection_id: String,
    query_state: State<'_, QueryExecutionState>,
    session_state: State<'_, QuerySessionManager>,
    app: AppHandle,
) -> Result<bool, String> {
    info!("[cancel_query] Attempting to cancel query for connection: {}", connection_id);
    
    let mut queries = query_state.active_queries.lock().await;
    if let Some(active_query) = queries.remove(&connection_id) {
        // 1. Native Postgres cancel if available
        if let Some(pg_token) = active_query.pg_cancel {
            debug!("[cancel_query] Sending native Postgres cancel signal for connection: {}", connection_id);
            // Non-blocking cancel attempt
            let _ = pg_token.cancel_query(tokio_postgres::NoTls).await;
        }

        // 2. Local token cancel to abort the tokio::select
        active_query.token.cancel();

        // 3. FORCE CLOSE SESSIONS for this connection
        // This is the most reliable way to ensure the next query doesn't hang on a busy/corrupted connection
        {
            let mut sessions = session_state.sessions.lock().await;
            sessions.retain(|(cid, _), _| cid != &connection_id);
            debug!("[cancel_query] Cleared active sessions for connection: {}", connection_id);
        }

        info!("[cancel_query] Successfully cancelled query and reset connection state: {}", connection_id);
        
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
    pub duration_ms: u64,
    pub affected_rows: Option<u64>,
    pub total: Option<u64>,
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
    pub source_schema: Option<String>,
    pub source_table: Option<String>,
    pub source_column: Option<String>,
    pub is_primary_key: bool,
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
    component: Option<String>,
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
        if let Some(old_active) = queries.remove(&connection_id) {
            old_active.token.cancel();
        }
        queries.insert(connection_id.clone(), ActiveQuery {
            token: cancel_token.clone(),
            pg_cancel: None,
        });
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
        Ok(a) => {
            // Register native cancel token if Postgres
            if let Some(pg_token) = a.get_pg_cancel_token() {
                let mut queries = query_state.active_queries.lock().await;
                if let Some(active_query) = queries.get_mut(&connection_id) {
                    active_query.pg_cancel = Some(pg_token);
                }
            }
            a
        },
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

    let component_name = component.as_deref().unwrap_or("preview");
    emit_query_start(&app, &correlation_id, &connection_id, &database, &data_query, Some(component_name));

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
        log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "cancelled", Some("Query cancelled by user"), None, Some(component_name));
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
            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "error", Some("Query cancelled by user"), None, Some(component_name));
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
                    source_schema: None, source_table: None, source_column: None, is_primary_key: false,
                }
            }).collect();

            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "success", None, Some(query_result.rows.len()), Some(component_name));

            Ok(TablePreviewResult {
                rows: query_result.rows,
                columns,
                total,
                duration_ms: duration,
            })
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            log_query_end(&app, &correlation_id, &connection_id, &database, &data_query, duration, "error", Some(&err_msg), None, Some(component_name));
            Err(err_msg)
        }
    }
}



/// Executes a generic SQL query
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    session_id: String,
    database: String,
    schema: String, // Context, used for search path or logging
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    session_state: State<'_, QuerySessionManager>,
    query_state: State<'_, QueryExecutionState>,
    app: AppHandle,
    component: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<QueryResult, String> {
    info!("Executing query on {}: {}", database, query);
    
    let correlation_id = uuid::Uuid::new_v4().to_string();
    let component_name = component.as_deref().unwrap_or("editor");
    emit_query_start(&app, &correlation_id, &connection_id, &database, &query, Some(component_name));

    // 1. Check if session already exists to avoid redundant DB hits for metadata/creds
    let mut session_found = None;
    {
        let sessions = session_state.sessions.lock().await;
        if let Some(session) = sessions.get(&(connection_id.clone(), session_id.clone())) {
            session_found = Some(match session {
                DBSession::Postgres(_) => "postgres",
                DBSession::Sqlite(_) => "sqlite",
            });
        }
    }

    // Create cancellation token and register it
    let cancel_token = CancellationToken::new();
    {
        let mut queries = query_state.active_queries.lock().await;
        if let Some(old_active) = queries.remove(&connection_id) {
            old_active.token.cancel();
        }
        queries.insert(connection_id.clone(), ActiveQuery {
            token: cancel_token.clone(),
            pg_cancel: None,
        });
    }
    
    // Cleanup function
    let cleanup_token = || async {
        let mut queries = query_state.active_queries.lock().await;
        queries.remove(&connection_id);
    };

    // Check if cancelled before starting
    if cancel_token.is_cancelled() {
        cleanup_token().await;
        return Err("Query cancelled".to_string());
    }

    let start = Instant::now();

    // Wrap execution in tokio::select for cancellation
    let execution_future = async {
        let engine = if let Some(e) = session_found {
            e.to_string()
        } else {
            let manager = ConnectionManager::from_state(&db_state, &conn_state);
            let connection = manager.get_connection(&connection_id).map(|(c, _)| c.engine)?;
            connection
        };

        // Determine final query text and optional count query
        let (final_query, count_query) = if let (Some(l), Some(offset_val)) = (limit, offset) {
            let trimmed = query.trim().trim_end_matches(';');
            match engine.as_str() {
                "postgres" | "postgresql" => {
                    (
                        format!("SELECT * FROM ({}) AS __subquery LIMIT {} OFFSET {}", trimmed, l, offset_val),
                        Some(format!("SELECT COUNT(*) FROM ({}) AS __total", trimmed))
                    )
                }
                "sqlite" => {
                    (
                        format!("SELECT * FROM ({}) LIMIT {} OFFSET {}", trimmed, l, offset_val),
                        Some(format!("SELECT COUNT(*) FROM ({})", trimmed))
                    )
                }
                _ => (query.clone(), None)
            }
        } else {
            (query.clone(), None)
        };

        let mut res = if let Some(engine_type) = session_found {
            // Session exists, skip connection/credential retrieval from local DB
            match engine_type {
                "postgres" => {
                    execute_postgres_query(&session_state, &query_state, &connection_id, &session_id, None, None, &database, &schema, &final_query).await
                }
                "sqlite" => {
                    execute_sqlite_query(&session_state, &connection_id, &session_id, None, &final_query).await
                }
                _ => Err(format!("Engine '{}' is not supported", engine_type)),
            }
        } else {
            // No session, perform full retrieval from local DB
            let manager = ConnectionManager::from_state(&db_state, &conn_state);
            let (connection, credentials) = match manager.get_connection(&connection_id) {
                Ok(res) => res,
                Err(e) => {
                     log_query_end(&app, &correlation_id, &connection_id, &database, &query, 0, "error", Some(&e), None, Some(component_name));
                     return Err(e);
                }
            };
    
            let config: serde_json::Value = serde_json::from_str(&connection.config_json)
                .map_err(|e| {
                    let err_msg = format!("Failed to parse connection config: {}", e);
                    log_query_end(&app, &correlation_id, &connection_id, &database, &query, 0, "error", Some(&err_msg), None, Some(component_name));
                    err_msg
                })?;
    
            match connection.engine.as_str() {
                "postgres" | "postgresql" => {
                    execute_postgres_query(&session_state, &query_state, &connection_id, &session_id, Some(&config), Some(&credentials), &database, &schema, &final_query).await
                }
                "sqlite" => {
                    execute_sqlite_query(&session_state, &connection_id, &session_id, Some(&config), &final_query).await
                }
                _ => Err(format!("Engine '{}' is not supported for query execution", connection.engine)),
            }
        }?;

        // If we have a count query, run it to get the total
        if let Some(cq) = count_query {
            let total_res = match engine.as_str() {
                "postgres" | "postgresql" | "postgres" => {
                    execute_postgres_query(&session_state, &query_state, &connection_id, &session_id, None, None, &database, &schema, &cq).await
                }
                "sqlite" => {
                    execute_sqlite_query(&session_state, &connection_id, &session_id, None, &cq).await
                }
                _ => Err("Unsupported engine for count".to_string())
            };

            if let Ok(tr) = total_res {
                if let Some(row) = tr.rows.first() {
                    let count = row.get("count")
                        .or_else(|| row.as_object().and_then(|obj| obj.values().next()))
                        .and_then(|v| v.as_u64());
                    res.total = count;
                }
            }
        }

        Ok(res)
    };

    let cancel_future = cancel_token.cancelled();

    let result = tokio::select! {
        res = execution_future => res,
        _ = cancel_future => {
            cleanup_token().await;
            let duration = start.elapsed().as_millis() as u64;
            log_query_end(&app, &correlation_id, &connection_id, &database, &query, duration, "error", Some("Query cancelled by user"), None, Some(component_name));
            return Err("Query cancelled".to_string());
        }
    };

    let duration = start.elapsed().as_millis() as u64;
    cleanup_token().await;

    let status = if result.is_ok() { "success" } else { "error" };
    let error = result.as_ref().err().map(|e| e.as_str());
    let row_count = result.as_ref().map(|r| r.rows.len()).ok();

    log_query_end(&app, &correlation_id, &connection_id, &database, &query, duration, status, error, row_count, Some(component_name));

    // Update the duration in the result before returning
    result.map(|mut r| {
        r.duration_ms = duration;
        r
    })
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

    let mut pg_config = tokio_postgres::Config::new();
    pg_config.host(host);
    pg_config.port(port);
    pg_config.user(user);
    pg_config.password(&password);
    pg_config.dbname(database);
    pg_config.connect_timeout(Duration::from_secs(15));
    pg_config.keepalives(true);
    pg_config.keepalives_idle(Duration::from_secs(30));

    let client: tokio_postgres::Client = if tls_enabled {
        debug!("Table preview with TLS enabled");
        let tls_connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
        let (client, connection) = pg_config.connect(connector).await
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
        let (client, connection) = pg_config.connect(tokio_postgres::NoTls).await
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
                source_schema: None, source_table: None, source_column: None, is_primary_key: false,
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
                source_schema: Some(schema.to_string()),
                source_table: Some(table_name.to_string()),
                source_column: Some(row.get::<_, String>(0)), 
                is_primary_key: false, // TODO: fetch this?
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
            source_schema: None, source_table: Some(table_name.to_string()), source_column: Some(row.get::<_, String>(1)?), is_primary_key: row.get::<_, i32>(5)? > 0,
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

async fn get_or_create_postgres_client(
    session_manager: &QuerySessionManager,
    connection_id: &str,
    session_id: &str,
    config: Option<&serde_json::Value>,
    credentials: Option<&crate::connection::SecureCredentials>,
    database: &str,
) -> Result<Arc<tokio_postgres::Client>, String> {
    let key = (connection_id.to_string(), session_id.to_string());
    
    // 1. Try to get existing
    {
        let sessions = session_manager.sessions.lock().await;
        if let Some(DBSession::Postgres(client)) = sessions.get(&key) {
            return Ok(client.clone());
        }
    }

    // 2. Create new (must have config/creds)
    let config = config.ok_or("Cannot create new session: Missing config")?;
    let credentials = credentials.ok_or("Cannot create new session: Missing credentials")?;
    
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

    let mut pg_config = tokio_postgres::Config::new();
    pg_config.host(host);
    pg_config.port(port);
    pg_config.user(user);
    pg_config.password(&password);
    pg_config.dbname(database);
    pg_config.connect_timeout(Duration::from_secs(15));
    pg_config.keepalives(true);
    pg_config.keepalives_idle(Duration::from_secs(30));

    debug!("Connecting to Postgres at {}:{}/{}", host, port, database);

    let client: tokio_postgres::Client = if tls_enabled {
        let tls_connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
        let (client, connection) = pg_config.connect(connector).await
            .map_err(|e| format!("Connection error: {}", e))?;
        debug!("Tipping off background connection task (TLS)");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error (TLS): {}", e);
            }
        });
        client
    } else {
        let (client, connection) = pg_config.connect(tokio_postgres::NoTls).await
            .map_err(|e| format!("Connection error: {}", e))?;
        debug!("Tipping off background connection task (NoTLS)");
        tauri::async_runtime::spawn(async move {
            if let Err(e) = connection.await {
                error!("Postgres connection error (NoTLS): {}", e);
            }
        });
        client
    };

    let client = Arc::new(client);

    // Store in manager
    {
        let mut sessions = session_manager.sessions.lock().await;
        sessions.insert(key, DBSession::Postgres(client.clone()));
    }

    Ok(client)
}

async fn execute_postgres_query(
    session_manager: &QuerySessionManager,
    query_state: &QueryExecutionState,
    connection_id: &str,
    session_id: &str,
    config: Option<&serde_json::Value>,
    credentials: Option<&crate::connection::SecureCredentials>,
    database: &str,
    _schema: &str,
    query: &str,
) -> Result<QueryResult, String> {
    for attempt in 0..2 {
        let client = get_or_create_postgres_client(session_manager, connection_id, session_id, config, credentials, database).await?;

        // Register native cancel token
        {
            let mut queries = query_state.active_queries.lock().await;
            if let Some(active_query) = queries.get_mut(connection_id) {
                active_query.pg_cancel = Some(client.cancel_token());
            }
        }

        // Split query and execute
        let tree = parse_sql(query, None);
        let statements = tree.map(|t| find_all_statement_ranges(&t, query)).unwrap_or_default();

        let result = if statements.is_empty() {
             execute_single_postgres_query(&client, query).await
        } else {
             let mut last_result = None;
             let mut loop_err = None;
             for (i, range) in statements.iter().enumerate() {
                let stmt_text = &query[range.start_byte..range.end_byte];
                debug!("[execute_postgres_query] Statement {}: {}", i, stmt_text);
                
                let rows_res = match timeout(Duration::from_secs(30), client.query(stmt_text, &[])).await {
                    Ok(res) => res,
                    Err(_) => return Err("Query timed out after 30 seconds".to_string()),
                };
                
                match rows_res {
                    Ok(rows) => {
                         if i == statements.len() - 1 {
                             // Process success for last statement
                             let raw_columns = if !rows.is_empty() {
                                rows[0].columns()
                             } else {
                                &[]
                             };

                             let mut columns: Vec<ColumnInfo> = raw_columns.iter().map(|col| {
                                 ColumnInfo {
                                     name: col.name().to_string(),
                                     column_type: format!("{:?}", col.type_()),
                                     source_schema: None,
                                     source_table: None,
                                     source_column: None,
                                     is_primary_key: false,
                                 }
                             }).collect();

                             // Enrich metadata if we have rows (and thus column info with OIDs)
                             if !raw_columns.is_empty() {
                                 if let Err(e) = enrich_postgres_metadata(&client, raw_columns, &mut columns).await {
                                     warn!("Failed to enrich postgres metadata: {}", crate::pg_utils::format_postgres_error(&e));
                                 }
                             }

                             let json_rows: Vec<serde_json::Value> = rows.iter().map(|row| {
                                 let mut obj = serde_json::Map::new();
                                 for (i, col) in row.columns().iter().enumerate() {
                                     let value = postgres_value_to_json(row, i);
                                     obj.insert(col.name().to_string(), value);
                                 }
                                 serde_json::Value::Object(obj)
                             }).collect();

                             last_result = Some(QueryResult {
                                 rows: json_rows,
                                 columns,
                                 affected_rows: None,
                                 duration_ms: 0,
                                 total: None,
                             });
                         }
                    }
                    Err(e) => {
                         loop_err = Some(crate::pg_utils::format_postgres_error(&e));
                         break; 
                    }
                }
             }
             
             if let Some(e) = loop_err {
                 Err(e)
             } else {
                 Ok(last_result.unwrap_or_else(|| QueryResult {
                     rows: vec![],
                     columns: vec![],
                     affected_rows: None,
                     duration_ms: 0,
                     total: None,
                 }))
             }
        };

        match result {
             Ok(res) => return Ok(res),
             Err(e) => {
                 if client.is_closed() && attempt == 0 {
                      warn!("Postgres connection closed, attempting reconnect...");
                      {
                          let mut sessions = session_manager.sessions.lock().await;
                          sessions.remove(&(connection_id.to_string(), session_id.to_string()));
                      }
                      continue; 
                 }
                 
                 return Err(e);
             }
        }
    }
    
    Err("Connection closed and reconnection failed.".to_string())
}

async fn execute_single_postgres_query(client: &tokio_postgres::Client, query: &str) -> Result<QueryResult, String> {
    let rows = match timeout(Duration::from_secs(30), client.query(query, &[])).await {
        Ok(res) => res.map_err(|e| crate::pg_utils::format_postgres_error(&e))?,
        Err(_) => return Err("Query timed out after 30 seconds".to_string()),
    };

    let raw_columns = if !rows.is_empty() {
        rows[0].columns()
    } else {
        &[]
    };

    let mut columns: Vec<ColumnInfo> = raw_columns.iter().map(|col| {
        ColumnInfo {
            name: col.name().to_string(),
            column_type: format!("{:?}", col.type_()),
            source_schema: None, source_table: None, source_column: None, is_primary_key: false,
        }
    }).collect();

    // Enrich metadata
    if !raw_columns.is_empty() {
        if let Err(e) = enrich_postgres_metadata(client, raw_columns, &mut columns).await {
            warn!("Failed to enrich single query metadata: {}", crate::pg_utils::format_postgres_error(&e));
        }
    }

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
        affected_rows: None,
        duration_ms: 0,
        total: None,
    })
}

async fn get_or_create_sqlite_conn(
    session_manager: &QuerySessionManager,
    connection_id: &str,
    session_id: &str,
    config: Option<&serde_json::Value>,
) -> Result<Arc<Mutex<rusqlite::Connection>>, String> {
    let key = (connection_id.to_string(), session_id.to_string());

    // 1. Try to get existing
    {
        let sessions = session_manager.sessions.lock().await;
        if let Some(DBSession::Sqlite(conn)) = sessions.get(&key) {
            return Ok(conn.clone());
        }
    }

    // 2. Create new
    let config = config.ok_or("Cannot create new session: Missing config")?;
    let sqlite_path = config.get("file")
        .and_then(|v| v.as_str())
        .ok_or("Missing SQLite file path in config")?;

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;
    let conn = Arc::new(Mutex::new(conn));

    // Store in manager
    {
        let mut sessions = session_manager.sessions.lock().await;
        sessions.insert(key, DBSession::Sqlite(conn.clone()));
    }

    Ok(conn)
}

async fn execute_sqlite_query(
    session_manager: &QuerySessionManager,
    connection_id: &str,
    session_id: &str,
    config: Option<&serde_json::Value>,
    query: &str,
) -> Result<QueryResult, String> {
    let conn_arc = get_or_create_sqlite_conn(session_manager, connection_id, session_id, config).await?;
    let conn = conn_arc.lock().await;

    // Split query and execute
    let tree = parse_sql(query, None);
    let statements = tree.map(|t| find_all_statement_ranges(&t, query)).unwrap_or_default();

    if statements.is_empty() {
        return execute_single_sqlite_query(&conn, query);
    }
        let mut last_result = None;
        for (i, range) in statements.iter().enumerate() {
            let stmt_text = &query[range.start_byte..range.end_byte];
            let mut stmt = conn.prepare(stmt_text)
                .map_err(|e| format!("Failed to prepare statement at indices {}-{}: {}", range.start_byte, range.end_byte, e))?;

            let mut columns = Vec::new();
            let mut column_names = Vec::new();
            {
                // Attempt to use columns_with_metadata for richer info if available
                // If the `column_metadata` feature is enabled for rusqlite,
                // `stmt.columns_with_metadata()` would return `Vec<ColumnMetadata>`.
                // Otherwise, `stmt.columns()` returns `Vec<Column>`.
                // We'll try to use `columns_with_metadata` and fall back to `columns` if it doesn't compile
                // or if the feature isn't enabled.
                // For now, we'll use `columns()` and populate basic info,
                // as `columns_with_metadata` is not a standard method on `Statement` without specific features.

                // The `rusqlite::Column` struct itself does not contain source_schema, source_table, source_column.
                // These are typically available through `rusqlite::ColumnMetadata` which requires the "column_metadata" feature.
                // Without that feature, we can only get name and declared type.
                for col in stmt.columns() {
                    let name = col.name().to_string();
                    let decl_type = col.decl_type().map(|s| s.to_string()).unwrap_or_else(|| "UNKNOWN".to_string());

                    // These fields will be None unless `column_metadata` feature is enabled and
                    // we use `columns_with_metadata()` and extract from `ColumnMetadata`.
                    let source_table = None;
                    let source_column = None;
                    let source_schema = None;

                    columns.push(ColumnInfo {
                        name: name.clone(),
                        column_type: decl_type.clone(),
                        source_schema,
                        source_table,
                        source_column,
                        is_primary_key: false,
                    });
                    column_names.push(name);
                }
            }

            // Enrich with PK metadata
            if let Err(e) = enrich_sqlite_metadata(&conn, &mut columns) {
                 warn!("Failed to enrich SQLite metadata: {}", e);
            }

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

            if i == statements.len() - 1 {
                last_result = Some(QueryResult {
                    rows,
                    columns,
                    affected_rows: None,
                    duration_ms: 0,
                    total: None,
                });
            }
        }

        last_result.ok_or_else(|| "No statements found to execute".to_string())
    }


fn execute_single_sqlite_query(conn: &rusqlite::Connection, query: &str) -> Result<QueryResult, String> {
    let mut stmt = conn.prepare(query)
        .map_err(|e| crate::sqlite_utils::format_sqlite_error(&e))?;

    let mut columns = Vec::new();
    let mut column_names = Vec::new();
    {
        for col in stmt.columns() {
             let name = col.name().to_string();
             let decl_type = col.decl_type().map(|s| s.to_string()).unwrap_or_else(|| "UNKNOWN".to_string());
             
             let source_table = None;
             let source_column = None;
             let source_schema = None;
             
             columns.push(ColumnInfo {
                 name: name.clone(),
                 column_type: decl_type.clone(),
                 source_schema,
                 source_table,
                 source_column,
                 is_primary_key: false,
             });
             column_names.push(name);
        }
    }

    // Enrich with PK metadata
    if let Err(e) = enrich_sqlite_metadata(conn, &mut columns) {
        warn!("Failed to enrich SQLite single query metadata: {}", e);
    }

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
        total: None,
    })
}

fn emit_query_start(
    app: &AppHandle,
    correlation_id: &str,
    connection_id: &str,
    database: &str,
    query: &str,
    component: Option<&str>,
) {
    let timestamp = crate::now_ts() * 1000;
    
    let _ = app.emit("query-started", serde_json::json!({
        "timestamp": timestamp,
        "correlationId": correlation_id,
        "connectionId": connection_id,
        "database": database,
        "query": query,
        "status": "running",
        "component": component
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
    component: Option<&str>,
) {
    let timestamp = crate::now_ts() * 1000;
    let mut log_id = None;

    // Persist to internal DB first to get the ID
    if let Some(db_state) = app.try_state::<DatabaseState>() {
        if let Ok(conn) = db_state.conn.lock() {
            let res = conn.execute(
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
            if res.is_ok() {
                log_id = Some(conn.last_insert_rowid());
            }
        }
    }
    
    // Emit event with the ID
    let _ = app.emit("query-log", serde_json::json!({
        "id": log_id,
        "timestamp": timestamp,
        "correlationId": correlation_id,
        "connectionId": connection_id,
        "database": database,
        "query": query,
        "durationMs": duration_ms,
        "status": status,
        "error": error,
        "rows": row_count,
        "component": component
    }));
}

/// Enriches column metadata by querying Postgres catalogs to resolve OIDs
async fn enrich_postgres_metadata(
    client: &tokio_postgres::Client,
    raw_columns: &[tokio_postgres::Column],
    column_infos: &mut [ColumnInfo],
) -> Result<(), tokio_postgres::Error> {
    if raw_columns.is_empty() {
        return Ok(());
    }

    // Collect table OIDs to batch query, deduplicated for efficiency
    let table_oid_set: std::collections::HashSet<u32> = raw_columns.iter()
        .filter_map(|c| c.table_oid())
        .filter(|&oid| oid != 0)
        .collect();
    
    let table_oids: Vec<u32> = table_oid_set.into_iter().collect();

    if table_oids.is_empty() {
        return Ok(());
    }

    // 1. Resolve Table Names & Schemas
    // We use a simple query to map OID -> (schema, table)
    let oid_query = "
        SELECT c.oid, nspname, relname 
        FROM pg_class c 
        JOIN pg_namespace n ON c.relnamespace = n.oid 
        WHERE c.oid = ANY($1)
    ";
    let oid_rows = client.query(oid_query, &[&table_oids]).await?;
    let mut table_map: HashMap<u32, (String, String)> = HashMap::new();
    for row in oid_rows {
        let oid: u32 = row.get(0);
        let schema: String = row.get(1);
        let table: String = row.get(2);
        table_map.insert(oid, (schema, table));
    }

    // 2. Resolve Primary Keys (via constraint names first to get column patterns)
    // Query: join pg_attribute to get names
    let pk_names_query = "
        SELECT c.conrelid, a.attname
        FROM pg_constraint c
        JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)
        WHERE c.conrelid = ANY($1) AND c.contype = 'p'
    ";
    let pk_name_rows = client.query(pk_names_query, &[&table_oids]).await?;
    let mut pk_names_map: HashMap<u32, std::collections::HashSet<String>> = HashMap::new();
    for row in pk_name_rows {
        let table_oid: u32 = row.get(0);
        let col_name: String = row.get(1);
        pk_names_map.entry(table_oid).or_default().insert(col_name);
    }
    
    // Now apply
    for (i, raw_col) in raw_columns.iter().enumerate() {
         if let Some(table_oid) = raw_col.table_oid() {
             if table_oid != 0 {
                 // Set Source Table/Schema
                 if let Some((schema, table)) = table_map.get(&table_oid) {
                    column_infos[i].source_schema = Some(schema.clone());
                    column_infos[i].source_table = Some(table.clone());
                    column_infos[i].source_column = Some(raw_col.name().to_string());
                 }

                 // Set PK status
                 if let Some(pk_set) = pk_names_map.get(&table_oid) {
                     if pk_set.contains(raw_col.name()) {
                         column_infos[i].is_primary_key = true;
                     }
                 }
             }
         }
    }

    Ok(())
}

/// Enriches SQLite column metadata by querying PRAGMA table_info to identify PKs
fn enrich_sqlite_metadata(
    conn: &rusqlite::Connection,
    column_infos: &mut [ColumnInfo],
) -> Result<(), rusqlite::Error> {
    // 1. Identify unique source tables
    let mut table_names = std::collections::HashSet::new();
    for col in column_infos.iter() {
        if let Some(ref table) = col.source_table {
            table_names.insert(table.clone());
        }
    }

    if table_names.is_empty() {
        return Ok(());
    }

    // 2. Fetch PKs for each table
    let mut pk_map: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    
    for table in table_names {
        // Safe PRAGMA query with interpolation? No, PRAGMA doesn't support parameters well.
        // We must be careful about injection if table name comes from untrusted source, 
        // but here it comes from sqlite itself (stmt.column_table_name).
        // Still, let's quote it properly or verify it's a valid identifier.
        // For simplicity in this context (origin name from DB), we assume it's safe-ish but quoting is better.
        // However, rusqlite doesn't easily parameterize PRAGMA table_info. 
        // We'll proceed with direct query assuming valid identifier from internal metadata.
        
        let query = format!("PRAGMA table_info(\"{}\")", table.replace("\"", "\"\""));
        let mut stmt = conn.prepare(&query)?;
        
        let pk_cols: std::collections::HashSet<String> = stmt.query_map([], |row| {
             let pk_flag: i32 = row.get("pk")?;
             let name: String = row.get("name")?;
             Ok((name, pk_flag))
        })?
        .filter_map(|r| r.ok())
        .filter(|(_, pk_flag)| *pk_flag > 0)
        .map(|(name, _)| name)
        .collect();
        
        if !pk_cols.is_empty() {
            pk_map.insert(table, pk_cols);
        }
    }

    // 3. Mark PKs
    for col in column_infos.iter_mut() {
        if let (Some(ref table), Some(ref source_col)) = (&col.source_table, &col.source_column) {
            if let Some(pk_set) = pk_map.get(table) {
                if pk_set.contains(source_col) {
                    col.is_primary_key = true;
                }
            }
        }
    }

    Ok(())
}
