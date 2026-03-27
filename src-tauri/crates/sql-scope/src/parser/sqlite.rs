use sqlparser::ast::{
    Cte, ObjectNamePart, Query, Select, SelectItem, SelectItemQualifiedWildcardKind, SetExpr,
    Statement, TableFactor, TableWithJoins, With,
};
use sqlparser::dialect::{Dialect, SQLiteDialect};
use sqlparser::parser::Parser;

use crate::ir::{
    CteIr, DangerousKind, ParsedStatement, SelectBodyIr, SelectIr, SelectItemIr, TableRefIr,
    WithIr,
};

/// Parse a single complete SQLite statement into IR.
/// Returns None if the statement is incomplete or invalid.
pub fn parse_sqlite(sql: &str) -> Option<ParsedStatement> {
    parse_with_dialect(sql, &SQLiteDialect {})
}

/// Shared implementation used by both the SQLite and MySQL backends.
pub(crate) fn parse_with_dialect(sql: &str, dialect: &dyn Dialect) -> Option<ParsedStatement> {
    if sql.trim().is_empty() {
        return None;
    }
    let stmts = Parser::parse_sql(dialect, sql).ok()?;
    let stmt = stmts.into_iter().next()?;
    convert_statement(stmt, sql)
}

fn convert_statement(stmt: Statement, sql: &str) -> Option<ParsedStatement> {
    match stmt {
        Statement::Query(q) => Some(ParsedStatement::Select(convert_query(*q, sql))),
        Statement::Delete(del) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: del.selection.is_some(),
        }),
        Statement::Update(update) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: update.selection.is_some(),
        }),
        Statement::Truncate { .. } => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Truncate,
            has_where: false,
        }),
        Statement::Drop { .. } => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        }),
        _ => Some(ParsedStatement::Other),
    }
}

fn convert_query(q: Query, sql: &str) -> SelectIr {
    SelectIr {
        with: q.with.map(|w| convert_with(w, sql)),
        body: convert_set_expr(*q.body, sql),
        byte_range: 0..sql.len(),
    }
}

fn convert_with(w: With, sql: &str) -> WithIr {
    WithIr {
        recursive: w.recursive,
        ctes: w
            .cte_tables
            .into_iter()
            .map(|cte| convert_cte(cte, sql))
            .collect(),
    }
}

fn convert_cte(cte: Cte, sql: &str) -> CteIr {
    // TableAlias.columns is Vec<TableAliasColumnDef> in sqlparser 0.61
    let explicit_columns: Vec<String> = cte
        .alias
        .columns
        .into_iter()
        .map(|c| c.name.value.to_lowercase())
        .collect();

    CteIr {
        name: cte.alias.name.value.to_lowercase(),
        explicit_columns,
        recursive: false, // sqlparser-rs sets recursive at the With level
        body: Box::new(convert_query(*cte.query, sql).body),
        byte_range: 0..sql.len(),
    }
}

fn convert_set_expr(expr: SetExpr, sql: &str) -> SelectBodyIr {
    match expr {
        SetExpr::Select(sel) => convert_select(*sel, sql),
        SetExpr::Query(q) => convert_query(*q, sql).body,
        // UNION/INTERSECT — use left branch for scope
        SetExpr::SetOperation { left, .. } => convert_set_expr(*left, sql),
        _ => SelectBodyIr {
            from: vec![],
            select_list: vec![],
            byte_range: 0..sql.len(),
        },
    }
}

fn convert_select(sel: Select, sql: &str) -> SelectBodyIr {
    let from: Vec<TableRefIr> = sel
        .from
        .into_iter()
        .flat_map(|twj| convert_table_with_joins(twj, sql))
        .collect();
    let select_list: Vec<SelectItemIr> = sel
        .projection
        .into_iter()
        .map(convert_select_item)
        .collect();
    SelectBodyIr {
        from,
        select_list,
        byte_range: 0..sql.len(),
    }
}

fn convert_table_with_joins(twj: TableWithJoins, sql: &str) -> Vec<TableRefIr> {
    let mut refs = vec![convert_table_factor(twj.relation, sql)];
    for join in twj.joins {
        refs.push(convert_table_factor(join.relation, sql));
    }
    refs
}

fn convert_table_factor(tf: TableFactor, sql: &str) -> TableRefIr {
    match tf {
        TableFactor::Table { name, alias, .. } => {
            let parts: Vec<String> = name
                .0
                .iter()
                .filter_map(|p| match p {
                    ObjectNamePart::Identifier(ident) => Some(ident.value.to_lowercase()),
                    _ => None,
                })
                .collect();
            let (schema, tname) = if parts.len() > 1 {
                (Some(parts[0].clone()), parts[parts.len() - 1].clone())
            } else {
                (None, parts[0].clone())
            };
            let alias_str = alias.map(|a| a.name.value.to_lowercase());
            TableRefIr::Table {
                schema,
                name: tname,
                alias: alias_str,
                byte_range: 0..sql.len(),
            }
        }
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            let alias_str = alias
                .map(|a| a.name.value.to_lowercase())
                .unwrap_or_default();
            let body = convert_query(*subquery, sql).body;
            TableRefIr::Subquery {
                body: Box::new(body),
                alias: alias_str,
                byte_range: 0..sql.len(),
            }
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            let refs = convert_table_with_joins(*table_with_joins, sql);
            refs.into_iter()
                .reduce(|l, r| TableRefIr::Join {
                    left: Box::new(l),
                    right: Box::new(r),
                })
                .unwrap_or_else(|| TableRefIr::Table {
                    schema: None,
                    name: "unknown".into(),
                    alias: None,
                    byte_range: 0..0,
                })
        }
        _ => TableRefIr::Table {
            schema: None,
            name: "unknown".into(),
            alias: None,
            byte_range: 0..sql.len(),
        },
    }
}

