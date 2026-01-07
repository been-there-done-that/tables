//! PostgreSQL Completion Engine
//!
//! PostgreSQL-specific keywords, functions, and completion behaviors.
//! Uses the shared core logic from `core.rs`.

use crate::adapter::DatabaseCapabilities;
use crate::completion::analysis::SemanticModel;
use crate::completion::context::{Context, CursorContext};
use crate::completion::document::Dialect;
use crate::completion::engine::{CompletionItem, CompletionKind};
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
    
    fn complete(
        &self,
        semantic: &SemanticModel,
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
            CursorContext::AfterDot { alias } => {
                CoreCompletionEngine::complete_after_dot(alias, semantic, context, schema)
            }
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                CoreCompletionEngine::complete_select_clause(
                    semantic, context, schema, SELECT_KEYWORDS, SELECT_FUNCTIONS
                )
            }
            CursorContext::RootContext => {
                let mut items = CoreCompletionEngine::complete_root_context(context);
                // Add PostgreSQL-specific root items
                self.add_postgres_root_items(&mut items, context);
                items
            }
            CursorContext::FromClause | CursorContext::JoinTable => {
                CoreCompletionEngine::complete_table_names(
                    schema, semantic, context, Some(&effective_schema), FROM_KEYWORDS
                )
            }
            CursorContext::JoinCondition { left_table, right_table } => {
                CoreCompletionEngine::complete_join_condition(left_table, right_table, semantic, schema)
            }
            CursorContext::JoinConditionRhs { .. } => {
                CoreCompletionEngine::complete_where_clause(
                    semantic, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS
                )
            }
            CursorContext::WhereClause => {
                CoreCompletionEngine::complete_where_clause(
                    semantic, context, schema, WHERE_KEYWORDS, WHERE_FUNCTIONS
                )
            }
            CursorContext::FunctionArgument { function_name } => {
                CoreCompletionEngine::complete_function_argument(function_name, semantic, context, schema)
            }
            CursorContext::Unknown => {
                CoreCompletionEngine::complete_generic(semantic, context, GENERIC_KEYWORDS)
            }
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
