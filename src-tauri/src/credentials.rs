use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use log::{info, debug, warn, error, trace};

/// Secure credential storage interface
pub trait CredentialStore: Send + Sync {
    fn store_secret(&self, key: &str, secret: &str) -> Result<(), String>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, String>;
    fn delete_secret(&self, key: &str) -> Result<bool, String>;
    fn list_keys(&self) -> Result<Vec<String>, String>;
}

/// In-memory credential store (for development/testing)
#[derive(Debug)]
pub struct MemoryCredentialStore {
    secrets: Arc<Mutex<HashMap<String, SecretString>>>,
}

impl MemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            secrets: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn store_secret(&self, key: &str, secret: &str) -> Result<(), String> {
        trace!("Storing secret in memory for key '{}'", key);
        let mut secrets = self.secrets.lock().map_err(|e| format!("Lock error: {}", e))?;
        secrets.insert(key.to_string(), SecretString::new(secret.to_string()));
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        trace!("Retrieving secret from memory for key '{}'", key);
        let secrets = self.secrets.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(secrets.get(key).map(|s| s.expose().to_string()))
    }

    fn delete_secret(&self, key: &str) -> Result<bool, String> {
        trace!("Deleting secret from memory for key '{}'", key);
        let mut secrets = self.secrets.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(secrets.remove(key).is_some())
    }

    fn list_keys(&self) -> Result<Vec<String>, String> {
        trace!("Listing keys from memory store");
        let secrets = self.secrets.lock().map_err(|e| format!("Lock error: {}", e))?;
        Ok(secrets.keys().cloned().collect())
    }
}

/// Keychain-based credential store (for production)
pub struct KeychainCredentialStore {
    service_name: String,
}

impl KeychainCredentialStore {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }
}

impl CredentialStore for KeychainCredentialStore {
    fn store_secret(&self, key: &str, secret: &str) -> Result<(), String> {
        debug!("Storing secret in keychain for key '{}'", key);
        use keyring::Entry;
        
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| {
                error!("Keyring error for key '{}': {}", key, e);
                format!("Keyring error: {}", e)
            })?;
        
        entry.set_password(secret)
            .map_err(|e| {
                error!("Failed to store secret for key '{}': {}", key, e);
                format!("Failed to store secret: {}", e)
            })?;
        
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<Option<String>, String> {
        debug!("Retrieving secret from keychain for key '{}'", key);
        use keyring::Entry;
        
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| {
                error!("Keyring error for key '{}': {}", key, e);
                format!("Keyring error: {}", e)
            })?;
        
        match entry.get_password() {
            Ok(password) => {
                trace!("Retrieved secret for key '{}'", key);
                Ok(Some(password))
            },
            Err(keyring::Error::NoEntry) => {
                trace!("No secret found for key '{}'", key);
                Ok(None)
            },
            Err(e) => {
                error!("Failed to retrieve secret for key '{}': {}", key, e);
                Err(format!("Failed to retrieve secret: {}", e))
            },
        }
    }

    fn delete_secret(&self, key: &str) -> Result<bool, String> {
        debug!("Deleting secret from keychain for key '{}'", key);
        use keyring::Entry;
        
        let entry = Entry::new(&self.service_name, key)
            .map_err(|e| {
                error!("Keyring error for key '{}': {}", key, e);
                format!("Keyring error: {}", e)
            })?;
        
        match entry.delete_credential() {
            Ok(_) => {
                trace!("Deleted secret for key '{}'", key);
                Ok(true)
            },
            Err(keyring::Error::NoEntry) => {
                trace!("No secret to delete for key '{}'", key);
                Ok(false)
            },
            Err(e) => {
                error!("Failed to delete secret for key '{}': {}", key, e);
                Err(format!("Failed to delete secret: {}", e))
            },
        }
    }

    fn list_keys(&self) -> Result<Vec<String>, String> {
        trace!("Listing keys from keychain (not supported)");
        // Keyring doesn't provide an easy way to list keys for a service
        // For now, return empty - in production you might want to maintain
        // a separate index of stored keys
        Ok(Vec::new())
    }
}

/// Default credential store based on platform
pub fn default_credential_store() -> Box<dyn CredentialStore> {
    Box::new(KeychainCredentialStore::new("tables_app"))
}

/// Secret string that zeroizes on drop
#[derive(Clone)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    pub fn new(s: String) -> Self {
        trace!("Creating new secret string");
        Self { inner: s }
    }

    pub fn expose(&self) -> &str {
        warn!("Exposing secret string - ensure secure handling");
        &self.inner
    }

    pub fn into_string(mut self) -> String {
        warn!("Converting secret to plain string - ensure zeroization");
        let result = self.inner.clone();
        self.zeroize();
        result
    }

    /// Zeroize the secret when dropped
    fn zeroize(&mut self) {
        trace!("Zeroizing secret string");
        // Simple zeroization - overwrite the string contents
        unsafe {
            let ptr = self.inner.as_mut_ptr();
            let len = self.inner.len();
            std::ptr::write_bytes(ptr, 0, len);
        }
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretString")
            .field("redacted", &"[REDACTED]")
            .finish()
    }
}

impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

/// Credential manager for handling secrets
pub struct CredentialManager {
    store: Box<dyn CredentialStore>,
}

impl CredentialManager {
    pub fn new(store: Box<dyn CredentialStore>) -> Self {
        Self { store }
    }

    pub fn with_default_store() -> Self {
        Self::new(default_credential_store())
    }

    /// Store a secret with a generated key reference
    pub fn store_secret(&self, secret: &str) -> Result<String, String> {
        debug!("Storing secret with generated key");
        let key = format!("secret-{}", Uuid::new_v4());
        self.store.store_secret(&key, secret)?;
        Ok(key)
    }

    /// Store a secret with a specific key
    pub fn store_secret_with_key(&self, key: &str, secret: &str) -> Result<(), String> {
        debug!("Storing secret with key '{}'", key);
        self.store.store_secret(key, secret)
    }

    /// Retrieve a secret by key reference
    pub fn get_secret(&self, key_ref: &str) -> Result<Option<String>, String> {
        debug!("Retrieving secret for key '{}'", key_ref);
        self.store.get_secret(key_ref)
    }

    /// Delete a secret
    pub fn delete_secret(&self, key_ref: &str) -> Result<bool, String> {
        debug!("Deleting secret for key '{}'", key_ref);
        self.store.delete_secret(key_ref)
    }

    /// List all stored secret keys
    pub fn list_secrets(&self) -> Result<Vec<String>, String> {
        debug!("Listing all stored secret keys");
        self.store.list_keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_credential_store() {
        let store = MemoryCredentialStore::new();
        let manager = CredentialManager::new(Box::new(store));

        // Store a secret
        let key = manager.store_secret("my_secret").unwrap();
        
        // Retrieve it
        let retrieved = manager.get_secret(&key).unwrap();
        assert_eq!(retrieved, Some("my_secret".to_string()));

        // Delete it
        let deleted = manager.delete_secret(&key).unwrap();
        assert!(deleted);

        // Verify it's gone
        let retrieved = manager.get_secret(&key).unwrap();
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_secret_string_zeroize() {
        let secret = SecretString::new("sensitive_data".to_string());
        let exposed = secret.expose();
        assert_eq!(exposed, "sensitive_data");
        
        // When dropped, the secret should be zeroized
        drop(secret);
    }
}
