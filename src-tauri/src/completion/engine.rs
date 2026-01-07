//! Completion engine.
//!
//! The core completion logic that combines:
//! - Semantic model (scopes, aliases)
//! - Schema graph (tables, columns, FKs)
//! - Cursor context (what kind of completion)
//! - Database capabilities (engine-specific behavior)
//!
//! Produces ranked completion items.

use crate::adapter::DatabaseCapabilities;
use crate::completion::analysis::{SemanticModel, SymbolKind};
use crate::completion::context::{Context, CursorContext};
use crate::completion::schema::SchemaGraph;

/// A completion item to return to the editor.
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// Display label
    pub label: String,
    /// Item kind for icon
    pub kind: CompletionKind,
    /// Detail text (e.g., "column of users")
    pub detail: Option<String>,
    /// Text to insert
    pub insert_text: String,
    /// Ranking score (higher = better)
    pub score: u32,
}

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[repr(u8)]
pub enum CompletionKind {
    Table = 0,
    Column = 1,
    Alias = 2,
    Keyword = 3,
    Function = 4,
    JoinCondition = 5,
    Schema = 6,
    Operator = 7,
}

// ============================================================================
// Scoring Constants (Additive Model)
// ============================================================================

/// Cursor context relevance (highest priority)
const SCORE_CURSOR_RELEVANCE: u32 = 1000;
/// Table/alias already in query scope
const SCORE_QUERY_SCOPE_MATCH: u32 = 800;
/// Alias matches exactly
const SCORE_ALIAS_MATCH: u32 = 700;
/// Exact prefix match
const SCORE_EXACT_MATCH: u32 = 600;
/// Prefix starts with typed text
const SCORE_PREFIX_MATCH: u32 = 400;
/// Matches UI schema hint (dropdown selection)
const SCORE_UI_SCHEMA_HINT: u32 = 300;
/// Matches default schema
const SCORE_DEFAULT_SCHEMA: u32 = 200;
/// Matches public schema
const SCORE_PUBLIC_SCHEMA: u32 = 150;
/// CTE definitions (local, highest priority for tables - beats UI schema hint)
const SCORE_CTE: u32 = 400;
/// Primary key column
const SCORE_PRIMARY_KEY: u32 = 30;
/// Indexed column
const SCORE_INDEXED: u32 = 20;

/// Penalty for cross-schema (not in selected schema)
const PENALTY_CROSS_SCHEMA: i32 = -250;
/// Penalty for ambiguous column (exists in multiple tables)
const PENALTY_AMBIGUITY: i32 = -400;

/// The completion engine.
pub struct CompletionEngine;

// ============================================================================
// Context-Specific Keyword/Function Subsets
// ============================================================================

/// Keywords valid in SELECT clause (after SELECT, before FROM)
const SELECT_KEYWORDS: &[&str] = &[
    "DISTINCT", "AS", "CASE", "WHEN", "THEN", "ELSE", "END", "FROM",
];

/// Functions valid in SELECT clause
const SELECT_FUNCTIONS: &[&str] = &[
    "COUNT", "SUM", "AVG", "MAX", "MIN", "ABS", "ROUND", "CEIL", "FLOOR", "POWER", "SQRT",
    "UPPER", "LOWER", "LENGTH", "TRIM", "SUBSTRING", "REPLACE", "CONCAT",
    "COALESCE", "NULLIF", "CAST", "CONVERT",
    "CURRENT_TIME", "CURRENT_DATE", "NOW", "DATE", "TIME", "TIMESTAMP",
    "EXTRACT", "DATE_PART", "TO_CHAR", "TO_DATE", "TO_TIMESTAMP",
];

/// Keywords valid in FROM clause (after FROM, for JOINs)
const FROM_KEYWORDS: &[&str] = &[
    "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "FULL JOIN", "CROSS JOIN",
    "ON", "WHERE", "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET",
];

/// Keywords valid in WHERE clause (logical operators, comparisons)
const WHERE_KEYWORDS: &[&str] = &[
    "AND", "OR", "NOT", "IN", "IS", "NULL", "LIKE", "BETWEEN", "EXISTS", "ANY", "ALL",
    "ORDER BY", "GROUP BY", "HAVING", "LIMIT",
];

/// Functions valid in WHERE clause
const WHERE_FUNCTIONS: &[&str] = &[
    "COALESCE", "NULLIF", "CAST", "UPPER", "LOWER", "LENGTH", "TRIM",
    "CURRENT_TIME", "CURRENT_DATE", "NOW",
];

/// Keywords for generic/unknown context (broad fallback)
const GENERIC_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "ON", "AND", "OR", "ORDER BY", "GROUP BY",
    "WITH", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
];

