use crate::connection::SecureCredentials;
use crate::crypto::{self, MasterKey, MasterKeyManager};
use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection, OptionalExtension};
use log::{debug, error};

pub struct CredentialManager {
    db: Arc<Mutex<Connection>>,
    master_key: MasterKey,
}

impl CredentialManager {
    pub fn new(app_data_dir: &Path, db: Arc<Mutex<Connection>>) -> Result<Self, String> {
        let key_manager = MasterKeyManager::new(app_data_dir);
        let master_key = key_manager.load_or_generate()?;
        
        Ok(Self {
            db,
            master_key,
        })
    }

    // Generic helper to encryption and store a credential
    fn store_credential(&self, connection_id: &str, key: &str, value: &str) -> Result<(), String> {
        debug!("Encrypting and storing credential '{}' for connection {}", key, connection_id);
        
        // Encrypt the value
        let (encrypted_value, nonce) = crypto::encrypt(value.as_bytes(), &self.master_key)?;
        
        let conn = self.db.lock().map_err(|e| format!("Database lock failed: {}", e))?;
        
        conn.execute(
            "INSERT OR REPLACE INTO credentials (connection_id, credential_key, encrypted_value, nonce, encryption_version, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                connection_id,
                key,
                encrypted_value,
                nonce,
                1, // Encryption version
                chrono::Utc::now().timestamp()
            ],
        ).map_err(|e| format!("Failed to store credential: {}", e))?;
        
