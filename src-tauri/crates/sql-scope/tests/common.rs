use sql_scope::schema::{ForeignKey, SchemaSnapshot, SqlType};
use std::collections::HashMap;

pub struct MockSchema {
    pub tables: HashMap<String, Vec<(String, SqlType)>>,
    pub fks: HashMap<String, Vec<ForeignKey>>,
}

impl MockSchema {
    pub fn new(tables: &[(&str, &[(&str, SqlType)])]) -> Self {
        Self {
            tables: tables
                .iter()
                .map(|(t, cols)| {
                    (
                        t.to_string(),
                        cols.iter()
                            .map(|(c, ty)| (c.to_string(), ty.clone()))
                            .collect(),
                    )
                })
                .collect(),
            fks: HashMap::new(),
        }
    }

    pub fn with_fk(
        mut self,
        from_table: &str,
        from_col: &str,
        to_table: &str,
        to_col: &str,
    ) -> Self {
        self.fks
            .entry(from_table.to_string())
            .or_default()
            .push(ForeignKey {
                from_column: from_col.to_string(),
                to_table: to_table.to_string(),
                to_column: to_col.to_string(),
            });
        self
    }
}

impl SchemaSnapshot for MockSchema {
    fn table_exists(&self, _: Option<&str>, t: &str) -> bool {
        self.tables.contains_key(t)
    }

    fn table_columns(&self, _: Option<&str>, t: &str) -> Option<Vec<String>> {
        self.tables
            .get(t)
            .map(|cols| cols.iter().map(|(n, _)| n.clone()).collect())
    }

    fn column_type(&self, _: Option<&str>, t: &str, col: &str) -> Option<SqlType> {
        self.tables
            .get(t)?
            .iter()
            .find(|(n, _)| n == col)
            .map(|(_, ty)| ty.clone())
    }

    fn foreign_keys(&self, _: Option<&str>, t: &str) -> Vec<ForeignKey> {
        self.fks.get(t).cloned().unwrap_or_default()
    }

    fn default_schema(&self) -> Option<&str> {
        Some("public")
    }
}
