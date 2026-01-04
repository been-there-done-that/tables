//! Unified Introspection Commands
//!
//! One public API path, many private adapters.
//!
//! This module replaces the legacy engine-specific introspection commands
//! with a unified surface that routes through the adapter registry.
//!
//! ## Architecture
//! ```text
//! Frontend
//!   ↓
//! Tauri Command (unified)
//!   ↓
//! ProgressiveIntrospector (engine-agnostic)
//!   ↓
//! DatabaseAdapter (engine-specific)
//! ```

use serde::{Deserialize, Serialize};
use tauri::{State, AppHandle, Emitter};
use log::{debug, info, error};

use crate::DatabaseState;
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use crate::adapter::{DatabaseCapabilities, TableRef};
use crate::adapter_registry;
use crate::orchestrator::{ProgressiveIntrospector, IntrospectorConfig, IntrospectionEvent};
use crate::introspection::{Introspector, MetaDatabase, MetaSchema, MetaTable};

// =============================================================================
// Introspection Options (Unified API)
// =============================================================================

/// Scope of introspection - what level to refresh
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IntrospectionScope {
    /// Full introspection of all databases
    Global,
    /// Single database
    Database { name: String },
    /// Single schema within a database
    Schema { database: String, schema: String },
    /// Single table
    Table { database: String, schema: String, table: String },
}

impl Default for IntrospectionScope {
    fn default() -> Self {
        Self::Global
    }
}

/// Options for introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionOptions {
    /// What to introspect
    #[serde(default)]
    pub scope: IntrospectionScope,
    
    /// Force refresh even if cached
    #[serde(default)]
    pub force: bool,
    
    /// Priority database (for ordering)
    pub priority_database: Option<String>,
    
    /// Priority schema (for ordering)
    pub priority_schema: Option<String>,
}

impl Default for IntrospectionOptions {
    fn default() -> Self {
        Self {
            scope: IntrospectionScope::Global,
            force: false,
            priority_database: None,
            priority_schema: None,
        }
    }
}

// =============================================================================
// Unified Introspection Command
// =============================================================================

/// Unified schema refresh command.
///
/// This is the **single entry point** for all introspection.
/// No engine-specific branching in public API.
///
/// # Arguments
/// * `connection_id` - Connection to introspect
/// * `options` - What/how to introspect
///
/// # Events Emitted
/// * `introspection:level_complete` - After each level
/// * `introspection:schema_ready` - When UI can render
/// * `introspection:complete` - When fully done
/// * `introspection:error` - On failure
#[tauri::command]
pub async fn refresh_schema_unified(
    connection_id: String,
    options: Option<IntrospectionOptions>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
    app: AppHandle,
) -> Result<(), String> {
    let options = options.unwrap_or_default();
    info!("Unified schema refresh for connection {} with scope {:?}", 
        connection_id, options.scope);

    // 1. Get connection info
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let (connection, credentials) = manager.get_connection(&connection_id)?;

    // 2. Build config with injected credentials
    let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
        .map_err(|e| format!("Failed to parse connection config: {}", e))?;

    // Inject password if available
    if let Some(db) = config.get_mut("db") {
        if let Some(db_obj) = db.as_object_mut() {
            if let Some(password) = &credentials.password {
                db_obj.insert("password".to_string(), 
                    serde_json::Value::String(password.expose().to_string()));
            }
        }
    }

    // 3. Get capabilities for this engine
    let caps = DatabaseCapabilities::for_engine(&connection.engine);
    debug!("Engine '{}' has profile {:?}", connection.engine, caps.profile());

    // 4. Create adapter via registry
    let mut adapter = adapter_registry::create(&connection.engine, config.clone())
        .map_err(|e| format!("Failed to create adapter: {:?}", e))?;

    // 4.5. Establish connection
    adapter.connect().await
        .map_err(|e| format!("Failed to connect to database: {:?}", e))?;

    // 5. Create orchestrator with event emission
    let app_clone = app.clone();
    let conn_id = connection_id.clone();
    
    let orchestrator_config = IntrospectorConfig::new(&connection_id)
        .with_cache(true)
        .with_events(true);
    
    let orchestrator = ProgressiveIntrospector::new(adapter, orchestrator_config)
        .with_event_callback(Box::new(move |event| {
            // Forward events to frontend
            let event_name = match &event {
                IntrospectionEvent::LevelComplete { .. } => "introspection:level_complete",
                IntrospectionEvent::SchemaReady { .. } => "introspection:schema_ready",
                IntrospectionEvent::Complete { .. } => "introspection:complete",
                IntrospectionEvent::Error { .. } => "introspection:error",
            };
            
            if let Err(e) = app_clone.emit(event_name, &event) {
                error!("Failed to emit introspection event: {}", e);
            }
        }));

    // 6. Execute based on scope
    let scope_for_emit = options.scope.clone();
    
    match options.scope {
        IntrospectionScope::Global => {
            let databases = orchestrator.introspect_all().await
                .map_err(|e| format!("Introspection failed: {:?}", e))?;
            
            // Save to cache
            let introspector = Introspector::new(db_state.conn.clone());
            for db in databases {
                introspector.save_introspected_database(&connection_id, &db)?;
            }
        },
        IntrospectionScope::Database { name } => {
            let database = orchestrator.introspect_database(&name).await
                .map_err(|e| format!("Introspection failed: {:?}", e))?;
            
            // Save to cache
            let introspector = Introspector::new(db_state.conn.clone());
            introspector.save_introspected_database(&connection_id, &database)?;
        },
        IntrospectionScope::Schema { database, schema } => {
            // For schema-level, we introspect the database and filter
            let db = orchestrator.introspect_database(&database).await
                .map_err(|e| format!("Introspection failed: {:?}", e))?;
            
            // Save to cache (will update only the requested schema)
            let introspector = Introspector::new(db_state.conn.clone());
            introspector.save_introspected_database(&connection_id, &db)?;
        },
        IntrospectionScope::Table { database, schema, table } => {
            // Table-level refresh - get table details only
            let table_ref = TableRef::new(&database, &schema, &table);
            let columns = orchestrator.adapter().list_columns(&table_ref).await
                .map_err(|e| format!("Failed to get columns: {:?}", e))?;
            
            // This would update just the table in cache
            let introspector = Introspector::new(db_state.conn.clone());
            introspector.save_introspected_columns(&connection_id, &database, &schema, &table, &columns)?;
        },
    }

    // 7. Emit completion
    let _ = app.emit("introspection:complete", serde_json::json!({
        "connection_id": connection_id,
        "scope": scope_for_emit,
    }));

    info!("Unified schema refresh completed for connection {}", connection_id);
    Ok(())
}

