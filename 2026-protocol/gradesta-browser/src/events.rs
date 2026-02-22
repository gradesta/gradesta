//! Server event processing
//!
//! Handles incoming server events and updates the graph and app state accordingly.

use std::thread;

use bevy::prelude::*;

use crate::elf_http;
use crate::graph::{GraphState, LayerContent};
use crate::media::{is_image_data, MediaCache};
use crate::network::{NetEventsTx, NetRx, ServerEvent, WsCommand, WsCommandTx};
use crate::state::{AppState, PendingIdentification};
use crate::video_player::VideoPlayer;
use crate::whisper;
use crate::ElfHttpTx;

/// Process all pending server events
pub fn ingest_server_events(
    mut graph: ResMut<GraphState>,
    mut app_state: ResMut<AppState>,
    rx: Res<NetRx>,
    net_tx: Res<NetEventsTx>,
    mut ws_cmd_tx: ResMut<WsCommandTx>,
    mut media_cache: ResMut<MediaCache>,
    elf_http_tx: Res<ElfHttpTx>,
) {
    while let Ok(event) = rx.0.try_recv() {
        match event {
            ServerEvent::Connected { base_url } => {
                app_state.connected = true;
                app_state.base_ws_url = Some(base_url);
                app_state.status = "Connected!".to_string();
            }
            ServerEvent::Disconnected { reason } => {
                app_state.connected = false;
                app_state.status = format!("Disconnected: {reason}");
                ws_cmd_tx.0 = None;
            }
            ServerEvent::Error { message } => {
                app_state.status = format!("Error: {message}");
            }
            ServerEvent::SetContext { uri } => {
                handle_set_context(&uri, &mut graph, &mut app_state);
            }
            ServerEvent::SetVertexLabel { vertex_id, layer, mime, data } => {
                handle_set_vertex_label(
                    vertex_id, layer, &mime, data,
                    &mut graph, &mut app_state, &net_tx, &mut media_cache,
                );
            }
            ServerEvent::SetEdges { vertex_id, edges, edit_mask } => {
                handle_set_edges(vertex_id, &edges, edit_mask, &mut graph, &mut app_state);
            }
            ServerEvent::Log { action_id, status, vertex_id, message } => {
                handle_log(
                    action_id, status, vertex_id, &message,
                    &mut graph, &mut app_state, &net_tx,
                );
            }
            ServerEvent::LocalSetVertexLabel { action_id, vertex_id, layer, mime, data } => {
                handle_local_set_vertex_label(
                    action_id, vertex_id, layer, &mime, data,
                    &mut graph, &ws_cmd_tx,
                );
            }
            ServerEvent::HttpStreamContentFetched { vertex_id, mime, data } => {
                handle_http_stream_content(
                    vertex_id, &mime, data,
                    &mut graph, &mut app_state, &mut media_cache,
                );
            }
            ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason } => {
                handle_request_identification(
                    action_id, nonce, timestamp, &reason,
                    &mut app_state,
                );
            }
            ServerEvent::IntroductionToken { action_id, token } => {
                // Look up the pending elf task and forward the token to the elf
                if let Some(task) = app_state.active_elf_tasks.get(&action_id) {
                    let elf_url = task.elf_url.clone();
                    let command = task.command.clone();

                    eprintln!("RECV IntroductionToken action={} token={}...",
                        action_id, &token[..std::cmp::min(8, token.len())]);
                    eprintln!("  elf_url={}", elf_url);
                    eprintln!("  command={}", command);

                    // Use the browser's own connection URL, not what the server advertises
                    // For elves via local service manager, translate localhost to Docker alias
                    let elf_server_ws_url = if let Some(ref base_url) = app_state.base_ws_url {
                        // Strip query params to get base WebSocket URL
                        let base = base_url.split('?').next().unwrap_or(base_url);
                        if elf_url.contains("localhost:19333") {
                            // Elf is in Docker, use Docker network alias
                            // Port 19333 on host maps to port 80 inside Docker
                            base.replace("//localhost:19333", "//gradesta-local-services")
                                .replace("//127.0.0.1:19333", "//gradesta-local-services")
                        } else {
                            base.to_string()
                        }
                    } else {
                        eprintln!("ERROR: No base_ws_url available for elf connection");
                        return;
                    };

                    eprintln!("Summoning elf {} with ws_url: {}", elf_url, elf_server_ws_url);

                    elf_http::summon_elf_async(
                        elf_url,
                        token,
                        elf_server_ws_url,
                        command,
                        std::collections::HashMap::new(),
                        elf_http_tx.0.clone(),
                    );
                } else {
                    eprintln!("ERROR: No pending elf task found for action {}", action_id);
                }
            }
        }
    }
}

