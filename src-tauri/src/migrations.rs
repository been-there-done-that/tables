use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json;

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
    conn.execute_batch(CREATE_THEMES_TABLE)
        .map_err(|e| format!("Failed to create themes table: {e}"))?;

    conn.execute_batch(CREATE_CONNECTIONS_TABLE)
        .map_err(|e| format!("Failed to create connections table: {e}"))?;

    let ts = now_fn();

    for (idx, (id, json)) in BUILTIN_THEME_FILES.iter().enumerate() {
        let parsed: ThemeSeed = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse theme {}: {e}", id))?;

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
        .map_err(|e| format!("Failed to seed theme {}: {e}", id))?;
    }

    // Ensure exactly one active theme.
    let active_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM themes WHERE is_active = 1", [], |row| row.get(0))
        .unwrap_or(0);
    if active_count == 0 {
        conn.execute("UPDATE themes SET is_active = 0", [])
            .map_err(|e| format!("Failed to clear active theme: {e}"))?;
        conn.execute(
            "UPDATE themes SET is_active = 1 WHERE id = (SELECT id FROM themes ORDER BY name LIMIT 1)",
            [],
        )
        .map_err(|e| format!("Failed to set default active theme: {e}"))?;
    }

    Ok(())
}
