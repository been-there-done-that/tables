/// Simple plugin system that groups commands by functionality
/// This provides better organization without the complexity of dynamic command registration

/// Plugin information for frontend discovery
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub command_count: usize,
    pub supported_engines: Vec<String>,
}

/// Get information about all available plugins
#[tauri::command]
pub fn get_available_plugins() -> Vec<PluginInfo> {
    vec![
        PluginInfo {
            name: "theme".to_string(),
            description: "Theme management commands".to_string(),
            command_count: 3,
            supported_engines: vec![],
        },
        PluginInfo {
            name: "connection".to_string(),
            description: "Database connection management".to_string(),
            command_count: 11,
            supported_engines: vec![
                "postgresql".to_string(),
                "mysql".to_string(),
                "sqlite".to_string(),
                "mongodb".to_string(),
                "redis".to_string(),
                "elasticsearch".to_string(),
                "s3".to_string(),
                "athena".to_string(),
            ],
        },
        PluginInfo {
            name: "aws".to_string(),
            description: "AWS services integration".to_string(),
            command_count: 9,
            supported_engines: vec!["s3".to_string()],
        },
        PluginInfo {
            name: "redis".to_string(),
            description: "Redis database operations".to_string(),
            command_count: 6,
            supported_engines: vec!["redis".to_string()],
        },
        PluginInfo {
            name: "athena".to_string(),
            description: "Amazon Athena query service".to_string(),
            command_count: 7,
            supported_engines: vec!["athena".to_string()],
        },
    ]
}
