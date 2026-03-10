//! Content-Addressable Storage (CAS) for note content
//!
//! Files are stored by their SHA256 hash, allowing:
//! - Deduplication: identical content stored once
//! - Immutable content: files never modified, only added
//! - Safe undo: old commits reference old hashes that still exist
//! - Git efficiency: only index.toml is tracked, not binary content

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::webdav::WebDavClient;

/// Directory where content is stored by hash
pub const CONTENT_STORE_DIR: &str = ".gradesta-notes/content-store";

/// Compute SHA256 hash of content, returned as hex string
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Get the path for a content file given its hash and extension
pub fn hash_to_path(hash: &str, ext: &str) -> String {
    format!("{}/{}.{}", CONTENT_STORE_DIR, hash, ext)
}

/// Extract hash from a content-store path
/// e.g., ".gradesta-notes/content-store/abc123.png" -> Some("abc123")
#[allow(dead_code)]
pub fn path_to_hash(path: &str) -> Option<&str> {
    if !path.starts_with(CONTENT_STORE_DIR) {
        return None;
    }
    let filename = path.rsplit('/').next()?;
    filename.split('.').next()
}

/// Extract extension from a content-store path
/// e.g., ".gradesta-notes/content-store/abc123.png" -> Some("png")
#[allow(dead_code)]
pub fn path_to_ext(path: &str) -> Option<&str> {
    let filename = path.rsplit('/').next()?;
    filename.rsplit('.').next()
}

/// Content-addressable storage backed by WebDAV
pub struct ContentStore<C: WebDavClient> {
    client: C,
}

impl<C: WebDavClient> ContentStore<C> {
    /// Create a new ContentStore with the given WebDAV client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Store content and return its hash
    ///
    /// If content with the same hash already exists, this is a no-op.
    /// Returns the SHA256 hash of the content.
    pub async fn put(&self, content: &[u8], ext: &str) -> Result<String> {
        let hash = compute_hash(content);
        let path = hash_to_path(&hash, ext);

        // Check if already exists (content-addressable means same hash = same content)
        if self.client.exists(&path).await {
            log::debug!("Content {} already exists, skipping upload", hash);
            return Ok(hash);
        }

        // Ensure content-store directory exists
        self.client.mkdir(CONTENT_STORE_DIR).await?;

        // Upload content
        self.client.upload(&path, content).await
            .with_context(|| format!("Failed to upload content {}", hash))?;

        log::info!("Stored content {} ({} bytes)", hash, content.len());
        Ok(hash)
    }

    /// Retrieve content by hash
    pub async fn get(&self, hash: &str, ext: &str) -> Result<Vec<u8>> {
        let path = hash_to_path(hash, ext);
        self.client.download(&path).await
            .with_context(|| format!("Failed to download content {}", hash))
    }

    /// Check if content exists
    #[allow(dead_code)]
    pub async fn exists(&self, hash: &str, ext: &str) -> bool {
        let path = hash_to_path(hash, ext);
        self.client.exists(&path).await
    }

    /// List all hashes in the content store
    ///
    /// Returns a vector of (hash, extension) tuples.
    #[allow(dead_code)]
    pub async fn list_hashes(&self) -> Result<Vec<(String, String)>> {
        let entries = self.client.list_directory(CONTENT_STORE_DIR).await?;

        let mut hashes = Vec::new();
        for entry in entries {
            if entry.is_directory {
                continue;
            }
            // Parse filename: hash.ext
            if let Some(dot_pos) = entry.name.rfind('.') {
                let hash = entry.name[..dot_pos].to_string();
                let ext = entry.name[dot_pos + 1..].to_string();
                hashes.push((hash, ext));
            }
        }

        Ok(hashes)
    }

    /// Delete content by hash (used by garbage collection)
    ///
    /// WARNING: Only call this after verifying the hash is not referenced
    /// by any commit in git history.
    #[allow(dead_code)]
    pub async fn delete(&self, hash: &str, ext: &str) -> Result<()> {
        let path = hash_to_path(hash, ext);
        self.client.delete(&path).await
            .with_context(|| format!("Failed to delete content {}", hash))
    }
}

