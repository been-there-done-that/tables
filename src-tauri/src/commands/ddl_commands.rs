//! DDL generation commands — return CREATE statements for PostgreSQL objects.
//! All commands are read-only and connect live to the database.

use tauri::State;
use crate::{DatabaseState, ConnectionManagerState};
use log::debug;

use super::introspection_commands::pg_connect_for_commands;

#[tauri::command]
pub async fn get_table_ddl(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    let qualified = format!("{}.{}", schema, table_name);
    debug!("Generating DDL for table {}", qualified);

    // Get column definitions
    let col_rows = client.query(
        "SELECT
            a.attname AS col_name,
            pg_catalog.format_type(a.atttypid, a.atttypmod) AS col_type,
            a.attnotnull AS not_null,
            pg_get_expr(d.adbin, d.adrelid) AS default_val
        FROM pg_attribute a
        LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
        WHERE a.attrelid = $1::regclass
          AND a.attnum > 0
          AND NOT a.attisdropped
        ORDER BY a.attnum",
        &[&qualified],
    ).await.map_err(|e| format!("Column query error: {}", e))?;

    // Get constraints
    let con_rows = client.query(
        "SELECT conname, pg_get_constraintdef(oid) AS def
        FROM pg_constraint
        WHERE conrelid = $1::regclass
        ORDER BY contype, conname",
        &[&qualified],
    ).await.map_err(|e| format!("Constraint query error: {}", e))?;

    // Build the CREATE TABLE statement
    let mut parts: Vec<String> = col_rows.iter().map(|row| {
        let col_name: &str = row.get(0);
        let col_type: &str = row.get(1);
        let not_null: bool = row.get(2);
        let default_val: Option<&str> = row.get(3);
        let mut def = format!("    {} {}", col_name, col_type);
        if not_null { def.push_str(" NOT NULL"); }
        if let Some(d) = default_val { def.push_str(&format!(" DEFAULT {}", d)); }
        def
    }).collect();

    for row in &con_rows {
        let con_name: &str = row.get(0);
        let con_def: &str = row.get(1);
        parts.push(format!("    CONSTRAINT {} {}", con_name, con_def));
    }

    Ok(format!(
        "CREATE TABLE {}.{} (\n{}\n);",
        schema, table_name,
        parts.join(",\n")
    ))
}

#[tauri::command]
pub async fn get_view_definition(
    connection_id: String,
    database: String,
    schema: String,
    view_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching view definition for {}.{}", schema, view_name);

    let row = client.query_one(
        "SELECT definition FROM pg_views WHERE schemaname = $1 AND viewname = $2",
        &[&schema, &view_name],
    ).await.map_err(|e| format!("View not found: {}", e))?;

    let definition: &str = row.get(0);
    Ok(format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", schema, view_name, definition.trim_end()))
}

#[tauri::command]
pub async fn get_matview_definition(
    connection_id: String,
    database: String,
    schema: String,
    view_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching matview definition for {}.{}", schema, view_name);

    let row = client.query_one(
        "SELECT definition FROM pg_matviews WHERE schemaname = $1 AND matviewname = $2",
        &[&schema, &view_name],
    ).await.map_err(|e| format!("Materialized view not found: {}", e))?;

    let definition: &str = row.get(0);
    Ok(format!("CREATE MATERIALIZED VIEW {}.{} AS\n{}", schema, view_name, definition.trim_end()))
}

#[tauri::command]
pub async fn get_function_ddl(
    connection_id: String,
    database: String,
    schema: String,
    function_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching function DDL for {}.{}", schema, function_name);

    // pg_get_functiondef returns the complete CREATE OR REPLACE FUNCTION statement
    let row = client.query_one(
        "SELECT pg_get_functiondef(p.oid)
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = $1 AND p.proname = $2
        LIMIT 1",
        &[&schema, &function_name],
    ).await.map_err(|e| format!("Function not found: {}", e))?;

    Ok(row.get(0))
}

#[tauri::command]
pub async fn get_sequence_ddl(
    connection_id: String,
    database: String,
    schema: String,
    sequence_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching sequence DDL for {}.{}", schema, sequence_name);

    let row = client.query_one(
        "SELECT data_type::text, start_value, min_value, max_value, increment_by, cycle, cache_size
        FROM pg_sequences
        WHERE schemaname = $1 AND sequencename = $2",
        &[&schema, &sequence_name],
    ).await.map_err(|e| format!("Sequence not found: {}", e))?;

    let data_type: &str = row.get(0);
    let start: i64 = row.get(1);
    let min: i64 = row.get(2);
    let max: i64 = row.get(3);
    let increment: i64 = row.get(4);
    let cycle: bool = row.get(5);
    let cache: i64 = row.get(6);

    Ok(format!(
        "CREATE SEQUENCE {}.{}\n    AS {}\n    START WITH {}\n    INCREMENT BY {}\n    MINVALUE {}\n    MAXVALUE {}\n    CACHE {}{};",
        schema, sequence_name, data_type, start, increment, min, max, cache,
        if cycle { "\n    CYCLE" } else { "\n    NO CYCLE" }
    ))
}

#[tauri::command]
pub async fn get_index_ddl(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    index_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching index DDL for {}", index_name);

    let row = client.query_one(
        "SELECT indexdef FROM pg_indexes WHERE schemaname = $1 AND tablename = $2 AND indexname = $3",
        &[&schema, &table_name, &index_name],
    ).await.map_err(|e| format!("Index not found: {}", e))?;

    Ok(row.get(0))
}

#[tauri::command]
pub async fn get_trigger_definition(
    connection_id: String,
    database: String,
    schema: String,
    table_name: String,
    trigger_name: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let client = pg_connect_for_commands(&connection_id, &database, &db_state, &conn_state).await?;
    debug!("Fetching trigger definition for {}.{}", table_name, trigger_name);

    let row = client.query_one(
        "SELECT pg_get_triggerdef(t.oid, true)
        FROM pg_trigger t
        JOIN pg_class c ON c.oid = t.tgrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = $1 AND c.relname = $2 AND t.tgname = $3
          AND NOT t.tgisinternal",
        &[&schema, &table_name, &trigger_name],
    ).await.map_err(|e| format!("Trigger not found: {}", e))?;

    Ok(row.get(0))
}
