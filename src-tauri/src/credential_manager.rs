use crate::connection::SecureCredentials;
use keyring::{Entry, Error};

pub struct CredentialManager {
    service_name: String,
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            service_name: "tables_db_manager".to_string(),
        }
    }

    fn build_key(&self, connection_id: &str, credential_type: &str) -> String {
        format!("connections:{}:{}", connection_id, credential_type)
    }

    // Password operations
    pub fn store_password(&self, connection_id: &str, password: &str) -> Result<(), Error> {
        let key = self.build_key(connection_id, "password");
        let entry = Entry::new(&self.service_name, &key)?;
        entry.set_password(password)
    }

    pub fn get_password(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key = self.build_key(connection_id, "password");
        let entry = Entry::new(&self.service_name, &key)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // SSH private key operations
    pub fn store_ssh_private_key(&self, connection_id: &str, key: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "ssh_private_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(key)
    }

    pub fn get_ssh_private_key(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "ssh_private_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // SSH passphrase operations
    pub fn store_ssh_passphrase(&self, connection_id: &str, passphrase: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "ssh_passphrase");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(passphrase)
    }

    pub fn get_ssh_passphrase(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "ssh_passphrase");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(passphrase) => Ok(Some(passphrase)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // SSL certificate operations
    pub fn store_ssl_certificate(&self, connection_id: &str, cert: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "ssl_cert");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(cert)
    }

    pub fn get_ssl_certificate(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "ssl_cert");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(cert) => Ok(Some(cert)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // SSL private key operations
    pub fn store_ssl_private_key(&self, connection_id: &str, key: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "ssl_private_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(key)
    }

    pub fn get_ssl_private_key(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "ssl_private_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // SSL CA certificate operations
    pub fn store_ssl_ca_certificate(&self, connection_id: &str, cert: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "ssl_ca_cert");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(cert)
    }

    pub fn get_ssl_ca_certificate(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "ssl_ca_cert");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(cert) => Ok(Some(cert)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // API token operations
    pub fn store_api_token(&self, connection_id: &str, token: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "api_token");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(token)
    }

    pub fn get_api_token(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "api_token");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // AWS S3 credentials operations
    pub fn store_aws_access_key_id(&self, connection_id: &str, key_id: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "aws_access_key_id");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(key_id)
    }

    pub fn get_aws_access_key_id(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "aws_access_key_id");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(key_id) => Ok(Some(key_id)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn store_aws_secret_access_key(&self, connection_id: &str, secret_key: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "aws_secret_access_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(secret_key)
    }

    pub fn get_aws_secret_access_key(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "aws_secret_access_key");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(secret_key) => Ok(Some(secret_key)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn store_aws_session_token(&self, connection_id: &str, session_token: &str) -> Result<(), Error> {
        let key_name = self.build_key(connection_id, "aws_session_token");
        let entry = Entry::new(&self.service_name, &key_name)?;
        entry.set_password(session_token)
    }

    pub fn get_aws_session_token(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let key_name = self.build_key(connection_id, "aws_session_token");
        let entry = Entry::new(&self.service_name, &key_name)?;
        match entry.get_password() {
            Ok(session_token) => Ok(Some(session_token)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // Store all credentials for a connection
    pub fn store_credentials(&self, connection_id: &str, credentials: &SecureCredentials) -> Result<(), Error> {
        if let Some(password) = &credentials.password {
            self.store_password(connection_id, password.expose())?;
        }

        if let Some(ssh_key) = &credentials.ssh_private_key {
            self.store_ssh_private_key(connection_id, ssh_key.expose())?;
        }

        if let Some(ssh_passphrase) = &credentials.ssh_passphrase {
            self.store_ssh_passphrase(connection_id, ssh_passphrase.expose())?;
        }

        if let Some(ssl_cert) = &credentials.ssl_certificate {
            self.store_ssl_certificate(connection_id, ssl_cert.expose())?;
        }

        if let Some(ssl_key) = &credentials.ssl_private_key {
            self.store_ssl_private_key(connection_id, ssl_key.expose())?;
        }

        if let Some(ssl_ca_cert) = &credentials.ssl_ca_certificate {
            self.store_ssl_ca_certificate(connection_id, ssl_ca_cert.expose())?;
        }

        if let Some(api_token) = &credentials.api_token {
            self.store_api_token(connection_id, api_token.expose())?;
        }

        // AWS S3 credentials
        if let Some(aws_access_key_id) = &credentials.aws_access_key_id {
            self.store_aws_access_key_id(connection_id, aws_access_key_id.expose())?;
        }

        if let Some(aws_secret_access_key) = &credentials.aws_secret_access_key {
            self.store_aws_secret_access_key(connection_id, aws_secret_access_key.expose())?;
        }

        if let Some(aws_session_token) = &credentials.aws_session_token {
            self.store_aws_session_token(connection_id, aws_session_token.expose())?;
        }

        Ok(())
    }

    // Get all credentials for a connection
    pub fn get_credentials(&self, connection_id: &str) -> Result<SecureCredentials, Error> {
        let mut credentials = SecureCredentials::new();

        if let Ok(Some(password)) = self.get_password(connection_id) {
            credentials.password = Some(password.into());
        }

        if let Ok(Some(ssh_key)) = self.get_ssh_private_key(connection_id) {
            credentials.ssh_private_key = Some(ssh_key.into());
        }

        if let Ok(Some(ssh_passphrase)) = self.get_ssh_passphrase(connection_id) {
            credentials.ssh_passphrase = Some(ssh_passphrase.into());
        }

        if let Ok(Some(ssl_cert)) = self.get_ssl_certificate(connection_id) {
            credentials.ssl_certificate = Some(ssl_cert.into());
        }

        if let Ok(Some(ssl_key)) = self.get_ssl_private_key(connection_id) {
            credentials.ssl_private_key = Some(ssl_key.into());
        }

        if let Ok(Some(ssl_ca_cert)) = self.get_ssl_ca_certificate(connection_id) {
            credentials.ssl_ca_certificate = Some(ssl_ca_cert.into());
        }

        if let Ok(Some(api_token)) = self.get_api_token(connection_id) {
            credentials.api_token = Some(api_token.into());
        }

        // AWS S3 credentials
        if let Ok(Some(aws_access_key_id)) = self.get_aws_access_key_id(connection_id) {
            credentials.aws_access_key_id = Some(aws_access_key_id.into());
        }

        if let Ok(Some(aws_secret_access_key)) = self.get_aws_secret_access_key(connection_id) {
            credentials.aws_secret_access_key = Some(aws_secret_access_key.into());
        }

        if let Ok(Some(aws_session_token)) = self.get_aws_session_token(connection_id) {
            credentials.aws_session_token = Some(aws_session_token.into());
        }

        Ok(credentials)
    }

    // Delete specific credential type
    pub fn delete_credential(&self, connection_id: &str, credential_type: &str) -> Result<(), Error> {
        let key = self.build_key(connection_id, credential_type);
        let entry = Entry::new(&self.service_name, &key)?;
        entry.delete_password()
    }

    // Delete all credentials for a connection
    pub fn delete_all_credentials(&self, connection_id: &str) -> Result<(), Error> {
        let credential_types = vec![
            "password",
            "ssh_private_key",
            "ssh_passphrase",
            "ssl_cert",
            "ssl_private_key",
            "ssl_ca_cert",
            "api_token",
            "aws_access_key_id",
            "aws_secret_access_key",
            "aws_session_token",
        ];

        for credential_type in credential_types {
            // Ignore errors for individual deletions - some might not exist
            let _ = self.delete_credential(connection_id, credential_type);
        }

        Ok(())
    }

    // Check if keyring is available
    pub fn is_available(&self) -> bool {
        // Try to store and retrieve a test value
        let test_id = "keyring_test";
        let test_value = "test_value";
        
        match self.store_password(test_id, test_value) {
            Ok(_) => {
                match self.get_password(test_id) {
                    Ok(Some(retrieved)) => {
                        let _ = self.delete_credential(test_id, "password");
                        retrieved == test_value
                    }
                    _ => {
                        let _ = self.delete_credential(test_id, "password");
                        false
                    }
                }
            }
            Err(_) => false,
        }
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}
