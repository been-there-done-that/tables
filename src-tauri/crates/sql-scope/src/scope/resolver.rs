use crate::ir::{ParsedStatement, SelectIr, TableRefIr};
use crate::schema::SchemaSnapshot;
use super::tree::{Scope, ScopeTree, ScopeType};
use super::symbol::{ColumnRef, ScopeId, Source};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Build a `ScopeTree` for the given parsed statement.
///
/// Only `ParsedStatement::Select` is handled; all other variants produce an
/// empty tree (no scopes registered).
pub fn traverse_scope(stmt: &ParsedStatement, schema: &dyn SchemaSnapshot) -> ScopeTree {
    let mut tree = ScopeTree::new();
    if let ParsedStatement::Select(sel) = stmt {
        build_select_scope(sel, None, &mut tree, schema);
    }
    tree
}

fn build_select_scope(
    sel: &SelectIr,
    parent_id: Option<ScopeId>,
    tree: &mut ScopeTree,
    schema: &dyn SchemaSnapshot,
) -> ScopeId {
    // Inherit cte_sources from parent scope
    let inherited_ctes: IndexMap<String, super::tree::CteInfo> = parent_id
        .map(|pid| tree.scope(pid).cte_sources.clone())
        .unwrap_or_default();

    let scope_id = tree.add_scope(Scope {
        id: 0, // placeholder — add_scope assigns the real id
        parent: parent_id,
        scope_type: ScopeType::Root,
        byte_range: sel.byte_range.clone(),
        sources: HashMap::new(),
        cte_sources: inherited_ctes,
        columns: Vec::new(),
    });

    // Process CTEs in order (if WITH clause present)
    if let Some(with_ir) = &sel.with {
        process_ctes(with_ir, scope_id, tree, schema);
    }

    // Register FROM sources
    for table_ref in &sel.body.from {
        register_table_ref(table_ref, scope_id, tree, schema);
    }

    scope_id
}

fn process_ctes(
    with_ir: &crate::ir::WithIr,
    parent_scope_id: ScopeId,
    tree: &mut ScopeTree,
    schema: &dyn SchemaSnapshot,
) {
    for cte_ir in &with_ir.ctes {
        // Child scope for CTE body inherits CTEs registered so far in parent
        let inherited = tree.scope(parent_scope_id).cte_sources.clone();

        let cte_scope_id = tree.add_scope(Scope {
            id: 0,
            parent: Some(parent_scope_id),
            scope_type: ScopeType::Cte { name: cte_ir.name.clone() },
            byte_range: cte_ir.byte_range.clone(),
            sources: HashMap::new(),
            cte_sources: inherited,
            columns: Vec::new(),
        });

        // For RECURSIVE: register self-reference in the CTE's own scope
        if with_ir.recursive || cte_ir.recursive {
            tree.scope_mut(cte_scope_id).sources.insert(
                cte_ir.name.clone(),
                Source::Cte { name: cte_ir.name.clone() },
            );
        }

        // Register CTE body's FROM sources into cte scope
        for tref in &cte_ir.body.from {
            register_table_ref(tref, cte_scope_id, tree, schema);
        }

        // Resolve CTE columns
        let columns = crate::scope::cte::resolve_cte_columns(cte_ir, cte_scope_id, tree, schema);

        // Store resolved columns in cte scope's `columns` field as ColumnRef
        tree.scope_mut(cte_scope_id).columns = columns
            .iter()
            .map(|c| ColumnRef { name: c.clone(), source_table: None, source_alias: None })
            .collect();

        // Register CteInfo in parent scope (now visible to subsequent CTEs)
        let cte_info = super::tree::CteInfo {
            scope_id: cte_scope_id,
            columns,
            is_recursive: with_ir.recursive || cte_ir.recursive,
        };
        tree.scope_mut(parent_scope_id).cte_sources.insert(cte_ir.name.clone(), cte_info);
    }
}