        Ok(())
    }

    // Generic helper to retrieve and decrypt a credential
    fn get_credential(&self, connection_id: &str, key: &str) -> Result<Option<String>, String> {
        debug!("Retrieving credential '{}' for connection {}", key, connection_id);
        
        let conn = self.db.lock().map_err(|e| format!("Database lock failed: {}", e))?;
        
        let result = conn.query_row(
            "SELECT encrypted_value, nonce FROM credentials WHERE connection_id = ?1 AND credential_key = ?2",
            params![connection_id, key],
            |row| {
                let encrypted_value: Vec<u8> = row.get(0)?;
                let nonce: Vec<u8> = row.get(1)?;
                Ok((encrypted_value, nonce))
            }
        ).optional().map_err(|e| format!("Failed to query credential: {}", e))?;
        
        match result {
            Some((encrypted_value, nonce)) => {
                match crypto::decrypt(&encrypted_value, &nonce, &self.master_key) {
                    Ok(plaintext_bytes) => {
                        let plaintext = String::from_utf8(plaintext_bytes)
                            .map_err(|e| format!("Invalid UTF-8 in decrypted credential: {}", e))?;
                        Ok(Some(plaintext))
                    },
                    Err(e) => {
                        error!("Failed to decrypt credential '{}' for connection {}: {}", key, connection_id, e);
                        // We return None here so the app doesn't crash, but logged the error
                        Ok(None)
                    }
                }
            },
            None => Ok(None),
        }
    }

    // Generic helper to delete a credential
    pub fn delete_credential(&self, connection_id: &str, key: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| format!("Database lock failed: {}", e))?;
        
        conn.execute(
            "DELETE FROM credentials WHERE connection_id = ?1 AND credential_key = ?2",
            params![connection_id, key],
        ).map_err(|e| format!("Failed to delete credential: {}", e))?;
        
        Ok(())
    }

    // Store all credentials for a connection
    pub fn store_credentials(&self, connection_id: &str, credentials: &SecureCredentials) -> Result<(), String> {
        debug!("Storing secure credentials for connection {}", connection_id);
        
        if let Some(password) = &credentials.password {
            self.store_credential(connection_id, "password", password.expose())?;
        }

        if let Some(ssh_key) = &credentials.ssh_private_key {
            self.store_credential(connection_id, "ssh_private_key", ssh_key.expose())?;
        }

        if let Some(ssh_passphrase) = &credentials.ssh_passphrase {
            self.store_credential(connection_id, "ssh_passphrase", ssh_passphrase.expose())?;
        }

        if let Some(ssl_cert) = &credentials.ssl_certificate {
            self.store_credential(connection_id, "ssl_cert", ssl_cert.expose())?;
        }

        if let Some(ssl_key) = &credentials.ssl_private_key {
            self.store_credential(connection_id, "ssl_private_key", ssl_key.expose())?;
        }

        if let Some(ssl_ca_cert) = &credentials.ssl_ca_certificate {
            self.store_credential(connection_id, "ssl_ca_cert", ssl_ca_cert.expose())?;
        }

        if let Some(api_token) = &credentials.api_token {
            self.store_credential(connection_id, "api_token", api_token.expose())?;
        }

        // AWS S3 credentials
        if let Some(aws_access_key_id) = &credentials.aws_access_key_id {
            self.store_credential(connection_id, "aws_access_key_id", aws_access_key_id.expose())?;
        }

        if let Some(aws_secret_access_key) = &credentials.aws_secret_access_key {
            self.store_credential(connection_id, "aws_secret_access_key", aws_secret_access_key.expose())?;
        }

        if let Some(aws_session_token) = &credentials.aws_session_token {
            self.store_credential(connection_id, "aws_session_token", aws_session_token.expose())?;
        }

        Ok(())
    }

    // Get all credentials for a connection
    pub fn get_credentials(&self, connection_id: &str) -> Result<SecureCredentials, String> {
        debug!("Retrieving secure credentials for connection {}", connection_id);
        let mut credentials = SecureCredentials::new();

        if let Some(password) = self.get_credential(connection_id, "password")? {
            credentials.password = Some(password.into());
        }

        if let Some(ssh_key) = self.get_credential(connection_id, "ssh_private_key")? {
            credentials.ssh_private_key = Some(ssh_key.into());
        }

        if let Some(ssh_passphrase) = self.get_credential(connection_id, "ssh_passphrase")? {
            credentials.ssh_passphrase = Some(ssh_passphrase.into());
        }

        if let Some(ssl_cert) = self.get_credential(connection_id, "ssl_cert")? {
            credentials.ssl_certificate = Some(ssl_cert.into());
        }

        if let Some(ssl_key) = self.get_credential(connection_id, "ssl_private_key")? {
            credentials.ssl_private_key = Some(ssl_key.into());
        }

        if let Some(ssl_ca_cert) = self.get_credential(connection_id, "ssl_ca_cert")? {
            credentials.ssl_ca_certificate = Some(ssl_ca_cert.into());
        }

        if let Some(api_token) = self.get_credential(connection_id, "api_token")? {
            credentials.api_token = Some(api_token.into());
        }

        // AWS S3 credentials
        if let Some(aws_access_key_id) = self.get_credential(connection_id, "aws_access_key_id")? {
            credentials.aws_access_key_id = Some(aws_access_key_id.into());
        }

        if let Some(aws_secret_access_key) = self.get_credential(connection_id, "aws_secret_access_key")? {
            credentials.aws_secret_access_key = Some(aws_secret_access_key.into());
        }

        if let Some(aws_session_token) = self.get_credential(connection_id, "aws_session_token")? {
            credentials.aws_session_token = Some(aws_session_token.into());
        }

        Ok(credentials)
    }

    // Delete all credentials for a connection
    pub fn delete_all_credentials(&self, connection_id: &str) -> Result<(), String> {
        let conn = self.db.lock().map_err(|e| format!("Database lock failed: {}", e))?;
        
        conn.execute(
            "DELETE FROM credentials WHERE connection_id = ?1",
            params![connection_id],
        ).map_err(|e| format!("Failed to delete credentials: {}", e))?;
        
        Ok(())
    }

    // Check if any credentials exist for a connection without decrypting them
    pub fn has_credentials(&self, connection_id: &str) -> Result<bool, String> {
        let conn = self.db.lock().map_err(|e| format!("Database lock failed: {}", e))?;
        
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM credentials WHERE connection_id = ?1 LIMIT 1",
            params![connection_id],
            |row| row.get(0),
        ).map_err(|e| format!("Failed to check credentials: {}", e))?;
        
        Ok(count > 0)
    }

    // Check if store is available (always true for DB-backed)
    pub fn is_available(&self) -> bool {
        true
    }
}

