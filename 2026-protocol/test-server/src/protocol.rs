//! Protocol message encoding/decoding

use anyhow::{anyhow, Result};
use std::collections::HashMap;

// Message type constants
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;
pub const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;
pub const MSG_SERVER_ELF_TASK: u8 = 0x21;

pub const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
pub const MSG_CLIENT_INTRODUCE_ELF: u8 = 0xA0;

pub const MSG_ELF_CONNECT: u8 = 0xB0;
pub const MSG_ELF_OUTPUT: u8 = 0xB1;
pub const MSG_ELF_COMPLETE: u8 = 0xB2;

pub const PERM_READ: u8 = 0x01;
pub const PERM_WRITE: u8 = 0x02;
pub const PERM_CREATE: u8 = 0x04;
pub const PERM_DELETE: u8 = 0x08;

/// Region specification
pub struct RegionSpec {
    pub origin_landmark: String,
    pub origin_vertex: u64,
    pub allowed_directions: u8,
    pub max_depth: i32,
}

// ============================================================================
// Encoding
// ============================================================================

pub fn encode_set_context(action_id: u64, landmark: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_SET_CONTEXT);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(landmark.as_bytes());
    buf
}

pub fn encode_set_vertex_label(action_id: u64, vertex_id: u64, mime: &str, content: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_SET_VERTEX_LABEL);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes()); // layer 0
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

pub fn encode_set_edges(action_id: u64, vertex_id: u64, edges: [u64; 6], edit_mask: u8) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_SET_EDGES);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    for edge in &edges {
        buf.extend_from_slice(&edge.to_be_bytes());
    }
    buf.push(edit_mask);
    buf
}

pub fn encode_log_message(action_id: u64, status: u32, vertex_id: u64, message: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_LOG_MESSAGE);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&status.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf.extend_from_slice(message.as_bytes());
    buf
}

pub fn encode_introduction_token(action_id: u64, token: &str, server_ws_url: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_INTRODUCTION_TOKEN);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(token.as_bytes());
    buf.push(0);
    buf.extend_from_slice(server_ws_url.as_bytes());
    buf.push(0);
    buf
}

pub fn encode_elf_task(
    command: &str,
    cursor_landmark: &str,
    cursor_vertex: u64,
    region_origin: u64,
    permissions: u8,
    params: &HashMap<String, String>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_ELF_TASK);
    buf.extend_from_slice(command.as_bytes());
    buf.push(0);
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
    // Region spec
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&region_origin.to_be_bytes());
    buf.push(0xFF); // all directions
    buf.extend_from_slice(&(-1i32).to_be_bytes()); // unlimited depth
    // Params
    buf.extend_from_slice(&(params.len() as u16).to_be_bytes());
    for (k, v) in params {
        buf.extend_from_slice(k.as_bytes());
        buf.push(0);
        buf.extend_from_slice(v.as_bytes());
        buf.push(0);
    }
    buf
}

// ============================================================================
// Parsing
// ============================================================================

pub fn parse_watch_landmark(data: &[u8]) -> Result<(u64, String)> {
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

pub fn parse_create_vertex(data: &[u8]) -> Result<(u64, u64, u8, u32, String, Vec<u8>)> {
    if data.len() < 22 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let from_vertex = u64::from_be_bytes(data[9..17].try_into()?);
    let direction = data[17];
    let layer = u32::from_be_bytes(data[18..22].try_into()?);

    let rest = &data[22..];
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator"))?;
    let mime = String::from_utf8(rest[..null_pos].to_vec())?;
    let content = rest[null_pos + 1..].to_vec();

    Ok((action_id, from_vertex, direction, layer, mime, content))
}

pub fn parse_click_vertex(data: &[u8]) -> Result<(u64, u64)> {
    if data.len() < 17 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
    Ok((action_id, vertex_id))
}

pub fn parse_introduce_elf(data: &[u8]) -> Result<(u64, String, String, String, u64, RegionSpec, u8, HashMap<String, String>)> {
    if data.len() < 9 {
        return Err(anyhow!("Message too short"));
    }
    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let mut rest = &data[9..];

    // Parse elf_url
    let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
    let elf_url = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse command
    let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
    let command = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse cursor_landmark
    let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
    let cursor_landmark = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse cursor_vertex
    let cursor_vertex = u64::from_be_bytes(rest[..8].try_into()?);
    rest = &rest[8..];

    // Parse region
    let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
    let origin_landmark = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    let origin_vertex = u64::from_be_bytes(rest[..8].try_into()?);
    let allowed_directions = rest[8];
    let max_depth = i32::from_be_bytes(rest[9..13].try_into()?);
    rest = &rest[13..];

    let region = RegionSpec {
        origin_landmark,
        origin_vertex,
        allowed_directions,
        max_depth,
    };

    // Parse permissions
    let permissions = rest[0];
    rest = &rest[1..];

    // Parse params
    let params_count = u16::from_be_bytes(rest[..2].try_into()?) as usize;
    rest = &rest[2..];

    let mut params = HashMap::new();
    for _ in 0..params_count {
        let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
        let key = String::from_utf8(rest[..null_pos].to_vec())?;
        rest = &rest[null_pos + 1..];

        let null_pos = rest.iter().position(|&b| b == 0).ok_or_else(|| anyhow!("No null"))?;
        let value = String::from_utf8(rest[..null_pos].to_vec())?;
        rest = &rest[null_pos + 1..];

        params.insert(key, value);
    }

    Ok((action_id, elf_url, command, cursor_landmark, cursor_vertex, region, permissions, params))
}

pub fn parse_elf_connect(data: &[u8]) -> Result<String> {
    if data.len() < 2 {
        return Err(anyhow!("Message too short"));
    }
    let null_pos = data[1..].iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator"))?;
    let token = String::from_utf8(data[1..1 + null_pos].to_vec())?;
    Ok(token)
}

pub fn parse_elf_output(data: &[u8]) -> Result<(u8, Vec<u8>)> {
    if data.len() < 2 {
        return Err(anyhow!("Message too short"));
    }
    let output_type = data[1];
    let output_data = data[2..].to_vec();
    Ok((output_type, output_data))
}

pub fn parse_elf_complete(data: &[u8]) -> Result<(u32, String)> {
    if data.len() < 5 {
        return Err(anyhow!("Message too short"));
    }
    let status = u32::from_be_bytes(data[1..5].try_into()?);
    let null_pos = data[5..].iter().position(|&b| b == 0).unwrap_or(data.len() - 5);
    let message = String::from_utf8(data[5..5 + null_pos].to_vec())?;
    Ok((status, message))
}
