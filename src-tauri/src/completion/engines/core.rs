//! Core Completion Logic
//!
//! Shared utilities and completion methods used by all engine variants.
//! Database-specific engines (Postgres, SQLite) delegate to these functions.

use std::collections::HashSet;

use sql_scope::{Source, ScopeTree};
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
        scope_tree: &ScopeTree,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        log::debug!("[AfterDot] Looking up '{}' at cursor offset {}", alias, context.cursor_offset);

        let visible = scope_tree.visible_at(context.cursor_offset);
        if let Some(source) = visible.get_source(alias) {
            log::debug!("[AfterDot] Resolved source: {:?}", source);

            if let Source::Cte { name: cte_name } = source {
                let cte_cols = get_cte_columns(scope_tree, context.cursor_offset, cte_name);
                log::debug!("[AfterDot] Found {} CTE columns for '{}'", cte_cols.len(), cte_name);
                for col_name in cte_cols {
                    items.push(CompletionItem {
                        label: col_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("CTE Column ({})", cte_name)),
                        insert_text: col_name.clone(),
                        score: 90,
                    });
                }
            } else if let Some(table_name) = resolve_table_name(source) {
                log::debug!("[AfterDot] Table name: '{}'", table_name);
                let columns = schema.get_columns(table_name);
                log::debug!("[AfterDot] Found {} columns", columns.len());
                for col in columns {
                    items.push(CompletionItem {
                        label: col.name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("{} ({})", col.data_type, table_name)),
                        insert_text: col.name.clone(),
                        score: Self::column_score(col.is_primary_key, col.is_indexed),
                    });
                }
            }
        }

        if items.is_empty() {
            log::debug!("[AfterDot] No alias match, checking if '{}' is a schema name", alias);
            let alias_lower = alias.to_lowercase();
            let schema_tables: Vec<_> = schema.tables.values()
                .filter(|t| t.schema.to_lowercase() == alias_lower)
                .collect();
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

        Self::filter_by_prefix(&mut items, &context.prefix);
        items
    }

    /// Complete table names for FROM/JOIN clauses.
    pub fn complete_table_names(
        schema: &SchemaGraph,
        scope_tree: &ScopeTree,
        context: &Context,
        default_schema: Option<&str>,
        from_keywords: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_schemas = HashSet::new();

        let ui_schema = default_schema.unwrap_or("public");

        log::debug!("[complete_table_names] default_schema={:?}, ui_schema={}, prefix='{}'",
            default_schema, ui_schema, context.prefix);

        // 1. Schema suggestions
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

        // 2. Schema tables
        for table_info in schema.tables.values() {
            let is_ui_schema = table_info.schema == ui_schema;
            let is_public = table_info.schema == "public";
            let is_main = table_info.schema == "main";

            let mut score: i32 = SCORE_CURSOR_RELEVANCE as i32;
            if is_ui_schema {
                score += SCORE_UI_SCHEMA_HINT as i32;
            } else if is_public || is_main {
                score += SCORE_PUBLIC_SCHEMA as i32;
            } else {
                score += PENALTY_CROSS_SCHEMA;
            }

            let insert_text = Self::qualify_table_name(&table_info.schema, &table_info.name, ui_schema);
            let label = if is_ui_schema || is_public || is_main {
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

        // 3. CTEs visible at cursor
        for (_alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
            if let Source::Cte { name: cte_name } = source {
                items.push(CompletionItem {
                    label: cte_name.clone(),
                    kind: CompletionKind::Table,
                    detail: Some("CTE".to_string()),
                    insert_text: cte_name.clone(),
                    score: SCORE_CURSOR_RELEVANCE + SCORE_CTE,
                });
            }
        }

        // 4. FROM keywords
        for kw in from_keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score: 40,
            });
        }

        Self::filter_by_prefix(&mut items, &context.prefix);
        items
    }

    /// Complete JOIN condition: infer from FK or heuristics.
    pub fn complete_join_condition(
        left_table: &Option<String>,
        right_table: &Option<String>,
        scope_tree: &ScopeTree,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        if let (Some(left), Some(right)) = (left_table, right_table) {
            let left_resolved = resolve_alias_to_table(left, scope_tree).unwrap_or_else(|| left.clone());
            let right_resolved = resolve_alias_to_table(right, scope_tree).unwrap_or_else(|| right.clone());

            if let Some((condition, score)) = schema.infer_join_condition(
                &left_resolved, &right_resolved, Some(left), Some(right)
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

        if let Some(left) = left_table {
            let left_resolved = resolve_alias_to_table(left, scope_tree).unwrap_or_else(|| left.clone());
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

    /// Complete in WHERE clause: columns + operators conditional on previous token.
    pub fn complete_where_clause(
        scope_tree: &ScopeTree,
        context: &Context,
        schema: &SchemaGraph,
        where_keywords: &[&str],
        where_functions: &[&str],
        operators: &[(&str, &str, u32)],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        let prev = context.previous_word.to_uppercase();
        let prev_is_keyword = ["WHERE", "AND", "OR", "NOT", "(", ","].contains(&prev.as_str());
        let prev_is_operator = operators.iter().any(|(op, _, _)| op.eq_ignore_ascii_case(&prev));

        if !prev_is_keyword && !prev_is_operator && !prev.is_empty() {
            for (op, detail, score) in operators {
                items.push(CompletionItem {
                    label: op.to_string(),
                    kind: CompletionKind::Operator,
                    detail: Some(detail.to_string()),
                    insert_text: op.to_string(),
                    score: *score,
                });
            }
        }

        let show_columns = prev_is_keyword || prev_is_operator || prev.is_empty() || !context.prefix.is_empty();

        if show_columns {
            for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
                if let Source::Cte { name: cte_name } = source {
                    for col_name in get_cte_columns(scope_tree, context.cursor_offset, cte_name) {
                        let qualified_name = format!("{}.{}", alias, col_name);
                        items.push(CompletionItem {
                            label: qualified_name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("CTE Column ({})", cte_name)),
                            insert_text: qualified_name,
                            score: 50,
                        });
                    }
                } else if let Some(table_name) = resolve_table_name(source) {
                    for col in schema.get_columns(table_name) {
                        let qualified_name = format!("{}.{}", alias, col.name);
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
        }

        Self::filter_by_prefix(&mut items, &context.prefix);
        items
    }

    /// Complete in SELECT clause: columns from visible aliases + functions.
    pub fn complete_select_clause(
        scope_tree: &ScopeTree,
        context: &Context,
        schema: &SchemaGraph,
        select_keywords: &[&str],
        select_functions: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        let mut seen_labels = HashSet::new();

        for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
            if seen_labels.insert(alias.clone()) {
                items.push(CompletionItem {
                    label: alias.clone(),
                    kind: CompletionKind::Alias,
                    detail: resolve_table_name(source).map(|t| format!("alias for {}", t)),
                    insert_text: format!("{}.", alias),
                    score: 80,
                });
            }

            if let Source::Cte { name: cte_name } = source {
                for col_name in get_cte_columns(scope_tree, context.cursor_offset, cte_name) {
                    if seen_labels.insert(col_name.clone()) {
                        items.push(CompletionItem {
                            label: col_name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("CTE Column ({})", cte_name)),
                            insert_text: col_name.clone(),
                            score: 70,
                        });
                    }
                }
            } else if let Some(table_name) = resolve_table_name(source) {
                for col in schema.get_columns(table_name) {
                    if seen_labels.insert(col.name.clone()) {
                        items.push(CompletionItem {
                            label: col.name.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("{} ({})", col.data_type, alias)),
                            insert_text: col.name.clone(),
                            score: 70,
                        });
                    }
                }
            }
        }

        items.push(CompletionItem {
            label: "*".to_string(),
            kind: CompletionKind::Keyword,
            detail: Some("All columns".to_string()),
            insert_text: "*".to_string(),
            score: 90,
        });

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

        for kw in select_keywords {
            if seen_labels.insert(kw.to_string()) {
                let mut score = 50;
                if matches!(context.context_type, CursorContext::AfterSelectList) {
                    if *kw == "FROM" { score = 200; }
                    else if *kw == "AS" { score = 150; }
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
        items
    }

    /// Complete function arguments with type-aware filtering.
    pub fn complete_function_argument(
        function_name: &str,
        scope_tree: &ScopeTree,
        context: &Context,
        schema: &SchemaGraph,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        let numeric_only = matches!(
            function_name.to_uppercase().as_str(),
            "SUM" | "AVG"
        );

        for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
            if let Source::Cte { name: cte_name } = source {
                // CTE columns have no type info — always include them (skip numeric filter)
                for col_name in get_cte_columns(scope_tree, context.cursor_offset, cte_name) {
                    let qualified_name = format!("{}.{}", alias, col_name);
                    items.push(CompletionItem {
                        label: qualified_name.clone(),
                        kind: CompletionKind::Column,
                        detail: Some(format!("CTE Column ({})", cte_name)),
                        insert_text: qualified_name,
                        score: 50,
                    });
                }
            } else if let Some(table_name) = resolve_table_name(source) {
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
                    let qualified_name = format!("{}.{}", alias, col.name);
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
        items
    }

    /// Complete at root/empty context.
    pub fn complete_root_context(context: &Context) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        let statement_starters = [
            ("SELECT", "Query data", 200), // Boosted score
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
        items
    }

    /// Generic completion fallback.
    pub fn complete_generic(
        scope_tree: &ScopeTree,
        context: &Context,
        generic_keywords: &[&str],
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for kw in generic_keywords {
            let score = match *kw {
                "SELECT" => 200,
                "INSERT" | "UPDATE" | "DELETE" => 90,
                "WHERE" | "FROM" | "JOIN" => 80,
                _ => 50,
            };
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: None,
                insert_text: kw.to_string(),
                score,
            });
        }

        for (alias, source) in &scope_tree.visible_at(context.cursor_offset).sources {
            items.push(CompletionItem {
                label: alias.clone(),
                kind: CompletionKind::Alias,
                detail: resolve_table_name(source).map(|t| t.to_string()),
                insert_text: alias.clone(),
                score: 70,
            });
        }

        Self::filter_by_prefix(&mut items, &context.prefix);
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

    /// Filter items by prefix using sql_scope::match_score for fuzzy matching.
    pub fn filter_by_prefix(items: &mut Vec<CompletionItem>, prefix: &str) {
        if prefix.is_empty() {
            // No prefix — just sort by base score
            items.sort_by(|a, b| b.score.cmp(&a.score));
            return;
        }
        items.retain(|item| sql_scope::match_score(prefix, &item.label) > 0);
        items.sort_by(|a, b| {
            let sa = sql_scope::match_score(prefix, &a.label);
            let sb = sql_scope::match_score(prefix, &b.label);
            sb.cmp(&sa).then(b.score.cmp(&a.score))
        });
    }
}

/// Extract the table/CTE name from a Source, recursively unwrapping Alias.
fn resolve_table_name(source: &sql_scope::Source) -> Option<&str> {
    match source {
        sql_scope::Source::Table { name, .. } => Some(name.as_str()),
        sql_scope::Source::Cte { name } => Some(name.as_str()),
        sql_scope::Source::Alias { target, .. } => resolve_table_name(target),
        sql_scope::Source::DerivedTable { .. } => None,
    }
}

/// Get the column list for a CTE visible at the cursor position.
fn get_cte_columns<'a>(scope_tree: &'a sql_scope::ScopeTree, cursor: usize, cte_name: &str) -> &'a [String] {
    scope_tree
        .scope_at(cursor)
        .and_then(|s| s.cte_sources.get(cte_name))
        .map(|info| info.columns.as_slice())
        .unwrap_or(&[])
}

/// Resolve an alias to its actual table/CTE name using the full scope tree.
fn resolve_alias_to_table(alias: &str, scope_tree: &sql_scope::ScopeTree) -> Option<String> {
    scope_tree.visible_at(usize::MAX)
        .get_source(alias)
        .and_then(|s| resolve_table_name(s))
        .map(|s| s.to_string())
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn resolve_table_name_from_table_source() {
        let src = sql_scope::Source::Table { schema: None, name: "users".to_string() };
        assert_eq!(resolve_table_name(&src), Some("users"));
    }

    #[test]
    fn resolve_table_name_from_cte_source() {
        let src = sql_scope::Source::Cte { name: "my_cte".to_string() };
        assert_eq!(resolve_table_name(&src), Some("my_cte"));
    }

    #[test]
    fn resolve_table_name_unwraps_alias() {
        let src = sql_scope::Source::Alias {
            alias: "u".to_string(),
            target: Box::new(sql_scope::Source::Table { schema: None, name: "users".to_string() }),
        };
        assert_eq!(resolve_table_name(&src), Some("users"));
    }

    #[test]
    fn resolve_table_name_derived_is_none() {
        let src = sql_scope::Source::DerivedTable { scope_id: 0 };
        assert_eq!(resolve_table_name(&src), None);
    }
}

#[cfg(test)]
mod scope_tree_method_tests {
    use super::*;
    use sql_scope::{ScopeTree, Source};
    use sql_scope::scope::tree::{Scope, ScopeType, CteInfo};
    use crate::completion::context::{Context, CursorContext};
    use crate::completion::schema::loader::MockSchemaLoader;

    fn tree_with_alias(alias: &str, table: &str) -> ScopeTree {
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.sources.insert(alias.to_string(), Source::Alias {
            alias: alias.to_string(),
            target: Box::new(Source::Table { schema: None, name: table.to_string() }),
        });
        tree.add_scope(scope);
        tree
    }

    fn tree_with_cte(cte_name: &str, cols: Vec<&str>) -> ScopeTree {
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.cte_sources.insert(cte_name.to_string(), CteInfo {
            scope_id: 0,
            columns: cols.into_iter().map(|s| s.to_string()).collect(),
            is_recursive: false,
        });
        tree.add_scope(scope);
        tree
    }

    fn ctx(cursor: usize, ctx_type: CursorContext) -> Context {
        Context { cursor_offset: cursor, context_type: ctx_type, prefix: String::new(), previous_word: String::new(), scope_depth: 0 }
    }

    #[test]
    fn after_dot_resolves_schema_table_alias() {
        let schema = MockSchemaLoader::create_test_schema();
        let tree = tree_with_alias("u", "users");
        let items = CoreCompletionEngine::complete_after_dot("u", &tree, &ctx(20, CursorContext::AfterDot { alias: "u".to_string() }), &schema);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"id"), "expected 'id' column from users table");
        assert!(labels.contains(&"email"), "expected 'email' column from users table");
    }

    #[test]
    fn after_dot_resolves_cte_columns() {
        let schema = MockSchemaLoader::create_test_schema();
        let mut tree = ScopeTree::new();
        let mut scope = Scope::new(0, None, ScopeType::Root, 0..1000);
        scope.cte_sources.insert("orders_cte".to_string(), CteInfo {
            scope_id: 0,
            columns: vec!["order_id".to_string(), "total".to_string()],
            is_recursive: false,
        });
        scope.sources.insert("oc".to_string(), Source::Cte { name: "orders_cte".to_string() });
        tree.add_scope(scope);
        let items = CoreCompletionEngine::complete_after_dot("oc", &tree, &ctx(20, CursorContext::AfterDot { alias: "oc".to_string() }), &schema);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"order_id"), "expected CTE column 'order_id'");
        assert!(labels.contains(&"total"), "expected CTE column 'total'");
    }

    #[test]
    fn table_names_includes_cte() {
        let schema = MockSchemaLoader::create_test_schema();
        let tree = tree_with_cte("my_cte", vec!["id", "name"]);
        let items = CoreCompletionEngine::complete_table_names(&schema, &tree, &ctx(50, CursorContext::FromClause), None, &[]);
        assert!(items.iter().any(|i| i.label == "my_cte"), "CTE must appear in FROM completions");
    }

    #[test]
    fn filter_by_prefix_uses_match_score_acronym() {
        let mut items = vec![
            CompletionItem { label: "user_id".to_string(), kind: CompletionKind::Column, detail: None, insert_text: "user_id".to_string(), score: 50 },
            CompletionItem { label: "order_total".to_string(), kind: CompletionKind::Column, detail: None, insert_text: "order_total".to_string(), score: 50 },
        ];
        CoreCompletionEngine::filter_by_prefix(&mut items, "ui");
        assert_eq!(items.len(), 1, "acronym 'ui' should match only 'user_id'");
        assert_eq!(items[0].label, "user_id");
    }
}
