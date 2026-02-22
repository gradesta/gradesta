//! Gradesta protocol message encoding/decoding

use anyhow::{anyhow, Result};
use std::collections::HashMap;

// Server to client message types
pub const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
pub const MSG_SERVER_SET_EDGES: u8 = 0x03;
pub const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
pub const MSG_SERVER_LOG_MESSAGE: u8 = 0x0F;
pub const MSG_SERVER_REQUEST_IDENTIFICATION: u8 = 0x10;
pub const MSG_SERVER_INTRODUCTION_TOKEN: u8 = 0x20;
pub const MSG_SERVER_ELF_TASK: u8 = 0x21;
pub const MSG_SERVER_ELF_OUTPUT_FWD: u8 = 0x22;

// Client to server message types
pub const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
pub const MSG_CLIENT_SET_EDGES: u8 = 0x83;
pub const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
pub const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
pub const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
pub const MSG_CLIENT_DELETE_VERTEX: u8 = 0x87;
pub const MSG_CLIENT_IDENTIFICATION_RESPONSE: u8 = 0x90;
pub const MSG_CLIENT_IDENTIFICATION_REFUSED: u8 = 0x91;

// Browser to server elf messages
pub const MSG_CLIENT_INTRODUCE_ELF: u8 = 0xA0;

// Elf to server message types
pub const MSG_ELF_CONNECT: u8 = 0xB0;
pub const MSG_ELF_OUTPUT: u8 = 0xB1;
pub const MSG_ELF_COMPLETE: u8 = 0xB2;

// Permission bitmask flags
pub const PERM_READ: u8 = 0x01;
pub const PERM_WRITE: u8 = 0x02;
pub const PERM_CREATE: u8 = 0x04;
pub const PERM_DELETE: u8 = 0x08;

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

// ============================================================================
// Elf Protocol Data Structures
// ============================================================================

/// Region specification for elf access scope
#[derive(Clone, Debug)]
pub struct RegionSpec {
    /// Landmark containing the origin vertex
    pub origin_landmark: String,
    /// Origin vertex ID within the landmark
    pub origin_vertex: u64,
    /// Bitmask of allowed directions (DIRECTION_* flags)
    pub allowed_directions: u8,
    /// Maximum traversal depth (-1 for unlimited)
    pub max_depth: i32,
}

impl RegionSpec {
    #[allow(dead_code)] // Used in tests
    pub fn new(origin_landmark: &str, origin_vertex: u64, directions: u8, max_depth: i32) -> Self {
        Self {
            origin_landmark: origin_landmark.to_string(),
            origin_vertex,
            allowed_directions: directions,
            max_depth,
        }
    }

    /// Encode to binary format
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(self.origin_landmark.as_bytes());
        buf.push(0); // null terminator
        buf.extend_from_slice(&self.origin_vertex.to_be_bytes());
        buf.push(self.allowed_directions);
        buf.extend_from_slice(&self.max_depth.to_be_bytes());
        buf
    }

    /// Decode from binary format
    pub fn decode(data: &[u8]) -> Result<(Self, &[u8])> {
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

        Ok((Self {
            origin_landmark,
            origin_vertex,
            allowed_directions,
            max_depth,
        }, &rest[13..]))
    }
}

/// Server-side invitation storing elf access permissions
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields used via Debug trait and in tests
pub struct Invitation {
    /// Opaque token for elf to use
    pub token: String,
    /// Identity of the user who created the invitation
    pub summoner_identity: String,
    /// URL of the elf service
    pub elf_url: String,
    /// Region the elf can access
    pub region: RegionSpec,
    /// Permission bitmask (PERM_* flags)
    pub permissions: u8,
    /// Unix timestamp when invitation expires
    pub expires_at: i64,
    /// Command to execute
    pub command: String,
    /// Command parameters
    pub params: HashMap<String, String>,
}

impl Invitation {
    /// Check if the invitation has expired
    #[allow(dead_code)] // Used by validate_token which is test-only
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now > self.expires_at
    }

    /// Check if the invitation allows a permission
    pub fn has_permission(&self, perm: u8) -> bool {
        (self.permissions & perm) != 0
    }
}

// ============================================================================
// Elf Message Encoding
// ============================================================================

