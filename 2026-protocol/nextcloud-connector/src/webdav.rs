//! WebDAV client trait for mocking in tests
//!
//! This module defines a trait abstraction over WebDAV operations,
//! allowing the NextcloudClient to be mocked for testing.

use anyhow::Result;
use async_trait::async_trait;

use crate::nextcloud::FileInfo;

/// Trait for WebDAV operations
///
/// This trait abstracts the WebDAV operations used by the nextcloud-connector,
/// enabling mock implementations for testing without actual network calls.
#[async_trait]
pub trait WebDavClient: Send + Sync {
    /// Upload content to a path
    async fn upload(&self, path: &str, content: &[u8]) -> Result<()>;

    /// Download content from a path
    async fn download(&self, path: &str) -> Result<Vec<u8>>;

    /// Delete a file or directory
    async fn delete(&self, path: &str) -> Result<()>;

    /// Check if a path exists
    async fn exists(&self, path: &str) -> bool;

    /// Create a directory
    async fn mkdir(&self, path: &str) -> Result<()>;

    /// List directory contents
    async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>>;
}