fn handle_set_context(uri: &str, graph: &mut GraphState, app_state: &mut AppState) {
    graph.current_receiving_landmark = Some(uri.to_string());
    graph.landmark_vertices.entry(uri.to_string()).or_insert_with(Vec::new);
    let is_following = app_state.following_portal.as_ref()
        .map(|p| p == uri)
        .unwrap_or(false);
    if is_following {
        app_state.following_portal = None;
        graph.pending_jump_context = Some(uri.to_string());
    }
    graph.context_uri = Some(uri.to_string());
    app_state.status = format!("Viewing: {uri}");
    let is_initial = app_state.landmark_history.is_empty();
    if is_following || is_initial {
        app_state.landmark_history.retain(|l| l != uri);
        app_state.landmark_history.push(uri.to_string());
        while app_state.landmark_history.len() > app_state.max_landmark_history {
            app_state.landmark_history.remove(0);
        }
    }
    // Update only the landmark input (server stays the same)
    app_state.landmark_input = uri.to_string();
}

fn handle_set_vertex_label(
    vertex_id: u64,
    layer: u32,
    mime: &str,
    data: Vec<u8>,
    graph: &mut GraphState,
    app_state: &mut AppState,
    net_tx: &NetEventsTx,
    media_cache: &mut MediaCache,
) {
    let entry = graph.vertices.entry(vertex_id).or_default();
    entry.id = vertex_id;

    // Special handling for layer 3 HTTP stream URLs
    if layer == 3 && mime == "text/x-http-stream-url" {
        if let Ok(content) = String::from_utf8(data.clone()) {
            if let Some((expected_mime, url)) = content.split_once('\n') {
                let url = url.trim().to_string();
                let expected_mime = expected_mime.trim().to_string();
                let v_id = vertex_id;
                let events_tx = net_tx.0.clone();

                thread::spawn(move || {
                    eprintln!("HTTP Fetch: Fetching {} from {}", expected_mime, url);
                    match reqwest::blocking::get(&url) {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.bytes() {
                                    Ok(bytes) => {
                                        eprintln!("HTTP Fetch: Got {} bytes for vertex {}", bytes.len(), v_id);
                                        let _ = events_tx.send(ServerEvent::HttpStreamContentFetched {
                                            vertex_id: v_id,
                                            mime: expected_mime,
                                            data: bytes.to_vec(),
                                        });
                                    }
                                    Err(e) => eprintln!("HTTP Fetch: Failed to read body: {}", e),
                                }
                            } else {
                                eprintln!("HTTP Fetch: HTTP error: {}", response.status());
                            }
                        }
                        Err(e) => eprintln!("HTTP Fetch: Failed to fetch: {}", e),
                    }
                });
            }
        }
        entry.layers.insert(layer, LayerContent {
            mime: mime.to_string(),
            data,
        });
        return;
    }

    // Invalidate cached texture/media when content changes
    if mime.starts_with("image/") || is_image_data(&data) {
        media_cache.textures.remove(&vertex_id);
        media_cache.animated_gifs.remove(&vertex_id);
    }

    // Store in appropriate layer
    if layer == 0 {
        entry.label = data;
        entry.mime = Some(mime.to_string());
    } else {
        entry.layers.insert(layer, LayerContent {
            mime: mime.to_string(),
            data,
        });
    }

    // Track which landmark this vertex belongs to
    if let Some(landmark) = graph.current_receiving_landmark.clone() {
        if let Some(vertices) = graph.landmark_vertices.get_mut(&landmark) {
            if !vertices.contains(&vertex_id) {
                vertices.push(vertex_id);
            }
        }
    }

    // If we're waiting to jump to a new context, and this vertex is NOT a portal, jump to it
    if layer == 0 && graph.pending_jump_context.is_some() && mime != "text/gradesta-url" {
        if let Some(current) = app_state.current_vertex {
            app_state.history.push(current);
        }
        app_state.current_vertex = Some(vertex_id);
        graph.pending_jump_context = None;
    } else if layer == 0 && app_state.current_vertex.is_none() {
        app_state.current_vertex = Some(vertex_id);
    }
}

