-- =============================================================================
-- Tables v2: Unified schema with internal IDs and engine capability model
-- =============================================================================

-- -----------------------------------------------------------------------------
-- Core Application Tables
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS themes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    author TEXT,
    description TEXT,
    theme_data TEXT NOT NULL,
    is_builtin INTEGER DEFAULT 1,
    is_active INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_themes_is_active ON themes(is_active);
CREATE INDEX IF NOT EXISTS idx_themes_name ON themes(name COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    engine TEXT NOT NULL,
    host TEXT,
    port INTEGER,
    database TEXT,
    username TEXT,
    uses_ssh INTEGER DEFAULT FALSE,
    uses_tls INTEGER DEFAULT FALSE,
    config_json TEXT NOT NULL,
    is_favorite INTEGER DEFAULT FALSE,
    color_tag TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_connected_at INTEGER,
    connection_count INTEGER DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_connections_engine ON connections(engine);
CREATE INDEX IF NOT EXISTS idx_connections_name ON connections(name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_connections_favorite ON connections(is_favorite);
CREATE INDEX IF NOT EXISTS idx_connections_last_used ON connections(last_connected_at DESC);

CREATE TABLE IF NOT EXISTS credentials (
    connection_id TEXT NOT NULL,
    credential_key TEXT NOT NULL,
    encrypted_value BLOB NOT NULL,
    nonce BLOB NOT NULL,
    encryption_version INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (connection_id, credential_key)
);

CREATE TABLE IF NOT EXISTS window_sessions (
    window_label TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);

-- -----------------------------------------------------------------------------
-- Engine Capability Registry (Source of Truth)
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS engine_capabilities (
    engine TEXT PRIMARY KEY,
    supports_databases INTEGER NOT NULL DEFAULT 1,
    supports_schemas INTEGER NOT NULL DEFAULT 1,
    supports_views INTEGER NOT NULL DEFAULT 1,
    supports_indexes INTEGER NOT NULL DEFAULT 1,
    supports_foreign_keys INTEGER NOT NULL DEFAULT 1,
    supports_triggers INTEGER NOT NULL DEFAULT 1,
    requires_qualified_names INTEGER NOT NULL DEFAULT 0,
    default_database TEXT,
    default_schema TEXT,
    case_sensitivity TEXT CHECK (case_sensitivity IN ('sensitive', 'insensitive', 'preserve')) DEFAULT 'insensitive'
);

-- Seed engine capabilities
INSERT OR REPLACE INTO engine_capabilities (engine, supports_databases, supports_schemas, requires_qualified_names, default_database, default_schema, case_sensitivity) VALUES
    ('postgres', 1, 1, 0, NULL, 'public', 'sensitive'),
    ('sqlite', 0, 0, 0, 'main', 'main', 'insensitive'),
    ('mysql', 1, 0, 0, NULL, NULL, 'insensitive'),
    ('mongodb', 1, 0, 0, NULL, NULL, 'sensitive'),
    ('redis', 0, 0, 0, '0', NULL, 'sensitive'),
    ('athena', 1, 1, 1, NULL, 'default', 'insensitive'),
    ('s3', 0, 0, 0, NULL, NULL, 'sensitive'),
    ('elasticsearch', 1, 0, 0, NULL, NULL, 'sensitive');

-- -----------------------------------------------------------------------------
-- Metadata Cache Tables (with Internal IDs for faster joins)
-- -----------------------------------------------------------------------------

-- Level 1: Databases
CREATE TABLE IF NOT EXISTS meta_databases (
    database_id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    UNIQUE (connection_id, name),
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_databases_connection ON meta_databases(connection_id);

-- Level 2: Schemas
CREATE TABLE IF NOT EXISTS meta_schemas (
    schema_id INTEGER PRIMARY KEY AUTOINCREMENT,
    database_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized for query convenience
    database TEXT NOT NULL,       -- Denormalized for query convenience
    name TEXT NOT NULL,
    schema_type TEXT NOT NULL CHECK (schema_type IN ('user', 'system', 'temporary', 'virtual')),
    UNIQUE (database_id, name),
    FOREIGN KEY (database_id) REFERENCES meta_databases(database_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_schemas_database ON meta_schemas(database_id);
CREATE INDEX IF NOT EXISTS idx_meta_schemas_type ON meta_schemas(schema_type);
CREATE INDEX IF NOT EXISTS idx_meta_schemas_connection ON meta_schemas(connection_id, database, name);

-- Level 3: Tables
CREATE TABLE IF NOT EXISTS meta_tables (
    table_id INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized for query convenience
    database TEXT NOT NULL,       -- Denormalized for query convenience
    schema TEXT NOT NULL,         -- Denormalized for query convenience
    table_name TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('table', 'view', 'materialized_view', 'foreign_table')),
    classification TEXT NOT NULL CHECK (classification IN ('user', 'system')),
    last_introspected_at INTEGER NOT NULL,
    UNIQUE (schema_id, table_name),
    -- Keep natural key for compatibility
    UNIQUE (connection_id, database, schema, table_name),
    FOREIGN KEY (schema_id) REFERENCES meta_schemas(schema_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_tables_schema ON meta_tables(schema_id);
CREATE INDEX IF NOT EXISTS idx_meta_tables_connection ON meta_tables(connection_id);
CREATE INDEX IF NOT EXISTS idx_meta_tables_lookup ON meta_tables(connection_id, database, schema);

-- Level 3b: Columns (uses table_id for joins)
CREATE TABLE IF NOT EXISTS meta_columns (
    column_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized for query convenience
    database TEXT NOT NULL,       -- Denormalized
    schema TEXT NOT NULL,         -- Denormalized
    table_name TEXT NOT NULL,     -- Denormalized
    ordinal_position INTEGER NOT NULL,
    column_name TEXT NOT NULL,
    raw_type TEXT NOT NULL,
    logical_type TEXT NOT NULL,
    nullable INTEGER DEFAULT 1,
    default_value TEXT,
    is_primary_key INTEGER DEFAULT 0,
    UNIQUE (table_id, column_name),
    FOREIGN KEY (table_id) REFERENCES meta_tables(table_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_columns_table ON meta_columns(table_id);
CREATE INDEX IF NOT EXISTS idx_meta_columns_lookup ON meta_columns(connection_id, database, schema, table_name);

-- Level 4: Indexes
CREATE TABLE IF NOT EXISTS meta_indexes (
    index_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized
    database TEXT NOT NULL,       -- Denormalized
    schema TEXT NOT NULL,         -- Denormalized
    table_name TEXT NOT NULL,     -- Denormalized
    index_name TEXT NOT NULL,
    is_unique INTEGER DEFAULT 0,
    UNIQUE (table_id, index_name),
    FOREIGN KEY (table_id) REFERENCES meta_tables(table_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_indexes_table ON meta_indexes(table_id);

-- Level 4b: Index Columns
CREATE TABLE IF NOT EXISTS meta_index_columns (
    index_id INTEGER NOT NULL,
    column_name TEXT NOT NULL,
    seq_no INTEGER NOT NULL,
    PRIMARY KEY (index_id, column_name),
    FOREIGN KEY (index_id) REFERENCES meta_indexes(index_id) ON DELETE CASCADE
);

-- Level 4: Foreign Keys (with constraint_hash for stability)
CREATE TABLE IF NOT EXISTS meta_foreign_keys (
    fk_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized
    database TEXT NOT NULL,       -- Denormalized
    schema TEXT NOT NULL,         -- Denormalized
    table_name TEXT NOT NULL,     -- Denormalized
    column_name TEXT NOT NULL,
    ref_schema TEXT NOT NULL,
    ref_table TEXT NOT NULL,
    ref_column TEXT NOT NULL,
    constraint_name TEXT,         -- May be NULL for engines that don't provide it
    constraint_hash TEXT NOT NULL, -- Deterministic hash for deduplication
    seq_no INTEGER NOT NULL DEFAULT 1,
    UNIQUE (table_id, constraint_hash, seq_no),
    FOREIGN KEY (table_id) REFERENCES meta_tables(table_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_fk_table ON meta_foreign_keys(table_id);
CREATE INDEX IF NOT EXISTS idx_meta_fk_lookup ON meta_foreign_keys(connection_id, database, schema, table_name);
CREATE INDEX IF NOT EXISTS idx_meta_fk_ref ON meta_foreign_keys(connection_id, ref_schema, ref_table);

-- Level 4: Triggers
CREATE TABLE IF NOT EXISTS meta_triggers (
    trigger_id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL,
    connection_id TEXT NOT NULL,  -- Denormalized
    database TEXT NOT NULL,       -- Denormalized
    schema TEXT NOT NULL,         -- Denormalized
    table_name TEXT NOT NULL,     -- Denormalized
    trigger_name TEXT NOT NULL,
    event TEXT NOT NULL CHECK (event IN ('INSERT', 'UPDATE', 'DELETE', 'TRUNCATE')),
    timing TEXT NOT NULL CHECK (timing IN ('BEFORE', 'AFTER', 'INSTEAD OF')),
    UNIQUE (table_id, trigger_name),
    FOREIGN KEY (table_id) REFERENCES meta_tables(table_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_meta_triggers_table ON meta_triggers(table_id);
