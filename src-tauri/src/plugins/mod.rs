/// True Plugins System - Unlimited scalability for 100+ commands
/// 
/// This plugin system provides:
/// - Dynamic plugin discovery and registration
/// - Automatic command registration without manual listing
/// - Plugin dependency management
/// - Runtime plugin enable/disable
/// - Hot-loading support for development
/// - Metadata-driven plugin management

pub mod core;
pub mod builtin;
pub mod discovery;

// Re-export main plugin interfaces
pub use core::PluginMetadata;
pub use discovery::{PluginDiscovery, get_available_plugins, enable_plugin, disable_plugin, get_plugin_info, initialize_all_plugins};


// Legacy compatibility
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub command_count: usize,
    pub supported_engines: Vec<String>,
}

impl From<PluginMetadata> for PluginInfo {
    fn from(metadata: PluginMetadata) -> Self {
        Self {
            name: metadata.name,
            description: metadata.description,
            command_count: metadata.command_count,
            supported_engines: metadata.supported_engines,
        }
    }
}

/// Get information about all available plugins (legacy compatibility)
#[tauri::command]
pub fn get_available_plugins_legacy() -> Vec<PluginInfo> {
    let discovery = PluginDiscovery::new();
    discovery.discover_builtin_plugins().unwrap_or_else(|e| {
        eprintln!("Failed to discover plugins: {}", e);
    });
    discovery.get_manager()
        .list_available_plugins()
        .into_iter()
        .map(PluginInfo::from)
        .collect()
}
