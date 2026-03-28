//! Progressive Introspection Orchestrator
//!
//! This module provides a generic orchestrator for progressive database introspection.
//! It coordinates the multi-level introspection process using any `DatabaseAdapter`,
//! emitting events at each level so the UI can progressively render results.
//!
//! ## Introspection Levels
//! 1. **Databases** - List all accessible databases
//! 2. **Schemas** - List all schemas in each database
//! 3. **Tables + Columns** - Core structure (UI unblocks after this)
//! 4. **Metadata** - FKs, indexes, triggers (background enrichment)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use log::{info, debug, error};
use rusqlite::Connection as SqliteConnection;
use serde::Serialize;

use crate::adapter::{AdapterError, DatabaseAdapter, DatabaseCapabilities, TableRef};
use crate::introspection::{
    MetaColumn, MetaDatabase, MetaForeignKey, MetaIndex, MetaSchema, MetaTable, MetaTrigger,
};
use crate::schema_types::NamespaceKind;

// =============================================================================
// Event Types
// =============================================================================

/// Events emitted during progressive introspection.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntrospectionEvent {
    /// A level of introspection has completed
    LevelComplete {
        level: u8,
        connection_id: String,
        database: Option<String>,
        schema_count: Option<usize>,
        table_count: Option<usize>,
    },
    /// Core schema is ready (after level 3), UI can unblock
    SchemaReady {
        connection_id: String,
        database: String,
    },
    /// Detailed progress update within a schema
    SchemaProgress {
        connection_id: String,
        database: String,
        schema: String,
        stage: String,
        count: usize,
    },
    /// Full introspection complete (after level 4)
    Complete {
        connection_id: String,
        database: String,
    },
    /// A single table was discovered (name only, no metadata yet)
    TableDiscovered {
        connection_id: String,
        database: String,
        schema: String,
        table_name: String,
        table_type: String,
    },
    /// A single table is now complete (has columns + metadata)
    TableComplete {
        connection_id: String,
        database: String,
        schema: String,
        table_name: String,
        column_count: usize,
    },
    /// Error during introspection
    Error {
        connection_id: String,
        level: u8,
        message: String,
    },
}

/// Callback type for introspection events
pub type EventCallback = Box<dyn Fn(IntrospectionEvent) + Send + Sync>;

// =============================================================================
// Orchestrator Configuration
// =============================================================================

/// Configuration for the progressive introspector.
#[derive(Debug, Clone)]
pub struct IntrospectorConfig {
    /// Connection ID for this introspection session
    pub connection_id: String,
    /// Whether to save results to the local cache
    pub save_to_cache: bool,
    /// Whether to emit events during introspection
    pub emit_events: bool,
    /// Priority database to introspect first
    pub priority_database: Option<String>,
    /// Priority schema to introspect first
    pub priority_schema: Option<String>,
}

impl Default for IntrospectorConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            save_to_cache: true,
            emit_events: true,
            priority_database: None,
            priority_schema: None,
        }
    }
}

impl IntrospectorConfig {
    pub fn new(connection_id: impl Into<String>) -> Self {
        Self {
            connection_id: connection_id.into(),
            ..Default::default()
        }
    }

    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.save_to_cache = enabled;
        self
    }

    pub fn with_events(mut self, enabled: bool) -> Self {
        self.emit_events = enabled;
        self
    }

    pub fn with_priority(mut self, database: Option<String>, schema: Option<String>) -> Self {
        self.priority_database = database;
        self.priority_schema = schema;
        self
    }
}

// =============================================================================
// Progressive Introspector
// =============================================================================

/// Orchestrates progressive database introspection using any DatabaseAdapter.
///
/// The orchestrator follows a multi-level approach:
/// 1. List databases (or use synthetic "main" for flat engines)
/// 2. List schemas (or use synthetic "main" for schema-less engines)
/// 3. List tables + columns (UI unblocks here)
/// 4. List FKs, indexes, triggers (background enrichment)
pub struct ProgressiveIntrospector<A: DatabaseAdapter> {
    adapter: A,
    config: IntrospectorConfig,
    app_db: Option<Arc<Mutex<SqliteConnection>>>,
    event_callback: Option<EventCallback>,
}

