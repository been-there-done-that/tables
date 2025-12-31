use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection as SqliteConnection};
use log::info;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaTable {
    pub connection_id: String,
    pub schema: String,
    pub table_name: String,
    pub table_type: String,
    pub classification: String,
    pub last_introspected_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaColumn {
    pub connection_id: String,
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
    pub schema: String,
    pub table_name: String,
    pub index_name: String,
    pub is_unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaIndexColumn {
    pub connection_id: String,
    pub schema: String,
    pub table_name: String,
    pub index_name: String,
    pub column_name: String,
    pub seq_no: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaForeignKey {
    pub connection_id: String,
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

    pub fn introspect_sqlite<F>(&self, connection_id: &str, sqlite_path: &str, on_progress: F) -> Result<(), String> 
    where F: Fn(&MetaTable) {
        info!("Starting SQLite introspection for connection {} at {}", connection_id, sqlite_path);
        
        // 1. Discovery (on target DB)
        let target_conn = SqliteConnection::open(sqlite_path)
            .map_err(|e| format!("Failed to open target SQLite database: {}", e))?;

        let mut stmt = target_conn.prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view')")
            .map_err(|e| e.to_string())?;
        
        let tables_and_views: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 2. Transactional Write to App DB
        let mut app_db_lock = self.app_db.lock().map_err(|e| e.to_string())?;
        let tx = app_db_lock.transaction().map_err(|e| e.to_string())?;

        // Clear existing cache for this connection
        self.clear_cache(&tx, connection_id)?;

        for (name, ttype) in tables_and_views {
            let classification = if name.starts_with("sqlite_") {
                "system"
            } else if name.contains("_fts_") || name.ends_with("_content") || name.ends_with("_segments") {
                "fts"
            } else {
                "user"
            };

            info!("Introspecting {} '{}'", ttype, name);

            let meta_table = MetaTable {
                connection_id: connection_id.to_string(),
                schema: "main".to_string(),
                table_name: name.clone(),
                table_type: ttype,
                classification: classification.to_string(),
                last_introspected_at: now,
            };

            // Save Table
            self.save_table(&tx, meta_table.clone())?;
            
            // Notify Progress
            on_progress(&meta_table);

            // 2. Columns
            self.introspect_columns(&target_conn, &tx, connection_id, "main", &name)?;

            // 3. Foreign Keys
            self.introspect_foreign_keys(&target_conn, &tx, connection_id, "main", &name)?;

            // 4. Indexes
            self.introspect_indexes(&target_conn, &tx, connection_id, "main", &name)?;
        }

        tx.commit().map_err(|e| e.to_string())?;

        info!("Introspection complete for connection {}", connection_id);
        Ok(())
    }

    fn introspect_columns(&self, target_conn: &SqliteConnection, app_conn: &SqliteConnection, connection_id: &str, schema: &str, table_name: &str) -> Result<(), String> {
        let mut stmt = target_conn.prepare(&format!("SELECT cid, \"name\", \"type\", \"notnull\", dflt_value, pk FROM pragma_table_xinfo('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let cid: i32 = row.get(0)?;
            let name: String = row.get(1)?;
            let raw_type: String = row.get(2)?;
            let notnull: i32 = row.get(3)?;
            let dflt_value: Option<String> = row.get(4)?;
            let pk: i32 = row.get(5)?;

            let logical_type = self.infer_logical_type(&raw_type);

            Ok(MetaColumn {
                connection_id: connection_id.to_string(),
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

        for col in rows {
            let col = col.map_err(|e| e.to_string())?;
            self.save_column(app_conn, col)?;
        }

        Ok(())
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

    fn introspect_foreign_keys(&self, target_conn: &SqliteConnection, app_conn: &SqliteConnection, connection_id: &str, schema: &str, table_name: &str) -> Result<(), String> {
        let mut stmt = target_conn.prepare(&format!("SELECT \"from\", \"table\", \"to\" FROM pragma_foreign_key_list('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0)?,
                ref_table: row.get(1)?,
                ref_column: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?;

        for fk in rows {
            let fk = fk.map_err(|e| e.to_string())?;
            self.save_foreign_key(app_conn, fk)?;
        }

        Ok(())
    }

    fn introspect_indexes(&self, target_conn: &SqliteConnection, app_conn: &SqliteConnection, connection_id: &str, schema: &str, table_name: &str) -> Result<(), String> {
        let mut stmt = target_conn.prepare(&format!("SELECT \"name\", \"unique\" FROM pragma_index_list('{}')", table_name))
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            let index_name: String = row.get(0)?;
            let unique: i32 = row.get(1)?;
            Ok((index_name, unique > 0))
        }).map_err(|e| e.to_string())?;

        for res in rows {
            let (index_name, is_unique) = res.map_err(|e| e.to_string())?;
            
            self.save_index(app_conn, MetaIndex {
                connection_id: connection_id.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                index_name: index_name.clone(),
                is_unique,
            })?;

            // Index Columns
            let mut col_stmt = target_conn.prepare(&format!("SELECT \"name\", \"seqno\" FROM pragma_index_info('{}')", index_name))
                .map_err(|e| e.to_string())?;
            
            let col_rows = col_stmt.query_map([], |row| {
                Ok(MetaIndexColumn {
                    connection_id: connection_id.to_string(),
                    schema: schema.to_string(),
                    table_name: table_name.to_string(),
                    index_name: index_name.clone(),
                    column_name: row.get(0)?,
                    seq_no: row.get(1)?,
                })
            }).map_err(|e| e.to_string())?;

            for col in col_rows {
                let col = col.map_err(|e| e.to_string())?;
                self.save_index_column(app_conn, col)?;
            }
        }

        Ok(())
    }

    // Persistence Helpers
    fn clear_cache(&self, conn: &SqliteConnection, connection_id: &str) -> Result<(), String> {
        conn.execute("DELETE FROM meta_tables WHERE connection_id = ?1", params![connection_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_table(&self, conn: &SqliteConnection, table: MetaTable) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_tables (connection_id, schema, table_name, type, classification, last_introspected_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(connection_id, schema, table_name) DO UPDATE SET
               type=excluded.type,
               classification=excluded.classification,
               last_introspected_at=excluded.last_introspected_at",
            params![table.connection_id, table.schema, table.table_name, table.table_type, table.classification, table.last_introspected_at]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_column(&self, conn: &SqliteConnection, col: MetaColumn) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_columns (connection_id, schema, table_name, ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![col.connection_id, col.schema, col.table_name, col.ordinal_position, col.column_name, col.raw_type, col.logical_type, col.nullable as i32, col.default_value, col.is_primary_key as i32]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_foreign_key(&self, conn: &SqliteConnection, fk: MetaForeignKey) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_foreign_keys (connection_id, schema, table_name, column_name, ref_table, ref_column) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![fk.connection_id, fk.schema, fk.table_name, fk.column_name, fk.ref_table, fk.ref_column]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_index(&self, conn: &SqliteConnection, idx: MetaIndex) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_indexes (connection_id, schema, table_name, index_name, is_unique) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![idx.connection_id, idx.schema, idx.table_name, idx.index_name, idx.is_unique as i32]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn save_index_column(&self, conn: &SqliteConnection, col: MetaIndexColumn) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_index_columns (connection_id, schema, table_name, index_name, column_name, seq_no) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![col.connection_id, col.schema, col.table_name, col.index_name, col.column_name, col.seq_no]
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    // API Helpers
    pub fn get_tables(&self, connection_id: &str) -> Result<Vec<MetaTable>, String> {
        let conn = self.app_db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT connection_id, schema, table_name, type, classification, last_introspected_at FROM meta_tables WHERE connection_id = ?1 ORDER BY schema, table_name")
            .map_err(|e| e.to_string())?;
        
        let rows = stmt.query_map(params![connection_id], |row| {
            Ok(MetaTable {
                connection_id: row.get(0)?,
                schema: row.get(1)?,
                table_name: row.get(2)?,
                table_type: row.get(3)?,
                classification: row.get(4)?,
                last_introspected_at: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_table_details(&self, connection_id: &str, schema: &str, table_name: &str) -> Result<serde_json::Value, String> {
        let conn = self.app_db.lock().unwrap();
        
        // Columns
        let mut col_stmt = conn.prepare("SELECT ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key FROM meta_columns WHERE connection_id = ?1 AND schema = ?2 AND table_name = ?3 ORDER BY ordinal_position")
            .map_err(|e| e.to_string())?;
        let columns = col_stmt.query_map(params![connection_id, schema, table_name], |row| {
            Ok(MetaColumn {
                connection_id: connection_id.to_string(),
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
        let mut fk_stmt = conn.prepare("SELECT column_name, ref_table, ref_column FROM meta_foreign_keys WHERE connection_id = ?1 AND schema = ?2 AND table_name = ?3")
            .map_err(|e| e.to_string())?;
        let foreign_keys = fk_stmt.query_map(params![connection_id, schema, table_name], |row| {
            Ok(MetaForeignKey {
                connection_id: connection_id.to_string(),
                schema: schema.to_string(),
                table_name: table_name.to_string(),
                column_name: row.get(0)?,
                ref_table: row.get(1)?,
                ref_column: row.get(2)?,
            })
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        // Indexes
        let mut idx_stmt = conn.prepare("SELECT index_name, is_unique FROM meta_indexes WHERE connection_id = ?1 AND schema = ?2 AND table_name = ?3")
            .map_err(|e| e.to_string())?;
        let indexes = idx_stmt.query_map(params![connection_id, schema, table_name], |row| {
            let index_name: String = row.get(0)?;
            Ok((index_name, row.get::<_, i32>(1)? != 0))
        }).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;

        let mut enriched_indexes = Vec::new();
        for (index_name, is_unique) in indexes {
            let mut col_stmt = conn.prepare("SELECT column_name, seq_no FROM meta_index_columns WHERE connection_id = ?1 AND schema = ?2 AND table_name = ?3 AND index_name = ?4 ORDER BY seq_no")
                .map_err(|e| e.to_string())?;
            let columns = col_stmt.query_map(params![connection_id, schema, table_name, index_name], |row| {
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
}
