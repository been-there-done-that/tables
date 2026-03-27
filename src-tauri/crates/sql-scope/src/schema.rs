/// Type of a SQL column as understood by sql-scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlType {
    Integer,
    BigInt,
    Float,
    Text,
    Boolean,
    Date,
    Timestamp,
    Uuid,
    Json,
    Unknown,
}

impl SqlType {
    /// Map a data_type string (from schema introspection) to SqlType.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            t if t.contains("bigint") || t == "int8" => SqlType::BigInt,
            // "interval" contains "int" — exclude it explicitly
            t if t.contains("int") && !t.contains("interval") => SqlType::Integer,
            t if t.contains("float")
                || t.contains("real")
                || t.contains("double")
                || t.contains("numeric")
                || t.contains("decimal") =>
            {
                SqlType::Float
            }
            t if t.contains("text")
                || t.contains("varchar")
                || t.contains("char")
                || t.contains("string") =>
            {
                SqlType::Text
            }
            t if t.contains("bool") => SqlType::Boolean,
            t if t.contains("timestamp") || t.contains("datetime") => SqlType::Timestamp,
            t if t.contains("date") => SqlType::Date,
            t if t.contains("uuid") => SqlType::Uuid,
            t if t.contains("json") => SqlType::Json,
            _ => SqlType::Unknown,
        }
    }
}

/// A foreign key relationship from `from_column` in the current table
/// to `to_column` in `to_table`.
#[derive(Debug, Clone)]
pub struct ForeignKey {
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

/// Thin read-only interface over the schema cache.
/// The existing `SchemaGraph` will implement this in the integration task.
pub trait SchemaSnapshot: Send + Sync {
    fn table_exists(&self, schema: Option<&str>, table: &str) -> bool;
    fn table_columns(&self, schema: Option<&str>, table: &str) -> Option<Vec<String>>;
    fn column_type(&self, schema: Option<&str>, table: &str, column: &str) -> Option<SqlType>;
    fn foreign_keys(&self, schema: Option<&str>, table: &str) -> Vec<ForeignKey>;
    fn default_schema(&self) -> Option<&str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // SqlType::from_str — integer variants
    // -------------------------------------------------------------------------

    #[test]
    fn integer_lowercase() {
        assert_eq!(SqlType::from_str("integer"), SqlType::Integer);
    }

    #[test]
    fn integer_int4() {
        assert_eq!(SqlType::from_str("int4"), SqlType::Integer);
    }

    #[test]
    fn integer_smallint() {
        assert_eq!(SqlType::from_str("smallint"), SqlType::Integer);
    }

    #[test]
    fn integer_tinyint() {
        assert_eq!(SqlType::from_str("tinyint"), SqlType::Integer);
    }

    #[test]
    fn integer_mediumint() {
        assert_eq!(SqlType::from_str("mediumint"), SqlType::Integer);
    }

    #[test]
    fn integer_uppercase() {
        assert_eq!(SqlType::from_str("INTEGER"), SqlType::Integer);
    }

