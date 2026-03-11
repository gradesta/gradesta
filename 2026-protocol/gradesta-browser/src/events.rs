//! Server event processing
//!
//! Handles incoming server events and updates the graph and app state accordingly.

use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

/// Get current timestamp with milliseconds for logging
fn ts() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() % 86400; // Time of day in seconds
    let millis = now.subsec_millis();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

use crate::audio::{AudioProcessingChannel, AudioProcessingResult};
use crate::elf_http;
use crate::graph::{GraphState, LayerContent};
use crate::media::{is_image_data, MediaCache};
use crate::network::{NetEventsTx, NetRx, ServerEvent, WsCommand, WsCommandTx};
use crate::state::{AppState, InputMode, PendingAudioStatus, PendingIdentification, PendingVertexCreation, TranscriptionMode};
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
            ServerEvent::SetVertexPreview { vertex_id, layer, total_length, mime, preview } => {
                handle_set_vertex_preview(
                    vertex_id, layer, total_length, &mime, preview,
                    &mut graph, &mut app_state,
                );
            }
            ServerEvent::SetVertexContent { vertex_id, layer, mime, data } => {
                handle_set_vertex_content(
                    vertex_id, layer, &mime, data,
                    &mut graph, &mut app_state, &mut media_cache,
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

                    eprintln!("[{}] RECV IntroductionToken action={} token={}...",
                        ts(), action_id, &token[..std::cmp::min(8, token.len())]);
                    eprintln!("[{}]   elf_url={}", ts(), elf_url);
                    eprintln!("[{}]   command={}", ts(), command);

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
                        eprintln!("[{}] ERROR: No base_ws_url available for elf connection", ts());
                        return;
                    };

                    eprintln!("[{}] Summoning elf {} with ws_url: {}", ts(), elf_url, elf_server_ws_url);

                    elf_http::summon_elf_async(
                        elf_url,
                        token,
                        elf_server_ws_url,
                        command,
                        std::collections::HashMap::new(),
                        elf_http_tx.0.clone(),
                    );
                } else {
                    eprintln!("[{}] ERROR: No pending elf task found for action {}", ts(), action_id);
                }
            }
        }
    }
}

fn handle_set_vertex_preview(
    vertex_id: u64,
    layer: u32,
    total_length: u32,
    mime: &str,
    preview: Vec<u8>,
    graph: &mut GraphState,
    app_state: &mut AppState,
) {
    let entry = graph.vertices.entry(vertex_id).or_default();
    entry.id = vertex_id;

    // u32::MAX means "unknown size, needs to be loaded"
    // Empty preview with u32::MAX means server didn't download content yet
    let needs_loading = total_length == u32::MAX || (total_length > 255 && preview.len() < total_length as usize);

    // Unified handling for all layers - don't overwrite loaded content with preview
    let layer_loaded = entry.layer_loaded.get(&layer).copied().unwrap_or(false);
    if !layer_loaded {
        entry.layers.insert(layer, LayerContent {
            mime: mime.to_string(),
            data: preview,
        });
        entry.layer_lengths.insert(layer, total_length);
        entry.layer_loaded.insert(layer, !needs_loading);
    } else {
        // Even if content is loaded, update the mime type (might have been unknown)
        if let Some(layer_content) = entry.layers.get_mut(&layer) {
            if layer_content.mime.is_empty() {
                layer_content.mime = mime.to_string();
            }
        }
    }

    // Track which landmark this vertex belongs to
    graph.landmark_mgr.track_vertex(vertex_id);

    // Handle initial navigation - jump to first non-portal vertex
    if layer == 0 && graph.pending_jump_context.is_some() && mime != "text/gradesta-url" {
        if let Some(current) = app_state.current_vertex {
            app_state.history.push(current);
        }
        app_state.current_vertex = Some(vertex_id);
        graph.pending_jump_context = None;
        app_state.loading_portal_vertex = None;
        app_state.loading_portal_cell = None;
    } else if layer == 0 && app_state.current_vertex.is_none() {
        app_state.current_vertex = Some(vertex_id);
    }
}

