use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use rand::{RngCore, rngs::OsRng};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng as AeadOsRng, AeadCore},
    Aes256Gcm, Nonce, Key // Or GenericArray
};
use aes_gcm::aead::generic_array::GenericArray;
use zeroize::{Zeroize, ZeroizeOnDrop};
use log::{info, warn, error, debug};
use std::os::unix::fs::PermissionsExt;

const KEY_SIZE: usize = 32;

/// A wrapper around the master key that ensures it is zeroized on drop.
pub struct MasterKey {
    key: [u8; KEY_SIZE],
}

impl MasterKey {
    pub fn new(key: [u8; KEY_SIZE]) -> Self {
        Self { key }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl Zeroize for MasterKey {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

impl ZeroizeOnDrop for MasterKey {}

pub struct MasterKeyManager {
    key_path: PathBuf,
}

impl MasterKeyManager {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            key_path: app_data_dir.join("tables.key"),
        }
    }

    pub fn load_or_generate(&self) -> Result<MasterKey, String> {
        if self.key_path.exists() {
            self.load()
        } else {
            self.generate()
        }
    }

    fn load(&self) -> Result<MasterKey, String> {
        debug!("Loading master key from {:?}", self.key_path);
        
        // Verify permissions on Unix
        #[cfg(unix)]
        {
            let metadata = fs::metadata(&self.key_path)
                .map_err(|e| format!("Failed to read key file metadata: {}", e))?;
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // Check if user has read/write, and no one else has access (0o600)
            if mode & 0o077 != 0 {
                warn!("Insecure permissions on master key file: {:o}. Attempting to fix.", mode);
                let mut perms = permissions;
                perms.set_mode(0o600);
                if let Err(e) = fs::set_permissions(&self.key_path, perms) {
                    error!("Failed to secure master key file permissions: {}", e);
                    // Decide if we should fail hard or proceed. For now, warn.
                }
            }
        }

        let hex_string = fs::read_to_string(&self.key_path)
            .map_err(|e| format!("Failed to read master key file: {}", e))?;
        
        let key_bytes = hex::decode(hex_string.trim())
            .map_err(|e| format!("Failed to decode master key: {}", e))?;

        if key_bytes.len() != KEY_SIZE {
            return Err(format!("Invalid master key size: expected {} bytes, got {}", KEY_SIZE, key_bytes.len()));
        }

        let mut key_array = [0u8; KEY_SIZE];
        key_array.copy_from_slice(&key_bytes);
        
        Ok(MasterKey::new(key_array))
    }

    fn generate(&self) -> Result<MasterKey, String> {
        debug!("Generating new master key at {:?}", self.key_path);
        
        let mut key_bytes = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key_bytes);
        
        let hex_string = hex::encode(key_bytes);
        
        // Write file with restricted permissions
        let mut file = fs::File::create(&self.key_path)
            .map_err(|e| format!("Failed to create master key file: {}", e))?;
            
        #[cfg(unix)]
        {
            let mut perms = file.metadata()
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o600); // Read/write for owner only
            file.set_permissions(perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }

        file.write_all(hex_string.as_bytes())
            .map_err(|e| format!("Failed to write master key: {}", e))?;

        Ok(MasterKey::new(key_bytes))
    }
}

pub fn encrypt(data: &[u8], key: &MasterKey) -> Result<(Vec<u8>, Vec<u8>), String> {
    let key_array = GenericArray::from_slice(key.as_bytes());
    let cipher = Aes256Gcm::new(key_array);
    
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits, unique per encryption
    
    let ciphertext = cipher.encrypt(&nonce, data)
        .map_err(|e| format!("Encryption failed: {}", e))?;
        
    Ok((ciphertext, nonce.to_vec()))
}

pub fn decrypt(ciphertext: &[u8], nonce_bytes: &[u8], key: &MasterKey) -> Result<Vec<u8>, String> {
    let key_array = GenericArray::from_slice(key.as_bytes());
    let cipher = Aes256Gcm::new(key_array);
    
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;
        
    Ok(plaintext)
}
