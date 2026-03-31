//! Completion commands for Tauri.
//!
//! Bridges the Rust completion engine to the frontend via async commands.
//! Features:
//! - Per-connection SchemaGraph caching
//! - Request cancellation for fast typing
//! - Off-thread execution for CPU-heavy parsing

use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use serde::Serialize;

use crate::introspection::MetaDatabase;
use crate::completion::schema::graph::{SchemaGraph, TableInfo, ColumnInfo, ForeignKey};
use crate::completion::parsing::parse_sql;
use crate::completion::context::Context;
use crate::completion::items::{CompletionItem, CompletionKind};
use crate::completion::document::Dialect;
use crate::completion::engines::create_engine;
use crate::completion::ranges::{find_all_statement_ranges, StatementRange, StatementRangeWithBytes};
use crate::completion::diagnostics::{Diagnostic, DiagnosticEngine};
use crate::completion::scope_builder::build_scope_tree;

/// Extract the text and relative cursor offset of the statement that contains
/// `cursor_offset` in the full editor text.
///
/// Returns `(stmt_text, stmt_cursor)`:
/// - `stmt_text` — the isolated statement (may be incomplete at cursor).
/// - `stmt_cursor` — cursor byte offset within `stmt_text`.
///
/// When the cursor falls **between** statements (blank line, between semicolons)
/// returns `("", 0)` — callers treat this as a fresh/root context.
fn extract_current_statement(
    text: &str,
    tree: Option<&tree_sitter::Tree>,
    cursor_offset: usize,
) -> (String, usize) {
    let tree = match tree {
        Some(t) => t,
        None => return (String::new(), 0),
    };

    // Use find_all_statement_ranges which already filters semicolons and comments.
    let ranges = find_all_statement_ranges(tree, text);

    // Primary: cursor falls within [start_byte, end_byte] (tree-sitter end_byte is exclusive,
    // so end_byte == cursor_offset means cursor is just past the last parsed char — still
    // "inside" for completion purposes).
    for r in &ranges {
        if r.start_byte <= cursor_offset && r.end_byte >= cursor_offset {
            let start = r.start_byte.min(text.len());
            let end = r.end_byte.min(text.len());
            return (text[start..end].to_string(), cursor_offset - start);
        }
    }

    // Fallback: tree-sitter truncates end_byte on parse errors (e.g. trailing dot
    // `WHERE o.` → the `.` becomes an error token, shrinking end_byte by 1).
    // Find the last statement whose start_byte <= cursor.  If the gap between its
    // end_byte and cursor contains no newline, the cursor is a continuation of that
    // statement (not a fresh blank line between queries).
    let fallback = ranges.iter()
        .filter(|r| r.start_byte <= cursor_offset)
        .max_by_key(|r| r.start_byte);

    if let Some(r) = fallback {
        let gap_start = r.end_byte.min(text.len());
        let gap_end = cursor_offset.min(text.len());
        let gap = &text[gap_start..gap_end];
        if !gap.contains('\n') {
            // Cursor is continuing this statement past what tree-sitter parsed.
            // Extend to cursor_offset so the incomplete trailing text is included.
            let start = r.start_byte.min(text.len());
            let end = cursor_offset.min(text.len());
            return (text[start..end].to_string(), cursor_offset - start);
        }
    }

    // Cursor is on a blank line between statements → return empty so engine gives
    // RootContext (SELECT / INSERT / WITH … keyword) suggestions.
    (String::new(), 0)
}

/// Build a scope tree with logging.  Delegates to `scope_builder::build_scope_tree`
/// for the actual 3-pass logic, then logs the result for debugging.
fn build_scope_tree_logged(
    text: &str,
    cursor_offset: usize,
    dialect: sql_scope::Dialect,
    schema: &SchemaGraph,
) -> sql_scope::ScopeTree {
    let snippet = &text[..cursor_offset.min(text.len()).min(120)];
    let tree = build_scope_tree(text, cursor_offset, dialect, schema);
    let sources: Vec<_> = tree.visible_at(cursor_offset).sources.iter().map(|(k, _)| k.clone()).collect();
    log::info!("[scope] cursor={} sources={:?} sql={:?}", cursor_offset, sources, snippet);
    tree
}

