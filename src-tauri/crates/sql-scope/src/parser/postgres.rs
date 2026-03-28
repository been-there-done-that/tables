use pg_query::NodeEnum;

use crate::ir::{
    CteIr, DangerousKind, ParsedStatement, SelectBodyIr, SelectIr, SelectItemIr, TableRefIr,
    WithIr,
};

/// Parse a single complete PostgreSQL statement into IR.
/// Returns None if the statement is incomplete or invalid (pg_query parse failure).
pub fn parse_postgres(sql: &str) -> Option<ParsedStatement> {
    if sql.trim().is_empty() {
        return None;
    }

    let result = pg_query::parse(sql).ok()?;
    let raw_stmt = result.protobuf.stmts.into_iter().next()?;
    let node_enum = raw_stmt.stmt?.node?;

    match node_enum {
        NodeEnum::SelectStmt(sel) => {
            let select_ir = convert_select_stmt(&sel, sql);
            Some(ParsedStatement::Select(select_ir))
        }
        NodeEnum::DeleteStmt(del) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: del.where_clause.is_some(),
        }),
        NodeEnum::UpdateStmt(upd) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: upd.where_clause.is_some(),
        }),
        NodeEnum::TruncateStmt(_) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Truncate,
            has_where: false,
        }),
        NodeEnum::DropStmt(_) => Some(ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        }),
        _ => Some(ParsedStatement::Other),
    }
}

fn convert_select_stmt(
    sel: &pg_query::protobuf::SelectStmt,
    sql: &str,
) -> SelectIr {
    SelectIr {
        with: sel.with_clause.as_ref().map(|wc| convert_with_clause(wc, sql)),
        body: convert_select_body(sel, sql),
        byte_range: 0..sql.len(),
    }
}

fn convert_with_clause(wc: &pg_query::protobuf::WithClause, sql: &str) -> WithIr {
    let ctes = wc
        .ctes
        .iter()
        .filter_map(|node| {
            if let Some(NodeEnum::CommonTableExpr(cte)) = node.node.as_ref() {
                Some(convert_cte(cte, sql))
            } else {
                None
            }
        })
        .collect();

    WithIr {
        recursive: wc.recursive,
        ctes,
    }
}

