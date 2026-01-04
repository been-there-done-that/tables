//! Database Adapter Module
//!
//! This module defines the core contracts for database adapters in the DCI
//! (Database Capability Interface) architecture. It provides:
//!
//! - `DatabaseCapabilities`: A struct describing what features a database engine supports
//! - `DatabaseAdapter`: A trait that all database adapters must implement
//! - `TableRef`: A fully-qualified table reference
//!
//! These contracts enable engine-agnostic introspection, completion, and plugin behavior.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::introspection::{
    MetaDatabase, MetaSchema, MetaTable, MetaColumn, MetaIndex, MetaForeignKey, MetaTrigger
};

// =============================================================================
// Core Types
// =============================================================================

/// Describes how an engine handles case sensitivity for identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseSensitivity {
    /// Identifiers are case-sensitive (e.g., PostgreSQL quoted identifiers)
    Sensitive,
    /// Identifiers are case-insensitive (e.g., MySQL, SQLite)
    Insensitive,
    /// Case is preserved but comparisons are insensitive
    Preserve,
}

impl Default for CaseSensitivity {
    fn default() -> Self {
        Self::Insensitive
    }
}

/// Engine capability profile for test matrix classification.
///
/// Profiles group engines by structural capabilities, enabling
/// capability-based testing rather than per-engine tests.
///
/// ## Profiles
/// - `DB0`: No database, no schema (SQLite)
/// - `DB1`: Database only, no schemas (MySQL, MongoDB)
/// - `DB2`: Database + schema (PostgreSQL)
/// - `DB3`: Multi-catalog (Trino, Snowflake - future)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EngineProfile {
    /// No database, no schema - flat structure (SQLite)
    DB0,
    /// Database only, no schemas (MySQL, MongoDB)
    DB1,
    /// Full database + schema support (PostgreSQL)
    DB2,
    /// Multi-catalog support (Trino, Snowflake)
    DB3,
}

/// A fully-qualified reference to a database table.
///
/// This struct normalizes table references across all database engines,
/// using synthetic values for engines that don't support certain levels
/// of hierarchy (e.g., SQLite uses "main" for both database and schema).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableRef {
    pub database: String,
    pub schema: String,
    pub name: String,
}

impl TableRef {
    /// Create a new table reference with explicit qualification.
    pub fn new(database: impl Into<String>, schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            database: database.into(),
            schema: schema.into(),
            name: name.into(),
        }
    }

    /// Create a table reference for a schema-less engine (e.g., SQLite).
    /// Uses "main" for both database and schema.
    pub fn from_name(name: impl Into<String>) -> Self {
        Self {
            database: "main".to_string(),
            schema: "main".to_string(),
            name: name.into(),
        }
    }

    /// Returns the fully qualified name as "database.schema.name".
    pub fn fully_qualified(&self) -> String {
        format!("{}.{}.{}", self.database, self.schema, self.name)
    }

    /// Returns the schema-qualified name as "schema.name".
    pub fn schema_qualified(&self) -> String {
        format!("{}.{}", self.schema, self.name)
    }
}

// =============================================================================
// Database Capabilities
// =============================================================================

/// Describes the capabilities and defaults of a database engine.
///
/// This struct is the source of truth for what a database supports.
/// It replaces implicit assumptions scattered throughout the codebase
/// with explicit, queryable capabilities.
///
/// # Example
/// ```ignore
/// let caps = DatabaseCapabilities::postgres();
/// assert!(caps.supports_schemas);
/// assert_eq!(caps.default_schema, Some("public".to_string()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseCapabilities {
    /// Engine identifier (e.g., "postgres", "sqlite", "mysql")
    pub engine: String,

    // Hierarchy Support
    /// Whether the engine supports multiple databases
    pub supports_databases: bool,
    /// Whether the engine supports schemas within databases
    pub supports_schemas: bool,

    // Feature Support
    /// Whether the engine supports views
    pub supports_views: bool,
    /// Whether the engine supports indexes
    pub supports_indexes: bool,
    /// Whether the engine supports foreign key constraints
    pub supports_foreign_keys: bool,
    /// Whether the engine supports triggers
    pub supports_triggers: bool,

    // Defaults (used when hierarchy level doesn't exist)
    /// Default database name for engines without database support
    pub default_database: Option<String>,
    /// Default schema name for engines without schema support
    pub default_schema: Option<String>,

    // Behavior
    /// Whether table names must always be qualified with schema
    pub requires_qualified_names: bool,
    /// How the engine handles identifier case
    pub case_sensitivity: CaseSensitivity,
}

