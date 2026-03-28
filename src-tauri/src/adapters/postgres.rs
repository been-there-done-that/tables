//! PostgreSQL Database Adapter
//!
//! Implements the `DatabaseAdapter` trait for PostgreSQL databases.
//! Supports full database/schema hierarchy with async tokio-postgres.

use async_trait::async_trait;
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Client;

use crate::adapter::{
    AdapterError, DatabaseAdapter, DatabaseCapabilities, TableRef,
};
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaFunction, MetaIndex, MetaSchema, MetaSequence,
    MetaConstraint, MetaTable, MetaTrigger, FunctionKind, ConstraintKind,
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

    /// Convert to tokio_postgres::Config for secure connection string building.
    fn to_pg_config(&self, database: &str) -> tokio_postgres::Config {
        let mut config = tokio_postgres::Config::new();
        config.host(&self.host);
        config.port(self.port);
        config.user(&self.username);
        config.password(&self.password);
        config.dbname(database);
        
        // Stale connection / Network Drop protections
        config.connect_timeout(std::time::Duration::from_secs(15));
        config.keepalives(true);
        config.keepalives_idle(std::time::Duration::from_secs(30));
        
        config
    }
}

/// Internal state for the Postgres adapter.
/// Holds the active client and the name of the connected database.
struct PgState {
    client: Client,
    database: String,
}

/// PostgreSQL database adapter
pub struct PostgresAdapter {
    capabilities: DatabaseCapabilities,
    config: PostgresConfig,
    /// Unified state protected by a single lock to prevent race conditions.
    state: Arc<Mutex<Option<PgState>>>,
}

impl PostgresAdapter {
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            capabilities: DatabaseCapabilities::postgres(),
            config,
            state: Arc::new(Mutex::new(None)),
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

