use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Transport, TlsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresConfig {
    pub version: u32,
    pub db: PostgresDb,
    pub transport: Transport,
    pub tls: Option<TlsConfig>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostgresDb {
    pub host: String,
    #[serde(default = "default_postgres_port")]
    pub port: u16,
    pub database: String,
    pub username: String,
}

fn default_postgres_port() -> u16 { 5432 }