impl CompletionEngine {
    /// Generate completions for the given context.
    ///
    /// # Arguments
    /// * `semantic` - The semantic model containing aliases and scopes
    /// * `context` - The cursor context and prefix
    /// * `schema` - The schema graph with tables and columns
    /// * `default_schema` - Optional default schema name (falls back to capabilities or "public")
    /// * `capabilities` - Optional database capabilities for engine-specific behavior
    pub fn complete(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        default_schema: Option<&str>,
        capabilities: Option<&DatabaseCapabilities>,
    ) -> Vec<CompletionItem> {
        // Resolve effective default schema from capabilities if not provided
        let effective_schema = default_schema
            .map(|s| s.to_string())
            .or_else(|| capabilities.and_then(|c| c.default_schema.clone()))
            .unwrap_or_else(|| "public".to_string());
        
        match &context.context_type {
            CursorContext::AfterDot { alias } => {
                Self::complete_after_dot(alias, semantic, context, schema)
            }
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                Self::complete_select_clause(semantic, context, schema, capabilities)
            }
            CursorContext::RootContext => {
                Self::complete_root_context(context)
            }
            CursorContext::FromClause | CursorContext::JoinTable => {
                Self::complete_table_names(schema, semantic, context, Some(&effective_schema))
            }
            CursorContext::JoinCondition { left_table, right_table } => {
                Self::complete_join_condition(left_table, right_table, semantic, schema, capabilities)
            }
            CursorContext::JoinConditionRhs { .. } => {
                // For RHS, just suggest columns from visible tables
                Self::complete_where_clause(semantic, context, schema, capabilities)
            }
            CursorContext::WhereClause => {
                Self::complete_where_clause(semantic, context, schema, capabilities)
            }
            CursorContext::FunctionArgument { function_name } => {
                Self::complete_function_argument(function_name, semantic, context, schema)
            }
            CursorContext::Unknown => {
                Self::complete_generic(semantic, context, schema)
            }
        }
    }

    /// Complete after a dot: `alias.|` → columns of the aliased table.
    /// Also handles `schema.|` → tables in that schema.
    fn complete_after_dot(
        alias: &str,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        log::debug!("[AfterDot] Looking up '{}' at cursor offset {}", alias, context.cursor_offset);
        
        // First, try to resolve as a table alias (e.g., SELECT u.| FROM users u)
        if let Some(symbol) = semantic.resolve_at_cursor(context.cursor_offset, alias) {
            log::debug!("[AfterDot] Resolved as alias: {:?}", symbol.kind);
            
            if let Some(table_name) = symbol.resolve_table_name() {
                log::debug!("[AfterDot] Table name: '{}'", table_name);
                
                // Get columns from schema
                let columns = schema.get_columns(table_name);
                log::debug!("[AfterDot] Found {} columns for table '{}'", columns.len(), table_name);
                
                if !columns.is_empty() {
                    for col in &columns {
                        log::debug!("[AfterDot] Column: {} ({})", col.name, col.data_type);
                    }
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
        
        // If no alias match, check if this is a schema name (e.g., SELECT * FROM public.|)
        if items.is_empty() {
            log::debug!("[AfterDot] No alias match, checking if '{}' is a schema name", alias);
            
            // Look for tables in this schema
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
                        insert_text: table.name.clone(), // Don't need schema prefix since user already typed it
                        score: SCORE_CURSOR_RELEVANCE + SCORE_UI_SCHEMA_HINT,
                    });
                }
            }
        }
        
        // Filter by prefix
        Self::filter_by_prefix(&mut items, &context.prefix);
        
        log::debug!("[AfterDot] Returning {} items after filtering by prefix '{}'", items.len(), context.prefix);
        
        // Sort by score descending
        items.sort_by(|a, b| b.score.cmp(&a.score));
        
        items
    }

    /// Complete in SELECT clause: columns from visible aliases + functions.
    fn complete_select_clause(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        _capabilities: Option<&DatabaseCapabilities>,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_labels = std::collections::HashSet::new();
        
        // Add visible aliases and their columns
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            // Suggest the alias itself
            if seen_labels.insert(sym.name.clone()) {
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind: CompletionKind::Alias,
                    detail: sym.resolve_table_name().map(|t| format!("alias for {}", t)),
                    insert_text: format!("{}.", sym.name),
                    score: 80,
                });
            }

            // Also suggest columns for this alias
            if let Some(table_name) = sym.resolve_table_name() {
                // Check schema tables
                for col in schema.get_columns(table_name) {
                    if seen_labels.insert(col.name.clone()) {
                        items.push(CompletionItem {
                            label: col.name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("{} ({})", col.data_type, sym.name)),
                            insert_text: col.name.clone(),
                            score: 70, // Slightly lower than alias
                        });
                    }
                }
                
                // Check CTE columns if not in schema
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

        // Add SELECT-context functions (aggregates, etc.)
        for func in SELECT_FUNCTIONS {
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
        
        // Add SELECT-context keywords only (DISTINCT, AS, CASE, FROM)
        for kw in SELECT_KEYWORDS {
            if seen_labels.insert(kw.to_string()) {
                let mut score = 50;
                
                // Boost FROM and AS if we are after a complete expression
                if matches!(context.context_type, CursorContext::AfterSelectList) {
                    if *kw == "FROM" {
                        score = 200; // Prioritize FROM after SELECT list
                    } else if *kw == "AS" {
                        score = 150; // Prioritize AS for aliasing
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

    /// Complete table names for FROM/JOIN clauses.
    /// 
    /// Uses additive scoring and explicit schema qualification:
    /// - Tables from UI schema hint get +300
    /// - Tables from default schema get +200
    /// - Tables from public schema get +150
    /// - Cross-schema tables get -250 penalty
    /// - Always qualify non-default/non-public schemas in insert_text
    fn complete_table_names(
        schema: &SchemaGraph, 
        semantic: &SemanticModel,
        context: &Context,
        default_schema: Option<&str>,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_schemas = std::collections::HashSet::new();
        
        let ui_schema = default_schema.unwrap_or("public");

        // 1. Add schema suggestions first (for schema.table completion)
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

        // 2. Schema tables with explicit qualification
        for table_info in schema.tables.values() {
            let is_ui_schema = table_info.schema == ui_schema;
            let is_public = table_info.schema == "public";
            
            // Calculate score using additive model
            let mut score: i32 = SCORE_CURSOR_RELEVANCE as i32; // Base: cursor relevance for FROM clause
            
            if is_ui_schema {
                score += SCORE_UI_SCHEMA_HINT as i32;
            } else if is_public {
                score += SCORE_PUBLIC_SCHEMA as i32;
            } else {
                score += PENALTY_CROSS_SCHEMA;
            }
            
            // Explicit qualification rule:
            // Insert qualified name for non-default, non-public schemas
            let insert_text = Self::qualify_table_name(
                &table_info.schema, 
                &table_info.name, 
                ui_schema
            );
            
            // Label shows table name with schema hint for disambiguation
            let label = if is_ui_schema || is_public {
                table_info.name.clone()
            } else {
                format!("{} ({})", table_info.name, table_info.schema)
            };

            items.push(CompletionItem {
                label,
                kind: CompletionKind::Table,
                detail: Some(table_info.schema.clone()),
                insert_text,
                score: score.max(0) as u32,
            });
        }

        // 3. CTEs from current scope (highest priority - local definitions)
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let SymbolKind::CTE { cte_name } = &sym.kind {
                items.push(CompletionItem {
                    label: cte_name.clone(),
                    kind: CompletionKind::Table,
                    detail: Some("CTE".to_string()),
                    insert_text: cte_name.clone(),
                    score: SCORE_CURSOR_RELEVANCE + SCORE_CTE, // CTEs beat all tables
                });
            }
        }
        
        // 4. FROM-context keywords (JOIN, WHERE, etc.) - lower priority
        for kw in FROM_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 40,
            });
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete JOIN condition: infer from FK or heuristics.
    fn complete_join_condition(
        left_table: &Option<String>,
        right_table: &Option<String>,
        semantic: &SemanticModel,
        schema: &SchemaGraph,
        _capabilities: Option<&DatabaseCapabilities>,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // If we have both tables, try to infer join condition
        if let (Some(left), Some(right)) = (left_table, right_table) {
            // Resolve aliases to actual table names
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
    fn complete_where_clause(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        _capabilities: Option<&DatabaseCapabilities>,
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

        // Add WHERE-context functions only
        for func in WHERE_FUNCTIONS {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: CompletionKind::Function,
                detail: Some("function".to_string()),
                insert_text: format!("{}()", func),
                score: 60,
            });
        }

        // Add WHERE-context keywords only (AND, OR, NOT, etc.)
        for kw in WHERE_KEYWORDS {
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

    /// Complete function arguments with type-aware filtering.
    fn complete_function_argument(
        function_name: &str,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Determine expected types based on function
        let numeric_only = matches!(
            function_name.to_uppercase().as_str(),
            "SUM" | "AVG"
        );
        
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let Some(table_name) = sym.resolve_table_name() {
                for col in schema.get_columns(table_name) {
                    // Type filtering
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

    
    /// Complete at root/empty context: `|` → suggest statement starters
    fn complete_root_context(context: &Context) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Statement starters with high priority
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
    fn complete_generic(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        // Generic Keywords (broad fallback, no functions - too noisy)
        for kw in GENERIC_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 50,
            });
        }
        
        // Visible aliases
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

    /// Calculate score for a column based on its properties.
    /// Uses additive scoring model.
    fn column_score(is_pk: bool, is_indexed: bool) -> u32 {
        let mut score: u32 = 50; // Base score
        if is_pk {
            score += SCORE_PRIMARY_KEY;
        }
        if is_indexed {
            score += SCORE_INDEXED;
        }
        score
    }
    
    /// Qualify a table name with schema if not in default/public schema.
    /// 
    /// Key rule: Always insert qualified names for non-default schemas.
    /// This prevents ambiguity and matches PostgreSQL semantics.
    fn qualify_table_name(schema: &str, table: &str, default_schema: &str) -> String {
        if schema == default_schema || schema == "public" {
            table.to_string()
        } else {
            format!("{}.{}", schema, table)
        }
    }
    
    /// Calculate score for a schema suggestion.
    fn calculate_schema_score(schema: &str, ui_schema: &str) -> u32 {
        let mut score: i32 = 50; // Base
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
    fn filter_by_prefix(items: &mut Vec<CompletionItem>, prefix: &str) {
        if prefix.is_empty() {
            return;
        }
        let prefix_lower = prefix.to_lowercase();
        items.retain(|item| item.label.to_lowercase().starts_with(&prefix_lower));
    }
}

/// Resolve an alias to its actual table name.
fn resolve_table_name_from_alias(name: &str, semantic: &SemanticModel) -> Option<String> {
    // Check if it's an alias in any scope
    for scope in &semantic.scopes {
        if let Some(sym) = scope.find_symbol(name) {
            if let Some(table) = sym.resolve_table_name() {
                return Some(table.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::schema::MockSchemaLoader;
    use crate::completion::analysis::scope::{Scope, Symbol};

    fn create_test_semantic_with_alias(alias: &str, table: &str) -> SemanticModel {
        let mut model = SemanticModel::new();
        let mut scope = Scope::new(0, None, 0..100);
        scope.symbols.push(Symbol::table_alias(alias, table, 10..15));
        model.scopes.push(scope);
        model
    }

    #[test]
    fn test_complete_after_dot() {
        let schema = MockSchemaLoader::create_test_schema();
        let semantic = create_test_semantic_with_alias("u", "users");
        let context = Context {
            cursor_offset: 20,
            context_type: CursorContext::AfterDot { alias: "u".to_string() },
            prefix: String::new(),
            scope_depth: 0,
        };
        
        let items = CompletionEngine::complete(&semantic, &context, &schema, None, None);
        
        // Should return columns of users table
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"id"), "Should contain 'id'");
        assert!(labels.contains(&"email"), "Should contain 'email'");
        assert!(labels.contains(&"created_at"), "Should contain 'created_at'");
        
        // Should NOT contain columns from other tables
        assert!(!labels.contains(&"user_id"), "Should NOT contain 'user_id' (orders column)");
    }

    #[test]
    fn test_complete_table_names() {
        let schema = MockSchemaLoader::create_test_schema();
        let semantic = SemanticModel::new(); // Empty semantic model
        let context = Context {
            cursor_offset: 0,
            context_type: CursorContext::FromClause,
            prefix: "".to_string(),
            scope_depth: 0,
        };
        
        let items = CompletionEngine::complete_table_names(&schema, &semantic, &context, None);
        
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"users"));
        assert!(labels.contains(&"orders"));
        assert!(labels.contains(&"teams"));
    }

    #[test]
    fn test_join_condition_from_fk() {
        let schema = MockSchemaLoader::create_test_schema();
        let semantic = SemanticModel::new();
        
        let items = CompletionEngine::complete_join_condition(
            &Some("users".to_string()),
            &Some("orders".to_string()),
            &semantic,
            &schema,
            None,
        );
        
        // Should suggest the FK-based join condition with high score
        let top_item = items.first();
        assert!(top_item.is_some());
        let top = top_item.unwrap();
        assert_eq!(top.score, 100, "FK match should have score 100");
        assert!(top.insert_text.contains("user_id"));
    }

    #[test]
    fn test_indexed_column_ranking() {
        let schema = MockSchemaLoader::create_test_schema();
        let semantic = create_test_semantic_with_alias("o", "orders");
        let context = Context {
            cursor_offset: 20,
            context_type: CursorContext::AfterDot { alias: "o".to_string() },
            prefix: String::new(),
            scope_depth: 0,
        };
        
        let items = CompletionEngine::complete(&semantic, &context, &schema, None, None);
        
        // user_id (indexed) should rank higher than description (not indexed)
        let user_id_pos = items.iter().position(|i| i.label == "user_id");
        let description_pos = items.iter().position(|i| i.label == "description");
        
        if let (Some(uid), Some(desc)) = (user_id_pos, description_pos) {
            assert!(uid < desc, "Indexed column should rank higher");
        }
    }
}
