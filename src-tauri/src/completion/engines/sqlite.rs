//! SQLite Completion Engine
//!
//! SQLite-specific keywords, functions, and completion behaviors.
//! Uses the shared core logic from `core.rs`.

use crate::adapter::DatabaseCapabilities;
use crate::completion::context::{Context, CursorContext};
use crate::completion::document::Dialect;
use crate::completion::engine::{CompletionItem, CompletionKind};
use crate::completion::schema::SchemaGraph;
use super::core::CoreCompletionEngine;
use super::CompletionEngineVariant;

/// SQLite-specific completion engine.
pub struct SqliteEngine;

impl SqliteEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SqliteEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SQLite-Specific Keywords and Functions
// ============================================================================

/// Keywords valid in SELECT clause (SQLite)
const SELECT_KEYWORDS: &[&str] = &[
    "DISTINCT", "AS", "CASE", "WHEN", "THEN", "ELSE", "END", "FROM",
];

/// Functions valid in SELECT clause (SQLite)
const SELECT_FUNCTIONS: &[&str] = &[
    // Aggregate functions
    "COUNT", "SUM", "AVG", "MAX", "MIN", "ABS", "ROUND", 
    "TOTAL", // SQLite-specific (similar to SUM but returns 0.0 for no rows)
    // String functions
    "UPPER", "LOWER", "LENGTH", "TRIM", "SUBSTR", "REPLACE", 
    "INSTR", "PRINTF", "LTRIM", "RTRIM",
    // SQLite-specific string
    "GLOB", "LIKE", "UNICODE", "ZEROBLOB",
    // Null handling
    "COALESCE", "NULLIF", "CAST", "IFNULL", // IFNULL is SQLite-specific
    "IIF", // SQLite inline-if
    // Date/time
    "DATE", "TIME", "DATETIME", "JULIANDAY", "STRFTIME",
    // SQLite-specific aggregate
    "GROUP_CONCAT", // SQLite equivalent of STRING_AGG
    // SQLite-specific functions
    "TYPEOF", "SQLITE_VERSION", "LAST_INSERT_ROWID", "CHANGES", "TOTAL_CHANGES",
    "RANDOMBLOB", "HEX", "UNHEX", "QUOTE", "SOUNDEX",
    // SQLite JSON functions
    "JSON", "JSON_EXTRACT", "JSON_INSERT", "JSON_REPLACE", "JSON_SET",
    "JSON_REMOVE", "JSON_TYPE", "JSON_VALID", "JSON_QUOTE",
    "JSON_ARRAY", "JSON_OBJECT", "JSON_GROUP_ARRAY", "JSON_GROUP_OBJECT",
    "JSON_EACH", "JSON_TREE",
    // Math functions (SQLite 3.35+)
    "CEIL", "FLOOR", "TRUNC", "EXP", "LN", "LOG", "LOG10", "LOG2",
    "POWER", "SQRT", "MOD", "PI", "ACOS", "ASIN", "ATAN", "COS", "SIN", "TAN",
];

/// Keywords valid in FROM clause (SQLite)
const FROM_KEYWORDS: &[&str] = &[
    "JOIN", "INNER JOIN", "LEFT JOIN", "LEFT OUTER JOIN", "CROSS JOIN",
    // SQLite doesn't support RIGHT JOIN or FULL JOIN
    "NATURAL JOIN", "NATURAL LEFT JOIN",
    "ON", "USING", "WHERE", "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET",
    "INDEXED BY", "NOT INDEXED", // SQLite-specific
];

/// Keywords valid in WHERE clause (SQLite)
const WHERE_KEYWORDS: &[&str] = &[
    "AND", "OR", "NOT", "IN", "IS", "NULL", "LIKE", "GLOB", // GLOB is SQLite-specific
    "BETWEEN", "EXISTS",
    "REGEXP", // SQLite-specific (requires extension)
    "ORDER BY", "GROUP BY", "HAVING", "LIMIT",
];

/// Functions valid in WHERE clause (SQLite)
const WHERE_FUNCTIONS: &[&str] = &[
    "COALESCE", "NULLIF", "CAST", "UPPER", "LOWER", "LENGTH", "TRIM",
    "DATE", "TIME", "DATETIME",
    // SQLite-specific
    "TYPEOF", "IFNULL", "IIF", "INSTR",
];

/// Keywords for generic/unknown context (SQLite)
const GENERIC_KEYWORDS: &[&str] = &[
    "SELECT", "FROM", "WHERE", "JOIN", "ON", "AND", "OR", "ORDER BY", "GROUP BY",
    "WITH", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
    // SQLite-specific
    "PRAGMA", "ATTACH", "DETACH", "VACUUM", "REINDEX",
    "EXPLAIN", "EXPLAIN QUERY PLAN",
    "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "RELEASE",
    "REPLACE", "INSERT OR REPLACE", "INSERT OR IGNORE",
];

impl CompletionEngineVariant for SqliteEngine {
    fn dialect(&self) -> Dialect {
        Dialect::SQLite
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
        "main"  // SQLite uses "main" as the default schema
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
            ("GLOB", "Unix pattern match", 85), // SQLite specific
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
            CursorContext::SelectClause | CursorContext::AfterSelectList => {
                let mut items = CoreCompletionEngine::complete_select_clause(
                    scope_tree, context, schema, SELECT_KEYWORDS, SELECT_FUNCTIONS
                );
                // Add SQLite-specific ROWID suggestion
                self.add_rowid_suggestion(&mut items, context);
                items
            }
            CursorContext::RootContext => {
                let mut items = CoreCompletionEngine::complete_root_context(context);
                // Add SQLite-specific root items
                self.add_sqlite_root_items(&mut items, context);
                items
            }
            CursorContext::FromClause | CursorContext::JoinTable =>
                CoreCompletionEngine::complete_table_names(schema, scope_tree, context, Some(&effective_schema), FROM_KEYWORDS),
            CursorContext::JoinCondition { left_table, right_table } =>
                CoreCompletionEngine::complete_join_condition(left_table, right_table, scope_tree, schema),
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

impl SqliteEngine {
    /// Add SQLite-specific root context items.
    fn add_sqlite_root_items(&self, items: &mut Vec<CompletionItem>, context: &Context) {
        let sqlite_starters = [
            ("PRAGMA", "SQLite pragma statement", 65),
            ("ATTACH", "Attach database file", 50),
            ("DETACH", "Detach database", 45),
            ("VACUUM", "Rebuild database file", 40),
            ("REINDEX", "Rebuild indexes", 35),
            ("EXPLAIN QUERY PLAN", "Show query plan", 55),
        ];
        
        for (kw, detail, score) in sqlite_starters {
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
    
    /// Add ROWID as a column suggestion for SELECT contexts.
    fn add_rowid_suggestion(&self, items: &mut Vec<CompletionItem>, context: &Context) {
        // ROWID is available on all SQLite tables (unless WITHOUT ROWID)
        items.push(CompletionItem {
            label: "rowid".to_string(),
            kind: CompletionKind::Column,
            detail: Some("SQLite implicit row ID".to_string()),
            insert_text: "rowid".to_string(),
            score: 45, // Lower than regular columns
        });
        
        // Also add _rowid_ and oid as aliases
        items.push(CompletionItem {
            label: "_rowid_".to_string(),
            kind: CompletionKind::Column,
            detail: Some("Alias for rowid".to_string()),
            insert_text: "_rowid_".to_string(),
            score: 40,
        });
        
        CoreCompletionEngine::filter_by_prefix(items, &context.prefix);
    }
}
