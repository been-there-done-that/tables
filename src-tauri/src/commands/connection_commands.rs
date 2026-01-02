use crate::connection::{Connection as DatabaseConnection, SecureCredentials, ConnectionInfo};
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use crate::DatabaseState;
use tauri::State;
use log::{info, debug, warn, error, trace};

/// Create a new database connection
#[tauri::command]
pub async fn create_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    debug!("Creating connection '{}' with engine '{}'", connection.name, connection.engine);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.create_connection(connection, credentials);
    if let Ok(ref id) = result {
        info!("Connection '{}' created with ID '{}'", "name", id);
    }
    result
}

/// Get a connection with its credentials
#[tauri::command]
pub async fn get_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(DatabaseConnection, SecureCredentials), String> {
    debug!("Getting connection '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_connection(&id)
}

/// Get connection metadata without credentials
#[tauri::command]
pub async fn get_connection_metadata(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<DatabaseConnection, String> {
    debug!("Getting connection metadata for '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_connection_metadata(&id)
}

/// List all connections (without credentials)
#[tauri::command]
pub async fn list_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    debug!("Listing all connections");
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.list_connections();
    if let Ok(ref connections) = result {
        debug!("Listed {} connections", connections.len());
    }
    result
}

/// Update an existing connection
#[tauri::command]
pub async fn update_connection(
    connection: DatabaseConnection,
    credentials: Option<SecureCredentials>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Updating connection '{}' with engine '{}'", connection.name, connection.engine);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.update_connection(connection, credentials);
    if result.is_ok() {
        info!("Connection '{}' updated successfully", "name");
    }
    result
}

/// Delete a connection
#[tauri::command]
pub async fn delete_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Deleting connection '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.delete_connection(&id);
    if result.is_ok() {
        info!("Connection '{}' deleted successfully", id);
    }
    result
}

/// Test a connection
#[tauri::command]
pub async fn test_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    debug!("Testing connection '{}' with engine '{}'", connection.name, connection.engine);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection(&connection, &credentials).await
}

/// Test a stored connection by ID
#[tauri::command]
pub async fn test_connection_by_id(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    debug!("Testing connection by ID '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection_by_id(&id).await
}

/// Get favorite connections
#[tauri::command]
pub async fn get_favorite_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    debug!("Getting favorite connections");
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.get_favorite_connections();
    if let Ok(ref connections) = result {
        debug!("Retrieved {} favorite connections", connections.len());
    }
    result
}

/// Search connections by name or host
#[tauri::command]
pub async fn search_connections(
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    debug!("Searching connections with query '{}'", query);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    let result = manager.search_connections(&query);
    if let Ok(ref connections) = result {
        debug!("Search returned {} connections", connections.len());
    }
    result
}

/// Update connection statistics (last used, connection count)
#[tauri::command]
pub async fn update_connection_stats(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Updating connection stats for '{}'", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.update_connection_stats(&id)
}

/// Test a connection with raw parameters (unsaved)
#[derive(serde::Deserialize)]
pub struct TestConnectionParams {
    pub engine: String,
    pub config: serde_json::Value,
}

#[tauri::command]
pub async fn test_connection_params(
    params: TestConnectionParams,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    debug!("Testing connection with raw params for engine '{}'", params.engine);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection_params(params.engine, params.config).await
}

/// Check if keyring is available for secure credential storage
#[tauri::command]
pub async fn check_keyring_available(
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<bool, String> {
    trace!("Checking keyring availability");
    Ok(conn_state.credential_manager.is_available())
}

/// Get active connection IDs
#[tauri::command]
pub async fn get_active_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<String>, String> {
    debug!("Getting active connection IDs");
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    Ok(manager.get_active_connection_ids())
}

/// Mark a connection as active
#[tauri::command]
pub async fn mark_connection_active(
    id: String,
    window_label: Option<String>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Marking connection '{}' as active", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.set_connection_active(&id, true);
    
    // Also save window session if label is provided
    if let Some(label) = window_label {
        manager.save_window_session(&label, &id)?;
    }
    
    Ok(())
}

/// Mark a connection as inactive
#[tauri::command]
pub async fn mark_connection_inactive(
    id: String,
    window_label: Option<String>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    debug!("Marking connection '{}' as inactive", id);
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.set_connection_active(&id, false);
    
    // Also clear window session if label matches
    if let Some(label) = window_label {
        if let Ok(Some(current_id)) = manager.get_window_session(&label) {
            if current_id == id {
                manager.delete_window_session(&label)?;
            }
        }
    }
    
    Ok(())
}

/// Save a window session manually
#[tauri::command]
pub async fn save_window_session(
    window_label: String,
    connection_id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.save_window_session(&window_label, &connection_id)
}

/// Get a persisted window session
#[tauri::command]
pub async fn get_window_session(
    window_label: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Option<String>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_window_session(&window_label)
}

/// Delete a window session
#[tauri::command]
pub async fn delete_window_session(
    window_label: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.delete_window_session(&window_label)
}
