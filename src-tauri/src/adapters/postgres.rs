//! PostgreSQL Database Adapter
//!
//! Implements the `DatabaseAdapter` trait for PostgreSQL databases.
//! Supports full database/schema hierarchy with async tokio-postgres.

use async_trait::async_trait;
use log::{info, debug, error};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::adapter::{
    AdapterError, DatabaseAdapter, DatabaseCapabilities, TableRef,
};
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaIndex, MetaSchema, MetaTable, MetaTrigger,
    compute_fk_hash,
};

/// PostgreSQL adapter configuration
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
    pub use_tls: bool,
}

impl PostgresConfig {
    pub fn new(host: impl Into<String>, username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 5432,
            username: username.into(),
            password: password.into(),
            database: None,
            use_tls: false,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }

    fn connection_string(&self, database: Option<&str>) -> String {
        let db = database.or(self.database.as_deref()).unwrap_or("postgres");
        format!("postgres://{}:{}@{}:{}/{}", self.username, self.password, self.host, self.port, db)
    }
}

/// PostgreSQL database adapter
pub struct PostgresAdapter {
    capabilities: DatabaseCapabilities,
    config: PostgresConfig,
    client: Option<Arc<Mutex<tokio_postgres::Client>>>,
}

impl PostgresAdapter {
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            capabilities: DatabaseCapabilities::postgres(),
            config,
            client: None,
        }
    }

    pub fn from_config(config: serde_json::Value) -> Result<Self, AdapterError> {
        let db = config.get("db").ok_or_else(|| {
            AdapterError::Connection("Missing 'db' object in config".to_string())
        })?;

        let host = db.get("host").and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::Connection("Missing 'host'".to_string()))?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let username = db.get("username").and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::Connection("Missing 'username'".to_string()))?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        let database = db.get("database").and_then(|v| v.as_str());
        let use_tls = config.get("tls").and_then(|t| t.get("enabled")).and_then(|v| v.as_bool()).unwrap_or(false);

        let mut pg_config = PostgresConfig::new(host, username, password).with_port(port).with_tls(use_tls);
        if let Some(db_name) = database {
            pg_config = pg_config.with_database(db_name);
        }

        Ok(Self::new(pg_config))
    }

    fn client(&self) -> Result<Arc<Mutex<tokio_postgres::Client>>, AdapterError> {
        self.client.clone().ok_or_else(|| AdapterError::Connection("Not connected".to_string()))
    }

    fn map_postgres_type(raw: &str) -> String {
        let lower = raw.to_lowercase();
        if lower.contains("int") { "integer".to_string() }
        else if lower.contains("char") || lower.contains("text") { "text".to_string() }
        else if lower.contains("bool") { "boolean".to_string() }
        else if lower.contains("numeric") || lower.contains("decimal") { "decimal".to_string() }
        else if lower.contains("float") || lower.contains("double") || lower.contains("real") { "float".to_string() }
        else if lower.contains("date") { "date".to_string() }
        else if lower.contains("time") { "datetime".to_string() }
        else if lower.contains("json") { "json".to_string() }
        else if lower.contains("uuid") { "uuid".to_string() }
        else if lower.contains("bytea") { "binary".to_string() }
        else { "text".to_string() }
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    fn capabilities(&self) -> &DatabaseCapabilities {
        &self.capabilities
    }

    async fn connect(&mut self) -> Result<(), AdapterError> {
        info!("Connecting to PostgreSQL at {}:{}", self.config.host, self.config.port);
        
        let database = self.config.database.clone().unwrap_or_else(|| "postgres".to_string());
        let conn_str = self.config.connection_string(Some(&database));

        let client = if self.config.use_tls {
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| AdapterError::Connection(format!("TLS error: {}", e)))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
                .map_err(|e| AdapterError::Connection(format!("Connection error: {}", e)))?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres connection error: {}", e);
                }
            });
            client
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
                .map_err(|e| AdapterError::Connection(format!("Connection error: {}", e)))?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres connection error: {}", e);
                }
            });
            client
        };

        self.client = Some(Arc::new(Mutex::new(client)));
        debug!("PostgreSQL connection established");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.client = None;
        debug!("PostgreSQL connection closed");
        Ok(())
    }

    async fn list_databases(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[])
            .await.map_err(|e| AdapterError::Query(format!("Failed to list databases: {}", e)))?;

        Ok(rows.iter().map(|row| MetaDatabase {
            name: row.get(0),
            is_connected: false,
            is_introspected: false,
            schemas: vec![],
        }).collect())
    }

    async fn list_schemas(&self, _database: &str) -> Result<Vec<MetaSchema>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query("SELECT schema_name FROM information_schema.schemata ORDER BY schema_name", &[])
            .await.map_err(|e| AdapterError::Query(format!("Failed to list schemas: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let name: String = row.get(0);
            let schema_type = if matches!(name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") { "system" } else { "user" };
            MetaSchema { name, schema_type: schema_type.to_string(), is_introspected: false, tables: vec![] }
        }).collect())
    }

    async fn list_tables(&self, database: &str, schema: &str) -> Result<Vec<MetaTable>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query(
            "SELECT table_name, table_type FROM information_schema.tables WHERE table_schema = $1 AND table_type IN ('BASE TABLE', 'VIEW') ORDER BY table_name",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list tables: {}", e)))?;

        let now = chrono::Utc::now().timestamp_millis();

        Ok(rows.iter().map(|row| {
            let name: String = row.get(0);
            let type_str: String = row.get(1);
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };
            
            MetaTable {
                connection_id: String::new(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            }
        }).collect())
    }

    async fn list_columns(&self, table: &TableRef) -> Result<Vec<MetaColumn>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query(
            "SELECT ordinal_position, column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_schema = $1 AND table_name = $2 ORDER BY ordinal_position",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list columns: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let raw_type: String = row.get(2);
            let nullable: String = row.get(3);
            MetaColumn {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                ordinal_position: row.get::<_, i32>(0),
                column_name: row.get(1),
                raw_type: raw_type.clone(),
                logical_type: Self::map_postgres_type(&raw_type),
                nullable: nullable == "YES",
                default_value: row.get(4),
                is_primary_key: false,
            }
        }).collect())
    }

    async fn list_indexes(&self, table: &TableRef) -> Result<Vec<MetaIndex>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query(
            "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = $1 AND tablename = $2",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list indexes: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let index_name: String = row.get(0);
            let indexdef: String = row.get(1);
            MetaIndex {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                index_name,
                is_unique: indexdef.to_uppercase().contains("UNIQUE"),
            }
        }).collect())
    }

    async fn list_foreign_keys(&self, table: &TableRef) -> Result<Vec<MetaForeignKey>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query(
            "SELECT con.conname, att.attname, ref_sch.nspname, ref_tab.relname, ref_att.attname, ordinality
             FROM pg_constraint con
             JOIN pg_class tab ON con.conrelid = tab.oid
             JOIN pg_namespace sch ON tab.relnamespace = sch.oid
             JOIN pg_class ref_tab ON con.confrelid = ref_tab.oid
             JOIN pg_namespace ref_sch ON ref_tab.relnamespace = ref_sch.oid
             CROSS JOIN LATERAL unnest(con.conkey, con.confkey) WITH ORDINALITY AS cols(attnum, ref_attnum, ordinality)
             JOIN pg_attribute att ON att.attrelid = tab.oid AND att.attnum = cols.attnum
             JOIN pg_attribute ref_att ON ref_att.attrelid = ref_tab.oid AND ref_att.attnum = cols.ref_attnum
             WHERE con.contype = 'f' AND sch.nspname = $1 AND tab.relname = $2
             ORDER BY ordinality",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list foreign keys: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let constraint_name: String = row.get(0);
            let column_name: String = row.get(1);
            let ref_schema: String = row.get(2);
            let ref_table: String = row.get(3);
            let ref_column: String = row.get(4);
            let seq_no: i64 = row.get(5);
            let hash = compute_fk_hash(&table.name, &column_name, &ref_table, &ref_column);

            MetaForeignKey {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                column_name,
                ref_schema,
                ref_table,
                ref_column,
                constraint_name: Some(constraint_name),
                constraint_hash: hash,
                seq_no: seq_no as i32,
            }
        }).collect())
    }

    async fn list_triggers(&self, table: &TableRef) -> Result<Vec<MetaTrigger>, AdapterError> {
        let client_arc = self.client()?;
        let client = client_arc.lock().await;

        let rows = client.query(
            "SELECT trigger_name, event_manipulation, action_timing FROM information_schema.triggers WHERE event_object_schema = $1 AND event_object_table = $2",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list triggers: {}", e)))?;

        Ok(rows.iter().map(|row| MetaTrigger {
            connection_id: String::new(),
            database: table.database.clone(),
            schema: table.schema.clone(),
            table_name: table.name.clone(),
            trigger_name: row.get(0),
            event: row.get(1),
            timing: row.get(2),
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postgres_config() {
        let config = PostgresConfig::new("localhost", "user", "pass").with_port(5433).with_database("mydb");
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5433);
        assert_eq!(config.database, Some("mydb".to_string()));
    }

    #[test]
    fn test_connection_string() {
        let config = PostgresConfig::new("localhost", "user", "pass").with_port(5432).with_database("mydb");
        assert_eq!(config.connection_string(None), "postgres://user:pass@localhost:5432/mydb");
    }

    #[test]
    fn test_postgres_capabilities() {
        let adapter = PostgresAdapter::new(PostgresConfig::new("localhost", "user", "pass"));
        let caps = adapter.capabilities();
        assert_eq!(caps.engine, "postgres");
        assert!(caps.supports_schemas);
    }

    #[test]
    fn test_type_mapping() {
        assert_eq!(PostgresAdapter::map_postgres_type("integer"), "integer");
        assert_eq!(PostgresAdapter::map_postgres_type("varchar(255)"), "text");
        assert_eq!(PostgresAdapter::map_postgres_type("jsonb"), "json");
    }
}