fn register_table_ref(
    tref: &TableRefIr,
    scope_id: ScopeId,
    tree: &mut ScopeTree,
    schema: &dyn SchemaSnapshot,
) {
    match tref {
        TableRefIr::Table { schema: tschema, name, alias, .. } => {
            let key = alias.clone().unwrap_or_else(|| name.clone());
            let source = if let Some(alias_str) = alias {
                Source::Alias {
                    alias: alias_str.clone(),
                    target: Box::new(Source::Table {
                        schema: tschema.clone(),
                        name: name.clone(),
                    }),
                }
            } else {
                Source::Table { schema: tschema.clone(), name: name.clone() }
            };
            tree.scope_mut(scope_id).sources.insert(key, source);
        }
        TableRefIr::Subquery { body, alias, byte_range } => {
            let sub_sel = SelectIr {
                with: None,
                body: *body.clone(),
                byte_range: byte_range.clone(),
            };
            let child_id = build_select_scope(&sub_sel, Some(scope_id), tree, schema);
            tree.scope_mut(child_id).scope_type =
                ScopeType::DerivedTable { alias: alias.clone() };
            // Register as Alias wrapping DerivedTable in parent scope
            let source = Source::Alias {
                alias: alias.clone(),
                target: Box::new(Source::DerivedTable { scope_id: child_id }),
            };
            tree.scope_mut(scope_id).sources.insert(alias.clone(), source);
        }
        TableRefIr::Join { left, right } => {
            register_table_ref(left, scope_id, tree, schema);
            register_table_ref(right, scope_id, tree, schema);
        }
        TableRefIr::WhereSubquery { body } => {
            // Build a child scope so outer aliases are visible inside the EXISTS/correlated subquery.
            let sub_sel = SelectIr {
                with: None,
                body: *body.clone(),
                byte_range: body.byte_range.clone(),
            };
            let child_id = build_select_scope(&sub_sel, Some(scope_id), tree, schema);
            let _ = child_id;
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build a fake SelectBodyIr wrapping a SelectBodyIr for subquery
// ---------------------------------------------------------------------------
// (Not needed — the Subquery arm above constructs a SelectIr inline)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::postgres::parse_postgres;
    use crate::schema::{ForeignKey, SchemaSnapshot, SqlType};
    use std::collections::HashMap;

    struct Mock {
        tables: HashMap<String, Vec<String>>,
    }

    impl SchemaSnapshot for Mock {
        fn table_exists(&self, _: Option<&str>, t: &str) -> bool {
            self.tables.contains_key(t)
        }
        fn table_columns(&self, _: Option<&str>, t: &str) -> Option<Vec<String>> {
            self.tables.get(t).cloned()
        }
        fn column_type(&self, _: Option<&str>, _: &str, _: &str) -> Option<SqlType> {
            None
        }
        fn foreign_keys(&self, _: Option<&str>, _: &str) -> Vec<ForeignKey> {
            vec![]
        }
        fn default_schema(&self) -> Option<&str> {
            Some("public")
        }
    }

    fn mock(tables: &[(&str, &[&str])]) -> Mock {
        Mock {
            tables: tables
                .iter()
                .map(|(t, cols)| {
                    (t.to_string(), cols.iter().map(|c| c.to_string()).collect())
                })
                .collect(),
        }
    }

    #[test]
    fn simple_from_registers_table() {
        let stmt = parse_postgres("SELECT id FROM users").unwrap();
        let schema = mock(&[("users", &["id", "name"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(5);
        assert!(vis.sources.iter().any(|(a, _)| a == "users"));
    }

    #[test]
    fn alias_is_used_as_key() {
        let stmt = parse_postgres("SELECT u.id FROM users u").unwrap();
        let schema = mock(&[("users", &["id"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(5);
        assert!(vis.sources.iter().any(|(a, _)| a == "u"));
        assert!(!vis.sources.iter().any(|(a, _)| a == "users"));
    }

    #[test]
    fn join_registers_both_tables() {
        let sql = "SELECT * FROM orders o JOIN users u ON o.user_id = u.id";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[("orders", &["id", "user_id"]), ("users", &["id"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(10);
        let aliases: Vec<&str> = vis.sources.iter().map(|(a, _)| a.as_str()).collect();
        assert!(aliases.contains(&"o"));
        assert!(aliases.contains(&"u"));
    }

    #[test]
    fn cte_visible_in_main_query() {
        let sql = "WITH active AS (SELECT id FROM users) SELECT * FROM active";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[("users", &["id"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(sql.len() - 5);
        assert!(vis.sources.iter().any(|(a, _)| a == "active"));
    }

    #[test]
    fn cte_b_can_reference_cte_a() {
        let sql =
            "WITH a AS (SELECT id FROM users), b AS (SELECT id FROM a) SELECT * FROM b";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[("users", &["id"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(sql.len() - 5);
        assert!(vis.sources.iter().any(|(a, _)| a == "b"));
    }

    #[test]
    fn cte_explicit_columns_stored() {
        let sql = "WITH cte(x, y) AS (SELECT 1, 2) SELECT x FROM cte";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[]);
        let tree = traverse_scope(&stmt, &schema);
        // The root scope (id=0) holds all registered CTE infos.
        // Parser assigns byte_range 0..sql.len() to every node, so multiple
        // scopes overlap; find the root by looking at id=0 directly.
        let root = tree.scope(0);
        let cte_info = root.cte_sources.get("cte").unwrap();
        assert_eq!(cte_info.columns, vec!["x", "y"]);
    }

    #[test]
    fn recursive_cte_self_reference_visible() {
        let sql = "WITH RECURSIVE nums(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM nums WHERE n < 10) SELECT * FROM nums";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[]);
        let tree = traverse_scope(&stmt, &schema);
        // nums should be visible in main query
        let vis = tree.visible_at(sql.len() - 5);
        assert!(vis.sources.iter().any(|(a, _)| a == "nums"));
    }

    #[test]
    fn derived_table_registers_as_source() {
        let sql = "SELECT t.id FROM (SELECT id FROM users) t";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[("users", &["id"])]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(10);
        assert!(vis.sources.iter().any(|(a, _)| a == "t"));
    }

    #[test]
    fn non_select_stmt_produces_empty_tree() {
        let sql = "DELETE FROM users WHERE id = 1";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[]);
        let tree = traverse_scope(&stmt, &schema);
        // No scopes registered for dangerous statements
        assert!(tree.scope_at(5).is_none());
    }

    #[test]
    fn multiple_ctes_visibility_order() {
        // cte_b references cte_a — both should be visible at end
        let sql =
            "WITH cte_a AS (SELECT 1 AS val), cte_b AS (SELECT val FROM cte_a) SELECT * FROM cte_b";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[]);
        let tree = traverse_scope(&stmt, &schema);
        let vis = tree.visible_at(sql.len() - 3);
        let names: Vec<&str> = vis.sources.iter().map(|(a, _)| a.as_str()).collect();
        assert!(names.contains(&"cte_a"));
        assert!(names.contains(&"cte_b"));
    }

    #[test]
    fn wildcard_through_cte_expands_from_schema() {
        // CTE uses SELECT * from a real table, main query selects from CTE
        // resolve_cte_columns should expand * to the table's columns
        let sql = "WITH cte AS (SELECT * FROM users) SELECT * FROM cte";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[("users", &["id", "name", "email"])]);
        let tree = traverse_scope(&stmt, &schema);
        // The CTE scope should have resolved columns from users table
        let root = tree.all_scopes().iter()
            .find(|s| s.parent.is_none())
            .expect("root scope not found");
        let cte_info = root.cte_sources.get("cte").expect("cte not found");
        assert_eq!(cte_info.columns, vec!["id", "name", "email"],
            "CTE SELECT * should expand to table columns");
    }

    #[test]
    fn cte_a_cannot_see_cte_b_declared_after() {
        // CTE `a` is declared before `b`; a's child scope must NOT contain `b` in cte_sources
        let sql = "WITH a AS (SELECT 1 AS x), b AS (SELECT 2 AS y) SELECT * FROM a";
        let stmt = parse_postgres(sql).unwrap();
        let schema = mock(&[]);
        let tree = traverse_scope(&stmt, &schema);
        // Find the CTE scope for "a" — its cte_sources must not contain "b"
        let cte_a_scope = tree.all_scopes()
            .iter()
            .find(|s| matches!(&s.scope_type, ScopeType::Cte { name } if name == "a"))
            .expect("CTE scope 'a' not found");
        assert!(
            !cte_a_scope.cte_sources.contains_key("b"),
            "CTE 'a' should not be able to see CTE 'b' which is declared after it"
        );
    }
}