impl Default for DatabaseCapabilities {
    fn default() -> Self {
        Self {
            engine: "unknown".to_string(),
            supports_databases: true,
            supports_schemas: true,
            supports_views: true,
            supports_indexes: true,
            supports_foreign_keys: true,
            supports_triggers: true,
            default_database: None,
            default_schema: None,
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Insensitive,
        }
    }
}

impl DatabaseCapabilities {
    /// PostgreSQL capabilities.
    pub fn postgres() -> Self {
        Self {
            engine: "postgres".to_string(),
            supports_databases: true,
            supports_schemas: true,
            supports_views: true,
            supports_indexes: true,
            supports_foreign_keys: true,
            supports_triggers: true,
            default_database: None,
            default_schema: Some("public".to_string()),
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Sensitive,
        }
    }

    /// SQLite capabilities.
    pub fn sqlite() -> Self {
        Self {
            engine: "sqlite".to_string(),
            supports_databases: false,
            supports_schemas: false,
            supports_views: true,
            supports_indexes: true,
            supports_foreign_keys: true,
            supports_triggers: true,
            default_database: Some("main".to_string()),
            default_schema: Some("main".to_string()),
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Insensitive,
        }
    }

    /// MySQL capabilities.
    pub fn mysql() -> Self {
        Self {
            engine: "mysql".to_string(),
            supports_databases: true,
            supports_schemas: false, // MySQL uses "database" as "schema"
            supports_views: true,
            supports_indexes: true,
            supports_foreign_keys: true,
            supports_triggers: true,
            default_database: None,
            default_schema: None,
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Insensitive,
        }
    }

    /// Athena (AWS) capabilities.
    pub fn athena() -> Self {
        Self {
            engine: "athena".to_string(),
            supports_databases: true, // "catalog" in Athena
            supports_schemas: true,   // "database" in Athena terminology
            supports_views: true,
            supports_indexes: false,  // Athena doesn't support indexes
            supports_foreign_keys: false,
            supports_triggers: false,
            default_database: None,
            default_schema: Some("default".to_string()),
            requires_qualified_names: true,
            case_sensitivity: CaseSensitivity::Insensitive,
        }
    }

    /// MongoDB capabilities.
    pub fn mongodb() -> Self {
        Self {
            engine: "mongodb".to_string(),
            supports_databases: true,
            supports_schemas: false, // Collections are schema-less
            supports_views: false,
            supports_indexes: true,
            supports_foreign_keys: false,
            supports_triggers: false,
            default_database: None,
            default_schema: None,
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Sensitive,
        }
    }

    /// Redis capabilities.
    pub fn redis() -> Self {
        Self {
            engine: "redis".to_string(),
            supports_databases: false, // Redis uses numeric database indices
            supports_schemas: false,
            supports_views: false,
            supports_indexes: false,
            supports_foreign_keys: false,
            supports_triggers: false,
            default_database: Some("0".to_string()),
            default_schema: None,
            requires_qualified_names: false,
            case_sensitivity: CaseSensitivity::Sensitive,
        }
    }

    /// Look up capabilities by engine name.
    pub fn for_engine(engine: &str) -> Self {
        match engine.to_lowercase().as_str() {
            "postgres" | "postgresql" => Self::postgres(),
            "sqlite" => Self::sqlite(),
            "mysql" | "mariadb" => Self::mysql(),
            "athena" => Self::athena(),
            "mongodb" | "mongo" => Self::mongodb(),
            "redis" => Self::redis(),
            _ => Self::default(),
        }
    }

