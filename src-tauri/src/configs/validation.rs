use serde_json::Value;
use crate::configs::{RuntimeConnection, PostgresConfig, SqliteConfig};

/// Validates that a config JSON follows the expected contract
pub fn validate_config_json(json: &str, engine: &str) -> Result<(), String> {
    let parsed: Value = serde_json::from_str(json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    
    // Check version field exists and is supported
    let version = parsed.get("version")
        .and_then(|v| v.as_u64())
        .ok_or("Missing or invalid 'version' field")?;
    
    if version != 1 {
        return Err(format!("Unsupported config version: {}", version));
    }
    
    // Engine-specific validation
    match engine {
        "postgres" => validate_postgres_config(&parsed)?,
        "mysql" => validate_mysql_config(&parsed)?,
        "sqlite" => validate_sqlite_config(&parsed)?,
        "mongodb" => validate_mongodb_config(&parsed)?,
        "redis" => validate_redis_config(&parsed)?,
        "elasticsearch" => validate_elasticsearch_config(&parsed)?,
        _ => return Err(format!("Unsupported engine: {}", engine)),
    }
    
    Ok(())
}

fn validate_postgres_config(parsed: &Value) -> Result<(), String> {
    let db = parsed.get("db").ok_or("Missing 'db' field")?;
    
    // Required database fields
    if db.get("host").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.host' field".to_string());
    }
    if db.get("database").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.database' field".to_string());
    }
    if db.get("username").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.username' field".to_string());
    }
    
    // Validate transport
    let transport = parsed.get("transport").ok_or("Missing 'transport' field")?;
    let transport_type = transport.get("type").and_then(|v| v.as_str())
        .ok_or("Missing 'transport.type' field")?;
    
    match transport_type {
        "direct" => {}, // Direct connection, no extra validation needed
        "ssh" => {
            let ssh = transport.get("ssh").ok_or("Missing 'transport.ssh' field")?;
            if ssh.get("host").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'transport.ssh.host' field".to_string());
            }
            if ssh.get("user").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'transport.ssh.user' field".to_string());
            }
            let auth = ssh.get("auth").ok_or("Missing 'transport.ssh.auth' field")?;
            let auth_type = auth.get("type").and_then(|v| v.as_str())
                .ok_or("Missing 'transport.ssh.auth.type' field")?;
            match auth_type {
                "key" => {
                    if auth.get("key_ref").and_then(|v| v.as_str()).is_none() {
                        return Err("Missing 'transport.ssh.auth.key_ref' field".to_string());
                    }
                },
                "password" => {
                    if auth.get("password_ref").and_then(|v| v.as_str()).is_none() {
                        return Err("Missing 'transport.ssh.auth.password_ref' field".to_string());
                    }
                },
                "agent" => {}, // SSH agent, no extra fields needed
                _ => return Err(format!("Invalid SSH auth type: {}", auth_type)),
            }
        },
        _ => return Err(format!("Invalid transport type: {}", transport_type)),
    }
    
    // Validate TLS if present
    if let Some(tls) = parsed.get("tls") {
        let enabled = tls.get("enabled").and_then(|v| v.as_bool())
            .ok_or("Missing or invalid 'tls.enabled' field")?;
        
        if enabled {
            if let Some(sslmode) = tls.get("sslmode").and_then(|v| v.as_str()) {
                match sslmode {
                    "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full" => {},
                    _ => return Err(format!("Invalid sslmode: {}", sslmode)),
                }
            }
        }
    }
    
    Ok(())
}

fn validate_sqlite_config(parsed: &Value) -> Result<(), String> {
    let mode = parsed.get("mode").and_then(|v| v.as_str())
        .ok_or("Missing 'mode' field")?;
    
    match mode {
        "file" => {
            if parsed.get("file").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'file' field for file mode".to_string());
            }
        },
        "memory" => {}, // Memory mode, no file needed
        _ => return Err(format!("Invalid SQLite mode: {}", mode)),
    }
    
    // Validate options if present
    if let Some(options) = parsed.get("options") {
        if let Some(_read_only) = options.get("read_only").and_then(|v| v.as_bool()) {
            // read_only is boolean, valid as-is
        }
        
        if let Some(pragmas) = options.get("pragmas").and_then(|v| v.as_object()) {
            // Pragmas should be key-value pairs, basic validation
            for (key, _value) in pragmas {
                if key.is_empty() {
                    return Err("Empty pragma key".to_string());
                }
                // Accept any JSON value for pragma values
            }
        }
    }
    
    Ok(())
}

fn validate_mysql_config(parsed: &Value) -> Result<(), String> {
    let db = parsed.get("db").ok_or("Missing 'db' field")?;
    if db.get("host").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.host'".into());
    }
    if db.get("database").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.database'".into());
    }
    if db.get("username").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.username'".into());
    }
    Ok(())
}

fn validate_mongodb_config(parsed: &Value) -> Result<(), String> {
    let auth = parsed.get("auth").ok_or("Missing 'auth' field")?;
    let method = auth.get("method").and_then(|v| v.as_str()).ok_or("Missing 'auth.method'")?;
    
    match method {
        "standard" => {
            let db = parsed.get("db").ok_or("Missing 'db' field")?;
            if db.get("host").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'db.host'".into());
            }
        },
        "uri" => {
            let db = parsed.get("db").ok_or("Missing 'db' field")?;
            if db.get("uri").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'db.uri'".into());
            }
        },
        _ => return Err(format!("Invalid MongoDB auth method: {}", method)),
    }
    Ok(())
}

