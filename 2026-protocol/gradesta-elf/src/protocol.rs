//! Protocol message encoding/decoding for elf communication

use anyhow::{anyhow, Result};
use std::collections::HashMap;

use crate::{ElfTask, RegionSpec};

// Message type constants (elf-related)
pub const MSG_ELF_CONNECT: u8 = 0xB0;
pub const MSG_ELF_OUTPUT: u8 = 0xB1;
pub const MSG_ELF_COMPLETE: u8 = 0xB2;
pub const MSG_SERVER_ELF_TASK: u8 = 0x21;

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
