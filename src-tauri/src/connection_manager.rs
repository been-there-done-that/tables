use crate::connection::{Connection, SecureCredentials, ConnectionInfo, load_connection_from_row};
use crate::configs::RuntimeConnection;
use crate::credential_manager::CredentialManager;
use rusqlite::{params, Connection as SqliteConnection, OptionalExtension};
use std::sync::{Arc, Mutex};
use tauri::State;
use log::{info, debug, warn, error, trace};

// Import DatabaseState from the parent module
use super::DatabaseState;

pub struct ConnectionManagerState {
    pub credential_manager: Arc<CredentialManager>,
    pub active_connections: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl ConnectionManagerState {
    pub fn new(credential_manager: Arc<CredentialManager>) -> Self {
        Self {
            credential_manager,
            active_connections: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

pub struct ConnectionManager {
    db: Arc<Mutex<SqliteConnection>>,
    credential_manager: Arc<CredentialManager>,
    active_connections: Arc<Mutex<std::collections::HashSet<String>>>,
}

impl ConnectionManager {
    pub fn new(db: Arc<Mutex<SqliteConnection>>, credential_manager: Arc<CredentialManager>, active_connections: Arc<Mutex<std::collections::HashSet<String>>>) -> Self {
        Self {
            db,
            credential_manager,
            active_connections,
        }
    }

    pub fn from_state(db_state: &State<'_, DatabaseState>, conn_state: &State<'_, ConnectionManagerState>) -> Self {
        Self::new(
            Arc::clone(&db_state.conn),
            Arc::clone(&conn_state.credential_manager),
            Arc::clone(&conn_state.active_connections),
        )
    }

    // Create a new connection
    pub fn create_connection(&self, connection: Connection, credentials: SecureCredentials) -> Result<String, String> {
        debug!("Creating connection '{}' with id {}", connection.name, connection.id);
        // Store connection metadata in database
        {
            debug!("Storing connection metadata in database");
            let conn = self.db.lock()
                .map_err(|e| {
                    error!("Failed to lock database for connection '{}': {}", connection.id, e);
                    format!("Failed to lock database: {}", e)
                })?;

            conn.execute(
                "INSERT INTO connections (
                    id, name, engine, host, port, database, username,
                    uses_ssh, uses_tls, config_json, is_favorite, color_tag,
                    created_at, updated_at, last_connected_at, connection_count
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    connection.id,
                    connection.name,
                    connection.engine,
                    connection.host,
                    connection.port,
                    connection.database,
                    connection.username,
                    connection.uses_ssh as i64,
                    connection.uses_tls as i64,
                    connection.config_json,
                    connection.is_favorite as i64,
                    connection.color_tag,
                    connection.created_at,
                    connection.updated_at,
                    connection.last_connected_at,
                    connection.connection_count,
                ],
            ).map_err(|e| {
                error!("Failed to insert connection '{}' into database: {}", connection.id, e);
                format!("Failed to insert connection: {}", e)
            })?;
            debug!("Connection metadata stored successfully");
        } // Lock is dropped here

        // Store credentials in secure storage
        if !credentials.is_empty() {
            debug!("Storing credentials for connection '{}'", connection.id);
            self.credential_manager.store_credentials(&connection.id, &credentials)
                .map_err(|e| {
                    error!("Failed to store credentials for connection '{}': {}", connection.id, e);
                    format!("Failed to store credentials: {}", e)
                })?;
            debug!("Credentials stored successfully");
        } else {
            debug!("No credentials to store for connection '{}'", connection.id);
        }

        info!("Connection '{}' created successfully", connection.name);
        Ok(connection.id)
    }

    // Get connection with credentials
    pub fn get_connection(&self, id: &str) -> Result<(Connection, SecureCredentials), String> {
        debug!("Retrieving connection with id '{}'", id);
        // Get connection metadata from database
        let connection = {
            debug!("Fetching connection metadata from database");
            let conn = self.db.lock()
                .map_err(|e| {
                    error!("Failed to lock database for connection '{}': {}", id, e);
                    format!("Failed to lock database: {}", e)
                })?;

            conn.query_row(
                "SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
                 FROM connections WHERE id = ?1",
                params![id],
                load_connection_from_row,
            ).map_err(|e| {
                error!("Failed to get connection '{}' from database: {}", id, e);
                format!("Failed to get connection: {}", e)
            })?
        }; // Lock is dropped here
        debug!("Connection metadata retrieved for '{}'", id);

        // Check if credentials exist for this connection
        let has_creds = self.credential_manager.has_credentials(id)
            .map_err(|e| {
                error!("Failed to check credentials existence for connection '{}': {}", id, e);
                format!("Failed to check if credentials exist: {}", e)
            })?;

        let credentials = if has_creds {
            debug!("Fetching credentials for connection '{}' (found in DB)", id);
            // Get credentials from secure storage
            self.credential_manager.get_credentials(id)
                .map_err(|e| {
                    error!("Failed to get credentials for connection '{}': {}", id, e);
                    format!("Failed to get credentials: {}", e)
                })?
        } else {
            debug!("Skipping credential fetch for connection '{}' (none found in DB)", id);
            SecureCredentials::new()
        };

        debug!("Connection '{}' retrieved successfully", id);
        Ok((connection, credentials))
    }

    // Get connection metadata only (without credentials)
    pub fn get_connection_metadata(&self, id: &str) -> Result<Connection, String> {
        debug!("Getting connection metadata for id '{}'", id);
        let conn = self.db.lock()
            .map_err(|e| {
                error!("Failed to lock database for connection metadata '{}': {}", id, e);
                format!("Failed to lock database: {}", e)
            })?;

        conn.query_row(
            "SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE id = ?1",
            params![id],
            load_connection_from_row,
        ).map_err(|e| {
            error!("Failed to get connection metadata for '{}': {}", id, e);
            format!("Failed to get connection: {}", e)
        })
    }

    // List all connections (without credentials)
    pub fn list_connections(&self) -> Result<Vec<Connection>, String> {
        debug!("Listing all connections");
        let conn = self.db.lock()
            .map_err(|e| {
                error!("Failed to lock database for listing connections: {}", e);
                format!("Failed to lock database: {}", e)
            })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
             FROM connections ORDER BY name COLLATE NOCASE"
        ).map_err(|e| {
            error!("Failed to prepare query for listing connections: {}", e);
            format!("Failed to prepare query: {}", e)
        })?;

        let rows = stmt.query_map([], load_connection_from_row)
            .map_err(|e| {
                error!("Failed to query connections: {}", e);
                format!("Failed to query connections: {}", e)
            })?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| {
                error!("Failed to read connection: {}", e);
                format!("Failed to read connection: {}", e)
            })?);
        }

