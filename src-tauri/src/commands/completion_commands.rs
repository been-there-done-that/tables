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

use crate::introspection::{MetaDatabase, MetaSchema, MetaTable, MetaColumn, MetaForeignKey, MetaIndex};
use crate::completion::schema::graph::{SchemaGraph, TableInfo, ColumnInfo, ForeignKey};
use crate::completion::parsing::parse_sql;
use crate::completion::context::Context;
use crate::completion::analysis::build_semantic_model;
use crate::completion::engine::{CompletionEngine, CompletionItem, CompletionKind};
use crate::completion::ranges::{find_current_statement_range, StatementRange};
use crate::completion::diagnostics::{Diagnostic, DiagnosticEngine};

/// Shared state for completion.
pub struct CompletionState {
    /// Cached SchemaGraph per connection (connection_id → SchemaGraph)
    pub schema_cache: Arc<Mutex<HashMap<String, Arc<SchemaGraph>>>>,
    /// Cancellation token for the current active request
    pub active_job: Mutex<Option<CancellationToken>>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self {
            schema_cache: Arc::new(Mutex::new(HashMap::new())),
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
    }
}

/// Build a SchemaGraph from MetaDatabase data.
pub fn schema_graph_from_meta(databases: &[MetaDatabase], selected_database: Option<&str>) -> SchemaGraph {
    let mut graph = SchemaGraph::new();
    
    // Collect all indexed columns for lookup
    let mut _indexed_columns: HashMap<(String, String), bool> = HashMap::new();
    
    for db in databases {
        // Filter by selected database if specified
        if let Some(selected) = selected_database {
            if db.name != selected {
                continue;
            }
        }
        
        for schema in &db.schemas {
            for table in &schema.tables {
            // Collect indexed columns
            for index in &table.indexes {
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

    graph
}

/// Update the schema cache for a connection.
#[tauri::command]
pub async fn update_completion_schema(
    state: State<'_, CompletionState>,
    connection_id: String,
    databases: Vec<MetaDatabase>,
    selected_database: Option<String>,
) -> Result<(), String> {
    let schema_graph = schema_graph_from_meta(&databases, selected_database.as_deref());
    
    let mut cache = state.schema_cache.lock().await;
    cache.insert(connection_id, Arc::new(schema_graph));
    
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
    
    // 2. Get schema from cache
    let schema_opt = {
        let cache = state.schema_cache.lock().await;
        cache.get(&connection_id).cloned()
    };
    
    let schema = match schema_opt {
        Some(s) => s,
        None => {
            // No schema cached for this connection - return empty
            return Ok(vec![]);
        }
    };
    
    // 3. Off-thread execution
    let text_clone = text.clone();
    let default_schema_clone = default_schema.clone();
    let result = tokio::task::spawn_blocking(move || {
        // Parse SQL
        let tree = parse_sql(&text, None);
        
        // Check cancellation before semantic analysis
        if cancel_token.is_cancelled() {
            return vec![];
        }
        
        // Build semantic model
        let semantic = tree.as_ref()
            .map(|t| build_semantic_model(&text, t))
            .unwrap_or_default();
        
        // Analyze cursor context
        let context = Context::analyze(&text, tree.as_ref(), cursor_offset);
        
        // Check cancellation before completion
        if cancel_token.is_cancelled() {
            return vec![];
        }
        
        // Run completion engine
        let items = CompletionEngine::complete(&semantic, &context, &schema, default_schema.as_deref());
        
        // Convert to DTOs
        items.into_iter().map(CompletionItemDto::from).collect()
    })
    .await
    .map_err(|e| e.to_string())?;
    
    Ok(result)
}

/// Find the range of the current SQL statement at the cursor.
#[tauri::command]
pub async fn get_current_statement(
    text: String,
    cursor_offset: usize
) -> Result<Option<StatementRange>, String> {
    let result = tokio::task::spawn_blocking(move || {
        let tree = parse_sql(&text, None);
        tree.as_ref().and_then(|t| find_current_statement_range(t, cursor_offset))
    }).await.map_err(|e| e.to_string())?;

    Ok(result)
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
