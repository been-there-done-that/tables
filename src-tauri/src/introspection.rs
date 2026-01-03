use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection as SqliteConnection};
use log::info;
use std::sync::{Arc, Mutex};
use chrono;
use tokio_postgres;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDatabase {
    pub name: String,
    pub is_connected: bool, // New!
    pub schemas: Vec<MetaSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaSchema {
    pub name: String,
    pub schema_type: String, // "user" or "system"
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
    pub ref_table: String,
    pub ref_column: String,
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
            schemas: vec![MetaSchema {
                name: "main".to_string(),
                schema_type: "user".to_string(),
                tables,
            }],
        }])
    }

    pub async fn introspect_database(&self, connection_id: &str, config: serde_json::Value, database_name: &str) -> Result<MetaDatabase, String> {
        info!("On-demand introspection for database {} on connection {}", database_name, connection_id);
        
        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");

        // Connect specifically to the requested database
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database_name);
        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Postgres on-demand connection error: {}", e);
            }
        });

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

            let columns = self.introspect_postgres_columns(&client, connection_id, database_name, &schema, &name).await?;
            let foreign_keys = self.introspect_postgres_foreign_keys(&client, connection_id, database_name, &schema, &name).await?;
            let indexes = self.introspect_postgres_indexes(&client, connection_id, database_name, &schema, &name).await?;

            tables.push(MetaTable {
                connection_id: connection_id.to_string(),
                database: database_name.to_string(),
                schema,
                table_name: name,
                table_type: table_type.to_string(),
                classification: "user".to_string(),
                last_introspected_at: now,
                columns,
                foreign_keys,
                indexes,
            });
        }

        // Save to cache
        {
            let app_db = self.app_db.lock().unwrap();
            let tx = app_db.unchecked_transaction().map_err(|e| e.to_string())?;
            for t in &tables {
                self.save_table_full(&tx, t)?;
            }
            tx.commit().map_err(|e| e.to_string())?;
        }

        // Group into schemas
        let mut schemas = Vec::new();
        let mut schema_map: std::collections::HashMap<String, Vec<MetaTable>> = std::collections::HashMap::new();
        for t in tables {
            schema_map.entry(t.schema.clone()).or_default().push(t);
        }
        for (s_name, s_tables) in schema_map {
            let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                "system"
            } else {
                "user"
            };
            schemas.push(MetaSchema { name: s_name, schema_type: schema_type.to_string(), tables: s_tables });
        }
        schemas.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(MetaDatabase {
            name: database_name.to_string(),
            is_connected: true, // It succeeded, so consider it "connected" for UI
            schemas,
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
        let mut stmt = target_conn.prepare(&format!("SELECT \"from\", \"table\", \"to\" FROM pragma_foreign_key_list('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0)?,
                ref_table: row.get(1)?,
                ref_column: row.get(2)?,
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
        Ok(())
    }

    fn save_database(&self, conn: &SqliteConnection, connection_id: &str, name: &str) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_databases (connection_id, name) VALUES (?1, ?2)",
            params![connection_id, name]
        ).map_err(|e| e.to_string())?;
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
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_column(&self, conn: &SqliteConnection, col: MetaColumn) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_columns (connection_id, database, schema, table_name, ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![col.connection_id, col.database, col.schema, col.table_name, col.ordinal_position, col.column_name, col.raw_type, col.logical_type, col.nullable as i32, col.default_value, col.is_primary_key as i32]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_foreign_key(&self, conn: &SqliteConnection, fk: MetaForeignKey) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_foreign_keys (connection_id, database, schema, table_name, column_name, ref_table, ref_column) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![fk.connection_id, fk.database, fk.schema, fk.table_name, fk.column_name, fk.ref_table, fk.ref_column]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_index(&self, conn: &SqliteConnection, idx: MetaIndex) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_indexes (connection_id, database, schema, table_name, index_name, is_unique) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![idx.connection_id, idx.database, idx.schema, idx.table_name, idx.index_name, idx.is_unique as i32]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_index_column(&self, conn: &SqliteConnection, col: MetaIndexColumn) -> Result<(), String> {
        conn.execute(
            "INSERT OR REPLACE INTO meta_index_columns (connection_id, database, schema, table_name, index_name, column_name, seq_no) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![col.connection_id, col.database, col.schema, col.table_name, col.index_name, col.column_name, col.seq_no]
        ).map_err(|e| e.to_string())?;
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
                columns: vec![],
                foreign_keys: vec![],
                indexes: vec![],
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_schema(&self, connection_id: &str) -> Result<Vec<MetaDatabase>, String> {
        // 1. Fetch all known databases
        let mut db_list = Vec::new();
        {
            let conn = self.app_db.lock().unwrap();
            let mut stmt = conn.prepare("SELECT name FROM meta_databases WHERE connection_id = ?1 ORDER BY name")
                .map_err(|e| e.to_string())?;
            let db_names = stmt.query_map(params![connection_id], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            
            for name in db_names {
                let name = name.map_err(|e| e.to_string())?;
                db_list.push(MetaDatabase {
                    name: name,
                    is_connected: false,
                    schemas: Vec::new(),
                });
            }
        }

        let tables_basic = self.get_tables(connection_id)?;
        
        // Group into Vec<MetaDatabase>
        let mut db_map: std::collections::HashMap<String, std::collections::HashMap<String, Vec<MetaTable>>> = std::collections::HashMap::new();

        for mut table in tables_basic {
            // Fill details
            let conn = self.app_db.lock().unwrap();
            
            // Columns
            let mut col_stmt = conn.prepare("SELECT ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key FROM meta_columns WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 ORDER BY ordinal_position")
                .map_err(|e| e.to_string())?;
            let columns = col_stmt.query_map(params![connection_id, table.database, table.schema, table.table_name], |row| {
                Ok(MetaColumn {
                    connection_id: connection_id.to_string(),
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
            }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            
            table.columns = columns;

            // Foreign Keys
            let mut fk_stmt = conn.prepare("SELECT column_name, ref_table, ref_column FROM meta_foreign_keys WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4")
                .map_err(|e| e.to_string())?;
            let foreign_keys = fk_stmt.query_map(params![connection_id, table.database, table.schema, table.table_name], |row| {
                Ok(MetaForeignKey {
                    connection_id: connection_id.to_string(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.table_name.clone(),
                    column_name: row.get(0)?,
                    ref_table: row.get(1)?,
                    ref_column: row.get(2)?,
                })
            }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            
            table.foreign_keys = foreign_keys;

            // Indexes
            let mut idx_stmt = conn.prepare("SELECT index_name, is_unique FROM meta_indexes WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4")
                .map_err(|e| e.to_string())?;
            let indexes = idx_stmt.query_map(params![connection_id, table.database, table.schema, table.table_name], |row| {
                Ok(MetaIndex {
                    connection_id: connection_id.to_string(),
                    database: table.database.clone(),
                    schema: table.schema.clone(),
                    table_name: table.table_name.clone(),
                    index_name: row.get(0)?,
                    is_unique: row.get::<_, i32>(1)? != 0,
                })
            }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
            
            table.indexes = indexes;

            db_map.entry(table.database.clone()).or_default()
                  .entry(table.schema.clone()).or_default()
                  .push(table);
        }

        // 6. Final merging (ensure all initialized DBs find their tables)
        for mut db in &mut db_list {
            if let Some(schema_map) = db_map.remove(&db.name) {
                for (s_name, s_tables) in schema_map {
                    let schema_type = if matches!(s_name.as_str(), "information_schema" | "pg_catalog" | "pg_toast") {
                        "system"
                    } else {
                        "user"
                    };
                    db.schemas.push(MetaSchema {
                        name: s_name,
                        schema_type: schema_type.to_string(),
                        tables: s_tables,
                    });
                }
                db.schemas.sort_by(|a, b| a.name.cmp(&b.name));
            }
        }

        Ok(db_list)
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
        let mut fk_stmt = conn.prepare("SELECT column_name, ref_table, ref_column FROM meta_foreign_keys WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4")
            .map_err(|e| e.to_string())?;
        let foreign_keys = fk_stmt.query_map(params![connection_id, database, schema, table_name], |row| {
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0)?,
                ref_table: row.get(1)?,
                ref_column: row.get(2)?,
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

        Ok(serde_json::json!({
            "table_name": table_name,
            "columns": columns,
            "foreign_keys": foreign_keys,
            "indexes": enriched_indexes
        }))
    }
    pub async fn introspect_postgres(&self, connection_id: &str, config: serde_json::Value) -> Result<Vec<MetaDatabase>, String> {
        info!("Starting Postgres introspection for connection {}", connection_id);

        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let current_database = db.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        // 1. Fetch ALL database names first
        // We connect to 'postgres' or the current db to list all
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, current_database);
        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Postgres connection error: {}", e);
            }
        });

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
                    schemas.push(MetaSchema { name: s_name, schema_type: schema_type.to_string(), tables: s_tables });
                }
            }
            db_list.push(MetaDatabase { 
                name: db_name.clone(), 
                is_connected: db_name == current_database,
                schemas 
            });
        }

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
                kcu.column_name,
                ccu.table_name AS foreign_table_name,
                ccu.column_name AS foreign_column_name
             FROM information_schema.table_constraints AS tc
             JOIN information_schema.key_column_usage AS kcu
               ON tc.constraint_name = kcu.constraint_name
               AND tc.table_schema = kcu.table_schema
             JOIN information_schema.constraint_column_usage AS ccu
               ON ccu.constraint_name = tc.constraint_name
               AND ccu.table_schema = tc.table_schema
             WHERE tc.constraint_type = 'FOREIGN KEY'
               AND tc.table_schema = $1
               AND tc.table_name = $2",
            &[&schema, &table_name]
        ).await.map_err(|e| e.to_string())?;

        let mut fks = Vec::new();
        for row in rows {
            fks.push(MetaForeignKey {
                connection_id: connection_id.to_string(),
                database: database.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0),
                ref_table: row.get(1),
                ref_column: row.get(2),
            });
        }
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
}
