//! Background sync worker for optimistic updates
//!
//! Handles queued operations (edit/create/delete) in the background,
//! allowing immediate 202 Accepted responses to the client. Operations
//! are committed to git locally and pushed to the remote after a 5-second
//! debounce period.
//!
//! On failure, the worker sends 409 Conflict along with the correct
//! (original) state via SetVertexLabel/SetEdges messages so the client
//! displays the actual server state.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

use crate::connection_manager::SharedConnectionManager;
use crate::content_store::{ContentStore, LocalContentStore};
use crate::git_undo::GitUndoRepo;
use crate::nextcloud::NextcloudClient;
use crate::notes::NotesIndex;
use crate::protocol::{self, Direction};

/// Work item for the sync worker
#[derive(Debug)]
pub struct SyncWorkItem {
    pub action_id: u64,
    pub vertex_id: u64,
    pub operation: SyncOperation,
}

/// Types of sync operations
#[derive(Debug)]
pub enum SyncOperation {
    EditVertex {
        uuid: Uuid,
        content: Vec<u8>,
        /// New content hash (computed before queueing)
        new_hash: String,
        /// File extension for content store
        ext: String,
        mime: String,
        layer: u32,
        /// Original content hash before edit (for rollback)
        original_hash: String,
        /// Original mime type
        original_mime: String,
    },
    CreateVertex {
        uuid: Uuid,
        content: Vec<u8>,
        /// Content hash (computed before queueing)
        content_hash: String,
        /// File extension for content store
        ext: String,
        mime: String,
        from_vertex: u64,
        direction: Direction,
        /// Rollback: original edges of source vertex (before insertion)
        original_source_edges: Option<(u64, [u64; 6])>,  // (hash, edges)
        /// Rollback: original edges of displaced vertex (if any)
        displaced_vertex: Option<(u64, [u64; 6])>,  // (hash, edges)
    },
    DeleteVertex {
        uuid: Uuid,
        /// Content hash (for potential future GC, not deleted immediately)
        content_hashes_to_gc: String,
        /// Original vertex data for rollback
        original_mime: String,
        original_hash: String,
        /// Original edges for rollback
        original_edges: [u64; 6],
        /// Neighbors that were updated (need rollback too)
        affected_neighbors: Vec<(Uuid, [u64; 6])>,
    },
}

/// Response message to send back to the WebSocket handler
#[derive(Debug)]
pub enum SyncResponse {
    /// Send a log message
    Log {
        action_id: u64,
        status: u32,
        vertex_id: u64,
        message: String,
    },
    /// Send a SetVertexLabel to restore original state
    RestoreVertexLabel {
        action_id: u64,
        vertex_id: u64,
        layer: u32,
        mime: String,
        content: Vec<u8>,
    },
    /// Send SetEdges to restore original edges
    RestoreEdges {
        action_id: u64,
        vertex_id: u64,
        edges: [u64; 6],
        edit_mask: u8,
    },
}

impl SyncResponse {
    /// Encode the response as a binary protocol message
    pub fn encode(&self) -> Vec<u8> {
        match self {
            SyncResponse::Log { action_id, status, vertex_id, message } => {
                protocol::encode_log_message(*action_id, *status, *vertex_id, message)
            }
            SyncResponse::RestoreVertexLabel { action_id, vertex_id, layer, mime, content } => {
                protocol::encode_set_vertex_label_layer(*action_id, *vertex_id, *layer, mime, content)
            }
            SyncResponse::RestoreEdges { action_id, vertex_id, edges, edit_mask } => {
                protocol::encode_set_edges(
                    *action_id, *vertex_id,
                    edges[0], edges[1], edges[2], edges[3], edges[4], edges[5],
                    *edit_mask,
                )
            }
        }
    }
}

/// Pending action awaiting git push confirmation
struct PendingAction {
    action_id: u64,
    vertex_id: u64,
    /// Rollback data if push fails
    rollback: Option<RollbackData>,
}

/// Data needed to rollback a failed operation
#[derive(Clone)]
enum RollbackData {
    Edit {
        layer: u32,
        /// Original content hash (for CAS lookup on rollback)
        original_hash: String,
        original_mime: String,
    },
    Create {
        new_vertex_hash: u64,
        /// Original edges of source vertex (hash, edges) - if source was modified
        original_source_edges: Option<(u64, [u64; 6])>,
        /// Original edges of displaced vertex (hash, edges) - if a vertex was displaced
        displaced_vertex: Option<(u64, [u64; 6])>,
    },
    Delete {
        /// Original content hash (for CAS lookup on rollback)
        original_hash: String,
        original_mime: String,
        original_edges: [u64; 6],
        affected_neighbors: Vec<(u64, [u64; 6])>,  // vertex_hash -> original edges
    },
}

