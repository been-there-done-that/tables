use sqlparser::dialect::MySqlDialect;

use crate::ir::ParsedStatement;
use crate::parser::sqlite::parse_with_dialect;

/// Parse a single complete MySQL statement into IR.
/// Returns None if the statement is incomplete or invalid.
pub fn parse_mysql(sql: &str) -> Option<ParsedStatement> {
    parse_with_dialect(sql, &MySqlDialect {})
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DangerousKind, SelectItemIr, TableRefIr};

    // =========================================================================
    // 1. Simple SELECT
    // =========================================================================

    #[test]
    fn test_mysql_simple_select() {
        let sql = "SELECT id, name FROM users";
        let result = parse_mysql(sql).expect("should parse");
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
    // 2. Backtick-quoted identifiers
    // =========================================================================

    #[test]
    fn test_mysql_backtick_identifiers() {
        let sql = "SELECT `id`, `name` FROM `users`";
        let result = parse_mysql(sql).expect("should parse backtick identifiers");
        if let ParsedStatement::Select(sel) = result {
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
    // 3. CTE (MySQL 8+ supports CTEs)
    // =========================================================================

    #[test]
    fn test_mysql_cte() {
        let sql = "WITH active AS (SELECT id FROM users) SELECT * FROM active";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 1);
            assert_eq!(with.ctes[0].name, "active");
        } else {
            panic!("Expected Select");
        }
    }

    // =========================================================================
    // 4. JOIN
    // =========================================================================

    #[test]
    fn test_mysql_join() {
        let sql = "SELECT * FROM orders o JOIN users u ON o.user_id = u.id";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            // flat structure from convert_table_with_joins
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
    // 5. Subquery
    // =========================================================================

    #[test]
    fn test_mysql_subquery() {
        let sql = "SELECT * FROM (SELECT id FROM users) AS sub";
        let result = parse_mysql(sql).expect("should parse");
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
    // 6. DELETE → Dangerous
    // =========================================================================

    #[test]
    fn test_mysql_delete_with_where() {
        let sql = "DELETE FROM users WHERE id = 1";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                has_where: true
            }
        ));
    }

    #[test]
    fn test_mysql_delete_without_where() {
        let sql = "DELETE FROM users";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 7. UPDATE without WHERE → Dangerous
    // =========================================================================

    #[test]
    fn test_mysql_update_with_where() {
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: true
            }
        ));
    }

    #[test]
    fn test_mysql_update_without_where() {
        let sql = "UPDATE users SET name = 'Bob'";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 8. DROP → Dangerous
    // =========================================================================

    #[test]
    fn test_mysql_drop_table() {
        let sql = "DROP TABLE users";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                has_where: false
            }
        ));
    }

    // =========================================================================
    // 9. Incomplete → None
    // =========================================================================

    #[test]
    fn test_mysql_incomplete_sql_returns_none() {
        let sql = "SELECT * FROM";
        let result = parse_mysql(sql);
        assert!(result.is_none(), "Incomplete SQL should return None");
    }

    #[test]
    fn test_mysql_incomplete_cte_returns_none() {
        let sql = "WITH cte AS (";
        let result = parse_mysql(sql);
        assert!(result.is_none(), "Incomplete CTE should return None");
    }

    // =========================================================================
    // Additional MySQL-specific tests
    // =========================================================================

    #[test]
    fn test_mysql_empty_string_returns_none() {
        let result = parse_mysql("");
        assert!(result.is_none());
    }

    #[test]
    fn test_mysql_whitespace_only_returns_none() {
        let result = parse_mysql("   \n\t  ");
        assert!(result.is_none());
    }

    #[test]
    fn test_mysql_truncate_table() {
        let sql = "TRUNCATE TABLE users";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(
            result,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Truncate,
                has_where: false
            }
        ));
    }

    #[test]
    fn test_mysql_insert_returns_other() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let result = parse_mysql(sql).expect("should parse");
        assert!(matches!(result, ParsedStatement::Other { .. }));
    }

    #[test]
    fn test_mysql_select_wildcard() {
        let sql = "SELECT * FROM users";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert!(matches!(&sel.body.select_list[0], SelectItemIr::Wildcard));
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_table_wildcard() {
        let sql = "SELECT t.* FROM users t";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
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

    #[test]
    fn test_mysql_column_alias() {
        let sql = "SELECT id AS user_id FROM users";
        let result = parse_mysql(sql).expect("should parse");
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

    #[test]
    fn test_mysql_schema_qualified_table() {
        let sql = "SELECT * FROM mydb.users";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("mydb"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref with schema");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_cte_with_explicit_columns() {
        let sql = "WITH cte(x, y) AS (SELECT 1, 2) SELECT * FROM cte";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            let cte = &with.ctes[0];
            assert_eq!(cte.explicit_columns, vec!["x", "y"]);
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_multiple_ctes() {
        let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            let with = sel.with.expect("should have with clause");
            assert_eq!(with.ctes.len(), 2);
            assert_eq!(with.ctes[0].name, "a");
            assert_eq!(with.ctes[1].name, "b");
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_union_uses_left_side() {
        let sql = "SELECT id FROM users UNION SELECT id FROM admins";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.body.from.len(), 1);
            assert!(
                matches!(&sel.body.from[0], TableRefIr::Table { name, .. } if name == "users")
            );
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_names_are_lowercased() {
        let sql = "SELECT * FROM MYDB.USERS";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("mydb"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_backtick_schema_qualified() {
        let sql = "SELECT * FROM `mydb`.`users`";
        let result = parse_mysql(sql).expect("should parse backtick schema-qualified");
        if let ParsedStatement::Select(sel) = result {
            if let TableRefIr::Table { schema, name, .. } = &sel.body.from[0] {
                assert_eq!(schema.as_deref(), Some("mydb"));
                assert_eq!(name, "users");
            } else {
                panic!("Expected Table ref with schema");
            }
        } else {
            panic!("Expected Select");
        }
    }

    #[test]
    fn test_mysql_byte_range_covers_whole_sql() {
        let sql = "SELECT id FROM users";
        let result = parse_mysql(sql).expect("should parse");
        if let ParsedStatement::Select(sel) = result {
            assert_eq!(sel.byte_range, 0..sql.len());
        } else {
            panic!("Expected Select");
        }
    }
}