fn convert_select_item(item: SelectItem) -> SelectItemIr {
    match item {
        SelectItem::Wildcard(_) => SelectItemIr::Wildcard,
        SelectItem::QualifiedWildcard(kind, _) => {
            let tname = match kind {
                SelectItemQualifiedWildcardKind::ObjectName(obj_name) => obj_name
                    .0
                    .last()
                    .and_then(|p| match p {
                        ObjectNamePart::Identifier(ident) => Some(ident.value.to_lowercase()),
                        _ => None,
                    })
                    .unwrap_or_default(),
                // For expression-based wildcards (e.g. STRUCT<STRING>('foo').*),
                // we don't have a meaningful table name.
                SelectItemQualifiedWildcardKind::Expr(_) => String::new(),
            };
            SelectItemIr::TableWildcard(tname)
        }
        SelectItem::ExprWithAlias { alias, .. } => SelectItemIr::Expr {
            alias: Some(alias.value.to_lowercase()),
            byte_range: 0..1,
        },
        SelectItem::UnnamedExpr(_) => SelectItemIr::Expr {
            alias: None,
            byte_range: 0..1,
        },
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // 1. Simple SELECT
    // =========================================================================

    #[test]
    fn test_sqlite_simple_select() {
        let sql = "SELECT id, name FROM users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert!(sel.with.is_none());
            assert_eq!(sel.body.from.len(), 1);
            assert!(
                matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
            assert_eq!(sel.body.select_list.len(), 2);
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 2. SELECT with table alias
    // =========================================================================

    #[test]
    fn test_sqlite_select_with_table_alias() {
        let sql = "SELECT u.id FROM users u";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { name, alias, .. } = &sel.body.from[0] {
                assert_eq!(name, "users");
                assert_eq!(alias.as_deref(), Some("u"));
            } else {
                panic!("Expected Table ref");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 3. Schema-qualified table
    // =========================================================================

    #[test]
    fn test_sqlite_schema_qualified_table() {
        let sql = "SELECT * FROM main.users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("main"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref with schema");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 4. SELECT * → Wildcard
    // =========================================================================

    #[test]
    fn test_sqlite_select_wildcard() {
        let sql = "SELECT * FROM users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.select_list.len(), 1);
            assert!(matches!(&sel.body.select_list[0], SelectItemIr::Wildcard));
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 5. SELECT t.* → TableWildcard
    // =========================================================================

    #[test]
    fn test_sqlite_table_wildcard() {
        let sql = "SELECT t.* FROM users t";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.select_list.len(), 1);
            if let SelectItemIr::TableWildcard(tname) = &sel.body.select_list[0] {
                assert_eq!(tname, "t");
            } else {
                panic!(
                    "Expected TableWildcard, got {:?}",
                    sel.body.select_list[0]
                );
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 6. JOIN → both tables in from (flat)
    // =========================================================================

    #[test]
    fn test_sqlite_join_flat_from() {
        let sql = "SELECT * FROM orders o JOIN users u ON o.user_id = u.id";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            // Our convert_table_with_joins flattens joins into separate TableRefIr entries
            assert_eq!(sel.body.from.len(), 2);
            assert!(
                matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "orders")
            );
            assert!(
                matches!(&sel.body.from[1], TableRefIr::Table { name, .. } if name == "users")
            );
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 7. Subquery → TableRefIr::Subquery
    // =========================================================================

    #[test]
    fn test_sqlite_subquery() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS sub";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 1);
            if let TableRefIr::Subquery { alias, body, .. } = &sel.body.from[0] {
                assert_eq!(alias, "sub");
                assert_eq!(body.from.len(), 1);
                assert!(
                    matches!(&body.from[0], TableRefIr::Table { name, .. } if name == "users")
                );
            } else {
                panic!("Expected Subquery");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 8. Simple CTE
    // =========================================================================

    #[test]
    fn test_sqlite_simple_cte() {
        let sql = "WITH active AS (SELECT id FROM users) SELECT * FROM active";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            assert_eq!(with.ctes[0].name, "active");
            assert_eq!(with.ctes[0].body.from.len(), 1);
            assert!(
                matches!(&with.ctes[0].body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 9. CTE with explicit columns
    // =========================================================================

    #[test]
    fn test_sqlite_cte_with_explicit_columns() {
        let sql = "WITH cte(x, y) AS (SELECT 1, 2) SELECT * FROM cte";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            let cte = &with.ctes[0];
            assert_eq!(cte.name, "cte");
            assert_eq!(cte.explicit_columns, vec!["x", "y"]);
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 10. Multiple CTEs
    // =========================================================================

    #[test]
    fn test_sqlite_multiple_ctes() {
        let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 2);
            assert_eq!(with.ctes[0].name, "a");
            assert_eq!(with.ctes[1].name, "b");
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 11. UNION → uses left branch
    // =========================================================================

    #[test]
    fn test_sqlite_union_uses_left_side() {
        let sql = "SELECT id FROM users UNION SELECT id FROM admins";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            // Left side (users) should be used for scope
            assert_eq!(sel.body.from.len(), 1);
            assert!(
                matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 12. DELETE with WHERE
    // =========================================================================

    #[test]
    fn test_sqlite_delete_with_where() {
        let sql = "DELETE FROM users WHERE id = 1";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                has_where: true
            }
        ));
    }

    // =========================================================================
    // 13. DELETE without WHERE
    // =========================================================================

    #[test]
    fn test_sqlite_delete_without_where() {
        let sql = "DELETE FROM users";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 14. TRUNCATE → Dangerous (note: SQLite does not support TRUNCATE natively
    //     but sqlparser will parse it anyway)
    // =========================================================================

    // SQLite does not natively support TRUNCATE; the sqlparser SQLiteDialect
    // may or may not parse it. We skip this test for SQLite and cover it in MySQL.

    // =========================================================================
    // 15. DROP TABLE → Dangerous
    // =========================================================================

    #[test]
    fn test_sqlite_drop_table() {
        let sql = "DROP TABLE users";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 16. INSERT → Other
    // =========================================================================

    #[test]
    fn test_sqlite_insert_returns_other() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other));
    }

    // =========================================================================
    // 17. Incomplete SQL → None
    // =========================================================================

    #[test]
    fn test_sqlite_incomplete_sql_returns_none() {
        let sql = "SELECT * FROM";
        let result = parse_sqlite(sql);
        assert!(result.is_none(), "Incomplete SQL should return None");
    }

    #[test]
    fn test_sqlite_incomplete_cte_returns_none() {
        let sql = "WITH cte AS (";
        let result = parse_sqlite(sql);
        assert!(result.is_none(), "Incomplete CTE should return None");
    }

    // =========================================================================
    // 18. Empty string → None
    // =========================================================================

    #[test]
    fn test_sqlite_empty_string_returns_none() {
        let result = parse_sqlite("");
        assert!(result.is_none());
    }

    #[test]
    fn test_sqlite_whitespace_only_returns_none() {
        let result = parse_sqlite("   \n\t  ");
        assert!(result.is_none());
    }

    // =========================================================================
    // 19. Column alias
    // =========================================================================

    #[test]
    fn test_sqlite_column_alias() {
        let sql = "SELECT id AS user_id FROM users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let SelectItemIr::Expr { alias, .. } = &sel.body.select_list[0] {
                assert_eq!(alias.as_deref(), Some("user_id"));
            } else {
                panic!("Expected Expr with alias");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 20. Multi-level 3-part name → schema = first, name = last
    // =========================================================================

    #[test]
    fn test_sqlite_three_part_name() {
        // SQLite: db.schema.table → parts = ["db", "schema", "table"]
        // Our logic: schema = parts[0], name = parts.last()
        let sql = "SELECT * FROM db.main.users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("db"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // Additional edge cases
    // =========================================================================

    #[test]
    fn test_sqlite_names_are_lowercased() {
        let sql = "SELECT * FROM MAIN.USERS";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("main"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_cte_names_are_lowercased() {
        let sql = "WITH MyActiveCTE AS (SELECT id FROM USERS) SELECT * FROM MyActiveCTE";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes[0].name, "myactivecte");
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_update_with_where() {
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: true
            }
        ));
    }

    #[test]
    fn test_sqlite_update_without_where() {
        let sql = "UPDATE users SET name = 'Bob'";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: false
            }
        ));
    }

    #[test]
    fn test_sqlite_select_no_from() {
        let sql = "SELECT 1 + 1";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 0);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_byte_range_covers_whole_sql() {
        let sql = "SELECT id FROM users";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.byte_range, 0..sql.len());
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_subquery_alias_is_lowercased() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS MySubquery";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Subquery { alias, .. } = &sel.body.from[0] {
                assert_eq!(alias, "mysubquery");
            } else {
                panic!("Expected Subquery");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_multiple_from_tables() {
        let sql = "SELECT * FROM a, b";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 2);
            assert!(matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "a"));
            assert!(matches!(&sel.body.from[1], TableRefIr::Table { name, .. } if name == "b"));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_with_non_recursive_flag() {
        let sql = "WITH cte AS (SELECT 1) SELECT * FROM cte";
        let result = parse_sqlite(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert!(!with.recursive);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_sqlite_create_table_returns_other() {
        let sql = "CREATE TABLE t (id INTEGER PRIMARY KEY)";
        let result = parse_sqlite(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other));
    }
}
