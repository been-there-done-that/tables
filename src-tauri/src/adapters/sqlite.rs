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

    fn map_sqlite_type(raw: &str) -> String {
        let upper = raw.to_uppercase();
        if upper.contains("INT") {
            "integer".to_string()
        } else if upper.contains("CHAR") || upper.contains("CLOB") || upper.contains("TEXT") {
            "text".to_string()
        } else if upper.contains("BLOB") {
            "binary".to_string()
        } else if upper.contains("REAL") || upper.contains("FLOA") || upper.contains("DOUB") {
            "float".to_string()
        } else if upper.contains("BOOL") {
            "boolean".to_string()
        } else if upper.contains("DATE") || upper.contains("TIME") {
            "datetime".to_string()
        } else {
            "text".to_string()
        }
    }
}

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

        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info('{}')", table.name))
            .map_err(|e| AdapterError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let type_str: String = row.get(2)?;
                Ok(MetaColumn {
                    connection_id: String::new(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.name.clone(),
                    ordinal_position: row.get(0)?,
                    column_name: row.get(1)?,
                    raw_type: type_str.clone(),
                    logical_type: Self::map_sqlite_type(&type_str),
                    nullable: row.get::<_, i32>(3)? == 0,
                    default_value: row.get(4)?,
                    is_primary_key: row.get::<_, i32>(5)? != 0,
                    engine_type: None,
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
        assert_eq!(SqliteAdapter::map_sqlite_type("INTEGER"), "integer");
        assert_eq!(SqliteAdapter::map_sqlite_type("TEXT"), "text");
        assert_eq!(SqliteAdapter::map_sqlite_type("REAL"), "float");
        assert_eq!(SqliteAdapter::map_sqlite_type("BLOB"), "binary");
    }

    #[test]
    fn test_sqlite_capabilities() {
        let adapter = SqliteAdapter::new(SqliteConfig::new("/tmp/test.db"));
        let caps = adapter.capabilities();
        
        assert_eq!(caps.engine, "sqlite");
        assert!(!caps.supports_schemas);
        assert!(!caps.supports_databases);
    }
}
