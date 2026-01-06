//! Database Adapters Module
//!
//! This module contains concrete implementations of the `DatabaseAdapter` trait
//! for different database engines.

mod postgres;
mod sqlite;

pub use postgres::PostgresAdapter;
pub use sqlite::SqliteAdapter;

use crate::adapter::{DatabaseCapabilities, AdapterError};

/// Create an adapter for each engine type in a standardized way.
pub fn create_adapter(engine: &str, config: serde_json::Value) -> Result<Box<dyn crate::adapter::DatabaseAdapter>, AdapterError> {
    match engine.to_lowercase().as_str() {
        "postgres" | "postgresql" => {
            Ok(Box::new(PostgresAdapter::from_config(config)?))
        }
        "sqlite" => {
            Ok(Box::new(SqliteAdapter::from_config(config)?))
        }
        _ => Err(AdapterError::NotSupported(format!("Engine '{}' is not supported", engine))),
    }
}
