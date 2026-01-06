//! Adapter Registry
//!
//! Provides dynamic registration of database adapters at runtime.
//! This enables plugins to contribute adapters for new database engines
//! without modifying core code.
//!
//! ## Usage
//!
//! ```ignore
//! // Plugin registers an adapter factory
//! adapter_registry::register("custom_db", |config| {
//!     Ok(Box::new(CustomDbAdapter::from_config(config)?))
//! });
//!
//! // Core code creates adapters dynamically
//! let adapter = adapter_registry::create("custom_db", config)?;
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

use crate::adapter::{AdapterError, DatabaseAdapter, DatabaseCapabilities};

// =============================================================================
// Types
// =============================================================================

/// Factory function type for creating adapters
pub type AdapterFactory = Box<dyn Fn(serde_json::Value) -> Result<Box<dyn DatabaseAdapter>, AdapterError> + Send + Sync>;

/// Metadata for a registered adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    /// Engine identifier (e.g., "postgres", "mysql")
    pub engine: String,
    /// Human-readable name
    pub name: String,
    /// Description
    pub description: String,
    /// Default capabilities for this adapter
    pub capabilities: DatabaseCapabilities,
    /// Whether this is a built-in adapter
    pub is_builtin: bool,
    /// Plugin that registered this adapter (None for built-ins)
    pub plugin_name: Option<String>,
}

impl AdapterInfo {
    /// Create info for a built-in adapter
    pub fn builtin(engine: &str, name: &str, description: &str, capabilities: DatabaseCapabilities) -> Self {
        Self {
            engine: engine.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            capabilities,
            is_builtin: true,
            plugin_name: None,
        }
    }

    /// Create info for a plugin-provided adapter
    pub fn from_plugin(engine: &str, name: &str, description: &str, capabilities: DatabaseCapabilities, plugin: &str) -> Self {
        Self {
            engine: engine.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            capabilities,
            is_builtin: false,
            plugin_name: Some(plugin.to_string()),
        }
    }
}

// =============================================================================
// Registry
// =============================================================================

struct AdapterEntry {
    info: AdapterInfo,
    factory: AdapterFactory,
}

/// Global adapter registry
struct AdapterRegistry {
    adapters: HashMap<String, AdapterEntry>,
}

impl AdapterRegistry {
    fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    fn register_entry(&mut self, info: AdapterInfo, factory: AdapterFactory) {
        self.adapters.insert(info.engine.clone(), AdapterEntry { info, factory });
    }

    fn get(&self, engine: &str) -> Option<&AdapterEntry> {
        self.adapters.get(engine)
    }

    fn list(&self) -> Vec<AdapterInfo> {
        self.adapters.values().map(|e| e.info.clone()).collect()
    }

    fn unregister(&mut self, engine: &str) -> bool {
        self.adapters.remove(engine).is_some()
    }
}

// Global registry instance
static ADAPTER_REGISTRY: Lazy<Arc<RwLock<AdapterRegistry>>> = 
    Lazy::new(|| Arc::new(RwLock::new(AdapterRegistry::new())));

// =============================================================================
// Public API
// =============================================================================

/// Register a database adapter with its factory function.
///
/// # Arguments
/// * `info` - Adapter metadata
/// * `factory` - Function that creates adapter instances from config
pub fn register(info: AdapterInfo, factory: AdapterFactory) {
    let mut registry = ADAPTER_REGISTRY.write().unwrap();
    registry.register_entry(info, factory);
}

/// Register a built-in adapter with default metadata.
pub fn register_builtin<F>(engine: &str, factory: F)
where
    F: Fn(serde_json::Value) -> Result<Box<dyn DatabaseAdapter>, AdapterError> + Send + Sync + 'static,
{
    let caps = DatabaseCapabilities::for_engine(engine);
    let engine_name = caps.engine.clone();
    let description = format!("{} database adapter", engine_name);
    let info = AdapterInfo::builtin(engine, &engine_name, &description, caps);
    register(info, Box::new(factory));
}

/// Create an adapter instance for the given engine.
pub fn create(engine: &str, config: serde_json::Value) -> Result<Box<dyn DatabaseAdapter>, AdapterError> {
    let registry = ADAPTER_REGISTRY.read().unwrap();
    
    match registry.get(engine) {
        Some(entry) => (entry.factory)(config),
        None => Err(AdapterError::NotSupported(format!("No adapter registered for engine '{}'", engine))),
    }
}

/// Get capabilities for an engine without creating an adapter.
pub fn capabilities(engine: &str) -> Option<DatabaseCapabilities> {
    let registry = ADAPTER_REGISTRY.read().unwrap();
    registry.get(engine).map(|e| e.info.capabilities.clone())
}

/// List all registered adapters.
pub fn list() -> Vec<AdapterInfo> {
    let registry = ADAPTER_REGISTRY.read().unwrap();
    registry.list()
}

/// Check if an adapter is registered for an engine.
pub fn is_registered(engine: &str) -> bool {
    let registry = ADAPTER_REGISTRY.read().unwrap();
    registry.get(engine).is_some()
}

/// Unregister an adapter (for plugin unload).
pub fn unregister(engine: &str) -> bool {
    let mut registry = ADAPTER_REGISTRY.write().unwrap();
    registry.unregister(engine)
}

/// Initialize built-in adapters.
/// Call this during application startup.
pub fn init_builtins() {
    use crate::adapters::{PostgresAdapter, SqliteAdapter};

    // Register PostgreSQL adapter
    register_builtin("postgres", |config| {
        Ok(Box::new(PostgresAdapter::from_config(config)?))
    });

    // Register SQLite adapter
    register_builtin("sqlite", |config| {
        Ok(Box::new(SqliteAdapter::from_config(config)?))
    });

    log::info!("Initialized {} built-in database adapters", list().len());
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_info_builtin() {
        let caps = DatabaseCapabilities::postgres();
        let info = AdapterInfo::builtin("postgres", "PostgreSQL", "PostgreSQL adapter", caps);
        
        assert_eq!(info.engine, "postgres");
        assert!(info.is_builtin);
        assert!(info.plugin_name.is_none());
    }

    #[test]
    fn test_adapter_info_plugin() {
        let caps = DatabaseCapabilities::default();
        let info = AdapterInfo::from_plugin("custom", "Custom DB", "Custom adapter", caps, "custom_plugin");
        
        assert_eq!(info.engine, "custom");
        assert!(!info.is_builtin);
        assert_eq!(info.plugin_name, Some("custom_plugin".to_string()));
    }
}
