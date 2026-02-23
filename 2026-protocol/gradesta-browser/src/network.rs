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


// Protocol message type constants - Server to Client
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG: u8 = 0x0F;
pub const MSG_SERVER_REQUEST_IDENTIFICATION: u8 = 0x10;
pub const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;

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
            let (token, _rest) = read_null_terminated(rest)?;
            eprintln!("RECV IntroductionToken action={} token={}...",
                action_id, &token[..std::cmp::min(8, token.len())]);
            Ok(ServerEvent::IntroductionToken { action_id, token })
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
