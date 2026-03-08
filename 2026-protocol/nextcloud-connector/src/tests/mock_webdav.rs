//! Mock WebDAV implementation for testing
//!
//! Provides an in-memory WebDAV mock that can simulate failures and
//! track all operations for assertions.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::nextcloud::FileInfo;
use crate::webdav::WebDavClient;

/// Record of a WebDAV operation for verification
#[derive(Debug, Clone, PartialEq)]
pub enum WebDavOperation {
    Upload { path: String, content: Vec<u8> },
    Download { path: String },
    Delete { path: String },
    Exists { path: String },
    Mkdir { path: String },
    ListDirectory { path: String },
}

/// Mock WebDAV client for testing
///
/// This mock stores files in memory and can be configured to fail
/// specific operations for testing error handling and rollback behavior.
pub struct MockWebDav {
    /// In-memory file storage: path -> content
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Directories that exist
    directories: Arc<Mutex<HashSet<String>>>,
    /// Paths that should fail on upload
    fail_uploads: Arc<Mutex<HashSet<String>>>,
    /// Paths that should fail on delete
    fail_deletes: Arc<Mutex<HashSet<String>>>,
    /// Paths that should fail on download
    fail_downloads: Arc<Mutex<HashSet<String>>>,
    /// Record of all operations for verification
    operations: Arc<Mutex<Vec<WebDavOperation>>>,
}

impl MockWebDav {
    /// Create a new mock WebDAV client
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            directories: Arc::new(Mutex::new(HashSet::new())),
            fail_uploads: Arc::new(Mutex::new(HashSet::new())),
            fail_deletes: Arc::new(Mutex::new(HashSet::new())),
            fail_downloads: Arc::new(Mutex::new(HashSet::new())),
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Pre-populate a file in the mock storage
    pub async fn set_file(&self, path: &str, content: Vec<u8>) {
        let mut files = self.files.lock().await;
        files.insert(path.to_string(), content);

        // Also create parent directories
        if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
            if !parent.is_empty() {
                let mut dirs = self.directories.lock().await;
                dirs.insert(parent.to_string());
            }
        }
    }

    /// Get a file from the mock storage (for assertions)
    pub async fn get_file(&self, path: &str) -> Option<Vec<u8>> {
        let files = self.files.lock().await;
        files.get(path).cloned()
    }

    /// Configure a path to fail on upload
    pub async fn fail_upload(&self, path: &str) {
        let mut fail_uploads = self.fail_uploads.lock().await;
        fail_uploads.insert(path.to_string());
    }

    /// Configure a path to fail on delete
    pub async fn fail_delete(&self, path: &str) {
        let mut fail_deletes = self.fail_deletes.lock().await;
        fail_deletes.insert(path.to_string());
    }

    /// Configure a path to fail on download
    pub async fn fail_download(&self, path: &str) {
        let mut fail_downloads = self.fail_downloads.lock().await;
        fail_downloads.insert(path.to_string());
    }

    /// Clear failure configuration
    pub async fn clear_failures(&self) {
        self.fail_uploads.lock().await.clear();
        self.fail_deletes.lock().await.clear();
        self.fail_downloads.lock().await.clear();
    }

    /// Get all recorded operations
    pub async fn get_operations(&self) -> Vec<WebDavOperation> {
        self.operations.lock().await.clone()
    }

    /// Clear recorded operations
    pub async fn clear_operations(&self) {
        self.operations.lock().await.clear();
    }

    /// Check if a specific operation was recorded
    pub async fn has_operation(&self, op: &WebDavOperation) -> bool {
        let ops = self.operations.lock().await;
        ops.contains(op)
    }

    /// Get all files currently in storage
    pub async fn all_files(&self) -> HashMap<String, Vec<u8>> {
        self.files.lock().await.clone()
    }
}

impl Default for MockWebDav {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockWebDav {
    fn clone(&self) -> Self {
        Self {
            files: Arc::clone(&self.files),
            directories: Arc::clone(&self.directories),
            fail_uploads: Arc::clone(&self.fail_uploads),
            fail_deletes: Arc::clone(&self.fail_deletes),
            fail_downloads: Arc::clone(&self.fail_downloads),
            operations: Arc::clone(&self.operations),
        }
    }
}

#[async_trait]
impl WebDavClient for MockWebDav {
    async fn upload(&self, path: &str, content: &[u8]) -> Result<()> {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::Upload {
                path: path.to_string(),
                content: content.to_vec(),
            });
        }

        // Check if this should fail
        {
            let fail_uploads = self.fail_uploads.lock().await;
            if fail_uploads.contains(path) {
                return Err(anyhow!("Mock upload failure for path: {}", path));
            }
        }

        // Store the file
        {
            let mut files = self.files.lock().await;
            files.insert(path.to_string(), content.to_vec());
        }

