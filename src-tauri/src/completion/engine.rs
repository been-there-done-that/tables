//! Completion engine.
//!
//! The core completion logic that combines:
//! - Semantic model (scopes, aliases)
//! - Schema graph (tables, columns, FKs)
//! - Cursor context (what kind of completion)
//!
//! Produces ranked completion items.

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
}

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
    pub fn complete(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        default_schema: Option<&str>,
    ) -> Vec<CompletionItem> {
        match &context.context_type {
            CursorContext::AfterDot { alias } => {
                Self::complete_after_dot(alias, semantic, context, schema)
            }
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                Self::complete_select_clause(semantic, context, schema)
            }
            CursorContext::RootContext => {
                Self::complete_root_context(context)
            }
            CursorContext::FromClause | CursorContext::JoinTable => {
                Self::complete_table_names(schema, semantic, context, default_schema)
            }
            CursorContext::JoinCondition { left_table, right_table } => {
                Self::complete_join_condition(left_table, right_table, semantic, schema)
            }
            CursorContext::JoinConditionRhs { .. } => {
                // For RHS, just suggest columns from visible tables
                // Similar to WHERE clause
                Self::complete_where_clause(semantic, context, schema)
            }
            CursorContext::WhereClause => {
                Self::complete_where_clause(semantic, context, schema)
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
    fn complete_after_dot(
        alias: &str,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Resolve alias to table
        let Some(symbol) = semantic.resolve_at_cursor(context.cursor_offset, alias) else {
            return items;
        };
        
        let Some(table_name) = symbol.resolve_table_name() else {
            return items;
        };
        
        // Get columns from schema
        let columns = schema.get_columns(table_name);
        
        if !columns.is_empty() {
            // Schema table match
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
            // CTE match
             if let Some(cte_cols) = semantic.ctes.get(&table_name.to_lowercase()) {
                for col_name in cte_cols {
                    items.push(CompletionItem {
                        label: col_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("CTE Column ({})", table_name)),
                        insert_text: col_name.clone(),
                        score: 90, // High score for CTE columns
                    });
                }
            }
        }
        
        // Filter by prefix
        Self::filter_by_prefix(&mut items, &context.prefix);
        
        // Sort by score descending
        items.sort_by(|a, b| b.score.cmp(&a.score));
        
        items
    }

    /// Complete in SELECT clause: columns from visible aliases + functions.
    fn complete_select_clause(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
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
    fn complete_table_names(
        schema: &SchemaGraph, 
        semantic: &SemanticModel,
        context: &Context,
        default_schema: Option<&str>,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        let target_schema = default_schema.unwrap_or("public");

        // 1. Schema tables
        for table_info in schema.tables.values() {
            // Filter: Only show tables from current schema if specified?
            // "tables should be coming from the current schema we have picked"
            // We prioritize current schema, but maybe we should strictly filter?
            // Let's implement prioritization + formatting first.
            
            let is_target = table_info.schema == target_schema;
            let is_public = table_info.schema == "public";

            // Format label: append schema if not public (or not target?)
            // User: "append schema name if not public"
            let label = if is_public {
                table_info.name.clone()
            } else {
                if is_target {
                   table_info.name.clone()
                } else {
                   format!("{}.{}", table_info.schema, table_info.name)
                }
            };
            
            // Score boost for current schema
            let score = if is_target { 100 } else { 80 };
            
            // Only add if it matches target OR if we want to show all
            // Interpreting "tables should be coming from the current schema we have picked"
            // strictly: Filter non-matching schemas?
            // But usually you want foreign tables available.
            // I'll show ALL, but ranked.
            // AND ensure non-public schemas are qualified in label.

            items.push(CompletionItem {
                label,
                kind: CompletionKind::Table,
                detail: Some(table_info.schema.clone()),
                insert_text: table_info.name.clone(), // Or qualified? Usually raw name works if in search path
                score,
            });
        }

        // 2. CTEs from current scope
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            if let SymbolKind::CTE { cte_name } = &sym.kind {
                items.push(CompletionItem {
                    label: cte_name.clone(),
                    kind: CompletionKind::Table, // Treat CTE as a table
                    detail: Some("CTE".to_string()),
                    insert_text: cte_name.clone(),
                    score: 110, // Higher priority than schema tables (local definition)
                });
            }
        }
        
        // 3. FROM-context keywords (JOIN, WHERE, etc.) - NO functions
        for kw in FROM_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 40, // Lower than tables
            });
        }
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score)); // Sort by score DESC
        items
    }

    /// Complete JOIN condition: infer from FK or heuristics.
    fn complete_join_condition(
        left_table: &Option<String>,
        right_table: &Option<String>,
        semantic: &SemanticModel,
        schema: &SchemaGraph,
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
    fn column_score(is_pk: bool, is_indexed: bool) -> u32 {
        let mut score = 50;
        if is_pk {
            score += 30;
        }
        if is_indexed {
            score += 20;
        }
        score
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
        
        let items = CompletionEngine::complete(&semantic, &context, &schema, None);
        
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
        
        let items = CompletionEngine::complete(&semantic, &context, &schema, None);
        
        // user_id (indexed) should rank higher than description (not indexed)
        let user_id_pos = items.iter().position(|i| i.label == "user_id");
        let description_pos = items.iter().position(|i| i.label == "description");
        
        if let (Some(uid), Some(desc)) = (user_id_pos, description_pos) {
            assert!(uid < desc, "Indexed column should rank higher");
        }
    }
}
