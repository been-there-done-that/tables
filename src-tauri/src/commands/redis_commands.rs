use crate::connection::{Connection as DatabaseConnection, SecureCredentials, ConnectionInfo};
use crate::connection_manager::{ConnectionManager, ConnectionManagerState};
use crate::DatabaseState;
use tauri::State;
use redis::{Client, RedisResult};
use std::collections::HashMap;

/// Get Redis server information
#[tauri::command]
pub async fn get_redis_info(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<RedisInfo, String> {
    let manager = ConnectionManager::from_state(&db_state, &conn_state);
    
    // Test connection and get info
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    let info: String = redis::cmd("INFO")
        .arg("server")
        .query(&mut conn)
        .map_err(|e| format!("Failed to get Redis info: {}", e))?;
    
    let config: HashMap<String, String> = redis::cmd("CONFIG")
        .arg("GET")
        .arg("*")
        .query(&mut conn)
        .map_err(|e| format!("Failed to get Redis config: {}", e))?;
    
    let dbsize: i64 = redis::cmd("DBSIZE")
        .query(&mut conn)
        .map_err(|e| format!("Failed to get database size: {}", e))?;
    
    Ok(RedisInfo {
        version: extract_redis_version(&info),
        mode: extract_redis_mode(&info),
        role: extract_redis_role(&info),
        os: extract_redis_os(&info),
        uptime: extract_redis_uptime(&info),
        connected_clients: extract_connected_clients(&info),
        used_memory: extract_used_memory(&info),
        dbsize,
        config,
        raw_info: info,
    })
}

/// List all Redis databases
#[tauri::command]
pub async fn list_redis_databases(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<RedisDatabase>, String> {
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    let info: String = redis::cmd("INFO")
        .arg("keyspace")
        .query(&mut conn)
        .map_err(|e| format!("Failed to get keyspace info: {}", e))?;
    
    parse_redis_databases(&info)
}

/// Get all keys in a Redis database
#[tauri::command]
pub async fn list_redis_keys(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    database: Option<i64>,
    pattern: Option<String>,
    limit: Option<i64>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<Vec<RedisKey>, String> {
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    // Select database if specified
    if let Some(db) = database {
        redis::cmd("SELECT")
            .arg(db)
            .query::<()>(&mut conn)
            .map_err(|e| format!("Failed to select database {}: {}", db, e))?;
    }
    
    let search_pattern = pattern.unwrap_or_else(|| "*".to_string());
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(&search_pattern)
        .query(&mut conn)
        .map_err(|e| format!("Failed to get keys: {}", e))?;
    
    let mut redis_keys = Vec::new();
    let limit_count = limit.unwrap_or(1000) as usize;
    
    for (i, key) in keys.iter().take(limit_count).enumerate() {
        let key_type: String = redis::cmd("TYPE")
            .arg(key)
            .query(&mut conn)
            .map_err(|e| format!("Failed to get key type: {}", e))?;
        
        let ttl: i64 = redis::cmd("TTL")
            .arg(key)
            .query(&mut conn)
            .map_err(|e| format!("Failed to get TTL: {}", e))?;
        
        let size = estimate_key_size(&mut conn, key, &key_type)?;
        
        redis_keys.push(RedisKey {
            key: key.clone(),
            key_type,
            ttl,
            size,
            index: i as i64,
        });
    }
    
    Ok(redis_keys)
}

/// Get Redis key value and details
#[tauri::command]
pub async fn get_redis_key(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    key: String,
    database: Option<i64>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<RedisKeyValue, String> {
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    // Select database if specified
    if let Some(db) = database {
        redis::cmd("SELECT")
            .arg(db)
            .query::<()>(&mut conn)
            .map_err(|e| format!("Failed to select database {}: {}", db, e))?;
    }
    
    let key_type: String = redis::cmd("TYPE")
        .arg(&key)
        .query(&mut conn)
        .map_err(|e| format!("Failed to get key type: {}", e))?;
    
    let ttl: i64 = redis::cmd("TTL")
        .arg(&key)
        .query(&mut conn)
        .map_err(|e| format!("Failed to get TTL: {}", e))?;
    
    let value = get_redis_value(&mut conn, &key, &key_type)?;
    
    Ok(RedisKeyValue {
        key,
        key_type,
        ttl,
        value,
    })
}

/// Execute Redis command
#[tauri::command]
pub async fn execute_redis_command(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    command: String,
    args: Vec<String>,
    database: Option<i64>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<RedisCommandResult, String> {
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    // Select database if specified
    if let Some(db) = database {
        redis::cmd("SELECT")
            .arg(db)
            .query::<()>(&mut conn)
            .map_err(|e| format!("Failed to select database {}: {}", db, e))?;
    }
    
    let mut cmd = redis::cmd(&command);
    for arg in args {
        cmd.arg(arg);
    }
    
    let result = match cmd.query::<redis::Value>(&mut conn) {
        Ok(value) => RedisCommandResult {
            success: true,
            result: format_redis_value(&value),
            error: None,
        },
        Err(e) => RedisCommandResult {
            success: false,
            result: String::new(),
            error: Some(e.to_string()),
        },
    };
    
    Ok(result)
}

/// Delete Redis key
#[tauri::command]
pub async fn delete_redis_key(
    connection: DatabaseConnection,
    credentials: SecureCredentials,
    key: String,
    database: Option<i64>,
    db_state: State<'_, DatabaseState>,
    conn_state: State<'_, ConnectionManagerState>,
) -> Result<bool, String> {
    let client = create_redis_client(&connection, &credentials)?;
    let mut conn = client.get_connection().map_err(|e| format!("Failed to connect to Redis: {}", e))?;
    
    // Select database if specified
    if let Some(db) = database {
        redis::cmd("SELECT")
            .arg(db)
            .query::<()>(&mut conn)
            .map_err(|e| format!("Failed to select database {}: {}", db, e))?;
    }
    
    let deleted: i64 = redis::cmd("DEL")
        .arg(&key)
        .query(&mut conn)
        .map_err(|e| format!("Failed to delete key: {}", e))?;
    
    Ok(deleted > 0)
}

// Helper functions

fn create_redis_client(
    connection: &DatabaseConnection,
    credentials: &SecureCredentials,
) -> Result<Client, String> {
    let host = connection.host.as_ref().ok_or("Host is required")?;
    let port = connection.port.unwrap_or(6379);
    
    let mut connection_string = format!("redis://");
    
    // Add authentication if provided
    if let Some(password) = &credentials.password {
        connection_string.push_str(&format!(":{}@", password.expose()));
    }
    
    connection_string.push_str(&format!("{}:{}", host, port));
    
    Client::open(connection_string)
        .map_err(|e| format!("Failed to create Redis client: {}", e))
}

fn extract_redis_version(info: &str) -> String {
    for line in info.lines() {
        if line.starts_with("redis_version:") {
            return line.split(':').nth(1).unwrap_or("unknown").to_string();
        }
    }
    "unknown".to_string()
}

fn extract_redis_mode(info: &str) -> String {
    for line in info.lines() {
        if line.starts_with("redis_mode:") {
            return line.split(':').nth(1).unwrap_or("unknown").to_string();
        }
    }
    "unknown".to_string()
}

fn extract_redis_role(info: &str) -> String {
    for line in info.lines() {
        if line.starts_with("role:") {
            return line.split(':').nth(1).unwrap_or("unknown").to_string();
        }
    }
    "unknown".to_string()
}

fn extract_redis_os(info: &str) -> String {
    for line in info.lines() {
        if line.starts_with("os:") {
            return line.split(':').nth(1).unwrap_or("unknown").to_string();
        }
    }
    "unknown".to_string()
}

fn extract_redis_uptime(info: &str) -> i64 {
    for line in info.lines() {
        if line.starts_with("uptime_in_seconds:") {
            return line.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    0
}

fn extract_connected_clients(info: &str) -> i64 {
    for line in info.lines() {
        if line.starts_with("connected_clients:") {
            return line.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    0
}

fn extract_used_memory(info: &str) -> i64 {
    for line in info.lines() {
        if line.starts_with("used_memory:") {
            return line.split(':').nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    0
}

fn parse_redis_databases(keyspace_info: &str) -> Result<Vec<RedisDatabase>, String> {
    let mut databases = Vec::new();
    
    for line in keyspace_info.lines() {
        if line.starts_with("db") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                let db_name = parts[0].to_string();
                let stats = parts[1];
                
                let keys = extract_stat(stats, "keys");
                let expires = extract_stat(stats, "expires");
                let avg_ttl = extract_stat(stats, "avg_ttl");
                
                databases.push(RedisDatabase {
                    name: db_name,
                    keys,
                    expires,
                    avg_ttl,
                });
            }
        }
    }
    
    Ok(databases)
}

fn extract_stat(stats: &str, stat_name: &str) -> i64 {
    for part in stats.split(',') {
        if part.starts_with(stat_name) {
            return part.split('=').nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    0
}

fn estimate_key_size(conn: &mut redis::Connection, key: &str, key_type: &str) -> Result<i64, String> {
    match key_type {
        "string" => {
            let len: i64 = redis::cmd("STRLEN")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get string length: {}", e))?;
            Ok(len)
        }
        "list" => {
            let len: i64 = redis::cmd("LLEN")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get list length: {}", e))?;
            Ok(len)
        }
        "set" => {
            let len: i64 = redis::cmd("SCARD")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get set size: {}", e))?;
            Ok(len)
        }
        "zset" => {
            let len: i64 = redis::cmd("ZCARD")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get zset size: {}", e))?;
            Ok(len)
        }
        "hash" => {
            let len: i64 = redis::cmd("HLEN")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get hash size: {}", e))?;
            Ok(len)
        }
        _ => Ok(0),
    }
}

fn get_redis_value(conn: &mut redis::Connection, key: &str, key_type: &str) -> Result<String, String> {
    match key_type {
        "string" => {
            let value: String = redis::cmd("GET")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get string value: {}", e))?;
            Ok(value)
        }
        "list" => {
            let value: Vec<String> = redis::cmd("LRANGE")
                .arg(key)
                .arg(0)
                .arg(-1)
                .query(conn)
                .map_err(|e| format!("Failed to get list value: {}", e))?;
            Ok(serde_json::to_string(&value).unwrap_or_default())
        }
        "set" => {
            let value: Vec<String> = redis::cmd("SMEMBERS")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get set value: {}", e))?;
            Ok(serde_json::to_string(&value).unwrap_or_default())
        }
        "hash" => {
            let value: HashMap<String, String> = redis::cmd("HGETALL")
                .arg(key)
                .query(conn)
                .map_err(|e| format!("Failed to get hash value: {}", e))?;
            Ok(serde_json::to_string(&value).unwrap_or_default())
        }
        _ => Ok(format!("Unsupported key type: {}", key_type)),
    }
}

fn format_redis_value(value: &redis::Value) -> String {
    match value {
        redis::Value::Nil => "null".to_string(),
        redis::Value::Int(i) => i.to_string(),
        redis::Value::Data(data) => String::from_utf8_lossy(data).to_string(),
        redis::Value::Bulk(values) => {
            let strings: Vec<String> = values.iter()
                .map(format_redis_value)
                .collect();
            format!("[{}]", strings.join(", "))
        }
        redis::Value::Status(s) => s.clone(),
        redis::Value::Okay => "OK".to_string(),
    }
}

// Data structures

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RedisInfo {
    pub version: String,
    pub mode: String,
    pub role: String,
    pub os: String,
    pub uptime: i64,
    pub connected_clients: i64,
    pub used_memory: i64,
    pub dbsize: i64,
    pub config: HashMap<String, String>,
    pub raw_info: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RedisDatabase {
    pub name: String,
    pub keys: i64,
    pub expires: i64,
    pub avg_ttl: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RedisKey {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub size: i64,
    pub index: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RedisKeyValue {
    pub key: String,
    pub key_type: String,
    pub ttl: i64,
    pub value: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RedisCommandResult {
    pub success: bool,
    pub result: String,
    pub error: Option<String>,
}