/// Background sync worker
pub struct SyncWorker {
    rx: mpsc::Receiver<SyncWorkItem>,
    nc: NextcloudClient,
    git_repo: Arc<Mutex<GitUndoRepo>>,
    index: Arc<Mutex<NotesIndex>>,
    /// Connection manager for sending responses back to browser
    connection_manager: SharedConnectionManager,
    /// Connection ID to send responses to
    conn_id: u64,
    /// Actions awaiting git push confirmation
    pending_actions: Vec<PendingAction>,
    /// Time of last git commit
    last_commit_time: Instant,
    /// Identity for git commits
    identity: String,
}

impl SyncWorker {
    /// Spawn the sync worker as a background task
    ///
    /// Returns a sender for queuing work items.
    pub fn spawn(
        nc: NextcloudClient,
        git_repo: Arc<Mutex<GitUndoRepo>>,
        index: Arc<Mutex<NotesIndex>>,
        connection_manager: SharedConnectionManager,
        conn_id: u64,
        identity: String,
    ) -> mpsc::Sender<SyncWorkItem> {
        let (tx, rx) = mpsc::channel(100);

        let worker = SyncWorker {
            rx,
            nc,
            git_repo,
            index,
            connection_manager,
            conn_id,
            pending_actions: Vec::new(),
            last_commit_time: Instant::now(),
            identity,
        };

        tokio::spawn(worker.run());

        tx
    }

    /// Send a message to the browser via connection manager
    fn send_to_browser(&self, msg: Vec<u8>) {
        let cm = self.connection_manager.clone();
        let conn_id = self.conn_id;
        // Use blocking_lock since we're in an async context but need sync access
        // Actually, we should make this async. Let's use try_send or spawn a task.
        tokio::spawn(async move {
            let cm = cm.lock().await;
            let _ = cm.send_to(conn_id, msg);
        });
    }