    /// Get the effective database name, using default if engine doesn't support databases.
    pub fn effective_database(&self, database: Option<&str>) -> String {
        database
            .map(|d| d.to_string())
            .or_else(|| self.default_database.clone())
            .unwrap_or_else(|| "main".to_string())
    }

    /// Get the effective schema name, using default if engine doesn't support schemas.
    pub fn effective_schema(&self, schema: Option<&str>) -> String {
        schema
            .map(|s| s.to_string())
            .or_else(|| self.default_schema.clone())
            .unwrap_or_else(|| "main".to_string())
    }

    /// Get the engine profile for test matrix classification.
    ///
    /// Profiles group engines by structural capabilities:
    /// - `DB0`: No database, no schema (SQLite)
    /// - `DB1`: Database only, no schemas (MySQL, MongoDB)
    /// - `DB2`: Database + schema (PostgreSQL)
    /// - `DB3`: Multi-catalog (future)
    pub fn profile(&self) -> EngineProfile {
        match (self.supports_databases, self.supports_schemas) {
            (false, false) => EngineProfile::DB0,
            (true, false) => EngineProfile::DB1,
            (true, true) => EngineProfile::DB2,
            (false, true) => EngineProfile::DB2, // Unusual but treat as DB2
        }
    }
}

// =============================================================================
// Adapter Error
// =============================================================================

/// Errors that can occur during adapter operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterError {
    /// Connection-related error
    Connection(String),
    /// Query execution error
    Query(String),
    /// Object not found
    NotFound(String),
    /// Feature not supported by this engine
    NotSupported(String),
    /// Internal adapter error
    Internal(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(msg) => write!(f, "Connection error: {}", msg),
            Self::Query(msg) => write!(f, "Query error: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<String> for AdapterError {
    fn from(s: String) -> Self {
        Self::Internal(s)
    }
}

// =============================================================================
// Database Adapter Trait
// =============================================================================

/// Core trait that all database adapters must implement.
///
/// This trait defines the contract for interacting with any database engine.
/// Adapters wrap engine-specific logic and present a uniform interface to
/// the rest of the application.
///
/// # Lifecycle
/// 1. Create adapter with connection configuration
/// 2. Call `connect()` to establish connection
/// 3. Use introspection methods (`list_databases`, `list_tables`, etc.)
/// 4. Connection is dropped when adapter is dropped
///
/// # Example
/// ```ignore
/// let adapter = PostgresAdapter::new(config)?;
/// adapter.connect().await?;
/// let databases = adapter.list_databases().await?;
/// ```
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Returns the capabilities of this database engine.
    fn capabilities(&self) -> &DatabaseCapabilities;

    /// Returns the engine identifier (e.g., "postgres", "sqlite").
    fn engine_id(&self) -> &str {
        &self.capabilities().engine
    }

    /// Establish connection to the database.
    async fn connect(&mut self) -> Result<(), AdapterError>;

    /// Check if the adapter is currently connected.
    fn is_connected(&self) -> bool;

    /// Close the connection.
    async fn disconnect(&mut self) -> Result<(), AdapterError>;

    // =========================================================================
    // Level 1: Databases
    // =========================================================================

    /// List all accessible databases.
    ///
    /// For engines that don't support databases (e.g., SQLite), returns a
    /// single database with the default name.
    async fn list_databases(&self) -> Result<Vec<MetaDatabase>, AdapterError>;

    // =========================================================================
    // Level 2: Schemas
    // =========================================================================

    /// List all schemas in a database.
    ///
    /// For engines that don't support schemas (e.g., SQLite, MySQL), returns
    /// a single schema with the default name.
    async fn list_schemas(&self, database: &str) -> Result<Vec<MetaSchema>, AdapterError>;

    // =========================================================================
    // Level 3: Tables
    // =========================================================================

    /// List all tables in a schema.
    async fn list_tables(&self, database: &str, schema: &str) -> Result<Vec<MetaTable>, AdapterError>;

    /// List all columns for a table.
    async fn list_columns(&self, table: &TableRef) -> Result<Vec<MetaColumn>, AdapterError>;

    // =========================================================================
    // Level 4: Metadata
    // =========================================================================

    /// List all indexes for a table.
    async fn list_indexes(&self, table: &TableRef) -> Result<Vec<MetaIndex>, AdapterError>;

    /// List all foreign keys for a table.
    async fn list_foreign_keys(&self, table: &TableRef) -> Result<Vec<MetaForeignKey>, AdapterError>;

    /// List all triggers for a table.
    async fn list_triggers(&self, table: &TableRef) -> Result<Vec<MetaTrigger>, AdapterError>;
}

