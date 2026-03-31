use crate::completion::schema::graph::SchemaGraph;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub message: String,
    pub start: usize, // byte offset
    pub end: usize,
    pub severity: u8, // 1=Error, 2=Warning
}

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    pub fn check(source: &str, schema: &SchemaGraph) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only run semantic diagnostics if schema is populated.
        // Avoids squiggling everything during initialization or while disconnected.
        let schema_populated = !schema.tables.is_empty();

        for (offset, stmt_str) in sql_scope::split_statements(source) {
            if stmt_str.trim().is_empty() {
                continue;
            }

            // pg_query is the primary validator via sql_scope
            match sql_scope::resolve(stmt_str, sql_scope::Dialect::Postgres, schema) {
                Ok(scope_tree) => {
                    // Parse succeeded — run semantic checks
                    if schema_populated {
                        let scope_diags = sql_scope::run_diagnostics(&scope_tree, schema, stmt_str);
                        for sd in scope_diags {
                            diagnostics.push(Diagnostic {
                                message: sd.message,
                                start: offset + sd.byte_range.start,
                                end: offset + sd.byte_range.end,
                                severity: match sd.severity {
                                    sql_scope::DiagSeverity::Error => 1,
                                    sql_scope::DiagSeverity::Warning => 2,
                                    sql_scope::DiagSeverity::Info => 3,
                                },
                            });
                        }
                    }

                    // Dangerous statement warnings (DROP/TRUNCATE/DELETE without WHERE/UPDATE without WHERE)
                    Self::check_dangerous(stmt_str, offset, &mut diagnostics);
                }
                Err(sql_scope::ScopeError::Parse(msg)) => {
                    // pg_query parse error — emit with position from error message
                    let (start, end) = Self::extract_error_position(&msg, stmt_str, offset);
                    diagnostics.push(Diagnostic {
                        message: format!("Syntax error: {}", Self::clean_pg_error(&msg)),

                        start,
                        end,
                        severity: 1,
                    });
                }
                Err(_) => {
                    // Other errors (shouldn't happen in practice)
                }
            }
        }

        diagnostics
    }

    /// Emit dangerous statement warnings by re-parsing with parse_postgres_stmt.
    /// This is fast (pure C pg_query parser) and avoids duplicating classification logic.
    fn check_dangerous(stmt_str: &str, offset: usize, diagnostics: &mut Vec<Diagnostic>) {
        use sql_scope::ParsedStatement;
        if let Some(stmt) = sql_scope::parse_postgres_stmt(stmt_str) {
            if let ParsedStatement::Dangerous { kind, has_where } = stmt {
                let message = match kind {
                    sql_scope::DangerousKind::Drop =>
                        Some("DROP will permanently delete the object and cannot be undone".to_string()),
                    sql_scope::DangerousKind::Truncate =>
                        Some("TRUNCATE will permanently delete all rows in the table".to_string()),
                    sql_scope::DangerousKind::DeleteWithoutWhere if !has_where =>
                        Some("DELETE has no WHERE clause — every row in the table will be deleted".to_string()),
                    sql_scope::DangerousKind::UpdateWithoutWhere if !has_where =>
                        Some("UPDATE has no WHERE clause — every row in the table will be modified".to_string()),
                    _ => None, // has WHERE — not dangerous
                };
                if let Some(msg) = message {
                    diagnostics.push(Diagnostic {
                        message: msg,
                        start: offset,
                        end: offset + stmt_str.len(),
                        severity: 2,
                    });
                }
            }
        }
    }

    /// Extract byte position from a pg_query error message.
    /// pg_query embeds "position: N" (1-based character position).
    fn extract_error_position(msg: &str, stmt_str: &str, offset: usize) -> (usize, usize) {
        if let Some(pos_str) = msg.split("position: ").nth(1) {
            if let Ok(char_pos) = pos_str.trim().trim_end_matches(')').trim().parse::<usize>() {
                let byte_pos = char_pos.saturating_sub(1).min(stmt_str.len());
                let start = offset + byte_pos;
                let end = (start + 1).min(offset + stmt_str.len());
                return (start, end);
            }
        }
        // Fallback: highlight the whole statement
        (offset, offset + stmt_str.len())
    }

    /// Clean up pg_query error message for user display.
    /// pg_query messages look like: "syntax error at or near \"foo\" (position: 42)"
    /// We strip the position suffix and the redundant "syntax error " prefix
    /// since the caller already labels it as a syntax error.
    fn clean_pg_error(msg: &str) -> String {
        let stripped = msg.split(" (position:").next().unwrap_or(msg).trim();
        // pg_query always prefixes with lowercase "syntax error " — remove it to avoid
        // "Syntax error: syntax error at or near ..." in the UI
        let without_prefix = stripped
            .strip_prefix("syntax error ")
            .unwrap_or(stripped);
        // Capitalize first letter
        let mut chars = without_prefix.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::schema::graph::{SchemaGraph, TableInfo};

    fn check(sql: &str, schema: &SchemaGraph) -> Vec<Diagnostic> {
        DiagnosticEngine::check(sql, schema)
    }

    #[test]
    fn test_syntax_error() {
        let diagnostics = check("SELECT * FROM ;", &SchemaGraph::new());
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_unknown_table() {
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("users", "public", vec![]));
        let diagnostics = check("SELECT * FROM non_existent_table;", &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("does not exist in the schema")));
    }

    #[test]
    fn test_valid_sql_with_semicolon() {
        let diagnostics = check("SELECT * FROM albums;", &SchemaGraph::new());
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in valid SQL: {}", d.message);
            }
        }
    }

    #[test]
    fn test_multiple_statements_with_semicolons() {
        let diagnostics = check("SELECT 1; SELECT 2;", &SchemaGraph::new());
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in multi-statement SQL: {}", d.message);
            }
        }
    }

    #[test]
    fn test_delete_without_where() {
        let diagnostics = check("DELETE FROM orders;", &SchemaGraph::new());
        assert!(diagnostics.iter().any(|d| d.message.contains("DELETE has no WHERE clause")));
    }

    #[test]
    fn test_delete_with_where() {
        let diagnostics = check("DELETE FROM orders WHERE id = 1;", &SchemaGraph::new());
        assert!(!diagnostics.iter().any(|d| d.message.contains("DELETE has no WHERE clause")));
    }

    #[test]
    fn test_update_without_where() {
        let diagnostics = check("UPDATE users SET active = false;", &SchemaGraph::new());
        assert!(diagnostics.iter().any(|d| d.message.contains("UPDATE has no WHERE clause")));
    }

    #[test]
    fn test_drop_table() {
        let diagnostics = check("DROP TABLE IF EXISTS users;", &SchemaGraph::new());
        assert!(diagnostics.iter().any(|d| d.message.contains("DROP will permanently delete")));
    }

    #[test]
    fn test_truncate() {
        let diagnostics = check("TRUNCATE TABLE logs;", &SchemaGraph::new());
        assert!(diagnostics.iter().any(|d| d.message.contains("TRUNCATE will permanently delete")));
    }

    #[test]
    fn test_cte_table_unknown() {
        let sql = "with apples as (select * from production.tasks t where t.id is not null) select * FROM apples;";
        let diagnostics = check(sql, &SchemaGraph::new());
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error: {}", d.message);
            }
        }
    }

    #[test]
    fn test_analyze_known_table_no_diagnostic() {
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = check("ANALYZE production.tasks;", &schema);
        let errors_or_warnings: Vec<_> = diagnostics.iter().filter(|d| d.severity <= 2).collect();
        assert!(
            errors_or_warnings.is_empty(),
            "ANALYZE with known table should produce no diagnostics, got {:?}", errors_or_warnings
        );
    }

    #[test]
    fn test_analyze_unknown_table_warns() {
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = check("ANALYZE production.tasksj;", &schema);
        assert!(
            diagnostics.iter().any(|d| d.message.contains("tasksj")),
            "should warn about unknown table 'tasksj', got {:?}", diagnostics
        );
    }

    #[test]
    fn test_pg_query_error_has_message() {
        let diagnostics = check("SELECT * FROM ;", &SchemaGraph::new());
        assert!(
            diagnostics.iter().any(|d| d.severity == 1),
            "invalid SQL should produce a severity=1 error, got {:?}", diagnostics
        );
    }

    #[test]
    fn test_multi_statement_mixed() {
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = check("ANALYZE production.tasks; SELECT * FROM production.tasks;", &schema);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == 1).collect();
        assert!(errors.is_empty(), "valid multi-statement SQL should have no errors, got {:?}", errors);
    }
}