/// ContentStore for local filesystem (used with git working directory)
pub struct LocalContentStore {
    base_dir: std::path::PathBuf,
}

impl LocalContentStore {
    /// Create a new LocalContentStore with the given base directory
    ///
    /// The base_dir should be the git working directory (e.g., /tmp/gradesta-undo-xxx)
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
        }
    }

    /// Get the full path for content storage
    fn content_path(&self, hash: &str, ext: &str) -> std::path::PathBuf {
        self.base_dir.join(CONTENT_STORE_DIR).join(format!("{}.{}", hash, ext))
    }

    /// Store content and return its hash
    pub fn put(&self, content: &[u8], ext: &str) -> Result<String> {
        let hash = compute_hash(content);
        let path = self.content_path(&hash, ext);

        // Check if already exists
        if path.exists() {
            log::debug!("Local content {} already exists", hash);
            return Ok(hash);
        }

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write content
        std::fs::write(&path, content)?;
        log::debug!("Stored local content {} ({} bytes)", hash, content.len());

        Ok(hash)
    }

    /// Retrieve content by hash
    #[allow(dead_code)]
    pub fn get(&self, hash: &str, ext: &str) -> Result<Vec<u8>> {
        let path = self.content_path(hash, ext);
        std::fs::read(&path)
            .with_context(|| format!("Failed to read local content {}", hash))
    }

    /// Check if content exists locally
    #[allow(dead_code)]
    pub fn exists(&self, hash: &str, ext: &str) -> bool {
        self.content_path(hash, ext).exists()
    }

    /// Delete content by hash
    #[allow(dead_code)]
    pub fn delete(&self, hash: &str, ext: &str) -> Result<()> {
        let path = self.content_path(hash, ext);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all hashes in the local content store
    #[allow(dead_code)]
    pub fn list_hashes(&self) -> Result<Vec<(String, String)>> {
        let content_dir = self.base_dir.join(CONTENT_STORE_DIR);
        if !content_dir.exists() {
            return Ok(Vec::new());
        }

        let mut hashes = Vec::new();
        for entry in std::fs::read_dir(&content_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let filename = entry.file_name();
                let filename = filename.to_string_lossy();
                if let Some(dot_pos) = filename.rfind('.') {
                    let hash = filename[..dot_pos].to_string();
                    let ext = filename[dot_pos + 1..].to_string();
                    hashes.push((hash, ext));
                }
            }
        }

        Ok(hashes)
    }
}

// Add hex encoding helper since we're using sha2 but need hex output
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut hex = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            hex.push(HEX_CHARS[(b >> 4) as usize] as char);
            hex.push(HEX_CHARS[(b & 0x0f) as usize] as char);
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let content = b"hello world";
        let hash = compute_hash(content);
        // SHA256 of "hello world"
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_hash_to_path() {
        let hash = "abc123";
        let ext = "png";
        let path = hash_to_path(hash, ext);
        assert_eq!(path, ".gradesta-notes/content-store/abc123.png");
    }

    #[test]
    fn test_path_to_hash() {
        let path = ".gradesta-notes/content-store/abc123.png";
        assert_eq!(path_to_hash(path), Some("abc123"));

        let path = ".gradesta-notes/content/old-style.png";
        assert_eq!(path_to_hash(path), None);
    }

    #[test]
    fn test_path_to_ext() {
        let path = ".gradesta-notes/content-store/abc123.png";
        assert_eq!(path_to_ext(path), Some("png"));
    }

    #[test]
    fn test_same_content_same_hash() {
        let content1 = b"test content";
        let content2 = b"test content";
        let hash1 = compute_hash(content1);
        let hash2 = compute_hash(content2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_content_different_hash() {
        let content1 = b"content A";
        let content2 = b"content B";
        let hash1 = compute_hash(content1);
        let hash2 = compute_hash(content2);
        assert_ne!(hash1, hash2);
    }
}
