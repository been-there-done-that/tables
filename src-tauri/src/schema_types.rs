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

    // Structured
    Json,
    Uuid,

    // Advanced
    Enum { values: Vec<String> },
    Array { element: Box<NormalizedType> },

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteTypeMeta {
    pub declared_type: String,     // e.g., "INTEGER", "VARCHAR(50)"
    pub affinity: String,          // e.g., "INTEGER", "TEXT" (Using string for simple serialization for now)
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
