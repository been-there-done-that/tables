use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::BaseConfig;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteConfig {
    pub version: u32,
    pub mode: SqliteMode,
    pub file: Option<String>,
    pub options: Option<SqliteOptions>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SqliteMode {
    File,
    Memory,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteOptions {
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub pragmas: HashMap<String, serde_json::Value>,
}
