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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Table,
    Column,
    Alias,
    Keyword,
    Function,
    JoinCondition,
}

/// The completion engine.
pub struct CompletionEngine;

impl CompletionEngine {
    /// Generate completions for the given context.
    pub fn complete(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        match &context.context_type {
            CursorContext::AfterDot { alias } => {
                Self::complete_after_dot(alias, semantic, context, schema)
            }
            CursorContext::SelectClause => {
                Self::complete_select_clause(semantic, context, schema)
            }
            CursorContext::FromClause | CursorContext::JoinTable => {
                Self::complete_table_names(schema, &context.prefix)
            }
            CursorContext::JoinCondition { left_table, right_table } => {
                Self::complete_join_condition(left_table, right_table, semantic, schema)
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
        
        // Add visible aliases
        for sym in semantic.visible_symbols_at(context.cursor_offset) {
            items.push(CompletionItem {
                label: sym.name.clone(),
                kind: CompletionKind::Alias,
                detail: sym.resolve_table_name().map(|t| format!("alias for {}", t)),
                insert_text: format!("{}.", sym.name),
                score: 80,
            });
        }
        
        // Add common functions
        for func in &["COUNT", "SUM", "AVG", "MAX", "MIN", "COALESCE", "CASE"] {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: CompletionKind::Function,
                detail: Some("function".to_string()),
                insert_text: format!("{}()", func),
                score: 60,
            });
        }
        
        // Add keywords
        items.push(CompletionItem {
            label: "DISTINCT".to_string(),
            kind: CompletionKind::Keyword,
            detail: None,
            insert_text: "DISTINCT ".to_string(),
            score: 50,
        });
        
        Self::filter_by_prefix(&mut items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
        items
    }

    /// Complete table names for FROM/JOIN clauses.
    fn complete_table_names(schema: &SchemaGraph, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        for table_name in schema.table_names() {
            items.push(CompletionItem {
                label: table_name.to_string(),
                kind: CompletionKind::Table,
                detail: Some("table".to_string()),
                insert_text: table_name.to_string(),
                score: 100,
            });
        }
        
        Self::filter_by_prefix(&mut items, prefix);
        items.sort_by(|a, b| a.label.cmp(&b.label));
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
            
            if let Some((condition, score)) = schema.infer_join_condition(&left_resolved, &right_resolved) {
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

    /// Generic completion fallback.
    fn complete_generic(
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        
        // Keywords
        for kw in &["SELECT", "FROM", "WHERE", "JOIN", "ON", "AND", "OR", "ORDER BY", "GROUP BY"] {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: format!("{} ", kw),
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
        
        let items = CompletionEngine::complete(&semantic, &context, &schema);
        
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
        let items = CompletionEngine::complete_table_names(&schema, "");
        
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
        
        let items = CompletionEngine::complete(&semantic, &context, &schema);
        
        // user_id (indexed) should rank higher than description (not indexed)
        let user_id_pos = items.iter().position(|i| i.label == "user_id");
        let description_pos = items.iter().position(|i| i.label == "description");
        
        if let (Some(uid), Some(desc)) = (user_id_pos, description_pos) {
            assert!(uid < desc, "Indexed column should rank higher");
        }
    }
}
