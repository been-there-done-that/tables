use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub editor_font_family: String,
    pub editor_font_size: u32,
    // Layout settings
    pub sidebar_left_visible: bool,
    pub sidebar_left_ratio: f64,
    pub sidebar_right_visible: bool,
    pub sidebar_right_ratio: f64,
    pub sidebar_bottom_visible: bool,
    pub sidebar_bottom_ratio: f64,
    // Last selected database (for postgres connections)
    pub selected_database: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            editor_font_family: "\"Fira Code\", monospace".to_string(),
            editor_font_size: 14,
            // Layout defaults
            sidebar_left_visible: true,
            sidebar_left_ratio: 0.2,
            sidebar_right_visible: false,
            sidebar_right_ratio: 0.75,
            sidebar_bottom_visible: false,
            sidebar_bottom_ratio: 0.7,
            // No database selected by default
            selected_database: None,
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
                "sidebar_left_visible" => settings.sidebar_left_visible = value == "true",
                "sidebar_left_ratio" => {
                    if let Ok(ratio) = value.parse() {
                        settings.sidebar_left_ratio = ratio;
                    }
                }
                "sidebar_right_visible" => settings.sidebar_right_visible = value == "true",
                "sidebar_right_ratio" => {
                    if let Ok(ratio) = value.parse() {
                        settings.sidebar_right_ratio = ratio;
                    }
                }
                "sidebar_bottom_visible" => settings.sidebar_bottom_visible = value == "true",
                "sidebar_bottom_ratio" => {
                    if let Ok(ratio) = value.parse() {
                        settings.sidebar_bottom_ratio = ratio;
                    }
                }
                "selected_database" => {
                    if !value.is_empty() {
                        settings.selected_database = Some(value);
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