/// Convert our Dialect to sql_scope::Dialect.
fn dialect_to_sql_scope(dialect: Dialect) -> sql_scope::Dialect {
    match dialect {
        Dialect::Postgres => sql_scope::Dialect::Postgres,
        Dialect::SQLite => sql_scope::Dialect::Sqlite,
        Dialect::MySQL => sql_scope::Dialect::Mysql,
    }
}

/// Shared state for completion.
pub struct CompletionState {
    /// Cached SchemaGraph per connection (connection_id → SchemaGraph)
    pub schema_cache: Arc<Mutex<HashMap<String, Arc<SchemaGraph>>>>,
    /// Cached Dialect per connection (connection_id → Dialect)
    pub dialect_cache: Arc<Mutex<HashMap<String, Dialect>>>,
    /// Cancellation token for the current active request
    pub active_job: Mutex<Option<CancellationToken>>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self {
            schema_cache: Arc::new(Mutex::new(HashMap::new())),
            dialect_cache: Arc::new(Mutex::new(HashMap::new())),
            active_job: Mutex::new(None),
        }
    }
}

/// DTO for sending completion items to frontend (Monaco format).
#[derive(Debug, Clone, Serialize)]
pub struct CompletionItemDto {
    pub label: String,
    pub kind: u8,
    pub detail: Option<String>,
    pub insert_text: String,
    pub score: u32,
    /// If true, trigger completions again after this item is selected (for chained completions)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub trigger_suggest: bool,
}

impl From<CompletionItem> for CompletionItemDto {
    fn from(item: CompletionItem) -> Self {
        // Trigger suggest for schema completions (insert_text ends with ".")
        let trigger_suggest = item.insert_text.ends_with('.');
        
        Self {
            label: item.label,
            kind: map_completion_kind(item.kind),
            detail: item.detail,
            insert_text: item.insert_text,
            score: item.score,
            trigger_suggest,
        }
    }
}

/// Map our CompletionKind to Monaco's CompletionItemKind.
fn map_completion_kind(kind: CompletionKind) -> u8 {
    match kind {
        CompletionKind::Table => 7,       // Class
        CompletionKind::Column => 5,      // Field
        CompletionKind::Alias => 6,       // Variable
        CompletionKind::Keyword => 14,    // Keyword
        CompletionKind::Function => 3,    // Function
        CompletionKind::JoinCondition => 15, // Snippet
        CompletionKind::Schema => 9,      // Module
        CompletionKind::Operator => 11,   // Operator
    }
}

/// Build a SchemaGraph from MetaDatabase data.
pub fn schema_graph_from_meta(databases: &[MetaDatabase], selected_database: Option<&str>) -> SchemaGraph {
    let mut graph = SchemaGraph::new();
    
    log::info!("[schema_graph_from_meta] Building schema graph from {} databases, selected: {:?}", 
        databases.len(), selected_database);
    
    // Collect all indexed columns for lookup
    let mut _indexed_columns: HashMap<(String, String), bool> = HashMap::new();
    let mut total_tables = 0;
    let mut total_columns = 0;
    
    for db in databases {
        // Filter by selected database if specified
        if let Some(selected) = selected_database {
            if db.name != selected {
                log::debug!("[schema_graph_from_meta] Skipping database '{}' (selected='{}')", db.name, selected);
                continue;
            }
        }
        
        log::debug!("[schema_graph_from_meta] Processing database '{}' with {} schemas", 
            db.name, db.schemas.len());
        
        for schema in &db.schemas {
            log::debug!("[schema_graph_from_meta] Processing schema '{}' with {} tables", 
                schema.name, schema.tables.len());
            
            for table in &schema.tables {
            // Collect indexed columns
            for _index in &table.indexes {
                // For each index, we need to mark columns as indexed
                // Since MetaIndex doesn't include column names directly,
                // we assume all columns with matching table are potentially indexed
                // (This is a simplification - you may need to enhance MetaIndex)
            }
            
            // Add table with columns
            let columns: Vec<ColumnInfo> = table.columns.iter().map(|col| {
                ColumnInfo {
                    name: col.column_name.clone(),
                    data_type: col.raw_type.clone(),
                    is_nullable: col.nullable,
                    is_primary_key: col.is_primary_key,
                    is_indexed: false, // We'd need index_columns join for this
                }
            }).collect();
            
            total_tables += 1;
            total_columns += columns.len();
            
            if total_tables <= 5 {
                log::debug!("[schema_graph_from_meta] Adding table '{}.{}' with {} columns", 
                    schema.name, table.table_name, columns.len());
            }
            
            graph.add_table(TableInfo {
                name: table.table_name.clone(),
                schema: schema.name.clone(),
                columns,
            });
            
            // Add foreign keys
            for fk in &table.foreign_keys {
                graph.add_foreign_key(ForeignKey {
                    from_table: table.table_name.clone(),
                    from_column: fk.column_name.clone(),
                    to_table: fk.ref_table.clone(),
                    to_column: fk.ref_column.clone(),
                });
            }
        }
    }
}

    log::info!("[schema_graph_from_meta] Built schema graph: {} tables, {} total columns, {} tables in graph", 
        total_tables, total_columns, graph.tables.len());

    graph
}