impl<A: DatabaseAdapter> ProgressiveIntrospector<A> {
    /// Create a new progressive introspector.
    pub fn new(adapter: A, config: IntrospectorConfig) -> Self {
        Self {
            adapter,
            config,
            app_db: None,
            event_callback: None,
        }
    }

    /// Set the local cache database for saving results.
    pub fn with_cache(mut self, app_db: Arc<Mutex<SqliteConnection>>) -> Self {
        self.app_db = Some(app_db);
        self
    }

    /// Set the event callback for progress notifications.
    pub fn with_event_callback(mut self, callback: EventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Get a reference to the inner adapter.
    pub fn adapter(&self) -> &A {
        &self.adapter
    }

    /// Get a reference to the adapter's capabilities.
    pub fn capabilities(&self) -> &DatabaseCapabilities {
        self.adapter.capabilities()
    }

    /// Emit an event if configured.
    fn emit(&self, event: IntrospectionEvent) {
        if self.config.emit_events {
            if let Some(ref callback) = self.event_callback {
                callback(event);
            }
        }
    }

    /// Helper to save to cache if enabled
    fn save_database_to_cache(&self, db: &MetaDatabase) -> Result<(), AdapterError> {
        if !self.config.save_to_cache {
            return Ok(());
        }

        if let Some(ref app_db) = self.app_db {
            use crate::introspection::Introspector;
            let introspector = Introspector::new(Arc::clone(app_db));
            introspector.save_introspected_database(&self.config.connection_id, db)
                .map_err(|e| AdapterError::Internal(format!("Failed to save to cache: {}", e)))?;
        }
        Ok(())
    }

    /// Run the foreground phase of global introspection (Level 1 + Priority DB core).
    pub async fn introspect_foreground(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();
        
        info!("Starting foreground introspection for connection '{}'", connection_id);

        // === LEVEL 1: Databases ===
        let mut databases = if caps.supports_databases {
            self.adapter.list_databases().await?
        } else {
            vec![MetaDatabase {
                name: caps.effective_database(None),
                is_connected: true,
                is_introspected: false,
                schemas: vec![],
            }]
        };

        // Cache foundations
        if self.config.save_to_cache {
            if let Some(ref app_db) = self.app_db {
                use crate::introspection::Introspector;
                let introspector = Introspector::new(Arc::clone(app_db));
                for db in &databases {
                    let _ = introspector.save_introspected_database(connection_id, db);
                }
            }
        }

        self.emit(IntrospectionEvent::LevelComplete {
            level: 1,
            connection_id: connection_id.clone(),
            database: None,
            schema_count: None,
            table_count: None,
        });

        // Prioritize
        if let Some(priority_db) = &self.config.priority_database {
            databases.sort_by(|a, b| {
                if &a.name == priority_db { std::cmp::Ordering::Less }
                else if &b.name == priority_db { std::cmp::Ordering::Greater }
                else { a.name.cmp(&b.name) }
            });

            // Introspect priority DB core immediately
            if let Some(db) = databases.first() {
                if &db.name == priority_db {
                    debug!("Introspecting priority database '{}' core in foreground", db.name);
                    let _ = self.introspect_database_core(&db.name).await;
                }
            }
        }

        Ok(databases)
    }

    /// Background task to finish everything else.
    pub async fn introspect_background(&self, databases: Vec<MetaDatabase>) {
        let connection_id = &self.config.connection_id;
        info!("Starting background introspection for connection '{}'", connection_id);

        for db in databases {
            // Already handled priority DB core in foreground, but we need Level 4 for it
            // and full for others.
            match self.introspect_database(&db.name).await {
                Ok(_) => debug!("Background introspection complete for '{}'", db.name),
                Err(e) => error!("Background introspection failed for '{}': {}", db.name, e),
            }
        }
    }

    /// Helper for core introspection (Level 2 & 3)
    async fn introspect_database_core(&self, database_name: &str) -> Result<MetaDatabase, AdapterError> {
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();

        // Level 2: Schemas
        let schemas = if caps.supports_schemas {
            self.adapter.list_schemas(database_name).await?
        } else {
            vec![MetaSchema {
                name: caps.effective_schema(None),
                schema_type: "user".to_string(),
                kind: NamespaceKind::LogicalGroup,
                is_introspected: false,
                tables: vec![],
                functions: vec![],
                sequences: vec![],
            }]
        };

        self.emit(IntrospectionEvent::LevelComplete {
            level: 2,
            connection_id: connection_id.clone(),
            database: Some(database_name.to_string()),
            schema_count: Some(schemas.len()),
            table_count: None,
        });

        // Cache foundations (Level 2)
        if self.config.save_to_cache {
            if let Some(ref app_db) = self.app_db {
                use crate::introspection::Introspector;
                let introspector = Introspector::new(Arc::clone(app_db));
                
                // Clear existing schemas and tables to ensure no stale data remains
                for schema in &schemas {
                    let _ = introspector.clear_schema_cache(connection_id, database_name, &schema.name);
                }
            }
            
            let partial_db = MetaDatabase {
                name: database_name.to_string(),
                is_connected: true,
                is_introspected: false,
                schemas: schemas.clone(),
            };
            self.save_database_to_cache(&partial_db)?;
        }

        // Level 3: Tables + Columns
        let mut all_tables: Vec<MetaTable> = Vec::new();
        for schema in &schemas {
            let mut tables = self.adapter.list_tables(database_name, &schema.name).await?;
            for table in &mut tables {
                table.connection_id = connection_id.clone();
                let columns = self.adapter.list_columns(&TableRef::new(database_name, &schema.name, &table.table_name)).await?;
                table.columns = columns.into_iter().map(|mut c| { c.connection_id = connection_id.clone(); c }).collect();
            }
            all_tables.extend(tables);
        }

        // Cache results
        let mut schema_map: HashMap<String, Vec<MetaTable>> = HashMap::new();
        for table in all_tables {
            schema_map.entry(table.schema.clone()).or_default().push(table);
        }

        let result_schemas: Vec<MetaSchema> = schemas.into_iter().map(|mut s| {
            s.tables = schema_map.remove(&s.name).unwrap_or_default();
            s.is_introspected = true;
            s
        }).collect();

        let db = MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: result_schemas,
        };

        self.save_database_to_cache(&db)?;

        self.emit(IntrospectionEvent::LevelComplete {
            level: 3,
            connection_id: connection_id.clone(),
            database: Some(database_name.to_string()),
            schema_count: Some(db.schemas.len()),
            table_count: Some(db.schemas.iter().map(|s| s.tables.len()).sum()),
        });

        self.emit(IntrospectionEvent::SchemaReady {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });
        
        Ok(db)
    }