    /// Ensure valid connection to the specified database.
    /// Reconnects if necessary.
    async fn ensure_connected(&self, database: &str) -> Result<(), AdapterError> {
        let mut state_guard = self.state.lock().await;

        // Check if we are already connected to the requested database
        if let Some(state) = state_guard.as_ref() {
            if state.database == database && !state.client.is_closed() {
                return Ok(());
            }
            if state.client.is_closed() {
                debug!("Postgres connection to '{}' is closed, reconnecting...", state.database);
            }
        }

        // Need to connect or switch
        debug!("Connecting/Switching Postgres database to '{}'", database);
        let config = self.config.to_pg_config(database);

        let new_client = if self.config.use_tls {
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| AdapterError::Connection(format!("TLS error: {}", e)))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = config.connect(connector).await
                .map_err(|e| AdapterError::Connection(format!("Connection error: {}", e)))?;
            
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres connection error: {}", e);
                }
            });
            client
        } else {
            let (client, connection) = config.connect(tokio_postgres::NoTls).await
                .map_err(|e| AdapterError::Connection(format!("Connection error: {}", e)))?;
            
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres connection error: {}", e);
                }
            });
            client
        };

        *state_guard = Some(PgState {
            client: new_client,
            database: database.to_string(),
        });

        Ok(())
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

    fn normalize_postgres(meta: &crate::schema_types::PostgresTypeMeta) -> crate::schema_types::NormalizedType {
        use crate::schema_types::{NormalizedType, IntegerSize, FloatPrecision};

        // 1. Arrays
        if meta.is_array {
            // Recursively normalize the element type
            let mut element_meta = meta.clone();
            element_meta.is_array = false; // "Peel" the array layer
            return NormalizedType::Array { 
                element: Box::new(Self::normalize_postgres(&element_meta)) 
            };
        }

        // 2. Domains (use base type if available)
        if meta.type_kind == 'd' {
             if let Some(ref base) = meta.base_type {
                 let base_meta = crate::schema_types::PostgresTypeMeta {
                     raw_type: base.clone(),
                     base_type: None,
                     type_kind: 'b', 
                     type_category: meta.type_category, 
                     is_array: false,
                     enum_values: None,
                 };
                 return Self::normalize_postgres(&base_meta);
             }
        }

        // 3. Enums
        if meta.type_kind == 'e' {
            return NormalizedType::Enum { 
                values: meta.enum_values.clone().unwrap_or_default() 
            };
        }

        // 4. Composite types (row types) - typtype = 'c'
        if meta.type_kind == 'c' {
            // Return as Composite with empty fields (field introspection requires separate query)
            return NormalizedType::Composite { fields: vec![] };
        }

        // 5. Range types (Postgres 9.2+) - typtype = 'r'
        if meta.type_kind == 'r' {
            // Infer element type from range type name
            let element = Self::infer_range_element(&meta.raw_type);
            return NormalizedType::Range { element: Box::new(element) };
        }

        // 6. Multirange types (Postgres 14+) - typtype = 'm'
        if meta.type_kind == 'm' {
            // Infer element type from multirange type name
            let element = Self::infer_range_element(&meta.raw_type);
            return NormalizedType::MultiRange { element: Box::new(element) };
        }

        // 7. Scalar Mapping
        match meta.raw_type.as_str() {
            "int2" | "smallint" => NormalizedType::Integer { size: IntegerSize::Small, unsigned: false },
            "int4" | "integer" | "serial" | "serial4" => NormalizedType::Integer { size: IntegerSize::Normal, unsigned: false },
            "int8" | "bigint" | "bigserial" | "serial8" => NormalizedType::Integer { size: IntegerSize::Big, unsigned: false },
            "float4" | "real" => NormalizedType::Float { precision: FloatPrecision::Single },
            "float8" | "double precision" => NormalizedType::Float { precision: FloatPrecision::Double },
            "numeric" | "decimal" | "money" => NormalizedType::Decimal,
            "bool" | "boolean" => NormalizedType::Boolean,
            "text" | "varchar" | "char" | "bpchar" | "name" => NormalizedType::Text,
            "citext" => NormalizedType::Text, // citext gets semantic hint, not special normalized type
            "bytea" => NormalizedType::Binary,
            "date" => NormalizedType::Date,
            "time" | "time without time zone" => NormalizedType::Time,
            "timetz" | "time with time zone" => NormalizedType::Time, 
            "timestamp" | "timestamp without time zone" => NormalizedType::DateTime { timezone: false },
            "timestamptz" | "timestamp with time zone" => NormalizedType::DateTime { timezone: true },
            "interval" => NormalizedType::Interval,
            "json" | "jsonb" => NormalizedType::Json,
            "uuid" => NormalizedType::Uuid,
            // Range types by name (fallback if typtype not available)
            "int4range" | "int8range" | "numrange" | "tsrange" | "tstzrange" | "daterange" => {
                let element = Self::infer_range_element(&meta.raw_type);
                NormalizedType::Range { element: Box::new(element) }
            }
            "int4multirange" | "int8multirange" | "nummultirange" | "tsmultirange" | "tstzmultirange" | "datemultirange" => {
                let element = Self::infer_range_element(&meta.raw_type);
                NormalizedType::MultiRange { element: Box::new(element) }
            }
            _ => match meta.type_category {
                'N' => NormalizedType::Decimal, 
                'S' => NormalizedType::Text,  
                'B' => NormalizedType::Boolean,
                'D' => NormalizedType::DateTime { timezone: false },
                'R' => {
                    // Range category fallback
                    let element = Self::infer_range_element(&meta.raw_type);
                    NormalizedType::Range { element: Box::new(element) }
                }
                _ => NormalizedType::Custom { name: meta.raw_type.clone() },
            }
        }
    }

    /// Infer the element type from a range/multirange type name
    fn infer_range_element(type_name: &str) -> crate::schema_types::NormalizedType {
        use crate::schema_types::{NormalizedType, IntegerSize};
        
        let name = type_name.to_lowercase();
        if name.contains("int4") {
            NormalizedType::Integer { size: IntegerSize::Normal, unsigned: false }
        } else if name.contains("int8") {
            NormalizedType::Integer { size: IntegerSize::Big, unsigned: false }
        } else if name.contains("num") {
            NormalizedType::Decimal
        } else if name.contains("tstz") {
            NormalizedType::DateTime { timezone: true }
        } else if name.contains("ts") {
            NormalizedType::DateTime { timezone: false }
        } else if name.contains("date") {
            NormalizedType::Date
        } else {
            NormalizedType::Unknown
        }
    }

    /// Derive semantic hint from type name and installed extensions.
    /// Extension presence enables semantic classification, but does not force it.
    fn semantic_hint_from_extension(
        type_name: &str,
        extensions: &[crate::schema_types::ExtensionInfo],
    ) -> Option<crate::schema_types::SemanticHint> {
        use crate::schema_types::SemanticHint;
        
        let type_lower = type_name.to_lowercase();
        
        // Check for extension-provided types
        let has_extension = |name: &str| extensions.iter().any(|e| e.name == name);
        
        // citext: case-insensitive text
        if type_lower == "citext" && has_extension("citext") {
            return Some(SemanticHint::CaseInsensitiveText);
        }
        
        // PostGIS: spatial types
        if (type_lower == "geometry" || type_lower == "geography") && has_extension("postgis") {
            return Some(SemanticHint::Spatial);
        }
        
        // pgvector: embedding vectors
        if type_lower.starts_with("vector") && has_extension("vector") {
            // Try to extract dimensions from vector(N)
            let dimensions = type_name
                .strip_prefix("vector(")
                .and_then(|s| s.strip_suffix(")"))
                .and_then(|s| s.parse::<usize>().ok());
            return Some(SemanticHint::Embedding { dimensions });
        }
        
        // ltree: hierarchical labels
        if type_lower == "ltree" && has_extension("ltree") {
            return Some(SemanticHint::Hierarchy);
        }
        
        // hstore: key-value pairs
        if type_lower == "hstore" && has_extension("hstore") {
            return Some(SemanticHint::KeyValue);
        }
        
        // Standard semantic hints (no extension required)
        match type_lower.as_str() {
            "uuid" => Some(SemanticHint::Uuid),
            "json" | "jsonb" => Some(SemanticHint::Json),
            "date" => Some(SemanticHint::Date),
            "time" | "timetz" => Some(SemanticHint::Time),
            "timestamp" | "timestamptz" => Some(SemanticHint::DateTime),
            "bool" | "boolean" => Some(SemanticHint::Boolean),
            "money" | "numeric" | "decimal" => Some(SemanticHint::Decimal),
            _ => None,
        }
    }

    /// Convert a PostgreSQL row value to JSON using shared utilities
    fn pg_value_to_json(row: &tokio_postgres::Row, idx: usize) -> serde_json::Value {
        let col = &row.columns()[idx];
        crate::pg_utils::pg_value_to_json(row, idx, col)
    }
}

