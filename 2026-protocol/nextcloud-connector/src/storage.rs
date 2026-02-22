//! Credential storage with AES-256-GCM encryption

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Stored credentials for a Nextcloud account
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credential {
    pub nextcloud_url: String,
    pub username: String,
    pub app_password: String,
}

/// Credential store
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CredentialStore {
    pub credentials: HashMap<String, Credential>,
    #[serde(skip)]
    config_dir: PathBuf,
}

impl CredentialStore {
    /// Get config directory path
    /// Uses /data if available (Docker volume mount), otherwise falls back to user config dir
    fn get_config_dir() -> Result<PathBuf> {
        // Check for Docker volume mount first
        let data_dir = PathBuf::from("/data");
        if data_dir.exists() {
            return Ok(data_dir);
        }

        // Fall back to user config directory
        let home = dirs::config_dir().ok_or_else(|| anyhow!("No config directory"))?;
        Ok(home.join("gradesta").join("nextcloud-connector"))
    }

    /// Load credentials from disk (decrypted)
    pub fn load() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        let cred_path = config_dir.join("credentials.enc");

        if !cred_path.exists() {
            return Ok(Self {
                credentials: HashMap::new(),
                config_dir,
            });
        }

        // Read encrypted file
        let enc_data = fs::read(&cred_path).context("Failed to read credentials")?;

        // Load encryption key
        let key = load_or_create_key(&config_dir)?;

        // Decrypt
        let data = decrypt(&enc_data, &key)?;

        // Parse JSON
        let mut store: Self = serde_json::from_slice(&data).context("Failed to parse credentials")?;
        store.config_dir = config_dir;

        Ok(store)
    }

    /// Save credentials to disk (encrypted)
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(&self.config_dir).context("Failed to create config dir")?;

        // Load encryption key
        let key = load_or_create_key(&self.config_dir)?;

        // Serialize to JSON
        let data = serde_json::to_vec_pretty(self)?;

        // Encrypt
        let enc_data = encrypt(&data, &key)?;

        // Write file
        let cred_path = self.config_dir.join("credentials.enc");
        fs::write(&cred_path, enc_data).context("Failed to write credentials")?;

        // Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&cred_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Get credentials for an identity
    pub fn get(&self, identity: &str) -> Option<&Credential> {
        self.credentials.get(identity)
    }

    /// Set credentials for an identity
    pub fn set(&mut self, identity: &str, cred: Credential) {
        self.credentials.insert(identity.to_string(), cred);
    }
}

/// Load or create encryption key
fn load_or_create_key(config_dir: &PathBuf) -> Result<[u8; 32]> {
    let key_path = config_dir.join("secret.key");

    if let Ok(data) = fs::read(&key_path) {
        if data.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data);
            return Ok(key);
        }
    }

    // Generate new key
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

    fs::create_dir_all(config_dir)?;
    fs::write(&key_path, &key)?;

    // Set permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(key)
}

/// Encrypt data using AES-256-GCM
fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    // Derive key using SHA-256
    let hash = Sha256::digest(key);
    let cipher = Aes256Gcm::new_from_slice(&hash).context("Failed to create cipher")?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| anyhow!("Encryption failed"))?;

    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(12 + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// Decrypt data using AES-256-GCM
fn decrypt(ciphertext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if ciphertext.len() < 12 {
        return Err(anyhow!("Ciphertext too short"));
    }

    // Derive key using SHA-256
    let hash = Sha256::digest(key);
    let cipher = Aes256Gcm::new_from_slice(&hash).context("Failed to create cipher")?;

    // Extract nonce and ciphertext
    let nonce = Nonce::from_slice(&ciphertext[..12]);
    let encrypted = &ciphertext[12..];

    // Decrypt
    cipher
        .decrypt(nonce, encrypted)
        .map_err(|_| anyhow!("Decryption failed"))
}