fn handle_set_vertex_content(
    vertex_id: u64,
    layer: u32,
    mime: &str,
    data: Vec<u8>,
    graph: &mut GraphState,
    app_state: &mut AppState,
    media_cache: &mut MediaCache,
) {
    let entry = graph.vertices.entry(vertex_id).or_default();
    entry.id = vertex_id;

    // Invalidate cached texture/media when content changes
    if mime.starts_with("image/") || is_image_data(&data) {
        media_cache.textures.remove(&vertex_id);
        media_cache.animated_gifs.remove(&vertex_id);
    }

    // Unified handling for all layers
    let data_len = data.len() as u32;
    entry.layers.insert(layer, LayerContent {
        mime: mime.to_string(),
        data,
    });
    entry.layer_lengths.insert(layer, data_len);
    entry.layer_loaded.insert(layer, true);

    // Mark the pending request as complete (content received)
    let key = (vertex_id, layer);
    app_state.pending_content_requests.remove(&key);
    // Add to active watches since we're now watching this content
    app_state.active_content_watches.insert(key);

    // Track which landmark this vertex belongs to
    graph.landmark_mgr.track_vertex(vertex_id);
}

fn handle_set_context(uri: &str, graph: &mut GraphState, app_state: &mut AppState) {
    // Update landmark manager state
    graph.landmark_mgr.on_set_context(uri);

    // Check if we should auto-navigate to this landmark
    let is_following = graph.landmark_mgr.should_follow(uri);
    if is_following {
        graph.landmark_mgr.clear_follow(uri);
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
                    eprintln!("[{}] HTTP Fetch: Fetching {} from {}", ts(), expected_mime, url);
                    match reqwest::blocking::get(&url) {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.bytes() {
                                    Ok(bytes) => {
                                        let ts_str = {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default();
                                            let secs = now.as_secs() % 86400;
                                            let millis = now.subsec_millis();
                                            format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                        };
                                        eprintln!("[{}] HTTP Fetch: Got {} bytes for vertex {}", ts_str, bytes.len(), v_id);
                                        let _ = events_tx.send(ServerEvent::HttpStreamContentFetched {
                                            vertex_id: v_id,
                                            mime: expected_mime,
                                            data: bytes.to_vec(),
                                        });
                                    }
                                    Err(e) => {
                                        let ts_str = {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default();
                                            let secs = now.as_secs() % 86400;
                                            let millis = now.subsec_millis();
                                            format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                        };
                                        eprintln!("[{}] HTTP Fetch: Failed to read body: {}", ts_str, e);
                                    }
                                }
                            } else {
                                let ts_str = {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default();
                                    let secs = now.as_secs() % 86400;
                                    let millis = now.subsec_millis();
                                    format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                };
                                eprintln!("[{}] HTTP Fetch: HTTP error: {}", ts_str, response.status());
                            }
                        }
                        Err(e) => {
                            let ts_str = {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default();
                                let secs = now.as_secs() % 86400;
                                let millis = now.subsec_millis();
                                format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                            };
                            eprintln!("[{}] HTTP Fetch: Failed to fetch: {}", ts_str, e);
                        }
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

    // Unified handling for all layers
    let data_len = data.len() as u32;
    entry.layers.insert(layer, LayerContent {
        mime: mime.to_string(),
        data,
    });
    entry.layer_lengths.insert(layer, data_len);
    entry.layer_loaded.insert(layer, true);

    // Track which landmark this vertex belongs to
    graph.landmark_mgr.track_vertex(vertex_id);

    // If we're waiting to jump to a new context, and this vertex is NOT a portal, jump to it
    if layer == 0 && graph.pending_jump_context.is_some() && mime != "text/gradesta-url" {
        if let Some(current) = app_state.current_vertex {
            app_state.history.push(current);
        }
        app_state.current_vertex = Some(vertex_id);
        graph.pending_jump_context = None;
        app_state.loading_portal_vertex = None;
        app_state.loading_portal_cell = None;
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
        graph.landmark_mgr.remove_vertex(vertex_id);
        if app_state.current_vertex == Some(vertex_id) {
            app_state.current_vertex = app_state.history.pop();
        }
        eprintln!("[{}] Vertex {} deleted from local graph", ts(), vertex_id);
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
        if let Some(pending) = app_state.pending_creations.remove(&action_id) {
            // If this was an async audio recording, update/remove the placeholder cell
            if let Some(local_id) = pending.local_placeholder_id {
                // Update pending cell status and server ID
                if let Some(pending_cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                    pending_cell.server_vertex_id = Some(vertex_id);
                    pending_cell.set_audio_status(PendingAudioStatus::Transcribing);
                }

                // Remove placeholder since the server vertex is now created
                app_state.pending_audio_cells.remove(&local_id);

                // Clear recording_placeholder_id if it was pointing to this placeholder
                if app_state.recording_placeholder_id == Some(local_id) {
                    app_state.recording_placeholder_id = None;
                }
            }

            if vertex_id != 0 {
                if let Some(current) = app_state.current_vertex {
                    if current != vertex_id {
                        app_state.history.push(current);
                        app_state.current_vertex = Some(vertex_id);
                    }
                } else {
                    app_state.current_vertex = Some(vertex_id);
                }
            }

            let entry = graph.vertices.entry(vertex_id).or_default();
            entry.id = vertex_id;
            // Store pending data in layer 0
            entry.layers.insert(0, LayerContent {
                mime: pending.mime.clone(),
                data: pending.data.clone(),
            });
            entry.layer_lengths.insert(0, pending.data.len() as u32);
            entry.layer_loaded.insert(0, true);

            if pending.mime.starts_with("audio/") {
                app_state.skip_autoplay_vertex = Some(vertex_id);

                // Handle transcription based on mode
                match pending.transcription_mode {
                    TranscriptionMode::Off => {
                        eprintln!("[{}] Transcription disabled for vertex {} (action={})", ts(), vertex_id, action_id);
                    }
                    TranscriptionMode::Cloud => {
                        // For cloud mode, transcript was captured during recording
                        if let Some(transcript) = pending.cloud_transcript {
                            if !transcript.is_empty() {
                                eprintln!("[{}] Using cloud transcript for vertex {}: {}", ts(), vertex_id, transcript);
                                let transcript_action_id = app_state.next_action_id;
                                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                                // Send to server immediately
                                let _ = net_tx.0.send(ServerEvent::LocalSetVertexLabel {
                                    action_id: transcript_action_id,
                                    vertex_id,
                                    layer: 1,
                                    mime: "text/plain".to_string(),
                                    data: transcript.into_bytes(),
                                });
                            } else {
                                eprintln!("[{}] Cloud transcript is empty for vertex {}", ts(), vertex_id);
                            }
                        } else {
                            eprintln!("[{}] No cloud transcript available for vertex {}", ts(), vertex_id);
                        }
                    }
                    TranscriptionMode::Local => {
                        eprintln!("[{}] Starting local Whisper transcription for vertex {} (action={})", ts(), vertex_id, action_id);
                        let event_tx = net_tx.0.clone();
                        let target_vertex = vertex_id;
                        let transcript_action_id = app_state.next_action_id;
                        app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                        thread::spawn(move || {
                            if whisper::is_model_available() {
                                match whisper::transcribe(&pending.samples, pending.sample_rate) {
                                    Ok(text) => {
                                        let ts_str = {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default();
                                            let secs = now.as_secs() % 86400;
                                            let millis = now.subsec_millis();
                                            format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                        };
                                        eprintln!("[{}] Transcription complete: {}", ts_str, text);
                                        let _ = event_tx.send(ServerEvent::LocalSetVertexLabel {
                                            action_id: transcript_action_id,
                                            vertex_id: target_vertex,
                                            layer: 1,
                                            mime: "text/plain".to_string(),
                                            data: text.into_bytes(),
                                        });
                                    }
                                    Err(e) => {
                                        let ts_str = {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default();
                                            let secs = now.as_secs() % 86400;
                                            let millis = now.subsec_millis();
                                            format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                        };
                                        eprintln!("[{}] Transcription failed: {}", ts_str, e);
                                    }
                                }
                            } else {
                                let ts_str = {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default();
                                    let secs = now.as_secs() % 86400;
                                    let millis = now.subsec_millis();
                                    format!("{:02}:{:02}:{:02}.{:03}", secs / 3600, (secs % 3600) / 60, secs % 60, millis)
                                };
                                eprintln!("[{}] Whisper model not available, skipping transcription", ts_str);
                            }
                        });
                    }
                }
            }
        }
    } else if status == 202 {
        // 202 Accepted - request is being processed
        // Navigate IMMEDIATELY on 202 for both text and audio cells (edges are already set up)
        // Transcription handling still happens on 200 for audio
        if let Some(pending) = app_state.pending_creations.get(&action_id) {
            if vertex_id != 0 {
                // Clean up the local placeholder
                if let Some(local_id) = pending.local_placeholder_id {
                    app_state.pending_audio_cells.remove(&local_id);
                    if app_state.recording_placeholder_id == Some(local_id) {
                        app_state.recording_placeholder_id = None;
                    }
                }

                // Switch from submitting mode to Normal (for text cells)
                if let InputMode::InlineEdit { submitting: true, .. } = app_state.input_mode {
                    app_state.input_mode = InputMode::Normal;
                }

                // Navigate to the new vertex
                if let Some(current) = app_state.current_vertex {
                    if current != vertex_id {
                        app_state.history.push(current);
                        app_state.current_vertex = Some(vertex_id);
                    }
                } else {
                    app_state.current_vertex = Some(vertex_id);
                }

                // For audio cells, populate the vertex data so it displays immediately
                if pending.mime.starts_with("audio/") {
                    let entry = graph.vertices.entry(vertex_id).or_default();
                    entry.id = vertex_id;
                    entry.layers.insert(0, LayerContent {
                        mime: pending.mime.clone(),
                        data: pending.data.clone(),
                    });
                    entry.layer_lengths.insert(0, pending.data.len() as u32);
                    entry.layer_loaded.insert(0, true);
                    app_state.skip_autoplay_vertex = Some(vertex_id);

                    // Also add the cloud transcript immediately so text is visible during transition
                    if let Some(ref transcript) = pending.cloud_transcript {
                        if !transcript.is_empty() {
                            let transcript_bytes = transcript.as_bytes().to_vec();
                            entry.layers.insert(1, LayerContent {
                                mime: "text/plain".to_string(),
                                data: transcript_bytes.clone(),
                            });
                            entry.layer_lengths.insert(1, transcript_bytes.len() as u32);
                            entry.layer_loaded.insert(1, true);
                        }
                    }
                }
            }
        }
        // Don't remove pending_creation - wait for the final 200 status for transcription
    } else {

        // If this was an async recording or text cell that failed, remove the placeholder
        if let Some(pending) = app_state.pending_creations.remove(&action_id) {
            if let Some(local_id) = pending.local_placeholder_id {
                app_state.pending_audio_cells.remove(&local_id);
                // Clear recording_placeholder_id if it was pointing to this placeholder
                if app_state.recording_placeholder_id == Some(local_id) {
                    app_state.recording_placeholder_id = None;
                }
                if pending.mime.starts_with("audio/") {
                    app_state.status = format!("Audio upload failed: {}", message);
                } else {
                    app_state.status = format!("Failed to create cell: {}", message);
                }
            }
        }
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
    entry.layers.insert(layer, LayerContent {
        mime: mime.to_string(),
        data: data.clone(),
    });

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
        eprintln!("[{}] HTTP Fetch: Got MP4 video ({} bytes), starting native player", ts(), data.len());

        match VideoPlayer::new(data.clone()) {
            Ok(player) => {
                media_cache.video_players.insert(vertex_id, player);
                app_state.video_modal_vertex_id = Some(vertex_id);
                app_state.show_video_modal = true;
                eprintln!("[{}] HTTP Fetch: Video player started for vertex {}", ts(), vertex_id);
            }
            Err(e) => {
                eprintln!("[{}] HTTP Fetch: Failed to create video player: {}", ts(), e);
                app_state.status = format!("Video error: {}", e);
            }
        }
        return;
    }

    // For other video formats, fall back to external player
    if mime.starts_with("video/") {
        eprintln!("[{}] HTTP Fetch: Got video {} ({} bytes), launching external player", ts(), mime, data.len());

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
                    eprintln!("[{}] HTTP Fetch: Failed to write temp file: {}", ts(), e);
                } else {
                    let path = temp.path().to_owned();
                    let (file, file_path) = temp.keep().unwrap_or_else(|e| {
                        eprintln!("[{}] Failed to keep temp file: {}", ts(), e);
                        (std::fs::File::create(&path).unwrap(), path.clone())
                    });
                    drop(file);

                    eprintln!("[{}] HTTP Fetch: Launching mpv for {}", ts(), file_path.display());
                    if let Err(e) = std::process::Command::new("mpv")
                        .arg(&file_path)
                        .spawn()
                    {
                        eprintln!("[{}] HTTP Fetch: Failed to launch mpv: {}", ts(), e);
                        if let Err(e2) = open::that(&file_path) {
                            eprintln!("[{}] HTTP Fetch: Failed to open with xdg-open: {}", ts(), e2);
                        }
                    }
                }
            }
            Err(e) => eprintln!("[{}] HTTP Fetch: Failed to create temp file: {}", ts(), e),
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

    eprintln!("[{}] HTTP Fetch: Stored {} content in layer 2 for vertex {}", ts(), mime, vertex_id);
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