    /// Run progressive introspection for a specific database.
    ///
    /// Returns the fully introspected database with all schemas and tables.
    pub async fn introspect_database(&self, database_name: &str) -> Result<MetaDatabase, AdapterError> {
        // Run core if needed (or just re-run list_ tables which is cheap if cached in adapter?)
        // Actually, we'll just run core then metadata.
        let mut db = self.introspect_database_core(database_name).await?;
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();

        // === LEVEL 4: Metadata (FKs, Indexes, Triggers) ===
        if caps.supports_foreign_keys || caps.supports_indexes || caps.supports_triggers {
            for schema in &mut db.schemas {
                for table in &mut schema.tables {
                    let table_ref = TableRef::new(database_name, &table.schema, &table.table_name);
                    
                    if caps.supports_foreign_keys {
                        let fks = self.adapter.list_foreign_keys(&table_ref).await?;
                        table.foreign_keys = fks.into_iter().map(|mut fk| { fk.connection_id = connection_id.clone(); fk }).collect();
                    }
                    
                    if caps.supports_indexes {
                        let indexes = self.adapter.list_indexes(&table_ref).await?;
                        table.indexes = indexes.into_iter().map(|mut idx| { idx.connection_id = connection_id.clone(); idx }).collect();
                    }
                    
                    if caps.supports_triggers {
                        let triggers = self.adapter.list_triggers(&table_ref).await?;
                        table.triggers = triggers.into_iter().map(|mut trg| { trg.connection_id = connection_id.clone(); trg }).collect();
                    }
                }
            }

            self.save_database_to_cache(&db)?;
            
            self.emit(IntrospectionEvent::LevelComplete {
                level: 4,
                connection_id: connection_id.clone(),
                database: Some(database_name.to_string()),
                schema_count: Some(db.schemas.len()),
                table_count: Some(db.schemas.iter().map(|s| s.tables.len()).sum()),
            });
        }

        self.emit(IntrospectionEvent::Complete {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });

        Ok(db)
    }

