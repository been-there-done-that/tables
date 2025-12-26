use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,                    // UUID
    pub name: String,
    pub engine: DatabaseEngine,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub auth_type: AuthType,
    pub ssl_enabled: bool,
    pub ssh_tunnel_enabled: bool,
    pub ssh_tunnel_host: Option<String>,
    pub ssh_tunnel_port: Option<u16>,
    pub ssh_tunnel_username: Option<String>,
    pub connection_params: HashMap<String, serde_json::Value>,
    pub is_favorite: bool,
    pub color_tag: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_connected_at: Option<i64>,
    pub connection_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseEngine {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    Elasticsearch,
    S3,
    Custom(String),
}

impl DatabaseEngine {
    pub fn default_port(&self) -> Option<u16> {
        match self {
            DatabaseEngine::PostgreSQL => Some(5432),
            DatabaseEngine::MySQL => Some(3306),
            DatabaseEngine::SQLite => None,
            DatabaseEngine::MongoDB => Some(27017),
            DatabaseEngine::Redis => Some(6379),
            DatabaseEngine::Elasticsearch => Some(9200),
            DatabaseEngine::S3 => None, // S3 doesn't use traditional ports
            DatabaseEngine::Custom(_) => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DatabaseEngine::PostgreSQL => "PostgreSQL",
            DatabaseEngine::MySQL => "MySQL",
            DatabaseEngine::SQLite => "SQLite",
            DatabaseEngine::MongoDB => "MongoDB",
            DatabaseEngine::Redis => "Redis",
            DatabaseEngine::Elasticsearch => "Elasticsearch",
            DatabaseEngine::S3 => "Amazon S3",
            DatabaseEngine::Custom(_) => "Custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl AuthType {
    pub fn default_value() -> Self {
        AuthType::Password
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
    #[serde(skip)]
    pub password: Option<SecretString>,
    #[serde(skip)]
    pub ssh_private_key: Option<SecretString>,
    #[serde(skip)]
    pub ssh_passphrase: Option<SecretString>,
    #[serde(skip)]
    pub ssl_certificate: Option<SecretString>,
    #[serde(skip)]
    pub ssl_private_key: Option<SecretString>,
    #[serde(skip)]
    pub ssl_ca_certificate: Option<SecretString>,
    #[serde(skip)]
    pub api_token: Option<SecretString>,
    // AWS S3 credentials
    #[serde(skip)]
    pub aws_access_key_id: Option<SecretString>,
    #[serde(skip)]
    pub aws_secret_access_key: Option<SecretString>,
    #[serde(skip)]
    pub aws_session_token: Option<SecretString>,
}

impl SecureCredentials {
    pub fn new() -> Self {
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
        self.password.is_none()
            && self.ssh_private_key.is_none()
            && self.ssh_passphrase.is_none()
            && self.ssl_certificate.is_none()
            && self.ssl_private_key.is_none()
            && self.ssl_ca_certificate.is_none()
            && self.api_token.is_none()
            && self.aws_access_key_id.is_none()
            && self.aws_secret_access_key.is_none()
            && self.aws_session_token.is_none()
    }
}

impl Default for SecureCredentials {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    pub fn expose(&self) -> &str {
        &self.inner
    }

    pub fn into_string(self) -> String {
        self.inner
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

impl Connection {
    pub fn new(name: String, engine: DatabaseEngine) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            engine: engine.clone(),
            host: None,
            port: engine.default_port(),
            database: None,
            username: None,
            auth_type: AuthType::default(),
            ssl_enabled: false,
            ssh_tunnel_enabled: false,
            ssh_tunnel_host: None,
            ssh_tunnel_port: None,
            ssh_tunnel_username: None,
            connection_params: HashMap::new(),
            is_favorite: false,
            color_tag: None,
            created_at: now,
            updated_at: now,
            last_connected_at: None,
            connection_count: 0,
        }
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
    }

    pub fn increment_connection_count(&mut self) {
        self.connection_count += 1;
        self.last_connected_at = Some(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64);
        self.update_timestamp();
    }
}

// Helper functions for database operations
pub fn load_connection_from_row(row: &rusqlite::Row<'_>) -> Result<Connection, rusqlite::Error> {
    let engine_str: String = row.get(3)?;
    let engine = match engine_str.as_str() {
        "postgresql" => DatabaseEngine::PostgreSQL,
        "mysql" => DatabaseEngine::MySQL,
        "sqlite" => DatabaseEngine::SQLite,
        "mongodb" => DatabaseEngine::MongoDB,
        "redis" => DatabaseEngine::Redis,
        "elasticsearch" => DatabaseEngine::Elasticsearch,
        custom => DatabaseEngine::Custom(custom.to_string()),
    };

    let auth_type_str: String = row.get(8)?;
    let auth_type = match auth_type_str.as_str() {
        "password" => AuthType::Password,
        "ssh_key" => AuthType::SshKey,
        "ssl_cert" => AuthType::SslCert,
        "api_token" => AuthType::ApiToken,
        "windows_auth" => AuthType::WindowsAuth,
        "kerberos" => AuthType::Kerberos,
        "none" => AuthType::None,
        "aws_credentials" => AuthType::AwsCredentials,
        "aws_profile" => AuthType::AwsProfile,
        "aws_iam_role" => AuthType::AwsIamRole,
        _ => AuthType::Password,
    };

    let connection_params_json: Option<String> = row.get(15)?;
    let connection_params = if let Some(json) = connection_params_json {
        serde_json::from_str(&json).unwrap_or_default()
    } else {
        HashMap::new()
    };

    Ok(Connection {
        id: row.get(0)?,
        name: row.get(1)?,
        engine,
        host: row.get(4)?,
        port: row.get(5)?,
        database: row.get(6)?,
        username: row.get(7)?,
        auth_type,
        ssl_enabled: row.get::<_, i64>(9)? != 0,
        ssh_tunnel_enabled: row.get::<_, i64>(10)? != 0,
        ssh_tunnel_host: row.get(11)?,
        ssh_tunnel_port: row.get(12)?,
        ssh_tunnel_username: row.get(13)?,
        connection_params,
        is_favorite: row.get::<_, i64>(16)? != 0,
        color_tag: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
        last_connected_at: Some(row.get(20)?),
        connection_count: row.get(21)?,
    })
}
