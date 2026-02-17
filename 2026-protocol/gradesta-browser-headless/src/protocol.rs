//! Protocol message encoding/decoding for headless browser

use anyhow::{anyhow, Result};

// Client -> Server message types
pub const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
pub const MSG_CLIENT_INTRODUCE_ELF: u8 = 0xA0;

// Server -> Client message types
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;
pub const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;

// ============================================================================
// Encoding (Client -> Server)
// ============================================================================

pub fn encode_watch_landmark(action_id: u64, landmark: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_WATCH_LANDMARK);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(landmark.as_bytes());
    buf
}

pub fn encode_set_vertex_label(action_id: u64, vertex_id: u64, layer: u32, mime: &str, content: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_SET_VERTEX_LABEL);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf.extend_from_slice(&layer.to_be_bytes());
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

pub fn encode_create_vertex(action_id: u64, from_vertex: u64, direction: u8, layer: u32, mime: &str, content: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_CREATE_VERTEX);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&from_vertex.to_be_bytes());
    buf.push(direction);
    buf.extend_from_slice(&layer.to_be_bytes());
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

pub fn encode_click_vertex(action_id: u64, vertex_id: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_CLICK_VERTEX);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf
}

pub fn encode_introduce_elf(
    action_id: u64,
    elf_url: &str,
    command: &str,
    cursor_landmark: &str,
    cursor_vertex: u64,
    region_origin: u64,
    permissions: u8,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_CLIENT_INTRODUCE_ELF);
    buf.extend_from_slice(&action_id.to_be_bytes());
    // elf_url
    buf.extend_from_slice(elf_url.as_bytes());
    buf.push(0);
    // command
    buf.extend_from_slice(command.as_bytes());
    buf.push(0);
    // cursor_landmark
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    // cursor_vertex
    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
    // region spec (use cursor as origin)
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&region_origin.to_be_bytes());
    buf.push(0xFF); // all directions
    buf.extend_from_slice(&(-1i32).to_be_bytes()); // unlimited depth
    // permissions
    buf.push(permissions);
    // params count = 0
    buf.extend_from_slice(&0u16.to_be_bytes());
    buf
}

// ============================================================================
// Parsing (Server -> Client)
// ============================================================================

pub fn parse_set_context(data: &[u8]) -> Result<(u64, String)> {
    if data.len() < 9 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let landmark = String::from_utf8(data[9..].to_vec())?;
    Ok((action_id, landmark))
}

pub fn parse_set_vertex_label(data: &[u8]) -> Result<(u64, u64, u32, String, Vec<u8>)> {
    if data.len() < 21 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
    let layer = u32::from_be_bytes(data[17..21].try_into()?);

    let rest = &data[21..];
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator"))?;
    let mime = String::from_utf8(rest[..null_pos].to_vec())?;
    let content = rest[null_pos + 1..].to_vec();

    Ok((action_id, vertex_id, layer, mime, content))
}

pub fn parse_set_edges(data: &[u8]) -> Result<(u64, u64, [u64; 6], u8)> {
    if data.len() < 66 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
    let mut edges = [0u64; 6];
    for i in 0..6 {
        let start = 17 + i * 8;
        edges[i] = u64::from_be_bytes(data[start..start + 8].try_into()?);
    }
    let edit_mask = data[65];
    Ok((action_id, vertex_id, edges, edit_mask))
}

pub fn parse_log_message(data: &[u8]) -> Result<(u64, u32, u64, String)> {
    if data.len() < 21 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let status = u32::from_be_bytes(data[9..13].try_into()?);
    let vertex_id = u64::from_be_bytes(data[13..21].try_into()?);
    let message = String::from_utf8(data[21..].to_vec())?;
    Ok((action_id, status, vertex_id, message))
}

pub fn parse_introduction_token(data: &[u8]) -> Result<(u64, String, String)> {
    if data.len() < 9 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let rest = &data[9..];

    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator"))?;
    let token = String::from_utf8(rest[..null_pos].to_vec())?;

    let rest = &rest[null_pos + 1..];
    let null_pos = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    let server_ws_url = String::from_utf8(rest[..null_pos].to_vec())?;

    Ok((action_id, token, server_ws_url))
}

/// Parse direction string to direction byte
pub fn direction_to_byte(dir: &str) -> Result<u8> {
    match dir.to_lowercase().as_str() {
        "west" | "w" => Ok(0),
        "east" | "e" => Ok(1),
        "north" | "n" => Ok(2),
        "south" | "s" => Ok(3),
        "up" | "u" => Ok(4),
        "down" | "d" => Ok(5),
        _ => Err(anyhow!("Invalid direction: {}", dir)),
    }
}
