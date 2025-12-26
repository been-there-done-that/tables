use crate::connection::{Connection, SecureCredentials, ConnectionInfo, load_connection_from_row};
use crate::credential_manager::CredentialManager;
use rusqlite::{params, Connection as SqliteConnection};
use std::sync::{Arc, Mutex};
use tauri::State;

// Import DatabaseState from the parent module
use super::DatabaseState;

pub struct ConnectionManagerState {
    pub credential_manager: Arc<CredentialManager>,
}

impl ConnectionManagerState {
    pub fn new() -> Self {
        Self {
            credential_manager: Arc::new(CredentialManager::new()),
        }
    }
}

impl Default for ConnectionManagerState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConnectionManager {
    db: Arc<Mutex<SqliteConnection>>,
    credential_manager: Arc<CredentialManager>,
}

impl ConnectionManager {
    pub fn new(db: Arc<Mutex<SqliteConnection>>, credential_manager: Arc<CredentialManager>) -> Self {
        Self {
            db,
            credential_manager,
        }
    }

    pub fn from_state(db_state: &State<'_, DatabaseState>, conn_state: &State<'_, ConnectionManagerState>) -> Self {
        Self::new(
            Arc::clone(&db_state.conn),
            Arc::clone(&conn_state.credential_manager),
        )
    }

    // Create a new connection
    pub fn create_connection(&self, mut connection: Connection, credentials: SecureCredentials) -> Result<String, String> {
        // Store connection metadata in database
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        connection.update_timestamp();

        let connection_params_json = serde_json::to_string(&connection.connection_params)
            .map_err(|e| format!("Failed to serialize connection params: {}", e))?;

        conn.execute(
            "INSERT INTO connections (
                id, name, engine, host, port, database, username, auth_type,
                ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                ssh_tunnel_username, connection_params, is_favorite, color_tag,
                created_at, updated_at, last_connected_at, connection_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                connection.id,
                connection.name,
                connection.engine.display_name().to_lowercase(),
                connection.host,
                connection.port,
                connection.database,
                connection.username,
                connection.auth_type.to_string(),
                connection.ssl_enabled as i64,
                connection.ssh_tunnel_enabled as i64,
                connection.ssh_tunnel_host,
                connection.ssh_tunnel_port,
                connection.ssh_tunnel_username,
                connection_params_json,
                connection.is_favorite as i64,
                connection.color_tag,
                connection.created_at,
                connection.updated_at,
                connection.last_connected_at,
                connection.connection_count
            ],
        ).map_err(|e| format!("Failed to insert connection: {}", e))?;

        // Store credentials in keyring
        if !credentials.is_empty() {
            self.credential_manager.store_credentials(&connection.id, &credentials)
                .map_err(|e| format!("Failed to store credentials: {}", e))?;
        }

