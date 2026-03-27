/// A resolved column reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRef {
    pub name: String,
    pub source_table: Option<String>,  // table or CTE name this column came from
    pub source_alias: Option<String>,  // alias used in the query
}

pub type ScopeId = usize;

/// A source (table, CTE, or derived table) available in a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Table {
        schema: Option<String>,
        name: String,
    },
    Cte {
        name: String,
    },
    DerivedTable {
        scope_id: ScopeId,
    },
    Alias {
        alias: String,
        target: Box<Source>,
    },
}

impl Source {
    /// The canonical name of this source (table name, CTE name, or alias).
    pub fn canonical_name(&self) -> &str {
        match self {
            Source::Table { name, .. } => name,
            Source::Cte { name } => name,
            Source::DerivedTable { .. } => "",  // scope_id has no name; caller uses alias key in IndexMap
            Source::Alias { alias, .. } => alias,
        }
    }
}

/// The set of symbols visible at a cursor position.
#[derive(Debug, Default)]
pub struct VisibleSymbols {
    /// (alias_or_name → Source) for all tables/CTEs visible at this position
    pub sources: Vec<(String, Source)>,
    /// Columns projected by visible sources (populated when schema is known)
    pub columns: Vec<ColumnRef>,
}

impl VisibleSymbols {
    /// Whether a source with the given name/alias is visible.
    pub fn has_source(&self, name: &str) -> bool {
        self.sources.iter().any(|(a, _)| a == name)
    }

