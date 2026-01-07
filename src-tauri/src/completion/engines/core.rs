//! Core Completion Logic
//!
//! Shared utilities and completion methods used by all engine variants.
//! Database-specific engines (Postgres, SQLite) delegate to these functions.

use std::collections::HashSet;

use crate::adapter::DatabaseCapabilities;
use crate::completion::analysis::{SemanticModel, SymbolKind};
use crate::completion::context::{Context, CursorContext};
use crate::completion::schema::SchemaGraph;
use super::super::engine::{CompletionItem, CompletionKind};

// ============================================================================
// Scoring Constants (Additive Model)
// ============================================================================

/// Cursor context relevance (highest priority)
pub const SCORE_CURSOR_RELEVANCE: u32 = 1000;
/// Table/alias already in query scope
pub const SCORE_QUERY_SCOPE_MATCH: u32 = 800;
/// Alias matches exactly
pub const SCORE_ALIAS_MATCH: u32 = 700;
/// Exact prefix match
pub const SCORE_EXACT_MATCH: u32 = 600;
/// Prefix starts with typed text
pub const SCORE_PREFIX_MATCH: u32 = 400;
/// Matches UI schema hint (dropdown selection)
pub const SCORE_UI_SCHEMA_HINT: u32 = 300;
/// Matches default schema
pub const SCORE_DEFAULT_SCHEMA: u32 = 200;
/// Matches public schema
pub const SCORE_PUBLIC_SCHEMA: u32 = 150;
/// CTE definitions (local, highest priority for tables - beats UI schema hint)
pub const SCORE_CTE: u32 = 400;
/// Primary key column
pub const SCORE_PRIMARY_KEY: u32 = 30;
/// Indexed column
pub const SCORE_INDEXED: u32 = 20;

/// Penalty for cross-schema (not in selected schema)
pub const PENALTY_CROSS_SCHEMA: i32 = -250;
/// Penalty for ambiguous column (exists in multiple tables)
pub const PENALTY_AMBIGUITY: i32 = -400;

/// Core completion engine with shared logic.
pub struct CoreCompletionEngine;

impl CoreCompletionEngine {
    /// Complete after a dot: `alias.|` → columns of the aliased table.
    /// Also handles `schema.|` → tables in that schema.
    pub fn complete_after_dot(
        alias: &str,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        log::debug!("[AfterDot] Looking up '{}' at cursor offset {}", alias, context.cursor_offset);
        
        // First, try to resolve as a table alias
        if let Some(symbol) = semantic.resolve_at_cursor(context.cursor_offset, alias) {
            log::debug!("[AfterDot] Resolved as alias: {:?}", symbol.kind);
            
            if let Some(table_name) = symbol.resolve_table_name() {
                log::debug!("[AfterDot] Table name: '{}'", table_name);
                
                let columns = schema.get_columns(table_name);
                log::debug!("[AfterDot] Found {} columns for table '{}'", columns.len(), table_name);
                
                if !columns.is_empty() {
                    for col in columns {
                        let score = Self::column_score(col.is_primary_key, col.is_indexed);
                        items.push(CompletionItem {
                            label: col.name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("{} ({})", col.data_type, table_name)),
                            insert_text: col.name.clone(),
                            score,
                        });
                    }
                } else {
                    // Check CTEs
                    if let Some(cte_cols) = semantic.ctes.get(&table_name.to_lowercase()) {
                        log::debug!("[AfterDot] Found {} CTE columns", cte_cols.len());
                        for col_name in cte_cols {
                            items.push(CompletionItem {
                                label: col_name.clone(),
                                kind: CompletionKind::Column,
                                detail: Some(format!("CTE Column ({})", table_name)),
                                insert_text: col_name.clone(),
                                score: 90,
                            });
                        }
                    }
                }
            }
        }
        
