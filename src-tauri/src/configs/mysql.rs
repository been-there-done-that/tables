use serde::{Deserialize, Serialize};
use super::{Transport, TlsConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MysqlConfig {
    pub version: u32,
    pub db: MysqlDb,
    pub transport: Transport,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MysqlDb {
    pub host: String,
    #[serde(default = "default_mysql_port")]
    pub port: u16,
    pub database: String,
    pub username: String,
}

fn default_mysql_port() -> u16 { 3306 }
