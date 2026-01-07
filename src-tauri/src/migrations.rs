use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json;
use log::{info, debug, warn, error};

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("../migrations/001_initial.sql")),
    ("002_add_type_system", include_str!("../migrations/002_add_type_system.sql")),
    ("003_add_namespace_kind", include_str!("../migrations/003_add_namespace_kind.sql")),
    ("004_create_settings_table", include_str!("../migrations/004_create_settings_table.sql")),
];

#[derive(Debug, Deserialize)]
struct ThemeSeed {
    id: String,
    name: String,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "ui")]
    _ui: serde_json::Value,
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

pub fn apply(conn: &mut Connection, now_fn: impl Fn() -> i64) -> Result<(), String> {
    debug!("Applying database migrations");
    
    // Disable foreign keys during migration to allow dropping/recreating tables with constraints
    conn.pragma_update(None, "foreign_keys", "OFF")
        .map_err(|e| format!("Failed to disable foreign keys for migration: {e}"))?;

    // 1. Ensure migrations tracking table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS meta_migrations (
            name TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    ).map_err(|e| format!("Failed to create migrations table: {e}"))?;

    // 2. Fetch applied migrations
    let applied: std::collections::HashSet<String> = {
        let mut stmt = conn.prepare("SELECT name FROM meta_migrations")
            .map_err(|e| format!("Failed to prepare migrations query: {e}"))?;
        let rows = stmt.query_map([], |row| row.get(0))
            .map_err(|e| format!("Failed to fetch applied migrations: {e}"))?;
        
        let mut set = std::collections::HashSet::new();
        for row_res in rows {
            set.insert(row_res.map_err(|e| format!("Error reading migration name: {e}"))?);
        }
        set
    };

    // 3. Apply missing migrations in transactions
    for (name, sql) in MIGRATIONS {
        if !applied.contains(*name) {
            info!("Applying migration: {}", name);
            
            let tx = conn.transaction()
                .map_err(|e| format!("Failed to start transaction for {}: {}", name, e))?;
            
            tx.execute_batch(sql)
                .map_err(|e| {
                    error!("Failed executing SQL for migration {}: {}", name, e);
                    format!("Failed to apply migration {}: {e}", name)
                })?;
            
            tx.execute(
                "INSERT INTO meta_migrations (name, applied_at) VALUES (?1, ?2)",
                params![name, now_fn()],
            ).map_err(|e| format!("Failed to record migration {}: {e}", name))?;
            
            tx.commit()
                .map_err(|e| format!("Failed to commit migration {}: {}", name, e))?;
        }
    }

    // 4. Seeding (always run, with UPSERT)
    seed_themes(conn, now_fn)?;

    // Re-enable foreign keys
    if let Err(e) = conn.pragma_update(None, "foreign_keys", "ON") {
        error!("Failed to re-enable foreign keys: {}", e);
    }

    info!("Database migrations applied successfully");
    Ok(())
}

fn seed_themes(conn: &Connection, now_fn: impl Fn() -> i64) -> Result<(), String> {
    let ts = now_fn();
    debug!("Seeding {} builtin themes", BUILTIN_THEME_FILES.len());

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
        ).map_err(|e| format!("Failed to seed theme {}: {e}", id))?;
    }

    // Ensure exactly one active theme.
    let active_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM themes WHERE is_active = 1", [], |row| row.get(0))
        .unwrap_or(0);
    
    if active_count == 0 {
        warn!("No active theme found, setting default");
        conn.execute(
            "UPDATE themes SET is_active = 1 WHERE id = (SELECT id FROM themes ORDER BY name LIMIT 1)",
            [],
        ).map_err(|e| format!("Failed to set default active theme: {e}"))?;
    }

    Ok(())
}
