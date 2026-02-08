//! Gradesta protocol message encoding/decoding

use anyhow::{anyhow, Result};

// Server to client message types
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;
pub const MSG_SERVER_REQUEST_IDENTIFICATION: u8 = 0x10;

// Client to server message types
pub const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
pub const MSG_CLIENT_SET_EDGES: u8 = 0x83;
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
pub const MSG_CLIENT_DELETE_VERTEX: u8 = 0x87;
pub const MSG_CLIENT_IDENTIFICATION_RESPONSE: u8 = 0x90;
pub const MSG_CLIENT_IDENTIFICATION_REFUSED: u8 = 0x91;

/// Sentinel value meaning "keep existing edge unchanged" when patching edges
pub const EDGE_UNCHANGED: u64 = u64::MAX;

/// Direction for edges
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    West = 0,
    East = 1,
    North = 2,
    South = 3,
    Up = 4,
    Down = 5,
}

impl TryFrom<u8> for Direction {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Direction::West),
            1 => Ok(Direction::East),
            2 => Ok(Direction::North),
            3 => Ok(Direction::South),
            4 => Ok(Direction::Up),
            5 => Ok(Direction::Down),
            _ => Err(anyhow!("Invalid direction: {}", value)),
        }
    }
}

/// Encode SetContext message
pub fn encode_set_context(action_id: u64, landmark: &str) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + 8 + landmark.len());
    msg.push(MSG_SERVER_SET_CONTEXT);
    msg.extend_from_slice(&action_id.to_be_bytes());
    msg.extend_from_slice(landmark.as_bytes());
    msg
}

/// Encode SetVertexLabel message for layer 0 (primary content)
pub fn encode_set_vertex_label(action_id: u64, vertex_id: u64, mime_type: &str, label: &[u8]) -> Vec<u8> {
    encode_set_vertex_label_layer(action_id, vertex_id, 0, mime_type, label)
}

/// Encode SetVertexLabel message with explicit layer
pub fn encode_set_vertex_label_layer(action_id: u64, vertex_id: u64, layer: u32, mime_type: &str, label: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + 8 + 8 + 4 + mime_type.len() + 1 + label.len());
    msg.push(MSG_SERVER_SET_VERTEX_LABEL);
    msg.extend_from_slice(&action_id.to_be_bytes());
    msg.extend_from_slice(&vertex_id.to_be_bytes());
    msg.extend_from_slice(&layer.to_be_bytes());
    msg.extend_from_slice(mime_type.as_bytes());
    msg.push(0); // null terminator
    msg.extend_from_slice(label);
    msg
}

/// Encode SetEdges message
pub fn encode_set_edges(
    action_id: u64,
    vertex_id: u64,
    west: u64,
    east: u64,
    north: u64,
    south: u64,
    up: u64,
    down: u64,
    edit_mask: u8,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + 8 + 8 + 48 + 1);
    msg.push(MSG_SERVER_SET_EDGES);
    msg.extend_from_slice(&action_id.to_be_bytes());
    msg.extend_from_slice(&vertex_id.to_be_bytes());
    msg.extend_from_slice(&west.to_be_bytes());
    msg.extend_from_slice(&east.to_be_bytes());
    msg.extend_from_slice(&north.to_be_bytes());
    msg.extend_from_slice(&south.to_be_bytes());
    msg.extend_from_slice(&up.to_be_bytes());
    msg.extend_from_slice(&down.to_be_bytes());
    msg.push(edit_mask);
    msg
}

/// Encode LogMessage
pub fn encode_log_message(action_id: u64, status: u32, vertex_id: u64, message: &str) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + 8 + 4 + 8 + message.len());
    msg.push(MSG_SERVER_LOG_MESSAGE);
    msg.extend_from_slice(&action_id.to_be_bytes());
    msg.extend_from_slice(&status.to_be_bytes());
    msg.extend_from_slice(&vertex_id.to_be_bytes());
    msg.extend_from_slice(message.as_bytes());
    msg
}

/// Encode RequestIdentification message
pub fn encode_request_identification(action_id: u64, nonce: &[u8; 32], timestamp: i64, reason: &str) -> Vec<u8> {
    let mut msg = Vec::with_capacity(1 + 8 + 32 + 8 + reason.len());
    msg.push(MSG_SERVER_REQUEST_IDENTIFICATION);
    msg.extend_from_slice(&action_id.to_be_bytes());
    msg.extend_from_slice(nonce);
    msg.extend_from_slice(&(timestamp as u64).to_be_bytes());
    msg.extend_from_slice(reason.as_bytes());
    msg
}