#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    fn capabilities(&self) -> &DatabaseCapabilities {
        &self.capabilities
    }

    async fn connect(&self) -> Result<(), AdapterError> {
        let database = self.config.database.clone().unwrap_or_else(|| "postgres".to_string());
        self.ensure_connected(&database).await
    }

    // Updated is_connected to check real connection status
    fn is_connected(&self) -> bool {
        let state = self.state.try_lock();
        match state {
            Ok(guard) => {
                if let Some(s) = guard.as_ref() {
                    !s.client.is_closed()
                } else {
                    false
                }
            }
            Err(_) => true, // Still busy, assume connected (optimistic)
        }
    }

    async fn disconnect(&self) -> Result<(), AdapterError> {
        let mut state_guard = self.state.lock().await;
        *state_guard = None;
        debug!("PostgreSQL connection closed");
        Ok(())
    }

    async fn ensure_database(&self, database: Option<&str>) -> Result<(), AdapterError> {
        let db = database.unwrap_or("postgres");
        self.ensure_connected(db).await
    }

    fn get_pg_cancel_token(&self) -> Option<tokio_postgres::CancelToken> {
        let state = self.state.try_lock().ok()?;
        state.as_ref().map(|s| s.client.cancel_token())
    }

    async fn query(&self, query_str: &str) -> Result<crate::adapter::AdapterQueryResult, AdapterError> {
        let state_guard = self.state.lock().await;
        let state = state_guard.as_ref()
            .ok_or_else(|| AdapterError::Connection("Not connected".to_string()))?;
        
        let rows = state.client.query(query_str, &[]).await
            .map_err(|e| AdapterError::Query(crate::pg_utils::format_postgres_error(&e)))?;
        
        // Extract column info
        let columns: Vec<crate::adapter::AdapterColumnInfo> = if !rows.is_empty() {
            rows[0].columns().iter().map(|col| {
                crate::adapter::AdapterColumnInfo {
                    name: col.name().to_string(),
                    column_type: format!("{:?}", col.type_()),
                }
            }).collect()
        } else {
            vec![]
        };
        
        // Convert rows to JSON
        let json_rows: Vec<serde_json::Value> = rows.iter().map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in row.columns().iter().enumerate() {
                let value = Self::pg_value_to_json(row, i);
                obj.insert(col.name().to_string(), value);
            }
            serde_json::Value::Object(obj)
        }).collect();
        
        Ok(crate::adapter::AdapterQueryResult {
            rows: json_rows,
            columns,
            affected_rows: None,
        })
    }

    async fn execute(&self, statement: &str) -> Result<u64, AdapterError> {
        let state_guard = self.state.lock().await;
        let state = state_guard.as_ref()
            .ok_or_else(|| AdapterError::Connection("Not connected".to_string()))?;
        
        let affected = state.client.execute(statement, &[]).await
            .map_err(|e| AdapterError::Query(crate::pg_utils::format_postgres_error(&e)))?;
        
        Ok(affected)
    }

    async fn list_databases(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        self.ensure_connected("postgres").await?;
        let state_guard = self.state.lock().await;
        // Unwrap logic: ensure_connected ensures it's Some
        let client = &state_guard.as_ref().unwrap().client;

        let rows = client.query(
            "SELECT datname FROM pg_database 
             WHERE datistemplate = false 
               AND datname NOT IN ('rdsadmin', 'template0', 'template1')
             ORDER BY datname",
            &[]
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list databases: {}", e)))?;

        Ok(rows.iter().map(|row| MetaDatabase {
            name: row.get(0),
            is_connected: false,
            is_introspected: false,
            schemas: vec![],
        }).collect())
    }

    async fn list_schemas(&self, database: &str) -> Result<Vec<MetaSchema>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // 1. Schemas (per database) - Golden Set
        let rows = client.query(
            "SELECT
              n.nspname  AS schema_name,
              CASE
                WHEN n.nspname IN ('pg_catalog', 'information_schema') THEN 'system'
                WHEN n.nspname LIKE 'pg_%' THEN 'system'
                ELSE 'user'
              END AS schema_type
            FROM pg_namespace n
            ORDER BY n.nspname",
            &[]
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list schemas: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let name: String = row.get(0);
            let schema_type: String = row.get(1);
            MetaSchema {
                name,
                schema_type,
                kind: crate::schema_types::NamespaceKind::Schema,
                is_introspected: false,
                tables: vec![],
                functions: vec![],
                sequences: vec![],
            }
        }).collect())
    }

    async fn list_tables(&self, database: &str, schema: &str) -> Result<Vec<MetaTable>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // 2. Tables / Views / Materialized Views - Golden Set
        // Map relkind: r=table, p=partitioned table, v=view, m=materialized view
        let rows = client.query(
            "SELECT
              c.relname   AS table_name,
              c.relkind   AS relkind
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND c.relkind IN ('r', 'v', 'm', 'p')
            ORDER BY c.relname",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list tables: {}", e)))?;

        let now = chrono::Utc::now().timestamp_millis();

        Ok(rows.iter().map(|row| {
            let name: String = row.get(0);
            let relkind: i8 = row.get(1); // relkind is "char"
            let kind_char = relkind as u8 as char;
            
            let table_type = match kind_char {
                'r' | 'p' => "table",
                'v' | 'm' => "view", // Treat mat views as views for now
                _ => "table",
            };
            
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
                constraints: vec![],
            }
        }).collect())
    }

    async fn list_columns(&self, table: &TableRef) -> Result<Vec<MetaColumn>, AdapterError> {
        self.ensure_connected(&table.database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // 3. Columns (table-scoped) - Golden Set
        let rows = client.query(
            "SELECT
              a.attnum                      AS ordinal_position,
              a.attname                     AS column_name,
              pg_catalog.format_type(a.atttypid, a.atttypmod) AS raw_type,
              NOT a.attnotnull               AS is_nullable,
              pg_get_expr(ad.adbin, ad.adrelid) AS default_value
            FROM pg_attribute a
            JOIN pg_class c ON c.oid = a.attrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            LEFT JOIN pg_attrdef ad
              ON ad.adrelid = c.oid
             AND ad.adnum = a.attnum
            WHERE n.nspname = $1
              AND c.relname = $2
              AND a.attnum > 0
              AND NOT a.attisdropped
            ORDER BY a.attnum",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list columns: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let raw_type: String = row.get(2);
            let is_nullable: bool = row.get(3);
            let default_val: Option<String> = row.get(4);
            
            MetaColumn {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                ordinal_position: row.get::<_, i16>(0) as i32, // attnum is int2 (i16)
                column_name: row.get(1),
                raw_type: raw_type.clone(),
                logical_type: Self::map_postgres_type(&raw_type),
                nullable: is_nullable,
                default_value: default_val,
                is_primary_key: false, // Introspected separately via indexes/constraints if needed, or pure metadata
                engine_type: None, // Legacy path: TODO update list_columns to use canonical query
                normalized_type: None,
            }
        }).collect())
    }

    async fn list_indexes(&self, table: &TableRef) -> Result<Vec<MetaIndex>, AdapterError> {
        self.ensure_connected(&table.database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // 6. Indexes (no parsing) - Golden Set (Adapted for Name lookup)
        let rows = client.query(
            "SELECT
              i.relname            AS index_name,
              ix.indisunique       AS is_unique,
              pg_get_indexdef(i.oid) AS index_def
            FROM pg_index ix
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
            WHERE n.nspname = $1
              AND t.relname = $2",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list indexes: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let index_name: String = row.get(0);
            let is_unique: bool = row.get(1);
            
            MetaIndex {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                index_name,
                is_unique,
                is_primary: false,
                index_type: "btree".to_string(),
                columns: vec![],
                predicate: None,
                definition: String::new(),
            }
        }).collect())
    }

    async fn list_foreign_keys(&self, table: &TableRef) -> Result<Vec<MetaForeignKey>, AdapterError> {
        self.ensure_connected(&table.database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // 5. Foreign Keys (cross-schema safe) - Golden Set (Adapted)
        let rows = client.query(
            "SELECT
              con.conname                 AS constraint_name,
              src_att.attname             AS column_name,
              ref_ns.nspname              AS ref_schema,
              ref_tab.relname             AS ref_table,
              ref_att.attname             AS ref_column,
              cols.ordinality             AS seq_no
            FROM pg_constraint con
            JOIN pg_class src_tab ON src_tab.oid = con.conrelid
            JOIN pg_namespace src_ns ON src_ns.oid = src_tab.relnamespace
            JOIN pg_class ref_tab ON ref_tab.oid = con.confrelid
            JOIN pg_namespace ref_ns ON ref_ns.oid = ref_tab.relnamespace
            JOIN unnest(con.conkey, con.confkey) WITH ORDINALITY
                 AS cols(src_attnum, ref_attnum, ordinality)
              ON TRUE
            JOIN pg_attribute src_att
              ON src_att.attrelid = src_tab.oid
             AND src_att.attnum = cols.src_attnum
            JOIN pg_attribute ref_att
              ON ref_att.attrelid = ref_tab.oid
             AND ref_att.attnum = cols.ref_attnum
            WHERE con.contype = 'f'
              AND src_ns.nspname = $1
              AND src_tab.relname = $2
            ORDER BY cols.ordinality",
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
        self.list_triggers_schema(&table.database, &table.schema).await.map(|triggers| {
            triggers.into_iter().filter(|t| t.table_name == table.name).collect()
        })
    }

    // =========================================================================
    // Bulk Optimizations
    // =========================================================================


    async fn list_columns_schema(&self, database: &str, schema: &str) -> Result<Vec<MetaColumn>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // Canonical Postgres Introspection Query
        let sql = "
        SELECT
            c.relname                                   AS table_name,
            a.attname                                   AS column_name,
            a.attnum                                    AS ordinal_position,

            -- Raw PostgreSQL type info (FIXED: use format_type for proper type representation with modifiers)
            pg_catalog.format_type(a.atttypid, a.atttypmod) AS raw_type,
            t.typtype                                   AS type_kind,       -- b=base, e=enum, d=domain, c=composite
            t.typcategory                               AS type_category,   -- numeric, string, boolean, etc.

            -- Array detection
            (t.typelem <> 0 AND t.typlen = -1)          AS is_array,

            -- Domain handling
            bt.typname                                  AS base_type,       -- underlying type for domains

            -- Nullability
            NOT a.attnotnull                            AS is_nullable,

            -- Default / identity
            pg_get_expr(ad.adbin, ad.adrelid)           AS column_default,
            a.attidentity                               AS identity_kind,   -- 'a' = always, 'd' = by default

            -- Enum values (if enum)
            CASE
                WHEN t.typtype = 'e' THEN
                    ARRAY(
                        SELECT e.enumlabel
                        FROM pg_enum e
                        WHERE e.enumtypid = t.oid
                        ORDER BY e.enumsortorder
                    )
                ELSE NULL
            END                                         AS enum_values

        FROM pg_attribute a
        JOIN pg_class c       ON c.oid = a.attrelid
        JOIN pg_namespace n   ON n.oid = c.relnamespace
        JOIN pg_type t        ON t.oid = a.atttypid
        LEFT JOIN pg_type bt  ON bt.oid = t.typbasetype
        LEFT JOIN pg_attrdef ad ON ad.adrelid = a.attrelid AND ad.adnum = a.attnum

        WHERE
            n.nspname = $1
            AND c.relkind IN ('r', 'p', 'v', 'f', 'm')  -- table, partition, view, foreign, mat view
            AND a.attnum > 0
            AND NOT a.attisdropped

        ORDER BY
            c.relname,
            a.attnum;
        ";

        let rows = client.query(sql, &[&schema])
            .await.map_err(|e| AdapterError::Query(format!("Failed to list schema columns: {}", e)))?;

        Ok(rows.iter().map(|row| {
             let table_name: String = row.get("table_name");
             let column_name: String = row.get("column_name");
             let ordinal: i16 = row.get("ordinal_position");
             
             // Extract metadata fields
             // Note: typtype/typcategory are \"char\" in postgres, mapping to i8 in rust-postgres
             let raw_type: String = row.get("raw_type");
             let type_kind_i8: i8 = row.get("type_kind");
             let type_category_i8: i8 = row.get("type_category");
             let is_array: bool = row.get("is_array");
             let base_type: Option<String> = row.get("base_type");
             let is_nullable: bool = row.get("is_nullable");
             let default_value: Option<String> = row.get("column_default");
             let enum_values: Option<Vec<String>> = row.get("enum_values");

             let type_kind = type_kind_i8 as u8 as char;
             let type_category = type_category_i8 as u8 as char;

             // Construct Engine Truth
             let pg_meta = crate::schema_types::PostgresTypeMeta {
                 raw_type: raw_type.clone(),
                 base_type,
                 type_kind,
                 type_category,
                 is_array,
                 enum_values,
             };

             // Construct Normalized Type
             let normalized = Self::normalize_postgres(&pg_meta);

             // Construct EngineType container
             let engine_type = crate::schema_types::EngineType {
                 engine: crate::schema_types::DatabaseEngine::Postgres,
                 raw_type: raw_type.clone(),
                 metadata: crate::schema_types::EngineTypeMeta::Postgres(pg_meta),
             };

             MetaColumn {
                 connection_id: String::new(),
                 database: database.to_string(),
                 schema: schema.to_string(),
                 table_name,
                 ordinal_position: ordinal as i32,
                 column_name,
                 raw_type: raw_type.clone(), // Legacy field
                 logical_type: Self::map_postgres_type(&raw_type), // Legacy field, eventually derive from normalized
                 engine_type: Some(engine_type),
                 normalized_type: Some(normalized),
                 nullable: is_nullable,
                 default_value,
                 is_primary_key: false,
             }
        }).collect())
    }

    async fn list_indexes_schema(&self, database: &str, schema: &str) -> Result<Vec<MetaIndex>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // Bulk Indexes Query
        let rows = client.query(
            "SELECT
              t.relname            AS table_name,
              i.relname            AS index_name,
              ix.indisunique       AS is_unique,
              pg_get_indexdef(i.oid) AS index_def
            FROM pg_index ix
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_class t ON t.oid = ix.indrelid
            JOIN pg_namespace n ON n.oid = t.relnamespace
            WHERE n.nspname = $1
              AND t.relkind IN ('r', 'm', 'p')",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list schema indexes: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let table_name: String = row.get(0);
            let index_name: String = row.get(1);
            let is_unique: bool = row.get(2);
            
            MetaIndex {
                connection_id: String::new(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name,
                index_name,
                is_unique,
                is_primary: false,
                index_type: "btree".to_string(),
                columns: vec![],
                predicate: None,
                definition: String::new(),
            }
        }).collect())
    }

    async fn list_foreign_keys_schema(&self, database: &str, schema: &str) -> Result<Vec<MetaForeignKey>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // Bulk FK Query
        let rows = client.query(
            "SELECT
              tab.relname                 AS table_name,
              con.conname                 AS constraint_name,
              src_att.attname             AS column_name,
              ref_ns.nspname              AS ref_schema,
              ref_tab.relname             AS ref_table,
              ref_att.attname             AS ref_column,
              cols.ordinality             AS seq_no
            FROM pg_constraint con
            JOIN pg_class tab ON con.conrelid = tab.oid
            JOIN pg_namespace sch ON tab.relnamespace = sch.oid
            JOIN pg_class ref_tab ON con.confrelid = ref_tab.oid
            JOIN pg_namespace ref_sch ON ref_tab.relnamespace = ref_sch.oid
            CROSS JOIN LATERAL unnest(con.conkey, con.confkey) WITH ORDINALITY AS cols(attnum, ref_attnum, ordinality)
            JOIN pg_attribute src_att ON src_att.attrelid = tab.oid AND src_att.attnum = cols.src_attnum
            JOIN pg_attribute ref_att ON ref_att.attrelid = ref_tab.oid AND ref_att.attnum = cols.ref_attnum
            WHERE con.contype = 'f' AND sch.nspname = $1
            ORDER BY tab.relname, cols.ordinality",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list schema foreign keys: {}", e)))?;

        Ok(rows.iter().map(|row| {
            let table_name: String = row.get(0);
            let constraint_name: String = row.get(1);
            let column_name: String = row.get(2);
            let ref_schema: String = row.get(3);
            let ref_table: String = row.get(4);
            let ref_column: String = row.get(5);
            let seq_no: i64 = row.get(6);
            let hash = compute_fk_hash(&table_name, &column_name, &ref_table, &ref_column);

            MetaForeignKey {
                connection_id: String::new(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name,
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

    async fn list_triggers_schema(&self, database: &str, schema: &str) -> Result<Vec<MetaTrigger>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        // Bulk Trigger Query with tgtype parsing
        let rows = client.query(
            "SELECT
              c.relname              AS table_name,
              tg.tgname              AS trigger_name,
              tg.tgenabled           AS enabled,
              tg.tgtype              AS tgtype
            FROM pg_trigger tg
            JOIN pg_class c ON c.oid = tg.tgrelid
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND NOT tg.tgisinternal",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list schema triggers: {}", e)))?;

        let mut triggers = Vec::new();

        for row in rows {
            let table_name: String = row.get(0);
            let trigger_name: String = row.get(1);
            let _enabled_char: i8 = row.get(2);
            let tgtype: i16 = row.get(3);
            
            let timing = if (tgtype & 2) != 0 {
                "BEFORE"
            } else if (tgtype & 64) != 0 {
                "INSTEAD OF"
            } else {
                "AFTER"
            };

            let mut events = Vec::new();
            if (tgtype & 4) != 0 { events.push("INSERT"); }
            if (tgtype & 8) != 0 { events.push("DELETE"); }
            if (tgtype & 16) != 0 { events.push("UPDATE"); }
            if (tgtype & 32) != 0 { events.push("TRUNCATE"); }

            for event in events {
                triggers.push(MetaTrigger {
                    connection_id: String::new(),
                    database: database.to_string(),
                    schema: schema.to_string(),
                    table_name: table_name.clone(),
                    trigger_name: trigger_name.clone(),
                    event: event.to_string(),
                    timing: timing.to_string(),
                });
            }
        }
        Ok(triggers)
    }

    async fn list_functions(&self, database: &str, schema: &str) -> Result<Vec<MetaFunction>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        let rows = client.query(
            "SELECT
                 p.oid::bigint,
                 p.proname,
                 l.lanname,
                 CASE p.prokind
                     WHEN 'f' THEN 'Function'
                     WHEN 'p' THEN 'Procedure'
                     WHEN 'a' THEN 'Aggregate'
                     WHEN 'w' THEN 'Window'
                     ELSE 'Function'
                 END,
                 COALESCE(pg_catalog.pg_get_function_result(p.oid), ''),
                 COALESCE(pg_catalog.pg_get_functiondef(p.oid), ''),
                 p.prosecdef,
                 CASE p.provolatile
                     WHEN 'i' THEN 'immutable'
                     WHEN 's' THEN 'stable'
                     ELSE 'volatile'
                 END
             FROM pg_proc p
             JOIN pg_namespace n ON p.pronamespace = n.oid
             JOIN pg_language l ON p.prolang = l.oid
             WHERE n.nspname = $1
             ORDER BY p.proname, p.oid",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list functions in {}: {}", schema, e)))?;

        let mut functions = Vec::new();
        for row in rows {
            let oid: i64 = row.get(0);
            let name: String = row.get(1);
            let language: String = row.get(2);
            let kind_str: String = row.get(3);
            let return_type: String = row.get(4);
            let definition: String = row.get(5);
            let security_definer: bool = row.get(6);
            let volatility: String = row.get(7);

            let kind = match kind_str.as_str() {
                "Procedure" => FunctionKind::Procedure,
                "Aggregate" => FunctionKind::Aggregate,
                "Window" => FunctionKind::Window,
                _ => FunctionKind::Function,
            };

            functions.push(MetaFunction {
                connection_id: String::new(),
                database: database.to_string(),
                schema: schema.to_string(),
                name,
                oid,
                language,
                kind,
                return_type,
                arguments: vec![],
                definition,
                security_definer,
                volatility,
            });
        }
        Ok(functions)
    }

    async fn list_sequences(&self, database: &str, schema: &str) -> Result<Vec<MetaSequence>, AdapterError> {
        self.ensure_connected(database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        let rows = client.query(
            "SELECT
                 sequencename,
                 data_type::text,
                 start_value,
                 min_value,
                 max_value,
                 increment_by,
                 cycle,
                 cache_size,
                 last_value
             FROM pg_sequences
             WHERE schemaname = $1
             ORDER BY sequencename",
            &[&schema],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list sequences in {}: {}", schema, e)))?;

        let mut sequences = Vec::new();
        for row in rows {
            sequences.push(MetaSequence {
                connection_id: String::new(),
                database: database.to_string(),
                schema: schema.to_string(),
                name: row.get(0),
                data_type: row.get(1),
                start_value: row.get(2),
                min_value: row.get(3),
                max_value: row.get(4),
                increment_by: row.get(5),
                cycle: row.get(6),
                cache_size: row.get(7),
                last_value: row.get(8),
            });
        }
        Ok(sequences)
    }

    async fn list_constraints(&self, table: &TableRef) -> Result<Vec<MetaConstraint>, AdapterError> {
        self.ensure_connected(&table.database).await?;
        let state_guard = self.state.lock().await;
        let client = &state_guard.as_ref().unwrap().client;

        let rows = client.query(
            "SELECT
                 c.conname,
                 CASE c.contype
                     WHEN 'p' THEN 'PrimaryKey'
                     WHEN 'f' THEN 'ForeignKey'
                     WHEN 'u' THEN 'Unique'
                     WHEN 'c' THEN 'Check'
                     WHEN 'x' THEN 'Exclusion'
                     ELSE 'Check'
                 END,
                 COALESCE(pg_get_constraintdef(c.oid), ''),
                 COALESCE(
                     (SELECT json_agg(a.attname ORDER BY array_position(c.conkey, a.attnum))
                      FROM pg_attribute a
                      WHERE a.attrelid = c.conrelid AND a.attnum = ANY(c.conkey)),
                     '[]'::json
                 )::text
             FROM pg_constraint c
             JOIN pg_class t ON t.oid = c.conrelid
             JOIN pg_namespace n ON n.oid = t.relnamespace
             WHERE n.nspname = $1 AND t.relname = $2
               AND c.contype IN ('p','f','u','c','x')
             ORDER BY c.contype, c.conname",
            &[&table.schema, &table.name],
        ).await.map_err(|e| AdapterError::Query(format!("Failed to list constraints for {}.{}: {}", table.schema, table.name, e)))?;

        let mut constraints = Vec::new();
        for row in rows {
            let name: String = row.get(0);
            let kind_str: String = row.get(1);
            let definition: String = row.get(2);
            let columns_json: String = row.get(3);

            let kind = match kind_str.as_str() {
                "PrimaryKey" => ConstraintKind::PrimaryKey,
                "ForeignKey" => ConstraintKind::ForeignKey,
                "Unique" => ConstraintKind::Unique,
                "Exclusion" => ConstraintKind::Exclusion,
                _ => ConstraintKind::Check,
            };

            let columns: Vec<String> = serde_json::from_str(&columns_json).unwrap_or_default();

            constraints.push(MetaConstraint {
                connection_id: String::new(),
                database: table.database.clone(),
                schema: table.schema.clone(),
                table_name: table.name.clone(),
                name,
                kind,
                definition,
                columns,
            });
        }
        Ok(constraints)
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
    fn test_to_pg_config() {
        let config = PostgresConfig::new("localhost", "user", "p@ssword").with_port(5432);
        // We probably can't easily inspect tokio_postgres::Config inner state without getters (some are unstable or limited),
        // mostly we rely on it working. Using manual string formatting test is no longer relevant for the internal impl.
        // We can just basic test that it compiles/runs.
        let _pg_config = config.to_pg_config("testdb");
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
