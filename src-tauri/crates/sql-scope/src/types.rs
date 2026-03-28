use crate::schema::{SchemaSnapshot, SqlType};

/// Resolve the type of a named column in a given table.
/// Returns `SqlType::Unknown` if the column or table is not found.
pub fn resolve_column_type(
    table: &str,
    column: &str,
    schema: &dyn SchemaSnapshot,
) -> SqlType {
    schema.column_type(None, table, column).unwrap_or(SqlType::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ForeignKey, SchemaSnapshot};
    use std::collections::HashMap;

    struct Mock(HashMap<String, Vec<(String, SqlType)>>);
    impl SchemaSnapshot for Mock {
        fn table_exists(&self, _: Option<&str>, t: &str) -> bool { self.0.contains_key(t) }
        fn table_columns(&self, _: Option<&str>, t: &str) -> Option<Vec<String>> {
            self.0.get(t).map(|cs| cs.iter().map(|(n, _)| n.clone()).collect())
        }
        fn column_type(&self, _: Option<&str>, t: &str, col: &str) -> Option<SqlType> {
            self.0.get(t)?.iter().find(|(n, _)| n == col).map(|(_, ty)| ty.clone())
        }
        fn foreign_keys(&self, _: Option<&str>, _: &str) -> Vec<ForeignKey> { vec![] }
        fn default_schema(&self) -> Option<&str> { Some("public") }
    }

    fn mock(entries: &[(&str, &[(&str, SqlType)])]) -> Mock {
        Mock(entries.iter().map(|(t, cols)| {
            (t.to_string(), cols.iter().map(|(c, ty)| (c.to_string(), ty.clone())).collect())
        }).collect())
    }

    #[test]
    fn resolves_integer_column() {
        let schema = mock(&[("users", &[("id", SqlType::Integer)])]);
        assert_eq!(resolve_column_type("users", "id", &schema), SqlType::Integer);
    }

    #[test]
    fn resolves_text_column() {
        let schema = mock(&[("users", &[("name", SqlType::Text)])]);
        assert_eq!(resolve_column_type("users", "name", &schema), SqlType::Text);
    }

    #[test]
    fn resolves_multiple_types() {
        let schema = mock(&[("users", &[
            ("id", SqlType::Integer),
            ("name", SqlType::Text),
            ("created_at", SqlType::Timestamp),
            ("is_active", SqlType::Boolean),
        ])]);
        assert_eq!(resolve_column_type("users", "id", &schema), SqlType::Integer);
        assert_eq!(resolve_column_type("users", "name", &schema), SqlType::Text);
        assert_eq!(resolve_column_type("users", "created_at", &schema), SqlType::Timestamp);
        assert_eq!(resolve_column_type("users", "is_active", &schema), SqlType::Boolean);
    }

    #[test]
    fn unknown_column_returns_unknown() {
        let schema = mock(&[("users", &[("id", SqlType::Integer)])]);
        assert_eq!(resolve_column_type("users", "nonexistent", &schema), SqlType::Unknown);
    }

    #[test]
    fn unknown_table_returns_unknown() {
        let schema = mock(&[]);
        assert_eq!(resolve_column_type("nonexistent", "id", &schema), SqlType::Unknown);
    }
}
