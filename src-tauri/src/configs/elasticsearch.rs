use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::{Transport, TlsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ElasticsearchConfig {
    pub version: u32,
    pub db: ElasticsearchDb,
    pub transport: Transport,
    pub tls: Option<TlsConfig>,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ElasticsearchDb {
    pub host: String,
    #[serde(default = "default_elasticsearch_port")]
    pub port: u16,
    pub username: Option<String>,
    pub api_key: Option<String>,
}

fn default_elasticsearch_port() -> u16 { 9200 }
