//! PostgreSQL Completion Engine
//!
//! PostgreSQL-specific keywords, functions, and completion behaviors.
//! Uses the shared core logic from `core.rs`.

use crate::adapter::DatabaseCapabilities;
use crate::completion::context::{Context, CursorContext};
use crate::completion::document::Dialect;
use crate::completion::items::{CompletionItem, CompletionKind};
use crate::completion::schema::SchemaGraph;
use super::core::CoreCompletionEngine;
use super::CompletionEngineVariant;

/// PostgreSQL-specific completion engine.
pub struct PostgresEngine;

impl PostgresEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PostgresEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PostgreSQL-Specific Keywords and Functions
// ============================================================================

/// Keywords valid in SELECT clause (PostgreSQL)
const SELECT_KEYWORDS: &[&str] = &[
    "DISTINCT", "AS", "CASE", "WHEN", "THEN", "ELSE", "END", "FROM",
    "DISTINCT ON", // PostgreSQL-specific
];

/// Functions valid in SELECT clause (PostgreSQL)
const SELECT_FUNCTIONS: &[&str] = &[
    // Aggregate functions
    "COUNT", "SUM", "AVG", "MAX", "MIN", "ABS", "ROUND", "CEIL", "FLOOR", "POWER", "SQRT",
    // String functions
    "UPPER", "LOWER", "LENGTH", "TRIM", "SUBSTRING", "REPLACE", "CONCAT",
    "LEFT", "RIGHT", "LPAD", "RPAD", "INITCAP", "REVERSE",
    // Null handling
    "COALESCE", "NULLIF", "CAST",
    // Date/time
    "CURRENT_TIME", "CURRENT_DATE", "NOW", "DATE", "TIME", "TIMESTAMP",
    "EXTRACT", "DATE_PART", "TO_CHAR", "TO_DATE", "TO_TIMESTAMP",
    "AGE", "DATE_TRUNC", "MAKE_DATE", "MAKE_TIME", "MAKE_TIMESTAMP",
    // PostgreSQL-specific aggregate functions
    "ARRAY_AGG", "STRING_AGG", "JSON_AGG", "JSONB_AGG",
    "BOOL_AND", "BOOL_OR", "BIT_AND", "BIT_OR",
    // PostgreSQL JSON functions
    "JSON_BUILD_OBJECT", "JSONB_BUILD_OBJECT", "JSON_BUILD_ARRAY", "JSONB_BUILD_ARRAY",
    "TO_JSON", "TO_JSONB", "ROW_TO_JSON", "JSON_OBJECT", "JSON_ARRAY",
    // PostgreSQL array functions
    "ARRAY_LENGTH", "ARRAY_DIMS", "ARRAY_LOWER", "ARRAY_UPPER",
    "UNNEST", "ARRAY_TO_STRING", "STRING_TO_ARRAY",
    // PostgreSQL-specific
    "GENERATE_SERIES", "REGEXP_MATCHES", "REGEXP_REPLACE", "REGEXP_SPLIT_TO_ARRAY",
    "PG_TYPEOF",
];

/// Keywords valid in FROM clause (PostgreSQL)
const FROM_KEYWORDS: &[&str] = &[
    "JOIN", "INNER JOIN", "LEFT JOIN", "RIGHT JOIN", "FULL JOIN", "CROSS JOIN",
    "LATERAL", // PostgreSQL-specific
    "ON", "WHERE", "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET",
    "FETCH FIRST", "FOR UPDATE", "FOR SHARE", // PostgreSQL-specific
];

/// Keywords valid in WHERE clause (PostgreSQL)
const WHERE_KEYWORDS: &[&str] = &[
    "AND", "OR", "NOT", "IN", "IS", "NULL", "LIKE", "ILIKE", // ILIKE is PostgreSQL-specific
    "BETWEEN", "EXISTS", "ANY", "ALL", "SIMILAR TO",
    "ORDER BY", "GROUP BY", "HAVING", "LIMIT",
    "RETURNING", // PostgreSQL-specific (for INSERT/UPDATE/DELETE)
];

/// Functions valid in WHERE clause (PostgreSQL)
const WHERE_FUNCTIONS: &[&str] = &[
    "COALESCE", "NULLIF", "CAST", "UPPER", "LOWER", "LENGTH", "TRIM",
    "CURRENT_TIME", "CURRENT_DATE", "NOW",
    // PostgreSQL-specific
    "REGEXP_MATCH", "STARTS_WITH", "POSITION",
];

/// Keywords for generic/unknown context (PostgreSQL)
const GENERIC_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "ON", "AND", "OR", "ORDER BY", "GROUP BY",
    "WITH", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
    // PostgreSQL-specific
    "RETURNING", "EXPLAIN", "EXPLAIN ANALYZE", "VACUUM", "ANALYZE",
    "GRANT", "REVOKE", "TRUNCATE", "COPY",
];

impl CompletionEngineVariant for PostgresEngine {
    fn dialect(&self) -> Dialect {
        Dialect::Postgres
    }
    
