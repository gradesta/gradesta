//! Test harness for nextcloud-connector tests
//!
//! Provides utilities for setting up test environments with mock WebDAV,
//! temporary git repositories, and message capture.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::connection_manager::{new_shared_connection_manager, SharedConnectionManager};
use crate::git_undo::GitUndoRepo;
use crate::notes::NotesIndex;
use crate::sync_worker::{SyncWorkItem, SyncWorker};

use super::mock_webdav::MockWebDav;

/// Test harness containing all test infrastructure
pub struct TestHarness {
    /// Mock WebDAV client
    pub webdav: MockWebDav,
    /// Temporary directory for git repo
    pub temp_dir: tempfile::TempDir,
    /// Notes index
    pub index: Arc<Mutex<NotesIndex>>,
    /// Git undo repository
    pub git_repo: Arc<Mutex<GitUndoRepo>>,
    /// Connection manager for message routing
    pub connection_manager: SharedConnectionManager,
    /// Receiver for captured messages (from browser connection)
    pub message_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Connection ID for the test connection
    pub conn_id: u64,
    /// Sender for sync work items (if sync worker is spawned)
    pub sync_tx: Option<mpsc::Sender<SyncWorkItem>>,
}

impl TestHarness {
    /// Create a new test harness without sync worker
    pub async fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        Self::with_temp_dir(temp_dir).await
    }

    /// Create a test harness with a specific temp directory
    pub async fn with_temp_dir(temp_dir: tempfile::TempDir) -> Self {
        let webdav = MockWebDav::new();

        // Create notes directory structure
        let notes_dir = temp_dir.path().join(".gradesta-notes");
        std::fs::create_dir_all(&notes_dir).expect("Failed to create notes dir");

        // Initialize index
        let index = NotesIndex::default();
        let index = Arc::new(Mutex::new(index));

        // Initialize git repo
        let git_repo = GitUndoRepo::open_or_init(&notes_dir)
            .expect("Failed to init git repo");
        let git_repo = Arc::new(Mutex::new(git_repo));

        // Set up connection manager with message capture
        let connection_manager = new_shared_connection_manager();
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let conn_id = 1;

        // Register our test connection
        {
            let mut cm = connection_manager.lock().await;
            cm.register(conn_id, message_tx);
        }

        Self {
            webdav,
            temp_dir,
            index,
            git_repo,
            connection_manager,
            message_rx,
            conn_id,
            sync_tx: None,
        }
    }

    /// Spawn a sync worker for async operation testing
    pub fn spawn_sync_worker(&mut self, identity: &str) {
        let tx = SyncWorker::spawn(
            self.webdav_as_nextcloud_client(),
            Arc::clone(&self.git_repo),
            Arc::clone(&self.index),
            Arc::clone(&self.connection_manager),
            self.conn_id,
            identity.to_string(),
        );
        self.sync_tx = Some(tx);
    }

    /// Get the path to the git workdir
    pub fn workdir(&self) -> PathBuf {
        self.temp_dir.path().join(".gradesta-notes")
    }

    /// Get the path to the content directory
    pub fn content_dir(&self) -> PathBuf {
        self.workdir().join("content")
    }

    /// Create a fake NextcloudClient that wraps the mock
    /// (For APIs that specifically need NextcloudClient)
    fn webdav_as_nextcloud_client(&self) -> crate::nextcloud::NextcloudClient {
        // Create a minimal NextcloudClient that won't be used for actual requests
        // since our tests use the mock directly
        crate::nextcloud::NextcloudClient::new(
            "http://mock.local",
            "testuser",
            "testpass",
        )
    }

    /// Get a mutable reference to the index
    pub async fn index_mut(&self) -> tokio::sync::MutexGuard<'_, NotesIndex> {
        self.index.lock().await
    }

    /// Get a reference to the git repo
    pub async fn git_repo(&self) -> tokio::sync::MutexGuard<'_, GitUndoRepo> {
        self.git_repo.lock().await
    }

    /// Receive all pending messages (non-blocking)
    pub fn drain_messages(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();
        while let Ok(msg) = self.message_rx.try_recv() {
            messages.push(msg);
        }
        messages
    }

    /// Wait for a message with timeout
    pub async fn recv_message_timeout(
        &mut self,
        timeout_ms: u64,
    ) -> Option<Vec<u8>> {
        tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            self.message_rx.recv(),
        )
        .await
        .ok()
        .flatten()
    }

    /// Parse a LogMessage from received bytes
    pub fn parse_log_message(msg: &[u8]) -> Option<(u64, u32, u64, String)> {
        if msg.is_empty() || msg[0] != crate::protocol::MSG_SERVER_LOG_MESSAGE {
            return None;
        }
        if msg.len() < 1 + 8 + 4 + 8 {
            return None;
        }
        let action_id = u64::from_be_bytes(msg[1..9].try_into().ok()?);
        let status = u32::from_be_bytes(msg[9..13].try_into().ok()?);
        let vertex_id = u64::from_be_bytes(msg[13..21].try_into().ok()?);
        let message = String::from_utf8(msg[21..].to_vec()).ok()?;
        Some((action_id, status, vertex_id, message))
    }

    /// Parse a SetEdges message from received bytes
    pub fn parse_set_edges(msg: &[u8]) -> Option<(u64, u64, [u64; 6], u8)> {
        if msg.is_empty() || msg[0] != crate::protocol::MSG_SERVER_SET_EDGES {
            return None;
        }
        if msg.len() < 1 + 8 + 8 + 48 + 1 {
            return None;
        }
        let action_id = u64::from_be_bytes(msg[1..9].try_into().ok()?);
        let vertex_id = u64::from_be_bytes(msg[9..17].try_into().ok()?);
        let west = u64::from_be_bytes(msg[17..25].try_into().ok()?);
        let east = u64::from_be_bytes(msg[25..33].try_into().ok()?);
        let north = u64::from_be_bytes(msg[33..41].try_into().ok()?);
        let south = u64::from_be_bytes(msg[41..49].try_into().ok()?);
        let up = u64::from_be_bytes(msg[49..57].try_into().ok()?);
        let down = u64::from_be_bytes(msg[57..65].try_into().ok()?);
        let edit_mask = msg[65];
        Some((action_id, vertex_id, [west, east, north, south, up, down], edit_mask))
    }

    /// Parse a SetVertexLabel message from received bytes
    pub fn parse_set_vertex_label(msg: &[u8]) -> Option<(u64, u64, u32, String, Vec<u8>)> {
        if msg.is_empty() || msg[0] != crate::protocol::MSG_SERVER_SET_VERTEX_LABEL {
            return None;
        }
        if msg.len() < 1 + 8 + 8 + 4 {
            return None;
        }
        let action_id = u64::from_be_bytes(msg[1..9].try_into().ok()?);
        let vertex_id = u64::from_be_bytes(msg[9..17].try_into().ok()?);
        let layer = u32::from_be_bytes(msg[17..21].try_into().ok()?);

        // Find null terminator for MIME type
        let mime_end = msg[21..].iter().position(|&b| b == 0)?;
        let mime = String::from_utf8(msg[21..21 + mime_end].to_vec()).ok()?;
        let content = msg[21 + mime_end + 1..].to_vec();

        Some((action_id, vertex_id, layer, mime, content))
    }
}

