//! Completion Engine Variants
//!
//! Database-specific completion engines with shared core logic.
//! - `PostgresEngine`: PostgreSQL-specific keywords, functions, and behaviors
//! - `SqliteEngine`: SQLite-specific keywords, functions, and behaviors
//!
//! Both engines share the core completion logic from `core.rs`.

pub mod core;
pub mod postgres;
pub mod sqlite;

pub use core::CoreCompletionEngine;
pub use postgres::PostgresEngine;
pub use sqlite::SqliteEngine;

use crate::completion::document::Dialect;
use crate::completion::analysis::SemanticModel;
use crate::completion::context::Context;
use crate::completion::schema::SchemaGraph;
use crate::adapter::DatabaseCapabilities;
use super::engine::CompletionItem;

/// Trait for database-specific completion behavior.
pub trait CompletionEngineVariant: Send + Sync {
    /// Get the dialect this engine handles.
    fn dialect(&self) -> Dialect;
    
    /// Get database-specific keywords for the given context.
    fn keywords(&self, context: &Context) -> Vec<&'static str>;
    
    /// Get database-specific functions for the given context.
    fn functions(&self, context: &Context) -> Vec<&'static str>;
    
    /// Get the default schema name for this database.
    fn default_schema(&self) -> &str;

    /// Get database-specific operators (label, detail, score).
    fn operators(&self) -> Vec<(&'static str, &'static str, u32)>;
    
    /// Generate completions using this engine's specific behavior.
    fn complete(
        &self,
        semantic: &SemanticModel,
        context: &Context,
        schema: &SchemaGraph,
        default_schema: Option<&str>,
        capabilities: Option<&DatabaseCapabilities>,
    ) -> Vec<CompletionItem>;
}

/// Factory to create the appropriate engine for a dialect.
pub fn create_engine(dialect: Dialect) -> Box<dyn CompletionEngineVariant> {
    match dialect {
        Dialect::Postgres => Box::new(PostgresEngine::new()),
        Dialect::SQLite => Box::new(SqliteEngine::new()),
        Dialect::MySQL => Box::new(PostgresEngine::new()), // Fallback to Postgres for now
    }
}
