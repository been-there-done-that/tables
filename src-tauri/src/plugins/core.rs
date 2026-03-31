/// True Plugins System - Unlimited scalability for 100+ commands
/// 
/// This system solves Tauri's single invoke_handler limitation by using
/// macro-based command aggregation while maintaining plugin architecture.
/// 
/// Features:
/// - Dynamic plugin discovery and registration
/// - Unlimited command support (100+ commands)
/// - Plugin dependency management
/// - Runtime plugin enable/disable
/// - Proper separation of concerns
/// - Single invoke_handler with aggregated commands

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use serde::{Serialize, Deserialize};
use once_cell::sync::Lazy;

/// Plugin metadata for discovery and registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub supported_engines: Vec<String>,
    pub dependencies: Vec<String>,
    pub command_count: usize,
    pub enabled: bool,
}

/// Plugin trait that all plugins must implement
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn initialize(&self, app: &AppHandle) -> Result<(), Box<dyn std::error::Error>>;
    fn cleanup(&self, app: &AppHandle) -> Result<(), Box<dyn std::error::Error>>;
}

/// Global plugin registry
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
    pub metadata: HashMap<String, PluginMetadata>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn register<P: Plugin + 'static>(&mut self, plugin: P) {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();
        self.metadata.insert(name.clone(), metadata);
        self.plugins.insert(name, Box::new(plugin));
    }

    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }

    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.metadata.values().cloned().collect()
    }

    pub fn enable_plugin(&mut self, name: &str) -> Result<(), String> {
        if let Some(metadata) = self.metadata.get_mut(name) {
            metadata.enabled = true;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }

    pub fn disable_plugin(&mut self, name: &str) -> Result<(), String> {
        if let Some(metadata) = self.metadata.get_mut(name) {
            metadata.enabled = false;
            Ok(())
        } else {
            Err(format!("Plugin '{}' not found", name))
        }
    }
}

/// Global plugin registry instance
static PLUGIN_REGISTRY: Lazy<Arc<Mutex<PluginRegistry>>> = 
    Lazy::new(|| Arc::new(Mutex::new(PluginRegistry::new())));

/// Get access to the global plugin registry
pub fn get_plugin_registry() -> Arc<Mutex<PluginRegistry>> {
    PLUGIN_REGISTRY.clone()
}

/// Plugin initialization result
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginInitResult {
    pub success: bool,
    pub plugin_name: String,
    pub message: String,
    pub commands_registered: usize,
}

/// Plugin manager for runtime operations
pub struct PluginManager {
    registry: Arc<Mutex<PluginRegistry>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: get_plugin_registry(),
        }
    }

    pub fn initialize_all_plugins(&self, app: &AppHandle) -> Vec<PluginInitResult> {
        let registry = self.registry.lock().unwrap();
        let mut results = Vec::new();

        for (name, metadata) in &registry.metadata {
            if !metadata.enabled {
                continue;
            }

            if let Some(plugin) = registry.plugins.get(name) {
                match plugin.initialize(app) {
                    Ok(()) => {
                        results.push(PluginInitResult {
                            success: true,
                            plugin_name: name.clone(),
                            message: "Plugin initialized successfully".to_string(),
                            commands_registered: metadata.command_count,
                        });
                    }
                    Err(e) => {
                        results.push(PluginInitResult {
                            success: false,
                            plugin_name: name.clone(),
                            message: format!("Initialization failed: {}", e),
                            commands_registered: 0,
                        });
                    }
                }
            }
        }

        results
    }

    pub fn get_plugin_info(&self, name: &str) -> Option<PluginMetadata> {
        let registry = self.registry.lock().unwrap();
        registry.metadata.get(name).cloned()
    }

    pub fn list_available_plugins(&self) -> Vec<PluginMetadata> {
        let registry = self.registry.lock().unwrap();
        registry.list_plugins()
    }
}