/// Update audio level in recording placeholder cells (for live visualization)
pub fn update_recording_audio_levels(
    mut app_state: ResMut<AppState>,
) {
    // Find any cells in Recording status and update their audio level
    let samples_arc = app_state.audio_samples.clone();

    for cell in app_state.pending_audio_cells.values_mut() {
        if cell.is_recording() {
            // Calculate RMS of recent samples (last ~100ms worth at 44100Hz = ~4410 samples)
            if let Ok(samples) = samples_arc.lock() {
                let recent_count = 4410.min(samples.len());
                if recent_count > 0 {
                    let start = samples.len() - recent_count;
                    let sum_sq: f32 = samples[start..].iter().map(|s| s * s).sum();
                    let rms = (sum_sq / recent_count as f32).sqrt();
                    // Normalize to 0-1 range (typical voice RMS is 0.01-0.3)
                    cell.set_audio_level((rms * 5.0).min(1.0));
                }
            }
        }
    }
}

/// Process audio encoding results from background threads
pub fn process_audio_results(
    mut app_state: ResMut<AppState>,
    audio_processing: Res<AudioProcessingChannel>,
    ws_cmd_tx: Res<WsCommandTx>,
) {
    while let Ok(result) = audio_processing.rx.try_recv() {
        match result {
            AudioProcessingResult::StatusUpdate { local_id, status } => {
                if let Some(pending) = app_state.pending_audio_cells.get_mut(&local_id) {
                    pending.set_audio_status(status);
                }
            }
            AudioProcessingResult::Encoded {
                local_id,
                ogg_data,
                samples,
                sample_rate,
            } => {
                eprintln!(
                    "[{}] Audio encoding complete: local_id={} size={} bytes",
                    ts(),
                    local_id,
                    ogg_data.len()
                );

                // Get the pending cell info
                let pending_info = app_state.pending_audio_cells.get(&local_id).cloned();

                if let Some(pending_cell) = pending_info {
                    // Update status to uploading
                    if let Some(cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                        cell.set_audio_status(PendingAudioStatus::Uploading);
                    }

                    // Allocate action_id for the CreateVertex
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                    // Store the action_id in the pending cell
                    if let Some(cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                        cell.action_id = Some(action_id);
                    }

                    // Convert direction to protocol byte
                    let dir_byte = match pending_cell.direction {
                        crate::state::EDGE_WEST => 0,
                        crate::state::EDGE_EAST => 1,
                        crate::state::EDGE_NORTH => 2,
                        crate::state::EDGE_SOUTH => 3,
                        crate::state::EDGE_UP => 4,
                        crate::state::EDGE_DOWN => 5,
                        _ => 3, // default south
                    };

                    // Extract transcription mode and cloud transcript from pending cell
                    let (transcription_mode, cloud_transcript) = if let Some(transcript) = pending_cell.live_transcript() {
                        let text = transcript.lock().ok().map(|t| t.clone()).filter(|t| !t.is_empty());
                        (TranscriptionMode::Cloud, text)
                    } else {
                        (app_state.transcription_mode, None)
                    };

                    // Send to server
                    if let Some(ref tx) = ws_cmd_tx.0 {
                        let _ = tx.send(WsCommand::CreateVertex {
                            action_id,
                            from_vertex: pending_cell.from_vertex,
                            direction: dir_byte,
                            layer: 0,
                            mime: "audio/ogg".to_string(),
                            data: ogg_data.clone(),
                        });

                        // Store in pending_creations for when server responds
                        app_state.pending_creations.insert(
                            action_id,
                            PendingVertexCreation {
                                samples,
                                sample_rate,
                                data: ogg_data,
                                mime: "audio/ogg".to_string(),
                                local_placeholder_id: Some(local_id),
                                transcription_mode,
                                cloud_transcript,
                            },
                        );

                        eprintln!(
                            "[{}] Sent CreateVertex to server: action_id={} local_id={} from_vertex={} direction={} mode={:?}",
                            ts(), action_id, local_id, pending_cell.from_vertex, dir_byte, transcription_mode
                        );
                    }
                }
            }
            AudioProcessingResult::EncodingFailed { local_id, error } => {
                eprintln!("[{}] Audio encoding failed: local_id={} error={}", ts(), local_id, error);

                // Remove the failed pending cell
                app_state.pending_audio_cells.remove(&local_id);
                // Clear recording_placeholder_id if it was pointing to this placeholder
                if app_state.recording_placeholder_id == Some(local_id) {
                    app_state.recording_placeholder_id = None;
                }
                app_state.status = format!("Audio encoding failed: {}", error);
            }
        }
    }
}

