// src-tauri/src/commands/export_commands.rs

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;
use crate::DatabaseState;
use crate::ConnectionManagerState;
use crate::ConnectionManager;

// ─── Public types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Jsonl,
    SqlInsert,
    SqlScript,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Connecting,
    Executing,
    Streaming,
    Done,
    Error,
    Cancelled,
}

/// Emitted to the frontend on each progress tick (every ~500ms or 1000 rows).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub export_id: String,
    pub status: ExportStatus,
    pub rows_written: u64,
    pub bytes_written: u64,
    pub elapsed_ms: u64,
    pub file_path: String,
    pub error: Option<String>,
}

// ─── State ──────────────────────────────────────────────────────────────────

struct ActiveExport {
    token: CancellationToken,
}

#[derive(Default)]
pub struct ExportState {
    pub active: Arc<Mutex<HashMap<String, ActiveExport>>>,
}

#[tauri::command]
pub async fn cancel_export(
    export_id: String,
    export_state: State<'_, ExportState>,
) -> Result<(), String> {
    let map = export_state.active.lock().await;
    if let Some(handle) = map.get(&export_id) {
        handle.token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn start_export(
    connection_id: String,
    database: Option<String>,
    query: String,
    format: ExportFormat,
    file_path: String,
    table_name: Option<String>,
    export_state: State<'_, ExportState>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: AppHandle,
) -> Result<String, String> {
    // Extract connection info BEFORE spawning (avoids lifetime issues with State<'_>)
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let (connection, credentials) = manager.get_connection(&connection_id)?;

    let config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    let db_conf = config.get("db").ok_or("Missing 'db' config key")?;
    let host = db_conf.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?.to_string();
    let port = db_conf.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
    let user = db_conf.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?.to_string();
    let password = credentials.password.as_ref()
        .map(|p| p.expose().to_string())
        .or_else(|| db_conf.get("password").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();
    let tls_enabled = config.get("tls")
        .and_then(|t| t.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let final_database = database.unwrap_or_else(|| {
        db_conf.get("database")
            .or_else(|| db_conf.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("postgres")
            .to_string()
    });

    let export_id = Uuid::new_v4().to_string();
    let cancel_token = CancellationToken::new();
    {
        let mut map = export_state.active.lock().await;
        map.insert(export_id.clone(), ActiveExport { token: cancel_token.clone() });
    }

    let active_map = export_state.active.clone();
    let export_id_clone = export_id.clone();
    let file_path_clone = file_path.clone();

    tokio::spawn(async move {
        let _result = run_export_postgres(
            &host, &user, &password, port, tls_enabled,
            &final_database, &query, &format, &file_path_clone,
            table_name.as_deref().unwrap_or("results"),
            &cancel_token, &app, &export_id_clone,
        ).await;

        active_map.lock().await.remove(&export_id_clone);
    });

    Ok(export_id)
}

async fn run_export_postgres(
    host: &str,
    user: &str,
    password: &str,
    port: u16,
    tls_enabled: bool,
    database: &str,
    query: &str,
    format: &ExportFormat,
    file_path: &str,
    table_name: &str,
    cancel_token: &CancellationToken,
    app: &AppHandle,
    export_id: &str,
) -> Result<(), String> {
    use std::time::Duration;

    let start = Instant::now();
    emit_progress(app, export_id, ExportStatus::Connecting, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    if cancel_token.is_cancelled() { return Err("cancelled".into()); }

    // Build postgres connection
    let mut pg_config = tokio_postgres::Config::new();
    pg_config.host(host);
    pg_config.port(port);
    pg_config.user(user);
    pg_config.password(password);
    pg_config.dbname(database);
    pg_config.connect_timeout(Duration::from_secs(30));

    let client = if tls_enabled {
        let tls_connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| format!("TLS error: {}", e))?;
        let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
        let (client, connection) = pg_config.connect(connector).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tauri::async_runtime::spawn(async move {
            let _ = connection.await;
        });
        client
    } else {
        let (client, connection) = pg_config.connect(tokio_postgres::NoTls).await
            .map_err(|e| format!("Connection error: {}", e))?;
        tauri::async_runtime::spawn(async move {
            let _ = connection.await;
        });
        client
    };

    emit_progress(app, export_id, ExportStatus::Executing, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    if cancel_token.is_cancelled() { return Err("cancelled".into()); }

    let file = File::create(file_path).await.map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    emit_progress(app, export_id, ExportStatus::Streaming, 0, 0, start.elapsed().as_millis() as u64, file_path, None);

    let rows = client.query(query, &[]).await.map_err(|e| e.to_string())?;

    if cancel_token.is_cancelled() {
        drop(writer);
        let _ = tokio::fs::remove_file(file_path).await;
        return Err("cancelled".into());
    }

    let col_names: Vec<String> = if rows.is_empty() {
        vec![]
    } else {
        rows[0].columns().iter().map(|c| c.name().to_string()).collect()
    };

    let mut rows_written: u64 = 0;
    let mut bytes_written: u64 = 0;
    let mut last_emit = Instant::now();

    if matches!(format, ExportFormat::SqlScript) {
        let ddl = build_create_table_ddl(table_name, &col_names, &rows);
        let bytes = ddl.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
    }

    if matches!(format, ExportFormat::Csv | ExportFormat::Tsv) {
        let delim = if matches!(format, ExportFormat::Csv) { "," } else { "\t" };
        let header = col_names.iter()
            .map(|c| if matches!(format, ExportFormat::Csv) { csv_quote(c) } else { c.clone() })
            .collect::<Vec<_>>()
            .join(delim) + "\n";
        let bytes = header.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
    }

    if matches!(format, ExportFormat::Json) {
        writer.write_all(b"[\n").await.map_err(|e| e.to_string())?;
        bytes_written += 2;
    }

    let total = rows.len();
    for (i, row) in rows.iter().enumerate() {
        if cancel_token.is_cancelled() {
            drop(writer);
            let _ = tokio::fs::remove_file(file_path).await;
            return Err("cancelled".into());
        }

        let line = format_row_for_export(row, &col_names, format, table_name, i, total);
        let bytes = line.as_bytes();
        writer.write_all(bytes).await.map_err(|e| e.to_string())?;
        bytes_written += bytes.len() as u64;
        rows_written += 1;

        if last_emit.elapsed().as_millis() >= 500 {
            emit_progress(app, export_id, ExportStatus::Streaming, rows_written, bytes_written, start.elapsed().as_millis() as u64, file_path, None);
            last_emit = Instant::now();
        }
    }

    if matches!(format, ExportFormat::Json) {
        writer.write_all(b"\n]\n").await.map_err(|e| e.to_string())?;
    }

    writer.flush().await.map_err(|e| e.to_string())?;

    emit_progress(app, export_id, ExportStatus::Done, rows_written, bytes_written, start.elapsed().as_millis() as u64, file_path, None);

    Ok(())
}

fn format_row_for_export(
    row: &tokio_postgres::Row,
    col_names: &[String],
    format: &ExportFormat,
    table_name: &str,
    row_index: usize,
    total: usize,
) -> String {
    let values: Vec<serde_json::Value> = col_names.iter().map(|name| {
        row_to_json_value(row, name)
    }).collect();

    match format {
        ExportFormat::Csv => {
            let cells: Vec<String> = values.iter().map(|v| csv_quote(&json_to_display(v))).collect();
            cells.join(",") + "\n"
        }
        ExportFormat::Tsv => {
            let cells: Vec<String> = values.iter().map(|v| json_to_display(v)).collect();
            cells.join("\t") + "\n"
        }
        ExportFormat::Json => {
            let obj: serde_json::Map<String, serde_json::Value> = col_names.iter()
                .zip(values.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let comma = if row_index < total - 1 { "," } else { "" };
            format!("  {}{}\n", serde_json::to_string(&obj).unwrap_or_default(), comma)
        }
        ExportFormat::Jsonl => {
            let obj: serde_json::Map<String, serde_json::Value> = col_names.iter()
                .zip(values.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            serde_json::to_string(&obj).unwrap_or_default() + "\n"
        }
        ExportFormat::SqlInsert | ExportFormat::SqlScript => {
            let col_list = col_names.join(", ");
            let val_list: Vec<String> = values.iter().map(|v| sql_quote_json(v)).collect();
            format!("INSERT INTO {} ({}) VALUES ({});\n", table_name, col_list, val_list.join(", "))
        }
    }
}

fn row_to_json_value(row: &tokio_postgres::Row, col_name: &str) -> serde_json::Value {
    use tokio_postgres::types::Type;
    let col = row.columns().iter().find(|c| c.name() == col_name);
    let col_type = col.map(|c| c.type_().clone()).unwrap_or(Type::TEXT);

    match col_type {
        Type::INT2 => row.try_get::<_, Option<i16>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::INT4 => row.try_get::<_, Option<i32>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::INT8 => row.try_get::<_, Option<i64>>(col_name).ok().flatten().map(|v| serde_json::Value::Number(v.into())).unwrap_or(serde_json::Value::Null),
        Type::BOOL => row.try_get::<_, Option<bool>>(col_name).ok().flatten().map(serde_json::Value::Bool).unwrap_or(serde_json::Value::Null),
        Type::FLOAT4 | Type::FLOAT8 => row.try_get::<_, Option<f64>>(col_name).ok().flatten().map(|v| serde_json::json!(v)).unwrap_or(serde_json::Value::Null),
        _ => row.try_get::<_, Option<String>>(col_name).ok().flatten().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null),
    }
}

fn json_to_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn sql_quote_json(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn build_create_table_ddl(table_name: &str, col_names: &[String], rows: &[tokio_postgres::Row]) -> String {
    if rows.is_empty() || col_names.is_empty() {
        return format!("-- No rows to infer schema for {}\n\n", table_name);
    }
    let col_defs: Vec<String> = rows[0].columns().iter().map(|col| {
        let pg_type = match col.type_().name() {
            "int2" => "smallint",
            "int4" => "integer",
            "int8" => "bigint",
            "float4" => "real",
            "float8" => "double precision",
            "bool" => "boolean",
            "timestamp" => "timestamp",
            "timestamptz" => "timestamptz",
            "date" => "date",
            "uuid" => "uuid",
            _ => "text",
        };
        format!("  {} {}", col.name(), pg_type)
    }).collect();

    format!(
        "-- Exported from Tables on {}\n-- Query exported as SQL Script\n\nCREATE TABLE IF NOT EXISTS {} (\n{}\n);\n\n",
        chrono::Utc::now().format("%Y-%m-%d"),
        table_name,
        col_defs.join(",\n"),
    )
}

fn emit_progress(
    app: &AppHandle,
    export_id: &str,
    status: ExportStatus,
    rows_written: u64,
    bytes_written: u64,
    elapsed_ms: u64,
    file_path: &str,
    error: Option<String>,
) {
    let _ = app.emit("export-progress", ExportProgress {
        export_id: export_id.to_string(),
        status,
        rows_written,
        bytes_written,
        elapsed_ms,
        file_path: file_path.to_string(),
        error,
    });
}