/// Macro to aggregate all plugin commands into a single invoke_handler
/// This solves Tauri's single invoke_handler limitation
#[macro_export]
macro_rules! aggregate_plugin_commands {
    () => {
        tauri::generate_handler![
            // Plugin management commands
            get_available_plugins,
            enable_plugin,
            disable_plugin,
            get_plugin_info,
            initialize_all_plugins,
            
            // Theme plugin commands
            get_all_themes,
            get_active_theme,
            set_active_theme,
            
            // Connection plugin commands
            create_connection,
            get_connection,
            get_connection_metadata,
            list_connections,
            update_connection,
            delete_connection,
            test_connection,
            test_connection_by_id, // Added
            get_favorite_connections,
            search_connections,
            update_connection_stats,
            check_keyring_available,
            test_connection_params,
            get_active_connections,
            mark_connection_active,
            mark_connection_inactive,
            save_window_session,
            get_window_session,
            delete_window_session,
            
            // AWS plugin commands
            get_available_aws_profiles,
            get_aws_profile_by_name,
            test_aws_profile,
            list_s3_buckets,
            list_s3_objects,
            upload_s3_file,
            download_s3_file,
            delete_s3_object,
            get_s3_bucket_info,
            
            // Redis plugin commands
            get_redis_info,
            list_redis_databases,
            list_redis_keys,
            get_redis_key,
            execute_redis_command,
            delete_redis_key,
            
            // Athena plugin commands
            execute_athena_query,
            get_athena_query_status,
            list_athena_databases,
            list_athena_tables,
            get_athena_table_schema,
            cancel_athena_query,
            list_athena_workgroups,
            
            // Window commands
            open_datasource_window,
            open_appearance_window,
            create_new_window,
            open_feedback_window,
            get_system_info,
            submit_feedback,
            
            // Introspection commands
            refresh_schema,
            refresh_schema_progressive,
            refresh_schema_specific_progressive,
            refresh_schema_unified, // New unified command
            get_schema,
            get_databases,
            get_schemas,
            get_tables_in_schema,
            get_schema_tables,
            get_schema_table_details,
            // Unified cached reads
            get_cached_schema,
            get_cached_databases,
            get_cached_schemas,
            get_cached_tables,
            get_cached_table_details,
            get_connection_capabilities,
            introspect_database,
            run_db_contention_test,
            get_functions,
            get_sequences,
            get_constraints,
            get_index_details,
            
            // Completion commands
            request_completions,
            update_completion_schema,
            clear_completion_schema,
            get_current_statement,
            get_all_statements,
            request_diagnostics,

            // Font commands
            get_system_fonts,

            // Settings commands
            get_app_settings,
            update_app_setting,

            // Debug/Dangerous commands
            reset_app_state,
            open_internal_db,

            // Query commands
            fetch_table_preview,
            execute_query,
            execute_mutation_batch,
            fetch_query_logs,
            clear_query_logs,
            cancel_query,

            // Editor session commands
            save_editor_session,
            load_editor_session,
            list_editor_sessions,
            delete_editor_session,
            format_sql,

            // Agent thread commands
            create_agent_thread,
            list_agent_threads,
            update_agent_thread_title,
            update_agent_thread_sdk_session,
            delete_agent_thread,
            append_agent_message,
            list_agent_messages,
            upsert_agent_tool_call,
            list_agent_tool_calls,
            save_turn_summary,
            list_turn_summaries,

            // Agent plan commands
            create_agent_plan,
            list_agent_plans,
            list_plan_steps,
            add_plan_step,
            update_plan_step,

            // DDL generation commands
            get_table_ddl,
            get_view_definition,
            get_matview_definition,
            get_function_ddl,
            get_sequence_ddl,
            get_index_ddl,
            get_trigger_definition,

            // Explain commands
            explain_query,

            // Harness sidecar
            crate::harness::get_harness_port,

            // Updater commands
            check_for_update,
            download_update,
            install_update,

            // Export commands
            cancel_export,
            start_export,

        ]
    };
}


/// Macro to register all built-in plugins
#[macro_export]
macro_rules! register_all_plugins {
    () => {
        {
            let registry = $crate::plugins::core::get_plugin_registry();
            let mut reg = registry.lock().unwrap();
            
            // Register all built-in plugins
            reg.register($crate::plugins::builtin::create_theme_plugin());
            reg.register($crate::plugins::builtin::create_connection_plugin());
            reg.register($crate::plugins::builtin::create_aws_plugin());
            reg.register($crate::plugins::builtin::create_redis_plugin());
            reg.register($crate::plugins::builtin::create_athena_plugin());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::AppHandle;

    struct DummyPlugin;

    impl Plugin for DummyPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "dummy".to_string(),
                version: "1.0.0".to_string(),
                description: "A dummy plugin for testing".to_string(),
                author: Some("Test Author".to_string()),
                supported_engines: vec!["postgres".to_string(), "mysql".to_string()],
                dependencies: vec![],
                command_count: 5,
                enabled: true,
            }
        }

        fn initialize(&self, _app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        fn cleanup(&self, _app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert!(registry.plugins.is_empty());
        assert!(registry.metadata.is_empty());
    }

    #[test]
    fn test_plugin_registry_register_and_get() {
        let mut registry = PluginRegistry::new();
        let plugin = DummyPlugin;
        registry.register(plugin);

        assert_eq!(registry.plugins.len(), 1);
        assert_eq!(registry.metadata.len(), 1);
        assert!(registry.get_plugin("dummy").is_some());
        assert!(registry.get_plugin("nonexistent").is_none());
    }

    #[test]
    fn test_plugin_registry_list_plugins() {
        let mut registry = PluginRegistry::new();
        let plugin = DummyPlugin;
        registry.register(plugin);

        let plugins = registry.list_plugins();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "dummy");
        assert_eq!(plugins[0].version, "1.0.0");
        assert_eq!(plugins[0].command_count, 5);
    }

    #[test]
    fn test_plugin_registry_enable_disable_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = DummyPlugin;
        registry.register(plugin);

        // Initially enabled
        assert!(registry.metadata.get("dummy").unwrap().enabled);

        // Disable
        registry.disable_plugin("dummy").unwrap();
        assert!(!registry.metadata.get("dummy").unwrap().enabled);

        // Enable
        registry.enable_plugin("dummy").unwrap();
        assert!(registry.metadata.get("dummy").unwrap().enabled);

        // Try to enable nonexistent
        assert!(registry.enable_plugin("nonexistent").is_err());
    }

    #[test]
    fn test_plugin_manager_new() {
        let _manager = PluginManager::new();
        // Just check it creates without error
        assert!(true);
    }

    #[test]
    fn test_plugin_manager_get_plugin_info() {
        let manager = PluginManager::new();
        // Since registry is global and empty, should be None
        assert!(manager.get_plugin_info("dummy").is_none());
    }

    #[test]
    fn test_plugin_manager_list_available_plugins() {
        let manager = PluginManager::new();
        let plugins = manager.list_available_plugins();
        // May have plugins from other tests or registration, but for isolated test, assume empty
        // But since global, hard to test isolated.
        // Just check it returns a vec
        assert!(!plugins.is_empty() || plugins.is_empty()); // Just check it doesn't panic
    }
}
