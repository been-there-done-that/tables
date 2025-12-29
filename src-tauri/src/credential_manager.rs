use crate::connection::SecureCredentials;
use crate::crypto::{self, MasterKey, MasterKeyManager};
use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection, OptionalExtension};
use log::{info, debug, warn, error, trace};

pub struct CredentialManager {
    db: Arc<Mutex<Connection>>,
    master_key: MasterKey,
}

impl CredentialManager {
    pub fn new(app_data_dir: &Path, db: Arc<Mutex<Connection>>) -> Result<Self, String> {
        debug!("Initializing credential manager with app data dir {}", app_data_dir.display());
        let key_manager = MasterKeyManager::new(app_data_dir);
        let master_key = key_manager.load_or_generate()?;
        
        debug!("Credential manager initialized successfully");
        Ok(Self {
            db,
            master_key,
        })
    }

    // Generic helper to encryption and store a credential
    fn store_credential(&self, connection_id: &str, key: &str, value: &str) -> Result<(), String> {
        debug!("Encrypting and storing credential '{}' for connection {}", key, connection_id);
        
        // Encrypt the value
        trace!("Encrypting credential value");
        let (encrypted_value, nonce) = crypto::encrypt(value.as_bytes(), &self.master_key)?;
        
        let conn = self.db.lock().map_err(|e| {
            error!("Failed to lock database for storing credential '{}': {}", key, e);
            format!("Database lock failed: {}", e)
        })?;
        
        debug!("Storing encrypted credential in database");
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
        ).map_err(|e| {
            error!("Failed to store credential '{}' in database: {}", key, e);
            format!("Failed to store credential: {}", e)
        })?;
        
