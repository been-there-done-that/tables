use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Transport, TlsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    pub version: u32,
    pub db: RedisDb,
    pub transport: Transport,
    pub tls: Option<TlsConfig>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisDb {
    pub host: String,
    #[serde(default = "default_redis_port")]
    pub port: u16,
    pub database: Option<u16>, // Redis database number
    pub username: Option<String>,
}

fn default_redis_port() -> u16 { 6379 }
