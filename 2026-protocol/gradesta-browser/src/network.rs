//! WebSocket communication and server protocol handling.
//!
//! This module handles the WebSocket connection to gradesta servers,
//! including message parsing, sending commands, and event handling.

use std::io;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use tungstenite::{client, Message};
use url::Url;

use crate::graph::{GraphState, LayerContent};
use crate::media::MediaCache;
use crate::state::AppState;

// Protocol message type constants - Server to Client
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG: u8 = 0x0F;
pub const MSG_SERVER_REQUEST_IDENTIFICATION: u8 = 0x10;
pub const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;
pub const MSG_SERVER_ELF_OUTPUT_FWD: u8 = 0x22;

// Protocol message type constants - Client to Server
pub const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
pub const MSG_CLIENT_SET_EDGES: u8 = 0x83;
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
pub const MSG_CLIENT_DELETE_VERTEX: u8 = 0x87;
pub const MSG_CLIENT_IDENTIFICATION_RESPONSE: u8 = 0x90;
pub const MSG_CLIENT_IDENTIFICATION_REFUSED: u8 = 0x91;
pub const MSG_CLIENT_INTRODUCE_ELF: u8 = 0xA0;

// Direction bitmask flags
pub const DIRECTION_WEST: u8 = 0x01;
pub const DIRECTION_EAST: u8 = 0x02;
pub const DIRECTION_NORTH: u8 = 0x04;
pub const DIRECTION_SOUTH: u8 = 0x08;
pub const DIRECTION_UP: u8 = 0x10;
pub const DIRECTION_DOWN: u8 = 0x20;

// Permission bitmask flags
pub const PERM_READ: u8 = 0x01;
pub const PERM_WRITE: u8 = 0x02;
pub const PERM_CREATE: u8 = 0x04;
pub const PERM_DELETE: u8 = 0x08;

