//! Protocol message encoding/decoding for elf communication

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::{ElfTask, RegionSpec};

// Message type constants (elf-related)
pub const MSG_ELF_CONNECT: u8 = 0xB0;
pub const MSG_ELF_OUTPUT: u8 = 0xB1;
pub const MSG_ELF_COMPLETE: u8 = 0xB2;
pub const MSG_SERVER_ELF_TASK: u8 = 0x21;

// Standard client message types (elves use same protocol as browser)
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;

// Standard server message types
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;

// Direction constants
pub const DIR_WEST: u8 = 0;
pub const DIR_EAST: u8 = 1;
pub const DIR_NORTH: u8 = 2;
pub const DIR_SOUTH: u8 = 3;
pub const DIR_UP: u8 = 4;
pub const DIR_DOWN: u8 = 5;

/// Encode ELF_CONNECT message
pub fn encode_elf_connect(token: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + token.len() + 1);
    buf.push(MSG_ELF_CONNECT);
    buf.extend_from_slice(token.as_bytes());
    buf.push(0);
    buf
}

/// Encode ELF_OUTPUT message
pub fn encode_elf_output(output_type: u8, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(2 + data.len());
    buf.push(MSG_ELF_OUTPUT);
    buf.push(output_type);
    buf.extend_from_slice(data);
    buf
}

/// Encode ELF_COMPLETE message
pub fn encode_elf_complete(status: u32, message: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 4 + message.len() + 1);
    buf.push(MSG_ELF_COMPLETE);
    buf.extend_from_slice(&status.to_be_bytes());
    buf.extend_from_slice(message.as_bytes());
    buf.push(0);
    buf
}

/// Parse ELF_TASK message
/// Format: [type:1][command\0][cursor_landmark\0][cursor_vertex:8][cursor_mime\0][cursor_content_len:4][cursor_content][region][params_count:2][params...]
pub fn parse_elf_task(data: &[u8]) -> Result<ElfTask> {
    if data.is_empty() || data[0] != MSG_SERVER_ELF_TASK {
        return Err(anyhow!("Not an ELF_TASK message"));
    }

    let mut rest = &data[1..];

    // Parse command
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in command"))?;
    let command = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse cursor_landmark
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in cursor_landmark"))?;
    let cursor_landmark = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse cursor_vertex
    if rest.len() < 8 {
        return Err(anyhow!("ElfTask message too short for cursor_vertex"));
    }
    let cursor_vertex = u64::from_be_bytes(rest[..8].try_into()?);
    rest = &rest[8..];

    // Parse cursor_mime
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in cursor_mime"))?;
    let cursor_mime = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

    // Parse cursor_content
    if rest.len() < 4 {
        return Err(anyhow!("ElfTask message too short for cursor_content_len"));
    }
    let cursor_content_len = u32::from_be_bytes(rest[..4].try_into()?) as usize;
    rest = &rest[4..];
    if rest.len() < cursor_content_len {
        return Err(anyhow!("ElfTask message too short for cursor_content"));
    }
    let cursor_content = rest[..cursor_content_len].to_vec();
    rest = &rest[cursor_content_len..];

    // Parse region
    let (region, remaining) = parse_region_spec(rest)?;
    rest = remaining;

    // Parse params
    if rest.len() < 2 {
        return Err(anyhow!("ElfTask message too short for params count"));
    }
    let params_count = u16::from_be_bytes(rest[..2].try_into()?) as usize;
    rest = &rest[2..];

    let mut params = HashMap::new();
    for _ in 0..params_count {
        let null_pos = rest.iter().position(|&b| b == 0)
            .ok_or_else(|| anyhow!("No null terminator in param key"))?;
        let key = String::from_utf8(rest[..null_pos].to_vec())?;
        rest = &rest[null_pos + 1..];

        let null_pos = rest.iter().position(|&b| b == 0)
            .ok_or_else(|| anyhow!("No null terminator in param value"))?;
        let value = String::from_utf8(rest[..null_pos].to_vec())?;
        rest = &rest[null_pos + 1..];

        params.insert(key, value);
    }

    Ok(ElfTask {
        command,
        cursor_landmark,
        cursor_vertex,
        cursor_mime,
        cursor_content,
        region,
        params,
    })
}

