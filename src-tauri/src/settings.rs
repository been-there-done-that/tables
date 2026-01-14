use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowLayout {
    pub sidebar_left_visible: bool,
    pub sidebar_left_ratio: f64,
    pub sidebar_right_visible: bool,
    pub sidebar_right_ratio: f64,
    pub sidebar_bottom_visible: bool,
    pub sidebar_bottom_ratio: f64,
    pub active_right_panel: Option<String>,
    pub selected_database: Option<String>,
    pub expanded_nodes: HashMap<String, Vec<String>>,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            sidebar_left_visible: true,
            sidebar_left_ratio: 0.2,
            sidebar_right_visible: false,
            sidebar_right_ratio: 0.75,
            sidebar_bottom_visible: false,
            sidebar_bottom_ratio: 0.7,
            active_right_panel: None,
            selected_database: None,
            expanded_nodes: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub editor_font_family: String,
    pub editor_font_size: u32,
    // Per-window layout settings
    pub window_layouts: HashMap<String, WindowLayout>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            editor_font_family: "\"Fira Code\", monospace".to_string(),
            editor_font_size: 14,
            window_layouts: HashMap::new(),
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
                k if k.starts_with("window:") => {
                    // Format: window:{label}:{property} OR window:{label}:conn:{id}:expanded
                    let parts: Vec<&str> = k.split(':').collect();
                    if parts.len() >= 3 {
                        let label = parts[1].to_string();
                        let property = parts[2];
                        
                        let layout = settings.window_layouts.entry(label).or_insert_with(WindowLayout::default);
                        
                        if parts.len() == 5 && property == "conn" && parts[4] == "expanded" {
                            let conn_id = parts[3].to_string();
                            let nodes = value.split(',').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect();
                            layout.expanded_nodes.insert(conn_id, nodes);
                        } else if parts.len() == 3 {
                            match property {
                                "sidebar_left_visible" => layout.sidebar_left_visible = value == "true",
                                "sidebar_left_ratio" => {
                                    if let Ok(ratio) = value.parse() {
                                        layout.sidebar_left_ratio = ratio;
                                    }
                                }
                                "sidebar_right_visible" => layout.sidebar_right_visible = value == "true",
                                "sidebar_right_ratio" => {
                                    if let Ok(ratio) = value.parse() {
                                        layout.sidebar_right_ratio = ratio;
                                    }
                                }
                                "sidebar_bottom_visible" => layout.sidebar_bottom_visible = value == "true",
                                "sidebar_bottom_ratio" => {
                                    if let Ok(ratio) = value.parse() {
                                        layout.sidebar_bottom_ratio = ratio;
                                    }
                                }
                                "active_right_panel" => {
                                    if !value.is_empty() {
                                        layout.active_right_panel = Some(value);
                                    } else {
                                        layout.active_right_panel = None;
                                    }
                                }
                                "selected_database" => {
                                    if !value.is_empty() {
                                        layout.selected_database = Some(value);
                                    }
                                }
                                _ => {}
                            }
                        }
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
