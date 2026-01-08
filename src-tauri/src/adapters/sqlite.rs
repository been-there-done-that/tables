//! SQLite Database Adapter
//!
//! Implements the `DatabaseAdapter` trait for SQLite databases.
//! SQLite has a flat namespace with no true schemas or databases,
//! so we use "main" as the synthetic database and schema name.

use async_trait::async_trait;
use rusqlite::Connection as RusqliteConnection;
use std::sync::{Arc, Mutex};
use log::{info, debug};

use crate::adapter::{
    AdapterError, DatabaseAdapter, DatabaseCapabilities, TableRef,
};
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaIndex, MetaSchema, MetaTable, MetaTrigger,
    compute_fk_hash,
};

/// SQLite adapter configuration
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    pub path: String,
}

impl SqliteConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

/// SQLite database adapter
pub struct SqliteAdapter {
    capabilities: DatabaseCapabilities,
    config: SqliteConfig,
    connection: Option<Arc<Mutex<RusqliteConnection>>>,
}

impl SqliteAdapter {
    pub fn new(config: SqliteConfig) -> Self {
        Self {
            capabilities: DatabaseCapabilities::sqlite(),
            config,
            connection: None,
        }
    }

    pub fn from_config(config: serde_json::Value) -> Result<Self, AdapterError> {
        // Try root level
        let path = config.get("path")
            .or_else(|| config.get("file"))
            // Try inside 'db' object (matching Postgres pattern)
            .or_else(|| config.get("db").and_then(|db| db.get("path").or_else(|| db.get("file"))))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AdapterError::Connection("Missing 'path' or 'file' in SQLite config".to_string()))?;
        
