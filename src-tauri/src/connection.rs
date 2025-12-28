use serde::{Deserialize, Serialize};
use uuid::Uuid;
use log::{info, debug, warn, error, trace};
use crate::configs::{RuntimeConnection, ConnectionSummary, validate_config_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,                    // UUID
    pub name: String,
    pub engine: String,                 // "postgres", "sqlite", etc.
    
    // Summary fields for indexing and list views
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    
    // Security flags
    pub uses_ssh: bool,
    pub uses_tls: bool,
    
    // Canonical configuration
    pub config_json: String,            // Serialized RuntimeConnection
    
    // UX / metadata
    pub is_favorite: bool,
    pub color_tag: Option<String>,
    
    pub created_at: i64,
    pub updated_at: i64,
    pub last_connected_at: Option<i64>,
    pub connection_count: i32,
}

impl Connection {
    pub fn new(name: String, config: RuntimeConnection) -> Result<Self, String> {
        debug!("Creating new connection '{}' with engine '{}'", name, config.engine());
        let summary = config.summary_fields();
        let engine = config.engine().to_string();
        debug!("Serializing config to JSON for connection '{}'", name);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| {
                error!("Failed to serialize config for connection '{}': {}", name, e);
                format!("Failed to serialize config: {}", e)
            })?;
        
        debug!("Validating config JSON for connection '{}'", name);
        // Validate the config JSON before storing
        validate_config_json(&config_json, &engine)?;
        
