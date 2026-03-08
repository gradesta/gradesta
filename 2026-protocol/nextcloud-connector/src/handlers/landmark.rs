//! Landmark navigation handlers

use std::sync::Arc;
use tokio::sync::Mutex;

use anyhow::{anyhow, Result};
use axum::extract::ws::Message as AxumWsMessage;
use futures_util::SinkExt;

use crate::calendar;
use crate::connection_manager::SharedConnectionManager;
use crate::files;
use crate::notes::{self, uuid_to_hash};
use crate::protocol::*;
use crate::router;
use crate::state::ConnState;
use crate::utils::{hash_string, router_hash};

use super::click::handle_undo_landmark;

/// Maximum vertices in a chain before creating a landmark boundary
pub const MAX_CHAIN_LENGTH: usize = 20;

pub async fn handle_watch_landmark<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    connection_manager: &SharedConnectionManager,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, landmark) = parse_watch_landmark(data)?;
    log::info!("Watch landmark: {} (action={})", landmark, action_id);
    eprintln!("DEBUG: Received landmark request: '{}' (len={})", landmark, landmark.len());

    let (identity, conn_id) = {
        let s = state.lock().await;
        (s.identity.clone().unwrap_or_default(), s.conn_id)
    };

    // Parse landmark URL to route to appropriate handler
    // Format: nextcloud://{identity}/ - router
    // Format: nextcloud://{identity}/notes/ - notes root
    // Format: nextcloud://{identity}/notes/{hash} - specific note
    // Format: nextcloud://{identity}/calendar/ - calendar root
    // Format: nextcloud://{identity}/calendar/{year}/ - year
    // Format: nextcloud://{identity}/calendar/{year}/{month}/ - month
    // Format: nextcloud://{identity}/calendar/{year}/{month}/{day}/ - day
    // Format: nextcloud://{identity}/files/ - files root
    // Format: nextcloud://{identity}/files/{path} - directory or file

    // Extract path after identity
    // The identity itself may contain slashes (e.g., https://server/s/token)
    // So we need to match against the known identity to find where it ends
    let path = if let Some(stripped) = landmark.strip_prefix("nextcloud://") {
        // Try to strip the identity prefix to get the path
        if let Some(after_identity) = stripped.strip_prefix(&identity) {
            after_identity.trim_start_matches('/')
        } else {
            // Identity doesn't match - this shouldn't happen but handle gracefully
            // Try to find common path segments
            if stripped.ends_with("/notes/") || stripped.contains("/notes/") {
                if let Some(idx) = stripped.rfind("/notes/") {
                    &stripped[idx + 1..]
                } else {
                    "notes/"
                }
            } else if stripped.ends_with("/calendar/") || stripped.contains("/calendar/") {
                if let Some(idx) = stripped.rfind("/calendar/") {
                    &stripped[idx + 1..]
                } else {
                    "calendar/"
                }
            } else if stripped.ends_with("/") {
                "" // Router root
            } else {
                stripped
            }
        }
    } else if let Some(stripped) = landmark.strip_prefix("notes://") {
        // Legacy notes:// URLs - treat as notes
        if let Some(idx) = stripped.find('/') {
            let rest = &stripped[idx + 1..];
            if rest.is_empty() {
                "notes/"
            } else {
                // Wrap in notes/ prefix for legacy support
                return handle_notes_landmark(state, connection_manager, conn_id, write, action_id, rest).await;
            }
        } else {
            "notes/"
        }
    } else if landmark.starts_with("gradesta://undo") {
        // Undo tree landmark - serve the undo history as a graph
        log::info!("Undo tree landmark requested");
        eprintln!("DEBUG: Handling gradesta://undo landmark");
        return handle_undo_landmark(state, write, action_id, &landmark).await;
    } else if let Some(vertex_hash) = landmark.strip_prefix("vertex/") {
        // Direct vertex request - try to find and load it
        // This is used by the browser when preloading unknown vertices
        log::info!("Direct vertex request: {}", vertex_hash);
        if let Ok(hash) = vertex_hash.parse::<u64>() {
            // Try to find this vertex - could be a note or a calendar vertex
            // First check if it's a note
            let index = {
                let s = state.lock().await;
                s.index.clone()
            };
            if let Some(index) = &index {
                if notes::hash_to_uuid(index, hash).is_some() {
                    // It's a note vertex
                    return handle_notes_landmark(state, connection_manager, conn_id, write, action_id, vertex_hash).await;
                }
            }
            // If not found in notes, it might be a calendar vertex
            // Calendar vertices are dynamically generated, so we need to figure out what it is
            // For now, just return an empty response - the calendar doesn't support direct vertex loading yet
            log::info!("Vertex {} not found in notes, checking if calendar vertex", hash);
            // Return the calendar root as a fallback so at least something loads
            return calendar::handle_landmark(state, write, action_id, "").await;
        }
        "" // Will fall through to router
    } else {
        ""
    };

    log::info!("Routing path: '{}'", path);
    eprintln!("DEBUG: Final routing path: '{}' (landmark was '{}')", path, landmark);

    match path {
        "" => {
            // Router root
            router::send_router(&identity, write, action_id).await
        }
        p if p.starts_with("notes/") || p.starts_with("notes") => {
            let notes_path = p.strip_prefix("notes/").or_else(|| p.strip_prefix("notes")).unwrap_or("");
            handle_notes_landmark(state, connection_manager, conn_id, write, action_id, notes_path).await
        }
        p if p.starts_with("calendar/") || p.starts_with("calendar") => {
            let calendar_path = p.strip_prefix("calendar/").or_else(|| p.strip_prefix("calendar")).unwrap_or("");
            calendar::handle_landmark(state, write, action_id, calendar_path).await
        }
        p if p.starts_with("files/") || p.starts_with("files") => {
            // Legacy: files/ prefix routes to directory listing
            let files_path = p.strip_prefix("files/").or_else(|| p.strip_prefix("files")).unwrap_or("");
            files::handle_landmark(state, write, action_id, files_path).await
        }
        p if p.starts_with("file/") => {
            // Legacy: file/ prefix routes to file view
            let file_path = p.strip_prefix("file/").unwrap_or("");
            files::handle_file_view(state, write, action_id, file_path).await
        }
        p if p.ends_with('/') => {
            // New scheme: paths ending in / are directories
            // Strip leading and trailing / to get the directory path
            let dir_path = p.trim_matches('/');
            files::handle_landmark(state, write, action_id, dir_path).await
        }
        p if !p.is_empty() => {
            // New scheme: paths not ending in / are files (but must not be empty)
            files::handle_file_view(state, write, action_id, p).await
        }
        _ => {
            log::warn!("Unknown landmark path: {}", path);
            Err(anyhow!("Unknown landmark path: {}", path))
        }
    }
}