// =============================================================================
// Tests
// =============================================================================
// =============================================================================
// Trait Implementation for Box
// =============================================================================

#[async_trait]
impl DatabaseAdapter for Box<dyn DatabaseAdapter> {
    fn capabilities(&self) -> &DatabaseCapabilities {
        (**self).capabilities()
    }

    async fn connect(&mut self) -> Result<(), AdapterError> {
        (**self).connect().await
    }

    fn is_connected(&self) -> bool {
        (**self).is_connected()
    }

    async fn disconnect(&mut self) -> Result<(), AdapterError> {
        (**self).disconnect().await
    }

    async fn list_databases(&self) -> Result<Vec<MetaDatabase>, AdapterError> {
        (**self).list_databases().await
    }

    async fn list_schemas(&self, database: &str) -> Result<Vec<MetaSchema>, AdapterError> {
        (**self).list_schemas(database).await
    }

    async fn list_tables(&self, database: &str, schema: &str) -> Result<Vec<MetaTable>, AdapterError> {
        (**self).list_tables(database, schema).await
    }

    async fn list_columns(&self, table: &TableRef) -> Result<Vec<MetaColumn>, AdapterError> {
        (**self).list_columns(table).await
    }

    async fn list_foreign_keys(&self, table: &TableRef) -> Result<Vec<MetaForeignKey>, AdapterError> {
        (**self).list_foreign_keys(table).await
    }

    async fn list_indexes(&self, table: &TableRef) -> Result<Vec<MetaIndex>, AdapterError> {
        (**self).list_indexes(table).await
    }

    async fn list_triggers(&self, table: &TableRef) -> Result<Vec<MetaTrigger>, AdapterError> {
        (**self).list_triggers(table).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_postgres() {
        let caps = DatabaseCapabilities::postgres();
        assert!(caps.supports_databases);
        assert!(caps.supports_schemas);
        assert_eq!(caps.default_schema, Some("public".to_string()));
        assert_eq!(caps.case_sensitivity, CaseSensitivity::Sensitive);
    }

    #[test]
    fn test_capabilities_sqlite() {
        let caps = DatabaseCapabilities::sqlite();
        assert!(!caps.supports_databases);
        assert!(!caps.supports_schemas);
        assert_eq!(caps.default_database, Some("main".to_string()));
        assert_eq!(caps.default_schema, Some("main".to_string()));
    }

    #[test]
    fn test_capabilities_for_engine() {
        let caps = DatabaseCapabilities::for_engine("PostgreSQL");
        assert_eq!(caps.engine, "postgres");
        
        let caps = DatabaseCapabilities::for_engine("unknown_db");
        assert_eq!(caps.engine, "unknown");
    }

    #[test]
    fn test_effective_schema() {
        let caps = DatabaseCapabilities::postgres();
        assert_eq!(caps.effective_schema(None), "public");
        assert_eq!(caps.effective_schema(Some("custom")), "custom");
        
        let caps = DatabaseCapabilities::sqlite();
        assert_eq!(caps.effective_schema(None), "main");
    }

    #[test]
    fn test_table_ref() {
        let ref1 = TableRef::new("mydb", "public", "users");
        assert_eq!(ref1.fully_qualified(), "mydb.public.users");
        assert_eq!(ref1.schema_qualified(), "public.users");

        let ref2 = TableRef::from_name("users");
        assert_eq!(ref2.database, "main");
        assert_eq!(ref2.schema, "main");
    }

    #[test]
    fn test_adapter_error_display() {
        let err = AdapterError::NotSupported("triggers".to_string());
        assert!(err.to_string().contains("triggers"));
    }
}