    /// Get the Source for a given alias/name, if visible.
    pub fn get_source(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|(a, _)| a == name).map(|(_, s)| s)
    }

    /// All visible source names (aliases or canonical names).
    pub fn source_names(&self) -> Vec<&str> {
        self.sources.iter().map(|(a, _)| a.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // ColumnRef — construction and field access
    // -------------------------------------------------------------------------

    #[test]
    fn column_ref_construction_and_fields() {
        let col = ColumnRef {
            name: "user_id".to_string(),
            source_table: Some("users".to_string()),
            source_alias: Some("u".to_string()),
        };
        assert_eq!(col.name, "user_id");
        assert_eq!(col.source_table.as_deref(), Some("users"));
        assert_eq!(col.source_alias.as_deref(), Some("u"));
    }

    #[test]
    fn column_ref_no_table_no_alias() {
        let col = ColumnRef {
            name: "count".to_string(),
            source_table: None,
            source_alias: None,
        };
        assert_eq!(col.name, "count");
        assert!(col.source_table.is_none());
        assert!(col.source_alias.is_none());
    }

    // -------------------------------------------------------------------------
    // ColumnRef — Clone and PartialEq
    // -------------------------------------------------------------------------

    #[test]
    fn column_ref_clone_and_eq() {
        let col = ColumnRef {
            name: "id".to_string(),
            source_table: Some("orders".to_string()),
            source_alias: None,
        };
        let col2 = col.clone();
        assert_eq!(col, col2);
    }

    #[test]
    fn column_ref_ne_different_name() {
        let col1 = ColumnRef {
            name: "id".to_string(),
            source_table: None,
            source_alias: None,
        };
        let col2 = ColumnRef {
            name: "name".to_string(),
            source_table: None,
            source_alias: None,
        };
        assert_ne!(col1, col2);
    }

    #[test]
    fn column_ref_ne_different_table() {
        let col1 = ColumnRef {
            name: "id".to_string(),
            source_table: Some("users".to_string()),
            source_alias: None,
        };
        let col2 = ColumnRef {
            name: "id".to_string(),
            source_table: Some("orders".to_string()),
            source_alias: None,
        };
        assert_ne!(col1, col2);
    }

    // -------------------------------------------------------------------------
    // Source::Table — canonical_name
    // -------------------------------------------------------------------------

    #[test]
    fn source_table_canonical_name() {
        let src = Source::Table {
            schema: Some("public".to_string()),
            name: "users".to_string(),
        };
        assert_eq!(src.canonical_name(), "users");
    }

    #[test]
    fn source_table_canonical_name_no_schema() {
        let src = Source::Table {
            schema: None,
            name: "products".to_string(),
        };
        assert_eq!(src.canonical_name(), "products");
    }

    // -------------------------------------------------------------------------
    // Source::Cte — canonical_name
    // -------------------------------------------------------------------------

    #[test]
    fn source_cte_canonical_name() {
        let src = Source::Cte {
            name: "recent_orders".to_string(),
        };
        assert_eq!(src.canonical_name(), "recent_orders");
    }

    // -------------------------------------------------------------------------
    // Source::DerivedTable — canonical_name returns empty string (alias is the IndexMap key)
    // -------------------------------------------------------------------------

    #[test]
    fn source_derived_table_canonical_name() {
        let src = Source::DerivedTable { scope_id: 42 };
        assert_eq!(src.canonical_name(), "");
    }

    // -------------------------------------------------------------------------
    // Source::Alias — canonical_name returns alias
    // -------------------------------------------------------------------------

    #[test]
    fn source_alias_canonical_name() {
        let src = Source::Alias {
            alias: "sub".to_string(),
            target: Box::new(Source::Table { schema: None, name: "users".to_string() }),
        };
        assert_eq!(src.canonical_name(), "sub");
    }

    // -------------------------------------------------------------------------
    // Source — Clone
    // -------------------------------------------------------------------------

    #[test]
    fn source_table_clone() {
        let src = Source::Table {
            schema: Some("public".to_string()),
            name: "users".to_string(),
        };
        let src2 = src.clone();
        assert_eq!(src2.canonical_name(), "users");
    }

    #[test]
    fn source_cte_clone() {
        let src = Source::Cte {
            name: "my_cte".to_string(),
        };
        let src2 = src.clone();
        assert_eq!(src2.canonical_name(), "my_cte");
    }

    #[test]
    fn source_derived_table_clone() {
        let src = Source::DerivedTable { scope_id: 5 };
        let src2 = src.clone();
        assert_eq!(src2.canonical_name(), "");
        if let Source::DerivedTable { scope_id } = src2 {
            assert_eq!(scope_id, 5);
        } else {
            panic!("expected DerivedTable");
        }
    }

    // -------------------------------------------------------------------------
    // VisibleSymbols — default is empty
    // -------------------------------------------------------------------------

    #[test]
    fn visible_symbols_default_is_empty() {
        let vis = VisibleSymbols::default();
        assert!(vis.sources.is_empty());
        assert!(vis.columns.is_empty());
    }

    // -------------------------------------------------------------------------
    // VisibleSymbols::has_source — found and not found
    // -------------------------------------------------------------------------

    #[test]
    fn visible_symbols_has_source_found() {
        let mut vis = VisibleSymbols::default();
        vis.sources.push(("users".to_string(), Source::Table { schema: None, name: "users".to_string() }));
        assert!(vis.has_source("users"));
    }

    #[test]
    fn visible_symbols_has_source_not_found() {
        let mut vis = VisibleSymbols::default();
        vis.sources.push(("users".to_string(), Source::Table { schema: None, name: "users".to_string() }));
        assert!(!vis.has_source("orders"));
    }

    // -------------------------------------------------------------------------
    // VisibleSymbols::get_source — Some and None
    // -------------------------------------------------------------------------

    #[test]
    fn visible_symbols_get_source_found() {
        let mut vis = VisibleSymbols::default();
        vis.sources.push(("u".to_string(), Source::Table { schema: None, name: "users".to_string() }));
        let src = vis.get_source("u");
        assert!(src.is_some());
        assert_eq!(src.unwrap().canonical_name(), "users");
    }

    #[test]
    fn visible_symbols_get_source_not_found() {
        let vis = VisibleSymbols::default();
        assert!(vis.get_source("anything").is_none());
    }

    // -------------------------------------------------------------------------
    // VisibleSymbols::source_names — returns all names
    // -------------------------------------------------------------------------

    #[test]
    fn visible_symbols_source_names_returns_all() {
        let mut vis = VisibleSymbols::default();
        vis.sources.push(("users".to_string(), Source::Table { schema: None, name: "users".to_string() }));
        vis.sources.push(("o".to_string(), Source::Table { schema: None, name: "orders".to_string() }));
        vis.sources.push(("my_cte".to_string(), Source::Cte { name: "my_cte".to_string() }));
        let names = vis.source_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"users"));
        assert!(names.contains(&"o"));
        assert!(names.contains(&"my_cte"));
    }

    #[test]
    fn visible_symbols_source_names_empty() {
        let vis = VisibleSymbols::default();
        assert!(vis.source_names().is_empty());
    }

    // -------------------------------------------------------------------------
    // Multiple sources with different types
    // -------------------------------------------------------------------------

    #[test]
    fn visible_symbols_multiple_types() {
        let mut vis = VisibleSymbols::default();
        vis.sources.push(("users".to_string(), Source::Table { schema: Some("public".to_string()), name: "users".to_string() }));
        vis.sources.push(("recent".to_string(), Source::Cte { name: "recent".to_string() }));
        vis.sources.push(("sub".to_string(), Source::DerivedTable { scope_id: 1 }));

        assert!(vis.has_source("users"));
        assert!(vis.has_source("recent"));
        assert!(vis.has_source("sub"));
        assert!(!vis.has_source("nonexistent"));

        let names = vis.source_names();
        assert_eq!(names.len(), 3);
    }
}
