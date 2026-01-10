use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseEngine {
    Postgres,
    MySql,
    Sqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamespaceKind {
    Database,       // Normal database (Postgres)
    Schema,         // Normal schema (Postgres, SQL Standard)
    LogicalGroup,   // Grouping for engines without strict schemas (MySQL, SQLite main)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegerSize {
    Small,      // 2 bytes (int2, smallint)
    Normal,     // 4 bytes (int4, int)
    Big,        // 8 bytes (int8, bigint)
    Unbounded,  // SQLite specific
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloatPrecision {
    Single,     // 4 bytes (float4, real)
    Double,     // 8 bytes (float8, double precision)
}

/// The unified, cross-database normalized type system.
/// This enum is the "source of truth" for the UI and completion engine logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub enum NormalizedType {
    // Scalar
    Integer { size: IntegerSize, unsigned: bool },
    Float { precision: FloatPrecision },
    Decimal,
    Boolean,
    Text,
    Binary,

    // Temporal
    Date,
    Time,
    DateTime { timezone: bool },
    Interval,

    // Structured
    Json,
    Uuid,

    // Advanced
    Enum { values: Vec<String> },
    Array { element: Box<NormalizedType> },
    
    // Composite (row types) - Postgres 2026+
    Composite { fields: Vec<(String, Box<NormalizedType>)> },
    
    // Range types (Postgres 9.2+)
    Range { element: Box<NormalizedType> },
    
    // Multirange types (Postgres 14+)
    MultiRange { element: Box<NormalizedType> },

    // Fallbacks
    Custom { name: String }, // Domain types, user-defined types
    Unknown,
}

// --- Engine-Specific Metadata Structures ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresTypeMeta {
    pub raw_type: String,          // typname (e.g., "int4", "my_enum")
    pub base_type: Option<String>, // typbasetype (underlying type for domains)
    pub type_kind: char,           // typtype: 'b' (base), 'e' (enum), 'd' (domain), 'c' (composite), etc.
    pub type_category: char,       // typcategory: 'N' (numeric), 'S' (string), etc.
    pub is_array: bool,
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySqlTypeMeta {
    pub data_type: String,         // e.g., "int", "varchar"
    pub column_type: String,       // e.g., "int(11) unsigned"
    pub is_unsigned: bool,
    pub enum_values: Option<Vec<String>>,
}

/// SQLite type affinity (how SQLite stores values)
/// Based on SQLite's affinity determination rules from declared type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqliteAffinity {
    Integer,  // INT, INTEGER, TINYINT, SMALLINT, MEDIUMINT, BIGINT, etc.
    Text,     // TEXT, VARCHAR, CLOB, CHARACTER, etc.
    Blob,     // BLOB, no type specified
    Real,     // REAL, DOUBLE, FLOAT
    Numeric,  // NUMERIC, DECIMAL, BOOLEAN, DATE, DATETIME, etc.
}

impl Default for SqliteAffinity {
    fn default() -> Self {
        SqliteAffinity::Blob // No type = BLOB affinity per SQLite rules
    }
}

/// Semantic hint inferred from type metadata or extension presence.
/// For SQLite: heuristic from declared type (disabled for STRICT tables).
/// For Postgres: extension-aware meaning derived from pg_extension.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticHint {
    None,                           // No special hint
    Uuid,                           // UUID, GUID
    Json,                           // JSON, JSONB
    DateTime,                       // DATETIME, TIMESTAMP
    Date,                           // DATE
    Time,                           // TIME
    Boolean,                        // BOOL, BOOLEAN
    Decimal,                        // MONEY, DECIMAL, CURRENCY
    Enum { values: Vec<String> },   // Detected from CHECK constraints or pg_enum
    
    // Extension-aware hints (Postgres 2026+)
    CaseInsensitiveText,            // citext extension
    Spatial,                        // PostGIS: geometry, geography
    Embedding { dimensions: Option<usize> },  // pgvector: vector(N)
    Hierarchy,                      // ltree extension
    KeyValue,                       // hstore extension
    TimeSeries,                     // TimescaleDB hypertables
    Encrypted,                      // pgcrypto
}

impl Default for SemanticHint {
    fn default() -> Self {
        SemanticHint::None
    }
}

/// Metadata about installed PostgreSQL extensions.
/// Queried from pg_extension and cached per-database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub version: String,
    pub schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteTypeMeta {
    pub declared_type: String,          // Verbatim from table_xinfo (e.g., "VARCHAR(36)")
    pub affinity: SqliteAffinity,       // Computed affinity per SQLite rules
    pub semantic_hint: SemanticHint,    // Heuristic inference (disabled for STRICT)
    pub is_strict_table: bool,          // Table uses STRICT mode (SQLite 3.37+)
    pub is_generated: bool,             // Virtual or stored generated column
    pub is_virtual_table: bool,         // Table is virtual (FTS5, R-Tree, etc.)
}

/// Container for engine-specific metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "engine", content = "meta", rename_all = "snake_case")]
pub enum EngineTypeMeta {
    Postgres(PostgresTypeMeta),
    MySql(MySqlTypeMeta),
    Sqlite(SqliteTypeMeta),
    None,
}

/// Lossless representation of the database type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineType {
    pub engine: DatabaseEngine,
    pub raw_type: String,
    pub metadata: EngineTypeMeta,
}
