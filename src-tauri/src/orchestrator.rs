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
    /// Full introspection complete (after level 4)
    Complete {
        connection_id: String,
        database: String,
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
}

impl Default for IntrospectorConfig {
    fn default() -> Self {
        Self {
            connection_id: String::new(),
            save_to_cache: true,
            emit_events: true,
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

    /// Run progressive introspection for a specific database.
    ///
    /// Returns the fully introspected database with all schemas and tables.
    pub async fn introspect_database(&self, database_name: &str) -> Result<MetaDatabase, AdapterError> {
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();
        
        info!("Starting progressive introspection for database '{}' on connection '{}'", 
              database_name, connection_id);

        // === LEVEL 2: Schemas ===
        let schemas = if caps.supports_schemas {
            self.adapter.list_schemas(database_name).await?
        } else {
            // Use synthetic schema for flat engines
            vec![MetaSchema {
                name: caps.effective_schema(None),
                schema_type: "user".to_string(),
                is_introspected: false,
                tables: vec![],
            }]
        };

        self.emit(IntrospectionEvent::LevelComplete {
            level: 2,
            connection_id: connection_id.clone(),
            database: Some(database_name.to_string()),
            schema_count: Some(schemas.len()),
            table_count: None,
        });

        // === LEVEL 3: Tables + Columns ===
        let mut all_tables: Vec<MetaTable> = Vec::new();
        
        for schema in &schemas {
            let mut tables = self.adapter.list_tables(database_name, &schema.name).await?;
            
            // Enrich each table with columns
            for table in &mut tables {
                table.connection_id = connection_id.clone();
                let table_ref = TableRef::new(database_name, &schema.name, &table.table_name);
                let columns = self.adapter.list_columns(&table_ref).await?;
                table.columns = columns.into_iter().map(|mut c| {
                    c.connection_id = connection_id.clone();
                    c
                }).collect();
            }
            
            all_tables.extend(tables);
        }

        self.emit(IntrospectionEvent::LevelComplete {
            level: 3,
            connection_id: connection_id.clone(),
            database: Some(database_name.to_string()),
            schema_count: Some(schemas.len()),
            table_count: Some(all_tables.len()),
        });

        // UI can unblock now
        self.emit(IntrospectionEvent::SchemaReady {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });

        // === LEVEL 4: Metadata (FKs, Indexes, Triggers) ===
        if caps.supports_foreign_keys || caps.supports_indexes || caps.supports_triggers {
            for table in &mut all_tables {
                let table_ref = TableRef::new(database_name, &table.schema, &table.table_name);
                
                if caps.supports_foreign_keys {
                    let fks = self.adapter.list_foreign_keys(&table_ref).await?;
                    table.foreign_keys = fks.into_iter().map(|mut fk| {
                        fk.connection_id = connection_id.clone();
                        fk
                    }).collect();
                }
                
                if caps.supports_indexes {
                    let indexes = self.adapter.list_indexes(&table_ref).await?;
                    table.indexes = indexes.into_iter().map(|mut idx| {
                        idx.connection_id = connection_id.clone();
                        idx
                    }).collect();
                }
                
                if caps.supports_triggers {
                    let triggers = self.adapter.list_triggers(&table_ref).await?;
                    table.triggers = triggers.into_iter().map(|mut trg| {
                        trg.connection_id = connection_id.clone();
                        trg
                    }).collect();
                }
            }
        }

        self.emit(IntrospectionEvent::LevelComplete {
            level: 4,
            connection_id: connection_id.clone(),
            database: Some(database_name.to_string()),
            schema_count: Some(schemas.len()),
            table_count: Some(all_tables.len()),
        });

        // Group tables by schema
        let mut schema_map: HashMap<String, Vec<MetaTable>> = HashMap::new();
        for table in all_tables {
            schema_map.entry(table.schema.clone()).or_default().push(table);
        }

        let mut result_schemas: Vec<MetaSchema> = schemas.into_iter().map(|mut s| {
            s.tables = schema_map.remove(&s.name).unwrap_or_default();
            s.is_introspected = true;
            s
        }).collect();
        result_schemas.sort_by(|a, b| a.name.cmp(&b.name));

        self.emit(IntrospectionEvent::Complete {
            connection_id: connection_id.clone(),
            database: database_name.to_string(),
        });

        Ok(MetaDatabase {
            name: database_name.to_string(),
            is_connected: true,
            is_introspected: true,
            schemas: result_schemas,
        })
    }

    /// Run progressive introspection for all accessible databases.
    pub async fn introspect_all(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        let connection_id = &self.config.connection_id;
        let caps = self.adapter.capabilities();
        
        // === LEVEL 1: Databases ===
        let databases = if caps.supports_databases {
            self.adapter.list_databases().await?
        } else {
            // Use synthetic database for flat engines
            vec![MetaDatabase {
                name: caps.effective_database(None),
                is_connected: true,
                is_introspected: false,
                schemas: vec![],
            }]
        };

        self.emit(IntrospectionEvent::LevelComplete {
            level: 1,
            connection_id: connection_id.clone(),
            database: None,
            schema_count: None,
            table_count: None,
        });

        // Introspect each database
        let mut results = Vec::new();
        for db in databases {
            match self.introspect_database(&db.name).await {
                Ok(introspected) => results.push(introspected),
                Err(e) => {
                    error!("Failed to introspect database '{}': {}", db.name, e);
                    self.emit(IntrospectionEvent::Error {
                        connection_id: connection_id.clone(),
                        level: 2,
                        message: e.to_string(),
                    });
                }
            }
        }

        Ok(results)
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