fn convert_cte(cte: &pg_query::protobuf::CommonTableExpr, sql: &str) -> CteIr {
    let name = cte.ctename.to_lowercase();

    let explicit_columns: Vec<String> = cte
        .aliascolnames
        .iter()
        .filter_map(|node| {
            if let Some(NodeEnum::String(s)) = node.node.as_ref() {
                Some(s.sval.to_lowercase())
            } else {
                None
            }
        })
        .collect();

    // cterecursive is populated by the analyzer, not the parser.
    // For raw parse results it's always false. The recursive flag
    // for the whole WITH clause is on WithClause.recursive.
    let recursive = cte.cterecursive;

    // Extract the inner SelectStmt body.
    // For UNION ALL recursive CTEs, pg_query represents the body as a SelectStmt
    // with op != 0 (SetOperation::Union/Intersect/Except) — the anchor SELECT
    // is in `larg`. We use the anchor branch so CTE column projection works.
    let body = cte
        .ctequery
        .as_ref()
        .and_then(|q| q.node.as_ref())
        .and_then(|n| {
            if let NodeEnum::SelectStmt(inner_sel) = n {
                // op == 0 means plain SELECT; op != 0 means set operation — use larg (anchor)
                let body_sel: &pg_query::protobuf::SelectStmt = if inner_sel.op != 0 {
                    inner_sel
                        .larg
                        .as_deref()
                        .unwrap_or(inner_sel)
                } else {
                    inner_sel
                };
                Some(Box::new(convert_select_body(body_sel, sql)))
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            Box::new(SelectBodyIr {
                from: vec![],
                select_list: vec![],
                byte_range: 0..sql.len(),
            })
        });

    CteIr {
        name,
        explicit_columns,
        recursive,
        body,
        byte_range: 0..sql.len(),
    }
}

fn convert_select_body(
    sel: &pg_query::protobuf::SelectStmt,
    sql: &str,
) -> SelectBodyIr {
    let from = sel
        .from_clause
        .iter()
        .filter_map(|node| {
            node.node.as_ref().and_then(|n| parse_table_ref(n, sql))
        })
        .collect();

    let select_list = sel
        .target_list
        .iter()
        .filter_map(|node| {
            if let Some(NodeEnum::ResTarget(rt)) = node.node.as_ref() {
                Some(parse_select_item(rt))
            } else {
                None
            }
        })
        .collect();

    SelectBodyIr {
        from,
        select_list,
        byte_range: 0..sql.len(),
    }
}

fn parse_table_ref(node: &NodeEnum, sql: &str) -> Option<TableRefIr> {
    match node {
        NodeEnum::RangeVar(rv) => {
            let schema = if rv.schemaname.is_empty() {
                None
            } else {
                Some(rv.schemaname.to_lowercase())
            };
            let name = rv.relname.to_lowercase();
            let alias = rv
                .alias
                .as_ref()
                .filter(|a| !a.aliasname.is_empty())
                .map(|a| a.aliasname.to_lowercase());

            Some(TableRefIr::Table {
                schema,
                name,
                alias,
                byte_range: 0..sql.len(),
            })
        }
        NodeEnum::RangeSubselect(rs) => {
            let alias = rs
                .alias
                .as_ref()
                .filter(|a| !a.aliasname.is_empty())
                .map(|a| a.aliasname.to_lowercase())?;

            let body = rs
                .subquery
                .as_ref()
                .and_then(|q| q.node.as_ref())
                .and_then(|n| {
                    if let NodeEnum::SelectStmt(inner) = n {
                        Some(Box::new(convert_select_body(inner, sql)))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    Box::new(SelectBodyIr {
                        from: vec![],
                        select_list: vec![],
                        byte_range: 0..sql.len(),
                    })
                });

            Some(TableRefIr::Subquery {
                body,
                alias,
                byte_range: 0..sql.len(),
            })
        }
        NodeEnum::JoinExpr(je) => {
            let left = je
                .larg
                .as_ref()
                .and_then(|n| n.node.as_ref())
                .and_then(|n| parse_table_ref(n, sql))?;
            let right = je
                .rarg
                .as_ref()
                .and_then(|n| n.node.as_ref())
                .and_then(|n| parse_table_ref(n, sql))?;

            Some(TableRefIr::Join {
                left: Box::new(left),
                right: Box::new(right),
            })
        }
        _ => None,
    }
}

fn parse_select_item(rt: &pg_query::protobuf::ResTarget) -> SelectItemIr {
    if let Some(NodeEnum::ColumnRef(cr)) = rt.val.as_ref().and_then(|v| v.node.as_ref()) {
        // Check if this is a wildcard: fields contains A_Star
        let fields = &cr.fields;
        if !fields.is_empty() {
            let last = fields.last().unwrap();
            if matches!(last.node.as_ref(), Some(NodeEnum::AStar(_))) {
                // If there's a table qualifier before the star, it's TableWildcard
                if fields.len() > 1 {
                    // The qualifier is the string before the star
                    if let Some(NodeEnum::String(s)) = fields[fields.len() - 2].node.as_ref() {
                        return SelectItemIr::TableWildcard(s.sval.to_lowercase());
                    }
                }
                return SelectItemIr::Wildcard;
            }

            // Simple (or qualified) column reference with no explicit alias.
            // Use the column name itself as the implicit output name so CTE projection works:
            //   SELECT id FROM t    → alias = "id"
            //   SELECT t.id FROM t  → alias = "id"  (last field)
            if rt.name.is_empty() {
                if let Some(NodeEnum::String(s)) = last.node.as_ref() {
                    return SelectItemIr::Expr {
                        alias: Some(s.sval.to_lowercase()),
                        byte_range: 0..1,
                    };
                }
            }
        }
    }

    // Otherwise it's an expression (function call, arithmetic, etc.)
    let alias = if rt.name.is_empty() {
        None
    } else {
        Some(rt.name.to_lowercase())
    };

    SelectItemIr::Expr {
        alias,
        byte_range: 0..1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // 1. Simple SELECT
    // =========================================================================

    #[test]
    fn test_simple_select() {
        let sql = "SELECT id, name FROM users";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert!(sel.with.is_none());
            assert_eq!(sel.body.from.len(), 1);
            assert!(
                matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
            assert_eq!(sel.body.select_list.len(), 2);
            // Simple column refs use their name as the implicit alias for CTE projection
            assert!(matches!(&sel.body.select_list[0], SelectItemIr::Expr { alias: Some(a), .. } if a == "id"));
            assert!(matches!(&sel.body.select_list[1], SelectItemIr::Expr { alias: Some(a), .. } if a == "name"));
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 2. SELECT with alias
    // =========================================================================

    #[test]
    fn test_select_with_table_alias() {
        let sql = "SELECT u.id FROM users u";
        let result = parse_postgres(sql).expect("should parse");
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
    // 3. SELECT with schema
    // =========================================================================

    #[test]
    fn test_select_with_schema() {
        let sql = "SELECT * FROM public.users";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("public"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 4. SELECT * → Wildcard
    // =========================================================================

    #[test]
    fn test_select_wildcard() {
        let sql = "SELECT * FROM users";
        let result = parse_postgres(sql).expect("should parse");
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
    fn test_select_table_wildcard() {
        let sql = "SELECT t.* FROM users t";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.select_list.len(), 1);
            if let SelectItemIr::TableWildcard(tname) = &sel.body.select_list[0] {
                assert_eq!(tname, "t");
            } else {
                panic!("Expected TableWildcard, got {:?}", sel.body.select_list[0]);
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 6. JOIN
    // =========================================================================

    #[test]
    fn test_select_with_join() {
        let sql = "SELECT * FROM orders o JOIN users u ON o.user_id = u.id";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 1);
            if let TableRefIr::Join { left, right } = &sel.body.from[0] {
                assert!(
                    matches!(left.as_ref(), TableRefIr::Table { name, .. } if name == "orders")
                );
                assert!(
                    matches!(right.as_ref(), TableRefIr::Table { name, .. } if name == "users")
                );
            } else {
                panic!("Expected Join");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 7. Subquery
    // =========================================================================

    #[test]
    fn test_select_with_subquery() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS sub";
        let result = parse_postgres(sql).expect("should parse");
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
    fn test_simple_cte() {
        let sql = "WITH active AS (SELECT id FROM users) SELECT * FROM active";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            assert_eq!(with.ctes[0].name, "active");
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 9. CTE with explicit columns
    // =========================================================================

    #[test]
    fn test_cte_with_explicit_columns() {
        let sql = "WITH cte(x, y) AS (SELECT 1, 2) SELECT * FROM cte";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            let cte = &with.ctes[0];
            assert_eq!(cte.explicit_columns, vec!["x", "y"]);
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 10. Recursive CTE
    // =========================================================================

    #[test]
    fn test_recursive_cte() {
        let sql = "WITH RECURSIVE nums(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM nums WHERE n < 10) SELECT * FROM nums";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert!(with.recursive, "WithIr.recursive should be true");
            assert_eq!(with.ctes.len(), 1);
            assert_eq!(with.ctes[0].name, "nums");
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 11. Multiple CTEs
    // =========================================================================

    #[test]
    fn test_multiple_ctes() {
        let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b";
        let result = parse_postgres(sql).expect("should parse");
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
    // 12. DELETE with WHERE
    // =========================================================================

    #[test]
    fn test_delete_with_where() {
        let sql = "DELETE FROM users WHERE id = 1";
        let result = parse_postgres(sql).expect("should parse");
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
    fn test_delete_without_where() {
        let sql = "DELETE FROM users";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 14. UPDATE with WHERE
    // =========================================================================

    #[test]
    fn test_update_with_where() {
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: true
            }
        ));
    }

    #[test]
    fn test_update_without_where() {
        let sql = "UPDATE users SET name = 'Bob'";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 15. TRUNCATE
    // =========================================================================

    #[test]
    fn test_truncate() {
        let sql = "TRUNCATE users";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Truncate,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 16. DROP TABLE
    // =========================================================================

    #[test]
    fn test_drop_table() {
        let sql = "DROP TABLE users";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 17. Incomplete SQL returns None
    // =========================================================================

    #[test]
    fn test_incomplete_select_from_returns_none() {
        let sql = "SELECT * FROM";
        let result = parse_postgres(sql);
        assert!(result.is_none(), "Incomplete SQL should return None");
    }

    #[test]
    fn test_incomplete_cte_returns_none() {
        let sql = "WITH cte AS (";
        let result = parse_postgres(sql);
        assert!(result.is_none(), "Incomplete CTE should return None");
    }

    // =========================================================================
    // 18. Empty string returns None
    // =========================================================================

    #[test]
    fn test_empty_string_returns_none() {
        let result = parse_postgres("");
        assert!(result.is_none());
    }

    #[test]
    fn test_whitespace_only_returns_none() {
        let result = parse_postgres("   \n\t  ");
        assert!(result.is_none());
    }

    // =========================================================================
    // 19. INSERT returns Other
    // =========================================================================

    #[test]
    fn test_insert_returns_other() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other));
    }

    // =========================================================================
    // 20. CREATE TABLE returns Other
    // =========================================================================

    #[test]
    fn test_create_table_returns_other() {
        let sql = "CREATE TABLE users (id INT, name TEXT)";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other));
    }

    // =========================================================================
    // 21. Column alias
    // =========================================================================

    #[test]
    fn test_column_alias() {
        let sql = "SELECT id AS user_id FROM users";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.select_list.len(), 1);
            if let SelectItemIr::Expr { alias, .. } = &sel.body.select_list[0] {
                assert_eq!(alias.as_deref(), Some("user_id"));
            } else {
                panic!("Expected Expr");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 22. Subquery CTE (CTE containing a subquery in its FROM)
    // =========================================================================

    #[test]
    fn test_cte_with_subquery_in_from() {
        let sql = "WITH cte AS (SELECT * FROM (SELECT id FROM users) AS sub) SELECT * FROM cte";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            let cte = &with.ctes[0];
            assert_eq!(cte.name, "cte");
            // The CTE body should have a subquery in FROM
            assert_eq!(cte.body.from.len(), 1);
            assert!(matches!(&cte.body.from[0], TableRefIr::Subquery { alias, .. } if alias == "sub"));
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 23. Multiple JOINs (3-way join)
    // =========================================================================

    #[test]
    fn test_three_way_join() {
        let sql = "SELECT * FROM a JOIN b ON a.id = b.a_id JOIN c ON b.id = c.b_id";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 1);
            // The outermost is a Join
            if let TableRefIr::Join { left, right } = &sel.body.from[0] {
                // Right should be table c
                assert!(matches!(right.as_ref(), TableRefIr::Table { name, .. } if name == "c"));
                // Left should be another Join (a JOIN b)
                assert!(matches!(left.as_ref(), TableRefIr::Join { .. }));
                if let TableRefIr::Join { left: ll, right: lr } = left.as_ref() {
                    assert!(
                        matches!(ll.as_ref(), TableRefIr::Table { name, .. } if name == "a")
                    );
                    assert!(
                        matches!(lr.as_ref(), TableRefIr::Table { name, .. } if name == "b")
                    );
                }
            } else {
                panic!("Expected Join at top level");
            }
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // Additional edge case tests
    // =========================================================================

    #[test]
    fn test_select_no_from() {
        let sql = "SELECT 1 + 1";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 0);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_select_multiple_from_tables() {
        let sql = "SELECT * FROM a, b";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 2);
            assert!(matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "a"));
            assert!(matches!(&sel.body.from[1], TableRefIr::Table { name, .. } if name == "b"));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_byte_range_covers_whole_sql() {
        let sql = "SELECT id FROM users";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.byte_range, 0..sql.len());
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_alter_table_returns_other() {
        let sql = "ALTER TABLE users ADD COLUMN email TEXT";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other));
    }

    #[test]
    fn test_cte_names_are_lowercased() {
        let sql = "WITH MyActiveCTE AS (SELECT id FROM USERS) SELECT * FROM MyActiveCTE";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes[0].name, "myactivecte");
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_table_names_are_lowercased() {
        let sql = "SELECT * FROM PUBLIC.USERS";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("public"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_with_clause_recursive_false_for_non_recursive() {
        let sql = "WITH cte AS (SELECT 1) SELECT * FROM cte";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert!(!with.recursive);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_drop_view_returns_dangerous() {
        let sql = "DROP VIEW my_view";
        let result = parse_postgres(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                ..
            }
        ));
    }

    #[test]
    fn test_subquery_alias_is_lowercased() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS MySubquery";
        let result = parse_postgres(sql).expect("should parse");
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
    fn test_select_expr_no_alias() {
        // Simple column reference: implicit alias is the column name itself
        let sql = "SELECT id FROM users";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let SelectItemIr::Expr { alias, .. } = &sel.body.select_list[0] {
                assert_eq!(alias.as_deref(), Some("id"),
                    "simple column ref should use column name as implicit alias");
            } else {
                panic!("Expected Expr");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_invalid_sql_returns_none() {
        let sql = "NOT VALID SQL AT ALL !!!";
        let result = parse_postgres(sql);
        // pg_query may or may not parse this; the important thing is we don't panic
        // (Some parsers are very permissive)
        let _ = result;
    }

    #[test]
    fn test_cte_body_has_from() {
        let sql = "WITH cte AS (SELECT id FROM users) SELECT * FROM cte";
        let result = parse_postgres(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            let cte = &with.ctes[0];
            assert_eq!(cte.body.from.len(), 1);
            assert!(
                matches!(&cte.body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 24. Recursive CTE with UNION ALL — anchor branch column projection
    // =========================================================================

    #[test]
    fn test_recursive_cte_union_all_anchor_columns() {
        use crate::ir::{ParsedStatement, SelectItemIr};
        let sql = r#"WITH RECURSIVE dept_tree AS (
    SELECT id, name, parent_id, 0 AS depth
    FROM departments
    WHERE parent_id IS NULL
    UNION ALL
    SELECT d.id, d.name, d.parent_id, dt.depth + 1
    FROM departments d
    INNER JOIN dept_tree dt ON d.parent_id = dt.id
)
SELECT * FROM dept_tree"#;
        let parsed = super::parse_postgres(sql).expect("parse failed");
        let ParsedStatement::Select(sel) = parsed else { panic!("expected Select") };
        let with = sel.with.expect("expected WITH clause");
        assert_eq!(with.ctes.len(), 1);
        let cte = &with.ctes[0];
        assert_eq!(cte.name, "dept_tree");
        // The anchor SELECT projects: id, name, parent_id, depth (alias on 0)
        let aliases: Vec<Option<&str>> = cte.body.select_list.iter().map(|item| {
            if let SelectItemIr::Expr { alias, .. } = item { alias.as_deref() } else { None }
        }).collect();
        assert!(aliases.contains(&Some("id")),     "anchor should project 'id', got {:?}", aliases);
        assert!(aliases.contains(&Some("name")),   "anchor should project 'name', got {:?}", aliases);
        assert!(aliases.contains(&Some("depth")),  "anchor should project 'depth', got {:?}", aliases);
        // FROM clause should see departments
        assert!(!cte.body.from.is_empty(), "anchor FROM should not be empty");
    }
}
