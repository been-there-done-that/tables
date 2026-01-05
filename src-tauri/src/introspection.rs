use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection as SqliteConnection};
use log::{info, debug};
use std::sync::{Arc, Mutex};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

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
    pub constraint_name: Option<String>,  // May be None for engines that don't provide it
    pub constraint_hash: String,          // Deterministic hash for deduplication
    pub seq_no: i32,
}

/// Compute a deterministic hash for a foreign key constraint.
/// This is used for stable deduplication across introspection runs.
pub fn compute_fk_hash(table: &str, column: &str, ref_table: &str, ref_column: &str) -> String {
    let mut hasher = DefaultHasher::new();
    table.hash(&mut hasher);
    column.hash(&mut hasher);
    ref_table.hash(&mut hasher);
    ref_column.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

    /// Public API for saving an introspected database hierarchy
    pub fn save_introspected_database(&self, connection_id: &str, database: &MetaDatabase) -> Result<(), String> {
        let conn = self.app_db.lock().map_err(|e| e.to_string())?;
        
        // Save database record
        self.save_database(&conn, connection_id, &database.name)?;
        
        debug!("[INTRO_DEBUG] saving database {} with {} schemas", database.name, database.schemas.len());

        for schema in &database.schemas {
            self.save_schema(&conn, connection_id, &database.name, &schema.name, &schema.schema_type)?;
            
            for table in &schema.tables {
                // debug!("[INTRO_DEBUG] saving table {}.{}.{}", database.name, schema.name, table.table_name);
                self.save_table_full(&conn, table)?;
            }
        }
        Ok(())
    }

    /// Public API for saving columns for a specific table
    pub fn save_introspected_columns(&self, _connection_id: &str, _database: &str, _schema: &str, _table: &str, columns: &[MetaColumn]) -> Result<(), String> {
        let conn = self.app_db.lock().map_err(|e| e.to_string())?;
        
        for col in columns {
            self.save_column(&conn, col.clone())?;
        }
        Ok(())
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

    fn save_database(&self, conn: &SqliteConnection, connection_id: &str, name: &str) -> Result<i64, String> {
        conn.execute(
            "INSERT INTO meta_databases (connection_id, name) VALUES (?1, ?2)
             ON CONFLICT(connection_id, name) DO NOTHING",
            params![connection_id, name]
        ).map_err(|e| format!("Failed to save database '{}' to local cache: {}", name, e))?;
        
        // Get the database_id (either from insert or existing row)
        let database_id: i64 = conn.query_row(
            "SELECT database_id FROM meta_databases WHERE connection_id = ?1 AND name = ?2",
            params![connection_id, name],
            |row| row.get(0)
        ).map_err(|e| format!("Failed to get database_id for '{}': {}", name, e))?;
        
        Ok(database_id)
    }

    fn save_schema(&self, conn: &SqliteConnection, connection_id: &str, database: &str, name: &str, schema_type: &str) -> Result<i64, String> {
        // Get database_id first
        let database_id: i64 = conn.query_row(
            "SELECT database_id FROM meta_databases WHERE connection_id = ?1 AND name = ?2",
            params![connection_id, database],
            |row| row.get(0)
        ).map_err(|e| format!("Database '{}' not found for schema '{}': {}", database, name, e))?;
        
        conn.execute(
            "INSERT INTO meta_schemas (database_id, connection_id, database, name, schema_type) 
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(database_id, name) DO UPDATE SET schema_type = excluded.schema_type",
            params![database_id, connection_id, database, name, schema_type]
        ).map_err(|e| format!("Failed to save schema '{}.{}' to local cache: {}", database, name, e))?;
        
        // Get the schema_id
        let schema_id: i64 = conn.query_row(
            "SELECT schema_id FROM meta_schemas WHERE database_id = ?1 AND name = ?2",
            params![database_id, name],
            |row| row.get(0)
        ).map_err(|e| format!("Failed to get schema_id for '{}.{}': {}", database, name, e))?;
        
        Ok(schema_id)
    }

    fn save_table(&self, conn: &SqliteConnection, table: MetaTable) -> Result<i64, String> {
        // Get schema_id first
        let schema_id: i64 = conn.query_row(
            "SELECT schema_id FROM meta_schemas WHERE connection_id = ?1 AND database = ?2 AND name = ?3",
            params![table.connection_id, table.database, table.schema],
            |row| row.get(0)
        ).map_err(|e| format!("Schema '{}.{}' not found for table '{}': {}", table.database, table.schema, table.table_name, e))?;
        
        conn.execute(
            "INSERT INTO meta_tables (schema_id, connection_id, database, schema, table_name, type, classification, last_introspected_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(connection_id, database, schema, table_name) DO UPDATE SET
               type=excluded.type,
               classification=excluded.classification,
               last_introspected_at=excluded.last_introspected_at",
            params![schema_id, table.connection_id, table.database, table.schema, table.table_name, table.table_type, table.classification, table.last_introspected_at]
        ).map_err(|e| format!("Failed to save table '{}.{}.{}' to local cache: {}", table.database, table.schema, table.table_name, e))?;
        
        // Get the table_id
        let table_id: i64 = conn.query_row(
            "SELECT table_id FROM meta_tables WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4",
            params![table.connection_id, table.database, table.schema, table.table_name],
            |row| row.get(0)
        ).map_err(|e| format!("Failed to get table_id for '{}.{}.{}': {}", table.database, table.schema, table.table_name, e))?;
        
        Ok(table_id)
    }

    fn save_column(&self, conn: &SqliteConnection, col: MetaColumn) -> Result<(), String> {
        // Get table_id
        let table_id: i64 = conn.query_row(
            "SELECT table_id FROM meta_tables WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4",
            params![col.connection_id, col.database, col.schema, col.table_name],
            |row| row.get(0)
        ).map_err(|e| format!("Table '{}.{}.{}' not found for column '{}': {}", col.database, col.schema, col.table_name, col.column_name, e))?;
        
        conn.execute(
            "INSERT INTO meta_columns (table_id, connection_id, database, schema, table_name, ordinal_position, column_name, raw_type, logical_type, nullable, default_value, is_primary_key) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(table_id, column_name) DO UPDATE SET
               ordinal_position=excluded.ordinal_position,
               raw_type=excluded.raw_type,
               logical_type=excluded.logical_type,
               nullable=excluded.nullable,
               default_value=excluded.default_value,
               is_primary_key=excluded.is_primary_key",
            params![table_id, col.connection_id, col.database, col.schema, col.table_name, col.ordinal_position, col.column_name, col.raw_type, col.logical_type, col.nullable as i32, col.default_value, col.is_primary_key as i32]
        ).map_err(|e| format!("Failed to save column '{}.{}.{}.{}' to local cache: {}", col.database, col.schema, col.table_name, col.column_name, e))?;
        Ok(())
    }

    fn save_foreign_key(&self, conn: &SqliteConnection, fk: MetaForeignKey) -> Result<(), String> {
        // Get table_id
        let table_id: i64 = conn.query_row(
            "SELECT table_id FROM meta_tables WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4",
            params![fk.connection_id, fk.database, fk.schema, fk.table_name],
            |row| row.get(0)
        ).map_err(|e| format!("Table '{}.{}.{}' not found for FK: {}", fk.database, fk.schema, fk.table_name, e))?;
        
        conn.execute(
            "INSERT INTO meta_foreign_keys (table_id, connection_id, database, schema, table_name, column_name, ref_schema, ref_table, ref_column, constraint_name, constraint_hash, seq_no) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(table_id, constraint_hash, seq_no) DO UPDATE SET
               column_name=excluded.column_name,
               ref_schema=excluded.ref_schema,
               ref_table=excluded.ref_table,
               ref_column=excluded.ref_column,
               constraint_name=excluded.constraint_name",
            params![table_id, fk.connection_id, fk.database, fk.schema, fk.table_name, fk.column_name, fk.ref_schema, fk.ref_table, fk.ref_column, fk.constraint_name, fk.constraint_hash, fk.seq_no]
        ).map_err(|e| format!("[SQLITE_FK_SAVE] Failed to save foreign key '{}' ({}.{}.{}) to local cache: {}", fk.constraint_hash, fk.database, fk.schema, fk.table_name, e))?;
        Ok(())
    }

    fn save_index(&self, conn: &SqliteConnection, idx: MetaIndex) -> Result<i64, String> {
        // Get table_id
        let table_id: i64 = conn.query_row(
            "SELECT table_id FROM meta_tables WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4",
            params![idx.connection_id, idx.database, idx.schema, idx.table_name],
            |row| row.get(0)
        ).map_err(|e| format!("Table '{}.{}.{}' not found for index '{}': {}", idx.database, idx.schema, idx.table_name, idx.index_name, e))?;
        
        conn.execute(
            "INSERT INTO meta_indexes (table_id, connection_id, database, schema, table_name, index_name, is_unique) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(table_id, index_name) DO UPDATE SET is_unique=excluded.is_unique",
            params![table_id, idx.connection_id, idx.database, idx.schema, idx.table_name, idx.index_name, idx.is_unique as i32]
        ).map_err(|e| format!("[SQLITE_IDX_SAVE] Failed to save index '{}' to local cache: {}", idx.index_name, e))?;
        
        // Get the index_id
        let index_id: i64 = conn.query_row(
            "SELECT index_id FROM meta_indexes WHERE table_id = ?1 AND index_name = ?2",
            params![table_id, idx.index_name],
            |row| row.get(0)
        ).map_err(|e| format!("Failed to get index_id for '{}': {}", idx.index_name, e))?;
        
        Ok(index_id)
    }

    fn save_index_column(&self, conn: &SqliteConnection, index_id: i64, column_name: &str, seq_no: i32) -> Result<(), String> {
        conn.execute(
            "INSERT INTO meta_index_columns (index_id, column_name, seq_no) 
             VALUES (?1, ?2, ?3)
             ON CONFLICT(index_id, column_name) DO UPDATE SET seq_no=excluded.seq_no",
            params![index_id, column_name, seq_no]
        ).map_err(|e| format!("Failed to save index column '{}' for index_id {}: {}", column_name, index_id, e))?;
        Ok(())
    }

    fn save_trigger(&self, conn: &SqliteConnection, trigger: MetaTrigger) -> Result<(), String> {
        // Get table_id
        let table_id: i64 = conn.query_row(
            "SELECT table_id FROM meta_tables WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4",
            params![trigger.connection_id, trigger.database, trigger.schema, trigger.table_name],
            |row| row.get(0)
        ).map_err(|e| format!("Table '{}.{}.{}' not found for trigger '{}': {}", trigger.database, trigger.schema, trigger.table_name, trigger.trigger_name, e))?;
        
        conn.execute(
            "INSERT INTO meta_triggers (table_id, connection_id, database, schema, table_name, trigger_name, event, timing) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(table_id, trigger_name) DO UPDATE SET event=excluded.event, timing=excluded.timing",
            params![table_id, trigger.connection_id, trigger.database, trigger.schema, trigger.table_name, trigger.trigger_name, trigger.event, trigger.timing]
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
                "SELECT column_name, ref_schema, ref_table, ref_column, constraint_name, constraint_hash, seq_no
                 FROM meta_foreign_keys 
                 WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4
                 ORDER BY constraint_hash, seq_no"
            ).map_err(|e| e.to_string())?;
            
            let fks = fk_stmt.query_map(
                params![&table.connection_id, &table.database, &table.schema, &table.table_name],
                |row| {
                    let column_name: String = row.get(0)?;
                    let ref_schema: String = row.get(1)?;
                    let ref_table: String = row.get(2)?;
                    let ref_column: String = row.get(3)?;
                    let constraint_name: Option<String> = row.get(4)?;
                    let constraint_hash: String = row.get(5)?;
                    Ok(MetaForeignKey {
                        connection_id: table.connection_id.clone(),
                        database: table.database.clone(),
                        schema: table.schema.clone(),
                        table_name: table.table_name.clone(),
                        column_name,
                        ref_schema,
                        ref_table,
                        ref_column,
                        constraint_name,
                        constraint_hash,
                        seq_no: row.get(6)?,
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
             AND s.name NOT LIKE 'pg_toast%' AND s.name NOT LIKE 'pg_temp%'
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
             AND table_name NOT LIKE 'pg_toast%'
             ORDER BY table_name"
        ).map_err(|e| e.to_string())?;

        info!("[INTRO_DEBUG] get_tables_in_schema: con={}, db={}, schema={}", connection_id, database, schema);

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

    // =========================================================================
    // PUBLIC SAVE METHODS - For lazy loading commands
    // =========================================================================

    /// Save a list of databases to cache (public bulk wrapper)
    pub fn save_databases_public(&self, connection_id: &str, names: &[String]) -> Result<(), String> {
        let mut conn = self.app_db.lock().map_err(|e| e.to_string())?;
        // Use a transaction for bulk insert
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for name in names {
            self.save_database(&tx, connection_id, name)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Save a database to cache (public wrapper)
    pub fn save_database_public(&self, connection_id: &str, name: &str) -> Result<(), String> {
        let conn = self.app_db.lock().unwrap();
        self.save_database(&conn, connection_id, name)?;
        Ok(())
    }

    /// Save a list of schemas to cache (public bulk wrapper)
    pub fn save_schemas_public(&self, connection_id: &str, database: &str, schemas: &[MetaSchema]) -> Result<(), String> {
        let mut conn = self.app_db.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        // Ensure database exists first
        self.save_database(&tx, connection_id, database)?;
        
        for schema in schemas {
            self.save_schema(&tx, connection_id, database, &schema.name, &schema.schema_type)?;
        }
        
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Save a schema to cache (public wrapper)
    pub fn save_schema_public(&self, connection_id: &str, database: &str, name: &str, schema_type: &str) -> Result<(), String> {
        let conn = self.app_db.lock().unwrap();
        // Ensure database exists first
        self.save_database(&conn, connection_id, database)?;
        self.save_schema(&conn, connection_id, database, name, schema_type)?;
        Ok(())
    }

    /// Save a table to cache (public wrapper)
    pub fn save_table_public(&self, connection_id: &str, database: &str, schema: &str, table: &MetaTable) -> Result<(), String> {
        let conn = self.app_db.lock().map_err(|e| e.to_string())?;
        // Ensure database and schema exist first
        self.save_database(&conn, connection_id, database)?;
        self.save_schema(&conn, connection_id, database, schema, "user")?;
        
        // Create a clone with the correct IDs
        let table_to_save = MetaTable {
            connection_id: connection_id.to_string(),
            database: database.to_string(),
            schema: schema.to_string(),
            table_name: table.table_name.clone(),
            table_type: table.table_type.clone(),
            classification: table.classification.clone(),
            last_introspected_at: table.last_introspected_at,
            columns: table.columns.clone(),
            foreign_keys: table.foreign_keys.clone(),
            indexes: table.indexes.clone(),
            triggers: table.triggers.clone(),
        };
        
        self.save_table_full(&conn, &table_to_save)?;
        Ok(())
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
        let mut fk_stmt = conn.prepare("SELECT column_name, ref_schema, ref_table, ref_column, constraint_name, constraint_hash, seq_no FROM meta_foreign_keys WHERE connection_id = ?1 AND database = ?2 AND schema = ?3 AND table_name = ?4 ORDER BY constraint_hash, seq_no")
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
                constraint_hash: row.get(5)?,
                seq_no: row.get(6)?,
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
            let mut col_stmt = conn.prepare("
                SELECT mic.column_name, mic.seq_no 
                FROM meta_index_columns mic
                JOIN meta_indexes mi ON mic.index_id = mi.index_id
                WHERE mi.connection_id = ?1 
                  AND mi.database = ?2 
                  AND mi.schema = ?3 
                  AND mi.table_name = ?4 
                  AND mi.index_name = ?5 
                ORDER BY mic.seq_no
            ").map_err(|e| e.to_string())?;
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
}