/// Build a SchemaGraph directly from the app database using Introspector.
/// This fetches tables WITH their columns, foreign keys, and indexes.
fn schema_graph_from_introspector(connection_id: &str, app_db: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> SchemaGraph {
    use crate::introspection::Introspector;
    
    let mut graph = SchemaGraph::new();
    let introspector = Introspector::new(app_db.clone());
    
    // get_tables() returns tables WITH columns populated
    match introspector.get_tables(connection_id) {
        Ok(tables) => {
            log::info!("[Completion] Fetched {} tables from introspector for connection {}", 
                tables.len(), connection_id);
            
            for table in &tables {
                let columns: Vec<ColumnInfo> = table.columns.iter().map(|col| {
                    ColumnInfo {
                        name: col.column_name.clone(),
                        data_type: col.raw_type.clone(),
                        is_nullable: col.nullable,
                        is_primary_key: col.is_primary_key,
                        is_indexed: false,
                    }
                }).collect();
                
                if tables.len() <= 30 || graph.tables.len() < 5 {
                    log::debug!("[Completion] Adding table '{}.{}' with {} columns", 
                        table.schema, table.table_name, columns.len());
                }
                
                graph.add_table(TableInfo {
                    name: table.table_name.clone(),
                    schema: table.schema.clone(),
                    columns,
                });
                
                for fk in &table.foreign_keys {
                    graph.add_foreign_key(ForeignKey {
                        from_table: table.table_name.clone(),
                        from_column: fk.column_name.clone(),
                        to_table: fk.ref_table.clone(),
                        to_column: fk.ref_column.clone(),
                    });
                }
            }
            
            let total_columns: usize = tables.iter().map(|t| t.columns.len()).sum();
            log::info!("[Completion] Built schema graph: {} tables, {} total columns", 
                graph.tables.len(), total_columns);
        }
        Err(e) => {
            log::error!("[Completion] Failed to fetch tables from introspector: {}", e);
        }
    }
    
    graph
}

/// Update the schema cache for a connection.
/// This fetches tables+columns directly from the app database (not from frontend).
#[tauri::command]
pub async fn update_completion_schema(
    state: State<'_, CompletionState>,
    db_state: State<'_, crate::DatabaseState>,
    connection_id: String,
    #[allow(unused_variables)]
    databases: Vec<MetaDatabase>,  // Kept for API compatibility, but ignored
    #[allow(unused_variables)]
    selected_database: Option<String>,  // Kept for API compatibility, but ignored  
    engine_type: Option<String>,
) -> Result<(), String> {
    // Fetch tables WITH columns directly from the introspector
    let schema_graph = schema_graph_from_introspector(&connection_id, &db_state.conn);
    
    // Parse engine type to Dialect
    let dialect = match engine_type.as_deref().map(|s| s.to_lowercase()).as_deref() {
        Some("sqlite") => Dialect::SQLite,
        Some("postgres") | Some("postgresql") => Dialect::Postgres,
        Some("mysql") => Dialect::MySQL,
        _ => Dialect::Postgres, // Default to Postgres for compatibility
    };
    
    log::debug!("[Completion] Caching schema for connection {} with dialect {:?}, {} tables", 
        connection_id, dialect, schema_graph.tables.len());
    
    let mut cache = state.schema_cache.lock().await;
    cache.insert(connection_id.clone(), Arc::new(schema_graph));
    
    let mut dialect_cache = state.dialect_cache.lock().await;
    dialect_cache.insert(connection_id, dialect);
    
    Ok(())
}

/// Clear the schema cache for a connection.
#[tauri::command]
pub async fn clear_completion_schema(
    state: State<'_, CompletionState>,
    connection_id: String,
) -> Result<(), String> {
    let mut cache = state.schema_cache.lock().await;
    cache.remove(&connection_id);
    Ok(())
}

/// Request completions for SQL text at a cursor position.
#[tauri::command]
pub async fn request_completions(
    state: State<'_, CompletionState>,
    connection_id: String,
    text: String,
    cursor_offset: usize,
    default_schema: Option<String>,
) -> Result<Vec<CompletionItemDto>, String> {
    // 1. Cancellation: create new token, cancel old
    let cancel_token = CancellationToken::new();
    {
        let mut job_guard = state.active_job.lock().await;
        if let Some(old_token) = job_guard.take() {
            old_token.cancel();
        }
        *job_guard = Some(cancel_token.clone());
    }
    
    // 2. Get schema and dialect from cache
    let schema_opt = {
        let cache = state.schema_cache.lock().await;
        cache.get(&connection_id).cloned()
    };
    
    let dialect = {
        let dialect_cache = state.dialect_cache.lock().await;
        dialect_cache.get(&connection_id).copied().unwrap_or(Dialect::Postgres)
    };
    
    let schema = match schema_opt {
        Some(s) => s,
        None => {
            // No schema cached for this connection - return empty
            return Ok(vec![]);
        }
    };
    
    // 3. Off-thread execution with dialect-specific engine
    let schema_tables_count = schema.tables.len();
    log::info!("[request_completions] connection={}, dialect={:?}, schema_tables={}, default_schema={:?}", 
        connection_id, dialect, schema_tables_count, default_schema);
    
    let result = tokio::task::spawn_blocking(move || {
        // Parse full text for statement boundary detection.
        let full_tree = parse_sql(&text, None);

        // Check cancellation before semantic analysis
        if cancel_token.is_cancelled() {
            return vec![];
        }

        // Isolate the current statement so scope and context analysis never
        // see aliases/tables from other queries in the editor.
        // If cursor is between statements, stmt_text = "" → RootContext suggestions.
        let (stmt_text, stmt_cursor) =
            extract_current_statement(&text, full_tree.as_ref(), cursor_offset);

        // Re-parse the isolated statement (fast: single statement, sub-ms).
        let stmt_tree = parse_sql(&stmt_text, None);

        // Build scope tree from the isolated statement only.
        let scope_tree = build_scope_tree_logged(&stmt_text, stmt_cursor, dialect_to_sql_scope(dialect), schema.as_ref());

        // Analyze cursor context from the isolated statement.
        let context = Context::analyze(&stmt_text, stmt_tree.as_ref(), stmt_cursor);

        log::info!("[completion] context={:?} prefix={:?} cursor={}",
            context.context_type, context.prefix, context.cursor_offset);

        // Check cancellation before completion
        if cancel_token.is_cancelled() {
            return vec![];
        }

        // Create dialect-specific engine and run completion
        let engine = create_engine(dialect);
        let items = engine.complete(&scope_tree, &context, &schema, default_schema.as_deref(), None);

        log::info!("[completion] returned {} items: {:?}", items.len(),
            items.iter().take(5).map(|i| &i.label).collect::<Vec<_>>());
        
        // Convert to DTOs
        items.into_iter().map(CompletionItemDto::from).collect()
    })
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(result)
}

/// Find the range of the current SQL statement at the cursor.
/// Uses sql_scope::split_statements (semicolon-aware) for accurate boundary detection.
#[tauri::command]
pub async fn get_current_statement(
    text: String,
    cursor_offset: usize
) -> Result<Option<StatementRange>, String> {
    let result = tokio::task::spawn_blocking(move || {
        let statements = sql_scope::split_statements(&text);
        for (offset, stmt) in &statements {
            let end_byte = offset + stmt.len();
            // Cursor is inside the statement text
            if cursor_offset >= *offset && cursor_offset <= end_byte {
                return Some(StatementRange {
                    start_line: byte_offset_to_line(&text, *offset),
                    end_line: byte_offset_to_line(&text, end_byte),
                });
            }
            // Cursor is just after the statement (at semicolon or trailing space) — same line
            if cursor_offset > end_byte {
                let gap_end = cursor_offset.min(text.len());
                let gap = &text[end_byte..gap_end];
                if !gap.contains('\n') && gap.len() <= 3 {
                    return Some(StatementRange {
                        start_line: byte_offset_to_line(&text, *offset),
                        end_line: byte_offset_to_line(&text, end_byte),
                    });
                }
            }
        }
        None
    }).await.map_err(|e| e.to_string())?;

    Ok(result)
}

fn byte_offset_to_line(text: &str, byte_offset: usize) -> u32 {
    let end = byte_offset.min(text.len());
    text[..end].bytes().filter(|&b| b == b'\n').count() as u32 + 1
}

/// Request SQL diagnostics (syntax and semantic errors).
#[tauri::command]
pub async fn request_diagnostics(
    state: tauri::State<'_, CompletionState>,
    connection_id: String,
    text: String,
) -> Result<Vec<Diagnostic>, String> {
    // 1. Get schema from cache
    let schema_opt = {
        let cache = state.schema_cache.lock().await;
        cache.get(&connection_id).cloned()
    };
    
    let schema = match schema_opt {
        Some(s) => s,
        None => {
            // No schema cached - we can still do syntax checks
            std::sync::Arc::new(crate::completion::schema::graph::SchemaGraph::new())
        }
    };

    // 2. Offload to thread
    let result = tokio::task::spawn_blocking(move || {
        let tree = crate::completion::parsing::parse_sql(&text, None);
        match tree {
            Some(t) => DiagnosticEngine::check(&t, &text, &schema),
            None => vec![],
        }
    }).await.map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get all SQL statement ranges in the document.
/// Used for CodeLens/glyph margin to show run buttons on each query.
/// Uses sql_scope::split_statements (semicolon-aware) for accurate boundary detection.
#[tauri::command]
pub async fn get_all_statements(
    text: String,
) -> Result<Vec<StatementRangeWithBytes>, String> {
    let result = tokio::task::spawn_blocking(move || {
        sql_scope::split_statements(&text)
            .into_iter()
            .filter_map(|(offset, stmt)| {
                if stmt.trim().is_empty() { return None; }
                let end_byte = offset + stmt.len();
                Some(StatementRangeWithBytes {
                    start_line: byte_offset_to_line(&text, offset),
                    end_line: byte_offset_to_line(&text, end_byte),
                    start_byte: offset,
                    end_byte,
                })
            })
            .collect()
    }).await.map_err(|e| e.to_string())?;

    Ok(result)
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    use crate::completion::document::Dialect;

    #[test]
    fn dialect_conversion_covers_all_variants() {
        assert!(matches!(dialect_to_sql_scope(Dialect::Postgres), sql_scope::Dialect::Postgres));
        assert!(matches!(dialect_to_sql_scope(Dialect::SQLite), sql_scope::Dialect::Sqlite));
        assert!(matches!(dialect_to_sql_scope(Dialect::MySQL), sql_scope::Dialect::Mysql));
    }
}