/// Events received from the server or generated locally
#[derive(Clone, Debug)]
pub enum ServerEvent {
    SetContext { uri: String },
    SetVertexLabel { vertex_id: u64, layer: u32, mime: String, data: Vec<u8> },
    SetEdges { vertex_id: u64, edges: [u64; 6], edit_mask: u8 },
    Log { action_id: u64, status: u32, vertex_id: u64, message: String },
    Connected { base_url: String },
    Disconnected { reason: String },
    Error { message: String },
    RequestIdentification {
        action_id: u64,
        nonce: [u8; 32],
        timestamp: u64,
        reason: String,
    },
    /// Local event: store a label locally and send to server
    LocalSetVertexLabel {
        action_id: u64,
        vertex_id: u64,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    /// HTTP stream content fetched (layer 3 -> layer 2)
    HttpStreamContentFetched {
        vertex_id: u64,
        mime: String,
        data: Vec<u8>,
    },
    /// Server provided an introduction token for an elf
    IntroductionToken {
        action_id: u64,
        token: String,
        server_ws_url: String,
    },
    /// Elf output forwarded from server
    ElfOutputFwd {
        action_id: u64,
        output_type: u8,
        data: Vec<u8>,
    },
}

/// Commands sent TO the websocket thread
#[derive(Clone, Debug)]
pub enum WsCommand {
    WatchLandmark { action_id: u64, landmark: String },
    ClickVertex { action_id: u64, vertex_id: u64 },
    IdentificationResponse {
        action_id: u64,
        identity_url: String,
        signature: Vec<u8>,
    },
    IdentificationRefused {
        action_id: u64,
    },
    SetVertexLabel {
        action_id: u64,
        vertex_id: u64,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    CreateVertex {
        action_id: u64,
        from_vertex: u64,
        direction: u8,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    DeleteVertex {
        action_id: u64,
        vertex_id: u64,
    },
    SetEdges {
        action_id: u64,
        vertex_id: u64,
        edges: [u64; 6],
    },
    /// Introduce an elf to the server
    IntroduceElf {
        action_id: u64,
        elf_url: String,
        command: String,
        cursor_landmark: String,
        cursor_vertex: u64,
        origin_landmark: String,
        origin_vertex: u64,
        allowed_directions: u8,
        max_depth: i32,
        permissions: u8,
        params: std::collections::HashMap<String, String>,
    },
}

/// Resource for receiving server events
#[derive(Resource)]
pub struct NetRx(pub Receiver<ServerEvent>);

/// Resource for sending events (used by background HTTP fetches)
#[derive(Resource)]
pub struct NetEventsTx(pub Sender<ServerEvent>);

/// Resource for sending commands to the WebSocket thread
#[derive(Resource)]
pub struct WsCommandTx(pub Option<Sender<WsCommand>>);

/// Run the WebSocket connection in a background thread
pub fn run_ws(
    url: Url,
    cmd_rx: Receiver<WsCommand>,
    net_tx: Sender<ServerEvent>,
) -> Result<()> {
    let host = url.host_str().ok_or_else(|| anyhow!("Missing host"))?;
    let port = url.port().unwrap_or(if url.scheme() == "wss" { 443 } else { 80 });

    let stream = TcpStream::connect((host, port)).context("Failed to connect")?;
    let (mut socket, _) = client(url.clone(), stream).context("WebSocket handshake failed")?;
    eprintln!("Connected!");

    let base_url = format!("{}://{}:{}{}", url.scheme(), host, port, url.path());
    let _ = net_tx.send(ServerEvent::Connected { base_url });

    let landmark = url.query_pairs()
        .find(|(k, _)| k == "landmark")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| url.path().to_string());

    eprintln!("SEND WatchLandmark action=0 uri={:?}", landmark);
    let mut buf = Vec::with_capacity(1 + 8 + landmark.len());
    buf.push(MSG_CLIENT_WATCH_LANDMARK);
    buf.extend_from_slice(&0u64.to_be_bytes());
    buf.extend_from_slice(landmark.as_bytes());
    socket.send(Message::Binary(buf))?;

    socket.get_mut().set_nonblocking(true)?;

    loop {
        // Check for commands to send
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                WsCommand::WatchLandmark { action_id, landmark } => {
                    eprintln!("SEND WatchLandmark action={} uri={:?}", action_id, landmark);
                    let mut buf = Vec::with_capacity(1 + 8 + landmark.len());
                    buf.push(MSG_CLIENT_WATCH_LANDMARK);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(landmark.as_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::ClickVertex { action_id, vertex_id } => {
                    eprintln!("SEND ClickVertex action={} vertex={}", action_id, vertex_id);
                    let mut buf = Vec::with_capacity(1 + 8 + 8);
                    buf.push(MSG_CLIENT_CLICK_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::IdentificationResponse { action_id, identity_url, signature } => {
                    eprintln!("SEND IdentificationResponse action={} url={:?}", action_id, identity_url);
                    let mut buf = Vec::with_capacity(1 + 8 + identity_url.len() + 1 + 64);
                    buf.push(MSG_CLIENT_IDENTIFICATION_RESPONSE);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(identity_url.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&signature);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::IdentificationRefused { action_id } => {
                    eprintln!("SEND IdentificationRefused action={}", action_id);
                    let mut buf = Vec::with_capacity(1 + 8);
                    buf.push(MSG_CLIENT_IDENTIFICATION_REFUSED);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::SetVertexLabel { action_id, vertex_id, layer, mime, data } => {
                    eprintln!("SEND SetVertexLabel action={} vertex={} layer={} mime={:?} len={}", action_id, vertex_id, layer, mime, data.len());
                    let mut buf = Vec::with_capacity(1 + 8 + 8 + 4 + mime.len() + 1 + data.len());
                    buf.push(MSG_CLIENT_SET_VERTEX_LABEL);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    buf.extend_from_slice(&layer.to_be_bytes());
                    buf.extend_from_slice(mime.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&data);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::CreateVertex { action_id, from_vertex, direction, layer, mime, data } => {
                    eprintln!("SEND CreateVertex action={} from={} dir={} layer={} mime={:?} len={}", action_id, from_vertex, direction, layer, mime, data.len());
                    let mut buf = Vec::with_capacity(1 + 8 + 8 + 1 + 4 + mime.len() + 1 + data.len());
                    buf.push(MSG_CLIENT_CREATE_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&from_vertex.to_be_bytes());
                    buf.push(direction);
                    buf.extend_from_slice(&layer.to_be_bytes());
                    buf.extend_from_slice(mime.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&data);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::DeleteVertex { action_id, vertex_id } => {
                    eprintln!("SEND DeleteVertex action={} vertex={}", action_id, vertex_id);
                    let mut buf = Vec::with_capacity(1 + 8 + 8);
                    buf.push(MSG_CLIENT_DELETE_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::SetEdges { action_id, vertex_id, edges } => {
                    eprintln!("SEND SetEdges action={} vertex={} edges={:?}", action_id, vertex_id, edges);
                    let mut buf = Vec::with_capacity(1 + 8 + 8 + 48);
                    buf.push(MSG_CLIENT_SET_EDGES);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    for edge in &edges {
                        buf.extend_from_slice(&edge.to_be_bytes());
                    }
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::IntroduceElf {
                    action_id,
                    elf_url,
                    command,
                    cursor_landmark,
                    cursor_vertex,
                    origin_landmark,
                    origin_vertex,
                    allowed_directions,
                    max_depth,
                    permissions,
                    params,
                } => {
                    eprintln!("SEND IntroduceElf action={} elf_url={} command={}", action_id, elf_url, command);
                    let mut buf = Vec::new();
                    buf.push(MSG_CLIENT_INTRODUCE_ELF);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(elf_url.as_bytes());
                    buf.push(0);
                    buf.extend_from_slice(command.as_bytes());
                    buf.push(0);
                    buf.extend_from_slice(cursor_landmark.as_bytes());
                    buf.push(0);
                    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
                    // Region spec
                    buf.extend_from_slice(origin_landmark.as_bytes());
                    buf.push(0);
                    buf.extend_from_slice(&origin_vertex.to_be_bytes());
                    buf.push(allowed_directions);
                    buf.extend_from_slice(&max_depth.to_be_bytes());
                    // Permissions
                    buf.push(permissions);
                    // Params
                    buf.extend_from_slice(&(params.len() as u16).to_be_bytes());
                    for (k, v) in &params {
                        buf.extend_from_slice(k.as_bytes());
                        buf.push(0);
                        buf.extend_from_slice(v.as_bytes());
                        buf.push(0);
                    }
                    socket.send(Message::Binary(buf))?;
                }
            }
        }

        // Read incoming messages
        match socket.read() {
            Ok(Message::Binary(data)) => {
                if let Ok(event) = parse_server_message(&data) {
                    let _ = net_tx.send(event);
                }
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping with pong (standard WebSocket heartbeat)
                let _ = socket.send(Message::Pong(data));
            }
            Ok(Message::Close(frame)) => {
                let reason = frame
                    .map(|f| f.reason.to_string())
                    .unwrap_or_else(|| "Connection closed".to_string());
                let _ = net_tx.send(ServerEvent::Disconnected { reason });
                return Ok(());
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = net_tx.send(ServerEvent::Disconnected {
                    reason: format!("Connection error: {}", e)
                });
                return Err(e.into());
            }
        }
    }
}

/// Parse a binary server message into a ServerEvent
pub fn parse_server_message(data: &[u8]) -> Result<ServerEvent> {
    if data.is_empty() {
        return Err(anyhow!("empty"));
    }
    match data[0] {
        MSG_SERVER_SET_CONTEXT => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let uri = String::from_utf8(rest.to_vec())?;
            eprintln!("RECV SetContext action={} uri={:?}", action_id, uri);
            Ok(ServerEvent::SetContext { uri })
        }
        MSG_SERVER_SET_VERTEX_LABEL => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (vertex_id, rest) = read_u64(rest)?;
            let (layer, rest) = read_u32(rest)?;
            let (mime, label) = read_null_terminated(rest)?;
            let label_preview: String = String::from_utf8_lossy(label).chars().take(40).collect();
            eprintln!("RECV SetVertexLabel action={} vertex={} layer={} mime={:?} label={:?}... ({} bytes)",
                action_id, vertex_id, layer, mime, label_preview, label.len());
            Ok(ServerEvent::SetVertexLabel { vertex_id, layer, mime, data: label.to_vec() })
        }
        MSG_SERVER_SET_EDGES => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (vertex_id, rest) = read_u64(rest)?;
            let mut edges = [0u64; 6];
            let mut slice = rest;
            for edge in edges.iter_mut() {
                let (v, next) = read_u64(slice)?;
                *edge = v;
                slice = next;
            }
            // Read edit_mask (1 byte) if present, default to 0x7F (editable)
            let edit_mask = if !slice.is_empty() { slice[0] } else { 0x7F };
            eprintln!("RECV SetEdges action={} vertex={} W={} E={} N={} S={} U={} D={} edit_mask=0x{:02x}",
                action_id, vertex_id, edges[0], edges[1], edges[2], edges[3], edges[4], edges[5], edit_mask);
            Ok(ServerEvent::SetEdges { vertex_id, edges, edit_mask })
        }
        MSG_SERVER_LOG => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (status, rest) = read_u32(rest)?;
            let (vertex_id, rest) = read_u64(rest)?;
            let message = String::from_utf8(rest.to_vec())?;
            eprintln!("RECV Log action={} status={} vertex={} msg={:?}", action_id, status, vertex_id, message);
            Ok(ServerEvent::Log { action_id, status, vertex_id, message })
        }
        MSG_SERVER_REQUEST_IDENTIFICATION => {
            if data.len() < 1 + 8 + 32 + 8 {
                return Err(anyhow!("Request identification message too short"));
            }
            let (action_id, rest) = read_u64(&data[1..])?;
            let mut nonce = [0u8; 32];
            nonce.copy_from_slice(&rest[..32]);
            let (timestamp, rest) = read_u64(&rest[32..])?;
            let reason = String::from_utf8(rest.to_vec()).unwrap_or_else(|_| "Unknown".to_string());
            eprintln!("RECV RequestIdentification action={} nonce={:?}... timestamp={} reason={:?}",
                action_id, &nonce[..8], timestamp, reason);
            Ok(ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason })
        }
        MSG_SERVER_INTRODUCTION_TOKEN => {
            if data.len() < 1 + 8 {
                return Err(anyhow!("IntroductionToken message too short"));
            }
            let (action_id, rest) = read_u64(&data[1..])?;
            let (token, rest) = read_null_terminated(rest)?;
            let (server_ws_url, _) = read_null_terminated(rest)?;
            eprintln!("RECV IntroductionToken action={} token={}... server_ws_url={}",
                action_id, &token[..std::cmp::min(8, token.len())], server_ws_url);
            Ok(ServerEvent::IntroductionToken { action_id, token, server_ws_url })
        }
        MSG_SERVER_ELF_OUTPUT_FWD => {
            if data.len() < 1 + 8 + 1 {
                return Err(anyhow!("ElfOutputFwd message too short"));
            }
            let (action_id, rest) = read_u64(&data[1..])?;
            let output_type = rest[0];
            let output_data = rest[1..].to_vec();
            eprintln!("RECV ElfOutputFwd action={} type={} {} bytes",
                action_id, output_type, output_data.len());
            Ok(ServerEvent::ElfOutputFwd { action_id, output_type, data: output_data })
        }
        other => {
            eprintln!("RECV Unknown message type: 0x{:02x}", other);
            Err(anyhow!("unknown message type"))
        }
    }
}

// Binary parsing helpers
fn read_u64(data: &[u8]) -> Result<(u64, &[u8])> {
    if data.len() < 8 {
        return Err(anyhow!("not enough bytes"));
    }
    let value = u64::from_be_bytes(data[..8].try_into()?);
    Ok((value, &data[8..]))
}

fn read_u32(data: &[u8]) -> Result<(u32, &[u8])> {
    if data.len() < 4 {
        return Err(anyhow!("not enough bytes"));
    }
    let value = u32::from_be_bytes(data[..4].try_into()?);
    Ok((value, &data[4..]))
}

fn read_null_terminated(data: &[u8]) -> Result<(String, &[u8])> {
    let pos = data.iter().position(|b| *b == 0).ok_or_else(|| anyhow!("no null"))?;
    let s = String::from_utf8(data[..pos].to_vec())?;
    Ok((s, &data[pos + 1..]))
}

/// Bevy system to ingest server events and update state
pub fn ingest_server_events(
    mut graph: ResMut<GraphState>,
    mut app_state: ResMut<AppState>,
    rx: Res<NetRx>,
    net_tx: Res<NetEventsTx>,
    mut ws_cmd_tx: ResMut<WsCommandTx>,
    mut media_cache: ResMut<MediaCache>,
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
                // Track which landmark we're receiving data for
                graph.current_receiving_landmark = Some(uri.clone());

                // Initialize the vertex list for this landmark if not already present
                graph.landmark_vertices.entry(uri.clone()).or_insert_with(Vec::new);

                // Check if we're following a portal to this context
                let is_following = app_state.following_portal.as_ref()
                    .map(|p| p == &uri)
                    .unwrap_or(false);

                if is_following {
                    // We're arriving at a followed portal - clear it and mark that
                    // we should jump to the first vertex of this context
                    app_state.following_portal = None;
                    graph.pending_jump_context = Some(uri.clone());
                }

                graph.context_uri = Some(uri.clone());
                app_state.status = format!("Viewing: {uri}");

                // Only add to landmark history if we're actively navigating there
                // (following a portal or initial connection), not just preloading data
                let is_initial = app_state.landmark_history.is_empty();
                if is_following || is_initial {
                    app_state.landmark_history.retain(|l| l != &uri);
                    app_state.landmark_history.push(uri.clone());
                    // Trim to max size
                    while app_state.landmark_history.len() > app_state.max_landmark_history {
                        app_state.landmark_history.remove(0);
                    }
                }

                // Update only the landmark input (server stays the same)
                app_state.landmark_input = uri.clone();
            }
            ServerEvent::SetVertexLabel { vertex_id, layer, mime, data } => {
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;

                // Special handling for layer 3 HTTP stream URLs
                if layer == 3 && mime == "text/x-http-stream-url" {
                    // Parse the content: expected-mime\nurl
                    if let Ok(content) = String::from_utf8(data.clone()) {
                        if let Some((expected_mime, url)) = content.split_once('\n') {
                            let url = url.trim().to_string();
                            let expected_mime = expected_mime.trim().to_string();
                            let v_id = vertex_id;
                            let events_tx = net_tx.0.clone();

                            // Spawn background thread to fetch content via HTTP
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
                    // Still store the layer 3 content for reference
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
                        data,
                    });
                    return;
                }

                // Invalidate cached texture/media when content changes
                if mime.starts_with("image/") || crate::media::is_image_data(&data) {
                    media_cache.textures.remove(&vertex_id);
                    media_cache.animated_gifs.remove(&vertex_id);
                }

                // Store in appropriate layer
                if layer == 0 {
                    // Primary layer - backwards compatible
                    entry.label = data;
                    entry.mime = Some(mime.clone());
                } else {
                    // Secondary layers
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
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

                // If we're waiting to jump to a new context, and this vertex is NOT a portal,
                // jump to it (skip portals since they're just links)
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
            ServerEvent::SetEdges { vertex_id, edges, edit_mask } => {
                // Check if this is a deletion signal (all edges = 0, edit_mask = 0)
                let vertex_exists = graph.vertices.contains_key(&vertex_id);
                let is_deletion = vertex_exists && edges.iter().all(|&e| e == 0) && edit_mask == 0;

                if is_deletion {
                    // Remove the vertex from the graph
                    graph.vertices.remove(&vertex_id);
                    // Remove from history
                    app_state.history.retain(|&id| id != vertex_id);
                    // Remove from landmark_vertices
                    for vertices in graph.landmark_vertices.values_mut() {
                        vertices.retain(|&id| id != vertex_id);
                    }
                    // If we're currently on this deleted vertex, navigate away
                    if app_state.current_vertex == Some(vertex_id) {
                        // Try to go back in history
                        if let Some(prev) = app_state.history.pop() {
                            app_state.current_vertex = Some(prev);
                        } else {
                            // Find any other vertex in the current landmark
                            let fallback = graph.vertices.keys().next().copied();
                            app_state.current_vertex = fallback;
                        }
                    }
                } else {
                    let entry = graph.vertices.entry(vertex_id).or_default();
                    entry.id = vertex_id;
                    entry.edges = edges;
                    entry.edit_mask = edit_mask;
                }
            }
            ServerEvent::Log { action_id, status, vertex_id, message } => {
                eprintln!("Log: action={} status={} vertex={} message={}", action_id, status, vertex_id, message);

                // Check for pending creation with this action_id
                if let Some(creation) = app_state.pending_creations.remove(&action_id) {
                    if status == 0 {
                        // Success - transcribe audio if we have samples
                        if !creation.samples.is_empty() {
                            // Spawn whisper transcription
                            let samples = creation.samples;
                            let sample_rate = creation.sample_rate;
                            let v_id = vertex_id;
                            let events_tx = net_tx.0.clone();

                            std::thread::spawn(move || {
                                if let Ok(result) = crate::whisper::transcribe(&samples, sample_rate) {
                                    if !result.trim().is_empty() {
                                        // Store transcript locally and send to server
                                        let _ = events_tx.send(ServerEvent::LocalSetVertexLabel {
                                            action_id: 0,
                                            vertex_id: v_id,
                                            layer: 1,
                                            mime: "text/plain".to_string(),
                                            data: result.into_bytes(),
                                        });
                                    }
                                }
                            });
                        }
                    }
                }
            }
            ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason } => {
                // Store as pending identification for the UI to handle
                if let Some(base_url) = &app_state.base_ws_url {
                    app_state.pending_identification = Some(crate::state::PendingIdentification {
                        action_id,
                        nonce,
                        timestamp,
                        reason,
                        server_url: base_url.clone(),
                    });
                }
            }
            ServerEvent::LocalSetVertexLabel { action_id: _, vertex_id, layer, mime, data } => {
                // Store locally
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;
                if layer == 0 {
                    entry.label = data.clone();
                    entry.mime = Some(mime.clone());
                } else {
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
                        data: data.clone(),
                    });
                }

                // Send to server
                if let Some(tx) = &ws_cmd_tx.0 {
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    let _ = tx.send(WsCommand::SetVertexLabel {
                        action_id,
                        vertex_id,
                        layer,
                        mime,
                        data,
                    });
                }
            }
            ServerEvent::HttpStreamContentFetched { vertex_id, mime, data } => {
                // Store the fetched content as layer 2
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;

                // For other video formats, fall back to external player
                if mime.starts_with("video/") {
                    eprintln!("HTTP Fetch: Got video {} ({} bytes), launching external player", mime, data.len());

                    // Determine file extension from mime type
                    let ext = match mime.as_str() {
                        "video/mp4" => "mp4",
                        "video/webm" => "webm",
                        "video/quicktime" => "mov",
                        _ => "mp4",
                    };

                    // Write to temp file and open with system player
                    let temp_path = format!("/tmp/gradesta_video_{}.{}", vertex_id, ext);
                    if std::fs::write(&temp_path, &data).is_ok() {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(&temp_path)
                            .spawn();
                    }
                }

                // Invalidate cache to trigger reload
                if mime.starts_with("image/") || crate::media::is_image_data(&data) {
                    media_cache.textures.remove(&vertex_id);
                    media_cache.animated_gifs.remove(&vertex_id);
                }

                entry.layers.insert(2, LayerContent {
                    mime,
                    data,
                });
            }
            ServerEvent::IntroductionToken { action_id, token, server_ws_url } => {
                eprintln!("IntroductionToken received: action={}, token={}..., server_ws_url={}",
                    action_id, &token[..std::cmp::min(8, token.len())], server_ws_url);

                // Find the elf task that requested this
                if let Some(task) = app_state.active_elf_tasks.get(&action_id) {
                    let elf_url = task.elf_url.clone();
                    let summon_url = format!("{}/summon", elf_url.trim_end_matches('/'));

                    // Spawn a thread to POST the summon request to the elf
                    let token_clone = token.clone();
                    let server_url_clone = server_ws_url.clone();
                    let command = task.command.clone();

                    thread::spawn(move || {
                        eprintln!("Summoning elf at {} with token {}...", summon_url, &token_clone[..std::cmp::min(8, token_clone.len())]);

                        // Build summon request JSON
                        let body = serde_json::json!({
                            "token": token_clone,
                            "server_ws_url": server_url_clone,
                            "command": command,
                            "params": {}
                        });

                        match reqwest::blocking::Client::new()
                            .post(&summon_url)
                            .json(&body)
                            .send()
                        {
                            Ok(response) => {
                                if response.status().is_success() {
                                    eprintln!("Elf summon request accepted");
                                } else {
                                    eprintln!("Elf summon failed: HTTP {}", response.status());
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to summon elf: {}", e);
                            }
                        }
                    });
                }
            }
            ServerEvent::ElfOutputFwd { action_id, output_type, data } => {
                // Add output to the active task
                if let Some(task) = app_state.active_elf_tasks.get_mut(&action_id) {
                    task.add_output(output_type, data);
                }
            }
        }
    }
}
