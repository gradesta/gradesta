//! Vertex CRUD handlers

use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{anyhow, Result};
use axum::extract::ws::Message as AxumWsMessage;
use futures_util::SinkExt;

use crate::content_store::{self, ContentStore};
use crate::git_undo;
use crate::notes::{self, mime_to_extension, uuid_to_hash};
use crate::protocol::*;
use crate::state::ConnState;
use crate::sync_worker;

pub async fn handle_set_vertex_label<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id, layer, mime, content) = parse_client_set_vertex_label(data)?;
    log::info!("SetVertexLabel: action={}, vertex={}, layer={}, mime={}", action_id, vertex_id, layer, mime);

    let (nc, index, sync_tx) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone(), s.sync_tx.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, vertex_id, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find vertex by hash
    let uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            log::warn!("Vertex not found: {}", vertex_id);
            let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    let vertex = match index.get_vertex(uuid) {
        Some(v) => v.clone(),
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Compute new content hash
    let ext = mime_to_extension(&mime);
    let new_content_hash = content_store::compute_hash(&content);

    // Check if we have a sync worker for optimistic updates
    if let Some(sync_tx) = sync_tx {
        // OPTIMISTIC PATH: Queue work and return 202 immediately

        // Store original hash for potential rollback
        let original_hash = vertex.content_hash.clone();
        let original_mime = vertex.mime.clone();

        // Queue the work item
        let work_item = sync_worker::SyncWorkItem {
            action_id,
            vertex_id,
            operation: sync_worker::SyncOperation::EditVertex {
                uuid,
                content: content.clone(),
                new_hash: new_content_hash.clone(),
                ext: ext.to_string(),
                mime: mime.clone(),
                layer,
                original_hash,
                original_mime,
            },
        };

        if let Err(e) = sync_tx.send(work_item).await {
            log::error!("Failed to queue edit work item: {}", e);
            let msg = encode_log_message(action_id, 500, vertex_id, "Failed to queue operation");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Send immediate 202 Accepted response
        let msg = encode_log_message(action_id, STATUS_ACCEPTED, vertex_id, "Accepted");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        log::info!("Queued edit for vertex {} with new hash {} (optimistic)", uuid, new_content_hash);
    } else {
        // SYNCHRONOUS PATH: No sync worker, do everything inline
        let mut index = index;

        // Get identity and git repo for undo
        let (identity, git_repo) = {
            let s = state.lock().await;
            (s.identity.clone(), s.git_undo_repo.clone())
        };

        // Use ContentStore for all layers
        let content_store = ContentStore::new(nc.clone());

        if layer == 0 {
            // Layer 0: Update primary content via CAS
            match content_store.put(&content, ext).await {
                Ok(stored_hash) => {
                    debug_assert_eq!(stored_hash, new_content_hash);
                    if let Err(e) = index.set_vertex_layer_hash(uuid, layer, &mime, &new_content_hash) {
                        log::warn!("Failed to update layer hash: {}", e);
                    }
                }
                Err(e) => {
                    let msg = encode_log_message(action_id, 500, vertex_id, &format!("Upload failed: {}", e));
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    return Ok(());
                }
            }
        } else if layer == 1 {
            // Layer 1: Transcript - stored directly in index.toml, NOT in CAS
            let transcript = String::from_utf8_lossy(&content).to_string();
            if let Err(e) = index.update_vertex(uuid, Some(transcript)) {
                log::warn!("Failed to update transcript: {}", e);
            }
            // No CAS upload or layer hash update needed - transcript is stored in transcript field
        } else {
            // Other layers - use CAS
            match content_store.put(&content, ext).await {
                Ok(hash) => {
                    if let Err(e) = index.set_vertex_layer_hash(uuid, layer, &mime, &hash) {
                        log::warn!("Failed to set layer hash: {}", e);
                    }
                }
                Err(e) => {
                    let msg = encode_log_message(action_id, 500, vertex_id, &format!("Upload failed: {}", e));
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    return Ok(());
                }
            }
        }

        // Save index to Nextcloud
        if let Err(e) = index.save(&nc).await {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Git commit and sync (only index.toml, not content)
        let mut should_sync = false;
        let mut sync_path = None;
        if let Some(git_repo) = &git_repo {
            let repo = git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                // Save index to git workdir
                if let Err(e) = index.save_to_path(workdir) {
                    log::warn!("Failed to save index to git workdir: {}", e);
                }
                // Store content in local content-store (for reference, not tracked by git)
                let local_store = content_store::LocalContentStore::new(workdir);
                if let Err(e) = local_store.put(&content, ext) {
                    log::warn!("Failed to write content to local store: {}", e);
                }
                let author = identity.as_deref().unwrap_or("unknown");
                if let Err(e) = repo.ensure_on_branch() {
                    log::warn!("Failed to ensure on branch: {}", e);
                }
                if let Err(e) = repo.commit_all(&format!("Edit vertex: {}", uuid), author) {
                    log::warn!("Failed to create git commit: {}", e);
                } else {
                    should_sync = true;
                    sync_path = Some(repo.local_path().to_path_buf());
                }
            }
        }
        if should_sync {
            if let Some(path) = sync_path {
                if let Err(e) = git_undo::sync_to_nextcloud(&path, &nc).await {
                    log::warn!("Failed to sync git repo to Nextcloud: {}", e);
                }
            }
        }

        // Update state
        {
            let mut s = state.lock().await;
            s.index = Some(index);
        }

        log::info!("Updated vertex {} with hash {} (sync)", uuid, new_content_hash);
        let msg = encode_log_message(action_id, 200, vertex_id, "OK");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    Ok(())
}

pub async fn handle_set_edges<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id, edges) = parse_client_set_edges(data)?;
    log::info!("SetEdges: action={}, vertex={}", action_id, vertex_id);

    let (nc, index) = {
        let s = state.lock().await;
        (s.nextcloud.clone(), s.index.clone())
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, vertex_id, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Find source vertex
    let from_uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Source vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Update edges
    // EDGE_UNCHANGED (u64::MAX) means keep existing, 0 means remove, other values set the edge
    let directions = [
        Direction::West,
        Direction::East,
        Direction::North,
        Direction::South,
        Direction::Up,
        Direction::Down,
    ];

    for (i, &target_hash) in edges.iter().enumerate() {
        if target_hash == u64::MAX {
            // EDGE_UNCHANGED - skip this edge
            continue;
        } else if target_hash == 0 {
            // Remove edge in this direction
            index.remove_edge(from_uuid, directions[i]);
        } else {
            // Set edge to target vertex
            if let Some(to_uuid) = notes::hash_to_uuid(&index, target_hash) {
                index.add_edge(from_uuid, to_uuid, directions[i]);
            }
        }
    }

    // Save index
    if let Err(e) = index.save(&nc).await {
        let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        return Ok(());
    }

    // Update state
    {
        let mut s = state.lock().await;
        s.index = Some(index);
    }

    log::info!("Updated edges for vertex");

    // Send success acknowledgment
    let msg = encode_log_message(action_id, 200, vertex_id, "OK");
    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    Ok(())
}

pub async fn handle_create_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, from_vertex, direction, _layer, mime, content) = parse_client_create_vertex(data)?;
    log::info!(
        "CreateVertex: action={}, from={}, direction={:?}, mime={}",
        action_id,
        from_vertex,
        direction,
        mime
    );

    let (nc, index, identity, git_repo, sync_tx, shared_index) = {
        let s = state.lock().await;
        (
            s.nextcloud.clone(),
            s.index.clone(),
            s.identity.clone(),
            s.git_undo_repo.clone(),
            s.sync_tx.clone(),
            s.shared_index.clone(),
        )
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, 0, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let identity_str = identity.clone().unwrap_or_default();

    // Compute content hash for CAS
    let ext = mime_to_extension(&mime);
    let content_hash = content_store::compute_hash(&content);

    // Capture original state for rollback BEFORE modifying index
    let original_source_edges: Option<(u64, [u64; 6])> = if from_vertex != 0 {
        notes::hash_to_uuid(&index, from_vertex)
            .map(|uuid| (from_vertex, index.build_edge_array(uuid)))
    } else {
        None
    };

    // Extract transcript for audio
    let transcript = if mime == "audio/ogg" {
        None
    } else {
        None
    };

    // Create vertex in index with content hash (optimistic)
    let new_id = index.create_vertex_with_hash(&mime, &content_hash, transcript);

    // Insert into chain and capture displaced vertex
    let displaced_vertex: Option<(u64, [u64; 6])> = if from_vertex != 0 {
        if let Some(from_uuid) = notes::hash_to_uuid(&index, from_vertex) {
            // Capture displaced vertex's edges BEFORE insertion
            let displaced_uuid = index.get_neighbor(from_uuid, direction.as_str());
            let displaced_info = displaced_uuid.map(|u| (uuid_to_hash(u), index.build_edge_array(u)));

            index.insert_vertex(from_uuid, new_id, direction);
            displaced_info
        } else {
            None
        }
    } else {
        None
    };

    let vertex_hash = uuid_to_hash(new_id);

    // Update shared index for sync worker
    if let Some(ref shared) = shared_index {
        *shared.lock().await = index.clone();
    }

    // Update state index
    {
        let mut s = state.lock().await;
        s.index = Some(index.clone());
    }

    // Send edge updates IMMEDIATELY (optimistic response)
    // Send new vertex label first (so browser has content to display)
    let label_msg = encode_set_vertex_label(action_id, vertex_hash, &mime, &content);
    write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send new vertex edges
    let edge_array = index.build_edge_array(new_id);
    let edges_msg = encode_set_edges(
        action_id,
        vertex_hash,
        edge_array[0],
        edge_array[1],
        edge_array[2],
        edge_array[3],
        edge_array[4],
        edge_array[5],
        0x7F,
    );
    write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send source vertex edges if applicable
    log::info!("CreateVertex: from_vertex={}, direction={:?}, new_vertex_hash={}", from_vertex, direction, vertex_hash);
    if from_vertex != 0 {
        if let Some(from_uuid) = notes::hash_to_uuid(&index, from_vertex) {
            let from_edge_array = index.build_edge_array(from_uuid);
            log::info!("Sending updated edges for source vertex {} (uuid={}) -> {:?}", from_vertex, from_uuid, from_edge_array);
            let from_edges_msg = encode_set_edges(
                action_id,
                from_vertex,
                from_edge_array[0],
                from_edge_array[1],
                from_edge_array[2],
                from_edge_array[3],
                from_edge_array[4],
                from_edge_array[5],
                0x7F,
            );
            write.send(AxumWsMessage::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        } else {
            // Source vertex not in index (e.g., the "empty placeholder")
            log::info!("Source vertex {} not in index - sending synthetic edge update pointing to new vertex", from_vertex);
            let dir_idx = match direction {
                Direction::West => 0,
                Direction::East => 1,
                Direction::North => 2,
                Direction::South => 3,
                Direction::Up => 4,
                Direction::Down => 5,
            };
            let mut edges = [0u64; 6];
            edges[dir_idx] = vertex_hash;
            let from_edges_msg = encode_set_edges(
                action_id,
                from_vertex,
                edges[0], edges[1], edges[2], edges[3], edges[4], edges[5],
                0x7F,
            );
            write.send(AxumWsMessage::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    }

    // Send displaced vertex edges if applicable
    if let Some((displaced_hash, _)) = &displaced_vertex {
        if let Some(displaced_uuid) = notes::hash_to_uuid(&index, *displaced_hash) {
            let displaced_edge_array = index.build_edge_array(displaced_uuid);
            log::info!("Sending updated edges for displaced vertex {} -> {:?}", displaced_hash, displaced_edge_array);
            let displaced_edges_msg = encode_set_edges(
                action_id,
                *displaced_hash,
                displaced_edge_array[0],
                displaced_edge_array[1],
                displaced_edge_array[2],
                displaced_edge_array[3],
                displaced_edge_array[4],
                displaced_edge_array[5],
                0x7F,
            );
            write.send(AxumWsMessage::Binary(displaced_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    }

    // Check if we have sync worker for async processing
    if let Some(sync_tx) = sync_tx {
        // Queue work to background
        let work_item = sync_worker::SyncWorkItem {
            action_id,
            vertex_id: vertex_hash,
            operation: sync_worker::SyncOperation::CreateVertex {
                uuid: new_id,
                content: content.clone(),
                content_hash: content_hash.clone(),
                ext: ext.to_string(),
                mime: mime.clone(),
                from_vertex,
                direction,
                original_source_edges,
                displaced_vertex,
            },
        };
        if let Err(e) = sync_tx.send(work_item).await {
            log::error!("Failed to queue create work: {}", e);
            let msg = encode_log_message(action_id, 500, vertex_hash, "Failed to queue work");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Send 202 Accepted
        let msg = encode_log_message(action_id, STATUS_ACCEPTED, vertex_hash, "Accepted");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        log::info!("Created vertex {} with hash {} (async)", new_id, content_hash);
    } else {
        // Synchronous fallback - use ContentStore for upload
        let content_store = ContentStore::new(nc.clone());
        let stored_hash = match content_store.put(&content, ext).await {
            Ok(h) => h,
            Err(e) => {
                let msg = encode_log_message(action_id, 500, 0, &format!("Upload failed: {}", e));
                write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                return Ok(());
            }
        };

        // Verify hash matches what we computed
        debug_assert_eq!(stored_hash, content_hash);

        // Save index to Nextcloud
        if let Err(e) = index.save(&nc).await {
            let msg = encode_log_message(action_id, 500, 0, &format!("Save failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Also save to local git working directory and commit
        // Note: Content is stored via ContentStore, git only tracks index.toml
        let mut should_sync = false;
        let mut sync_path = None;
        if let Some(git_repo) = &git_repo {
            let repo = git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                // Save index to git workdir
                if let Err(e) = index.save_to_path(workdir) {
                    log::warn!("Failed to save index to git workdir: {}", e);
                }
                // Store content in local content-store (for reference, not tracked by git)
                let local_store = content_store::LocalContentStore::new(workdir);
                if let Err(e) = local_store.put(&content, ext) {
                    log::warn!("Failed to write content to local store: {}", e);
                }
                let author = &identity_str;
                let message = format!("Create vertex: {}", new_id);
                if let Err(e) = repo.ensure_on_branch() {
                    log::warn!("Failed to ensure on branch: {}", e);
                }
                if let Err(e) = repo.commit_all(&message, author) {
                    log::warn!("Failed to create git commit: {}", e);
                } else {
                    should_sync = true;
                    sync_path = Some(repo.local_path().to_path_buf());
                }
            }
        }
        if should_sync {
            if let Some(path) = sync_path {
                if let Err(e) = git_undo::sync_to_nextcloud(&path, &nc).await {
                    log::warn!("Failed to sync git repo to Nextcloud: {}", e);
                }
            }
        }

        // Send success acknowledgment
        let msg = encode_log_message(action_id, 200, vertex_hash, "Created");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        log::info!("Created vertex {} with hash {}", new_id, content_hash);
    }

    Ok(())
}

pub async fn handle_delete_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id) = parse_client_delete_vertex(data)?;
    log::info!("DeleteVertex: action={}, vertex={}", action_id, vertex_id);

    let (nc, index, identity, git_repo, sync_tx, shared_index) = {
        let s = state.lock().await;
        (
            s.nextcloud.clone(),
            s.index.clone(),
            s.identity.clone(),
            s.git_undo_repo.clone(),
            s.sync_tx.clone(),
            s.shared_index.clone(),
        )
    };

    let nc = match nc {
        Some(nc) => nc,
        None => {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let mut index = match index {
        Some(idx) => idx,
        None => {
            let msg = encode_log_message(action_id, 500, 0, "No index loaded");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };
    let identity_str = identity.unwrap_or_default();

    // Find vertex by hash
    let vertex_uuid = match notes::hash_to_uuid(&index, vertex_id) {
        Some(uuid) => uuid,
        None => {
            let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Capture original state BEFORE deletion
    let vertex = index.get_vertex(vertex_uuid).ok_or_else(|| anyhow!("Vertex not found"))?.clone();
    let original_edges = index.build_edge_array(vertex_uuid);

    // Capture affected neighbors' original edges before deletion
    let mut affected_neighbors_original: Vec<(uuid::Uuid, [u64; 6])> = Vec::new();
    for dir in ["west", "east", "north", "south", "up", "down"] {
        if let Some(neighbor_uuid) = index.get_neighbor(vertex_uuid, dir) {
            // Avoid duplicates
            if !affected_neighbors_original.iter().any(|(u, _)| *u == neighbor_uuid) {
                affected_neighbors_original.push((neighbor_uuid, index.build_edge_array(neighbor_uuid)));
            }
        }
    }

    // Delete vertex from index (optimistic)
    // Note: With CAS, files_to_delete is legacy - we don't delete content immediately
    let (_files_to_delete, affected_neighbors) = match index.delete_vertex(vertex_uuid) {
        Ok(result) => result,
        Err(e) => {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Delete failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    // Update shared index for sync worker
    if let Some(ref shared) = shared_index {
        *shared.lock().await = index.clone();
    }

    // Update state index
    {
        let mut s = state.lock().await;
        s.index = Some(index.clone());
    }

    // Send edge updates IMMEDIATELY (optimistic response)
    // Send updated edges for all affected neighbors
    for neighbor_uuid in &affected_neighbors {
        let neighbor_hash = uuid_to_hash(*neighbor_uuid);
        let edge_array = index.build_edge_array(*neighbor_uuid);
        log::info!("Sending updated edges for neighbor {} after delete -> {:?}", neighbor_hash, edge_array);
        let edges_msg = encode_set_edges(
            action_id,
            neighbor_hash,
            edge_array[0],
            edge_array[1],
            edge_array[2],
            edge_array[3],
            edge_array[4],
            edge_array[5],
            0x7F,
        );
        write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    // Send deletion signal for the vertex
    let deleted_edges_msg = encode_set_edges(
        action_id,
        vertex_id,
        0, 0, 0, 0, 0, 0,
        0,  // edit_mask = 0 means read-only (deleted)
    );
    write.send(AxumWsMessage::Binary(deleted_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Check if we have sync worker for async processing
    if let Some(sync_tx) = sync_tx {
        // Queue work to background
        // Note: Content hashes are NOT deleted immediately - GC handles cleanup
        let work_item = sync_worker::SyncWorkItem {
            action_id,
            vertex_id,
            operation: sync_worker::SyncOperation::DeleteVertex {
                uuid: vertex_uuid,
                // For CAS, we don't delete content files immediately
                // GC will clean up unreferenced hashes later
                content_hashes_to_gc: vertex.content_hash.clone(),
                original_mime: vertex.mime.clone(),
                original_hash: vertex.content_hash.clone(),
                original_edges,
                affected_neighbors: affected_neighbors_original,
            },
        };
        if let Err(e) = sync_tx.send(work_item).await {
            log::error!("Failed to queue delete work: {}", e);
            let msg = encode_log_message(action_id, 500, vertex_id, "Failed to queue work");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Send 202 Accepted
        let msg = encode_log_message(action_id, STATUS_ACCEPTED, vertex_id, "Accepted");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        log::info!("Deleted vertex {} (uuid={}) (async)", vertex_id, vertex_uuid);
    } else {
        // Synchronous fallback
        // Note: With CAS, we DON'T delete content files immediately
        // GC handles cleanup of unreferenced hashes

        // Save updated index to Nextcloud
        if let Err(e) = index.save(&nc).await {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Save failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }

        // Also save to local git working directory and commit
        // Content store files are NOT deleted - GC handles that
        let mut should_sync = false;
        let mut sync_path = None;
        if let Some(git_repo) = &git_repo {
            let repo = git_repo.lock().await;
            if let Some(workdir) = repo.workdir() {
                if let Err(e) = index.save_to_path(workdir) {
                    log::warn!("Failed to save index to git workdir: {}", e);
                }
                // Note: We don't delete content from local store
                // GC will clean up unreferenced hashes
                let author = &identity_str;
                let message = format!("Delete vertex: {}", vertex_uuid);
                if let Err(e) = repo.ensure_on_branch() {
                    log::warn!("Failed to ensure on branch: {}", e);
                }
                if let Err(e) = repo.commit_all(&message, author) {
                    log::warn!("Failed to create git commit: {}", e);
                } else {
                    should_sync = true;
                    sync_path = Some(repo.local_path().to_path_buf());
                }
            }
        }
        if should_sync {
            if let Some(path) = sync_path {
                if let Err(e) = git_undo::sync_to_nextcloud(&path, &nc).await {
                    log::warn!("Failed to sync git repo to Nextcloud: {}", e);
                }
            }
        }

        // Send success acknowledgment
        let msg = encode_log_message(action_id, 200, vertex_id, "Deleted");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        log::info!("Deleted vertex {} (uuid={})", vertex_id, vertex_uuid);
    }

    Ok(())
}
