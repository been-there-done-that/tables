use tauri::State;
use serde::{Deserialize, Serialize};
use log::{debug, error, info};
use crate::{DatabaseState, ConnectionManager, ConnectionManagerState};

/// Result of a table preview query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePreviewResult {
    pub rows: Vec<serde_json::Value>,
    pub columns: Vec<ColumnInfo>,
    pub total: i64,
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
#[tauri::command]
pub async fn fetch_table_preview(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    offset: i64,
    limit: i64,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<TablePreviewResult, String> {
    info!(
        "Fetching table preview for {}.{}.{} (offset={}, limit={})",
        database, schema, table_name, offset, limit
    );

    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let (connection, credentials) = manager.get_connection(&connection_id)?;

    let config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    match connection.engine.as_str() {
        "postgres" | "postgresql" => {
            fetch_postgres_preview(&config, &credentials, &database, &schema, &table_name, offset, limit).await
        }
        "sqlite" => {
            fetch_sqlite_preview(&config, &table_name, offset, limit)
        }
        _ => Err(format!("Engine '{}' is not supported for table preview", connection.engine)),
    }
}

async fn fetch_postgres_preview(
    config: &serde_json::Value,
    credentials: &crate::connection::SecureCredentials,
    database: &str,
    schema: &str,
    table_name: &str,
    offset: i64,
    limit: i64,
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

    // Get total count
    let count_query = format!(
        "SELECT COUNT(*) FROM \"{}\".\"{}\"",
        schema, table_name
    );
    let count_row = client.query_one(&count_query, &[]).await
        .map_err(|e| format!("Count query failed: {}", e))?;
    let total: i64 = count_row.get(0);

    // Fetch rows with limit/offset
    let data_query = format!(
        "SELECT * FROM \"{}\".\"{}\" LIMIT {} OFFSET {}",
        schema, table_name, limit, offset
    );
    let rows = client.query(&data_query, &[]).await
        .map_err(|e| format!("Data query failed: {}", e))?;

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
            .map_err(|e| format!("Column query failed: {}", e))?;
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
    })
}

fn fetch_sqlite_preview(
    config: &serde_json::Value,
    table_name: &str,
    offset: i64,
    limit: i64,
) -> Result<TablePreviewResult, String> {
    let sqlite_path = config.get("file")
        .and_then(|v| v.as_str())
        .ok_or("Missing SQLite file path in config")?;

    let conn = rusqlite::Connection::open(sqlite_path)
        .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

    // Get total count
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM \"{}\"", table_name),
        [],
        |row| row.get(0)
    ).map_err(|e| format!("Count query failed: {}", e))?;

    // Get column info
    let mut stmt = conn.prepare(&format!("PRAGMA table_info(\"{}\")", table_name))
        .map_err(|e| format!("Failed to get table info: {}", e))?;
    let columns: Vec<ColumnInfo> = stmt.query_map([], |row| {
        Ok(ColumnInfo {
            name: row.get(1)?,
            column_type: row.get(2)?,
        })
    }).map_err(|e| format!("Failed to query columns: {}", e))?
      .filter_map(|r| r.ok())
      .collect();

    // Fetch rows
    let query = format!("SELECT * FROM \"{}\" LIMIT {} OFFSET {}", table_name, limit, offset);
    let mut stmt = conn.prepare(&query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let column_names: Vec<String> = columns.iter().map(|c| c.name.clone()).collect();
    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        let mut obj = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let value = sqlite_value_to_json(row, i);
            obj.insert(name.clone(), value);
        }
        Ok(serde_json::Value::Object(obj))
    }).map_err(|e| format!("Failed to query rows: {}", e))?
      .filter_map(|r| r.ok())
      .collect();

    Ok(TablePreviewResult {
        rows,
        columns,
        total,
    })
}

/// Convert a PostgreSQL row value to JSON
fn postgres_value_to_json(row: &tokio_postgres::Row, idx: usize) -> serde_json::Value {
    use tokio_postgres::types::Type;
    
    let col = &row.columns()[idx];
    
    // Try different types - we need to handle NULL separately
    // Check if value is null first by trying to get Option<T>
    
    match *col.type_() {
        Type::BOOL => {
            if let Ok(Some(v)) = row.try_get::<_, Option<bool>>(idx) {
                serde_json::Value::Bool(v)
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT2 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i16>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT4 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i32>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::INT8 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<i64>>(idx) {
                serde_json::Value::Number(v.into())
            } else {
                serde_json::Value::Null
            }
        }
        Type::FLOAT4 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<f32>>(idx) {
                serde_json::Number::from_f64(v as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        Type::FLOAT8 => {
            if let Ok(Some(v)) = row.try_get::<_, Option<f64>>(idx) {
                serde_json::Number::from_f64(v)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        Type::JSON | Type::JSONB => {
            // tokio_postgres doesn't have built-in serde_json FromSql, so we get as string and parse
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v))
            } else {
                serde_json::Value::Null
            }
        }
        _ => {
            // Default to string representation
            if let Ok(Some(v)) = row.try_get::<_, Option<String>>(idx) {
                serde_json::Value::String(v)
            } else {
                serde_json::Value::Null
            }
        }
    }
}

/// Convert a SQLite row value to JSON
fn sqlite_value_to_json(row: &rusqlite::Row, idx: usize) -> serde_json::Value {
    // Try different types in order of likelihood
    if let Ok(v) = row.get::<_, Option<i64>>(idx) {
        if let Some(n) = v {
            return serde_json::Value::Number(n.into());
        }
        return serde_json::Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<f64>>(idx) {
        if let Some(n) = v {
            return serde_json::Number::from_f64(n)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null);
        }
        return serde_json::Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<String>>(idx) {
        if let Some(s) = v {
            // Try to parse as JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&s) {
                if json.is_object() || json.is_array() {
                    return json;
                }
            }
            return serde_json::Value::String(s);
        }
        return serde_json::Value::Null;
    }
    if let Ok(v) = row.get::<_, Option<bool>>(idx) {
        if let Some(b) = v {
            return serde_json::Value::Bool(b);
        }
    }
    serde_json::Value::Null
}