/// Handle notes-specific landmarks
pub async fn handle_notes_landmark<W>(
    state: &Arc<Mutex<ConnState>>,
    connection_manager: &SharedConnectionManager,
    conn_id: u64,
    write: &mut W,
    action_id: u64,
    notes_path: &str,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Parse vertex hash from path if present
    let start_vertex = if !notes_path.is_empty() {
        // Try to parse as vertex hash
        if let Ok(hash) = notes_path.parse::<u64>() {
            let index = {
                let s = state.lock().await;
                s.index.clone()
            };
            if let Some(index) = index {
                notes::hash_to_uuid(&index, hash)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    log::info!("Notes landmark - start vertex: {:?}", start_vertex);
    send_notes_from_vertex(state, connection_manager, conn_id, write, action_id, start_vertex).await
}

/// Send notes starting from a specific vertex (or root if None)
/// Only sends vertices within landmark boundaries (forks or every MAX_CHAIN_LENGTH vertices)
pub async fn send_notes_from_vertex<W>(
    state: &Arc<Mutex<ConnState>>,
    connection_manager: &SharedConnectionManager,
    conn_id: u64,
    write: &mut W,
    action_id: u64,
    start_vertex: Option<uuid::Uuid>,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (identity, index, nc) = {
        let s = state.lock().await;
        (
            s.identity.clone().unwrap_or_default(),
            s.index.clone(),
            s.nextcloud.clone(),
        )
    };

    let index = index.ok_or_else(|| anyhow!("No index loaded"))?;
    let nc = nc.ok_or_else(|| anyhow!("No Nextcloud client"))?;

    // Send context
    let landmark = if let Some(v) = start_vertex {
        format!("nextcloud://{}/notes/{}", identity, uuid_to_hash(v))
    } else {
        format!("nextcloud://{}/notes/", identity)
    };
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Send notes portal vertex - allows navigation back to home screen
    let portal_id = router_hash(&identity, "notes-portal");
    let router_id = router_hash(&identity, "main");
    let calendar_portal_id = router_hash(&identity, "calendar-portal");
    let portal_msg = encode_set_vertex_label(action_id, portal_id, "text/plain", b"Notes");
    write.send(AxumWsMessage::Binary(portal_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    // Get root vertex ID for portal's east edge
    let root_vertex_id = if index.vertices.is_empty() {
        hash_string(&format!("empty:{}", identity))
    } else {
        index.get_root_vertex().map(uuid_to_hash).unwrap_or(0)
    };

    // Portal edges: north to router, south to calendar portal (vertical menu), east to root note
    let portal_edges = encode_set_edges(action_id, portal_id, 0, root_vertex_id, router_id, calendar_portal_id, 0, 0, 0);
    write.send(AxumWsMessage::Binary(portal_edges)).await.map_err(|e| anyhow!("{:?}", e))?;

    if index.vertices.is_empty() {
        // Send empty placeholder
        let empty_id = hash_string(&format!("empty:{}", identity));
        let msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(no notes yet - press 'i' to create one)");
        write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Edges: west to portal, full editability
        let edges = encode_set_edges(action_id, empty_id, portal_id, 0, 0, 0, 0, 0, 0x7F);
        write.send(AxumWsMessage::Binary(edges)).await.map_err(|e| anyhow!("{:?}", e))?;
    } else {
        // Get the starting vertex (specified or root)
        let start = start_vertex.or_else(|| index.get_root_vertex());
        let start = match start {
            Some(v) => v,
            None => {
                log::warn!("No starting vertex found");
                return Ok(());
            }
        };

        // Get the actual root vertex (first by creation time)
        let root_uuid = index.get_root_vertex();

        // Get vertices within landmark boundaries
        let (vertices_to_send, landmark_vertices) = index.get_vertices_within_landmark(start, MAX_CHAIN_LENGTH);

        log::info!(
            "Sending {} vertices (bounded by {} landmarks) from start {:?}",
            vertices_to_send.len(),
            landmark_vertices.len(),
            start
        );

        // Collect vertex IDs for registering watchers
        let mut vertex_ids_to_watch: Vec<u64> = Vec::new();

        // Send each vertex
        for vertex_uuid in &vertices_to_send {
            let vertex = match index.get_vertex(*vertex_uuid) {
                Some(v) => v,
                None => continue,
            };
            let vertex_id = uuid_to_hash(vertex.id);
            let is_landmark = landmark_vertices.contains(vertex_uuid);
            let is_root = root_uuid == Some(vertex.id);

            // Track vertex for watcher registration
            vertex_ids_to_watch.push(vertex_id);

            // Load actual content for layer 0
            let (content, mime) = match nc.download(&vertex.file).await {
                Ok(data) => (data, vertex.mime.clone()),
                Err(e) => {
                    log::warn!("Failed to load {}: {}", vertex.file, e);
                    (format!("(failed to load: {})", e).into_bytes(), "text/plain".to_string())
                }
            };

            // Send vertex label (layer 0 - actual content)
            let label_msg = encode_set_vertex_label(action_id, vertex_id, &mime, &content);
            write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            // Send transcript as layer 1 if available
            if let Some(ref transcript) = vertex.transcript {
                let transcript_msg = encode_set_vertex_label_layer(
                    action_id, vertex_id, 1, "text/plain", transcript.as_bytes());
                write.send(AxumWsMessage::Binary(transcript_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }

            // Build edges - for landmark boundaries, replace edges going "outside"
            // with portal URLs that the client can follow to load more
            let mut edge_array = index.build_edge_array(vertex.id);

            // Root vertex connects west to the notes portal
            if is_root && edge_array[0] == 0 {
                edge_array[0] = portal_id;
            }

            if is_landmark {
                // For landmark vertices, edges to unloaded vertices become portals
                // The client will see the vertex hash but won't have the data,
                // causing it to request a WatchLandmark for that vertex
                // We keep the hash but the client knows to request more data
                log::info!("Vertex {} is a landmark boundary", vertex_id);
            }

            // Send edges with full editability
            let edges_msg = encode_set_edges(
                action_id,
                vertex_id,
                edge_array[0],
                edge_array[1],
                edge_array[2],
                edge_array[3],
                edge_array[4],
                edge_array[5],
                0x7F, // All edges and label editable
            );
            write.send(AxumWsMessage::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }

        // Register this connection as watching all sent vertices
        if !vertex_ids_to_watch.is_empty() {
            let mut cm = connection_manager.lock().await;
            cm.add_vertex_watchers(conn_id, &vertex_ids_to_watch);
            log::info!("Registered conn_id={} as watcher for {} vertices", conn_id, vertex_ids_to_watch.len());
        }
    }

    log::info!("Sent {} notes (action={})", index.vertices.len(), action_id);
    Ok(())
}

/// Forward a vertex update from an elf to the originating browser and all watchers
pub async fn forward_vertex_update_to_browser(
    state: &Arc<Mutex<ConnState>>,
    connection_manager: &SharedConnectionManager,
    data: &[u8],
) {
    // Get the browser connection ID from the elf connection
    let browser_conn_id = {
        let s = state.lock().await;
        s.elf_connection.as_ref().map(|ec| ec.browser_conn_id)
    };

    // Parse the message to construct proper server message
    let msg_type = if !data.is_empty() { data[0] } else { return };

    // Build appropriate server message based on message type
    let (server_msg, vertex_id) = match msg_type {
        MSG_CLIENT_SET_VERTEX_LABEL => {
            // Parse: [type:1][action_id:8][vertex_id:8][layer:4][mime\0][content]
            if let Ok((action_id, vertex_id, layer, mime, content)) = parse_client_set_vertex_label(data) {
                let msg = encode_set_vertex_label_layer(action_id, vertex_id, layer, &mime, &content);
                (msg, vertex_id)
            } else {
                return;
            }
        }
        MSG_CLIENT_CREATE_VERTEX => {
            // CreateVertex doesn't directly map to a server message for forwarding
            // The server already sends SetVertexLabel and SetEdges responses
            // We mainly need to ensure the browser gets notified
            // Parse vertex_id from the response we just sent (action_id tells us)
            if data.len() >= 17 {
                let vertex_id = u64::from_be_bytes(data[9..17].try_into().unwrap_or([0; 8]));
                // For create, we need to notify but the actual data will come from
                // the handle_create_vertex response
                (Vec::new(), vertex_id)
            } else {
                return;
            }
        }
        MSG_CLIENT_DELETE_VERTEX => {
            // Parse: [type:1][action_id:8][vertex_id:8]
            if let Ok((action_id, vertex_id)) = parse_client_delete_vertex(data) {
                // Send SetEdges with all zeros to indicate deletion
                let msg = encode_set_edges(action_id, vertex_id, 0, 0, 0, 0, 0, 0, 0);
                (msg, vertex_id)
            } else {
                return;
            }
        }
        _ => return,
    };

    // Skip if no message to send (CreateVertex case)
    if server_msg.is_empty() {
        return;
    }

    let cm = connection_manager.lock().await;

    // Forward to originating browser
    if let Some(conn_id) = browser_conn_id {
        if cm.send_to(conn_id, server_msg.clone()).is_ok() {
            log::info!("Forwarded SetVertexLabel to browser conn_id={}", conn_id);
        } else {
            log::warn!("Failed to forward vertex update to browser conn_id={}", conn_id);
        }
    }

    // Broadcast to all watchers of this vertex (excluding the originating browser)
    let broadcast_count = cm.broadcast_to_vertex_watchers(vertex_id, &server_msg, browser_conn_id);
    if broadcast_count > 0 {
        log::info!("Broadcast vertex {} update to {} watchers", vertex_id, broadcast_count);
    }
}
