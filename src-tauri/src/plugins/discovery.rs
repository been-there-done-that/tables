use crate::plugins::core::{PluginManager, PluginMetadata, PluginInitResult};
use crate::plugins::builtin::*;
use tauri::{AppHandle, State};
use serde::{Serialize, Deserialize};

/// Plugin discovery and registration system
pub struct PluginDiscovery {
    manager: PluginManager,
}

impl PluginDiscovery {
    pub fn new() -> Self {
        Self {
            manager: PluginManager::new(),
        }
    }

    /// Auto-discover and register all built-in plugins
    pub fn discover_builtin_plugins(&self) -> Result<(), String> {
        let registry = crate::plugins::core::get_plugin_registry();
        let mut reg = registry.lock().unwrap();
        
        // Register all built-in plugins
        reg.register(create_theme_plugin());
        reg.register(create_connection_plugin());
        reg.register(create_aws_plugin());
        reg.register(create_redis_plugin());
        reg.register(create_athena_plugin());
        
        Ok(())
    }

    /// Initialize all enabled plugins
    pub fn initialize_plugins(&self, app: &AppHandle) -> Vec<PluginInitResult> {
        self.manager.initialize_all_plugins(app)
    }

    /// Get plugin manager for runtime operations
    pub fn get_manager(&self) -> &PluginManager {
        &self.manager
    }
}

/// Plugin commands for frontend management
#[tauri::command]
pub fn get_available_plugins() -> Vec<PluginMetadata> {
    let discovery = PluginDiscovery::new();
    discovery.discover_builtin_plugins().unwrap_or_else(|e| {
        eprintln!("Failed to discover plugins: {}", e);
    });
    discovery.get_manager().list_available_plugins()
}

#[tauri::command]
pub fn enable_plugin(name: String) -> Result<String, String> {
    let registry = crate::plugins::core::get_plugin_registry();
    let mut reg = registry.lock().unwrap();
    reg.enable_plugin(&name)?;
    Ok(format!("Plugin '{}' enabled successfully", name))
}

#[tauri::command]
pub fn disable_plugin(name: String) -> Result<String, String> {
    let registry = crate::plugins::core::get_plugin_registry();
    let mut reg = registry.lock().unwrap();
    reg.disable_plugin(&name)?;
    Ok(format!("Plugin '{}' disabled successfully", name))
}

#[tauri::command]
pub fn get_plugin_info(name: String) -> Option<PluginMetadata> {
    let discovery = PluginDiscovery::new();
    discovery.get_manager().get_plugin_info(&name)
}

#[tauri::command]
pub fn initialize_all_plugins(app: AppHandle) -> Vec<PluginInitResult> {
    let discovery = PluginDiscovery::new();
    discovery.discover_builtin_plugins().unwrap_or_else(|e| {
        eprintln!("Failed to discover plugins: {}", e);
    });
    discovery.initialize_plugins(&app)
}

/// Plugin dependency resolver
pub struct DependencyResolver;

impl DependencyResolver {
    pub fn resolve_dependencies(plugin_names: &[String]) -> Result<Vec<String>, String> {
        let registry = crate::plugins::core::get_plugin_registry();
        let reg = registry.lock().unwrap();
        
        let mut resolved = Vec::new();
        let mut to_resolve = plugin_names.to_vec();
        
        while !to_resolve.is_empty() {
            let current = to_resolve.pop().unwrap();
            
            if resolved.contains(&current) {
                continue;
            }
            
            if let Some(metadata) = reg.metadata.get(&current) {
                // Add dependencies first
                for dep in &metadata.dependencies {
                    if !resolved.contains(dep) && !to_resolve.contains(dep) {
                        to_resolve.push(dep.clone());
                    }
                }
                resolved.push(current);
            } else {
                return Err(format!("Plugin '{}' not found", current));
            }
        }
        
        Ok(resolved)
    }
}

/// Plugin configuration management
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PluginConfig {
    pub enabled: bool,
    pub auto_load: bool,
    pub priority: i32,
    pub custom_settings: std::collections::HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_load: true,
            priority: 0,
            custom_settings: std::collections::HashMap::new(),
        }
    }
}

/// Plugin configuration manager
pub struct ConfigManager;

impl ConfigManager {
    pub fn load_config(_plugin_name: &str) -> PluginConfig {
        // In a real implementation, this would load from a config file
        // For now, return default config
        PluginConfig::default()
    }
    
    pub fn save_config(plugin_name: &str, config: &PluginConfig) -> Result<(), String> {
        // In a real implementation, this would save to a config file
        println!("Saving config for plugin {}: {:?}", plugin_name, config);
        Ok(())
    }
}
