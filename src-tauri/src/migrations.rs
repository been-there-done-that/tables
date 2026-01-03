use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json;
use log::{info, debug, warn, error};

const CREATE_THEMES_TABLE: &str = r#"
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
"#;

const CREATE_CONNECTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS connections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    engine TEXT NOT NULL,
    
    -- Optional summary fields (for list views & indexing only)
    host TEXT,
    port INTEGER,
    database TEXT,
    username TEXT,
    
    -- Transport/security summary flags
    uses_ssh INTEGER DEFAULT FALSE,
    uses_tls INTEGER DEFAULT FALSE,
    
    -- Canonical, versioned configuration
    config_json TEXT NOT NULL,
    
    -- UX / metadata
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
"#;

const CREATE_CREDENTIALS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS credentials (
    connection_id TEXT NOT NULL,
    credential_key TEXT NOT NULL,
    encrypted_value BLOB NOT NULL,
    nonce BLOB NOT NULL,
    encryption_version INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (connection_id, credential_key)
);
"#;

const CREATE_META_TABLES: &str = r#"
CREATE TABLE IF NOT EXISTS meta_tables (
    connection_id TEXT NOT NULL,
    database TEXT NOT NULL DEFAULT 'main',
    schema TEXT NOT NULL DEFAULT 'main',
    table_name TEXT NOT NULL,
    type TEXT NOT NULL, -- 'table' or 'view'
    classification TEXT NOT NULL, -- 'user', 'system', 'fts', 'virtual'
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
    ref_table TEXT NOT NULL,
    ref_column TEXT NOT NULL,
    PRIMARY KEY (connection_id, database, schema, table_name, column_name, ref_table, ref_column),
    FOREIGN KEY (connection_id, database, schema, table_name) REFERENCES meta_tables(connection_id, database, schema, table_name) ON DELETE CASCADE
);
"#;

const CREATE_WINDOW_SESSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS window_sessions (
    window_label TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE CASCADE
);
"#;

#[derive(Debug, Deserialize)]
struct ThemeSeed {
    id: String,
    name: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "ui")]
    _ui: serde_json::Value, // kept for validation
    #[serde(rename = "syntax")]
    _syntax: serde_json::Value,
    #[serde(rename = "editor")]
    _editor: Option<serde_json::Value>,
}

const BUILTIN_THEME_FILES: &[(&str, &str)] = &[
    ("monokai", include_str!("themes/monokai.json")),
    ("monokai-light", include_str!("themes/monokai-light.json")),
    ("dracula", include_str!("themes/dracula.json")),
    ("dracula-light", include_str!("themes/dracula-light.json")),
    ("nord", include_str!("themes/nord.json")),
    ("nord-light", include_str!("themes/nord-light.json")),
    ("solarized-dark", include_str!("themes/solarized-dark.json")),
    ("solarized-light", include_str!("themes/solarized-light.json")),
    ("gruvbox-dark", include_str!("themes/gruvbox-dark.json")),
    ("gruvbox-light", include_str!("themes/gruvbox-light.json")),
    ("tokyo-night-dark", include_str!("themes/tokyo-night-dark.json")),
    ("tokyo-night-light", include_str!("themes/tokyo-night-light.json")),
    ("catppuccin-mocha", include_str!("themes/catppuccin-mocha.json")),
    ("catppuccin-latte", include_str!("themes/catppuccin-latte.json")),
    ("forest-dark", include_str!("themes/forest-dark.json")),
    ("forest-light", include_str!("themes/forest-light.json")),
    ("oceanic-dark", include_str!("themes/oceanic-dark.json")),
    ("oceanic-light", include_str!("themes/oceanic-light.json")),
    ("sunset-dark", include_str!("themes/sunset-dark.json")),
    ("sunset-light", include_str!("themes/sunset-light.json")),
    ("iceberg-dark", include_str!("themes/iceberg-dark.json")),
    ("iceberg-light", include_str!("themes/iceberg-light.json")),
    ("snow-light", include_str!("themes/snow-light.json")),
    ("snow-dark", include_str!("themes/snow-dark.json")),
    ("ember-light", include_str!("themes/ember-light.json")),
    ("ember-dark", include_str!("themes/ember-dark.json")),
    ("neon-light", include_str!("themes/neon-light.json")),
    ("neon-dark", include_str!("themes/neon-dark.json")),
    ("meadow-light", include_str!("themes/meadow-light.json")),
    ("meadow-dark", include_str!("themes/meadow-dark.json")),
    ("paper", include_str!("themes/paper.json")),
    ("paper-dark", include_str!("themes/paper-dark.json")),
];

