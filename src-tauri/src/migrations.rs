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
    ("dracula", include_str!("themes/dracula.json")),
    ("nord", include_str!("themes/nord.json")),
    ("solarized-dark", include_str!("themes/solarized-dark.json")),
];

pub fn apply(conn: &Connection, now_fn: impl Fn() -> i64) -> Result<(), String> {
    conn.execute_batch(CREATE_THEMES_TABLE)
        .map_err(|e| format!("Failed to create themes table: {e}"))?;

    let ts = now_fn();

    for (idx, (id, json)) in BUILTIN_THEME_FILES.iter().enumerate() {
        let parsed: ThemeSeed = serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse theme {}: {e}", id))?;

        conn.execute(
            "INSERT OR IGNORE INTO themes (id, name, author, description, theme_data, is_builtin, is_active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?7)",
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
