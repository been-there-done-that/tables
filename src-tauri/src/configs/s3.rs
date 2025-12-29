use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::BaseConfig;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct S3Config {
    pub version: u32,
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub auth: S3Auth,
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum S3Auth {
    Credentials {
        access_key_ref: String,
        secret_key_ref: String,
        session_token_ref: Option<String>,
    },
    Profile {
        profile_name: String,
    },
    IamRole {
        role_arn: String,
    },
}