        trace!("Credential '{}' stored successfully", key);
        Ok(())
    }

    // Generic helper to retrieve and decrypt a credential
    fn get_credential(&self, connection_id: &str, key: &str) -> Result<Option<String>, String> {
        debug!("Retrieving credential '{}' for connection {}", key, connection_id);
        
        let conn = self.db.lock().map_err(|e| {
            error!("Failed to lock database for retrieving credential '{}': {}", key, e);
            format!("Database lock failed: {}", e)
        })?;
        
        trace!("Querying database for encrypted credential");
        let result = conn.query_row(
            "SELECT encrypted_value, nonce FROM credentials WHERE connection_id = ?1 AND credential_key = ?2",
            params![connection_id, key],
            |row| {
                let encrypted_value: Vec<u8> = row.get(0)?;
                let nonce: Vec<u8> = row.get(1)?;
                Ok((encrypted_value, nonce))
            }
        ).optional().map_err(|e| {
            error!("Failed to query credential '{}' from database: {}", key, e);
            format!("Failed to query credential: {}", e)
        })?;
        
        match result {
            Some((encrypted_value, nonce)) => {
                trace!("Decrypting credential '{}'", key);
                match crypto::decrypt(&encrypted_value, &nonce, &self.master_key) {
                    Ok(plaintext_bytes) => {
                        let plaintext = String::from_utf8(plaintext_bytes)
                            .map_err(|e| {
                                error!("Invalid UTF-8 in decrypted credential '{}': {}", key, e);
                                format!("Invalid UTF-8 in decrypted credential: {}", e)
                            })?;
                        trace!("Credential '{}' retrieved and decrypted successfully", key);
                        Ok(Some(plaintext))
                    },
                    Err(e) => {
                        error!("Failed to decrypt credential '{}' for connection {}: {}", key, connection_id, e);
                        // We return None here so the app doesn't crash, but logged the error
                        Ok(None)
                    }
                }
            },
            None => {
                trace!("Credential '{}' not found for connection {}", key, connection_id);
                Ok(None)
            },
        }
    }

    // Generic helper to delete a credential
    pub fn delete_credential(&self, connection_id: &str, key: &str) -> Result<(), String> {
        debug!("Deleting credential '{}' for connection '{}'", key, connection_id);
        let conn = self.db.lock().map_err(|e| {
            error!("Failed to lock database for deleting credential '{}': {}", key, e);
            format!("Database lock failed: {}", e)
        })?;
        
        conn.execute(
            "DELETE FROM credentials WHERE connection_id = ?1 AND credential_key = ?2",
            params![connection_id, key],
        ).map_err(|e| {
            error!("Failed to delete credential '{}' from database: {}", key, e);
            format!("Failed to delete credential: {}", e)
        })?;
        
        trace!("Credential '{}' deleted successfully", key);
        Ok(())
    }

    // Store all credentials for a connection
    pub fn store_credentials(&self, connection_id: &str, credentials: &SecureCredentials) -> Result<(), String> {
        debug!("Storing secure credentials for connection {}", connection_id);
        
        if let Some(password) = &credentials.password {
            trace!("Storing password credential");
            self.store_credential(connection_id, "password", password.expose())?;
        }

        if let Some(ssh_key) = &credentials.ssh_private_key {
            trace!("Storing SSH private key credential");
            self.store_credential(connection_id, "ssh_private_key", ssh_key.expose())?;
        }

        if let Some(ssh_passphrase) = &credentials.ssh_passphrase {
            trace!("Storing SSH passphrase credential");
            self.store_credential(connection_id, "ssh_passphrase", ssh_passphrase.expose())?;
        }

        if let Some(ssl_cert) = &credentials.ssl_certificate {
            trace!("Storing SSL certificate credential");
            self.store_credential(connection_id, "ssl_cert", ssl_cert.expose())?;
        }

        if let Some(ssl_key) = &credentials.ssl_private_key {
            trace!("Storing SSL private key credential");
            self.store_credential(connection_id, "ssl_private_key", ssl_key.expose())?;
        }

        if let Some(ssl_ca_cert) = &credentials.ssl_ca_certificate {
            trace!("Storing SSL CA certificate credential");
            self.store_credential(connection_id, "ssl_ca_cert", ssl_ca_cert.expose())?;
        }

        if let Some(api_token) = &credentials.api_token {
            trace!("Storing API token credential");
            self.store_credential(connection_id, "api_token", api_token.expose())?;
        }

        // AWS S3 credentials
        if let Some(aws_access_key_id) = &credentials.aws_access_key_id {
            trace!("Storing AWS access key ID credential");
            self.store_credential(connection_id, "aws_access_key_id", aws_access_key_id.expose())?;
        }

        if let Some(aws_secret_access_key) = &credentials.aws_secret_access_key {
            trace!("Storing AWS secret access key credential");
            self.store_credential(connection_id, "aws_secret_access_key", aws_secret_access_key.expose())?;
        }

        if let Some(aws_session_token) = &credentials.aws_session_token {
            trace!("Storing AWS session token credential");
            self.store_credential(connection_id, "aws_session_token", aws_session_token.expose())?;
        }

        debug!("Secure credentials stored successfully for connection {}", connection_id);
        Ok(())
    }

    // Get all credentials for a connection
    pub fn get_credentials(&self, connection_id: &str) -> Result<SecureCredentials, String> {
        debug!("Retrieving secure credentials for connection {}", connection_id);
        let mut credentials = SecureCredentials::new();

        if let Some(password) = self.get_credential(connection_id, "password")? {
            trace!("Retrieved password credential");
            credentials.password = Some(password.into());
        }

        if let Some(ssh_key) = self.get_credential(connection_id, "ssh_private_key")? {
            trace!("Retrieved SSH private key credential");
            credentials.ssh_private_key = Some(ssh_key.into());
        }

        if let Some(ssh_passphrase) = self.get_credential(connection_id, "ssh_passphrase")? {
            trace!("Retrieved SSH passphrase credential");
            credentials.ssh_passphrase = Some(ssh_passphrase.into());
        }

        if let Some(ssl_cert) = self.get_credential(connection_id, "ssl_cert")? {
            trace!("Retrieved SSL certificate credential");
            credentials.ssl_certificate = Some(ssl_cert.into());
        }

        if let Some(ssl_key) = self.get_credential(connection_id, "ssl_private_key")? {
            trace!("Retrieved SSL private key credential");
            credentials.ssl_private_key = Some(ssl_key.into());
        }

        if let Some(ssl_ca_cert) = self.get_credential(connection_id, "ssl_ca_cert")? {
            trace!("Retrieved SSL CA certificate credential");
            credentials.ssl_ca_certificate = Some(ssl_ca_cert.into());
        }

        if let Some(api_token) = self.get_credential(connection_id, "api_token")? {
            trace!("Retrieved API token credential");
            credentials.api_token = Some(api_token.into());
        }

        // AWS S3 credentials
        if let Some(aws_access_key_id) = self.get_credential(connection_id, "aws_access_key_id")? {
            trace!("Retrieved AWS access key ID credential");
            credentials.aws_access_key_id = Some(aws_access_key_id.into());
        }

        if let Some(aws_secret_access_key) = self.get_credential(connection_id, "aws_secret_access_key")? {
            trace!("Retrieved AWS secret access key credential");
            credentials.aws_secret_access_key = Some(aws_secret_access_key.into());
        }

        if let Some(aws_session_token) = self.get_credential(connection_id, "aws_session_token")? {
            trace!("Retrieved AWS session token credential");
            credentials.aws_session_token = Some(aws_session_token.into());
        }

        debug!("Secure credentials retrieved successfully for connection {}", connection_id);
        Ok(credentials)
    }

    // Delete all credentials for a connection
    pub fn delete_all_credentials(&self, connection_id: &str) -> Result<(), String> {
        debug!("Deleting all credentials for connection '{}'", connection_id);
        let conn = self.db.lock().map_err(|e| {
            error!("Failed to lock database for deleting all credentials for connection '{}': {}", connection_id, e);
            format!("Database lock failed: {}", e)
        })?;
        
        conn.execute(
            "DELETE FROM credentials WHERE connection_id = ?1",
            params![connection_id],
        ).map_err(|e| {
            error!("Failed to delete all credentials for connection '{}' from database: {}", connection_id, e);
            format!("Failed to delete credentials: {}", e)
        })?;
        
        debug!("All credentials deleted successfully for connection '{}'", connection_id);
        Ok(())
    }

    // Check if any credentials exist for a connection without decrypting them
    pub fn has_credentials(&self, connection_id: &str) -> Result<bool, String> {
        trace!("Checking if credentials exist for connection '{}'", connection_id);
        let conn = self.db.lock().map_err(|e| {
            error!("Failed to lock database for checking credentials existence for connection '{}': {}", connection_id, e);
            format!("Database lock failed: {}", e)
        })?;
        
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM credentials WHERE connection_id = ?1 LIMIT 1",
            params![connection_id],
            |row| row.get(0),
        ).map_err(|e| {
            error!("Failed to check credentials existence for connection '{}' in database: {}", connection_id, e);
            format!("Failed to check credentials: {}", e)
        })?;
        
        let has_creds = count > 0;
        trace!("Credentials existence check for connection '{}' returned {}", connection_id, has_creds);
        Ok(has_creds)
    }

    // Check if store is available (always true for DB-backed)
    pub fn is_available(&self) -> bool {
        trace!("Checking credential store availability: true");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use rand::{RngCore, rngs::OsRng};

    const CREATE_CREDENTIALS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS credentials (
        connection_id TEXT NOT NULL,
        credential_key TEXT NOT NULL,
        encrypted_value BLOB NOT NULL,
        nonce BLOB NOT NULL,
        encryption_version INTEGER NOT NULL DEFAULT 1,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (connection_id, credential_key)
    );
    "#;

    fn create_test_manager() -> CredentialManager {
        let temp_dir = std::env::temp_dir().join(format!("test_tables_{}", OsRng.next_u64()));
        fs::create_dir_all(&temp_dir).unwrap();
        let db = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        db.lock().unwrap().execute(CREATE_CREDENTIALS_TABLE, []).unwrap();
        CredentialManager::new(&temp_dir, db).unwrap()
    }

    #[test]
    fn test_store_and_get_credential() {
        let manager = create_test_manager();
        manager.store_credential("conn1", "key1", "value1").unwrap();
        let result = manager.get_credential("conn1", "key1").unwrap();
        assert_eq!(result, Some("value1".to_string()));
    }

    #[test]
    fn test_get_nonexistent_credential() {
        let manager = create_test_manager();
        let result = manager.get_credential("conn1", "key1").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_delete_credential() {
        let manager = create_test_manager();
        manager.store_credential("conn1", "key1", "value1").unwrap();
        assert_eq!(manager.get_credential("conn1", "key1").unwrap(), Some("value1".to_string()));
        manager.delete_credential("conn1", "key1").unwrap();
        assert_eq!(manager.get_credential("conn1", "key1").unwrap(), None);
    }

    #[test]
    fn test_store_credentials() {
        let manager = create_test_manager();
        let mut creds = SecureCredentials::new();
        creds.password = Some("pass123".into());
        creds.api_token = Some("token456".into());
        manager.store_credentials("conn1", &creds).unwrap();
        let retrieved = manager.get_credentials("conn1").unwrap();
        assert_eq!(retrieved.password.as_ref().map(|s| s.expose()), Some("pass123"));
        assert_eq!(retrieved.api_token.as_ref().map(|s| s.expose()), Some("token456"));
        assert!(retrieved.ssh_private_key.is_none());
    }

    #[test]
    fn test_get_credentials_empty() {
        let manager = create_test_manager();
        let creds = manager.get_credentials("conn1").unwrap();
        assert!(creds.is_empty());
    }

    #[test]
    fn test_delete_all_credentials() {
        let manager = create_test_manager();
        let mut creds = SecureCredentials::new();
        creds.password = Some("pass123".into());
        creds.api_token = Some("token456".into());
        manager.store_credentials("conn1", &creds).unwrap();
        assert!(manager.has_credentials("conn1").unwrap());
        manager.delete_all_credentials("conn1").unwrap();
        assert!(!manager.has_credentials("conn1").unwrap());
        let retrieved = manager.get_credentials("conn1").unwrap();
        assert!(retrieved.is_empty());
    }

    #[test]
    fn test_has_credentials() {
        let manager = create_test_manager();
        assert!(!manager.has_credentials("conn1").unwrap());
        manager.store_credential("conn1", "key1", "value1").unwrap();
        assert!(manager.has_credentials("conn1").unwrap());
    }

    #[test]
    fn test_is_available() {
        let manager = create_test_manager();
        assert!(manager.is_available());
    }

    #[test]
    fn test_overwrite_credential() {
        let manager = create_test_manager();
        manager.store_credential("conn1", "key1", "value1").unwrap();
        assert_eq!(manager.get_credential("conn1", "key1").unwrap(), Some("value1".to_string()));
        manager.store_credential("conn1", "key1", "value2").unwrap();
        assert_eq!(manager.get_credential("conn1", "key1").unwrap(), Some("value2".to_string()));
    }
}