fn parse_region_spec(data: &[u8]) -> Result<(RegionSpec, &[u8])> {
    // Find null terminator for landmark
    let landmark_end = data.iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in region landmark"))?;
    let origin_landmark = String::from_utf8(data[..landmark_end].to_vec())?;
    let rest = &data[landmark_end + 1..];

    if rest.len() < 8 + 1 + 4 {
        return Err(anyhow!("RegionSpec too short"));
    }
    let origin_vertex = u64::from_be_bytes(rest[..8].try_into()?);
    let allowed_directions = rest[8];
    let max_depth = i32::from_be_bytes(rest[9..13].try_into()?);

    Ok((RegionSpec {
        origin_landmark,
        origin_vertex,
        allowed_directions,
        max_depth,
    }, &rest[13..]))
}

// ============================================================================
// Standard Protocol Messages (same as browser uses)
// ============================================================================

/// Encode SetVertexLabel message (layer 0)
/// Format: [type:1][action_id:8][vertex_id:8][layer:4][mime\0][content...]
pub fn encode_set_vertex_label(action_id: u64, vertex_id: u64, mime: &str, content: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 8 + 8 + 4 + mime.len() + 1 + content.len());
    buf.push(MSG_CLIENT_SET_VERTEX_LABEL);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes()); // layer 0
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

/// Encode ClickVertex message (to request vertex content)
/// Format: [type:1][action_id:8][vertex_id:8]
pub fn encode_click_vertex(action_id: u64, vertex_id: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 8 + 8);
    buf.push(MSG_CLIENT_CLICK_VERTEX);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&vertex_id.to_be_bytes());
    buf
}

/// Encode CreateVertex message
/// Format: [type:1][action_id:8][from_vertex:8][direction:1][layer:4][mime\0][content...]
pub fn encode_create_vertex(
    action_id: u64,
    from_vertex: u64,
    direction: u8,
    mime: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 8 + 8 + 1 + 4 + mime.len() + 1 + content.len());
    buf.push(MSG_CLIENT_CREATE_VERTEX);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(&from_vertex.to_be_bytes());
    buf.push(direction);
    buf.extend_from_slice(&0u32.to_be_bytes()); // layer 0
    buf.extend_from_slice(mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(content);
    buf
}

/// Parse SetVertexLabel message from server
/// Returns: (action_id, vertex_id, layer, mime, content)
pub fn parse_server_set_vertex_label(data: &[u8]) -> Result<(u64, u64, u32, String, Vec<u8>)> {
    if data.is_empty() || data[0] != MSG_SERVER_SET_VERTEX_LABEL {
        return Err(anyhow!("Not a SetVertexLabel message"));
    }
    if data.len() < 1 + 8 + 8 + 4 {
        return Err(anyhow!("SetVertexLabel message too short"));
    }

    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let vertex_id = u64::from_be_bytes(data[9..17].try_into()?);
    let layer = u32::from_be_bytes(data[17..21].try_into()?);

    let rest = &data[21..];
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in mime type"))?;
    let mime = String::from_utf8(rest[..null_pos].to_vec())?;
    let content = rest[null_pos + 1..].to_vec();

    Ok((action_id, vertex_id, layer, mime, content))
}

/// Parse LogMessage from server
/// Returns: (action_id, status, vertex_id, message)
pub fn parse_server_log_message(data: &[u8]) -> Result<(u64, u32, u64, String)> {
    if data.is_empty() || data[0] != MSG_SERVER_LOG_MESSAGE {
        return Err(anyhow!("Not a LogMessage"));
    }
    if data.len() < 1 + 8 + 4 + 8 {
        return Err(anyhow!("LogMessage too short"));
    }

    let action_id = u64::from_be_bytes(data[1..9].try_into()?);
    let status = u32::from_be_bytes(data[9..13].try_into()?);
    let vertex_id = u64::from_be_bytes(data[13..21].try_into()?);
    let message = String::from_utf8(data[21..].to_vec())?;

    Ok((action_id, status, vertex_id, message))
}

/// Parse SetEdges from server (used to get new vertex ID after create)
/// Returns: (action_id, vertex_id, edges[6], edit_mask)
pub fn parse_server_set_edges(data: &[u8]) -> Result<(u64, u64, [u64; 6], u8)> {
    if data.is_empty() || data[0] != MSG_SERVER_SET_EDGES {
        return Err(anyhow!("Not a SetEdges message"));
    }
    if data.len() < 1 + 8 + 8 + 48 + 1 {
        return Err(anyhow!("SetEdges message too short"));
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
