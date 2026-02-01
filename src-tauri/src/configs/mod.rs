pub mod postgres;
pub mod sqlite;
pub mod mongodb;
pub mod redis;
pub mod elasticsearch;
pub mod s3;
pub mod mysql;
pub mod validation;

use serde::{Deserialize, Serialize};

// Re-export all config types
pub use postgres::*;
pub use sqlite::*;
pub use mongodb::*;
pub use redis::*;
pub use elasticsearch::*;
pub use s3::*;
pub use mysql::*;
pub use validation::*;

/// Base configuration that all configs must have
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BaseConfig {
    pub version: u32,
}

/// Transport layer configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Transport {
    #[serde(rename = "direct")]
    Direct,
    
    #[serde(rename = "ssh")]
    Ssh { ssh: SshConfig },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SshConfig {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
}

fn default_ssh_port() -> u16 { 22 }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SshAuth {
    #[serde(rename = "key")]
    Key { key_ref: String },
    
    #[serde(rename = "password")]
    Password { password_ref: String },
    
    #[serde(rename = "agent")]
    Agent,
}

/// TLS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub sslmode: Option<String>,
    pub ca_ref: Option<String>,
    pub cert_ref: Option<String>,
    pub key_ref: Option<String>,
}

/// Runtime connection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "engine", rename_all = "lowercase")]
pub enum RuntimeConnection {
    Postgres(PostgresConfig),
    Sqlite(SqliteConfig),
    MongoDb(MongoDbConfig),
    Redis(RedisConfig),
    Elasticsearch(ElasticsearchConfig),
    S3(S3Config),
    Mysql(MysqlConfig),
}

impl RuntimeConnection {
    pub fn engine(&self) -> &'static str {
        match self {
            RuntimeConnection::Postgres(_) => "postgres",
            RuntimeConnection::Sqlite(_) => "sqlite",
            RuntimeConnection::MongoDb(_) => "mongodb",
            RuntimeConnection::Redis(_) => "redis",
            RuntimeConnection::Elasticsearch(_) => "elasticsearch",
            RuntimeConnection::S3(_) => "s3",
            RuntimeConnection::Mysql(_) => "mysql",
        }
    }
    


    pub fn summary_fields(&self) -> ConnectionSummary {
        match self {
            RuntimeConnection::Postgres(config) => ConnectionSummary {
                host: Some(config.db.host.clone()),
                port: Some(config.db.port),
                database: Some(config.db.database.clone()),
                username: Some(config.db.username.clone()),
                uses_ssh: matches!(config.transport, Transport::Ssh { .. }),
                uses_tls: config.tls.as_ref().map_or(false, |t| t.enabled),
            },
            RuntimeConnection::Sqlite(config) => ConnectionSummary {
                host: None,
                port: None,
                database: config.file.clone(),
                username: None,
                uses_ssh: false,
                uses_tls: false,
            },
            RuntimeConnection::MongoDb(config) => ConnectionSummary {
                host: Some(config.db.host.clone()),
                port: Some(config.db.port),
                database: Some(config.db.database.clone()),
                username: config.db.username.clone(),
                uses_ssh: matches!(config.transport, Transport::Ssh { .. }),
                uses_tls: config.tls.as_ref().map_or(false, |t| t.enabled),
            },
            RuntimeConnection::Redis(config) => ConnectionSummary {
                host: Some(config.db.host.clone()),
                port: Some(config.db.port),
                database: config.db.database.as_ref().map(|i| i.to_string()),
                username: None,
                uses_ssh: matches!(config.transport, Transport::Ssh { .. }),
                uses_tls: config.tls.as_ref().map_or(false, |t| t.enabled),
            },
            RuntimeConnection::Elasticsearch(config) => ConnectionSummary {
                host: Some(config.db.host.clone()),
                port: Some(config.db.port),
                database: None,
                username: config.db.username.clone(),
                uses_ssh: matches!(config.transport, Transport::Ssh { .. }),
                uses_tls: config.tls.as_ref().map_or(false, |t| t.enabled),
            },
            RuntimeConnection::S3(config) => ConnectionSummary {
                host: Some(config.endpoint.clone()),
                port: None,
                database: Some(config.bucket.clone()),
                username: None,
                uses_ssh: false,
                uses_tls: true, // S3 always uses TLS
            },
            RuntimeConnection::Mysql(config) => ConnectionSummary {
                host: Some(config.db.host.clone()),
                port: Some(config.db.port),
                database: Some(config.db.database.clone()),
                username: Some(config.db.username.clone()),
                uses_ssh: matches!(config.transport, Transport::Ssh { .. }),
                uses_tls: config.tls.as_ref().map_or(false, |t| t.enabled),
            },
        }
    }
}

/// Summary fields for indexing and list views
#[derive(Debug, Clone)]
pub struct ConnectionSummary {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub uses_ssh: bool,
    pub uses_tls: bool,
}