fn validate_redis_config(parsed: &Value) -> Result<(), String> {
    let db = parsed.get("db").ok_or("Missing 'db' field")?;
    if db.get("host").and_then(|v| v.as_str()).is_none() {
        return Err("Missing 'db.host'".into());
    }
    Ok(())
}

fn validate_elasticsearch_config(parsed: &Value) -> Result<(), String> {
    let auth = parsed.get("auth").ok_or("Missing 'auth' field")?;
    let method = auth.get("method").and_then(|v| v.as_str()).ok_or("Missing 'auth.method'")?;
    
    match method {
        "cloud_id" => {
            let db = parsed.get("db").ok_or("Missing 'db' field")?;
            if db.get("cloud_id").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'db.cloud_id'".into());
            }
        },
        "basic" | "api_key" => {
            let db = parsed.get("db").ok_or("Missing 'db' field")?;
            if db.get("host").and_then(|v| v.as_str()).is_none() {
                return Err("Missing 'db.host'".into());
            }
        },
        _ => return Err(format!("Invalid Elasticsearch auth method: {}", method)),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_postgres_config() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "port": 5432,
                "database": "test",
                "username": "postgres"
            },
            "transport": {
                "type": "direct"
            },
            "tls": {
                "enabled": false
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_ok());
    }
    
    #[test]
    fn test_valid_postgres_ssh_config() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "db.example.com",
                "port": 5432,
                "database": "prod",
                "username": "app_user"
            },
            "transport": {
                "type": "ssh",
                "ssh": {
                    "host": "bastion.example.com",
                    "port": 22,
                    "user": "ubuntu",
                    "auth": {
                        "type": "key",
                        "key_ref": "ssh-key-1"
                    }
                }
            },
            "tls": {
                "enabled": true,
                "sslmode": "verify-full",
                "ca_ref": "pg-ca-1"
            },
            "options": {
                "search_path": "public",
                "application_name": "MyApp"
            }
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_ok());
    }
    
    #[test]
    fn test_valid_sqlite_file_config() {
        let config = r#"
        {
            "version": 1,
            "mode": "file",
            "file": "/path/to/database.db",
            "options": {
                "read_only": false,
                "pragmas": {
                    "journal_mode": "WAL",
                    "foreign_keys": true
                }
            }
        }
        "#;
        
        assert!(validate_config_json(config, "sqlite").is_ok());
    }
    
    #[test]
    fn test_valid_sqlite_memory_config() {
        let config = r#"
        {
            "version": 1,
            "mode": "memory"
        }
        "#;
        
        assert!(validate_config_json(config, "sqlite").is_ok());
    }
    
    #[test]
    fn test_invalid_version() {
        let config = r#"
        {
            "version": 2,
            "mode": "memory"
        }
        "#;
        
        assert!(validate_config_json(config, "sqlite").is_err());
    }
    
    #[test]
    fn test_missing_required_fields() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost"
                // Missing database and username
            },
            "transport": {
                "type": "direct"
            },
            "tls": {
                "enabled": false
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_err());
    }

    #[test]
    fn test_valid_mysql_config() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "port": 3306,
                "database": "test",
                "username": "root"
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "mysql").is_ok());
    }

    #[test]
    fn test_valid_mongodb_config() {
        let config = r#"
        {
            "version": 1,
            "auth": {
                "method": "standard"
            },
            "db": {
                "host": "localhost",
                "port": 27017
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "mongodb").is_ok());
    }

    #[test]
    fn test_valid_mongodb_uri_config() {
        let config = r#"
        {
            "version": 1,
            "auth": {
                "method": "uri"
            },
            "db": {
                "uri": "mongodb://localhost:27017"
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "mongodb").is_ok());
    }

    #[test]
    fn test_valid_redis_config() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "port": 6379
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "redis").is_ok());
    }

    #[test]
    fn test_valid_elasticsearch_config() {
        let config = r#"
        {
            "version": 1,
            "auth": {
                "method": "basic"
            },
            "db": {
                "host": "localhost",
                "port": 9200
            },
            "options": {}
        }
        "#;
        
        assert!(validate_config_json(config, "elasticsearch").is_ok());
    }

    #[test]
    fn test_invalid_postgres_transport_type() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "database": "test",
                "username": "postgres"
            },
            "transport": {
                "type": "invalid"
            }
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_err());
    }

    #[test]
    fn test_invalid_postgres_ssh_auth_type() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "database": "test",
                "username": "postgres"
            },
            "transport": {
                "type": "ssh",
                "ssh": {
                    "host": "bastion",
                    "user": "user",
                    "auth": {
                        "type": "invalid"
                    }
                }
            }
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_err());
    }

    #[test]
    fn test_invalid_postgres_sslmode() {
        let config = r#"
        {
            "version": 1,
            "db": {
                "host": "localhost",
                "database": "test",
                "username": "postgres"
            },
            "transport": {
                "type": "direct"
            },
            "tls": {
                "enabled": true,
                "sslmode": "invalid"
            }
        }
        "#;
        
        assert!(validate_config_json(config, "postgres").is_err());
    }
}
