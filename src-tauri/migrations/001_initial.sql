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

CREATE TABLE IF NOT EXISTS meta_databases (
    connection_id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (connection_id, name),
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_schemas (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL,
    name TEXT NOT NULL,
    schema_type TEXT NOT NULL,
    PRIMARY KEY (connection_id, database, name),
    FOREIGN KEY (connection_id, database) REFERENCES meta_databases(connection_id, name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_tables (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    type TEXT NOT NULL,
    classification TEXT NOT NULL,
    last_introspected_at INTEGER NOT NULL,
    PRIMARY KEY (connection_id, database, schema, table_name),
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_columns (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    ordinal_position INTEGER NOT NULL,
    column_name TEXT NOT NULL,
    raw_type TEXT NOT NULL,
    logical_type TEXT NOT NULL,
    nullable INTEGER DEFAULT 1,
    default_value TEXT,
    is_primary_key INTEGER DEFAULT 0,
    PRIMARY KEY (connection_id, database, schema, table_name, column_name),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_indexes (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    index_name TEXT NOT NULL,
    is_unique INTEGER DEFAULT 0,
    PRIMARY KEY (connection_id, database, schema, table_name, index_name),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_index_columns (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    index_name TEXT NOT NULL,
    column_name TEXT NOT NULL,
    seq_no INTEGER NOT NULL,
    PRIMARY KEY (connection_id, database, schema, table_name, index_name, column_name),
    FOREIGN KEY (connection_id, database, schema, table_name, index_name) REFERENCES meta_indexes(connection_id, database, schema, table_name, index_name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_foreign_keys (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    column_name TEXT NOT NULL,
    ref_schema TEXT NOT NULL DEFAULT 'main',
    ref_table TEXT NOT NULL,
    ref_column TEXT NOT NULL,
    constraint_name TEXT NOT NULL,
    seq_no INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (connection_id, database, schema, table_name, constraint_name, seq_no),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS meta_triggers (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    trigger_name TEXT NOT NULL,
    event TEXT NOT NULL,
    timing TEXT NOT NULL,
    PRIMARY KEY (connection_id, database, schema, table_name, trigger_name),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS window_sessions (
    window_label TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);