fn handle_set_edges(
    vertex_id: u64,
    edges: &[u64; 6],
    edit_mask: u8,
    graph: &mut GraphState,
    app_state: &mut AppState,
) {
    let vertex_exists = graph.vertices.contains_key(&vertex_id);
    let is_deletion = vertex_exists && edges.iter().all(|&e| e == 0) && edit_mask == 0;

    if is_deletion {
        graph.vertices.remove(&vertex_id);
        app_state.history.retain(|&id| id != vertex_id);
        for vertices in graph.landmark_vertices.values_mut() {
            vertices.retain(|&id| id != vertex_id);
        }
        if app_state.current_vertex == Some(vertex_id) {
            app_state.current_vertex = app_state.history.pop();
        }
        eprintln!("Vertex {} deleted from local graph", vertex_id);
    } else {
        let entry = graph.vertices.entry(vertex_id).or_default();
        entry.id = vertex_id;
        entry.edit_mask = edit_mask;
        for (i, &new_edge) in edges.iter().enumerate() {
            if new_edge != u64::MAX {
                entry.edges[i] = new_edge;
            }
        }
        if app_state.current_vertex.is_none() {
            app_state.current_vertex = Some(vertex_id);
        }
    }
}

fn handle_log(
    action_id: u64,
    status: u32,
    vertex_id: u64,
    message: &str,
    graph: &mut GraphState,
    app_state: &mut AppState,
    net_tx: &NetEventsTx,
) {
    app_state.status = format!("Server [{}]: {}", status, message);
    if status == 200 {
        eprintln!("Edit acknowledged: action={} vertex={} status={}", action_id, vertex_id, status);

        if let Some(pending) = app_state.pending_creations.remove(&action_id) {
            if vertex_id != 0 {
                if let Some(current) = app_state.current_vertex {
                    if current != vertex_id {
                        app_state.history.push(current);
                        app_state.current_vertex = Some(vertex_id);
                        eprintln!("Navigating to newly created vertex {}", vertex_id);
                    }
                } else {
                    app_state.current_vertex = Some(vertex_id);
                }
            }

            let entry = graph.vertices.entry(vertex_id).or_default();
            entry.id = vertex_id;
            entry.label = pending.data.clone();
            entry.mime = Some(pending.mime.clone());

            if pending.mime.starts_with("audio/") {
                app_state.skip_autoplay_vertex = Some(vertex_id);

                eprintln!("Starting async transcription for vertex {} (action={})", vertex_id, action_id);
                let event_tx = net_tx.0.clone();
                let target_vertex = vertex_id;
                let transcript_action_id = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                thread::spawn(move || {
                    if whisper::is_model_available() {
                        match whisper::transcribe(&pending.samples, pending.sample_rate) {
                            Ok(text) => {
                                eprintln!("Transcription complete: {}", text);
                                let _ = event_tx.send(ServerEvent::LocalSetVertexLabel {
                                    action_id: transcript_action_id,
                                    vertex_id: target_vertex,
                                    layer: 1,
                                    mime: "text/plain".to_string(),
                                    data: text.into_bytes(),
                                });
                            }
                            Err(e) => {
                                eprintln!("Transcription failed: {}", e);
                            }
                        }
                    } else {
                        eprintln!("Whisper model not available, skipping transcription");
                    }
                });
            }
        }
    } else {
        eprintln!("Edit failed: action={} vertex={} status={} msg={}", action_id, vertex_id, status, message);
        app_state.pending_creations.remove(&action_id);
    }
}

fn handle_local_set_vertex_label(
    action_id: u64,
    vertex_id: u64,
    layer: u32,
    mime: &str,
    data: Vec<u8>,
    graph: &mut GraphState,
    ws_cmd_tx: &WsCommandTx,
) {
    let entry = graph.vertices.entry(vertex_id).or_default();
    entry.id = vertex_id;
    if layer == 0 {
        entry.label = data.clone();
        entry.mime = Some(mime.to_string());
    } else {
        entry.layers.insert(layer, LayerContent {
            mime: mime.to_string(),
            data: data.clone(),
        });
    }

    if let Some(ref tx) = ws_cmd_tx.0 {
        let _ = tx.send(WsCommand::SetVertexLabel {
            action_id,
            vertex_id,
            layer,
            mime: mime.to_string(),
            data,
        });
    }
}