use crate::graph::{build_grid_view, direction_priority_order};
use crate::state::{EDGE_NORTH, EDGE_SOUTH, EDGE_EAST, EDGE_WEST};

/// Request full content for visible cells that only have previews loaded.
/// Prioritizes cells in the navigation direction (north/south typically).
pub fn request_content_for_visible_cells(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    ws_cmd_tx: Res<WsCommandTx>,
) {
    let Some(current_id) = app_state.current_vertex else { return };
    let Some(cmd_tx) = &ws_cmd_tx.0 else { return };

    // Build grid view to find visible cells
    let grid = build_grid_view(&graph, Some(current_id), &app_state.pending_audio_cells, app_state.loading_portal_cell.as_ref());

    // Collect cells that need content loaded
    let mut cells_needing_content: Vec<(u64, u32, i32)> = Vec::new(); // (vertex_id, layer, priority)

    // Get direction priorities based on last navigation direction
    let priority_order = direction_priority_order(app_state.last_nav_direction);

    for (&pos, &vertex_id) in &grid.cells {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Check all layers uniformly
            for (&layer, _content) in &vertex.layers {
                let loaded = vertex.is_layer_loaded(layer);
                let length = vertex.layer_length(layer);
                if !loaded && length > 255 {
                    let key = (vertex_id, layer);
                    if !app_state.pending_content_requests.contains(&key) &&
                       !app_state.active_content_watches.contains(&key) {
                        let distance = pos.0.abs() + pos.1.abs();
                        let priority = calculate_priority(pos, &priority_order, distance);
                        cells_needing_content.push((vertex_id, layer, priority));
                    }
                }
            }
        }
    }

    // Sort by priority (lower = higher priority)
    cells_needing_content.sort_by_key(|&(_, _, priority)| priority);

    // Limit concurrent requests to avoid overwhelming the server
    const MAX_CONCURRENT_REQUESTS: usize = 5;
    let available_slots = MAX_CONCURRENT_REQUESTS.saturating_sub(app_state.pending_content_requests.len());

    for (vertex_id, layer, _) in cells_needing_content.into_iter().take(available_slots) {
        let action_id = app_state.next_action_id;
        app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

        eprintln!("[{}] Requesting content: vertex={} layer={} action={}", ts(), vertex_id, layer, action_id);

        let key = (vertex_id, layer);
        app_state.pending_content_requests.insert(key);

        let _ = cmd_tx.send(WsCommand::WatchContent { action_id, vertex_id, layer });
    }

    // Clean up: unwatch content for cells no longer in view
    let visible_vertices: std::collections::HashSet<u64> = grid.cells.values().copied().collect();
    let watches_to_remove: Vec<(u64, u32)> = app_state.active_content_watches
        .iter()
        .filter(|(vid, _)| !visible_vertices.contains(vid))
        .copied()
        .collect();

    for (vertex_id, layer) in watches_to_remove {
        let action_id = app_state.next_action_id;
        app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

        eprintln!("[{}] Unwatching content: vertex={} layer={} action={}", ts(), vertex_id, layer, action_id);

        app_state.active_content_watches.remove(&(vertex_id, layer));
        let _ = cmd_tx.send(WsCommand::UnwatchContent { action_id, vertex_id, layer });
    }
}

/// Calculate priority for content loading based on position and navigation direction.
/// Lower priority = load sooner.
fn calculate_priority(pos: (i32, i32), priority_order: &[usize; 6], distance: i32) -> i32 {
    // Base priority is distance from cursor
    let mut priority = distance * 10;

    // Bonus for being in the navigation direction
    // Check if cell is in the direction we're navigating
    let (x, y) = pos;

    // priority_order[0] and priority_order[1] are the primary directions
    let primary_dir = priority_order[0];

    match primary_dir {
        EDGE_NORTH if y < 0 => priority -= 5, // Cell is north of cursor
        EDGE_SOUTH if y > 0 => priority -= 5, // Cell is south of cursor
        EDGE_WEST if x < 0 => priority -= 5,  // Cell is west of cursor
        EDGE_EAST if x > 0 => priority -= 5,  // Cell is east of cursor
        _ => {}
    }

    // Secondary direction bonus
    let secondary_dir = priority_order[1];
    match secondary_dir {
        EDGE_NORTH if y < 0 => priority -= 3,
        EDGE_SOUTH if y > 0 => priority -= 3,
        EDGE_WEST if x < 0 => priority -= 3,
        EDGE_EAST if x > 0 => priority -= 3,
        _ => {}
    }

    priority
}
