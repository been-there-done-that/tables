use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub editor_font_family: String,
    pub editor_font_size: u32,
    // Add other settings here as needed
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            editor_font_family: "\"Fira Code\", monospace".to_string(),
            editor_font_size: 14,
        }
    }
}

pub fn get_settings(conn: &Connection) -> Result<AppSettings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut settings = AppSettings::default();

    for row in rows {
        if let Ok((key, value)) = row {
            match key.as_str() {
                "editor_font_family" => settings.editor_font_family = value,
                "editor_font_size" => {
                    if let Ok(size) = value.parse() {
                        settings.editor_font_size = size;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(settings)
}

pub fn update_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, strftime('%s', 'now'))
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = strftime('%s', 'now')",
        params![key, value],
    )?;
    Ok(())
}