        Ok(Self::new(SqliteConfig::new(path)))
    }

    fn conn(&self) -> Result<Arc<Mutex<RusqliteConnection>>, AdapterError> {
        self.connection
            .clone()
            .ok_or_else(|| AdapterError::Connection("Not connected".to_string()))
    }

    // =========================================================================
    // 2026-Grade Type System: Multi-Layer Introspection
    // =========================================================================

    /// Compute SQLite affinity from declared type per SQLite's affinity rules.
    /// Reference: https://www.sqlite.org/datatype3.html#determination_of_column_affinity
    fn compute_affinity(raw_type: &str) -> SqliteAffinity {
        let upper = raw_type.to_uppercase();
        
        // Rule 1: If "INT" is in the type name -> INTEGER affinity
        if upper.contains("INT") {
            return SqliteAffinity::Integer;
        }
        
        // Rule 2: If "CHAR", "CLOB", or "TEXT" -> TEXT affinity
        if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
            return SqliteAffinity::Text;
        }
        
        // Rule 3: If "BLOB" or no type specified -> BLOB affinity
        if upper.contains("BLOB") || upper.is_empty() {
            return SqliteAffinity::Blob;
        }
        
        // Rule 4: If "REAL", "FLOA", or "DOUB" -> REAL affinity
        if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
            return SqliteAffinity::Real;
        }
        
        // Rule 5: Otherwise -> NUMERIC affinity
        SqliteAffinity::Numeric
    }

    /// Infer semantic hint from declared type.
    /// This heuristic is DISABLED for STRICT tables (returns SemanticHint::None).
    fn infer_semantic_hint(raw_type: &str, is_strict: bool) -> SemanticHint {
        // STRICT tables only allow INT, INTEGER, REAL, TEXT, BLOB, ANY
        // No semantic inference for strict tables
        if is_strict {
            return SemanticHint::None;
        }
        
        let upper = raw_type.to_uppercase();
        
        // UUID / GUID detection
        if upper.contains("UUID") || upper.contains("GUID") {
            return SemanticHint::Uuid;
        }
        
        // JSON detection
        if upper.contains("JSON") {
            return SemanticHint::Json;
        }
        
        // Boolean detection
        if upper.contains("BOOL") {
            return SemanticHint::Boolean;
        }
        
        // DateTime detection (before Date/Time to catch DATETIME/TIMESTAMP)
        if upper.contains("DATETIME") || upper.contains("TIMESTAMP") {
            return SemanticHint::DateTime;
        }
        
        // Date detection
        if upper.contains("DATE") {
            return SemanticHint::Date;
        }
        
        // Time detection
        if upper.contains("TIME") {
            return SemanticHint::Time;
        }
        
        // Decimal / Money detection
        if upper.contains("DECIMAL") || upper.contains("MONEY") || upper.contains("CURRENCY") {
            return SemanticHint::Decimal;
        }
        
        SemanticHint::None
    }

    /// Detect if a column is generated (virtual or stored) from table_xinfo hidden value.
    /// hidden = 0: normal column
    /// hidden = 1: dynamic/virtual (e.g., FTS5 columns)
    /// hidden = 2: virtual generated column
    /// hidden = 3: stored generated column
    fn is_column_generated(hidden: i32) -> bool {
        hidden == 2 || hidden == 3
    }

    /// Map raw type to logical type string (backwards compatible)
    fn map_sqlite_type(raw: &str) -> String {
        let affinity = Self::compute_affinity(raw);
        let hint = Self::infer_semantic_hint(raw, false);
        
        // Use semantic hint if available, otherwise use affinity
        match hint {
            SemanticHint::Uuid => "uuid".to_string(),
            SemanticHint::Json => "json".to_string(),
            SemanticHint::Boolean => "boolean".to_string(),
            SemanticHint::DateTime => "datetime".to_string(),
            SemanticHint::Date => "date".to_string(),
            SemanticHint::Time => "time".to_string(),
            SemanticHint::Decimal => "decimal".to_string(),
            SemanticHint::Enum { .. } => "enum".to_string(),
            SemanticHint::None => match affinity {
                SqliteAffinity::Integer => "integer".to_string(),
                SqliteAffinity::Text => "text".to_string(),
                SqliteAffinity::Blob => "binary".to_string(),
                SqliteAffinity::Real => "float".to_string(),
                SqliteAffinity::Numeric => "numeric".to_string(),
            },
        }
    }

    /// Get table metadata using PRAGMA table_list (SQLite 3.37+)
    /// Returns (is_strict, is_virtual, table_type)
    fn get_table_meta(conn: &RusqliteConnection, table_name: &str) -> (bool, bool, String) {
        // PRAGMA table_list returns: schema, name, type, ncol, wr, strict
        // Available in SQLite 3.37+ (2021-11-27)
        let result: Result<(String, i32), _> = conn.query_row(
            "SELECT type, strict FROM pragma_table_list WHERE name = ?1",
            [table_name],
            |row| Ok((row.get(0)?, row.get(1)?))
        );
        
        match result {
            Ok((table_type, strict)) => {
                let is_virtual = table_type == "virtual";
                let is_strict = strict == 1;
                (is_strict, is_virtual, table_type)
            }
            Err(_) => {
                // Fallback for older SQLite versions without table_list
                // Check sqlite_master for virtual table detection
                let is_virtual = conn
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name = ?1 AND type = 'table'",
                        [table_name],
                        |row| {
                            let sql: String = row.get(0)?;
                            Ok(sql.to_uppercase().contains("USING "))
                        }
                    )
                    .unwrap_or(false);
                (false, is_virtual, "table".to_string())
            }
        }
    }
}

use crate::schema_types::{SqliteAffinity, SemanticHint, SqliteTypeMeta, EngineType, EngineTypeMeta, DatabaseEngine};

#[async_trait]
impl DatabaseAdapter for SqliteAdapter {
    fn capabilities(&self) -> &DatabaseCapabilities {
        &self.capabilities
    }

    async fn connect(&mut self) -> Result<(), AdapterError> {
        info!("Connecting to SQLite database at {}", self.config.path);
        let conn = RusqliteConnection::open(&self.config.path)
            .map_err(|e| AdapterError::Connection(format!("Failed to open SQLite: {}", e)))?;
        self.connection = Some(Arc::new(Mutex::new(conn)));
        debug!("SQLite connection established");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.connection = None;
        debug!("SQLite connection closed");
        Ok(())
    }

    async fn list_databases(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        Ok(vec![MetaDatabase {
            name: "main".to_string(),
            is_connected: self.is_connected(),
            is_introspected: false,
            schemas: vec![],
        }])
    }

