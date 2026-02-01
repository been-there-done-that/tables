use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Transport, TlsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongoDbConfig {
    pub version: u32,
    pub db: MongoDbDb,
    pub transport: Transport,
    pub tls: Option<TlsConfig>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MongoDbDb {
    pub host: String,
    #[serde(default = "default_mongodb_port")]
    pub port: u16,
    pub database: String,
    pub username: Option<String>,
    pub auth_source: Option<String>,
}

fn default_mongodb_port() -> u16 { 27017 }
