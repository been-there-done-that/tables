use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection as SqliteConnection};
use log::{info, debug, error};
use std::sync::{Arc, Mutex};
use chrono;
use tokio_postgres;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDatabase {
    pub name: String,
    pub is_connected: bool,
    pub is_introspected: bool, // New!
    pub schemas: Vec<MetaSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSchema {
    pub name: String,
    pub schema_type: String, // "user" or "system"
    pub is_introspected: bool,
    pub tables: Vec<MetaTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTable {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub table_type: String,
    pub classification: String,
    pub last_introspected_at: i64,
    pub columns: Vec<MetaColumn>,
    pub foreign_keys: Vec<MetaForeignKey>,
    pub indexes: Vec<MetaIndex>,
    pub triggers: Vec<MetaTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaColumn {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub ordinal_position: i32,
    pub column_name: String,
    pub raw_type: String,
    pub logical_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaIndex {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub index_name: String,
    pub is_unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaIndexColumn {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub index_name: String,
    pub column_name: String,
    pub seq_no: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaForeignKey {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub column_name: String,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_column: String,
    pub constraint_name: String,
    pub seq_no: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTrigger {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table_name: String,
    pub trigger_name: String,
    pub event: String,      // INSERT, UPDATE, DELETE
    pub timing: String,     // BEFORE, AFTER, INSTEAD OF
}

pub struct Introspector {
    app_db: Arc<Mutex<SqliteConnection>>,
}

impl Introspector {
    pub fn new(app_db: Arc<Mutex<SqliteConnection>>) -> Self {
        Self { app_db }
    }

    pub fn introspect_sqlite(&self, connection_id: &str, sqlite_path: &str) -> Result<Vec<MetaDatabase>, String> {
        info!("Starting SQLite introspection for connection {} at {}", connection_id, sqlite_path);
        
        // 1. Discovery (on target DB)
        let target_conn = SqliteConnection::open(sqlite_path).map_err(|e| e.to_string())?;
        
        // SQLite really only has "main" (and attached ones, but usually just main)
        // We will introspect "main"
        let mut tables = Vec::new();

        let mut stmt = target_conn.prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'")
            .map_err(|e| e.to_string())?;

        let table_iter = stmt.query_map([], |row| {
             Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())?;

        // App DB Connection
        let app_db = self.app_db.lock().unwrap();
        let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;

        // Clear existing metadata for this connection/schema before re-inserting?
        // For "Hard Refresh", yes, or we use UPSERT. UPSERT is safer for partial failures,
        // but explicit clear ensures no stale data. 
        // Let's stick to UPSERT logic we have, but maybe we want to fetch details immediately?
        // The V1 spec says: "Return Full Schema".
        // So we need to build the structs in memory AND save them.
        
        let now = chrono::Utc::now().timestamp_millis();
        
        for table_result in table_iter {
            let (name, ttype) = table_result.map_err(|e| e.to_string())?;
            let classification = "user"; // simple classification

            info!("Introspecting {} '{}'", ttype, name);

            // 1. Details
            let columns = self.introspect_columns_internal(&target_conn, connection_id, "main", "main", &name)?;
            let foreign_keys = self.introspect_foreign_keys_internal(&target_conn, connection_id, "main", "main", &name)?;
            let indexes = self.introspect_indexes_internal(&target_conn, connection_id, "main", "main", &name)?;

            let meta_table = MetaTable {
                connection_id: connection_id.to_string(),
                database: "main".to_string(),
                schema: "main".to_string(),
                table_name: name.clone(),
                table_type: ttype,
                classification: classification.to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys,
                indexes,
                triggers: vec![],  // SQLite triggers introspection TODO
            };

            // Save everything to DB (Cache)
            self.save_database(&tx, connection_id, "main")?;
            self.save_table_full(&tx, &meta_table)?;
            
            tables.push(meta_table);
        }

        tx.commit().map_err(|e| e.to_string())?;

        Ok(vec![MetaDatabase {
            name: "main".to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: vec![MetaSchema {
                name: "main".to_string(),
                schema_type: "user".to_string(),
                is_introspected: true,
                tables,
            }],
        }])
    }

    pub async fn introspect_database(&self, connection_id: &str, config: serde_json::Value, database_name: &str, app: &tauri::AppHandle) -> Result<MetaDatabase, String> {
        use tauri::Emitter;
        
        info!("On-demand progressive introspection for database {} on connection {}", database_name, connection_id);
        
        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");

        // Check TLS config
        let tls_enabled = config.get("tls")
            .and_then(|t| t.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Connect specifically to the requested database
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database_name);
        
        let client: tokio_postgres::Client = if tls_enabled {
            debug!("On-demand introspection with TLS enabled for database {}", database_name);
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
                .map_err(|e| {
                    error!("Postgres TLS on-demand connection failed: {:?}", e);
                    format!("Connection error: {}", e)
                })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres on-demand connection error: {}", e);
                }
            });
            client
        } else {
            debug!("On-demand introspection without TLS for database {}", database_name);
            let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
                .map_err(|e| {
                    error!("Postgres on-demand connection failed: {:?}", e);
                    format!("Connection error: {}", e)
                })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres on-demand connection error: {}", e);
                }
            });
            client
        };

        // === LEVEL 2: Schemas ===
        let schema_rows = client.query(
            "SELECT schema_name FROM information_schema.schemata WHERE catalog_name = $1",
            &[&database_name]
        ).await.map_err(|e| e.to_string())?;
        let all_schemas: Vec<String> = schema_rows.iter().map(|r| r.get(0)).collect();

        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for s_name in &all_schemas {
                let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                    "system"
                } else {
                    "user"
                };
                self.save_schema(&tx, connection_id, database_name, s_name, schema_type)?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 2,
            "connection_id": connection_id,
            "database": database_name,
            "schemas": &all_schemas,
        }));

        // === LEVEL 3: Tables + Columns (bulk) ===
        let table_rows = client.query(
            "SELECT table_schema, table_name, table_type 
             FROM information_schema.tables 
             WHERE table_type IN ('BASE TABLE', 'VIEW')", 
            &[]
        ).await.map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp_millis();

        let mut column_map = self.introspect_postgres_columns_bulk(&client, connection_id, database_name, &all_schemas).await?;

        let mut all_tables = Vec::new();
        for row in table_rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let type_str: String = row.get(2);
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };

            let columns = column_map.remove(&(schema.clone(), name.clone())).unwrap_or_default();

            all_tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: database_name.to_string(),
                schema,
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            });
        }

        // Save LEVEL 3 results
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for t in &all_tables {
                self.save_table(&tx, t.clone())?;
                for col in &t.columns {
                    self.save_column(&tx, col.clone())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 3,
            "connection_id": connection_id,
            "database": database_name,
        }));

        // Emit ready after level 3 so UI can unblock
        let _ = app.emit("schema:ready", serde_json::json!({
            "connection_id": connection_id,
            "database": database_name,
        }));

        // === LEVEL 4: FKs + Indexes + Triggers (bulk) ===
        let fk_map = self.introspect_postgres_foreign_keys_bulk(&client, connection_id, database_name, &all_schemas).await?;
        let idx_map = self.introspect_postgres_indexes_bulk(&client, connection_id, database_name, &all_schemas).await?;
        let trg_map = self.introspect_postgres_triggers_bulk(&client, connection_id, database_name, &all_schemas).await?;

        // Save LEVEL 4 results
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for t in &all_tables {
                let fks = fk_map.get(&(t.schema.clone(), t.table_name.clone())).cloned().unwrap_or_default();
                for fk in fks {
                    self.save_foreign_key(&tx, fk)?;
                }
                
                let idxs = idx_map.get(&(t.schema.clone(), t.table_name.clone())).cloned().unwrap_or_default();
                for idx in idxs {
                    self.save_index(&tx, idx.clone())?;
                    // Note: indexes bulk method returns MetaIndex which doesn't include columns currently in the simplified bulk?
                    // Wait, let's check introspect_postgres_indexes_bulk.
                }

                let trgs = trg_map.get(&(t.schema.clone(), t.table_name.clone())).cloned().unwrap_or_default();
                for trg in trgs {
                    self.save_trigger(&tx, trg)?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 4,
            "connection_id": connection_id,
            "database": database_name,
        }));

        // Construct return value
        let mut schema_vec = Vec::new();
        let mut grouped_tables: std::collections::HashMap<String, Vec<MetaTable>> = std::collections::HashMap::new();
        for t in all_tables {
            grouped_tables.entry(t.schema.clone()).or_default().push(t);
        }

        for (s_name, s_tables) in grouped_tables {
            let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                "system"
            } else {
                "user"
            };
            schema_vec.push(MetaSchema {
                name: s_name,
                schema_type: schema_type.to_string(),
                is_introspected: true,
                tables: s_tables,
            });
        }
        schema_vec.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: schema_vec,
        })
    }

    // Internal introspection methods that return data instead of just saving
    fn introspect_columns_internal(&self, target_conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaColumn>, String> {
        let mut stmt = target_conn.prepare(&format!("SELECT cid, \"name\", \"type\", \"notnull\", dflt_value, pk FROM pragma_table_xinfo('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let cid: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let raw_type: String = row.get(2)?;
            let notnull: i32 = row.get(3)?;
            let dflt_value: Option<String> = row.get(4)?;
            let pk: i32 = row.get(5)?;
            
            let logical_type = raw_type.clone(); // simplifiction

            Ok(MetaColumn {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                ordinal_position: cid,
                column_name: name,
                raw_type,
                logical_type,
                nullable: notnull == 0,
                default_value: dflt_value,
                is_primary_key: pk > 0,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn infer_logical_type(&self, raw_type: &str) -> String {
        let rt = raw_type.to_uppercase();
        if rt.contains("INT") {
            "int".to_string()
        } else if rt.contains("REAL") || rt.contains("FLOAT") || rt.contains("DOUBLE") {
            "float".to_string()
        } else if rt.contains("JSON") {
            "json".to_string()
        } else if rt.contains("BOOL") {
            "boolean".to_string()
        } else if rt.contains("DATE") {
            "date".to_string()
        } else if rt.contains("TIME") {
            "timestamp".to_string()
        } else if rt.contains("TEXT") || rt.contains("CHAR") || rt.contains("CLOB") {
            "text".to_string()
        } else {
            "text".to_string()
        }
    }

    fn introspect_foreign_keys_internal(&self, target_conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaForeignKey>, String> {
        let mut stmt = target_conn.prepare(&format!("SELECT \"id\", \"seq\", \"table\", \"from\", \"to\" FROM pragma_foreign_key_list('{}') ORDER BY \"id\", \"seq\"", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let id: i32 = row.get(0)?;
            let seq: i32 = row.get(1)?;
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(3)?,
                ref_schema: "main".to_string(),
                ref_table: row.get(2)?,
                ref_column: row.get(4)?,
                constraint_name: format!("fk_{}_{}", table_name, id),
                seq_no: seq + 1,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    fn introspect_indexes_internal(&self, target_conn: &SqliteConnection, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaIndex>, String> {
        let mut stmt = target_conn.prepare(&format!("SELECT \"name\", \"unique\" FROM pragma_index_list('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let index_name: String = row.get(0)?;
            let is_unique = row.get::<_, i32>(1)? != 0;
            
            if index_name.starts_with("sqlite_autoindex_") {
                 return Ok(None);
            }

            Ok(Some(MetaIndex {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                index_name,
                is_unique,
            }))
        }).map_err(|e| e.to_string())?;

        let mut indexes = Vec::new();
        for r in rows {
            if let Some(idx) = r.map_err(|e| e.to_string())? {
                indexes.push(idx);
            }
        }
        Ok(indexes)
    }

    // Persistence Helpers
    fn clear_cache(&self, conn: &SqliteConnection, connection_id: &str) -> Result<(), String> {
        conn.execute("DELETE FROM meta_tables WHERE connection_id = ?1", params![connection_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_table_full(&self, tx: &SqliteConnection, table: &MetaTable) -> Result<(), String> {
        self.save_table(tx, table.clone())?;
        for col in &table.columns {
            self.save_column(tx, col.clone())?;
        }
        for fk in &table.foreign_keys {
            self.save_foreign_key(tx, fk.clone())?;
        }
        for idx in &table.indexes {
            self.save_index(tx, idx.clone())?;
        }
        for trigger in &table.triggers {
            self.save_trigger(tx, trigger.clone())?;
        }
        Ok(())
    }

    fn save_database(&self, conn: &SqliteConnection, connection_id: &str, name: &str) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_databases (connection_id, name) VALUES (?1, ?2)",
            params![connection_id, name]
        ).map_err(|e| format!("Failed to save database '{}' to local cache: {}", name, e))?;
        Ok(())
    }

    fn save_schema(&self, conn: &SqliteConnection, connection_id: &str, database: &str, name: &str, schema_type: &str) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_schemas (connection_id, database, name, schema_type) VALUES (?1, ?2, ?3, ?4)",
            params![connection_id, database, name, schema_type]
        ).map_err(|e| format!("Failed to save schema '{}.{}' to local cache: {}", database, name, e))?;
        Ok(())
    }

    fn save_table(&self, conn: &SqliteConnection, table: MetaTable) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_tables (connection_id, database, schema, table_name, type, classification, last_introspected_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(connection_id, database, schema, table_name) DO UPDATE SET
               type=excluded.type,
               classification=excluded.classification,
               last_introspected_at=excluded.last_introspected_at",
            params![table.connection_id, table.database, table.schema, table.table_name, table.table_type, table.classification, table.last_introspected_at]
        ).map_err(|e| format!("Failed to save table '{}.{}.{}' to local cache: {}", table.database, table.schema, table.table_name, e))?;
        Ok(())
    }

    fn save_column(&self, conn: &SqliteConnection, col: MetaColumn) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_columns (connection_id, database, schema, table_name, ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![col.connection_id, col.database, col.schema, col.table_name, col.ordinal_position, col.column_name, col.raw_type, col.logical_type, col.nullable as i32, col.default_value, col.is_primary_key as i32]
        ).map_err(|e| format!("Failed to save column '{}.{}.{}.{}' to local cache: {}", col.database, col.schema, col.table_name, col.column_name, e))?;
        Ok(())
    }

    fn save_foreign_key(&self, conn: &SqliteConnection, fk: MetaForeignKey) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_foreign_keys (connection_id, database, schema, table_name, column_name, ref_schema, ref_table, ref_column, constraint_name, seq_no) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(connection_id, database, schema, table_name, constraint_name, seq_no) DO UPDATE SET
               column_name=excluded.column_name,
               ref_schema=excluded.ref_schema,
               ref_table=excluded.ref_table,
               ref_column=excluded.ref_column",
            params![fk.connection_id, fk.database, fk.schema, fk.table_name, fk.column_name, fk.ref_schema, fk.ref_table, fk.ref_column, fk.constraint_name, fk.seq_no]
        ).map_err(|e| format!("[SQLITE_FK_SAVE] Failed to save foreign key '{}' ({}.{}.{}) to local cache: {}", fk.constraint_name, fk.database, fk.schema, fk.table_name, e))?;
        Ok(())
    }

    fn save_index(&self, conn: &SqliteConnection, idx: MetaIndex) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_indexes (connection_id, database, schema, table_name, index_name, is_unique) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![idx.connection_id, idx.database, idx.schema, idx.table_name, idx.index_name, idx.is_unique as i32]
        ).map_err(|e| format!("[SQLITE_IDX_SAVE] Failed to save index '{}' to local cache: {}", idx.index_name, e))?;
        Ok(())
    }

    fn save_index_column(&self, conn: &SqliteConnection, col: MetaIndexColumn) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_index_columns (connection_id, database, schema, table_name, index_name, column_name, seq_no) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![col.connection_id, col.database, col.schema, col.table_name, col.index_name, col.column_name, col.seq_no]
        ).map_err(|e| format!("Failed to save index column '{}' for index '{}' to local cache: {}", col.column_name, col.index_name, e))?;
        Ok(())
    }

    fn save_trigger(&self, conn: &SqliteConnection, trigger: MetaTrigger) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_triggers (connection_id, database, schema, table_name, trigger_name, event, timing) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![trigger.connection_id, trigger.database, trigger.schema, trigger.table_name, trigger.trigger_name, trigger.event, trigger.timing]
        ).map_err(|e| format!("[SQLITE_TRG_SAVE] Failed to save trigger '{}' to local cache: {}", trigger.trigger_name, e))?;
        Ok(())
    }

    // API Helpers
    pub fn get_tables(&self, connection_id: &str) -> Result<Vec<MetaTable>, String> {
        let conn = self.app_db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT connection_id, database, schema, table_name, type, classification, last_introspected_at FROM meta_tables WHERE connection_id = ?1 ORDER BY database, schema, table_name")
            .map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(params![connection_id], |row| {
            Ok(MetaTable {
                connection_id: row.get(0)?,
                database: row.get(1)?,
                schema: row.get(2)?,
                table_name: row.get(3)?,
                table_type: row.get(4)?,
                classification: row.get(5)?,
                last_introspected_at: row.get(6)?,
                columns: vec![],  // Will be populated below
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            })
        }).map_err(|e| e.to_string())?;

        let mut tables: Vec<MetaTable> = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        
        // Load columns for each table
        for table in &mut tables {
            let mut col_stmt = conn.prepare(
                "SELECT ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key 
                 FROM meta_columns 
                 WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 
                 ORDER BY ordinal_position"
            ).map_err(|e| e.to_string())?;
            
            let columns = col_stmt.query_map(
                params![&table.connection_id, &table.database, &table.schema, &table.table_name],
                |row| {
                    Ok(MetaColumn {
                        connection_id: table.connection_id.clone(),
                        database: table.database.clone(),
                        schema: table.schema.clone(),
                        table_name: table.table_name.clone(),
                        ordinal_position: row.get(0)?,
                        column_name: row.get(1)?,
                        raw_type: row.get(2)?,
                        logical_type: row.get(3)?,
                        nullable: row.get::<_, i32>(4)? != 0,
                        default_value: row.get(5)?,
                        is_primary_key: row.get::<_, i32>(6)? != 0,
                    })
                }
            ).map_err(|e| e.to_string())?;
            
            table.columns = columns.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

            // Load foreign keys
            let mut fk_stmt = conn.prepare(
                "SELECT column_name, ref_schema, ref_table, ref_column, constraint_name, seq_no
                 FROM meta_foreign_keys 
                 WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4
                 ORDER BY constraint_name, seq_no"
            ).map_err(|e| e.to_string())?;
            
            let fks = fk_stmt.query_map(
                params![&table.connection_id, &table.database, &table.schema, &table.table_name],
                |row| {
                    Ok(MetaForeignKey {
                        connection_id: table.connection_id.clone(),
                        database: table.database.clone(),
                        schema: table.schema.clone(),
                        table_name: table.table_name.clone(),
                        column_name: row.get(0)?,
                        ref_schema: row.get(1)?,
                        ref_table: row.get(2)?,
                        ref_column: row.get(3)?,
                        constraint_name: row.get(4)?,
                        seq_no: row.get(5)?,
                    })
                }
            ).map_err(|e| e.to_string())?;
            
            table.foreign_keys = fks.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

            // Load indexes
            let mut idx_stmt = conn.prepare(
                "SELECT index_name, is_unique 
                 FROM meta_indexes 
                 WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4"
            ).map_err(|e| e.to_string())?;
            
            let indexes = idx_stmt.query_map(
                params![&table.connection_id, &table.database, &table.schema, &table.table_name],
                |row| {
                    Ok(MetaIndex {
                        connection_id: table.connection_id.clone(),
                        database: table.database.clone(),
                        schema: table.schema.clone(),
                        table_name: table.table_name.clone(),
                        index_name: row.get(0)?,
                        is_unique: row.get::<_, i32>(1)? != 0,
                    })
                }
            ).map_err(|e| e.to_string())?;
            
            table.indexes = indexes.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

            // Load triggers
            let mut trigger_stmt = conn.prepare(
                "SELECT trigger_name, event, timing 
                 FROM meta_triggers 
                 WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4"
            ).map_err(|e| e.to_string())?;
            
            let triggers = trigger_stmt.query_map(
                params![&table.connection_id, &table.database, &table.schema, &table.table_name],
                |row| {
                    Ok(MetaTrigger {
                        connection_id: table.connection_id.clone(),
                        database: table.database.clone(),
                        schema: table.schema.clone(),
                        table_name: table.table_name.clone(),
                        trigger_name: row.get(0)?,
                        event: row.get(1)?,
                        timing: row.get(2)?,
                    })
                }
            ).map_err(|e| e.to_string())?;
            
            table.triggers = triggers.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        }
        
        Ok(tables)
    }

    pub fn get_databases(&self, connection_id: &str) -> Result<Vec<MetaDatabase>, String> {
        let conn = self.app_db.lock().unwrap();
        
        // Query databases and check if any schemas exist for each to determine is_introspected
        let mut stmt = conn.prepare(
            "SELECT d.name, 
             EXISTS(SELECT 1 FROM meta_schemas s WHERE s.connection_id = d.connection_id AND s.database = d.name) as introspected
             FROM meta_databases d 
             WHERE d.connection_id = ?1 
             ORDER BY d.name"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![connection_id], |row| {
            Ok(MetaDatabase {
                name: row.get(0)?,
                is_connected: false,
                is_introspected: row.get(1)?,
                schemas: vec![],
            })
        }).map_err(|e| e.to_string())?;

        let mut dbs = Vec::new();
        for r in rows {
            dbs.push(r.map_err(|e| e.to_string())?);
        }
        Ok(dbs)
    }

    pub fn get_schemas(&self, connection_id: &str, database: &str) -> Result<Vec<MetaSchema>, String> {
        let conn = self.app_db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT s.name, s.schema_type,
             EXISTS(SELECT 1 FROM meta_tables t WHERE t.connection_id = s.connection_id AND t.database = s.database AND t.schema = s.name) as introspected
             FROM meta_schemas s 
             WHERE s.connection_id = ?1 AND s.database = ?2 
             ORDER BY s.name"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![connection_id, database], |row| {
            Ok(MetaSchema {
                name: row.get(0)?,
                schema_type: row.get(1)?,
                is_introspected: row.get(2)?,
                tables: vec![],
            })
        }).map_err(|e| e.to_string())?;

        let mut schemas = Vec::new();
        for r in rows {
            schemas.push(r.map_err(|e| e.to_string())?);
        }
        Ok(schemas)
    }

    pub fn get_tables_in_schema(&self, connection_id: &str, database: &str, schema: &str) -> Result<Vec<MetaTable>, String> {
        let conn = self.app_db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT connection_id, database, schema, table_name, type, classification, last_introspected_at 
             FROM meta_tables 
             WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 
             ORDER BY table_name"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![connection_id, database, schema], |row| {
            Ok(MetaTable {
                connection_id: row.get(0)?,
                database: row.get(1)?,
                schema: row.get(2)?,
                table_name: row.get(3)?,
                table_type: row.get(4)?,
                classification: row.get(5)?,
                last_introspected_at: row.get(6)?,
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            })
        }).map_err(|e| e.to_string())?;

        let mut tables = Vec::new();
        for r in rows {
            tables.push(r.map_err(|e| e.to_string())?);
        }
        Ok(tables)
    }

    pub fn get_schema(&self, connection_id: &str) -> Result<Vec<MetaDatabase>, String> {
        // Optimized to only fetch databases and their nested structure if small,
        // but for lazy loading, we might want to just call get_databases.
        // For backwards compatibility, we'll still try to load the full tree if it's cached.
        let mut dbs = self.get_databases(connection_id)?;
        for db in &mut dbs {
            db.schemas = self.get_schemas(connection_id, &db.name)?;
            for schema in &mut db.schemas {
                schema.tables = self.get_tables_in_schema(connection_id, &db.name, &schema.name)?;
            }
            if !db.schemas.is_empty() {
                db.is_introspected = true;
            }
        }
        Ok(dbs)
    }

    pub fn get_table_details(&self, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<serde_json::Value, String> {
        let conn = self.app_db.lock().unwrap();
        
        // Columns
        let mut col_stmt = conn.prepare("SELECT ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key FROM meta_columns WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 ORDER BY ordinal_position")
            .map_err(|e| e.to_string())?;
        let columns = col_stmt.query_map(params![connection_id, database, schema, table_name], |row| {
            Ok(MetaColumn {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                ordinal_position: row.get(0)?,
                column_name: row.get(1)?,
                raw_type: row.get(2)?,
                logical_type: row.get(3)?,
                nullable: row.get::<_, i32>(4)? != 0,
                default_value: row.get(5)?,
                is_primary_key: row.get::<_, i32>(6)? != 0,
            })
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        // Foreign Keys
        let mut fk_stmt = conn.prepare("SELECT column_name, ref_schema, ref_table, ref_column, constraint_name, seq_no FROM meta_foreign_keys WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 ORDER BY constraint_name, seq_no")
            .map_err(|e| e.to_string())?;
        let foreign_keys = fk_stmt.query_map(params![connection_id, database, schema, table_name], |row| {
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0)?,
                ref_schema: row.get(1)?,
                ref_table: row.get(2)?,
                ref_column: row.get(3)?,
                constraint_name: row.get(4)?,
                seq_no: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        // Indexes
        let mut idx_stmt = conn.prepare("SELECT index_name, is_unique FROM meta_indexes WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4")
            .map_err(|e| e.to_string())?;
        let indexes = idx_stmt.query_map(params![connection_id, database, schema, table_name], |row| {
            let index_name: String = row.get(0)?;
            Ok((index_name, row.get::<_, i32>(1)? != 0))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        let mut enriched_indexes = Vec::new();
        for (index_name, is_unique) in indexes {
            let mut col_stmt = conn.prepare("SELECT column_name, seq_no FROM meta_index_columns WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 AND index_name = ?5 ORDER BY seq_no")
                .map_err(|e| e.to_string())?;
            let columns = col_stmt.query_map(params![connection_id, database, schema, table_name, index_name], |row| {
                Ok(serde_json::json!({
                    "column_name": row.get::<_, String>(0)?,
                    "seq_no": row.get::<_, i32>(1)?
                }))
            }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

            enriched_indexes.push(serde_json::json!({
                "name": index_name,
                "is_unique": is_unique,
                "columns": columns
            }));
        }

        // Triggers
        let mut trigger_stmt = conn.prepare("SELECT trigger_name, event, timing FROM meta_triggers WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4")
            .map_err(|e| e.to_string())?;
        let triggers = trigger_stmt.query_map(params![connection_id, database, schema, table_name], |row| {
            Ok(serde_json::json!({
                "trigger_name": row.get::<_, String>(0)?,
                "event": row.get::<_, String>(1)?,
                "timing": row.get::<_, String>(2)?
            }))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "table_name": table_name,
            "columns": columns,
            "foreign_keys": foreign_keys,
            "indexes": enriched_indexes,
            "triggers": triggers
        }))
    }
    pub async fn introspect_postgres(&self, connection_id: &str, config: serde_json::Value) -> Result<Vec<MetaDatabase>, String> {
        let start_time = std::time::Instant::now();
        info!("Starting Postgres introspection for connection {}", connection_id);

        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let current_database = db.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        // Check TLS config
        let tls_enabled = config.get("tls")
            .and_then(|t| t.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // 1. Fetch ALL database names first
        // We connect to 'postgres' or the current db to list all
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, current_database);
        
        // Get client with proper TLS handling
        let client: tokio_postgres::Client = if tls_enabled {
            debug!("Introspecting Postgres with TLS enabled");
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
                .map_err(|e| {
                    error!("Postgres TLS introspection connection failed: {:?}", e);
                    format!("Connection error: {}", e)
                })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres introspection connection error: {}", e);
                }
            });
            client
        } else {
            debug!("Introspecting Postgres without TLS");
            let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
                .map_err(|e| {
                    error!("Postgres introspection connection failed: {:?}", e);
                    format!("Connection error: {}", e)
                })?;
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    error!("Postgres introspection connection error: {}", e);
                }
            });
            client
        };

        let db_rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false", &[]).await
            .map_err(|e| e.to_string())?;
        
        let all_databases: Vec<String> = db_rows.iter().map(|r| r.get(0)).collect();

        // 2. Introspect only the CURRENT database's tables/schemas
        // (In future we could loop through all_databases if requested)
        
        let table_rows = client.query(
            "SELECT table_schema, table_name, table_type 
             FROM information_schema.tables 
             WHERE table_type IN ('BASE TABLE', 'VIEW')", 
            &[]
        ).await.map_err(|e| e.to_string())?;

        let mut tables = Vec::new();
        let now = chrono::Utc::now().timestamp_millis();

        for row in table_rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let type_str: String = row.get(2);
            
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };
            let classification = "user";

            // Details
            let columns = self.introspect_postgres_columns(&client, connection_id, current_database, &schema, &name).await?;
            let foreign_keys = self.introspect_postgres_foreign_keys(&client, connection_id, current_database, &schema, &name).await?;
            let indexes = self.introspect_postgres_indexes(&client, connection_id, current_database, &schema, &name).await?;

            tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: current_database.to_string(),
                schema,
                table_name: name,
                table_type: table_type.to_string(),
                classification: classification.to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys,
                indexes,
                triggers: vec![],  // Will be loaded in level 4
            });
        }

        // 3. Save cache
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            
            // Save ALL discovery databases
            for db_name in &all_databases {
                self.save_database(&tx, connection_id, db_name)?;
            }
            
            for t in &tables {
                self.save_table_full(&tx, t)?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        // 4. Construct Hierarchy Result (All DBs, but with schemas only for current one)
        let mut db_list = Vec::new();
        for db_name in all_databases {
            let mut schemas = Vec::new();
            if db_name == current_database {
                // Group tables into schemas
                let mut schema_map: std::collections::HashMap<String, Vec<MetaTable>> = std::collections::HashMap::new();
                for t in tables.clone() {
                    schema_map.entry(t.schema.clone()).or_default().push(t);
                }
                for (s_name, s_tables) in schema_map {
                    let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                        "system"
                    } else {
                        "user"
                    };
                    schemas.push(MetaSchema { 
            name: s_name, 
            schema_type: schema_type.to_string(), 
            is_introspected: true,
            tables: s_tables 
        });
                }
            }
            db_list.push(MetaDatabase { 
                name: db_name.clone(), 
                is_connected: db_name == current_database,
                is_introspected: db_name == current_database,
                schemas 
            });
        }

        let elapsed = start_time.elapsed();
        info!("Postgres introspection completed in {:.2?} ({} databases, {} tables)", 
            elapsed, db_list.len(), tables.len());

        Ok(db_list)
    }

    async fn introspect_postgres_columns(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaColumn>, String> {
        let rows = client.query(
            "SELECT ordinal_position, column_name, udt_name as data_type, is_nullable, column_default 
             FROM information_schema.columns 
             WHERE table_schema = $1 AND table_name = $2 
             ORDER BY ordinal_position",
            &[&schema, &table_name]
        ).await.map_err(|e| e.to_string())?;

        // Check for PKs
        let pk_rows = client.query(
            "SELECT kcu.column_name
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
             WHERE tc.constraint_type = 'PRIMARY KEY'
               AND tc.table_schema = $1
               AND tc.table_name = $2",
            &[&schema, &table_name]
        ).await.map_err(|e| e.to_string())?;

        let pks: Vec<String> = pk_rows.iter().map(|r| r.get(0)).collect();

        let mut columns = Vec::new();
        for row in rows {
            let ordinal: i32 = row.get(0);
            let name: String = row.get(1);
            let raw_type: String = row.get(2);
            let is_nullable_str: String = row.get(3);
            let default_val: Option<String> = row.get(4);

            let logical_type = self.infer_postgres_logical_type(&raw_type);
            let is_primary_key = pks.contains(&name);

            columns.push(MetaColumn {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                ordinal_position: ordinal,
                column_name: name,
                raw_type,
                logical_type,
                nullable: is_nullable_str == "YES",
                default_value: default_val,
                is_primary_key,
            });
        }

        Ok(columns)
    }

    fn infer_postgres_logical_type(&self, raw_type: &str) -> String {
        let rt = raw_type.to_lowercase();
        if rt.contains("int") || rt == "serial" || rt == "bigserial" {
            "int".to_string()
        } else if rt.contains("numeric") || rt.contains("decimal") || rt.contains("real") || rt.contains("double") {
            "float".to_string()
        } else if rt.contains("json") {
            "json".to_string()
        } else if rt.contains("bool") {
            "boolean".to_string()
        } else if rt.contains("timestamp") {
            "timestamp".to_string()
        } else if rt.contains("date") {
            "date".to_string()
        } else if rt.contains("char") || rt.contains("text") || rt == "uuid" {
            "text".to_string()
        } else {
            "text".to_string()
        }
    }

    async fn introspect_postgres_foreign_keys(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaForeignKey>, String> {
        let rows = client.query(
            "SELECT
                con.conname AS constraint_name,
                sch.nspname AS schema_name,
                tab.relname AS table_name,
                col.attname AS column_name,
                rsch.nspname AS ref_schema_name,
                rtab.relname AS ref_table_name,
                rcol.attname AS ref_column_name,
                pos.seq_no
             FROM pg_constraint con
             JOIN pg_namespace sch ON sch.oid = con.connamespace
             JOIN pg_class tab ON tab.oid = con.conrelid
             JOIN pg_namespace rsch ON rsch.oid = con.confnamespace
             JOIN pg_class rtab ON rtab.oid = con.confrelid
             JOIN LATERAL unnest(con.conkey) WITH ORDINALITY AS pos(attnum, seq_no) ON true
             JOIN pg_attribute col ON col.attrelid = tab.oid AND col.attnum = pos.attnum
             JOIN LATERAL (
                SELECT attname, row_number() OVER() as rnum 
                FROM unnest(con.confkey) r_attnum
                JOIN pg_attribute r_attr ON r_attr.attrelid = rtab.oid AND r_attr.attnum = r_attnum
             ) rcol_info ON rcol_info.rnum = pos.seq_no
             JOIN pg_attribute rcol ON rcol.attrelid = rtab.oid AND rcol.attname = rcol_info.attname
             WHERE con.contype = 'f'
               AND sch.nspname = $1
               AND tab.relname = $2
             ORDER BY seq_no",
            &[&schema, &table_name]
        ).await.map_err(|e| e.to_string())?;

        let fks = rows.iter().map(|row| {
            MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(3),
                ref_schema: row.get(4),
                ref_table: row.get(5),
                ref_column: row.get(6),
                constraint_name: row.get(0),
                seq_no: row.get::<_, i64>(7) as i32,
            }
        }).collect();
        
        Ok(fks)
    }

    async fn introspect_postgres_indexes(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schema: &str, table_name: &str) -> Result<Vec<MetaIndex>, String> {
        let rows = client.query(
            "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = $1 AND tablename = $2",
            &[&schema, &table_name]
        ).await.map_err(|e| e.to_string())?;

        let mut indexes = Vec::new();
        for row in rows {
            let index_name: String = row.get(0);
            let def: String = row.get(1);
            let is_unique = def.to_uppercase().contains("UNIQUE INDEX");

            indexes.push(MetaIndex {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                index_name,
                is_unique,
            });
        }
        Ok(indexes)
    }

    async fn introspect_postgres_columns_bulk(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schemas: &[String]) -> Result<HashMap<(String, String), Vec<MetaColumn>>, String> {
        let rows = client.query(
            "SELECT table_schema, table_name, ordinal_position, column_name, udt_name as data_type, is_nullable, column_default 
             FROM information_schema.columns 
             WHERE table_schema = ANY($1)
             ORDER BY table_schema, table_name, ordinal_position",
            &[&schemas]
        ).await.map_err(|e| format!("Postgres query for columns failed: {}", e))?;

        // PKs bulk
        let pk_rows = client.query(
            "SELECT kcu.table_schema, kcu.table_name, kcu.column_name
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
             WHERE tc.constraint_type = 'PRIMARY KEY'
               AND tc.table_schema = ANY($1)",
            &[&schemas]
        ).await.map_err(|e| format!("Postgres query for primary keys failed: {}", e))?;

        let mut pk_map: HashSet<(String, String, String)> = HashSet::new();
        for r in pk_rows {
            pk_map.insert((r.get(0), r.get(1), r.get(2)));
        }

        let mut col_map: HashMap<(String, String), Vec<MetaColumn>> = HashMap::new();
        for row in rows {
            let schema: String = row.get(0);
            let table: String = row.get(1);
            let ordinal: i32 = row.get(2);
            let name: String = row.get(3);
            let raw_type: String = row.get(4);
            let is_nullable_str: String = row.get(5);
            let default_val: Option<String> = row.get(6);

            let logical_type = self.infer_postgres_logical_type(&raw_type);
            let is_primary_key = pk_map.contains(&(schema.clone(), table.clone(), name.clone()));

            col_map.entry((schema.clone(), table.clone())).or_default().push(MetaColumn {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table_name: table,
                ordinal_position: ordinal,
                column_name: name,
                raw_type,
                logical_type,
                nullable: is_nullable_str == "YES",
                default_value: default_val,
                is_primary_key,
            });
        }

        Ok(col_map)
    }

    async fn introspect_postgres_foreign_keys_bulk(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schemas: &[String]) -> Result<HashMap<(String, String), Vec<MetaForeignKey>>, String> {
        let rows = client.query(
            "SELECT
                con.conname AS constraint_name,
                sch.nspname AS schema_name,
                tab.relname AS table_name,
                col.attname AS column_name,
                rsch.nspname AS ref_schema_name,
                rtab.relname AS ref_table_name,
                rcol.attname AS ref_column_name,
                pos.seq_no
             FROM pg_constraint con
             JOIN pg_namespace sch ON sch.oid = con.connamespace
             JOIN pg_class tab ON tab.oid = con.conrelid
             JOIN pg_namespace rsch ON rsch.oid = con.confnamespace
             JOIN pg_class rtab ON rtab.oid = con.confrelid
             -- Unnesting keys and their ordinality to handle multi-column FKs correctly
             JOIN LATERAL unnest(con.conkey) WITH ORDINALITY AS pos(attnum, seq_no) ON true
             JOIN pg_attribute col ON col.attrelid = tab.oid AND col.attnum = pos.attnum
             -- Join for referenced columns using the same ordinality
             JOIN LATERAL (
                SELECT attname, row_number() OVER() as rnum 
                FROM unnest(con.confkey) r_attnum
                JOIN pg_attribute r_attr ON r_attr.attrelid = rtab.oid AND r_attr.attnum = r_attnum
             ) rcol_info ON rcol_info.rnum = pos.seq_no
             JOIN pg_attribute rcol ON rcol.attrelid = rtab.oid AND rcol.attname = rcol_info.attname
             WHERE con.contype = 'f'
               AND sch.nspname = ANY($1)",
            &[&schemas]
        ).await.map_err(|e| {
            let code = e.as_db_error().map(|db_err| db_err.code().code().to_string()).unwrap_or_else(|| "UNKNOWN".to_string());
            let message = e.as_db_error().map(|db_err| db_err.message().to_string()).unwrap_or_else(|| e.to_string());
            error!("[POSTGRES_FK_QUERY] Failed. Code: {}, Message: {}, Schemas: {:?}", code, message, schemas);
            format!("[POSTGRES_FK_QUERY] Postgres query for foreign keys failed: Code: {}, Message: {}", code, message)
        })?;

        let mut fk_map: HashMap<(String, String), Vec<MetaForeignKey>> = HashMap::new();
        for row in rows {
            let constraint_name: String = row.get(0);
            let schema: String = row.get(1);
            let table: String = row.get(2);
            let column_name: String = row.get(3);
            let ref_schema: String = row.get(4);
            let ref_table: String = row.get(5);
            let ref_column: String = row.get(6);
            let seq_no: i64 = row.get(7);

            fk_map.entry((schema.clone(), table.clone())).or_default().push(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table_name: table,
                column_name,
                ref_schema,
                ref_table,
                ref_column,
                constraint_name,
                seq_no: seq_no as i32,
            });
        }
        Ok(fk_map)
    }

    async fn introspect_postgres_indexes_bulk(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schemas: &[String]) -> Result<HashMap<(String, String), Vec<MetaIndex>>, String> {
        let rows = client.query(
            "SELECT schemaname, tablename, indexname, indexdef FROM pg_indexes WHERE schemaname = ANY($1)",
            &[&schemas]
        ).await.map_err(|e| format!("[POSTGRES_IDX_QUERY] Postgres query for indexes failed: {}", e))?;

        let mut idx_map: HashMap<(String, String), Vec<MetaIndex>> = HashMap::new();
        for row in rows {
            let schema: String = row.get(0);
            let table: String = row.get(1);
            let index_name: String = row.get(2);
            let def: String = row.get(3);
            let is_unique = def.to_uppercase().contains("UNIQUE INDEX");

            idx_map.entry((schema.clone(), table.clone())).or_default().push(MetaIndex {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table_name: table,
                index_name,
                is_unique,
            });
        }
        Ok(idx_map)
    }

    async fn introspect_postgres_triggers_bulk(&self, client: &tokio_postgres::Client, connection_id: &str, database: &str, schemas: &[String]) -> Result<HashMap<(String, String), Vec<MetaTrigger>>, String> {
        let rows = client.query(
            "SELECT event_object_schema, event_object_table, trigger_name, event_manipulation, action_timing
             FROM information_schema.triggers
             WHERE event_object_schema = ANY($1)",
            &[&schemas]
        ).await.map_err(|e| format!("[POSTGRES_TRG_QUERY] Postgres query for triggers failed: {}", e))?;

        let mut trg_map: HashMap<(String, String), Vec<MetaTrigger>> = HashMap::new();
        for row in rows {
            let schema: String = row.get(0);
            let table: String = row.get(1);
            trg_map.entry((schema.clone(), table.clone())).or_default().push(MetaTrigger {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema,
                table_name: table,
                trigger_name: row.get(2),
                event: row.get(3),
                timing: row.get(4),
            });
        }
        Ok(trg_map)
    }

    /// Progressive introspection with event emission at each level
    pub async fn introspect_postgres_progressive(&self, connection_id: &str, config: serde_json::Value, priority_database: Option<String>, priority_schema: Option<String>, app: &tauri::AppHandle) -> Result<(), String> {
        use tauri::Emitter;
        
        let start_time = std::time::Instant::now();
        info!("Starting progressive Postgres introspection for connection {} (priority: {:?}.{:?})", 
            connection_id, priority_database, priority_schema);

        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let current_database = db.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let tls_enabled = config.get("tls")
            .and_then(|t| t.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, current_database);
        
        let client: tokio_postgres::Client = if tls_enabled {
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
                .map_err(|e| format!("Connection error: {}", e))?;
            tokio::spawn(async move { let _ = connection.await; });
            client
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
                .map_err(|e| format!("Connection error: {}", e))?;
            tokio::spawn(async move { let _ = connection.await; });
            client
        };

        // === LEVEL 1: Databases ===
        let db_rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false", &[]).await
            .map_err(|e| e.to_string())?;
        let all_databases: Vec<String> = db_rows.iter().map(|r| r.get(0)).collect();
        
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| format!("Failed to start transaction for database level: {}", e))?;
            for db_name in &all_databases {
                self.save_database(&tx, connection_id, db_name)?;
            }
            tx.commit().map_err(|e| format!("Failed to commit transaction for database level: {}", e))?;
        }
        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 1,
            "connection_id": connection_id,
            "databases": &all_databases,
        }));
        info!("Level 1 complete: {} databases", all_databases.len());

        // === LEVEL 2: Schemas ===
        // If priority_database is set and different from current_database, we must reconnect 
        // because information_schema/pg_catalog are database-local.
        let mut current_client = client;
        let mut effective_database = current_database.to_string();

        if let Some(priority_db) = &priority_database {
            if priority_db != &effective_database {
                info!("[Introspector] Priority database {} is different from current connected database {}, reconnecting...", 
                    priority_db, effective_database);
                
                let priority_conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, priority_db);
                
                let new_client = if tls_enabled {
                    let tls_connector = native_tls::TlsConnector::builder()
                        .danger_accept_invalid_certs(true)
                        .build()
                        .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
                    let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
                    let (client, connection) = tokio_postgres::connect(&priority_conn_str, connector).await
                        .map_err(|e| format!("Priority connection error: {}", e))?;
                    tokio::spawn(async move { let _ = connection.await; });
                    client
                } else {
                    let (client, connection) = tokio_postgres::connect(&priority_conn_str, tokio_postgres::NoTls).await
                        .map_err(|e| format!("Priority connection error: {}", e))?;
                    tokio::spawn(async move { let _ = connection.await; });
                    client
                };
                current_client = new_client;
                effective_database = priority_db.clone();
            }
        }

        let schema_rows = current_client.query(
            "SELECT schema_name FROM information_schema.schemata WHERE catalog_name = $1",
            &[&effective_database]
        ).await.map_err(|e| e.to_string())?;
        let all_schemas: Vec<String> = schema_rows.iter().map(|r| r.get(0)).collect();
        
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| format!("Failed to start transaction for schema level: {}", e))?;
            for s_name in &all_schemas {
                let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                    "system"
                } else {
                    "user"
                };
                self.save_schema(&tx, connection_id, &effective_database, s_name, schema_type)?;
            }
            tx.commit().map_err(|e| format!("Failed to commit transaction for schema level: {}", e))?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 2,
            "connection_id": connection_id,
            "database": &effective_database,
            "schemas": &all_schemas,
        }));
        info!("Level 2 complete: {} schemas for database {}", all_schemas.len(), effective_database);

        // === LEVEL 3: Tables + Columns (bulk) ===
        let table_rows = current_client.query(
            "SELECT table_schema, table_name, table_type 
             FROM information_schema.tables 
             WHERE table_type IN ('BASE TABLE', 'VIEW')", 
            &[]
        ).await.map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp_millis();

        let mut column_map = self.introspect_postgres_columns_bulk(&current_client, connection_id, &effective_database, &all_schemas).await?;

        // Partition rows by priority
        let mut priority_rows = Vec::new();
        let mut other_rows = Vec::new();

        for row in table_rows {
            let schema: String = row.get(0);
            let is_priority = priority_schema.as_ref().map(|s| s == &schema).unwrap_or(false) 
                              && priority_database.as_ref().map(|d| d == &effective_database).unwrap_or(true);
            
            if is_priority {
                priority_rows.push(row);
            } else {
                other_rows.push(row);
            }
        }

        // Process priority first
        let mut priority_tables = Vec::new();
        for row in &priority_rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let type_str: String = row.get(2);
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };

            let columns = column_map.remove(&(schema.clone(), name.clone())).unwrap_or_default();

            priority_tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: effective_database.to_string(),
                schema,
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            });
        }

        // Save priority in transaction and EMIT READY
        if !priority_tables.is_empty() {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| format!("Failed to start transaction for priority tables: {}", e))?;
            for t in &priority_tables {
                self.save_table(&tx, t.clone())?;
                for col in &t.columns {
                    self.save_column(&tx, col.clone())?;
                }
            }
            tx.commit().map_err(|e| format!("Failed to commit transaction for priority tables: {}", e))?;

            let _ = app.emit("schema:ready", serde_json::json!({
                "connection_id": connection_id,
                "database": &effective_database,
                "schema": priority_schema.as_ref().unwrap_or(&"public".to_string()),
            }));
            info!("Priority schema ready: {} tables", priority_tables.len());
        }

        // Process others
        let mut other_tables = Vec::new();
        for row in &other_rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            let type_str: String = row.get(2);
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };

            let columns = column_map.remove(&(schema.clone(), name.clone())).unwrap_or_default();

            other_tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: effective_database.to_string(),
                schema,
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            });
        }

        // Save others in transaction
        if !other_tables.is_empty() {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| format!("Failed to start transaction for other tables: {}", e))?;
            for t in &other_tables {
                self.save_table(&tx, t.clone())?;
                for col in &t.columns {
                    self.save_column(&tx, col.clone())?;
                }
            }
            tx.commit().map_err(|e| format!("Failed to commit transaction for other tables: {}", e))?;
        }

        let mut all_tables = priority_tables;
        all_tables.extend(other_tables);

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 3,
            "connection_id": connection_id,
            "table_count": all_tables.len(),
        }));
        info!("Level 3 complete: {} tables", all_tables.len());

        // === LEVEL 4: FK + Indexes + Triggers (bulk) ===
        // We could prioritize here too, but Level 3 is the "unblock" point.
        let level4_res: Result<(), String> = (async {
            let mut fk_map = self.introspect_postgres_foreign_keys_bulk(&current_client, connection_id, &effective_database, &all_schemas).await?;
            let mut idx_map = self.introspect_postgres_indexes_bulk(&current_client, connection_id, &effective_database, &all_schemas).await?;
            let mut trg_map = self.introspect_postgres_triggers_bulk(&current_client, connection_id, &effective_database, &all_schemas).await?;

            for table in &mut all_tables {
                let key = (table.schema.clone(), table.table_name.clone());
                table.foreign_keys = fk_map.remove(&key).unwrap_or_default();
                table.indexes = idx_map.remove(&key).unwrap_or_default();
                table.triggers = trg_map.remove(&key).unwrap_or_default();
            }

            // Save FK/indexes/triggers in transaction
            {
                let app_db = self.app_db.lock().unwrap();
                let tx = app_db.unchecked_transaction().map_err(|e| format!("[TX_START_L4] Failed to start transaction for Level 4 metadata: {}", e))?;
                for t in &all_tables {
                    for fk in &t.foreign_keys {
                        self.save_foreign_key(&tx, fk.clone())?;
                    }
                    for idx in &t.indexes {
                        self.save_index(&tx, idx.clone())?;
                    }
                    for trigger in &t.triggers {
                        self.save_trigger(&tx, trigger.clone())?;
                    }
                }
                tx.commit().map_err(|e| format!("[TX_COMMIT_L4] Failed to commit transaction for Level 4 metadata: {}", e))?;
            }
            Ok(())
        }).await;

        if let Err(e) = level4_res {
            error!("[Introspector] Level 4 introspection failed: {}", e);
            return Err(format!("Level 4 introspection failed: {}", e));
        }
        
        let fk_count: usize = all_tables.iter().map(|t| t.foreign_keys.len()).sum();
        let idx_count: usize = all_tables.iter().map(|t| t.indexes.len()).sum();
        let trigger_count: usize = all_tables.iter().map(|t| t.triggers.len()).sum();
        
        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 4,
            "connection_id": connection_id,
            "fk_count": fk_count,
            "index_count": idx_count,
            "trigger_count": trigger_count,
        }));
        
        let elapsed = start_time.elapsed();
        info!("Progressive introspection completed in {:.2?} ({} tables, {} FK, {} indexes, {} triggers)", 
            elapsed, all_tables.len(), fk_count, idx_count, trigger_count);

        Ok(())
    }

    /// Targeted introspection for a specific schema
    pub async fn introspect_postgres_schema_progressive(&self, connection_id: &str, config: serde_json::Value, database: &str, schema: &str, app: &tauri::AppHandle) -> Result<(), String> {
        use tauri::Emitter;
        let start_time = std::time::Instant::now();
        info!("Starting specific schema introspection for {}.{} in connection {}", database, schema, connection_id);

        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let tls_enabled = config.get("tls")
            .and_then(|t| t.get("enabled"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database);
        
        let client: tokio_postgres::Client = if tls_enabled {
            let tls_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| format!("Failed to build TLS connector: {}", e))?;
            let connector = postgres_native_tls::MakeTlsConnector::new(tls_connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await
                .map_err(|e| format!("Connection error: {}", e))?;
            tokio::spawn(async move { let _ = connection.await; });
            client
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
                .map_err(|e| format!("Connection error: {}", e))?;
            tokio::spawn(async move { let _ = connection.await; });
            client
        };

        // Level 3: Tables + Columns
        {
            let app_db = self.app_db.lock().unwrap();
            let schema_type = if matches!(schema, "information_schema" | "pg_catalog" | "pg_toast") {
                "system"
            } else {
                "user"
            };
            self.save_schema(&app_db, connection_id, database, schema, schema_type)?;
        }

        let table_rows = client.query(
            "SELECT table_schema, table_name, table_type 
             FROM information_schema.tables 
             WHERE table_schema = $1 AND table_type IN ('BASE TABLE', 'VIEW')", 
            &[&schema]
        ).await.map_err(|e| e.to_string())?;

        let mut tables = Vec::new();
        let now = chrono::Utc::now().timestamp_millis();

        let s_list = vec![schema.to_string()];
        let mut column_map = self.introspect_postgres_columns_bulk(&client, connection_id, database, &s_list).await?;

        for row in table_rows {
            let name: String = row.get(1);
            let type_str: String = row.get(2);
            let table_type = if type_str == "BASE TABLE" { "table" } else { "view" };

            let columns = column_map.remove(&(schema.to_string(), name.clone())).unwrap_or_default();

            tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys: vec![],
                indexes: vec![],
                triggers: vec![],
            });
        }

        // Save tables
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for t in &tables {
                self.save_table(&tx, t.clone())?;
                for col in &t.columns {
                    self.save_column(&tx, col.clone())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 3,
            "connection_id": connection_id,
        }));
        let _ = app.emit("schema:ready", serde_json::json!({
            "connection_id": connection_id,
            "database": database,
            "schema": schema,
        }));

        // Level 4: Relations
        let mut fk_map = self.introspect_postgres_foreign_keys_bulk(&client, connection_id, database, &s_list).await?;
        let mut idx_map = self.introspect_postgres_indexes_bulk(&client, connection_id, database, &s_list).await?;
        let mut trg_map = self.introspect_postgres_triggers_bulk(&client, connection_id, database, &s_list).await?;

        for table in &mut tables {
            let key = (table.schema.clone(), table.table_name.clone());
            table.foreign_keys = fk_map.remove(&key).unwrap_or_default();
            table.indexes = idx_map.remove(&key).unwrap_or_default();
            table.triggers = trg_map.remove(&key).unwrap_or_default();
        }

        // Save Level 4
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for t in &tables {
                for fk in &t.foreign_keys {
                    self.save_foreign_key(&tx, fk.clone())?;
                }
                for idx in &t.indexes {
                    self.save_index(&tx, idx.clone())?;
                }
                for trigger in &t.triggers {
                    self.save_trigger(&tx, trigger.clone())?;
                }
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        let _ = app.emit("schema:level-complete", serde_json::json!({
            "level": 4,
            "connection_id": connection_id,
        }));

        info!("Specific schema introspection completed in {:.2?}", start_time.elapsed());
        Ok(())
    }
}
