use crate::scope::tree::{DiagSeverity, ScopeDiagnostic, ScopeTree};
use crate::scope::symbol::Source;
use crate::schema::SchemaSnapshot;

/// Run diagnostic checks over a resolved ScopeTree.
/// Checks:
/// 1. Unknown table references (Warning) — table not in schema and not a CTE
/// 2. Unknown CTE reference (Error) — CTE name used but not in cte_sources
///    (this would be caught by traverse_scope producing no source entry, but
///    a future resolver pass can emit these)
pub fn run_diagnostics(tree: &ScopeTree, schema: &dyn SchemaSnapshot, _sql: &str) -> Vec<ScopeDiagnostic> {
    let mut diags = Vec::new();

    for scope in tree.all_scopes() {
        for source in scope.sources.values() {
            // Unwrap Alias to get the underlying source
            let inner = unwrap_source(source);
            if let Source::Table { schema: tschema, name } = inner {
                if !schema.table_exists(tschema.as_deref(), name)
                    && !scope.cte_sources.contains_key(name.as_str())
                {
                    diags.push(ScopeDiagnostic {
                        message: format!("Unknown table '{}'", name),
                        severity: DiagSeverity::Warning,
                        byte_range: 0..1,
                    });
                }
            }
        }
    }

    diags
}

fn unwrap_source(source: &Source) -> &Source {
    match source {
        Source::Alias { target, .. } => unwrap_source(target),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::postgres::parse_postgres;
    use crate::scope::resolver::traverse_scope;
    use crate::schema::{ForeignKey, SchemaSnapshot, SqlType};
    use std::collections::HashMap;

    struct Mock(HashMap<String, Vec<String>>);
    impl SchemaSnapshot for Mock {
        fn table_exists(&self, _: Option<&str>, t: &str) -> bool { self.0.contains_key(t) }
        fn table_columns(&self, _: Option<&str>, t: &str) -> Option<Vec<String>> { self.0.get(t).cloned() }
        fn column_type(&self, _: Option<&str>, _: &str, _: &str) -> Option<SqlType> { None }
        fn foreign_keys(&self, _: Option<&str>, _: &str) -> Vec<ForeignKey> { vec![] }
        fn default_schema(&self) -> Option<&str> { Some("public") }
    }

    fn schema_with(tables: &[&str]) -> Mock {
        Mock(tables.iter().map(|t| (t.to_string(), vec![])).collect())
    }

    #[test]
    fn unknown_table_warns() {
        let sql = "SELECT * FROM nonexistent";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&["users"]);
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        assert!(diags.iter().any(|d| d.message.contains("nonexistent") && d.severity == DiagSeverity::Warning),
            "expected warning for nonexistent, got {:?}", diags);
    }

    #[test]
    fn known_table_no_warning() {
        let sql = "SELECT * FROM users";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&["users"]);
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
    }

    #[test]
    fn cte_reference_no_warning() {
        let sql = "WITH active AS (SELECT id FROM users) SELECT * FROM active";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&["users"]);
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        // "active" is a CTE — should not warn about it
        assert!(!diags.iter().any(|d| d.message.contains("active")),
            "CTE reference 'active' should not produce a warning");
    }

    #[test]
    fn aliased_known_table_no_warning() {
        let sql = "SELECT u.id FROM users u";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&["users"]);
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        assert!(diags.is_empty(), "aliased known table should not warn, got {:?}", diags);
    }

    #[test]
    fn multiple_unknown_tables_all_warn() {
        let sql = "SELECT * FROM foo f JOIN bar b ON f.id = b.foo_id";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&["users"]); // neither foo nor bar in schema
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        let msgs: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("foo")), "expected warning for 'foo'");
        assert!(msgs.iter().any(|m| m.contains("bar")), "expected warning for 'bar'");
    }

    #[test]
    fn cte_body_references_real_table_warns_if_unknown() {
        let sql = "WITH cte AS (SELECT * FROM nonexistent) SELECT * FROM cte";
        let stmt = parse_postgres(sql).unwrap();
        let schema = schema_with(&[]); // no tables
        let tree = traverse_scope(&stmt, &schema);
        let diags = run_diagnostics(&tree, &schema, sql);
        assert!(diags.iter().any(|d| d.message.contains("nonexistent")),
            "expected warning for 'nonexistent' used inside CTE body");
    }
}