    /// Run progressive introspection for a specific schema.
    pub async fn introspect_schema(&self, database_name: &str, schema_name: &str) -> Result<MetaDatabase, AdapterError> {
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();
        
        info!("Starting schema introspection for {}.{}", database_name, schema_name);

        // Core Introspection for single schema
        // We construct a partial MetaDatabase with just this schema
        
        // 1. Get Schema Metadata (we need the schema object)
        // We'll list all schemas and find the one we want to ensure we have correct metadata
        let schemas = if caps.supports_schemas {
            self.adapter.list_schemas(database_name).await?
        } else {
            vec![MetaSchema {
                name: caps.effective_schema(None),
                schema_type: "user".to_string(),
                kind: NamespaceKind::LogicalGroup,
                is_introspected: false,
                tables: vec![],
                functions: vec![],
                sequences: vec![],
            }]
        };
        
        let target_schema = schemas.into_iter()
            .find(|s| s.name == schema_name)
            .ok_or_else(|| AdapterError::Internal(format!("Schema '{}' not found", schema_name)))?;

        // Clear existing metadata for this schema before re-inserting
        // This ensures that deleted tables/columns are removed from the cache.
        if self.config.save_to_cache {
            if let Some(ref app_db) = self.app_db {
                use crate::introspection::Introspector;
                let introspector = Introspector::new(Arc::clone(app_db));
                let _ = introspector.clear_schema_cache(connection_id, database_name, schema_name);
            }
        }

        // 2. Fetch All Data Progressively
        
        // A. Tables
        let mut tables = self.adapter.list_tables(database_name, schema_name).await?;
        
        // Save TABLES only first (fast feedback)
        let tables_count = tables.len();
        let target_schema_base = target_schema.clone();
        
        // Note: We need a way to save *partial* tables (without columns/FKs yet).
        // Our save_database_to_cache saves the whole DB struct.
        // We'll construct a partial DB and save it.
        let mut partial_schema = target_schema_base.clone();
        partial_schema.tables = tables.clone();
        partial_schema.is_introspected = true; // Mark true so UI shows it, even if incomplete? Or maybe false?
        // If we mark false, UI might show spinner. If true, it shows content. 
        // Let's keep it true but the content is just tables.
        
        let partial_db = MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: vec![partial_schema.clone()],
        };
        self.save_database_to_cache(&partial_db)?;
        
        // Emit per-table discovery events for optimistic rendering
        for table in &tables {
            self.emit(IntrospectionEvent::TableDiscovered {
                connection_id: connection_id.clone(),
                database: database_name.to_string(),
                schema: schema_name.to_string(),
                table_name: table.table_name.clone(),
                table_type: table.table_type.clone(),
            });
        }
        
        self.emit(IntrospectionEvent::SchemaProgress {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
            schema: schema_name.to_string(),
            stage: "tables".to_string(),
            count: tables_count,
        });

        // B. Columns (Bulk)
        let all_columns = self.adapter.list_columns_schema(database_name, schema_name).await?;
        let columns_count = all_columns.len();
        
        let mut columns_map: HashMap<String, Vec<MetaColumn>> = HashMap::new();
        for mut col in all_columns {
            col.connection_id = connection_id.clone();
            columns_map.entry(col.table_name.clone()).or_default().push(col);
        }
        
        // Attach columns to tables
        for table in &mut tables {
            // We need to re-attach columns. 
            // Note: tables currently has empty columns/FKs/etc from list_tables (except what list_tables gives)
            table.columns = columns_map.remove(&table.table_name).unwrap_or_default();
        }
        