/// Helper to create a vertex in the index and return its hash
pub async fn create_test_vertex(
    index: &Arc<Mutex<NotesIndex>>,
    mime: &str,
    file: &str,
    transcript: Option<String>,
) -> (uuid::Uuid, u64) {
    let mut index = index.lock().await;
    let uuid = index.create_vertex(mime, file, transcript);
    let hash = crate::notes::uuid_to_hash(uuid);
    (uuid, hash)
}

/// Helper to add an edge between vertices
pub async fn add_test_edge(
    index: &Arc<Mutex<NotesIndex>>,
    from: uuid::Uuid,
    to: uuid::Uuid,
    direction: crate::protocol::Direction,
) {
    let mut index = index.lock().await;
    index.add_edge(from, to, direction);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_creation() {
        let harness = TestHarness::new().await;

        // Should have an empty index
        let index = harness.index.lock().await;
        assert!(index.vertices.is_empty());

        // Git repo should exist
        let repo = harness.git_repo.lock().await;
        assert!(repo.head_commit().is_some());
    }

    #[tokio::test]
    async fn test_message_capture() {
        let mut harness = TestHarness::new().await;

        // Send a message through the connection manager
        {
            let cm = harness.connection_manager.lock().await;
            cm.send_to(harness.conn_id, vec![1, 2, 3]).unwrap();
        }

        // Should receive it
        let messages = harness.drain_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_create_test_vertex() {
        let harness = TestHarness::new().await;

        let (uuid, hash) = create_test_vertex(
            &harness.index,
            "text/plain",
            "test.txt",
            Some("Hello".to_string()),
        ).await;

        // Verify vertex was created
        let index = harness.index.lock().await;
        let vertex = index.get_vertex(uuid).unwrap();
        assert_eq!(vertex.mime, "text/plain");
        assert_eq!(vertex.file, "test.txt");
        assert_eq!(vertex.transcript, Some("Hello".to_string()));

        // Hash should be valid
        assert_ne!(hash, 0);
    }
}
