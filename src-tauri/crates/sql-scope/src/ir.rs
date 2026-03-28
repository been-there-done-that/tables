use std::ops::Range;

/// A single parsed SQL statement.
#[derive(Debug, Clone)]
pub enum ParsedStatement {
    Select(SelectIr),
    /// DROP / TRUNCATE / DELETE / UPDATE — flagged for dangerous statement warning
    Dangerous { kind: DangerousKind, has_where: bool },
    /// INSERT, CREATE, ALTER, etc. — kept for future expansion
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DangerousKind {
    Drop,
    Truncate,
    DeleteWithoutWhere,
    UpdateWithoutWhere,
}

/// A SELECT statement, optionally with a WITH clause.
#[derive(Debug, Clone)]
pub struct SelectIr {
    pub with: Option<WithIr>,
    pub body: SelectBodyIr,
    /// Byte range of this statement in the original SQL string.
    pub byte_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct WithIr {
    pub recursive: bool,
    pub ctes: Vec<CteIr>,
}

#[derive(Debug, Clone)]
pub struct CteIr {
    /// CTE name as declared (lowercased by the parser backend).
    pub name: String,
    /// Explicit column list if provided: `WITH cte(a, b) AS (...)`.
    pub explicit_columns: Vec<String>,
    pub recursive: bool,
    pub body: Box<SelectBodyIr>,
    pub byte_range: Range<usize>,
}

impl CteIr {
    /// Returns explicit_columns if non-empty, else empty slice (columns not yet expanded).
    pub fn resolved_columns(&self) -> &[String] {
        if self.explicit_columns.is_empty() {
            &[]
        } else {
            &self.explicit_columns
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectBodyIr {
    pub from: Vec<TableRefIr>,
    pub select_list: Vec<SelectItemIr>,
    pub byte_range: Range<usize>,
}

impl SelectBodyIr {
    /// Returns true if select_list contains `SelectItemIr::Wildcard`.
    pub fn has_wildcard(&self) -> bool {
        self.select_list
            .iter()
            .any(|item| matches!(item, SelectItemIr::Wildcard))
    }
}

#[derive(Debug, Clone)]
pub enum TableRefIr {
    Table {
        schema: Option<String>,
        /// Table name (lowercased by the parser backend).
        name: String,
        alias: Option<String>,
        byte_range: Range<usize>,
    },
    Subquery {
        body: Box<SelectBodyIr>,
        alias: String,
        byte_range: Range<usize>,
    },
    Join {
        left: Box<TableRefIr>,
        right: Box<TableRefIr>,
    },
    /// A scalar or EXISTS subquery appearing in a WHERE/HAVING clause.
    /// Not in the FROM list, so has no alias. Registered to build a child scope
    /// that inherits the enclosing query's sources (enabling outer alias propagation).
    WhereSubquery {
        body: Box<SelectBodyIr>,
    },
}

impl TableRefIr {
    /// Returns alias if present, else table name for Table variant,
    /// alias for Subquery, None for Join.
    pub fn alias_or_name(&self) -> Option<&str> {
        match self {
            TableRefIr::Table { alias, name, .. } => {
                Some(alias.as_deref().unwrap_or(name.as_str()))
            }
            TableRefIr::Subquery { alias, .. } => Some(alias.as_str()),
            TableRefIr::Join { .. } => None,
            TableRefIr::WhereSubquery { .. } => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelectItemIr {
    /// SELECT *
    Wildcard,
    /// SELECT t.*
    TableWildcard(String),
    /// SELECT expr [AS alias]
    Expr {
        alias: Option<String>,
        byte_range: Range<usize>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Helper constructors
    // -------------------------------------------------------------------------

    fn make_select_body(select_list: Vec<SelectItemIr>, from: Vec<TableRefIr>) -> SelectBodyIr {
        SelectBodyIr {
            from,
            select_list,
            byte_range: 0..10,
        }
    }

    fn make_table_ref(name: &str) -> TableRefIr {
        TableRefIr::Table {
            schema: None,
            name: name.to_string(),
            alias: None,
            byte_range: 0..5,
        }
    }

    // =========================================================================
    // 1. Construction tests
    // =========================================================================

    #[test]
    fn test_parsed_statement_select_construction() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![make_table_ref("users")]);
        let stmt = ParsedStatement::Select(SelectIr {
            with: None,
            body,
            byte_range: 0..20,
        });
        assert!(matches!(stmt, ParsedStatement::Select(_)));
    }

    #[test]
    fn test_parsed_statement_dangerous_drop() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                has_where: false
            }
        ));
    }

    #[test]
    fn test_parsed_statement_dangerous_truncate() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::Truncate,
            has_where: false,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Truncate,
                ..
            }
        ));
    }

    #[test]
    fn test_parsed_statement_dangerous_delete_without_where() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: false,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous {
                kind: DangerousKind::DeleteWithoutWhere,
                ..
            }
        ));
    }

    #[test]
    fn test_parsed_statement_dangerous_update_without_where() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: false,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous {
                kind: DangerousKind::UpdateWithoutWhere,
                ..
            }
        ));
    }

    #[test]
    fn test_parsed_statement_dangerous_has_where_true() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: true,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous { has_where: true, .. }
        ));
    }

    #[test]
    fn test_parsed_statement_other() {
        let stmt = ParsedStatement::Other;
        assert!(matches!(stmt, ParsedStatement::Other));
    }

    #[test]
    fn test_select_ir_without_with() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
        let select = SelectIr {
            with: None,
            body,
            byte_range: 0..30,
        };
        assert!(select.with.is_none());
        assert_eq!(select.byte_range, 0..30);
    }

    #[test]
    fn test_select_ir_with_with_clause() {
        let cte_body = make_select_body(vec![SelectItemIr::Wildcard], vec![make_table_ref("t")]);
        let with = WithIr {
            recursive: false,
            ctes: vec![CteIr {
                name: "cte1".to_string(),
                explicit_columns: vec![],
                recursive: false,
                body: Box::new(cte_body),
                byte_range: 5..20,
            }],
        };
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
        let select = SelectIr {
            with: Some(with),
            body,
            byte_range: 0..50,
        };
        assert!(select.with.is_some());
        assert_eq!(select.with.unwrap().ctes.len(), 1);
    }

    #[test]
    fn test_with_ir_recursive_true() {
        let with = WithIr {
            recursive: true,
            ctes: vec![],
        };
        assert!(with.recursive);
    }

    #[test]
    fn test_with_ir_recursive_false() {
        let with = WithIr {
            recursive: false,
            ctes: vec![],
        };
        assert!(!with.recursive);
    }

    #[test]
    fn test_cte_ir_with_explicit_columns() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "my_cte".to_string(),
            explicit_columns: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            recursive: false,
            body: Box::new(body),
            byte_range: 0..15,
        };
        assert_eq!(cte.name, "my_cte");
        assert_eq!(cte.explicit_columns.len(), 3);
        assert_eq!(cte.explicit_columns[0], "a");
    }

    #[test]
    fn test_cte_ir_without_explicit_columns() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "anon_cte".to_string(),
            explicit_columns: vec![],
            recursive: false,
            body: Box::new(body),
            byte_range: 0..12,
        };
        assert!(cte.explicit_columns.is_empty());
    }

    #[test]
    fn test_select_body_ir_multiple_from_refs() {
        let from = vec![
            make_table_ref("users"),
            make_table_ref("orders"),
            make_table_ref("products"),
        ];
        let body = SelectBodyIr {
            from,
            select_list: vec![SelectItemIr::Wildcard],
            byte_range: 0..40,
        };
        assert_eq!(body.from.len(), 3);
    }

    #[test]
    fn test_table_ref_with_schema() {
        let tref = TableRefIr::Table {
            schema: Some("public".to_string()),
            name: "users".to_string(),
            alias: None,
            byte_range: 0..12,
        };
        if let TableRefIr::Table { schema, name, .. } = &tref {
            assert_eq!(schema.as_deref(), Some("public"));
            assert_eq!(name, "users");
        } else {
            panic!("Expected Table variant");
        }
    }

    #[test]
    fn test_table_ref_without_schema() {
        let tref = make_table_ref("orders");
        if let TableRefIr::Table { schema, name, .. } = &tref {
            assert!(schema.is_none());
            assert_eq!(name, "orders");
        } else {
            panic!("Expected Table variant");
        }
    }

    #[test]
    fn test_table_ref_with_alias() {
        let tref = TableRefIr::Table {
            schema: None,
            name: "users".to_string(),
            alias: Some("u".to_string()),
            byte_range: 0..10,
        };
        if let TableRefIr::Table { alias, .. } = &tref {
            assert_eq!(alias.as_deref(), Some("u"));
        } else {
            panic!("Expected Table variant");
        }
    }

    #[test]
    fn test_table_ref_without_alias() {
        let tref = make_table_ref("products");
        if let TableRefIr::Table { alias, .. } = &tref {
            assert!(alias.is_none());
        } else {
            panic!("Expected Table variant");
        }
    }

    #[test]
    fn test_table_ref_subquery() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![make_table_ref("inner_t")]);
        let subq = TableRefIr::Subquery {
            body: Box::new(body),
            alias: "sub".to_string(),
            byte_range: 5..30,
        };
        assert!(matches!(subq, TableRefIr::Subquery { .. }));
        if let TableRefIr::Subquery { alias, byte_range, .. } = &subq {
            assert_eq!(alias, "sub");
            assert_eq!(*byte_range, 5..30);
        }
    }

    #[test]
    fn test_table_ref_join_nested() {
        let left = make_table_ref("employees");
        let right = make_table_ref("departments");
        let join = TableRefIr::Join {
            left: Box::new(left),
            right: Box::new(right),
        };
        assert!(matches!(join, TableRefIr::Join { .. }));
        if let TableRefIr::Join { left, right } = &join {
            assert!(matches!(left.as_ref(), TableRefIr::Table { name, .. } if name == "employees"));
            assert!(matches!(right.as_ref(), TableRefIr::Table { name, .. } if name == "departments"));
        }
    }

    #[test]
    fn test_table_ref_join_deeply_nested() {
        let t1 = make_table_ref("a");
        let t2 = make_table_ref("b");
        let t3 = make_table_ref("c");
        let inner_join = TableRefIr::Join {
            left: Box::new(t1),
            right: Box::new(t2),
        };
        let outer_join = TableRefIr::Join {
            left: Box::new(inner_join),
            right: Box::new(t3),
        };
        assert!(matches!(outer_join, TableRefIr::Join { .. }));
        if let TableRefIr::Join { left, .. } = &outer_join {
            assert!(matches!(left.as_ref(), TableRefIr::Join { .. }));
        }
    }

    #[test]
    fn test_select_item_wildcard() {
        let item = SelectItemIr::Wildcard;
        assert!(matches!(item, SelectItemIr::Wildcard));
    }

    #[test]
    fn test_select_item_table_wildcard() {
        let item = SelectItemIr::TableWildcard("t".to_string());
        if let SelectItemIr::TableWildcard(table) = &item {
            assert_eq!(table, "t");
        } else {
            panic!("Expected TableWildcard");
        }
    }

    #[test]
    fn test_select_item_expr_with_alias() {
        let item = SelectItemIr::Expr {
            alias: Some("full_name".to_string()),
            byte_range: 7..20,
        };
        if let SelectItemIr::Expr { alias, byte_range } = &item {
            assert_eq!(alias.as_deref(), Some("full_name"));
            assert_eq!(*byte_range, 7..20);
        } else {
            panic!("Expected Expr");
        }
    }

    #[test]
    fn test_select_item_expr_without_alias() {
        let item = SelectItemIr::Expr {
            alias: None,
            byte_range: 0..5,
        };
        if let SelectItemIr::Expr { alias, .. } = &item {
            assert!(alias.is_none());
        } else {
            panic!("Expected Expr");
        }
    }

    // =========================================================================
    // 2. Pattern matching tests
    // =========================================================================

    #[test]
    fn test_pattern_match_parsed_statement_select() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
        let stmt = ParsedStatement::Select(SelectIr {
            with: None,
            body,
            byte_range: 0..10,
        });
        assert!(matches!(stmt, ParsedStatement::Select(_)));
        assert!(!matches!(stmt, ParsedStatement::Other));
    }

    #[test]
    fn test_pattern_match_dangerous_has_where_true() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::DeleteWithoutWhere,
            has_where: true,
        };
        assert!(matches!(
            stmt,
            ParsedStatement::Dangerous { has_where: true, .. }
        ));
    }

    #[test]
    fn test_pattern_match_dangerous_has_where_false() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::UpdateWithoutWhere,
            has_where: false,
        };
        assert!(!matches!(
            stmt,
            ParsedStatement::Dangerous { has_where: true, .. }
        ));
    }

    #[test]
    fn test_pattern_match_table_ref_destructuring() {
        let tref = TableRefIr::Table {
            schema: Some("myschema".to_string()),
            name: "mytable".to_string(),
            alias: Some("mt".to_string()),
            byte_range: 0..20,
        };
        assert!(
            matches!(&tref, TableRefIr::Table { name, .. } if name == "mytable")
        );
        assert!(
            matches!(&tref, TableRefIr::Table { schema: Some(s), .. } if s == "myschema")
        );
        assert!(
            matches!(&tref, TableRefIr::Table { alias: Some(a), .. } if a == "mt")
        );
    }

    // =========================================================================
    // 3. Clone tests
    // =========================================================================

    #[test]
    fn test_clone_parsed_statement_select() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![make_table_ref("users")]);
        let stmt = ParsedStatement::Select(SelectIr {
            with: None,
            body,
            byte_range: 0..20,
        });
        let cloned = stmt.clone();
        assert!(matches!(cloned, ParsedStatement::Select(_)));
    }

    #[test]
    fn test_clone_parsed_statement_dangerous() {
        let stmt = ParsedStatement::Dangerous {
            kind: DangerousKind::Drop,
            has_where: false,
        };
        let cloned = stmt.clone();
        assert!(matches!(
            cloned,
            ParsedStatement::Dangerous {
                kind: DangerousKind::Drop,
                ..
            }
        ));
    }

    #[test]
    fn test_clone_select_ir() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
        let select = SelectIr {
            with: None,
            body,
            byte_range: 5..25,
        };
        let cloned = select.clone();
        assert_eq!(cloned.byte_range, 5..25);
        assert!(cloned.with.is_none());
    }

    #[test]
    fn test_clone_with_ir() {
        let cte_body = make_select_body(vec![], vec![]);
        let with = WithIr {
            recursive: true,
            ctes: vec![CteIr {
                name: "cte_a".to_string(),
                explicit_columns: vec!["x".to_string()],
                recursive: false,
                body: Box::new(cte_body),
                byte_range: 0..10,
            }],
        };
        let cloned = with.clone();
        assert!(cloned.recursive);
        assert_eq!(cloned.ctes.len(), 1);
        assert_eq!(cloned.ctes[0].name, "cte_a");
    }

    #[test]
    fn test_clone_cte_ir() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "my_cte".to_string(),
            explicit_columns: vec!["col1".to_string(), "col2".to_string()],
            recursive: true,
            body: Box::new(body),
            byte_range: 0..15,
        };
        let cloned = cte.clone();
        assert_eq!(cloned.name, "my_cte");
        assert_eq!(cloned.explicit_columns, vec!["col1", "col2"]);
        assert!(cloned.recursive);
    }

    #[test]
    fn test_clone_select_body_ir() {
        let body = SelectBodyIr {
            from: vec![make_table_ref("a"), make_table_ref("b")],
            select_list: vec![SelectItemIr::Wildcard],
            byte_range: 0..30,
        };
        let cloned = body.clone();
        assert_eq!(cloned.from.len(), 2);
        assert_eq!(cloned.byte_range, 0..30);
    }

    #[test]
    fn test_clone_table_ref_table() {
        let tref = TableRefIr::Table {
            schema: Some("pub".to_string()),
            name: "items".to_string(),
            alias: None,
            byte_range: 0..10,
        };
        let cloned = tref.clone();
        assert!(matches!(cloned, TableRefIr::Table { name, .. } if name == "items"));
    }

    #[test]
    fn test_clone_table_ref_subquery() {
        let body = make_select_body(vec![], vec![]);
        let subq = TableRefIr::Subquery {
            body: Box::new(body),
            alias: "sq".to_string(),
            byte_range: 0..20,
        };
        let cloned = subq.clone();
        if let TableRefIr::Subquery { alias, .. } = cloned {
            assert_eq!(alias, "sq");
        } else {
            panic!("Expected Subquery");
        }
    }

    #[test]
    fn test_clone_table_ref_join() {
        let join = TableRefIr::Join {
            left: Box::new(make_table_ref("left_t")),
            right: Box::new(make_table_ref("right_t")),
        };
        let cloned = join.clone();
        assert!(matches!(cloned, TableRefIr::Join { .. }));
    }

    #[test]
    fn test_clone_select_item_wildcard() {
        let item = SelectItemIr::Wildcard;
        let cloned = item.clone();
        assert!(matches!(cloned, SelectItemIr::Wildcard));
    }

    #[test]
    fn test_clone_select_item_table_wildcard() {
        let item = SelectItemIr::TableWildcard("alias".to_string());
        let cloned = item.clone();
        if let SelectItemIr::TableWildcard(name) = cloned {
            assert_eq!(name, "alias");
        } else {
            panic!("Expected TableWildcard");
        }
    }

    #[test]
    fn test_clone_select_item_expr() {
        let item = SelectItemIr::Expr {
            alias: Some("label".to_string()),
            byte_range: 3..12,
        };
        let cloned = item.clone();
        if let SelectItemIr::Expr { alias, byte_range } = cloned {
            assert_eq!(alias.as_deref(), Some("label"));
            assert_eq!(byte_range, 3..12);
        } else {
            panic!("Expected Expr");
        }
    }

    // =========================================================================
    // 4. DangerousKind PartialEq tests
    // =========================================================================

    #[test]
    fn test_dangerous_kind_eq_drop() {
        assert_eq!(DangerousKind::Drop, DangerousKind::Drop);
    }

    #[test]
    fn test_dangerous_kind_eq_truncate() {
        assert_eq!(DangerousKind::Truncate, DangerousKind::Truncate);
    }

    #[test]
    fn test_dangerous_kind_eq_delete() {
        assert_eq!(
            DangerousKind::DeleteWithoutWhere,
            DangerousKind::DeleteWithoutWhere
        );
    }

    #[test]
    fn test_dangerous_kind_eq_update() {
        assert_eq!(
            DangerousKind::UpdateWithoutWhere,
            DangerousKind::UpdateWithoutWhere
        );
    }

    #[test]
    fn test_dangerous_kind_ne_drop_vs_truncate() {
        assert_ne!(DangerousKind::Drop, DangerousKind::Truncate);
    }

    #[test]
    fn test_dangerous_kind_ne_drop_vs_delete() {
        assert_ne!(DangerousKind::Drop, DangerousKind::DeleteWithoutWhere);
    }

    #[test]
    fn test_dangerous_kind_ne_drop_vs_update() {
        assert_ne!(DangerousKind::Drop, DangerousKind::UpdateWithoutWhere);
    }

    #[test]
    fn test_dangerous_kind_ne_truncate_vs_delete() {
        assert_ne!(DangerousKind::Truncate, DangerousKind::DeleteWithoutWhere);
    }

    #[test]
    fn test_dangerous_kind_ne_truncate_vs_update() {
        assert_ne!(DangerousKind::Truncate, DangerousKind::UpdateWithoutWhere);
    }

    #[test]
    fn test_dangerous_kind_ne_delete_vs_update() {
        assert_ne!(
            DangerousKind::DeleteWithoutWhere,
            DangerousKind::UpdateWithoutWhere
        );
    }

    // =========================================================================
    // 5. Helper method tests
    // =========================================================================

    // --- SelectBodyIr::has_wildcard ---

    #[test]
    fn test_has_wildcard_true_single() {
        let body = make_select_body(vec![SelectItemIr::Wildcard], vec![]);
        assert!(body.has_wildcard());
    }

    #[test]
    fn test_has_wildcard_true_mixed() {
        let body = make_select_body(
            vec![
                SelectItemIr::Expr {
                    alias: None,
                    byte_range: 0..3,
                },
                SelectItemIr::Wildcard,
                SelectItemIr::TableWildcard("t".to_string()),
            ],
            vec![],
        );
        assert!(body.has_wildcard());
    }

    #[test]
    fn test_has_wildcard_false_no_items() {
        let body = make_select_body(vec![], vec![]);
        assert!(!body.has_wildcard());
    }

    #[test]
    fn test_has_wildcard_false_only_table_wildcard() {
        let body = make_select_body(
            vec![SelectItemIr::TableWildcard("alias".to_string())],
            vec![],
        );
        assert!(!body.has_wildcard());
    }

    #[test]
    fn test_has_wildcard_false_only_expr() {
        let body = make_select_body(
            vec![SelectItemIr::Expr {
                alias: Some("col".to_string()),
                byte_range: 0..5,
            }],
            vec![],
        );
        assert!(!body.has_wildcard());
    }

    // --- TableRefIr::alias_or_name ---

    #[test]
    fn test_alias_or_name_table_with_alias() {
        let tref = TableRefIr::Table {
            schema: None,
            name: "users".to_string(),
            alias: Some("u".to_string()),
            byte_range: 0..10,
        };
        assert_eq!(tref.alias_or_name(), Some("u"));
    }

    #[test]
    fn test_alias_or_name_table_without_alias() {
        let tref = make_table_ref("orders");
        assert_eq!(tref.alias_or_name(), Some("orders"));
    }

    #[test]
    fn test_alias_or_name_table_with_schema_no_alias() {
        let tref = TableRefIr::Table {
            schema: Some("public".to_string()),
            name: "products".to_string(),
            alias: None,
            byte_range: 0..15,
        };
        assert_eq!(tref.alias_or_name(), Some("products"));
    }

    #[test]
    fn test_alias_or_name_subquery() {
        let body = make_select_body(vec![], vec![]);
        let subq = TableRefIr::Subquery {
            body: Box::new(body),
            alias: "derived".to_string(),
            byte_range: 0..20,
        };
        assert_eq!(subq.alias_or_name(), Some("derived"));
    }

    #[test]
    fn test_alias_or_name_join_returns_none() {
        let join = TableRefIr::Join {
            left: Box::new(make_table_ref("a")),
            right: Box::new(make_table_ref("b")),
        };
        assert_eq!(join.alias_or_name(), None);
    }

    // --- CteIr::resolved_columns ---

    #[test]
    fn test_resolved_columns_with_explicit_columns() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "my_cte".to_string(),
            explicit_columns: vec!["x".to_string(), "y".to_string(), "z".to_string()],
            recursive: false,
            body: Box::new(body),
            byte_range: 0..10,
        };
        let cols = cte.resolved_columns();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0], "x");
        assert_eq!(cols[1], "y");
        assert_eq!(cols[2], "z");
    }

    #[test]
    fn test_resolved_columns_empty_when_no_explicit_columns() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "unnamed_cte".to_string(),
            explicit_columns: vec![],
            recursive: false,
            body: Box::new(body),
            byte_range: 0..10,
        };
        let cols = cte.resolved_columns();
        assert!(cols.is_empty());
    }

    #[test]
    fn test_resolved_columns_returns_slice_ref() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "cte".to_string(),
            explicit_columns: vec!["a".to_string()],
            recursive: false,
            body: Box::new(body),
            byte_range: 0..5,
        };
        // Ensure it returns a reference (no clone), validate lifetimes work
        let cols: &[String] = cte.resolved_columns();
        assert_eq!(cols[0], "a");
    }

    // =========================================================================
    // 6. Byte range tests
    // =========================================================================

    #[test]
    fn test_byte_range_stored_in_select_ir() {
        let body = make_select_body(vec![], vec![]);
        let select = SelectIr {
            with: None,
            body,
            byte_range: 10..50,
        };
        assert_eq!(select.byte_range, 10..50);
        assert_eq!(select.byte_range.start, 10);
        assert_eq!(select.byte_range.end, 50);
    }

    #[test]
    fn test_byte_range_stored_in_cte_ir() {
        let body = make_select_body(vec![], vec![]);
        let cte = CteIr {
            name: "cte".to_string(),
            explicit_columns: vec![],
            recursive: false,
            body: Box::new(body),
            byte_range: 5..25,
        };
        assert_eq!(cte.byte_range, 5..25);
    }

    #[test]
    fn test_byte_range_stored_in_select_body_ir() {
        let body = SelectBodyIr {
            from: vec![],
            select_list: vec![],
            byte_range: 100..200,
        };
        assert_eq!(body.byte_range, 100..200);
    }

    #[test]
    fn test_byte_range_stored_in_table_ref_table() {
        let tref = TableRefIr::Table {
            schema: None,
            name: "tbl".to_string(),
            alias: None,
            byte_range: 7..14,
        };
        if let TableRefIr::Table { byte_range, .. } = tref {
            assert_eq!(byte_range, 7..14);
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_byte_range_stored_in_table_ref_subquery() {
        let body = make_select_body(vec![], vec![]);
        let subq = TableRefIr::Subquery {
            body: Box::new(body),
            alias: "s".to_string(),
            byte_range: 20..60,
        };
        if let TableRefIr::Subquery { byte_range, .. } = subq {
            assert_eq!(byte_range, 20..60);
        } else {
            panic!("Expected Subquery");
        }
    }

    #[test]
    fn test_byte_range_stored_in_select_item_expr() {
        let item = SelectItemIr::Expr {
            alias: None,
            byte_range: 30..45,
        };
        if let SelectItemIr::Expr { byte_range, .. } = item {
            assert_eq!(byte_range, 30..45);
        } else {
            panic!("Expected Expr");
        }
    }

    #[test]
    fn test_byte_range_zero_length() {
        let body = make_select_body(vec![], vec![]);
        let select = SelectIr {
            with: None,
            body,
            byte_range: 0..0,
        };
        assert_eq!(select.byte_range.len(), 0);
    }

    #[test]
    fn test_byte_range_clone_preserves_range() {
        let item = SelectItemIr::Expr {
            alias: None,
            byte_range: 42..99,
        };
        let cloned = item.clone();
        if let SelectItemIr::Expr { byte_range, .. } = cloned {
            assert_eq!(byte_range, 42..99);
        }
    }
}