        // If no alias match, check if this is a schema name
        if items.is_empty() {
            log::debug!("[AfterDot] No alias match, checking if '{}' is a schema name", alias);
            
            let alias_lower = alias.to_lowercase();
            let schema_tables: Vec<_> = schema.tables.values()
                .filter(|t| t.schema.to_lowercase() == alias_lower)
                .collect();
            
            log::debug!("[AfterDot] Found {} tables in schema '{}'", schema_tables.len(), alias);
            
            if !schema_tables.is_empty() {
                for table in schema_tables {
                    items.push(CompletionItem {
                        label: table.name.clone(),
                        kind: CompletionKind::Table,
                        detail: Some(table.schema.clone()),
                        insert_text: table.name.clone(),
                        score: SCORE_CURSOR_RELEVANCE + SCORE_UI_SCHEMA_HINT,
                    });
                }
            }
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete table names for FROM/JOIN clauses.
    pub fn complete_table_names(
        schema: &SchemaGraph, 
        semantic: &SemanticModel,
        context: &Context,
        default_schema: Option<&str>,
        from_keywords: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_schemas = HashSet::new();
        
        let ui_schema = default_schema.unwrap_or("public");
        
        log::debug!("[complete_table_names] Called with default_schema={:?}, ui_schema={}, prefix='{}', schema.tables.len()={}", 
            default_schema, ui_schema, context.prefix, schema.tables.len());

        // 1. Add schema suggestions first
        for table_info in schema.tables.values() {
            if seen_schemas.insert(table_info.schema.clone()) {
                let score = Self::calculate_schema_score(&table_info.schema, ui_schema);
                items.push(CompletionItem {
                    label: table_info.schema.clone(),
                    kind: CompletionKind::Schema,
                    detail: Some("schema".to_string()),
                    insert_text: format!("{}.", table_info.schema),
                    score,
                });
            }
        }
        
        log::debug!("[complete_table_names] Found {} unique schemas", seen_schemas.len());

        // 2. Schema tables with explicit qualification
        let mut table_count = 0;
        for table_info in schema.tables.values() {
            let is_ui_schema = table_info.schema == ui_schema;
            let is_public = table_info.schema == "public";
            let is_main = table_info.schema == "main";  // SQLite default
            
            let mut score: i32 = SCORE_CURSOR_RELEVANCE as i32;
            
            // For SQLite, treat "main" like "public" for scoring
            if is_ui_schema {
                score += SCORE_UI_SCHEMA_HINT as i32;
            } else if is_public || is_main {
                score += SCORE_PUBLIC_SCHEMA as i32;
            } else {
                score += PENALTY_CROSS_SCHEMA;
            }
            
            let insert_text = Self::qualify_table_name(
                &table_info.schema, 
                &table_info.name, 
                ui_schema
            );
            
            // For SQLite/main schema, don't show "(main)" suffix - it's the default
            let label = if is_ui_schema || is_public || is_main {
                table_info.name.clone()
            } else {
                format!("{} ({})", table_info.name, table_info.schema)
            };
            
            table_count += 1;
            if table_count <= 3 {
                log::debug!("[complete_table_names] Adding table: label='{}', schema='{}', score={}", 
                    label, table_info.schema, score);
            }

            items.push(CompletionItem {
                label,
                kind: CompletionKind::Table,
                detail: Some(table_info.schema.clone()),
                insert_text,
                score: score.max(0) as u32,
            });
        }
        
        log::debug!("[complete_table_names] Added {} tables", table_count);

        // 3. CTEs from current scope
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let SymbolKind::CTE { cte_name } = &sym.kind {
                items.push(CompletionItem {
                    label: cte_name.clone(),
                    kind: CompletionKind::Table,
                    detail: Some("CTE".to_string()),
                    insert_text: cte_name.clone(),
                    score: SCORE_CURSOR_RELEVANCE + SCORE_CTE,
                });
            }
        }
        
        // 4. FROM-context keywords
        for kw in from_keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 40,
            });
        }
        
        log::debug!("[complete_table_names] Total items before filter: {}", items.len());
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        
        log::debug!("[complete_table_names] Total items after filter: {}", items.len());
        
        items
    }

    /// Complete JOIN condition: infer from FK or heuristics.
    pub fn complete_join_condition(
        left_table: &Option<String>,
        right_table: &Option<String>,
        semantic: &SemanticModel,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        if let (Some(left), Some(right)) = (left_table, right_table) {
            let left_resolved = resolve_table_name_from_alias(left, semantic)
                .unwrap_or_else(|| left.clone());
            let right_resolved = resolve_table_name_from_alias(right, semantic)
                .unwrap_or_else(|| right.clone());
            
            if let Some((condition, score)) = schema.infer_join_condition(
                &left_resolved, 
                &right_resolved,
                Some(left),
                Some(right)
            ) {
                items.push(CompletionItem {
                    label: condition.clone(),
                    kind: CompletionKind::JoinCondition,
                    detail: Some(format!("confidence: {}%", score)),
                    insert_text: condition,
                    score,
                });
            }
        }
        
        // Also show individual column completions as fallback
        if let Some(left) = left_table {
            let left_resolved = resolve_table_name_from_alias(left, semantic)
                .unwrap_or_else(|| left.clone());
            for col in schema.get_columns(&left_resolved) {
                items.push(CompletionItem {
                    label: format!("{}.{}", left, col.name),
                    kind: CompletionKind::Column,
                    detail: Some(col.data_type.clone()),
                    insert_text: format!("{}.{}", left, col.name),
                    score: Self::column_score(col.is_primary_key, col.is_indexed) / 2,
                });
            }
        }
        
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete in WHERE clause: columns from visible tables.
    pub fn complete_where_clause(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        where_keywords: &[&str],
        where_functions: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let Some(table_name) = sym.resolve_table_name() {
                for col in schema.get_columns(table_name) {
                    let qualified_name = format!("{}.{}", sym.name, col.name);
                    items.push(CompletionItem {
                        label: qualified_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(col.data_type.clone()),
                        insert_text: qualified_name,
                        score: Self::column_score(col.is_primary_key, col.is_indexed),
                    });
                }
            }
        }

        for func in where_functions {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: CompletionKind::Function,
                detail: Some("function".to_string()),
                insert_text: format!("{}()", func),
                score: 60,
            });
        }

        for kw in where_keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 50,
            });
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete in SELECT clause: columns from visible aliases + functions.
    pub fn complete_select_clause(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        select_keywords: &[&str],
        select_functions: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_labels = HashSet::new();
        
        // Add visible aliases and their columns
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if seen_labels.insert(sym.name.clone()) {
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind: CompletionKind::Alias,
                    detail: sym.resolve_table_name().map(|t| format!("alias for {}", t)),
                    insert_text: format!("{}.", sym.name),
                    score: 80,
                });
            }

            if let Some(table_name) = sym.resolve_table_name() {
                for col in schema.get_columns(table_name) {
                    if seen_labels.insert(col.name.clone()) {
                        items.push(CompletionItem {
                            label: col.name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("{} ({})", col.data_type, sym.name)),
                            insert_text: col.name.clone(),
                            score: 70,
                        });
                    }
                }
                
                if let Some(cte_cols) = semantic.ctes.get(&table_name.to_lowercase()) {
                    for col_name in cte_cols {
                        if seen_labels.insert(col_name.clone()) {
                            items.push(CompletionItem {
                                label: col_name.clone(),
                                kind: CompletionKind::Column,
                                detail: Some(format!("CTE Column ({})", table_name)),
                                insert_text: col_name.clone(),
                                score: 70,
                            });
                        }
                    }
                }
            }
        }
        
        // Add wildcard
        items.push(CompletionItem {
            label: "*".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("All columns".to_string()),
            insert_text: "*".to_string(),
            score: 90,
        });

        // Add SELECT-context functions
        for func in select_functions {
            if seen_labels.insert(func.to_string()) {
                items.push(CompletionItem {
                    label: func.to_string(),
                    kind: CompletionKind::Function,
                    detail: Some("function".to_string()),
                    insert_text: format!("{}()", func),
                    score: 60,
                });
            }
        }
        
        // Add SELECT-context keywords
        for kw in select_keywords {
            if seen_labels.insert(kw.to_string()) {
                let mut score = 50;
                
                if matches!(context.context_type, CursorContext::AfterSelectList) {
                    if *kw == "FROM" {
                        score = 200;
                    } else if *kw == "AS" {
                        score = 150;
                    }
                }

                items.push(CompletionItem {
                    label: kw.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: None,
                    insert_text: kw.to_string(),
                    score,
                });
            }
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete function arguments with type-aware filtering.
    pub fn complete_function_argument(
        function_name: &str,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        let numeric_only = matches!(
            function_name.to_uppercase().as_str(),
            "SUM" | "AVG"
        );
        
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let Some(table_name) = sym.resolve_table_name() {
                for col in schema.get_columns(table_name) {
                    if numeric_only {
                        let type_lower = col.data_type.to_lowercase();
                        if !type_lower.contains("int")
                           && !type_lower.contains("decimal")
                           && !type_lower.contains("numeric")
                           && !type_lower.contains("float")
                           && !type_lower.contains("double")
                        {
                            continue;
                        }
                    }
                    
                    let qualified_name = format!("{}.{}", sym.name, col.name);
                    items.push(CompletionItem {
                        label: qualified_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(col.data_type.clone()),
                        insert_text: qualified_name,
                        score: Self::column_score(col.is_primary_key, col.is_indexed),
                    });
                }
            }
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete at root/empty context.
    pub fn complete_root_context(context: &Context) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        let statement_starters = [
            ("SELECT", "Query data", 100),
            ("WITH", "Common Table Expression", 90),
            ("INSERT", "Insert data", 80),
            ("UPDATE", "Update data", 80),
            ("DELETE", "Delete data", 80),
            ("CREATE", "Create object", 70),
            ("ALTER", "Alter object", 60),
            ("DROP", "Drop object", 60),
        ];
        
        for (kw, detail, score) in statement_starters {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some(detail.to_string()),
                insert_text: kw.to_string(),
                score,
            });
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Generic completion fallback.
    pub fn complete_generic(
        semantic: &SemanticModel,
        context: &Context,
        generic_keywords: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        for kw in generic_keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 50,
            });
        }
        
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            items.push(CompletionItem {
                label: sym.name.clone(),
                kind: CompletionKind::Alias,
                detail: sym.resolve_table_name().map(|t| t.to_string()),
                insert_text: sym.name.clone(),
                score: 70,
            });
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    // ========================================================================
    // Utility Functions
    // ========================================================================

    /// Calculate score for a column based on its properties.
    pub fn column_score(is_pk: bool, is_indexed: bool) -> u32 {
        let mut score: u32 = 50;
        if is_pk {
            score += SCORE_PRIMARY_KEY;
        }
        if is_indexed {
            score += SCORE_INDEXED;
        }
        score
    }
    
    /// Qualify a table name with schema if not in default/public/main schema.
    pub fn qualify_table_name(schema: &str, table: &str, default_schema: &str) -> String {
        // Don't qualify for default schema, "public" (PostgreSQL), or "main" (SQLite)
        if schema == default_schema || schema == "public" || schema == "main" {
            table.to_string()
        } else {
            format!("{}.{}", schema, table)
        }
    }
    
    /// Calculate score for a schema suggestion.
    pub fn calculate_schema_score(schema: &str, ui_schema: &str) -> u32 {
        let mut score: i32 = 50;
        if schema == ui_schema {
            score += SCORE_UI_SCHEMA_HINT as i32;
        } else if schema == "public" {
            score += SCORE_PUBLIC_SCHEMA as i32;
        } else {
            score += PENALTY_CROSS_SCHEMA;
        }
        score.max(0) as u32
    }

    /// Filter items by prefix.
    pub fn filter_by_prefix(items: &mut Vec<CompletionItem>, prefix: &str) {
        if prefix.is_empty() {
            return;
        }
        let prefix_lower = prefix.to_lowercase();
        items.retain(|item| item.label.to_lowercase().starts_with(&prefix_lower));
    }
}

/// Resolve an alias to its actual table name.
pub fn resolve_table_name_from_alias(name: &str, semantic: &SemanticModel) -> Option<String> {
    for scope in &semantic.scopes {
        if let Some(sym) = scope.find_symbol(name) {
            if let Some(table) = sym.resolve_table_name() {
                return Some(table.to_string());
            }
        }
    }
    None
}