        // Save TABLES + COLUMNS
        partial_schema.tables = tables.clone();
        let db_with_cols = MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: vec![partial_schema.clone()],
        };
        self.save_database_to_cache(&db_with_cols)?;
        
        // Emit per-table completion events (table now has columns)
        for table in &tables {
            self.emit(IntrospectionEvent::TableComplete {
                connection_id: connection_id.clone(),
                database: database_name.to_string(),
                schema: schema_name.to_string(),
                table_name: table.table_name.clone(),
                column_count: table.columns.len(),
            });
        }
        
        self.emit(IntrospectionEvent::SchemaProgress {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
            schema: schema_name.to_string(),
            stage: "columns".to_string(),
            count: columns_count,
        });

        // C, D, E. Metadata (Bulk)
        // Convert remaining steps to "metadata" stage or generic "metadata"
        
        let mut metadata_count = 0;

        // C. Indexes
        let all_indexes = if caps.supports_indexes {
            self.adapter.list_indexes_schema(database_name, schema_name).await?
        } else {
            Vec::new()
        };
        metadata_count += all_indexes.len();

        // D. Foreign Keys
        let all_fks = if caps.supports_foreign_keys {
            self.adapter.list_foreign_keys_schema(database_name, schema_name).await?
        } else {
            Vec::new()
        };
        metadata_count += all_fks.len();

        // E. Triggers
        let all_triggers = if caps.supports_triggers {
            self.adapter.list_triggers_schema(database_name, schema_name).await?
        } else {
            Vec::new()
        };
        metadata_count += all_triggers.len();

        // Assemble Metadata
        let mut indexes_map: HashMap<String, Vec<MetaIndex>> = HashMap::new();
        for mut idx in all_indexes {
            idx.connection_id = connection_id.clone();
            indexes_map.entry(idx.table_name.clone()).or_default().push(idx);
        }

        let mut fks_map: HashMap<String, Vec<MetaForeignKey>> = HashMap::new();
        for mut fk in all_fks {
            fk.connection_id = connection_id.clone();
            fks_map.entry(fk.table_name.clone()).or_default().push(fk);
        }

        let mut triggers_map: HashMap<String, Vec<MetaTrigger>> = HashMap::new();
        for mut trg in all_triggers {
            trg.connection_id = connection_id.clone();
            triggers_map.entry(trg.table_name.clone()).or_default().push(trg);
        }

        // Attach to tables
        for table in &mut tables {
            // Columns are already there, don't overwrite them or lose them
            // table struct here in loop refers to the one from previous step? 
            // Yes, locally 'tables' var has columns attached.
            
            table.connection_id = connection_id.clone();
            table.indexes = indexes_map.remove(&table.table_name).unwrap_or_default();
            table.foreign_keys = fks_map.remove(&table.table_name).unwrap_or_default();
            table.triggers = triggers_map.remove(&table.table_name).unwrap_or_default();
        }

        // 4. Final Save & Result
        let mut result_schema = target_schema.clone();
        result_schema.tables = tables;
        result_schema.is_introspected = true;

        let db = MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: vec![result_schema],
        };

        // Cache foundations (Level 3 + 4 in one go)
        self.save_database_to_cache(&db)?;

        self.emit(IntrospectionEvent::SchemaProgress {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
            schema: schema_name.to_string(),
            stage: "metadata".to_string(),
            count: metadata_count,
        });

        self.emit(IntrospectionEvent::SchemaReady {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });

        self.emit(IntrospectionEvent::Complete {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });

        Ok(db)
    }

    /// Run progressive introspection for all accessible databases.
    pub async fn introspect_all(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        let databases = self.introspect_foreground().await?;
        self.introspect_background(databases.clone()).await;
        // In this synchronous version, we return the names we found
        Ok(databases)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspector_config() {
        let config = IntrospectorConfig::new("conn-123");
        assert_eq!(config.connection_id, "conn-123");
        assert!(config.save_to_cache);
        assert!(config.emit_events);
    }

    #[test]
    fn test_event_serialization() {
        let event = IntrospectionEvent::LevelComplete {
            level: 3,
            connection_id: "conn-1".to_string(),
            database: Some("mydb".to_string()),
            schema_count: Some(5),
            table_count: Some(50),
        };
        
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("level_complete"));
        assert!(json.contains("mydb"));
    }
}