fn handle_http_stream_content(
    vertex_id: u64,
    mime: &str,
    data: Vec<u8>,
    graph: &mut GraphState,
    app_state: &mut AppState,
    media_cache: &mut MediaCache,
) {
    // For MP4 videos, use native video player
    if mime == "video/mp4" {
        eprintln!("HTTP Fetch: Got MP4 video ({} bytes), starting native player", data.len());

        match VideoPlayer::new(data.clone()) {
            Ok(player) => {
                media_cache.video_players.insert(vertex_id, player);
                app_state.video_modal_vertex_id = Some(vertex_id);
                app_state.show_video_modal = true;
                eprintln!("HTTP Fetch: Video player started for vertex {}", vertex_id);
            }
            Err(e) => {
                eprintln!("HTTP Fetch: Failed to create video player: {}", e);
                app_state.status = format!("Video error: {}", e);
            }
        }
        return;
    }

    // For other video formats, fall back to external player
    if mime.starts_with("video/") {
        eprintln!("HTTP Fetch: Got video {} ({} bytes), launching external player", mime, data.len());

        let ext = match mime {
            "video/webm" => "webm",
            "video/quicktime" => "mov",
            "video/x-matroska" => "mkv",
            "video/ogg" => "ogv",
            _ => "mp4",
        };

        match tempfile::Builder::new()
            .prefix("gradesta-video-")
            .suffix(&format!(".{}", ext))
            .tempfile()
        {
            Ok(mut temp) => {
                use std::io::Write;
                if let Err(e) = temp.write_all(&data) {
                    eprintln!("HTTP Fetch: Failed to write temp file: {}", e);
                } else {
                    let path = temp.path().to_owned();
                    let (file, file_path) = temp.keep().unwrap_or_else(|e| {
                        eprintln!("Failed to keep temp file: {}", e);
                        (std::fs::File::create(&path).unwrap(), path.clone())
                    });
                    drop(file);

                    eprintln!("HTTP Fetch: Launching mpv for {}", file_path.display());
                    if let Err(e) = std::process::Command::new("mpv")
                        .arg(&file_path)
                        .spawn()
                    {
                        eprintln!("HTTP Fetch: Failed to launch mpv: {}", e);
                        if let Err(e2) = open::that(&file_path) {
                            eprintln!("HTTP Fetch: Failed to open with xdg-open: {}", e2);
                        }
                    }
                }
            }
            Err(e) => eprintln!("HTTP Fetch: Failed to create temp file: {}", e),
        }
        return;
    }

    // Store fetched content in layer 2
    let entry = graph.vertices.entry(vertex_id).or_default();
    entry.id = vertex_id;

    if mime.starts_with("image/") || is_image_data(&data) {
        media_cache.textures.remove(&vertex_id);
        media_cache.animated_gifs.remove(&vertex_id);
    }

    entry.layers.insert(2, LayerContent {
        mime: mime.to_string(),
        data,
    });

    eprintln!("HTTP Fetch: Stored {} content in layer 2 for vertex {}", mime, vertex_id);
}

fn handle_request_identification(
    action_id: u64,
    nonce: [u8; 32],
    timestamp: u64,
    reason: &str,
    app_state: &mut AppState,
) {
    let server_url = app_state.base_ws_url.clone().unwrap_or_default();

    let remembered_identity = app_state.identity_config.identities.iter()
        .position(|id| id.remembered_servers.contains(&server_url));

    if let Some(idx) = remembered_identity {
        app_state.selected_identity_index = idx;
        app_state.pending_identification = Some(PendingIdentification {
            action_id,
            nonce,
            timestamp,
            reason: reason.to_string(),
            server_url,
        });
        app_state.status = format!("Auto-identifying as {}...",
            app_state.identity_config.identities[idx].display_name);
    } else {
        app_state.pending_identification = Some(PendingIdentification {
            action_id,
            nonce,
            timestamp,
            reason: reason.to_string(),
            server_url,
        });
        app_state.status = "Server requests identification".to_string();
    }
}