pub fn apply(conn: &Connection, now_fn: impl Fn() -> i64) -> Result<(), String> {
    debug!("Applying database migrations");
    
    debug!("Creating themes table");
    conn.execute_batch(CREATE_THEMES_TABLE)
        .map_err(|e| {
            error!("Failed to create themes table: {}", e);
            format!("Failed to create themes table: {e}")
        })?;

    debug!("Creating connections table");
    conn.execute_batch(CREATE_CONNECTIONS_TABLE)
        .map_err(|e| {
            error!("Failed to create connections table: {}", e);
            format!("Failed to create connections table: {e}")
        })?;

    debug!("Creating credentials table");
    conn.execute_batch(CREATE_CREDENTIALS_TABLE)
        .map_err(|e| {
            error!("Failed to create credentials table: {}", e);
            format!("Failed to create credentials table: {e}")
        })?;

    debug!("Creating meta cache tables");
    conn.execute_batch(CREATE_META_TABLES)
        .map_err(|e| {
            error!("Failed to create meta tables: {}", e);
            format!("Failed to create meta tables: {e}")
        })?;

    debug!("Creating window sessions table");
    conn.execute_batch(CREATE_WINDOW_SESSIONS_TABLE)
        .map_err(|e| {
            error!("Failed to create window sessions table: {}", e);
            format!("Failed to create window sessions table: {e}")
        })?;

    // Safe Migration: Ensure 'database' column exists in all meta tables
    // Since we changed the Primary Key, we should drop and recreate these tables if 'database' is missing.
    // This ensures a clean new hierarchy cache.
    let tables_to_check = vec![
        "meta_tables",
        "meta_columns",
        "meta_indexes",
        "meta_index_columns",
        "meta_foreign_keys",
    ];

    for table in tables_to_check {
        let has_database_col: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name='database'", table),
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        if has_database_col == 0 {
             debug!("Migrating {} - dropping to recreate with new PK (database hierarchy)", table);
             conn.execute(&format!("DROP TABLE IF EXISTS {}", table), [])
                .map_err(|e| format!("Failed to drop table {}: {e}", table))?;
        }
    }
    
    // Re-run batch to create them with correct PKs if they were dropped
    conn.execute_batch(CREATE_META_TABLES)
        .map_err(|e| format!("Failed to ensure meta tables after drop: {e}"))?;

    let ts = now_fn();
    info!("Seeding {} builtin themes", BUILTIN_THEME_FILES.len());

    for (idx, (id, json)) in BUILTIN_THEME_FILES.iter().enumerate() {
        debug!("Seeding theme '{}' (index {})", id, idx);
        let parsed: ThemeSeed = serde_json::from_str(json)
            .map_err(|e| {
                error!("Failed to parse theme '{}': {}", id, e);
                format!("Failed to parse theme {}: {e}", id)
            })?;

        conn.execute(
            "INSERT INTO themes (id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name,
               author=excluded.author,
               description=excluded.description,
               theme_data=excluded.theme_data,
               is_builtin=1,
               updated_at=excluded.updated_at",
            params![
                parsed.id,
                parsed.name,
                parsed.author,
                parsed.description,
                json,
                if idx == 0 { 1 } else { 0 },
                ts
            ],
        )
        .map_err(|e| {
            error!("Failed to seed theme '{}': {}", id, e);
            format!("Failed to seed theme {}: {e}", id)
        })?;
    }

    debug!("Ensuring exactly one active theme");
    // Ensure exactly one active theme.
    let active_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM themes WHERE is_active = 1", [], |row| row.get(0))
        .unwrap_or(0);
    if active_count == 0 {
        warn!("No active theme found, setting default");
        conn.execute("UPDATE themes SET is_active = 0", [])
            .map_err(|e| {
                error!("Failed to clear active theme: {}", e);
                format!("Failed to clear active theme: {e}")
            })?;
        conn.execute(
            "UPDATE themes SET is_active = 1 WHERE id = (SELECT id FROM themes ORDER BY name LIMIT 1)",
            [],
        )
        .map_err(|e| {
            error!("Failed to set default active theme: {}", e);
            format!("Failed to set default active theme: {e}")
        })?;
    }

    info!("Database migrations applied successfully");
    Ok(())
}