// =============================================================================
// Cache Read Commands (Already Unified - Keep As Is)
// =============================================================================

/// Get cached schema - reads from cache, never touches network.
#[tauri::command]
pub async fn get_cached_schema(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaDatabase>, String> {
    debug!("Fetching cached schema for connection {}", connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_schema(&connection_id)
}

/// Get cached databases.
#[tauri::command]
pub async fn get_cached_databases(
    connection_id: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaDatabase>, String> {
    debug!("Fetching cached databases for connection {}", connection_id);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_databases(&connection_id)
}

/// Get cached schemas for a database.
#[tauri::command]
pub async fn get_cached_schemas(
    connection_id: String,
    database: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaSchema>, String> {
    debug!("Fetching cached schemas for {}.{}", connection_id, database);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_schemas(&connection_id, &database)
}

/// Get cached tables for a schema.
#[tauri::command]
pub async fn get_cached_tables(
    connection_id: String,
    database: String,
    schema: String,
    db_state: State<'_, DatabaseState>,
) -> Result<Vec<MetaTable>, String> {
    debug!("Fetching cached tables for {}.{}.{}", connection_id, database, schema);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_tables_in_schema(&connection_id, &database, &schema)
}

/// Get cached table details.
#[tauri::command]
pub async fn get_cached_table_details(
    connection_id: String,
    database: Option<String>,
    schema: Option<String>,
    table_name: String,
    db_state: State<'_, DatabaseState>,
) -> Result<serde_json::Value, String> {
    let database = database.unwrap_or_else(|| "main".to_string());
    let schema = schema.unwrap_or_else(|| "main".to_string());
    debug!("Fetching cached details for table {}.{}.{}", database, schema, table_name);
    let introspector = Introspector::new(db_state.conn.clone());
    introspector.get_table_details(&connection_id, &database, &schema, &table_name)
}

/// Get engine capabilities for a connection.
#[tauri::command]
pub async fn get_connection_capabilities(
    connection_id: String,
    conn_state: State<'_, ConnectionManagerState>,
    db_state: State<'_, DatabaseState>,
) -> Result<DatabaseCapabilities, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let (connection, _) = manager.get_connection(&connection_id)?;
    Ok(DatabaseCapabilities::for_engine(&connection.engine))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection_scope_serialization() {
        let scope = IntrospectionScope::Database { name: "mydb".to_string() };
        let json = serde_json::to_string(&scope).unwrap();
        assert!(json.contains("database"));
        assert!(json.contains("mydb"));
    }

    #[test]
    fn test_introspection_options_default() {
        let opts = IntrospectionOptions::default();
        matches!(opts.scope, IntrospectionScope::Global);
        assert!(!opts.force);
    }

    #[test]
    fn test_scope_variants() {
        let global = IntrospectionScope::Global;
        let db = IntrospectionScope::Database { name: "test".to_string() };
        let schema = IntrospectionScope::Schema { 
            database: "db".to_string(), 
            schema: "public".to_string() 
        };
        let table = IntrospectionScope::Table { 
            database: "db".to_string(), 
            schema: "public".to_string(), 
            table: "users".to_string() 
        };

        // All should serialize
        assert!(serde_json::to_string(&global).is_ok());
        assert!(serde_json::to_string(&db).is_ok());
        assert!(serde_json::to_string(&schema).is_ok());
        assert!(serde_json::to_string(&table).is_ok());
    }
}
