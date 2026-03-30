use tree_sitter::Tree;
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
    pub fn check(_tree: &Tree, source: &str, schema: &SchemaGraph) -> Vec<Diagnostic> {
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
                        Some("Destructive: DROP cannot be undone".to_string()),
                    sql_scope::DangerousKind::Truncate =>
                        Some("Destructive: TRUNCATE will delete all rows".to_string()),
                    sql_scope::DangerousKind::DeleteWithoutWhere if !has_where =>
                        Some("DELETE without WHERE will erase every row".to_string()),
                    sql_scope::DangerousKind::UpdateWithoutWhere if !has_where =>
                        Some("UPDATE without WHERE will modify every row".to_string()),
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
    fn clean_pg_error(msg: &str) -> &str {
        msg.split(" (position:").next().unwrap_or(msg).trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::parsing::parse_sql;
    use crate::completion::schema::graph::{SchemaGraph, TableInfo};

    #[test]
    fn test_syntax_error() {
        let sql = "SELECT * FROM ;"; // Missing table
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);

        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_unknown_table() {
        let sql = "SELECT * FROM non_existent_table;";
        let tree = parse_sql(sql, None).unwrap();

        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("users", "public", vec![]));

        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);

        assert!(diagnostics.iter().any(|d| d.message.contains("Unknown table")));
    }

    #[test]
    fn test_valid_sql_with_semicolon() {
        let sql = "SELECT * FROM albums;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);

        // Assert no syntax errors for valid SQL when no schema
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in valid SQL: {}", d.message);
            }
        }
    }

    #[test]
    fn test_multiple_statements_with_semicolons() {
        let sql = "SELECT 1; SELECT 2;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);

        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error in multi-statement SQL: {}", d.message);
            }
        }
    }

    #[test]
    fn test_delete_without_where() {
        let sql = "DELETE FROM orders;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("DELETE without WHERE")));
    }

    #[test]
    fn test_delete_with_where() {
        let sql = "DELETE FROM orders WHERE id = 1;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(!diagnostics.iter().any(|d| d.message.contains("DELETE without WHERE")));
    }

    #[test]
    fn test_update_without_where() {
        let sql = "UPDATE users SET active = false;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("UPDATE without WHERE")));
    }

    #[test]
    fn test_drop_table() {
        let sql = "DROP TABLE IF EXISTS users;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("DROP cannot be undone")));
    }

    #[test]
    fn test_truncate() {
        let sql = "TRUNCATE TABLE logs;";
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(diagnostics.iter().any(|d| d.message.contains("TRUNCATE will delete all rows")));
    }

    #[test]
    fn test_cte_table_unknown() {
        let sql = r#"
        with apples as (
            select * from production.tasks t where t.id is not null
        ) select * FROM apples;
        "#;
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        // No errors — CTE is valid even without schema (schema empty → no semantic checks)
        for d in &diagnostics {
            if d.severity == 1 {
                panic!("Unexpected syntax error: {}", d.message);
            }
        }
    }

    #[test]
    fn test_analyze_known_table_no_diagnostic() {
        let sql = "ANALYZE production.tasks;";
        let tree = parse_sql(sql, None).unwrap();
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        let errors_or_warnings: Vec<_> = diagnostics.iter()
            .filter(|d| d.severity <= 2)
            .collect();
        assert!(
            errors_or_warnings.is_empty(),
            "ANALYZE with known table should produce no diagnostics, got {:?}", errors_or_warnings
        );
    }

    #[test]
    fn test_analyze_unknown_table_warns() {
        let sql = "ANALYZE production.tasksj;";
        let tree = parse_sql(sql, None).unwrap();
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(
            diagnostics.iter().any(|d| d.message.contains("tasksj")),
            "should warn about unknown table 'tasksj', got {:?}", diagnostics
        );
    }

    #[test]
    fn test_pg_query_error_has_message() {
        let sql = "SELECT * FROM ;"; // actual parse error
        let tree = parse_sql(sql, None).unwrap();
        let schema = SchemaGraph::new();
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        assert!(
            diagnostics.iter().any(|d| d.severity == 1),
            "invalid SQL should produce a severity=1 error, got {:?}", diagnostics
        );
    }

    #[test]
    fn test_multi_statement_mixed() {
        let sql = "ANALYZE production.tasks; SELECT * FROM production.tasks;";
        let tree = parse_sql(sql, None).unwrap();
        let mut schema = SchemaGraph::new();
        schema.add_table(TableInfo::new("tasks", "production", vec![]));
        let diagnostics = DiagnosticEngine::check(&tree, sql, &schema);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity == 1).collect();
        assert!(errors.is_empty(), "valid multi-statement SQL should have no errors, got {:?}", errors);
    }
}