        debug!("Successfully created connection '{}' with ID '{}'", name, "to be generated");
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            engine,
            host: summary.host,
            port: summary.port,
            database: summary.database,
            username: summary.username,
            uses_ssh: summary.uses_ssh,
            uses_tls: summary.uses_tls,
            config_json,
            is_favorite: false,
            color_tag: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            last_connected_at: None,
            connection_count: 0,
        })
    }
    
    pub fn parse_config(&self) -> Result<RuntimeConnection, serde_json::Error> {
        trace!("Parsing config JSON for connection '{}'", self.id);
        serde_json::from_str(&self.config_json)
    }
    
    pub fn update_config(&mut self, config: RuntimeConnection) -> Result<(), String> {
        debug!("Updating config for connection '{}'", self.id);
        let summary = config.summary_fields();
        let engine = config.engine().to_string();
        debug!("Serializing updated config to JSON for connection '{}'", self.id);
        let config_json = serde_json::to_string(&config)
            .map_err(|e| {
                error!("Failed to serialize updated config for connection '{}': {}", self.id, e);
                format!("Failed to serialize config: {}", e)
            })?;
        
        debug!("Validating updated config JSON for connection '{}'", self.id);
        // Validate the config JSON before storing
        validate_config_json(&config_json, &engine)?;
        
        self.engine = engine;
        self.host = summary.host;
        self.port = summary.port;
        self.database = summary.database;
        self.username = summary.username;
        self.uses_ssh = summary.uses_ssh;
        self.uses_tls = summary.uses_tls;
        self.config_json = config_json;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        debug!("Successfully updated config for connection '{}'", self.id);
        Ok(())
    }

    pub fn update_timestamp(&mut self) {
        trace!("Updating timestamp for connection '{}'", self.id);
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseEngine {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    Elasticsearch,
    S3,
    Athena,
    Custom(String),
}

impl DatabaseEngine {
    pub fn default_port(&self) -> Option<u16> {
        trace!("Getting default port for database engine {:?}", self);
        match self {
            DatabaseEngine::PostgreSQL => Some(5432),
            DatabaseEngine::MySQL => Some(3306),
            DatabaseEngine::SQLite => None,
            DatabaseEngine::MongoDB => Some(27017),
            DatabaseEngine::Redis => Some(6379),
            DatabaseEngine::Elasticsearch => Some(9200),
            DatabaseEngine::S3 => None, // S3 doesn't use traditional ports
            DatabaseEngine::Athena => None, // Athena uses HTTPS/HTTP
            DatabaseEngine::Custom(_) => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        trace!("Getting display name for database engine {:?}", self);
        match self {
            DatabaseEngine::PostgreSQL => "PostgreSQL",
            DatabaseEngine::MySQL => "MySQL",
            DatabaseEngine::SQLite => "SQLite",
            DatabaseEngine::MongoDB => "MongoDB",
            DatabaseEngine::Redis => "Redis",
            DatabaseEngine::Elasticsearch => "Elasticsearch",
            DatabaseEngine::S3 => "Amazon S3",
            DatabaseEngine::Athena => "Amazon Athena",
            DatabaseEngine::Custom(_) => "Custom",
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Password,
    SshKey,
    SslCert,
    ApiToken,
    WindowsAuth,
    Kerberos,
    None, // For SQLite or no auth
    AwsCredentials,
    AwsProfile,
    AwsIamRole,
    AthenaJdbc, // Athena JDBC connection
}

impl AuthType {
    pub fn default_value() -> Self {
        AuthType::Password
    }

    pub fn to_string(&self) -> &'static str {
        trace!("Converting auth type {:?} to string", self);
        match self {
            AuthType::Password => "password",
            AuthType::SshKey => "ssh_key",
            AuthType::SslCert => "ssl_cert",
            AuthType::ApiToken => "api_token",
            AuthType::WindowsAuth => "windows_auth",
            AuthType::Kerberos => "kerberos",
            AuthType::None => "none",
            AuthType::AwsCredentials => "aws_credentials",
            AuthType::AwsProfile => "aws_profile",
            AuthType::AwsIamRole => "aws_iam_role",
            AuthType::AthenaJdbc => "athena_jdbc",
        }
    }
}

impl Default for AuthType {
    fn default() -> Self {
        Self::default_value()
    }
}

// Sensitive data - never stored in database
#[derive(Clone, Serialize, Deserialize)]
pub struct SecureCredentials {
    #[serde(skip_serializing)]
    pub password: Option<SecretString>,
    #[serde(skip_serializing)]
    pub ssh_private_key: Option<SecretString>,
    #[serde(skip_serializing)]
    pub ssh_passphrase: Option<SecretString>,
    #[serde(skip_serializing)]
    pub ssl_certificate: Option<SecretString>,
    #[serde(skip_serializing)]
    pub ssl_private_key: Option<SecretString>,
    #[serde(skip_serializing)]
    pub ssl_ca_certificate: Option<SecretString>,
    #[serde(skip_serializing)]
    pub api_token: Option<SecretString>,
    // AWS S3 credentials
    #[serde(skip_serializing)]
    pub aws_access_key_id: Option<SecretString>,
    #[serde(skip_serializing)]
    pub aws_secret_access_key: Option<SecretString>,
    #[serde(skip_serializing)]
    pub aws_session_token: Option<SecretString>,
}

impl SecureCredentials {
    pub fn new() -> Self {
        trace!("Creating new secure credentials");
        Self {
            password: None,
            ssh_private_key: None,
            ssh_passphrase: None,
            ssl_certificate: None,
            ssl_private_key: None,
            ssl_ca_certificate: None,
            api_token: None,
            aws_access_key_id: None,
            aws_secret_access_key: None,
            aws_session_token: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        let result = self.password.is_none()
            && self.ssh_private_key.is_none()
            && self.ssh_passphrase.is_none()
            && self.ssl_certificate.is_none()
            && self.ssl_private_key.is_none()
            && self.ssl_ca_certificate.is_none()
            && self.api_token.is_none()
            && self.aws_access_key_id.is_none()
            && self.aws_secret_access_key.is_none()
            && self.aws_session_token.is_none();
        trace!("Checking if secure credentials are empty: {}", result);
        result
    }
}

impl Default for SecureCredentials {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    pub fn new(s: String) -> Self {
        trace!("Creating new secret string");
        Self { inner: s }
    }

    pub fn expose(&self) -> &str {
        warn!("Exposing secret string - ensure secure handling");
        &self.inner
    }

    pub fn into_string(mut self) -> String {
        warn!("Converting secret to plain string - ensure zeroization");
        let result = self.inner.clone();
        self.zeroize();
        result
    }

    /// Zeroize the secret when dropped
    fn zeroize(&mut self) {
        trace!("Zeroizing secret string");
        // Simple zeroization - overwrite the string contents
        unsafe {
            let ptr = self.inner.as_mut_ptr();
            let len = self.inner.len();
            std::ptr::write_bytes(ptr, 0, len);
        }
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretString")
            .field("redacted", &"[REDACTED]")
            .finish()
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

// Connection test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub connected: bool,
    pub version: Option<String>,
    pub database_name: Option<String>,
    pub error: Option<String>,
    pub response_time_ms: Option<u64>,
}


// Helper functions for database operations
pub fn load_connection_from_row(row: &rusqlite::Row<'_>) -> Result<Connection, rusqlite::Error> {
    debug!("Loading connection from database row");
    Ok(Connection {
        id: row.get(0)?,
        name: row.get(1)?,
        engine: row.get(2)?,
        host: row.get(3)?,
        port: row.get(4)?,
        database: row.get(5)?,
        username: row.get(6)?,
        uses_ssh: row.get::<_, i64>(7)? != 0,
        uses_tls: row.get::<_, i64>(8)? != 0,
        config_json: row.get(9)?,
        is_favorite: row.get::<_, i64>(10)? != 0,
        color_tag: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        last_connected_at: row.get(14)?,
        connection_count: row.get(15)?,
    })
}
