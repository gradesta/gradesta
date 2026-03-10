//! Click and undo handlers

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{anyhow, Result};
use axum::extract::ws::Message as AxumWsMessage;
use futures_util::SinkExt;

use crate::content_store::ContentStore;
use crate::git_undo;
use crate::http_stream;
use crate::notes::{self, uuid_to_hash};
use crate::protocol::*;
use crate::state::ConnState;
use crate::utils::hash_string;

/// Max file size to send via WebSocket (10 MB) - larger files use HTTP streaming
pub const MAX_WS_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// MIME types that should always use HTTP streaming (videos)
pub const STREAMING_MIME_TYPES: &[&str] = &[
    "video/mp4",
    "video/webm",
    "video/quicktime",
    "video/x-matroska",
    "video/ogg",
];

/// Handle click on a vertex (toggle, action, etc.)
pub async fn handle_click_vertex<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, vertex_id) = parse_click_vertex(data)?;
    log::info!("ClickVertex: action={}, vertex={}", action_id, vertex_id);

    // Check if this is an undo tree vertex click
    if handle_undo_click(state, write, action_id, vertex_id).await? {
        return Ok(()); // Handled as undo click
    }

    // Check if this is a file entry click
    let (file_path, nc, jwt_secret, server_port, index) = {
        let s = state.lock().await;
        (
            s.file_entries.get(&vertex_id).cloned(),
            s.nextcloud.clone(),
            Arc::clone(&s.jwt_secret),
            s.server_port,
            s.index.clone(),
        )
    };

    if let Some(path) = file_path {
        let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;

        log::info!("Loading full content for file: {}", path);

        // Get file info to decide whether to use WebSocket or HTTP streaming
        let (file_size, mime_type) = nc.get_file_info(&path).await?;
        let is_video = STREAMING_MIME_TYPES.iter().any(|&m| mime_type.starts_with(m));
        let use_http_streaming = file_size > MAX_WS_FILE_SIZE || is_video;

        if use_http_streaming {
            // Generate JWT token and send HTTP streaming URL on layer 3
            log::info!("Using HTTP streaming for {} ({} bytes, {})", path, file_size, mime_type);

            let token = http_stream::generate_token(&jwt_secret, &path, &mime_type, &nc)?;
            let stream_url = format!("http://localhost:{}/stream/{}", server_port, token);

            // Format: expected-mime\nurl
            let layer3_content = format!("{}\n{}", mime_type, stream_url);
            let msg = encode_set_vertex_label_layer(
                action_id,
                vertex_id,
                3,
                "text/x-http-stream-url",
                layer3_content.as_bytes(),
            );
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            log::info!("Sent HTTP streaming URL for {} on layer 3", path);
        } else {
            // Small file - fetch full content via WebSocket
            match nc.download_with_type(&path, MAX_WS_FILE_SIZE).await {
                Ok((content, mime_type)) => {
                    log::info!("Loaded full content for {} ({} bytes, {})", path, content.len(), mime_type);
                    // Send full content on layer 2, replacing thumbnail
                    let msg = encode_set_vertex_label_layer(action_id, vertex_id, 2, &mime_type, &content);
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                }
                Err(err) => {
                    log::error!("Failed to load {}: {}", path, err);
                    let msg = encode_log_message(action_id, 500, vertex_id, &format!("Failed to load: {}", err));
                    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                }
            }
        }
    } else if let (Some(index), Some(nc)) = (&index, &nc) {
        // Check if this is a notes vertex
        if let Some(uuid) = notes::hash_to_uuid(index, vertex_id) {
            if let Some(vertex) = index.get_vertex(uuid) {
                log::info!("Loading notes vertex content: uuid={}, hash={}", uuid, vertex.content_hash);

                // Use CAS with caching for content-hash based content
                let content_result = if !vertex.content_hash.is_empty() {
                    let ext = notes::mime_to_extension(&vertex.mime);
                    // Use ContentStore with caching
                    let content_store = ContentStore::with_cache(nc.clone(), &nc.url, &nc.username);
                    content_store.get(&vertex.content_hash, ext).await
                } else if !vertex.file.is_empty() {
                    // Legacy file-based content (no caching)
                    nc.download(&vertex.file).await
                } else {
                    Err(anyhow!("Vertex has no content"))
                };

                match content_result {
                    Ok(content) => {
                        log::info!("Loaded notes vertex {} ({} bytes, {})", vertex_id, content.len(), vertex.mime);
                        // Send content as SetVertexLabel (layer 0)
                        let msg = encode_set_vertex_label(action_id, vertex_id, &vertex.mime, &content);
                        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    }
                    Err(e) => {
                        log::error!("Failed to load notes vertex {}: {}", vertex_id, e);
                        let msg = encode_log_message(action_id, 500, vertex_id, &format!("Failed to load: {}", e));
                        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    }
                }
            } else {
                log::warn!("Notes vertex {} found in hash but not in index", vertex_id);
                let msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found in index");
                write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
        } else {
            // Not a file entry and not a notes vertex - just acknowledge
            log::info!("ClickVertex: vertex {} is not a file or notes vertex", vertex_id);
            let msg = encode_log_message(action_id, 200, vertex_id, "Clicked");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }
    } else {
        // No index/nextcloud client - just acknowledge
        log::info!("ClickVertex: no index or nextcloud client available");
        let msg = encode_log_message(action_id, 200, vertex_id, "Clicked");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    Ok(())
}

/// Handle the gradesta://undo landmark - serve git history as a graph
pub async fn handle_undo_landmark<W>(
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
    action_id: u64,
    _landmark: &str,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (git_repo, identity) = {
        let s = state.lock().await;
        (s.git_undo_repo.clone(), s.identity.clone())
    };

    let identity = identity.unwrap_or_default();

    // Send context
    let landmark_uri = "gradesta://undo";
    let ctx_msg = encode_set_context(action_id, landmark_uri);
    write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send a "back to notes" portal vertex
    let portal_id = hash_string(&format!("undo:{}:portal", identity));
    let notes_landmark = format!("nextcloud://{}/notes/", identity);
    let portal_msg = encode_set_vertex_label(action_id, portal_id, "text/plain", b"< Back to Notes");
    write.send(AxumWsMessage::Binary(portal_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Portal has layer 1 with gradesta-url for navigation
    let portal_layer1 = encode_set_vertex_label_layer(action_id, portal_id, 1, "text/gradesta-url", notes_landmark.as_bytes());
    write.send(AxumWsMessage::Binary(portal_layer1)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Get commits from git repo
    let git_repo = match git_repo {
        Some(repo) => repo,
        None => {
            // No git repo - send empty placeholder
            let empty_id = hash_string(&format!("undo:{}:empty", identity));
            let empty_msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(no undo history - git repo not initialized)");
            write.send(AxumWsMessage::Binary(empty_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            let portal_edges = encode_set_edges(action_id, portal_id, 0, empty_id, 0, 0, 0, 0, 0);
            write.send(AxumWsMessage::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

            let empty_edges = encode_set_edges(action_id, empty_id, portal_id, 0, 0, 0, 0, 0, 0);
            write.send(AxumWsMessage::Binary(empty_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

            return Ok(());
        }
    };

    let repo = git_repo.lock().await;
    let commits = match repo.get_all_commits() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to get git commits: {}", e);
            let empty_id = hash_string(&format!("undo:{}:error", identity));
            let empty_msg = encode_set_vertex_label(action_id, empty_id, "text/plain",
                format!("(error loading history: {})", e).as_bytes());
            write.send(AxumWsMessage::Binary(empty_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
    };

    let head_oid = repo.head_commit();

    if commits.is_empty() || (commits.len() == 1 && commits[0].message == "Initial state") {
        // Empty or just initial commit - send placeholder
        let empty_id = hash_string(&format!("undo:{}:empty", identity));
        let empty_msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(no undo history yet)");
        write.send(AxumWsMessage::Binary(empty_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        let portal_edges = encode_set_edges(action_id, portal_id, 0, empty_id, 0, 0, 0, 0, 0);
        write.send(AxumWsMessage::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

        let empty_edges = encode_set_edges(action_id, empty_id, portal_id, 0, 0, 0, 0, 0, 0);
        write.send(AxumWsMessage::Binary(empty_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

        return Ok(());
    }

    // Build edges structure
    let mut vertex_edges: HashMap<u64, [u64; 6]> = HashMap::new();

    // Send all commits as vertices
    for commit in &commits {
        let vertex_id = git_undo::commit_to_vertex_hash(&commit.oid);
        let is_current = head_oid == Some(commit.oid);

        // Format timestamp
        let datetime = chrono::DateTime::from_timestamp(commit.timestamp, 0)
            .unwrap_or_else(|| chrono::Utc::now());
        let time_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        // Create label with description and timestamp
        let label = if is_current {
            format!("-> Currently at:\n{}\n{}", commit.message.trim(), time_str)
        } else {
            format!("{}\n{}", commit.message.trim(), time_str)
        };

        // Send vertex label
        let label_msg = encode_set_vertex_label(action_id, vertex_id, "text/plain", label.as_bytes());
        write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send metadata as layer 1 (JSON)
        let metadata = serde_json::json!({
            "commit_oid": commit.oid.to_string(),
            "timestamp": datetime.to_rfc3339(),
            "is_current": is_current,
            "author": commit.author,
        });
        let meta_msg = encode_set_vertex_label_layer(
            action_id, vertex_id, 1, "application/json",
            metadata.to_string().as_bytes()
        );
        write.send(AxumWsMessage::Binary(meta_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Build edges
        let children = repo.get_children(commit.oid, &commits);
        let edges = git_undo::build_commit_edges(commit, &commits, &children);
        vertex_edges.insert(vertex_id, edges);
    }

    // Find ALL root-level commits (children of Initial or no parent)
    let root_commits: Vec<&git_undo::CommitInfo> = commits.iter()
        .filter(|c| {
            c.message != "Initial state" &&
            (c.parent_oids.is_empty() ||
             c.parent_oids.iter().all(|p| {
                 commits.iter().any(|pc| pc.oid == *p && pc.message == "Initial state")
             }))
        })
        .collect();

    if !root_commits.is_empty() {
        // Connect portal east to first root commit
        let first_hash = git_undo::commit_to_vertex_hash(&root_commits[0].oid);
        let portal_edges = encode_set_edges(action_id, portal_id, 0, first_hash, 0, 0, 0, 0, 0);
        write.send(AxumWsMessage::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Point all root commits' west edge to portal (instead of Initial commit)
        // Sibling linking (north/south) is already handled by build_commit_edges
        for root_commit in &root_commits {
            let root_hash = git_undo::commit_to_vertex_hash(&root_commit.oid);
            if let Some(edges) = vertex_edges.get_mut(&root_hash) {
                edges[0] = portal_id;
            }
        }
    }

    // Send all edges
    for (vertex_id, edges) in &vertex_edges {
        let edges_msg = encode_set_edges(
            action_id, *vertex_id,
            edges[0], edges[1], edges[2], edges[3], edges[4], edges[5],
            0x01, // Only label editable (clicking triggers undo)
        );
        write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    }

    log::info!("Sent git undo history with {} commits", commits.len());
    Ok(())
}

/// Handle click on a git commit vertex - checkout that commit (undo/redo)
pub async fn handle_undo_click<W>(
    state: &Arc<Mutex<ConnState>>,
    write: &mut W,
    action_id: u64,
    vertex_id: u64,
) -> Result<bool>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (git_repo, nc) = {
        let s = state.lock().await;
        (s.git_undo_repo.clone(), s.nextcloud.clone())
    };

    let git_repo = match git_repo {
        Some(repo) => repo,
        None => {
            eprintln!("DEBUG: handle_undo_click - no git repo");
            return Ok(false); // No git repo, not an undo click
        }
    };

    // Get all commits to find the target
    let commits = {
        let repo = git_repo.lock().await;
        match repo.get_all_commits() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to get commits: {}", e);
                return Ok(false);
            }
        }
    };

    eprintln!("DEBUG: handle_undo_click - checking vertex {} in {} commits", vertex_id, commits.len());

    // Check if this vertex corresponds to a commit
    let target_oid = match git_undo::vertex_hash_to_oid(&commits, vertex_id) {
        Some(oid) => oid,
        None => {
            eprintln!("DEBUG: handle_undo_click - vertex {} not found in git history", vertex_id);
            return Ok(false); // Not a commit vertex
        }
    };

    // Find the commit info for logging
    let commit_info = commits.iter().find(|c| c.oid == target_oid);
    let commit_msg = commit_info.map(|c| c.message.clone()).unwrap_or_default();
    log::info!("Undo click: checking out commit {} ({})", target_oid, commit_msg.trim());
    eprintln!("DEBUG: Undo click - checking out commit {}", target_oid);

    // If not at a branch tip and about to make changes, ensure we're on a branch
    {
        let repo = git_repo.lock().await;
        if let Err(e) = repo.ensure_on_branch() {
            log::warn!("Failed to ensure on branch: {}", e);
        }
    }

    // Get the workdir path before checkout
    let workdir = {
        let repo = git_repo.lock().await;
        repo.workdir().map(|p| p.to_path_buf())
    };

    let workdir = match workdir {
        Some(w) => w,
        None => {
            let msg = encode_log_message(action_id, 500, vertex_id, "No git working directory");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(true);
        }
    };

    // Checkout the target commit
    {
        let repo = git_repo.lock().await;
        if let Err(e) = repo.checkout_commit(target_oid) {
            let msg = encode_log_message(action_id, 500, vertex_id, &format!("Checkout failed: {}", e));
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(true);
        }
    }

    // Load the index from the LOCAL git working directory (updated by checkout)
    let new_index = notes::NotesIndex::load_from_path(&workdir)?;

    log::info!("Checked out commit {}, loaded index with {} vertices from local git workdir",
        target_oid, new_index.vertices.len());

    // Upload the restored state to Nextcloud
    let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;
    if let Err(e) = new_index.save(&nc).await {
        log::warn!("Failed to save restored index to Nextcloud: {}", e);
    }

    // Upload all content files to Nextcloud
    for vertex in &new_index.vertices {
        let local_file = workdir.join(&vertex.file);
        if local_file.exists() {
            match std::fs::read(&local_file) {
                Ok(content) => {
                    if let Err(e) = nc.upload(&vertex.file, &content).await {
                        log::warn!("Failed to upload {} to Nextcloud: {}", vertex.file, e);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read local file {}: {}", vertex.file, e);
                }
            }
        }
    }

    // Sync git repo to Nextcloud (HEAD position changed)
    {
        let repo = git_repo.lock().await;
        let path = repo.local_path().to_path_buf();
        drop(repo);
        if let Err(e) = git_undo::sync_to_nextcloud(&path, &nc).await {
            log::warn!("Failed to sync git repo to Nextcloud after checkout: {}", e);
        }
    }

    // Update state with new index
    {
        let mut s = state.lock().await;
        s.index = Some(new_index.clone());
    }

    // Send success message
    let msg = encode_log_message(action_id, 200, vertex_id, "State restored");
    write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send updates for all vertices so browser's cache is updated
    let s = state.lock().await;
    let identity = s.identity.clone().unwrap_or_default();
    drop(s);

    let notes_landmark = format!("nextcloud://{}/notes/", identity);
    let ctx_msg = encode_set_context(action_id, &notes_landmark);
    write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send all vertices from the new index (load content from local workdir)
    for vertex in &new_index.vertices {
        let vertex_hash = uuid_to_hash(vertex.id);

        // Load content from local git working directory
        let local_file = workdir.join(&vertex.file);
        let content = match std::fs::read(&local_file) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("DEBUG: Failed to load content for {}: {}", vertex.id, e);
                format!("(failed to load: {})", e).into_bytes()
            }
        };

        // Send label
        let label_msg = encode_set_vertex_label(
            action_id,
            vertex_hash,
            &vertex.mime,
            &content
        );
        write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Build and send edges
        let edge_array = new_index.build_edge_array(vertex.id);
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
    }

    // Resend the undo tree view with updated "Currently at" marker
    handle_undo_landmark(state, write, action_id, "gradesta://undo").await?;

    Ok(true)
}