        // Create parent directory
        if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
            if !parent.is_empty() {
                let mut dirs = self.directories.lock().await;
                dirs.insert(parent.to_string());
            }
        }

        Ok(())
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>> {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::Download {
                path: path.to_string(),
            });
        }

        // Check if this should fail
        {
            let fail_downloads = self.fail_downloads.lock().await;
            if fail_downloads.contains(path) {
                return Err(anyhow!("Mock download failure for path: {}", path));
            }
        }

        // Return the file content
        let files = self.files.lock().await;
        files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("File not found: {}", path))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::Delete {
                path: path.to_string(),
            });
        }

        // Check if this should fail
        {
            let fail_deletes = self.fail_deletes.lock().await;
            if fail_deletes.contains(path) {
                return Err(anyhow!("Mock delete failure for path: {}", path));
            }
        }

        // Remove the file
        {
            let mut files = self.files.lock().await;
            files.remove(path);
        }

        Ok(())
    }

    async fn exists(&self, path: &str) -> bool {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::Exists {
                path: path.to_string(),
            });
        }

        let files = self.files.lock().await;
        let dirs = self.directories.lock().await;
        files.contains_key(path) || dirs.contains(path)
    }

    async fn mkdir(&self, path: &str) -> Result<()> {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::Mkdir {
                path: path.to_string(),
            });
        }

        // Create the directory
        {
            let mut dirs = self.directories.lock().await;
            dirs.insert(path.to_string());
        }

        Ok(())
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>> {
        // Record the operation
        {
            let mut ops = self.operations.lock().await;
            ops.push(WebDavOperation::ListDirectory {
                path: path.to_string(),
            });
        }

        let files = self.files.lock().await;
        let dirs = self.directories.lock().await;
        let path_normalized = path.trim_end_matches('/');
        let path_prefix = format!("{}/", path_normalized);

        let mut results = Vec::new();

        // Find all files in this directory
        for (file_path, content) in files.iter() {
            if file_path.starts_with(&path_prefix) {
                let relative = &file_path[path_prefix.len()..];
                // Only include direct children (no further slashes)
                if !relative.is_empty() && !relative.contains('/') {
                    let name = relative.to_string();
                    results.push(FileInfo {
                        name: name.clone(),
                        path: file_path.clone(),
                        is_directory: false,
                        size: content.len() as u64,
                        content_type: Some("application/octet-stream".to_string()),
                    });
                }
            }
        }

        // Find all subdirectories that are direct children
        for dir_path in dirs.iter() {
            let dir_normalized = dir_path.trim_end_matches('/');
            if dir_normalized.starts_with(&path_prefix) && dir_normalized != path_normalized {
                let relative = &dir_normalized[path_prefix.len()..];
                // Only include direct children (no further slashes in relative part)
                if !relative.is_empty() && !relative.contains('/') {
                    results.push(FileInfo {
                        name: relative.to_string(),
                        path: dir_path.clone(),
                        is_directory: true,
                        size: 0,
                        content_type: None,
                    });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_upload_download() {
        let mock = MockWebDav::new();

        // Upload a file
        mock.upload("test/file.txt", b"hello world").await.unwrap();

        // Download it back
        let content = mock.download("test/file.txt").await.unwrap();
        assert_eq!(content, b"hello world");

        // Check operations were recorded
        let ops = mock.get_operations().await;
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], WebDavOperation::Upload { path, .. } if path == "test/file.txt"));
        assert!(matches!(&ops[1], WebDavOperation::Download { path } if path == "test/file.txt"));
    }

    #[tokio::test]
    async fn test_mock_fail_upload() {
        let mock = MockWebDav::new();

        // Configure failure
        mock.fail_upload("test/fail.txt").await;

        // Try to upload
        let result = mock.upload("test/fail.txt", b"content").await;
        assert!(result.is_err());

        // Verify file was not stored
        assert!(mock.get_file("test/fail.txt").await.is_none());
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let mock = MockWebDav::new();

        // Create a file
        mock.set_file("test/delete-me.txt", b"content".to_vec()).await;

        // Delete it
        mock.delete("test/delete-me.txt").await.unwrap();

        // Verify it's gone
        assert!(mock.get_file("test/delete-me.txt").await.is_none());
    }

    #[tokio::test]
    async fn test_mock_exists() {
        let mock = MockWebDav::new();

        // File doesn't exist yet
        assert!(!mock.exists("test/file.txt").await);

        // Create it
        mock.set_file("test/file.txt", b"content".to_vec()).await;

        // Now it exists
        assert!(mock.exists("test/file.txt").await);
    }

    #[tokio::test]
    async fn test_mock_list_directory() {
        let mock = MockWebDav::new();

        // Create some files
        mock.set_file("test/a.txt", b"a".to_vec()).await;
        mock.set_file("test/b.txt", b"b".to_vec()).await;
        mock.set_file("test/subdir/c.txt", b"c".to_vec()).await;

        // List the test directory
        let entries = mock.list_directory("test").await.unwrap();

        // Should have a.txt, b.txt, and subdir (but not subdir/c.txt directly)
        assert_eq!(entries.len(), 3, "Expected 3 entries: {:?}", entries.iter().map(|e| &e.name).collect::<Vec<_>>());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        assert!(names.contains(&"subdir"));

        // Verify subdir is marked as directory
        let subdir_entry = entries.iter().find(|e| e.name == "subdir").unwrap();
        assert!(subdir_entry.is_directory);
    }
}