        Ok(connection.id)
    }

    // Get connection with credentials
    pub fn get_connection(&self, id: &str) -> Result<(Connection, SecureCredentials), String> {
        // Get connection metadata from database
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let connection = conn.query_row(
            "SELECT id, name, engine, host, port, database, username, auth_type,
                    ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                    ssh_tunnel_username, connection_params, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE id = ?1",
            params![id],
            load_connection_from_row,
        ).map_err(|e| format!("Failed to get connection: {}", e))?;

        // Get credentials from keyring
        let credentials = self.credential_manager.get_credentials(id)
            .map_err(|e| format!("Failed to get credentials: {}", e))?;

        Ok((connection, credentials))
    }

    // Get connection metadata only (without credentials)
    pub fn get_connection_metadata(&self, id: &str) -> Result<Connection, String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        conn.query_row(
            "SELECT id, name, engine, host, port, database, username, auth_type,
                    ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                    ssh_tunnel_username, connection_params, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE id = ?1",
            params![id],
            load_connection_from_row,
        ).map_err(|e| format!("Failed to get connection: {}", e))
    }

    // List all connections (without credentials)
    pub fn list_connections(&self) -> Result<Vec<Connection>, String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, auth_type,
                    ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                    ssh_tunnel_username, connection_params, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
             FROM connections ORDER BY name COLLATE NOCASE"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map([], load_connection_from_row)
            .map_err(|e| format!("Failed to query connections: {}", e))?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| format!("Failed to read connection: {}", e))?);
        }

        Ok(connections)
    }

    // Update connection
    pub fn update_connection(&self, mut connection: Connection, credentials: Option<SecureCredentials>) -> Result<(), String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        connection.update_timestamp();

        let connection_params_json = serde_json::to_string(&connection.connection_params)
            .map_err(|e| format!("Failed to serialize connection params: {}", e))?;

        conn.execute(
            "UPDATE connections SET
                name = ?2, engine = ?3, host = ?4, port = ?5, database = ?6, username = ?7,
                auth_type = ?8, ssl_enabled = ?9, ssh_tunnel_enabled = ?10, ssh_tunnel_host = ?11,
                ssh_tunnel_port = ?12, ssh_tunnel_username = ?13, connection_params = ?14,
                is_favorite = ?15, color_tag = ?16, updated_at = ?17
             WHERE id = ?1",
            params![
                connection.id,
                connection.name,
                serde_json::to_string(&connection.engine).unwrap(),
                connection.host,
                connection.port,
                connection.database,
                connection.username,
                serde_json::to_string(&connection.auth_type).unwrap(),
                connection.ssl_enabled as i64,
                connection.ssh_tunnel_enabled as i64,
                connection.ssh_tunnel_host,
                connection.ssh_tunnel_port,
                connection.ssh_tunnel_username,
                connection_params_json,
                connection.is_favorite as i64,
                connection.color_tag,
                connection.updated_at
            ],
        ).map_err(|e| format!("Failed to update connection: {}", e))?;

        // Update credentials if provided
        if let Some(credentials) = credentials {
            if !credentials.is_empty() {
                self.credential_manager.store_credentials(&connection.id, &credentials)
                    .map_err(|e| format!("Failed to store credentials: {}", e))?;
            }
        }

        Ok(())
    }

    // Delete connection
    pub fn delete_connection(&self, id: &str) -> Result<(), String> {
        // Delete from database
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        conn.execute("DELETE FROM connections WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete connection: {}", e))?;

        // Delete credentials from keyring
        self.credential_manager.delete_all_credentials(id)
            .map_err(|e| format!("Failed to delete credentials: {}", e))?;

        Ok(())
    }

    // Test connection
    pub fn test_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> Result<ConnectionInfo, String> {
        let start_time = std::time::Instant::now();
        
        // Mock implementation - replace with actual connection testing
        let connected = match connection.engine {
            crate::connection::DatabaseEngine::SQLite => {
                // For SQLite, check if file exists
                if let Some(host) = &connection.host {
                    std::path::Path::new(host).exists()
                } else {
                    false
                }
            }
            crate::connection::DatabaseEngine::Redis => {
                // For Redis, test connection
                self.test_redis_connection(connection, credentials)
            }
            crate::connection::DatabaseEngine::Athena => {
                // For Athena, test AWS credentials and connectivity
                self.test_athena_connection(connection, credentials)
            }
            crate::connection::DatabaseEngine::S3 => {
                // For S3, test AWS credentials and connectivity
                self.test_s3_connection(connection, credentials)
            }
            _ => {
                // For other engines, just check if we have required credentials
                !credentials.is_empty()
            }
        };

        let response_time = start_time.elapsed().as_millis() as u64;

        Ok(ConnectionInfo {
            connected,
            version: Some("Mock Version".to_string()),
            database_name: connection.database.clone(),
            error: if connected { None } else { Some("Connection failed".to_string()) },
            response_time_ms: Some(response_time),
        })
    }

    // Test Redis connection specifically
    fn test_redis_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        // For now, just check if we have basic connection info
        // In production, would use Redis client to test actual connectivity
        connection.host.is_some() && 
        (credentials.password.is_some() || matches!(connection.auth_type, crate::connection::AuthType::None))
    }

    // Test Athena connection specifically
    fn test_athena_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        match connection.auth_type {
            crate::connection::AuthType::AwsCredentials => {
                // Check if we have access key and secret key
                credentials.aws_access_key_id.is_some() && 
                credentials.aws_secret_access_key.is_some()
            }
            crate::connection::AuthType::AwsProfile => {
                // Profile-based authentication - assume valid for now
                true
            }
            crate::connection::AuthType::AwsIamRole => {
                // IAM role authentication - assume valid for now
                true
            }
            crate::connection::AuthType::AthenaJdbc => {
                // JDBC authentication - check for password
                credentials.password.is_some()
            }
            _ => false,
        }
    }

    // Test S3 connection specifically
    fn test_s3_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        match connection.auth_type {
            crate::connection::AuthType::AwsCredentials => {
                // Check if we have access key and secret key
                credentials.aws_access_key_id.is_some() && 
                credentials.aws_secret_access_key.is_some()
            }
            crate::connection::AuthType::AwsProfile => {
                // Profile-based authentication - assume valid for now
                // In real implementation, would use AWS SDK
                true
            }
            crate::connection::AuthType::AwsIamRole => {
                // IAM role authentication - assume valid for now
                // In real implementation, would use AWS SDK
                true
            }
            _ => false,
        }
    }

    // Increment connection count and update last connected timestamp
    pub fn update_connection_stats(&self, id: &str) -> Result<(), String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "UPDATE connections SET connection_count = connection_count + 1, last_connected_at = ?2 WHERE id = ?1",
            params![id, now]
        ).map_err(|e| format!("Failed to update connection stats: {}", e))?;

        Ok(())
    }

    // Get favorite connections
    pub fn get_favorite_connections(&self) -> Result<Vec<Connection>, String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, auth_type,
                    ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                    ssh_tunnel_username, connection_params, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE is_favorite = 1 ORDER BY name COLLATE NOCASE"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map([], load_connection_from_row)
            .map_err(|e| format!("Failed to query favorite connections: {}", e))?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| format!("Failed to read connection: {}", e))?);
        }

        Ok(connections)
    }

    // Search connections by name
    pub fn search_connections(&self, query: &str) -> Result<Vec<Connection>, String> {
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        // Escape SQL LIKE special characters to prevent unexpected behavior
        let escaped_query = query.replace('%', "\\%").replace('_', "\\_");
        let search_pattern = format!("%{}%", escaped_query);

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, auth_type,
                    ssl_enabled, ssh_tunnel_enabled, ssh_tunnel_host, ssh_tunnel_port,
                    ssh_tunnel_username, connection_params, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE name LIKE ?1 ESCAPE '\\' OR host LIKE ?1 ESCAPE '\\' ORDER BY name COLLATE NOCASE"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map([search_pattern.as_str()], load_connection_from_row)
            .map_err(|e| format!("Failed to search connections: {}", e))?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| format!("Failed to read connection: {}", e))?);
        }

        Ok(connections)
    }
}
