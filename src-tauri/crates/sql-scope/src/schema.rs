use std::str::FromStr;

/// Type of a SQL column as understood by sql-scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlType {
    Integer,
    BigInt,
    /// Covers FLOAT, REAL, DOUBLE PRECISION, NUMERIC, DECIMAL.
    /// Note: NUMERIC/DECIMAL are fixed-point but mapped here for simplicity;
    /// full type-safety checking would distinguish them.
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
    pub fn from_db_type(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            t if t.contains("bigint") || t.contains("int8") => SqlType::BigInt,
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

impl FromStr for SqlType {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SqlType::from_db_type(s))
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
    // SqlType::from_db_type — integer variants
    // -------------------------------------------------------------------------

    #[test]
    fn integer_lowercase() {
        assert_eq!(SqlType::from_db_type("integer"), SqlType::Integer);
    }

    #[test]
    fn integer_int4() {
        assert_eq!(SqlType::from_db_type("int4"), SqlType::Integer);
    }

    #[test]
    fn integer_smallint() {
        assert_eq!(SqlType::from_db_type("smallint"), SqlType::Integer);
    }

    #[test]
    fn integer_tinyint() {
        assert_eq!(SqlType::from_db_type("tinyint"), SqlType::Integer);
    }

    #[test]
    fn integer_mediumint() {
        assert_eq!(SqlType::from_db_type("mediumint"), SqlType::Integer);
    }

    #[test]
    fn integer_uppercase() {
        assert_eq!(SqlType::from_db_type("INTEGER"), SqlType::Integer);
    }

    #[test]
    fn integer_int_bare() {
        assert_eq!(SqlType::from_db_type("int"), SqlType::Integer);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — bigint variants (must match before int)
    // -------------------------------------------------------------------------

    #[test]
    fn bigint_int8() {
        assert_eq!(SqlType::from_db_type("int8"), SqlType::BigInt);
    }

    #[test]
    fn bigint_int8_with_constraint() {
        assert_eq!(SqlType::from_db_type("int8 not null"), SqlType::BigInt);
    }

    #[test]
    fn bigint_bigint() {
        assert_eq!(SqlType::from_db_type("bigint"), SqlType::BigInt);
    }

    #[test]
    fn bigint_uppercase() {
        assert_eq!(SqlType::from_db_type("BIGINT"), SqlType::BigInt);
    }

    #[test]
    fn bigint_mixed_case() {
        assert_eq!(SqlType::from_db_type("BigInt"), SqlType::BigInt);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — float variants
    // -------------------------------------------------------------------------

    #[test]
    fn float_float4() {
        assert_eq!(SqlType::from_db_type("float4"), SqlType::Float);
    }

    #[test]
    fn float_float8() {
        assert_eq!(SqlType::from_db_type("float8"), SqlType::Float);
    }

    #[test]
    fn float_real() {
        assert_eq!(SqlType::from_db_type("real"), SqlType::Float);
    }

    #[test]
    fn float_double_precision() {
        assert_eq!(SqlType::from_db_type("double precision"), SqlType::Float);
    }

    #[test]
    fn float_numeric_with_precision() {
        assert_eq!(SqlType::from_db_type("numeric(10,2)"), SqlType::Float);
    }

    #[test]
    fn float_decimal() {
        assert_eq!(SqlType::from_db_type("decimal"), SqlType::Float);
    }

    #[test]
    fn float_decimal_with_precision() {
        assert_eq!(SqlType::from_db_type("decimal(18,4)"), SqlType::Float);
    }

    #[test]
    fn float_numeric_bare() {
        assert_eq!(SqlType::from_db_type("numeric"), SqlType::Float);
    }

    #[test]
    fn float_uppercase() {
        assert_eq!(SqlType::from_db_type("FLOAT"), SqlType::Float);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — text variants
    // -------------------------------------------------------------------------

    #[test]
    fn text_varchar_with_len() {
        assert_eq!(SqlType::from_db_type("varchar(255)"), SqlType::Text);
    }

    #[test]
    fn text_character_varying() {
        assert_eq!(SqlType::from_db_type("character varying"), SqlType::Text);
    }

    #[test]
    fn text_text() {
        assert_eq!(SqlType::from_db_type("text"), SqlType::Text);
    }

    #[test]
    fn text_char_with_len() {
        assert_eq!(SqlType::from_db_type("char(10)"), SqlType::Text);
    }

    #[test]
    fn text_string() {
        assert_eq!(SqlType::from_db_type("string"), SqlType::Text);
    }

    #[test]
    fn text_varchar_uppercase() {
        assert_eq!(SqlType::from_db_type("VARCHAR"), SqlType::Text);
    }

    #[test]
    fn text_tinytext() {
        assert_eq!(SqlType::from_db_type("tinytext"), SqlType::Text);
    }

    #[test]
    fn text_mediumtext() {
        assert_eq!(SqlType::from_db_type("mediumtext"), SqlType::Text);
    }

    #[test]
    fn text_longtext() {
        assert_eq!(SqlType::from_db_type("longtext"), SqlType::Text);
    }

    #[test]
    fn text_nvarchar() {
        assert_eq!(SqlType::from_db_type("nvarchar(100)"), SqlType::Text);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — boolean variants
    // -------------------------------------------------------------------------

    #[test]
    fn boolean_boolean() {
        assert_eq!(SqlType::from_db_type("boolean"), SqlType::Boolean);
    }

    #[test]
    fn boolean_bool() {
        assert_eq!(SqlType::from_db_type("bool"), SqlType::Boolean);
    }

    #[test]
    fn boolean_uppercase() {
        assert_eq!(SqlType::from_db_type("BOOLEAN"), SqlType::Boolean);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — timestamp variants
    // -------------------------------------------------------------------------

    #[test]
    fn timestamp_timestamp() {
        assert_eq!(SqlType::from_db_type("timestamp"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_with_time_zone() {
        assert_eq!(
            SqlType::from_db_type("timestamp with time zone"),
            SqlType::Timestamp
        );
    }

    #[test]
    fn timestamp_timestamptz() {
        assert_eq!(SqlType::from_db_type("timestamptz"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_datetime() {
        assert_eq!(SqlType::from_db_type("datetime"), SqlType::Timestamp);
    }

    #[test]
    fn timestamp_without_time_zone() {
        assert_eq!(
            SqlType::from_db_type("timestamp without time zone"),
            SqlType::Timestamp
        );
    }

    #[test]
    fn timestamp_uppercase() {
        assert_eq!(SqlType::from_db_type("TIMESTAMP"), SqlType::Timestamp);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — date (must not match timestamp since "timestamp"
    // is checked before "date")
    // -------------------------------------------------------------------------

    #[test]
    fn date_date() {
        assert_eq!(SqlType::from_db_type("date"), SqlType::Date);
    }

    #[test]
    fn date_uppercase() {
        assert_eq!(SqlType::from_db_type("DATE"), SqlType::Date);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — uuid
    // -------------------------------------------------------------------------

    #[test]
    fn uuid_uuid() {
        assert_eq!(SqlType::from_db_type("uuid"), SqlType::Uuid);
    }

    #[test]
    fn uuid_uppercase() {
        assert_eq!(SqlType::from_db_type("UUID"), SqlType::Uuid);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — json variants
    // -------------------------------------------------------------------------

    #[test]
    fn json_json() {
        assert_eq!(SqlType::from_db_type("json"), SqlType::Json);
    }

    #[test]
    fn json_jsonb() {
        assert_eq!(SqlType::from_db_type("jsonb"), SqlType::Json);
    }

    #[test]
    fn json_uppercase() {
        assert_eq!(SqlType::from_db_type("JSON"), SqlType::Json);
    }

    // -------------------------------------------------------------------------
    // SqlType::from_db_type — unknown / fallthrough
    // -------------------------------------------------------------------------

    #[test]
    fn unknown_bytea() {
        assert_eq!(SqlType::from_db_type("bytea"), SqlType::Unknown);
    }

    #[test]
    fn unknown_blob() {
        assert_eq!(SqlType::from_db_type("blob"), SqlType::Unknown);
    }

    #[test]
    fn unknown_empty_string() {
        assert_eq!(SqlType::from_db_type(""), SqlType::Unknown);
    }

    #[test]
    fn unknown_binary() {
        assert_eq!(SqlType::from_db_type("binary"), SqlType::Unknown);
    }

    #[test]
    fn unknown_varbinary() {
        assert_eq!(SqlType::from_db_type("varbinary(255)"), SqlType::Unknown);
    }

    #[test]
    fn unknown_completely_made_up() {
        assert_eq!(SqlType::from_db_type("xyzzy_type"), SqlType::Unknown);
    }

    #[test]
    fn unknown_time() {
        // "time" does not contain "timestamp", "datetime", or "date" as substrings
        // after the timestamp/date arms; but "time" doesn't contain "date" either.
        // Verify it falls to Unknown rather than a false positive.
        assert_eq!(SqlType::from_db_type("time"), SqlType::Unknown);
    }

    #[test]
    fn unknown_interval() {
        assert_eq!(SqlType::from_db_type("interval"), SqlType::Unknown);
    }

    // -------------------------------------------------------------------------
    // FromStr / .parse::<SqlType>() — standard trait
    // -------------------------------------------------------------------------

    #[test]
    fn parse_integer() {
        assert_eq!("integer".parse::<SqlType>().unwrap(), SqlType::Integer);
    }

    #[test]
    fn parse_bigint() {
        assert_eq!("bigint".parse::<SqlType>().unwrap(), SqlType::BigInt);
    }

    #[test]
    fn parse_text() {
        assert_eq!("text".parse::<SqlType>().unwrap(), SqlType::Text);
    }

    #[test]
    fn parse_unknown() {
        assert_eq!("xyzzy".parse::<SqlType>().unwrap(), SqlType::Unknown);
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
