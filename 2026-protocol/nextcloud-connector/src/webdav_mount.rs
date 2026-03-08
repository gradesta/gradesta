//! WebDAV mount handling via rclone
//!
//! Mounts Nextcloud WebDAV to a local path using rclone for efficient
//! git operations. The mount provides a file:// based git remote that
//! can be used for incremental git push/pull operations instead of
//! manually uploading the entire .git directory.

use anyhow::{anyhow, Context, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// WebDAV mount point managed by rclone
pub struct WebDavMount {
    mount_point: PathBuf,
    mounted: bool,
}

impl WebDavMount {
    /// Mount Nextcloud WebDAV using rclone
    ///
    /// Creates a mount at `/tmp/gradesta-webdav-{hash}/` where hash is derived
    /// from the Nextcloud URL and username. The mount uses rclone's WebDAV
    /// backend with write caching enabled for performance.
    pub fn mount(nc_url: &str, username: &str, password: &str) -> Result<Self> {
        let hash = hash_credentials(nc_url, username);
        let mount_point = PathBuf::from(format!("/tmp/gradesta-webdav-{}", &hash[..8]));

        log::info!("WebDAV mount: mounting {} to {}", nc_url, mount_point.display());

        // Clean up any existing mount point
        if mount_point.exists() {
            // Try to unmount first in case it's stale
            let _ = Command::new("fusermount")
                .args(["-u", mount_point.to_str().unwrap()])
                .status();

            // Give it a moment to unmount
            std::thread::sleep(Duration::from_millis(500));

            // Remove the directory if empty
            let _ = std::fs::remove_dir(&mount_point);
        }

        std::fs::create_dir_all(&mount_point)
            .context("Failed to create mount point directory")?;

        // Build WebDAV URL for rclone
        let webdav_url = format!("{}/remote.php/dav/files/{}/", nc_url.trim_end_matches('/'), username);

        // Use rclone mount with WebDAV backend
        // --daemon runs in background
        // --vfs-cache-mode=writes enables write caching for better performance
        // --dir-cache-time sets directory cache duration
        let status = Command::new("rclone")
            .args([
                "mount",
                ":webdav:",
                mount_point.to_str().unwrap(),
                &format!("--webdav-url={}", webdav_url),
                &format!("--webdav-user={}", username),
                &format!("--webdav-pass={}", obscure_password(password)),
                "--daemon",
                "--vfs-cache-mode=writes",
                "--dir-cache-time=5s",
                "--vfs-write-back=1s",
                "--no-modtime",  // Don't try to preserve modification times
            ])
            .status()
            .context("Failed to execute rclone mount command")?;

        if !status.success() {
            return Err(anyhow!("rclone mount failed with status: {}", status));
        }

        // Wait for mount to be ready by checking if the mount point is a mount
        let mut attempts = 0;
        let max_attempts = 20; // 10 seconds total
        loop {
            std::thread::sleep(Duration::from_millis(500));
            attempts += 1;

            // Check if mount point is actually mounted
            if is_mounted(&mount_point) {
                log::info!("WebDAV mount: mounted successfully after {}ms", attempts * 500);
                break;
            }

            if attempts >= max_attempts {
                // Clean up on failure
                let _ = std::fs::remove_dir(&mount_point);
                return Err(anyhow!("Timeout waiting for rclone mount to become ready"));
            }
        }

        Ok(Self {
            mount_point,
            mounted: true,
        })
    }

    /// Get the mount point path
    pub fn path(&self) -> &Path {
        &self.mount_point
    }

    /// Get the path to the bare git repo within the mount
    ///
    /// Returns `{mount}/Notes/.gradesta-undo.git/`
    pub fn bare_repo_path(&self) -> PathBuf {
        self.mount_point.join("Notes/.gradesta-undo.git")
    }

    /// Check if the mount is currently active
    pub fn is_mounted(&self) -> bool {
        self.mounted && is_mounted(&self.mount_point)
    }
}

impl Drop for WebDavMount {
    fn drop(&mut self) {
        if self.mounted {
            log::info!("WebDAV mount: unmounting {}", self.mount_point.display());

            let status = Command::new("fusermount")
                .args(["-u", self.mount_point.to_str().unwrap()])
                .status();

            match status {
                Ok(s) if s.success() => {
                    log::info!("WebDAV mount: unmounted successfully");
                    // Clean up mount point directory
                    let _ = std::fs::remove_dir(&self.mount_point);
                }
                Ok(s) => {
                    log::warn!("WebDAV mount: fusermount failed with status: {}", s);
                }
                Err(e) => {
                    log::warn!("WebDAV mount: failed to execute fusermount: {}", e);
                }
            }
        }
    }
}

/// Hash credentials to create a unique mount point identifier
fn hash_credentials(url: &str, username: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    username.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Check if a path is a mount point
fn is_mounted(path: &Path) -> bool {
    // Read /proc/mounts and check if our path is listed
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        let path_str = path.to_string_lossy();
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == path_str {
                return true;
            }
        }
    }

    // Fallback: check if the directory exists and has content
    // (mounted directories typically aren't empty)
    path.exists() && path.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false)
}

/// Obscure password for rclone (rclone expects obscured passwords)
///
/// Uses rclone's password obscuration if available, otherwise uses base64
fn obscure_password(password: &str) -> String {
    // Try to use rclone obscure command
    let output = Command::new("rclone")
        .args(["obscure", password])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => {
            // Fallback: just use the password directly (may not work with all rclone versions)
            log::warn!("Could not obscure password with rclone, using plain text");
            password.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_credentials() {
        let hash1 = hash_credentials("https://cloud.example.com", "user1");
        let hash2 = hash_credentials("https://cloud.example.com", "user2");
        let hash3 = hash_credentials("https://other.example.com", "user1");

        // Same inputs should produce same hash
        assert_eq!(hash1, hash_credentials("https://cloud.example.com", "user1"));

        // Different inputs should produce different hashes
        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);

        // Hash should be 16 hex characters (64-bit hash)
        assert_eq!(hash1.len(), 16);
    }

    #[test]
    fn test_bare_repo_path() {
        let mount = WebDavMount {
            mount_point: PathBuf::from("/tmp/gradesta-webdav-12345678"),
            mounted: false, // Don't try to unmount in test
        };

        assert_eq!(
            mount.bare_repo_path(),
            PathBuf::from("/tmp/gradesta-webdav-12345678/Notes/.gradesta-undo.git")
        );
    }
}