/// Parse IntroduceElf message (Browser→Server)
pub fn parse_introduce_elf(msg: &[u8]) -> Result<(u64, String, String, String, u64, RegionSpec, u8, HashMap<String, String>)> {
    if msg.len() < 1 + 8 {
        return Err(anyhow!("IntroduceElf message too short"));
    }
    let action_id = u64::from_be_bytes(msg[1..9].try_into()?);
    let mut rest = &msg[9..];

    // Parse elf_url
    let null_pos = rest.iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in elf_url"))?;
    let elf_url = String::from_utf8(rest[..null_pos].to_vec())?;
    rest = &rest[null_pos + 1..];

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
        return Err(anyhow!("IntroduceElf message too short for cursor_vertex"));
    }
    let cursor_vertex = u64::from_be_bytes(rest[..8].try_into()?);
    rest = &rest[8..];

    // Parse region
    let (region, remaining) = RegionSpec::decode(rest)?;
    rest = remaining;

    // Parse permissions
    if rest.is_empty() {
        return Err(anyhow!("IntroduceElf message too short for permissions"));
    }
    let permissions = rest[0];
    rest = &rest[1..];

    // Parse params
    if rest.len() < 2 {
        return Err(anyhow!("IntroduceElf message too short for params count"));
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

    Ok((action_id, elf_url, command, cursor_landmark, cursor_vertex, region, permissions, params))
}

/// Encode IntroductionToken message (Server→Browser)
/// Format: [type:1][action_id:8][token\0]
pub fn encode_introduction_token(action_id: u64, token: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 8 + token.len() + 1);
    buf.push(MSG_SERVER_INTRODUCTION_TOKEN);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.extend_from_slice(token.as_bytes());
    buf.push(0);
    buf
}

/// Parse ElfConnect message (Elf→Server)
pub fn parse_elf_connect(msg: &[u8]) -> Result<String> {
    if msg.len() < 2 {
        return Err(anyhow!("ElfConnect message too short"));
    }
    let null_pos = msg[1..].iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in elf connect token"))?;
    let token = String::from_utf8(msg[1..1 + null_pos].to_vec())?;
    Ok(token)
}

/// Encode ElfTask message (Server→Elf)
/// Format: [type:1][command\0][cursor_landmark\0][cursor_vertex:8][cursor_mime\0][cursor_content_len:4][cursor_content][region][params_count:2][params...]
pub fn encode_elf_task(
    command: &str,
    cursor_landmark: &str,
    cursor_vertex: u64,
    cursor_mime: &str,
    cursor_content: &[u8],
    region: &RegionSpec,
    params: &HashMap<String, String>,
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSG_SERVER_ELF_TASK);
    buf.extend_from_slice(command.as_bytes());
    buf.push(0);
    buf.extend_from_slice(cursor_landmark.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&cursor_vertex.to_be_bytes());
    // Include cursor vertex content so elf doesn't need to request it
    buf.extend_from_slice(cursor_mime.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&(cursor_content.len() as u32).to_be_bytes());
    buf.extend_from_slice(cursor_content);
    buf.extend_from_slice(&region.encode());
    buf.extend_from_slice(&(params.len() as u16).to_be_bytes());
    for (k, v) in params {
        buf.extend_from_slice(k.as_bytes());
        buf.push(0);
        buf.extend_from_slice(v.as_bytes());
        buf.push(0);
    }
    buf
}

/// Parse ElfOutput message (Elf→Server)
pub fn parse_elf_output(msg: &[u8]) -> Result<(u8, Vec<u8>)> {
    if msg.len() < 2 {
        return Err(anyhow!("ElfOutput message too short"));
    }
    let output_type = msg[1];
    let data = msg[2..].to_vec();
    Ok((output_type, data))
}

/// Parse ElfComplete message (Elf→Server)
pub fn parse_elf_complete(msg: &[u8]) -> Result<(u32, String)> {
    if msg.len() < 1 + 4 {
        return Err(anyhow!("ElfComplete message too short"));
    }
    let status = u32::from_be_bytes(msg[1..5].try_into()?);
    let null_pos = msg[5..].iter().position(|&b| b == 0)
        .ok_or_else(|| anyhow!("No null terminator in message"))?;
    let message = String::from_utf8(msg[5..5 + null_pos].to_vec())?;
    Ok((status, message))
}

/// Encode ElfOutputFwd message (Server→Browser)
/// Format: [type:1][action_id:8][output_type:1][data...]
pub fn encode_elf_output_fwd(action_id: u64, output_type: u8, data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + 8 + 1 + data.len());
    buf.push(MSG_SERVER_ELF_OUTPUT_FWD);
    buf.extend_from_slice(&action_id.to_be_bytes());
    buf.push(output_type);
    buf.extend_from_slice(data);
    buf
}
