//! Content cache with LRU eviction
//!
//! Caches downloaded content (audio, images) in /data/content-cache with
//! a configurable size limit. Uses LRU eviction when the cache is full.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Default cache size limit: 100 MB
const DEFAULT_CACHE_LIMIT_BYTES: u64 = 100 * 1024 * 1024;

/// Cache directory base path
const CONTENT_CACHE_DIR: &str = "/data/content-cache";

/// Content cache with LRU eviction
pub struct ContentCache {
    /// Base directory for this user's cache
    cache_dir: PathBuf,
    /// Maximum cache size in bytes
    max_size: u64,
}

impl ContentCache {
    /// Create a new content cache
    ///
    /// With CAS, content is identified by hash, so we use a single global cache
    /// directory. The url/username parameters are ignored (kept for API compat).
    pub fn new(_url: &str, _username: &str) -> Option<Self> {
        // Only use cache if /data exists (Docker persistent volume)
        if !Path::new("/data").exists() {
            return None;
        }

        // Use flat cache directory - CAS hashes already uniquely identify content
        let cache_dir = PathBuf::from(CONTENT_CACHE_DIR);

        // Create cache directory
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            log::warn!("Failed to create content cache directory: {}", e);
            return None;
        }

        Some(Self {
            cache_dir,
            max_size: DEFAULT_CACHE_LIMIT_BYTES,
        })
    }

    /// Get the cache path for a content hash
    fn cache_path(&self, hash: &str, ext: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.{}", hash, ext))
    }

    /// Try to get content from cache
    pub fn get(&self, hash: &str, ext: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(hash, ext);

        if path.exists() {
            // Update access time by touching the file
            let _ = filetime::set_file_atime(&path, filetime::FileTime::now());

            match std::fs::read(&path) {
                Ok(data) => {
                    log::debug!("Content cache hit: {}.{}", hash, ext);
                    return Some(data);
                }
                Err(e) => {
                    log::warn!("Failed to read cached content: {}", e);
                }
            }
        }

        None
    }

    /// Store content in cache, evicting old entries if needed
    pub fn put(&self, hash: &str, ext: &str, content: &[u8]) -> Result<()> {
        let path = self.cache_path(hash, ext);

        // Check if we need to evict before adding
        let content_size = content.len() as u64;
        self.ensure_space(content_size)?;

        // Write the content
        std::fs::write(&path, content)?;
        log::debug!("Cached content: {}.{} ({} bytes)", hash, ext, content_size);

        Ok(())
    }

    /// Ensure there's enough space for new content, evicting LRU entries if needed
    fn ensure_space(&self, needed: u64) -> Result<()> {
        let mut entries = self.list_entries()?;

        // Calculate current cache size
        let current_size: u64 = entries.iter().map(|e| e.size).sum();

        if current_size + needed <= self.max_size {
            return Ok(()); // Enough space
        }

        // Sort by access time (oldest first)
        entries.sort_by_key(|e| e.accessed);

        // Evict until we have enough space
        let mut freed = 0u64;
        let need_to_free = (current_size + needed).saturating_sub(self.max_size);

        for entry in entries {
            if freed >= need_to_free {
                break;
            }

            log::info!("Evicting from cache: {} ({} bytes)", entry.path.display(), entry.size);
            if std::fs::remove_file(&entry.path).is_ok() {
                freed += entry.size;
            }
        }

        Ok(())
    }

    /// List all cache entries with metadata
    fn list_entries(&self) -> Result<Vec<CacheEntry>> {
        let mut entries = Vec::new();

        if let Ok(dir) = std::fs::read_dir(&self.cache_dir) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        let accessed = meta.accessed().unwrap_or(SystemTime::UNIX_EPOCH);
                        entries.push(CacheEntry {
                            path,
                            size: meta.len(),
                            accessed,
                        });
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Get current cache size and entry count
    pub fn stats(&self) -> (u64, usize) {
        match self.list_entries() {
            Ok(entries) => {
                let size: u64 = entries.iter().map(|e| e.size).sum();
                (size, entries.len())
            }
            Err(_) => (0, 0),
        }
    }

    /// Clear the entire cache
    pub fn clear(&self) -> Result<()> {
        if let Ok(entries) = self.list_entries() {
            for entry in entries {
                let _ = std::fs::remove_file(&entry.path);
            }
        }
        Ok(())
    }
}

/// A cache entry with metadata
struct CacheEntry {
    path: PathBuf,
    size: u64,
    accessed: SystemTime,
}

/// Global content cache accessor
pub fn get_content_cache(url: &str, username: &str) -> Option<ContentCache> {
    ContentCache::new(url, username)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_eviction() {
        // This test would need to mock /data
        // For now, just verify the logic compiles
    }
}