    fn keywords(&self, context: &Context) -> Vec<&'static str> {
        match &context.context_type {
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                SELECT_KEYWORDS.to_vec()
            }
            CursorContext::FromClause | CursorContext::JoinTable => {
                FROM_KEYWORDS.to_vec()
            }
            CursorContext::WhereClause => {
                WHERE_KEYWORDS.to_vec()
            }
            _ => GENERIC_KEYWORDS.to_vec()
        }
    }
    
    fn functions(&self, context: &Context) -> Vec<&'static str> {
        match &context.context_type {
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                SELECT_FUNCTIONS.to_vec()
            }
            CursorContext::WhereClause => {
                WHERE_FUNCTIONS.to_vec()
            }
            _ => vec![]
        }
    }
    
    fn default_schema(&self) -> &str {
        "public"
    }
    
    fn operators(&self) -> Vec<(&'static str, &'static str, u32)> {
        vec![
            ("=", "Equal", 90),
            ("<>", "Not equal", 90),
            (">", "Greater than", 90),
            ("<", "Less than", 90),
            (">=", "Greater or equal", 90),
            ("<=", "Less or equal", 90),
            ("LIKE", "Pattern match", 85),
            ("ILIKE", "Case-insensitive match", 85), // Postgres specific
            ("SIMILAR TO", "Regex match", 80),      // Postgres specific
            ("IN", "List presence", 85),
            ("IS", "Identity check", 85),
            ("IS NULL", "Null check", 85),
            ("IS NOT", "Negated identity", 85),
            ("BETWEEN", "Range check", 85),
            ("AND", "Logical AND", 80),
            ("OR", "Logical OR", 80),
            ("NOT", "Logical NOT", 80),
        ]
    }
    
    fn complete(
        &self,
        scope_tree: &sql_scope::ScopeTree,
        context: &Context,
        schema: &SchemaGraph,
        default_schema: Option<&str>,
        capabilities: Option<&DatabaseCapabilities>,
    ) -> Vec<CompletionItem> {
        let effective_schema = default_schema
            .map(|s| s.to_string())
            .or_else(|| capabilities.and_then(|c| c.default_schema.clone()))
            .unwrap_or_else(|| self.default_schema().to_string());

        match &context.context_type {
            CursorContext::AfterDot { alias } =>
                CoreCompletionEngine::complete_after_dot(alias, scope_tree, context, schema),
            CursorContext::SelectClause | CursorContext::AfterSelectList =>
                CoreCompletionEngine::complete_select_clause(scope_tree, context, schema, SELECT_KEYWORDS, SELECT_FUNCTIONS),
            CursorContext::RootContext => {
                let mut items = CoreCompletionEngine::complete_root_context(context);
                // Add PostgreSQL-specific root items
                self.add_postgres_root_items(&mut items, context);
                items
            }
            CursorContext::FromClause | CursorContext::JoinTable =>
                CoreCompletionEngine::complete_table_names(schema, scope_tree, context, Some(&effective_schema), FROM_KEYWORDS),
            CursorContext::JoinCondition { left_table, right_table } =>
                CoreCompletionEngine::complete_join_condition(left_table, right_table, scope_tree, schema, context.cursor_offset),
            CursorContext::JoinConditionRhs { .. } => {
                let operators = self.operators();
                CoreCompletionEngine::complete_where_clause(scope_tree, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS, &operators)
            }
            CursorContext::WhereClause => {
                let operators = self.operators();
                CoreCompletionEngine::complete_where_clause(scope_tree, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS, &operators)
            }
            CursorContext::FunctionArgument { function_name } =>
                CoreCompletionEngine::complete_function_argument(function_name, scope_tree, context, schema),
            CursorContext::Unknown =>
                CoreCompletionEngine::complete_generic(scope_tree, context, GENERIC_KEYWORDS),
        }
    }
}

impl PostgresEngine {
    /// Add PostgreSQL-specific root context items.
    fn add_postgres_root_items(&self, items: &mut Vec<CompletionItem>, context: &Context) {
        let postgres_starters = [
            ("EXPLAIN", "Explain query plan", 55),
            ("EXPLAIN ANALYZE", "Explain with execution stats", 54),
            ("VACUUM", "Reclaim storage", 40),
            ("ANALYZE", "Collect statistics", 40),
            ("COPY", "Copy data between table and file", 35),
        ];
        
        for (kw, detail, score) in postgres_starters {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: CompletionKind::Keyword,
                detail: Some(detail.to_string()),
                insert_text: kw.to_string(),
                score,
            });
        }
        
        CoreCompletionEngine::filter_by_prefix(items, &context.prefix);
        items.sort_by(|a, b| b.score.cmp(&a.score));
    }
}

#[cfg(test)]
mod postgres_scope_tree_tests {
    use super::*;
    use sql_scope::{ScopeTree, Source};
    use sql_scope::scope::tree::{Scope, ScopeType};
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

    #[test]
    fn postgres_complete_after_dot_returns_columns() {
        let engine = PostgresEngine::new();
        let schema = MockSchemaLoader::create_test_schema();
        let scope_tree = tree_with_alias("u", "users");
        let context = Context {
            cursor_offset: 20,
            context_type: CursorContext::AfterDot { alias: "u".to_string() },
            prefix: String::new(),
            previous_word: String::new(),
            scope_depth: 0,
        };
        let items = engine.complete(&scope_tree, &context, &schema, None, None);
        assert!(!items.is_empty(), "should return column completions for 'u' alias");
        assert!(items.iter().any(|i| i.label == "id"), "should contain 'id' column");
    }

    #[test]
    fn postgres_complete_from_clause_returns_tables() {
        let engine = PostgresEngine::new();
        let schema = MockSchemaLoader::create_test_schema();
        let scope_tree = ScopeTree::new();
        let context = Context {
            cursor_offset: 10,
            context_type: CursorContext::FromClause,
            prefix: String::new(),
            previous_word: String::new(),
            scope_depth: 0,
        };
        let items = engine.complete(&scope_tree, &context, &schema, None, None);
        assert!(!items.is_empty(), "should return table completions");
    }
}