/// Parse WatchLandmark message
pub fn parse_watch_landmark(msg: &[u8]) -> Result<(u64, String)> {
    if msg.len() < 9 {
        return Err(anyhow!("WatchLandmark message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let landmark = String::from_utf8(msg[9..].to_vec())?;
    Ok((action_id, landmark))
}

/// Parse SetVertexLabel message from client (with layer support)
/// Returns: (action_id, vertex_id, layer, mime_type, label)
pub fn parse_client_set_vertex_label(msg: &[u8]) -> Result<(u64, u64, u32, String, Vec<u8>)> {
    if msg.len() < 21 {  // 1 + 8 + 8 + 4
        return Err(anyhow!("SetVertexLabel message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(msg[9..17].try_into()?);
    let layer = u32::from_be_bytes(msg[17..21].try_into()?);

    // Find null terminator for MIME type
    let mime_end = msg[21..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in MIME type"))?;

    let mime_type = String::from_utf8(msg[21..21 + mime_end].to_vec())?;
    let label = msg[21 + mime_end + 1..].to_vec();

    Ok((action_id, vertex_id, layer, mime_type, label))
}

/// Parse SetEdges message from client
pub fn parse_client_set_edges(msg: &[u8]) -> Result<(u64, u64, [u64; 6])> {
    if msg.len() < 1 + 8 + 8 + 48 {
        return Err(anyhow!("SetEdges message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(msg[9..17].try_into()?);
    let west = u64::from_be_bytes(msg[17..25].try_into()?);
    let east = u64::from_be_bytes(msg[25..33].try_into()?);
    let north = u64::from_be_bytes(msg[33..41].try_into()?);
    let south = u64::from_be_bytes(msg[41..49].try_into()?);
    let up = u64::from_be_bytes(msg[49..57].try_into()?);
    let down = u64::from_be_bytes(msg[57..65].try_into()?);

    Ok((action_id, vertex_id, [west, east, north, south, up, down]))
}

/// Parse CreateVertex message from client (with layer support)
/// Returns: (action_id, from_vertex, direction, layer, mime_type, content)
pub fn parse_client_create_vertex(msg: &[u8]) -> Result<(u64, u64, Direction, u32, String, Vec<u8>)> {
    if msg.len() < 1 + 8 + 8 + 1 + 4 {
        return Err(anyhow!("CreateVertex message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let from_vertex = u64::from_be_bytes(msg[9..17].try_into()?);
    let direction = Direction::try_from(msg[17])?;
    let layer = u32::from_be_bytes(msg[18..22].try_into()?);

    // Find null terminator for MIME type
    let mime_end = msg[22..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in MIME type"))?;

    let mime_type = String::from_utf8(msg[22..22 + mime_end].to_vec())?;
    let content = msg[22 + mime_end + 1..].to_vec();

    Ok((action_id, from_vertex, direction, layer, mime_type, content))
}

/// Parse ClickVertex message from client
/// Returns: (action_id, vertex_id)
pub fn parse_click_vertex(msg: &[u8]) -> Result<(u64, u64)> {
    if msg.len() < 1 + 8 + 8 {
        return Err(anyhow!("ClickVertex message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(msg[9..17].try_into()?);
    Ok((action_id, vertex_id))
}

/// Parse DeleteVertex message from client
/// Returns: (action_id, vertex_id)
pub fn parse_client_delete_vertex(msg: &[u8]) -> Result<(u64, u64)> {
    if msg.len() < 1 + 8 + 8 {
        return Err(anyhow!("DeleteVertex message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(msg[9..17].try_into()?);
    Ok((action_id, vertex_id))
}

/// Parse IdentificationResponse message
pub fn parse_identification_response(msg: &[u8]) -> Result<(u64, String, Vec<u8>)> {
    if msg.len() < 1 + 8 + 1 + 64 {
        return Err(anyhow!("IdentificationResponse message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);

    // Find null terminator for identity URL
    let url_end = msg[9..msg.len() - 64]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in identity URL"))?;

    let identity_url = String::from_utf8(msg[9..9 + url_end].to_vec())?;
    let signature = msg[9 + url_end + 1..].to_vec();

    if signature.len() != 64 {
        return Err(anyhow!("Invalid signature length: {} (expected 64)", signature.len()));
    }

    Ok((action_id, identity_url, signature))
}