    async fn list_schemas(&self, _database: &str) -> Result<Vec<MetaSchema>, AdapterError> {
        Ok(vec![MetaSchema {
            name: "main".to_string(),
            schema_type: "user".to_string(),
            kind: crate::schema_types::NamespaceKind::LogicalGroup,
            is_introspected: false,
            tables: vec![],
        }])
    }

    async fn list_tables(&self, _database: &str, _schema: &str) -> Result<Vec<MetaTable>, AdapterError> {
        let conn_arc = self.conn()?;
        let conn = conn_arc.lock().unwrap();

        let mut stmt = conn
            .prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'")
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        info!("[SQLITE_DEBUG] list_tables for {} - Query prepared", self.config.path);

        let now = chrono::Utc::now().timestamp_millis();
        
        let rows = stmt
            .query_map([], |row| {
                Ok(MetaTable {
                    connection_id: String::new(),
                    database: "main".to_string(),
                    schema: "main".to_string(),
                    table_name: row.get(0)?,
                    table_type: row.get(1)?,
                    classification: "user".to_string(),
                    last_introspected_at: now,
                    columns: vec![],
                    foreign_keys: vec![],
                    indexes: vec![],
                    triggers: vec![],
                })
            })
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AdapterError::Query(e.to_string()))
    }

    async fn list_columns(&self, table: &TableRef) -> Result<Vec<MetaColumn>, AdapterError> {
        let conn_arc = self.conn()?;
        let conn = conn_arc.lock().unwrap();

        // Step 1: Get table metadata using table_list (STRICT, virtual detection)
        let (is_strict, is_virtual, _table_type) = Self::get_table_meta(&conn, &table.name);
        
        debug!(
            "[SQLITE_2026] Table '{}' metadata: strict={}, virtual={}",
            table.name, is_strict, is_virtual
        );

        // Step 2: Use table_xinfo instead of table_info to get hidden column
        // table_xinfo columns: cid, name, type, notnull, dflt_value, pk, hidden
        let mut stmt = conn
            .prepare(&format!(
                "SELECT cid, name, type, notnull, dflt_value, pk, hidden FROM pragma_table_xinfo('{}')",
                table.name
            ))
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let cid: i32 = row.get(0)?;
                let col_name: String = row.get(1)?;
                let type_str: String = row.get(2)?;
                let notnull: i32 = row.get(3)?;
                let dflt_value: Option<String> = row.get(4)?;
                let pk: i32 = row.get(5)?;
                let hidden: i32 = row.get(6)?;
                
                // Compute affinity and semantic hint
                let affinity = Self::compute_affinity(&type_str);
                let semantic_hint = Self::infer_semantic_hint(&type_str, is_strict);
                let is_generated = Self::is_column_generated(hidden);
                
                // Build lossless engine-specific metadata
                let sqlite_meta = SqliteTypeMeta {
                    declared_type: type_str.clone(),
                    affinity,
                    semantic_hint: semantic_hint.clone(),
                    is_strict_table: is_strict,
                    is_generated,
                    is_virtual_table: is_virtual,
                };
                
                let engine_type = EngineType {
                    engine: DatabaseEngine::Sqlite,
                    raw_type: type_str.clone(),
                    metadata: EngineTypeMeta::Sqlite(sqlite_meta),
                };
                
                Ok(MetaColumn {
                    connection_id: String::new(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.name.clone(),
                    ordinal_position: cid,
                    column_name: col_name,
                    raw_type: type_str.clone(),
                    logical_type: Self::map_sqlite_type(&type_str),
                    nullable: notnull == 0,
                    default_value: dflt_value,
                    is_primary_key: pk > 0,
                    engine_type: Some(engine_type),
                    normalized_type: None,
                })
            })
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AdapterError::Query(e.to_string()))
    }

    async fn list_indexes(&self, table: &TableRef) -> Result<Vec<MetaIndex>, AdapterError> {
        let conn_arc = self.conn()?;
        let conn = conn_arc.lock().unwrap();

        let mut stmt = conn
            .prepare(&format!("SELECT name, \"unique\" FROM pragma_index_list('{}')", table.name))
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(MetaIndex {
                    connection_id: String::new(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.name.clone(),
                    index_name: row.get(0)?,
                    is_unique: row.get::<_, i32>(1)? != 0,
                })
            })
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AdapterError::Query(e.to_string()))
    }

    async fn list_foreign_keys(&self, table: &TableRef) -> Result<Vec<MetaForeignKey>, AdapterError> {
        let conn_arc = self.conn()?;
        let conn = conn_arc.lock().unwrap();

        let mut stmt = conn
            .prepare(&format!("PRAGMA foreign_key_list('{}')", table.name))
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: i32 = row.get(0)?;
                let seq: i32 = row.get(1)?;
                let ref_table: String = row.get(2)?;
                let column_name: String = row.get(3)?;
                let ref_column: String = row.get(4)?;
                let hash = compute_fk_hash(&table.name, &column_name, &ref_table, &ref_column);

                Ok(MetaForeignKey {
                    connection_id: String::new(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.name.clone(),
                    column_name,
                    ref_schema: "main".to_string(),
                    ref_table,
                    ref_column,
                    constraint_name: Some(format!("fk_{}_{}", table.name, id)),
                    constraint_hash: hash,
                    seq_no: seq + 1,
                })
            })
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AdapterError::Query(e.to_string()))
    }

    async fn list_triggers(&self, _table: &TableRef) -> Result<Vec<MetaTrigger>, AdapterError> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_config() {
        let config = SqliteConfig::new("/tmp/test.db");
        assert_eq!(config.path, "/tmp/test.db");
    }

    #[test]
    fn test_sqlite_type_mapping() {
        // Backwards compatible tests
        assert_eq!(SqliteAdapter::map_sqlite_type("INTEGER"), "integer");
        assert_eq!(SqliteAdapter::map_sqlite_type("TEXT"), "text");
        assert_eq!(SqliteAdapter::map_sqlite_type("REAL"), "float");
        assert_eq!(SqliteAdapter::map_sqlite_type("BLOB"), "binary");
        
        // New semantic types
        assert_eq!(SqliteAdapter::map_sqlite_type("UUID"), "uuid");
        assert_eq!(SqliteAdapter::map_sqlite_type("JSON"), "json");
        assert_eq!(SqliteAdapter::map_sqlite_type("BOOLEAN"), "boolean");
        assert_eq!(SqliteAdapter::map_sqlite_type("DATETIME"), "datetime");
    }

    #[test]
    fn test_sqlite_capabilities() {
        let adapter = SqliteAdapter::new(SqliteConfig::new("/tmp/test.db"));
        let caps = adapter.capabilities();
        
        assert_eq!(caps.engine, "sqlite");
        assert!(!caps.supports_schemas);
        assert!(!caps.supports_databases);
    }

    // =========================================================================
    // 2026-Grade Type System Tests
    // =========================================================================

    #[test]
    fn test_sqlite_affinity_computation() {
        // Rule 1: INT -> INTEGER affinity
        assert_eq!(SqliteAdapter::compute_affinity("INTEGER"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("INT"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("BIGINT"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("SMALLINT"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("TINYINT"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("MEDIUMINT"), SqliteAffinity::Integer);
        assert_eq!(SqliteAdapter::compute_affinity("UNSIGNED BIG INT"), SqliteAffinity::Integer);
        
        // Rule 2: CHAR/CLOB/TEXT -> TEXT affinity
        assert_eq!(SqliteAdapter::compute_affinity("TEXT"), SqliteAffinity::Text);
        assert_eq!(SqliteAdapter::compute_affinity("VARCHAR(255)"), SqliteAffinity::Text);
        assert_eq!(SqliteAdapter::compute_affinity("NVARCHAR(100)"), SqliteAffinity::Text);
        assert_eq!(SqliteAdapter::compute_affinity("CHARACTER(20)"), SqliteAffinity::Text);
        assert_eq!(SqliteAdapter::compute_affinity("CLOB"), SqliteAffinity::Text);
        
        // Rule 3: BLOB or empty -> BLOB affinity
        assert_eq!(SqliteAdapter::compute_affinity("BLOB"), SqliteAffinity::Blob);
        assert_eq!(SqliteAdapter::compute_affinity(""), SqliteAffinity::Blob);
        
        // Rule 4: REAL/FLOA/DOUB -> REAL affinity
        assert_eq!(SqliteAdapter::compute_affinity("REAL"), SqliteAffinity::Real);
        assert_eq!(SqliteAdapter::compute_affinity("DOUBLE"), SqliteAffinity::Real);
        assert_eq!(SqliteAdapter::compute_affinity("DOUBLE PRECISION"), SqliteAffinity::Real);
        assert_eq!(SqliteAdapter::compute_affinity("FLOAT"), SqliteAffinity::Real);
        
        // Rule 5: Everything else -> NUMERIC affinity
        assert_eq!(SqliteAdapter::compute_affinity("NUMERIC"), SqliteAffinity::Numeric);
        assert_eq!(SqliteAdapter::compute_affinity("DECIMAL(10,5)"), SqliteAffinity::Numeric);
        assert_eq!(SqliteAdapter::compute_affinity("BOOLEAN"), SqliteAffinity::Numeric);
        assert_eq!(SqliteAdapter::compute_affinity("DATE"), SqliteAffinity::Numeric);
        assert_eq!(SqliteAdapter::compute_affinity("DATETIME"), SqliteAffinity::Numeric);
    }

    #[test]
    fn test_semantic_hint_inference_non_strict() {
        // Semantic hints should be inferred for non-STRICT tables
        assert_eq!(SqliteAdapter::infer_semantic_hint("UUID", false), SemanticHint::Uuid);
        assert_eq!(SqliteAdapter::infer_semantic_hint("GUID", false), SemanticHint::Uuid);
        assert_eq!(SqliteAdapter::infer_semantic_hint("JSON", false), SemanticHint::Json);
        assert_eq!(SqliteAdapter::infer_semantic_hint("JSONB", false), SemanticHint::Json);
        assert_eq!(SqliteAdapter::infer_semantic_hint("BOOL", false), SemanticHint::Boolean);
        assert_eq!(SqliteAdapter::infer_semantic_hint("BOOLEAN", false), SemanticHint::Boolean);
        assert_eq!(SqliteAdapter::infer_semantic_hint("DATETIME", false), SemanticHint::DateTime);
        assert_eq!(SqliteAdapter::infer_semantic_hint("TIMESTAMP", false), SemanticHint::DateTime);
        assert_eq!(SqliteAdapter::infer_semantic_hint("DATE", false), SemanticHint::Date);
        assert_eq!(SqliteAdapter::infer_semantic_hint("TIME", false), SemanticHint::Time);
        assert_eq!(SqliteAdapter::infer_semantic_hint("DECIMAL(10,2)", false), SemanticHint::Decimal);
        assert_eq!(SqliteAdapter::infer_semantic_hint("MONEY", false), SemanticHint::Decimal);
        
        // No semantic hint for standard types
        assert_eq!(SqliteAdapter::infer_semantic_hint("INTEGER", false), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("TEXT", false), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("REAL", false), SemanticHint::None);
    }

    #[test]
    fn test_semantic_hint_disabled_for_strict() {
        // STRICT tables disable ALL semantic inference
        assert_eq!(SqliteAdapter::infer_semantic_hint("UUID", true), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("JSON", true), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("BOOLEAN", true), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("DATETIME", true), SemanticHint::None);
        assert_eq!(SqliteAdapter::infer_semantic_hint("DECIMAL(10,2)", true), SemanticHint::None);
    }

    #[test]
    fn test_generated_column_detection() {
        // hidden = 0: normal column
        assert!(!SqliteAdapter::is_column_generated(0));
        
        // hidden = 1: dynamic/virtual (FTS5 columns) - not treated as generated
        assert!(!SqliteAdapter::is_column_generated(1));
        
        // hidden = 2: virtual generated column
        assert!(SqliteAdapter::is_column_generated(2));
        
        // hidden = 3: stored generated column
        assert!(SqliteAdapter::is_column_generated(3));
    }
}
