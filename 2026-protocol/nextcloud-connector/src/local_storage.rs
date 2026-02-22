//! Local storage client for offline mode
//!
//! This module provides local file storage as an alternative to Nextcloud.

use std::path::PathBuf;

/// Client for local file storage
pub struct LocalStorageClient {
    #[allow(dead_code)]
    storage_path: PathBuf,
}

impl LocalStorageClient {
    /// Create a new local storage client
    pub fn new(storage_path: &PathBuf) -> Self {
        Self {
            storage_path: storage_path.clone(),
        }
    }
}