        debug!("Retrieved {} connections", connections.len());
        Ok(connections)
    }

    // Update connection
    pub fn update_connection(&self, mut connection: Connection, credentials: Option<SecureCredentials>) -> Result<(), String> {
        debug!("Updating connection '{}' with id {}", connection.name, connection.id);
        {
            debug!("Updating connection metadata in database");
            let conn = self.db.lock()
                .map_err(|e| {
                    error!("Failed to lock database for updating connection '{}': {}", connection.id, e);
                    format!("Failed to lock database: {}", e)
                })?;

            connection.update_timestamp();

            conn.execute(
                "UPDATE connections SET
                    name = ?2, engine = ?3, host = ?4, port = ?5, database = ?6, username = ?7,
                    uses_ssh = ?8, uses_tls = ?9, config_json = ?10, is_favorite = ?11, color_tag = ?12,
                    updated_at = ?13
                 WHERE id = ?1",
                params![
                    connection.id,
                    connection.name,
                    connection.engine,
                    connection.host,
                    connection.port,
                    connection.database,
                    connection.username,
                    connection.uses_ssh as i64,
                    connection.uses_tls as i64,
                    connection.config_json,
                    connection.is_favorite as i64,
                    connection.color_tag,
                    connection.updated_at
                ],
            ).map_err(|e| {
                error!("Failed to update connection '{}' in database: {}", connection.id, e);
                format!("Failed to update connection: {}", e)
            })?;
            debug!("Connection metadata updated successfully");
        } // Lock is dropped here

        // Update credentials if provided
        if let Some(credentials) = credentials {
            if !credentials.is_empty() {
                debug!("Updating credentials for connection '{}'", connection.id);
                self.credential_manager.store_credentials(&connection.id, &credentials)
                    .map_err(|e| {
                        error!("Failed to store updated credentials for connection '{}': {}", connection.id, e);
                        format!("Failed to store credentials: {}", e)
                    })?;
                debug!("Credentials updated successfully");
            } else {
                debug!("No credentials provided for connection '{}'", connection.id);
            }
        }

        info!("Connection '{}' updated successfully", connection.name);
        Ok(())
    }

    // Delete connection
    pub fn delete_connection(&self, id: &str) -> Result<(), String> {
        debug!("Deleting connection with id '{}'", id);
        // Delete from database
        {
            debug!("Deleting connection metadata from database");
            let conn = self.db.lock()
                .map_err(|e| {
                    error!("Failed to lock database for deleting connection '{}': {}", id, e);
                    format!("Failed to lock database: {}", e)
                })?;

            conn.execute("DELETE FROM connections WHERE id = ?1", params![id])
                .map_err(|e| {
                    error!("Failed to delete connection '{}' from database: {}", id, e);
                    format!("Failed to delete connection: {}", e)
                })?;
            debug!("Connection metadata deleted successfully");
        } // Lock is dropped here

        // Delete credentials from secure storage
        debug!("Deleting credentials for connection '{}'", id);
        self.credential_manager.delete_all_credentials(id)
            .map_err(|e| {
                error!("Failed to delete credentials for connection '{}': {}", id, e);
                format!("Failed to delete credentials: {}", e)
            })?;
        debug!("Credentials deleted successfully");

        info!("Connection '{}' deleted successfully", id);
        Ok(())
    }

    // Test connection
    pub async fn test_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> Result<ConnectionInfo, String> {
        // For saved connections, we use the stored config_json
        let mut config: serde_json::Value = serde_json::from_str(&connection.config_json)
            .map_err(|e| {
                error!("Failed to parse connection config for {}: {}", connection.id, e);
                format!("Failed to parse connection config: {}", e)
            })?;
            
        // Inject credentials into config
        if let Some(db) = config.get_mut("db") {
            if let Some(db_obj) = db.as_object_mut() {
                if let Some(password) = &credentials.password {
                    debug!("Injecting password from secure credentials into connection config");
                    db_obj.insert("password".to_string(), serde_json::Value::String(password.expose().to_string()));
                }
            }
        }
            
        self.test_connection_params(connection.engine.clone(), config).await
    }

    pub async fn test_connection_by_id(&self, id: &str) -> Result<ConnectionInfo, String> {
        // 1. Get connection metadata (no credentials returned)
        let (connection, credentials) = self.get_connection(id).map_err(|e| format!("Failed to get connection: {}", e))?;
        
        // 2. Test using the retrieved internal credentials
        self.test_connection(&connection, &credentials).await
    }

    pub async fn test_connection_params(&self, engine: String, config: serde_json::Value) -> Result<ConnectionInfo, String> {
        let start_time = std::time::Instant::now();
        debug!("Testing connection for engine={} with config={}", engine, config);
        
        let result = match engine.as_str() {
            "postgres" => self.test_postgres_raw(&config).await,
            "mysql" => self.test_mysql_raw(&config).await,
            "sqlite" => self.test_sqlite_raw(&config).await,
            "mongodb" => self.test_mongodb_raw(&config).await,
            "redis" => self.test_redis_raw(&config).await,
            _ => {
                warn!("Connection testing not implemented for engine: {}", engine);
                Err(format!("Connection testing not implemented for engine: {}", engine))
            },
        };

        let response_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok((version, db_name)) => Ok(ConnectionInfo {
                connected: true,
                version: Some(version),
                database_name: Some(db_name),
                error: None,
                response_time_ms: Some(response_time),
            }),
            Err(e) => {
                warn!("Connection test failed for engine {} after {} ms: {}", engine, response_time, e);
                Ok(ConnectionInfo {
                    connected: false,
                    version: None,
                    database_name: None,
                    error: Some(e),
                    response_time_ms: Some(response_time),
                })
            },
        }
    }

    async fn test_postgres_raw(&self, config: &serde_json::Value) -> Result<(String, String), String> {
        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(5432);
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let database = db.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");
        
        let conn_str = format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database);

        let (client, connection) = tokio_postgres::connect(&conn_str, tokio_postgres::NoTls).await
            .map_err(|e| {
                if let Some(db_error) = e.as_db_error() {
                    return db_error.message().to_string();
                }
                e.to_string()
            })?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let row = client.query_one("SELECT version()", &[]).await
            .map_err(|e| {
                if let Some(db_error) = e.as_db_error() {
                    return db_error.message().to_string();
                }
                e.to_string()
            })?;
        
        let version: String = row.get(0);
        Ok((version, database.to_string()))
    }

    async fn test_mysql_raw(&self, config: &serde_json::Value) -> Result<(String, String), String> {
        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(3306) as u16;
        let user = db.get("username").and_then(|v| v.as_str()).ok_or("Missing username")?;
        let database = db.get("database").and_then(|v| v.as_str()).ok_or("Missing database")?;
        let password = db.get("password").and_then(|v| v.as_str()).unwrap_or("");

        let url = format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database);
        let pool = mysql::Pool::new(url.as_str()).map_err(|e| e.to_string())?;
        let mut conn = pool.get_conn().map_err(|e| e.to_string())?;

        let version: String = mysql::prelude::Queryable::query_first(&mut conn, "SELECT VERSION()")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "Unknown".to_string());

        Ok((version, database.to_string()))
    }

    async fn test_sqlite_raw(&self, config: &serde_json::Value) -> Result<(String, String), String> {
        let mode_raw = config.get("mode").and_then(|v| v.as_str()).unwrap_or("file");
        let mode = mode_raw.to_ascii_lowercase();

        // Treat mode case-insensitively; default to file unless explicitly memory.
        if mode == "memory" {
            info!("SQLite test using in-memory mode (mode={})", mode_raw);
            return Ok(("SQLite In-Memory".to_string(), ":memory:".to_string()));
        }

        let file = config.get("file").and_then(|v| v.as_str()).ok_or("Missing file path")?;
        let path = std::path::Path::new(file);
        
        if path.exists() {
            debug!("SQLite test opening file at {:?}", path);
            let conn = rusqlite::Connection::open(file).map_err(|e| {
                error!("SQLite open failed for {:?}: {}", path, e);
                e.to_string()
            })?;
            let version: String = conn.query_row("SELECT sqlite_version()", [], |r| r.get(0))
                .map_err(|e| {
                    error!("SQLite version query failed for {:?}: {}", path, e);
                    e.to_string()
                })?;
            info!("SQLite test successful for {:?}, version {}", path, version);
            Ok((version, file.to_string()))
        } else {
            warn!("SQLite test file does not exist: {:?}", path);
            Err(format!("File does not exist: {}", file))
        }
    }

    async fn test_mongodb_raw(&self, config: &serde_json::Value) -> Result<(String, String), String> {
        let auth = config.get("auth").ok_or("Missing 'auth' config")?;
        let method = auth.get("method").and_then(|v| v.as_str()).unwrap_or("standard");
        
        let client_uri = if method == "uri" {
            config.get("db").and_then(|d| d.get("uri")).and_then(|v| v.as_str()).ok_or("Missing URI")?.to_string()
        } else {
            let db = config.get("db").ok_or("Missing 'db' config")?;
            let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
            let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(27017);
            let user = db.get("username").and_then(|v| v.as_str());
            let pass = db.get("password").and_then(|v| v.as_str());
            
            if let (Some(u), Some(p)) = (user, pass) {
                format!("mongodb://{}:{}@{}:{}", u, p, host, port)
            } else {
                format!("mongodb://{}:{}", host, port)
            }
        };

        let client = mongodb::Client::with_uri_str(&client_uri).await.map_err(|e| e.to_string())?;
        let db_name = config.get("db").and_then(|d| d.get("database")).and_then(|v| v.as_str()).unwrap_or("admin");
        
        // Try to ping the server
        let db = client.database(db_name);
        mongodb::bson::doc! { "ping": 1 };
        db.run_command(mongodb::bson::doc! { "ping": 1 }, None).await
            .map_err(|e| format!("Ping failed: {}", e))?;

        Ok(("MongoDB Server".to_string(), db_name.to_string()))
    }

    async fn test_redis_raw(&self, config: &serde_json::Value) -> Result<(String, String), String> {
        let db = config.get("db").ok_or("Missing 'db' config")?;
        let host = db.get("host").and_then(|v| v.as_str()).ok_or("Missing host")?;
        let port = db.get("port").and_then(|v| v.as_u64()).unwrap_or(6379);
        let pass = db.get("password").and_then(|v| v.as_str());
        let user = db.get("username").and_then(|v| v.as_str());

        let url = if let Some(p) = pass {
            if let Some(u) = user {
                format!("redis://{}:{}@{}:{}", u, p, host, port)
            } else {
                format!("redis://:{}@{}:{}", p, host, port)
            }
        } else {
            format!("redis://{}:{}", host, port)
        };

        let client = redis::Client::open(url).map_err(|e| e.to_string())?;
        let mut conn = client.get_connection().map_err(|e| e.to_string())?;
        
        let info: String = redis::cmd("INFO").arg("server").query(&mut conn)
            .map_err(|e| e.to_string())?;
        
        let version = info.lines()
            .find(|l| l.starts_with("redis_version:"))
            .and_then(|l| l.split(':').nth(1))
            .unwrap_or("unknown")
            .to_string();
        
        Ok((format!("Redis {}", version), "0".to_string()))
    }

    // Test Redis connection specifically
    fn test_redis_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        // For now, just check if we have basic connection info
        // In production, would use Redis client to test actual connectivity
        connection.host.is_some() && !credentials.is_empty()
    }

    // Test Athena connection specifically
    fn test_athena_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        // For now, just check if we have AWS credentials
        // In production, would parse config and check auth type
        credentials.aws_access_key_id.is_some() || 
        credentials.aws_secret_access_key.is_some() ||
        credentials.aws_session_token.is_some()
    }

    // Test S3 connection specifically
    fn test_s3_connection(&self, connection: &Connection, credentials: &SecureCredentials) -> bool {
        // For now, just check if we have AWS credentials
        // In production, would parse config and check auth type
        credentials.aws_access_key_id.is_some() || 
        credentials.aws_secret_access_key.is_some() ||
        credentials.aws_session_token.is_some()
    }

    // Increment connection count and update last connected timestamp
    pub fn update_connection_stats(&self, id: &str) -> Result<(), String> {
        debug!("Updating connection stats for id '{}'", id);
        let conn = self.db.lock()
            .map_err(|e| {
                error!("Failed to lock database for updating stats for connection '{}': {}", id, e);
                format!("Failed to lock database: {}", e)
            })?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "UPDATE connections SET connection_count = connection_count + 1, last_connected_at = ?2 WHERE id = ?1",
            params![id, now]
        ).map_err(|e| {
            error!("Failed to update connection stats for '{}': {}", id, e);
            format!("Failed to update connection stats: {}", e)
        })?;

        debug!("Connection stats updated for '{}'", id);
        Ok(())
    }

    // Get favorite connections
    pub fn get_favorite_connections(&self) -> Result<Vec<Connection>, String> {
        debug!("Getting favorite connections");
        let conn = self.db.lock()
            .map_err(|e| {
                error!("Failed to lock database for getting favorite connections: {}", e);
                format!("Failed to lock database: {}", e)
            })?;

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE is_favorite = 1 ORDER BY name COLLATE NOCASE"
        ).map_err(|e| {
            error!("Failed to prepare query for favorite connections: {}", e);
            format!("Failed to prepare query: {}", e)
        })?;

        let rows = stmt.query_map([], load_connection_from_row)
            .map_err(|e| {
                error!("Failed to query favorite connections: {}", e);
                format!("Failed to query favorite connections: {}", e)
            })?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| {
                error!("Failed to read favorite connection: {}", e);
                format!("Failed to read connection: {}", e)
            })?);
        }

        debug!("Retrieved {} favorite connections", connections.len());
        Ok(connections)
    }

    // Search connections by name
    pub fn search_connections(&self, query: &str) -> Result<Vec<Connection>, String> {
        debug!("Searching connections with query '{}'", query);
        let conn = self.db.lock()
            .map_err(|e| {
                error!("Failed to lock database for searching connections: {}", e);
                format!("Failed to lock database: {}", e)
            })?;

        // Escape SQL LIKE special characters to prevent unexpected behavior
        let escaped_query = query.replace('%', "\\%").replace('_', "\\_");
        let search_pattern = format!("%{}%", escaped_query);

        let mut stmt = conn.prepare(
            "SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
             FROM connections WHERE name LIKE ?1 ESCAPE '\\' OR host LIKE ?1 ESCAPE '\\' ORDER BY name COLLATE NOCASE"
        ).map_err(|e| {
            error!("Failed to prepare search query for '{}': {}", query, e);
            format!("Failed to prepare query: {}", e)
        })?;

        let rows = stmt.query_map([search_pattern.as_str()], load_connection_from_row)
            .map_err(|e| {
                error!("Failed to search connections for '{}': {}", query, e);
                format!("Failed to search connections: {}", e)
            })?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(row.map_err(|e| {
                error!("Failed to read searched connection: {}", e);
                format!("Failed to read connection: {}", e)
            })?);
        }

        debug!("Search for '{}' returned {} connections", query, connections.len());
        Ok(connections)
    }
    // Active Connection Management
    pub fn get_active_connection_ids(&self) -> Vec<String> {
        let active = self.active_connections.lock().unwrap();
        active.iter().cloned().collect()
    }

    pub fn set_connection_active(&self, id: &str, active: bool) {
        let mut set = self.active_connections.lock().unwrap();
        if active {
            set.insert(id.to_string());
        } else {
            set.remove(id);
        }
    }

    // Window Session Persistence
    pub fn save_window_session(&self, window_label: &str, connection_id: &str) -> Result<(), String> {
        debug!("Saving window session: {} -> {}", window_label, connection_id);
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO window_sessions (window_label, connection_id, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(window_label) DO UPDATE SET
                connection_id = excluded.connection_id,
                updated_at = excluded.updated_at",
            params![window_label, connection_id, now],
        ).map_err(|e| {
            error!("Failed to save window session for {}: {}", window_label, e);
            format!("Failed to save window session: {}", e)
        })?;

        Ok(())
    }

    pub fn get_window_session(&self, window_label: &str) -> Result<Option<String>, String> {
        debug!("Getting window session for label '{}'", window_label);
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        let mut stmt = conn.prepare("SELECT connection_id FROM window_sessions WHERE window_label = ?1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;
        
        let result = stmt.query_row(params![window_label], |row| row.get::<_, String>(0)).optional()
            .map_err(|e| {
                error!("Failed to get window session for {}: {}", window_label, e);
                format!("Failed to get window session: {}", e)
            })?;

        Ok(result)
    }

    pub fn delete_window_session(&self, window_label: &str) -> Result<(), String> {
        debug!("Deleting window session for label '{}'", window_label);
        let conn = self.db.lock()
            .map_err(|e| format!("Failed to lock database: {}", e))?;

        conn.execute("DELETE FROM window_sessions WHERE window_label = ?1", params![window_label])
            .map_err(|e| {
                error!("Failed to delete window session for {}: {}", window_label, e);
                format!("Failed to delete window session: {}", e)
            })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{MasterKeyManager, MasterKey};
    use std::fs;
    use rand::{RngCore, rngs::OsRng};

    const CREATE_CONNECTIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS connections (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        engine TEXT NOT NULL,
        
        -- Optional summary fields (for list views & indexing only)
        host TEXT,
        port INTEGER,
        database TEXT,
        username TEXT,
        
        -- Transport/security summary flags
        uses_ssh INTEGER DEFAULT FALSE,
        uses_tls INTEGER DEFAULT FALSE,
        
        -- Canonical, versioned configuration
        config_json TEXT NOT NULL,
        
        -- UX / metadata
        is_favorite INTEGER DEFAULT FALSE,
        color_tag TEXT,
        
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        last_connected_at INTEGER,
        connection_count INTEGER DEFAULT 0
    );
    "#;

    const CREATE_CONNECTIONS_INDEXES: &str = r#"
    CREATE INDEX IF NOT EXISTS idx_connections_engine ON connections(engine);
    CREATE INDEX IF NOT EXISTS idx_connections_name ON connections(name COLLATE NOCASE);
    CREATE INDEX IF NOT EXISTS idx_connections_favorite ON connections(is_favorite);
    CREATE INDEX IF NOT EXISTS idx_connections_last_used ON connections(last_connected_at DESC);
    "#;

    const CREATE_CREDENTIALS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS credentials (
        connection_id TEXT NOT NULL,
        credential_key TEXT NOT NULL,
        encrypted_value BLOB NOT NULL,
        nonce BLOB NOT NULL,
        encryption_version INTEGER NOT NULL DEFAULT 1,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (connection_id, credential_key)
    );
    "#;

    fn create_test_manager() -> (ConnectionManager, Arc<Mutex<SqliteConnection>>) {
        let temp_dir = std::env::temp_dir().join(format!("test_tables_conn_{}", OsRng.next_u64()));
        fs::create_dir_all(&temp_dir).unwrap();
        let db = Arc::new(Mutex::new(SqliteConnection::open_in_memory().unwrap()));
        {
            let conn = db.lock().unwrap();
            conn.execute(CREATE_CONNECTIONS_TABLE, []).unwrap();
            conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_engine ON connections(engine);", []).unwrap();
            conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_name ON connections(name COLLATE NOCASE);", []).unwrap();
            conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_favorite ON connections(is_favorite);", []).unwrap();
            conn.execute("CREATE INDEX IF NOT EXISTS idx_connections_last_used ON connections(last_connected_at DESC);", []).unwrap();
            conn.execute(CREATE_CREDENTIALS_TABLE, []).unwrap();
        }
        let key_manager = MasterKeyManager::new(&temp_dir);
        let _master_key = key_manager.load_or_generate().unwrap();
        let credential_manager = Arc::new(CredentialManager::new(&temp_dir, Arc::clone(&db)).unwrap());
        let active_connections = Arc::new(Mutex::new(std::collections::HashSet::new()));
        let manager = ConnectionManager::new(db, credential_manager, active_connections);
        (manager, Arc::new(Mutex::new(SqliteConnection::open_in_memory().unwrap()))) // dummy db for manager
    }

    fn create_sample_connection() -> Connection {
        let now = chrono::Utc::now().timestamp();
        Connection {
            id: "test_conn_1".to_string(),
            name: "Test Connection".to_string(),
            engine: "postgres".to_string(),
            host: Some("localhost".to_string()),
            port: Some(5432),
            database: Some("testdb".to_string()),
            username: Some("user".to_string()),
            uses_ssh: false,
            uses_tls: true,
            config_json: r#"{"db":{"host":"localhost","port":5432,"database":"testdb","username":"user"}}"#.to_string(),
            is_favorite: false,
            color_tag: None,
            created_at: now,
            updated_at: now,
            last_connected_at: None,
            connection_count: 0,
        }
    }

    #[test]
    fn test_create_and_get_connection() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        let creds = SecureCredentials::new();
        manager.create_connection(connection.clone(), creds).unwrap();
        let (retrieved_conn, retrieved_creds) = manager.get_connection("test_conn_1").unwrap();
        assert_eq!(retrieved_conn.id, "test_conn_1");
        assert_eq!(retrieved_conn.name, "Test Connection");
        assert!(retrieved_creds.is_empty());
    }

    #[test]
    fn test_create_connection_with_credentials() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        let mut creds = SecureCredentials::new();
        creds.password = Some("secret123".into());
        manager.create_connection(connection.clone(), creds).unwrap();
        let (retrieved_conn, retrieved_creds) = manager.get_connection("test_conn_1").unwrap();
        assert_eq!(retrieved_conn.id, "test_conn_1");
        assert_eq!(retrieved_creds.password.as_ref().map(|s| s.expose()), Some("secret123"));
    }

    #[test]
    fn test_update_connection() {
        let (manager, _) = create_test_manager();
        let mut connection = create_sample_connection();
        let creds = SecureCredentials::new();
        manager.create_connection(connection.clone(), creds).unwrap();
        connection.name = "Updated Connection".to_string();
        manager.update_connection(connection, None).unwrap();
        let (retrieved_conn, _) = manager.get_connection("test_conn_1").unwrap();
        assert_eq!(retrieved_conn.name, "Updated Connection");
    }

    #[test]
    fn test_delete_connection() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        let creds = SecureCredentials::new();
        manager.create_connection(connection, creds).unwrap();
        assert!(manager.get_connection("test_conn_1").is_ok());
        manager.delete_connection("test_conn_1").unwrap();
        assert!(manager.get_connection("test_conn_1").is_err());
    }

    #[test]
    fn test_list_connections() {
        let (manager, _) = create_test_manager();
        let connection1 = create_sample_connection();
        let mut connection2 = create_sample_connection();
        connection2.id = "test_conn_2".to_string();
        connection2.name = "Test Connection 2".to_string();
        manager.create_connection(connection1, SecureCredentials::new()).unwrap();
        manager.create_connection(connection2, SecureCredentials::new()).unwrap();
        let connections = manager.list_connections().unwrap();
        assert_eq!(connections.len(), 2);
    }

    #[test]
    fn test_get_connection_metadata() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        manager.create_connection(connection.clone(), SecureCredentials::new()).unwrap();
        let metadata = manager.get_connection_metadata("test_conn_1").unwrap();
        assert_eq!(metadata.id, "test_conn_1");
        assert_eq!(metadata.name, "Test Connection");
    }

    #[test]
    fn test_update_connection_stats() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        manager.create_connection(connection.clone(), SecureCredentials::new()).unwrap();
        manager.update_connection_stats("test_conn_1").unwrap();
        let (retrieved, _) = manager.get_connection("test_conn_1").unwrap();
        assert_eq!(retrieved.connection_count, 1);
    }

    #[test]
    fn test_get_favorite_connections() {
        let (manager, _) = create_test_manager();
        let mut connection = create_sample_connection();
        connection.is_favorite = true;
        manager.create_connection(connection.clone(), SecureCredentials::new()).unwrap();
        let favorites = manager.get_favorite_connections().unwrap();
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].id, "test_conn_1");
    }

    #[test]
    fn test_search_connections() {
        let (manager, _) = create_test_manager();
        let connection = create_sample_connection();
        manager.create_connection(connection.clone(), SecureCredentials::new()).unwrap();
        let results = manager.search_connections("Test").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test_conn_1");
    }
}