    #[test]
    fn integer_int_bare() {
        assert_eq!(SqlType::from_str("int"), SqlType::Integer);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — bigint variants (must match before int)
    // -------------------------------------------------------------------------

    #[test]
    fn bigint_int8() {
        assert_eq!(SqlType::from_str("int8"), SqlType::BigInt);
    }

    #[test]
    fn bigint_bigint() {
        assert_eq!(SqlType::from_str("bigint"), SqlType::BigInt);
    }

    #[test]
    fn bigint_uppercase() {
        assert_eq!(SqlType::from_str("BIGINT"), SqlType::BigInt);
    }

    #[test]
    fn bigint_mixed_case() {
        assert_eq!(SqlType::from_str("BigInt"), SqlType::BigInt);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — float variants
    // -------------------------------------------------------------------------

    #[test]
    fn float_float4() {
        assert_eq!(SqlType::from_str("float4"), SqlType::Float);
    }

    #[test]
    fn float_float8() {
        assert_eq!(SqlType::from_str("float8"), SqlType::Float);
    }

    #[test]
    fn float_real() {
        assert_eq!(SqlType::from_str("real"), SqlType::Float);
    }

    #[test]
    fn float_double_precision() {
        assert_eq!(SqlType::from_str("double precision"), SqlType::Float);
    }

    #[test]
    fn float_numeric_with_precision() {
        assert_eq!(SqlType::from_str("numeric(10,2)"), SqlType::Float);
    }

    #[test]
    fn float_decimal() {
        assert_eq!(SqlType::from_str("decimal"), SqlType::Float);
    }

    #[test]
    fn float_decimal_with_precision() {
        assert_eq!(SqlType::from_str("decimal(18,4)"), SqlType::Float);
    }

    #[test]
    fn float_numeric_bare() {
        assert_eq!(SqlType::from_str("numeric"), SqlType::Float);
    }

    #[test]
    fn float_uppercase() {
        assert_eq!(SqlType::from_str("FLOAT"), SqlType::Float);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — text variants
    // -------------------------------------------------------------------------

    #[test]
    fn text_varchar_with_len() {
        assert_eq!(SqlType::from_str("varchar(255)"), SqlType::Text);
    }

    #[test]
    fn text_character_varying() {
        assert_eq!(SqlType::from_str("character varying"), SqlType::Text);
    }

    #[test]
    fn text_text() {
        assert_eq!(SqlType::from_str("text"), SqlType::Text);
    }

    #[test]
    fn text_char_with_len() {
        assert_eq!(SqlType::from_str("char(10)"), SqlType::Text);
    }

    #[test]
    fn text_string() {
        assert_eq!(SqlType::from_str("string"), SqlType::Text);
    }

    #[test]
    fn text_varchar_uppercase() {
        assert_eq!(SqlType::from_str("VARCHAR"), SqlType::Text);
    }

    #[test]
    fn text_tinytext() {
        assert_eq!(SqlType::from_str("tinytext"), SqlType::Text);
    }

    #[test]
    fn text_mediumtext() {
        assert_eq!(SqlType::from_str("mediumtext"), SqlType::Text);
    }

    #[test]
    fn text_longtext() {
        assert_eq!(SqlType::from_str("longtext"), SqlType::Text);
    }

    #[test]
    fn text_nvarchar() {
        assert_eq!(SqlType::from_str("nvarchar(100)"), SqlType::Text);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — boolean variants
    // -------------------------------------------------------------------------

    #[test]
    fn boolean_boolean() {
        assert_eq!(SqlType::from_str("boolean"), SqlType::Boolean);
    }

    #[test]
    fn boolean_bool() {
        assert_eq!(SqlType::from_str("bool"), SqlType::Boolean);
    }

    #[test]
    fn boolean_uppercase() {
        assert_eq!(SqlType::from_str("BOOLEAN"), SqlType::Boolean);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — timestamp variants
    // -------------------------------------------------------------------------

    #[test]
    fn timestamp_timestamp() {
        assert_eq!(SqlType::from_str("timestamp"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_with_time_zone() {
        assert_eq!(
            SqlType::from_str("timestamp with time zone"),
            SqlType::Timestamp
        );
    }

    #[test]
    fn timestamp_timestamptz() {
        assert_eq!(SqlType::from_str("timestamptz"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_datetime() {
        assert_eq!(SqlType::from_str("datetime"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_without_time_zone() {
        assert_eq!(
            SqlType::from_str("timestamp without time zone"),
            SqlType::Timestamp
        );
    }

    #[test]
    fn timestamp_uppercase() {
        assert_eq!(SqlType::from_str("TIMESTAMP"), SqlType::Timestamp);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — date (must not match timestamp since "timestamp"
    // is checked before "date")
    // -------------------------------------------------------------------------

    #[test]
    fn date_date() {
        assert_eq!(SqlType::from_str("date"), SqlType::Date);
    }

    #[test]
    fn date_uppercase() {
        assert_eq!(SqlType::from_str("DATE"), SqlType::Date);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — uuid
    // -------------------------------------------------------------------------

    #[test]
    fn uuid_uuid() {
        assert_eq!(SqlType::from_str("uuid"), SqlType::Uuid);
    }

    #[test]
    fn uuid_uppercase() {
        assert_eq!(SqlType::from_str("UUID"), SqlType::Uuid);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — json variants
    // -------------------------------------------------------------------------

    #[test]
    fn json_json() {
        assert_eq!(SqlType::from_str("json"), SqlType::Json);
    }

    #[test]
    fn json_jsonb() {
        assert_eq!(SqlType::from_str("jsonb"), SqlType::Json);
    }

    #[test]
    fn json_uppercase() {
        assert_eq!(SqlType::from_str("JSON"), SqlType::Json);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_str — unknown / fallthrough
    // -------------------------------------------------------------------------

    #[test]
    fn unknown_bytea() {
        assert_eq!(SqlType::from_str("bytea"), SqlType::Unknown);
    }

    #[test]
    fn unknown_blob() {
        assert_eq!(SqlType::from_str("blob"), SqlType::Unknown);
    }

    #[test]
    fn unknown_empty_string() {
        assert_eq!(SqlType::from_str(""), SqlType::Unknown);
    }

    #[test]
    fn unknown_binary() {
        assert_eq!(SqlType::from_str("binary"), SqlType::Unknown);
    }

    #[test]
    fn unknown_varbinary() {
        assert_eq!(SqlType::from_str("varbinary(255)"), SqlType::Unknown);
    }

    #[test]
    fn unknown_completely_made_up() {
        assert_eq!(SqlType::from_str("xyzzy_type"), SqlType::Unknown);
    }

    #[test]
    fn unknown_time() {
        // "time" does not contain "timestamp", "datetime", or "date" as substrings
        // after the timestamp/date arms; but "time" doesn't contain "date" either.
        // Verify it falls to Unknown rather than a false positive.
        assert_eq!(SqlType::from_str("time"), SqlType::Unknown);
    }

    #[test]
    fn unknown_interval() {
        assert_eq!(SqlType::from_str("interval"), SqlType::Unknown);
    }

    // -------------------------------------------------------------------------
    // ForeignKey — construction and field access
    // -------------------------------------------------------------------------

    #[test]
    fn foreign_key_fields() {
        let fk = ForeignKey {
            from_column: "user_id".to_string(),
            to_table: "users".to_string(),
            to_column: "id".to_string(),
        };
        assert_eq!(fk.from_column, "user_id");
        assert_eq!(fk.to_table, "users");
        assert_eq!(fk.to_column, "id");
    }

    #[test]
    fn foreign_key_clone() {
        let fk = ForeignKey {
            from_column: "order_id".to_string(),
            to_table: "orders".to_string(),
            to_column: "id".to_string(),
        };
        let fk2 = fk.clone();
        assert_eq!(fk2.from_column, fk.from_column);
        assert_eq!(fk2.to_table, fk.to_table);
        assert_eq!(fk2.to_column, fk.to_column);
    }

    // -------------------------------------------------------------------------
    // SchemaSnapshot trait — verify a concrete impl compiles and satisfies
    // the trait bounds (Send + Sync).
    // -------------------------------------------------------------------------

    struct MockSchema {
        default_schema: Option<String>,
    }

    impl SchemaSnapshot for MockSchema {
        fn table_exists(&self, _schema: Option<&str>, table: &str) -> bool {
            matches!(table, "users" | "orders" | "products")
        }

        fn table_columns(&self, _schema: Option<&str>, table: &str) -> Option<Vec<String>> {
            match table {
                "users" => Some(vec![
                    "id".to_string(),
                    "name".to_string(),
                    "email".to_string(),
                ]),
                "orders" => Some(vec![
                    "id".to_string(),
                    "user_id".to_string(),
                    "total".to_string(),
                ]),
                _ => None,
            }
        }

        fn column_type(
            &self,
            _schema: Option<&str>,
            table: &str,
            column: &str,
        ) -> Option<SqlType> {
            match (table, column) {
                ("users", "id") => Some(SqlType::Integer),
                ("users", "name") => Some(SqlType::Text),
                ("users", "email") => Some(SqlType::Text),
                ("orders", "id") => Some(SqlType::Integer),
                ("orders", "user_id") => Some(SqlType::Integer),
                ("orders", "total") => Some(SqlType::Float),
                _ => None,
            }
        }

        fn foreign_keys(&self, _schema: Option<&str>, table: &str) -> Vec<ForeignKey> {
            match table {
                "orders" => vec![ForeignKey {
                    from_column: "user_id".to_string(),
                    to_table: "users".to_string(),
                    to_column: "id".to_string(),
                }],
                _ => vec![],
            }
        }

        fn default_schema(&self) -> Option<&str> {
            self.default_schema.as_deref()
        }
    }

    // Verify Send + Sync at compile time.
    fn assert_send_sync<T: Send + Sync>(_: &T) {}

    #[test]
    fn mock_schema_send_sync() {
        let mock = MockSchema {
            default_schema: Some("public".to_string()),
        };
        assert_send_sync(&mock);
    }

    #[test]
    fn mock_table_exists_known() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert!(mock.table_exists(None, "users"));
        assert!(mock.table_exists(Some("public"), "orders"));
        assert!(mock.table_exists(None, "products"));
    }

    #[test]
    fn mock_table_exists_unknown() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert!(!mock.table_exists(None, "nonexistent"));
    }

    #[test]
    fn mock_table_columns_users() {
        let mock = MockSchema {
            default_schema: None,
        };
        let cols = mock.table_columns(None, "users").unwrap();
        assert_eq!(cols, vec!["id", "name", "email"]);
    }

    #[test]
    fn mock_table_columns_missing() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert!(mock.table_columns(None, "nonexistent").is_none());
    }

    #[test]
    fn mock_column_type_integer() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert_eq!(mock.column_type(None, "users", "id"), Some(SqlType::Integer));
    }

    #[test]
    fn mock_column_type_text() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert_eq!(mock.column_type(None, "users", "name"), Some(SqlType::Text));
    }

    #[test]
    fn mock_column_type_float() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert_eq!(
            mock.column_type(None, "orders", "total"),
            Some(SqlType::Float)
        );
    }

    #[test]
    fn mock_column_type_unknown_column() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert!(mock.column_type(None, "users", "nonexistent").is_none());
    }

    #[test]
    fn mock_foreign_keys_orders() {
        let mock = MockSchema {
            default_schema: None,
        };
        let fks = mock.foreign_keys(None, "orders");
        assert_eq!(fks.len(), 1);
        assert_eq!(fks[0].from_column, "user_id");
        assert_eq!(fks[0].to_table, "users");
        assert_eq!(fks[0].to_column, "id");
    }

    #[test]
    fn mock_foreign_keys_empty() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert!(mock.foreign_keys(None, "users").is_empty());
    }

    #[test]
    fn mock_default_schema_some() {
        let mock = MockSchema {
            default_schema: Some("public".to_string()),
        };
        assert_eq!(mock.default_schema(), Some("public"));
    }

    #[test]
    fn mock_default_schema_none() {
        let mock = MockSchema {
            default_schema: None,
        };
        assert_eq!(mock.default_schema(), None);
    }

    // -------------------------------------------------------------------------
    // Trait object usage — dyn SchemaSnapshot
    // -------------------------------------------------------------------------

    #[test]
    fn trait_object_table_exists() {
        let mock: Box<dyn SchemaSnapshot> = Box::new(MockSchema {
            default_schema: Some("main".to_string()),
        });
        assert!(mock.table_exists(None, "users"));
        assert!(!mock.table_exists(None, "missing_table"));
    }

    #[test]
    fn trait_object_column_type() {
        let mock: Box<dyn SchemaSnapshot> = Box::new(MockSchema {
            default_schema: None,
        });
        assert_eq!(
            mock.column_type(None, "orders", "user_id"),
            Some(SqlType::Integer)
        );
    }
}