    /// Main worker loop
    async fn run(mut self) {
        let mut debounce_interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                // Handle incoming work items
                item = self.rx.recv() => {
                    match item {
                        Some(work_item) => {
                            self.process_item(work_item).await;
                        }
                        None => {
                            // Channel closed, worker should shut down
                            log::info!("Sync worker: channel closed, shutting down");
                            // Do final push before exit
                            if !self.pending_actions.is_empty() {
                                self.do_git_push().await;
                            }
                            break;
                        }
                    }
                }

                // Check for debounce timeout
                _ = debounce_interval.tick() => {
                    // Push if we have pending actions and 5 seconds have passed
                    if !self.pending_actions.is_empty()
                        && self.last_commit_time.elapsed() > Duration::from_secs(5)
                    {
                        self.do_git_push().await;
                    }
                }
            }
        }
    }

    /// Process a single work item
    async fn process_item(&mut self, item: SyncWorkItem) {
        log::info!("Sync worker: processing {:?}", item.operation);

        let result = match &item.operation {
            SyncOperation::EditVertex { .. } => self.do_edit(&item).await,
            SyncOperation::CreateVertex { .. } => self.do_create(&item).await,
            SyncOperation::DeleteVertex { .. } => self.do_delete(&item).await,
        };

        match result {
            Ok(rollback) => {
                // Operation succeeded locally, add to pending for push confirmation
                self.pending_actions.push(PendingAction {
                    action_id: item.action_id,
                    vertex_id: item.vertex_id,
                    rollback,
                });
                self.last_commit_time = Instant::now();
            }
            Err(e) => {
                // Operation failed immediately - send 409 and rollback state
                log::error!("Sync worker: operation failed: {}", e);
                self.send_failure_response(&item, &e.to_string()).await;
            }
        }
    }

    /// Execute edit operation
    async fn do_edit(&mut self, item: &SyncWorkItem) -> Result<Option<RollbackData>> {
        let SyncOperation::EditVertex {
            uuid, content, new_hash, ext, mime, layer,
            original_hash, original_mime,
        } = &item.operation else {
            return Err(anyhow!("Invalid operation type"));
        };

        // 1. Upload content via ContentStore (CAS)
        let content_store = ContentStore::new(self.nc.clone());
        content_store.put(content, ext).await?;

        // 2. Update index with new hash
        {
            let mut index = self.index.lock().await;
            if *layer == 1 {
                let transcript = String::from_utf8_lossy(content).to_string();
                index.update_vertex(*uuid, Some(transcript))?;
            }
            index.set_vertex_layer_hash(*uuid, *layer, mime, new_hash)?;
        }

        // 3. Save index to WebDAV
        {
            let index = self.index.lock().await;
            index.save(&self.nc).await?;
        }

        // 4. Write to local git workdir and commit
        // Note: Only index.toml is committed, not content files
        {
            let repo = self.git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                // Write index
                let index = self.index.lock().await;
                index.save_to_path(workdir)?;

                // Store content in local content-store (for reference, not tracked by git)
                let local_store = LocalContentStore::new(workdir);
                local_store.put(content, ext)?;

                // Commit (only index.toml)
                drop(index);
                repo.ensure_on_branch()?;
                repo.commit_all(&format!("Edit vertex: {}", uuid), &self.identity)?;
            }
        }

        Ok(Some(RollbackData::Edit {
            layer: *layer,
            original_hash: original_hash.clone(),
            original_mime: original_mime.clone(),
        }))
    }

    /// Execute create operation
    async fn do_create(&mut self, item: &SyncWorkItem) -> Result<Option<RollbackData>> {
        let SyncOperation::CreateVertex {
            uuid, content, content_hash: _, ext, mime: _, from_vertex: _, direction: _,
            original_source_edges, displaced_vertex,
        } = &item.operation else {
            return Err(anyhow!("Invalid operation type"));
        };

        // 1. Upload content via ContentStore (CAS)
        let content_store = ContentStore::new(self.nc.clone());
        content_store.put(content, ext).await?;

        // 2. Save index to WebDAV (index already updated optimistically)
        {
            let index = self.index.lock().await;
            index.save(&self.nc).await?;
        }

        // 3. Write to local git workdir and commit
        // Note: Only index.toml is committed, not content files
        {
            let repo = self.git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                let index = self.index.lock().await;
                index.save_to_path(workdir)?;

                // Store content in local content-store (for reference, not tracked by git)
                let local_store = LocalContentStore::new(workdir);
                local_store.put(content, ext)?;

                drop(index);
                repo.ensure_on_branch()?;
                repo.commit_all(&format!("Create vertex: {}", uuid), &self.identity)?;
            }
        }

        // For create, rollback means deleting the vertex and restoring original edges
        Ok(Some(RollbackData::Create {
            new_vertex_hash: item.vertex_id,
            original_source_edges: *original_source_edges,
            displaced_vertex: *displaced_vertex,
        }))
    }

    /// Execute delete operation
    async fn do_delete(&mut self, item: &SyncWorkItem) -> Result<Option<RollbackData>> {
        let SyncOperation::DeleteVertex {
            uuid, content_hashes_to_gc: _,
            original_mime, original_hash,
            original_edges, affected_neighbors,
        } = &item.operation else {
            return Err(anyhow!("Invalid operation type"));
        };

        // Note: With CAS, we DON'T delete content files immediately
        // Content files are immutable and may be referenced by old commits
        // Garbage collection handles cleanup of unreferenced hashes

        // 1. Save updated index to WebDAV
        {
            let index = self.index.lock().await;
            index.save(&self.nc).await?;
        }

        // 2. Update local git workdir and commit
        // Note: Only index.toml is committed, content files are NOT deleted
        {
            let repo = self.git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                let index = self.index.lock().await;
                index.save_to_path(workdir)?;

                // Note: We don't delete content from local store
                // GC will clean up unreferenced hashes

                drop(index);
                repo.ensure_on_branch()?;
                repo.commit_all(&format!("Delete vertex: {}", uuid), &self.identity)?;
            }
        }

        // Collect neighbor vertex hashes for rollback
        let neighbor_rollbacks: Vec<(u64, [u64; 6])> = affected_neighbors.iter()
            .map(|(uuid, edges)| (crate::notes::uuid_to_hash(*uuid), *edges))
            .collect();

        Ok(Some(RollbackData::Delete {
            original_hash: original_hash.clone(),
            original_mime: original_mime.clone(),
            original_edges: *original_edges,
            affected_neighbors: neighbor_rollbacks,
        }))
    }

    /// Push pending commits to remote
    async fn do_git_push(&mut self) {
        if self.pending_actions.is_empty() {
            return;
        }

        log::info!("Sync worker: pushing {} pending actions to remote", self.pending_actions.len());

        let push_result = {
            let repo = self.git_repo.lock().await;
            if repo.has_remote() {
                repo.push()
            } else {
                // No remote configured, treat as success
                Ok(())
            }
        };

        let actions = std::mem::take(&mut self.pending_actions);
        let action_count = actions.len();

        match push_result {
            Ok(()) => {
                // Send 200 OK for all pending actions
                for action in actions {
                    let msg = protocol::encode_log_message(
                        action.action_id,
                        protocol::STATUS_OK,
                        action.vertex_id,
                        "Synced",
                    );
                    self.send_to_browser(msg);
                }
                log::info!("Sync worker: push succeeded, sent OK for {} actions", action_count);
            }
            Err(e) => {
                // Push failed - send 409 Conflict and rollback state for all pending
                log::error!("Sync worker: push failed: {}", e);
                for action in actions {
                    self.send_rollback(
                        action.action_id,
                        action.vertex_id,
                        action.rollback,
                        &e.to_string(),
                    ).await;
                }
            }
        }
    }

    /// Send failure response with rollback data
    async fn send_failure_response(&self, item: &SyncWorkItem, error: &str) {
        // Send 409 Conflict
        let msg = protocol::encode_log_message(
            item.action_id,
            protocol::STATUS_CONFLICT,
            item.vertex_id,
            error,
        );
        self.send_to_browser(msg);

        // Send rollback state based on operation type
        // Note: For CAS, we use content hashes. The browser will need to re-fetch
        // content from the server if it wants to display the restored state.
        match &item.operation {
            SyncOperation::EditVertex {
                original_hash, original_mime, layer, ext, ..
            } => {
                // For rollback, fetch original content from CAS and send it
                let content_store = ContentStore::new(self.nc.clone());
                if let Ok(original_content) = content_store.get(original_hash, ext).await {
                    let restore_msg = protocol::encode_set_vertex_label_layer(
                        item.action_id,
                        item.vertex_id,
                        *layer,
                        original_mime,
                        &original_content,
                    );
                    self.send_to_browser(restore_msg);
                }
            }
            SyncOperation::CreateVertex { original_source_edges, displaced_vertex, .. } => {
                // Signal vertex doesn't exist by sending empty edges with edit_mask=0
                let delete_msg = protocol::encode_set_edges(
                    item.action_id,
                    item.vertex_id,
                    0, 0, 0, 0, 0, 0,
                    0,  // edit_mask = 0 means deleted/non-existent
                );
                self.send_to_browser(delete_msg);

                // Restore source vertex's original edges if applicable
                if let Some((source_hash, source_edges)) = original_source_edges {
                    let restore_source_msg = protocol::encode_set_edges(
                        item.action_id,
                        *source_hash,
                        source_edges[0], source_edges[1], source_edges[2],
                        source_edges[3], source_edges[4], source_edges[5],
                        0x7F,
                    );
                    self.send_to_browser(restore_source_msg);
                }

                // Restore displaced vertex's original edges if applicable
                if let Some((displaced_hash, displaced_edges)) = displaced_vertex {
                    let restore_displaced_msg = protocol::encode_set_edges(
                        item.action_id,
                        *displaced_hash,
                        displaced_edges[0], displaced_edges[1], displaced_edges[2],
                        displaced_edges[3], displaced_edges[4], displaced_edges[5],
                        0x7F,
                    );
                    self.send_to_browser(restore_displaced_msg);
                }
            }
            SyncOperation::DeleteVertex {
                original_hash, original_mime, original_edges, affected_neighbors, ..
            } => {
                // For rollback, fetch original content from CAS
                // Note: Content should still exist since we don't delete on delete_vertex
                let ext = crate::notes::mime_to_extension(original_mime);
                let content_store = ContentStore::new(self.nc.clone());
                if let Ok(original_content) = content_store.get(original_hash, ext).await {
                    // Restore deleted vertex
                    let restore_label = protocol::encode_set_vertex_label(
                        item.action_id,
                        item.vertex_id,
                        original_mime,
                        &original_content,
                    );
                    self.send_to_browser(restore_label);
                }

                // Restore edges
                let restore_edges = protocol::encode_set_edges(
                    item.action_id,
                    item.vertex_id,
                    original_edges[0], original_edges[1], original_edges[2],
                    original_edges[3], original_edges[4], original_edges[5],
                    0x7F,  // Full editability
                );
                self.send_to_browser(restore_edges);

                // Restore affected neighbors
                for (neighbor_uuid, neighbor_edges) in affected_neighbors {
                    let neighbor_hash = crate::notes::uuid_to_hash(*neighbor_uuid);
                    let neighbor_edges_msg = protocol::encode_set_edges(
                        item.action_id,
                        neighbor_hash,
                        neighbor_edges[0], neighbor_edges[1], neighbor_edges[2],
                        neighbor_edges[3], neighbor_edges[4], neighbor_edges[5],
                        0x7F,
                    );
                    self.send_to_browser(neighbor_edges_msg);
                }
            }
        }
    }

    /// Send rollback for a specific action
    async fn send_rollback(
        &self,
        action_id: u64,
        vertex_id: u64,
        rollback: Option<RollbackData>,
        error: &str,
    ) {
        // Send 409 Conflict
        let msg = protocol::encode_log_message(
            action_id,
            protocol::STATUS_CONFLICT,
            vertex_id,
            error,
        );
        self.send_to_browser(msg);

        // Send rollback state if available
        // Note: For CAS, we need to fetch content from store using hash
        if let Some(rollback) = rollback {
            match rollback {
                RollbackData::Edit { layer, original_hash, original_mime } => {
                    // Fetch original content from CAS
                    let ext = crate::notes::mime_to_extension(&original_mime);
                    let content_store = ContentStore::new(self.nc.clone());
                    if let Ok(original_content) = content_store.get(&original_hash, ext).await {
                        let restore_msg = protocol::encode_set_vertex_label_layer(
                            action_id,
                            vertex_id,
                            layer,
                            &original_mime,
                            &original_content,
                        );
                        self.send_to_browser(restore_msg);
                    }
                }
                RollbackData::Create { new_vertex_hash, original_source_edges, displaced_vertex } => {
                    // Signal vertex doesn't exist
                    let delete_msg = protocol::encode_set_edges(
                        action_id,
                        new_vertex_hash,
                        0, 0, 0, 0, 0, 0,
                        0,
                    );
                    self.send_to_browser(delete_msg);

                    // Restore source vertex's original edges if applicable
                    if let Some((source_hash, source_edges)) = original_source_edges {
                        let restore_source_msg = protocol::encode_set_edges(
                            action_id,
                            source_hash,
                            source_edges[0], source_edges[1], source_edges[2],
                            source_edges[3], source_edges[4], source_edges[5],
                            0x7F,
                        );
                        self.send_to_browser(restore_source_msg);
                    }

                    // Restore displaced vertex's original edges if applicable
                    if let Some((displaced_hash, displaced_edges)) = displaced_vertex {
                        let restore_displaced_msg = protocol::encode_set_edges(
                            action_id,
                            displaced_hash,
                            displaced_edges[0], displaced_edges[1], displaced_edges[2],
                            displaced_edges[3], displaced_edges[4], displaced_edges[5],
                            0x7F,
                        );
                        self.send_to_browser(restore_displaced_msg);
                    }
                }
                RollbackData::Delete { original_hash, original_mime, original_edges, affected_neighbors } => {
                    // Fetch original content from CAS
                    let ext = crate::notes::mime_to_extension(&original_mime);
                    let content_store = ContentStore::new(self.nc.clone());
                    if let Ok(original_content) = content_store.get(&original_hash, ext).await {
                        // Restore vertex
                        let restore_label = protocol::encode_set_vertex_label(
                            action_id,
                            vertex_id,
                            &original_mime,
                            &original_content,
                        );
                        self.send_to_browser(restore_label);
                    }

                    let restore_edges = protocol::encode_set_edges(
                        action_id,
                        vertex_id,
                        original_edges[0], original_edges[1], original_edges[2],
                        original_edges[3], original_edges[4], original_edges[5],
                        0x7F,
                    );
                    self.send_to_browser(restore_edges);

                    // Restore neighbors
                    for (neighbor_hash, neighbor_edges) in affected_neighbors {
                        let neighbor_msg = protocol::encode_set_edges(
                            action_id,
                            neighbor_hash,
                            neighbor_edges[0], neighbor_edges[1], neighbor_edges[2],
                            neighbor_edges[3], neighbor_edges[4], neighbor_edges[5],
                            0x7F,
                        );
                        self.send_to_browser(neighbor_msg);
                    }
                }
            }
        }
    }
}
