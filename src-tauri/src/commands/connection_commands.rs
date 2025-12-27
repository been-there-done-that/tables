use crate::connection::{Connection as DatabaseConnection, SecureCredentials, ConnectionInfo};
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use crate::DatabaseState;
use tauri::State;

/// Create a new database connection
#[tauri::command]
pub async fn create_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<String, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.create_connection(connection, credentials)
}

/// Get a connection with its credentials
#[tauri::command]
pub async fn get_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(DatabaseConnection, SecureCredentials), String> {
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
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_connection_metadata(&id)
}

/// List all connections (without credentials)
#[tauri::command]
pub async fn list_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.list_connections()
}

/// Update an existing connection
#[tauri::command]
pub async fn update_connection(
    connection: DatabaseConnection,
    credentials: Option<SecureCredentials>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.update_connection(connection, credentials)
}

/// Delete a connection
#[tauri::command]
pub async fn delete_connection(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.delete_connection(&id)
}

/// Test a connection
#[tauri::command]
pub async fn test_connection(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection(&connection, &credentials)
}

/// Get favorite connections
#[tauri::command]
pub async fn get_favorite_connections(
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.get_favorite_connections()
}

/// Search connections by name or host
#[tauri::command]
pub async fn search_connections(
    query: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<DatabaseConnection>, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.search_connections(&query)
}

/// Update connection statistics (last used, connection count)
#[tauri::command]
pub async fn update_connection_stats(
    id: String,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<(), String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.update_connection_stats(&id)
}

#[derive(serde::Deserialize)]
pub struct TestConnectionParams {
    pub engine: String,
    pub config: serde_json::Value,
}

/// Test a connection with raw parameters (unsaved)
#[tauri::command]
pub async fn test_connection_params(
    params: TestConnectionParams,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<ConnectionInfo, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    manager.test_connection_params(params.engine, params.config).await
}

/// Check if keyring is available for secure credential storage
#[tauri::command]
pub async fn check_keyring_available(
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<bool, String> {
    Ok(conn_state.credential_manager.is_available())
}
