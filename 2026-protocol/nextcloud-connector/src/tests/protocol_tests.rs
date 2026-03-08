//! Protocol encoding tests
//!
//! Tests for binary protocol message encoding and decoding,
//! ensuring correct format for all message types.

use crate::notes::uuid_to_hash;
use crate::protocol::{
    encode_log_message, encode_set_edges, encode_set_vertex_label, encode_set_vertex_label_layer,
    parse_client_create_vertex, parse_client_delete_vertex, parse_client_set_edges,
    parse_client_set_vertex_label, Direction, MSG_SERVER_LOG_MESSAGE, MSG_SERVER_SET_EDGES,
    MSG_SERVER_SET_VERTEX_LABEL, STATUS_ACCEPTED, STATUS_CONFLICT, STATUS_OK,
};

/// Test SetEdges encoding follows [west, east, north, south, up, down] order
#[test]
fn test_set_edges_encoding() {
    let action_id = 42u64;
    let vertex_id = 12345u64;
    let west = 100u64;
    let east = 200u64;
    let north = 300u64;
    let south = 400u64;
    let up = 500u64;
    let down = 600u64;
    let edit_mask = 0x7F;

    let encoded = encode_set_edges(action_id, vertex_id, west, east, north, south, up, down, edit_mask);

    // Verify structure
    assert_eq!(encoded[0], MSG_SERVER_SET_EDGES);
    assert_eq!(u64::from_be_bytes(encoded[1..9].try_into().unwrap()), action_id);
    assert_eq!(u64::from_be_bytes(encoded[9..17].try_into().unwrap()), vertex_id);
    assert_eq!(u64::from_be_bytes(encoded[17..25].try_into().unwrap()), west);
    assert_eq!(u64::from_be_bytes(encoded[25..33].try_into().unwrap()), east);
    assert_eq!(u64::from_be_bytes(encoded[33..41].try_into().unwrap()), north);
    assert_eq!(u64::from_be_bytes(encoded[41..49].try_into().unwrap()), south);
    assert_eq!(u64::from_be_bytes(encoded[49..57].try_into().unwrap()), up);
    assert_eq!(u64::from_be_bytes(encoded[57..65].try_into().unwrap()), down);
    assert_eq!(encoded[65], edit_mask);
}

/// Test SetVertexLabel encoding with layer field
#[test]
fn test_set_vertex_label_layer_encoding() {
    let action_id = 1u64;
    let vertex_id = 999u64;
    let layer = 1u32;
    let mime = "text/plain";
    let content = b"Hello, world!";

    let encoded = encode_set_vertex_label_layer(action_id, vertex_id, layer, mime, content);

    // Verify structure
    assert_eq!(encoded[0], MSG_SERVER_SET_VERTEX_LABEL);
    assert_eq!(u64::from_be_bytes(encoded[1..9].try_into().unwrap()), action_id);
    assert_eq!(u64::from_be_bytes(encoded[9..17].try_into().unwrap()), vertex_id);
    assert_eq!(u32::from_be_bytes(encoded[17..21].try_into().unwrap()), layer);

    // Find null terminator for mime
    let mime_end = encoded[21..].iter().position(|&b| b == 0).unwrap();
    let decoded_mime = std::str::from_utf8(&encoded[21..21 + mime_end]).unwrap();
    assert_eq!(decoded_mime, mime);

    // Content follows null terminator
    let decoded_content = &encoded[21 + mime_end + 1..];
    assert_eq!(decoded_content, content);
}

/// Test LogMessage encoding with status codes
#[test]
fn test_log_message_status_codes() {
    for (status, name) in [(STATUS_OK, "OK"), (STATUS_ACCEPTED, "Accepted"), (STATUS_CONFLICT, "Conflict")] {
        let action_id = 100u64;
        let vertex_id = 200u64;
        let message = format!("Test {}", name);

        let encoded = encode_log_message(action_id, status, vertex_id, &message);

        // Verify structure
        assert_eq!(encoded[0], MSG_SERVER_LOG_MESSAGE);
        assert_eq!(u64::from_be_bytes(encoded[1..9].try_into().unwrap()), action_id);
        assert_eq!(u32::from_be_bytes(encoded[9..13].try_into().unwrap()), status);
        assert_eq!(u64::from_be_bytes(encoded[13..21].try_into().unwrap()), vertex_id);
        assert_eq!(std::str::from_utf8(&encoded[21..]).unwrap(), message);
    }
}

/// Test edge array hash consistency (same UUID always produces same hash)
#[test]
fn test_edge_array_hash_consistency() {
    let uuid1 = uuid::Uuid::new_v4();
    let uuid2 = uuid::Uuid::new_v4();

    // Same UUID should always produce same hash
    let hash1a = uuid_to_hash(uuid1);
    let hash1b = uuid_to_hash(uuid1);
    assert_eq!(hash1a, hash1b);

    // Different UUIDs should produce different hashes (with high probability)
    let hash2 = uuid_to_hash(uuid2);
    assert_ne!(hash1a, hash2);
}

/// Test SetVertexLabel layer 0 encoding (convenience wrapper)
#[test]
fn test_set_vertex_label_layer0() {
    let action_id = 5u64;
    let vertex_id = 123u64;
    let mime = "audio/ogg";
    let content = b"binary data here";

    let encoded = encode_set_vertex_label(action_id, vertex_id, mime, content);

    // Should have layer 0
    assert_eq!(u32::from_be_bytes(encoded[17..21].try_into().unwrap()), 0);
}

/// Test parsing client SetVertexLabel message
#[test]
fn test_parse_client_set_vertex_label() {
    // Build a client message
    let mut msg = vec![0x85]; // MSG_CLIENT_SET_VERTEX_LABEL
    msg.extend_from_slice(&42u64.to_be_bytes()); // action_id
    msg.extend_from_slice(&123u64.to_be_bytes()); // vertex_id
    msg.extend_from_slice(&1u32.to_be_bytes()); // layer
    msg.extend_from_slice(b"text/plain\0"); // mime + null
    msg.extend_from_slice(b"Hello content"); // content

    let (action_id, vertex_id, layer, mime, content) = parse_client_set_vertex_label(&msg).unwrap();

    assert_eq!(action_id, 42);
    assert_eq!(vertex_id, 123);
    assert_eq!(layer, 1);
    assert_eq!(mime, "text/plain");
    assert_eq!(content, b"Hello content");
}

/// Test parsing client SetEdges message
#[test]
fn test_parse_client_set_edges() {
    let mut msg = vec![0x83]; // MSG_CLIENT_SET_EDGES
    msg.extend_from_slice(&1u64.to_be_bytes()); // action_id
    msg.extend_from_slice(&2u64.to_be_bytes()); // vertex_id
    msg.extend_from_slice(&10u64.to_be_bytes()); // west
    msg.extend_from_slice(&20u64.to_be_bytes()); // east
    msg.extend_from_slice(&30u64.to_be_bytes()); // north
    msg.extend_from_slice(&40u64.to_be_bytes()); // south
    msg.extend_from_slice(&50u64.to_be_bytes()); // up
    msg.extend_from_slice(&60u64.to_be_bytes()); // down

    let (action_id, vertex_id, edges) = parse_client_set_edges(&msg).unwrap();

    assert_eq!(action_id, 1);
    assert_eq!(vertex_id, 2);
    assert_eq!(edges, [10, 20, 30, 40, 50, 60]);
}

/// Test parsing client CreateVertex message
#[test]
fn test_parse_client_create_vertex() {
    let mut msg = vec![0x86]; // MSG_CLIENT_CREATE_VERTEX
    msg.extend_from_slice(&100u64.to_be_bytes()); // action_id
    msg.extend_from_slice(&200u64.to_be_bytes()); // from_vertex
    msg.push(1); // direction (East)
    msg.extend_from_slice(&0u32.to_be_bytes()); // layer
    msg.extend_from_slice(b"text/markdown\0"); // mime + null
    msg.extend_from_slice(b"# New Note"); // content

    let (action_id, from_vertex, direction, layer, mime, content) =
        parse_client_create_vertex(&msg).unwrap();

    assert_eq!(action_id, 100);
    assert_eq!(from_vertex, 200);
    assert_eq!(direction, Direction::East);
    assert_eq!(layer, 0);
    assert_eq!(mime, "text/markdown");
    assert_eq!(content, b"# New Note");
}

/// Test parsing client DeleteVertex message
#[test]
fn test_parse_client_delete_vertex() {
    let mut msg = vec![0x87]; // MSG_CLIENT_DELETE_VERTEX
    msg.extend_from_slice(&50u64.to_be_bytes()); // action_id
    msg.extend_from_slice(&999u64.to_be_bytes()); // vertex_id

    let (action_id, vertex_id) = parse_client_delete_vertex(&msg).unwrap();

    assert_eq!(action_id, 50);
    assert_eq!(vertex_id, 999);
}

/// Test Direction encoding
#[test]
fn test_direction_encoding() {
    assert_eq!(Direction::West as u8, 0);
    assert_eq!(Direction::East as u8, 1);
    assert_eq!(Direction::North as u8, 2);
    assert_eq!(Direction::South as u8, 3);
    assert_eq!(Direction::Up as u8, 4);
    assert_eq!(Direction::Down as u8, 5);
}

/// Test Direction parsing
#[test]
fn test_direction_parsing() {
    assert_eq!(Direction::try_from(0).unwrap(), Direction::West);
    assert_eq!(Direction::try_from(1).unwrap(), Direction::East);
    assert_eq!(Direction::try_from(2).unwrap(), Direction::North);
    assert_eq!(Direction::try_from(3).unwrap(), Direction::South);
    assert_eq!(Direction::try_from(4).unwrap(), Direction::Up);
    assert_eq!(Direction::try_from(5).unwrap(), Direction::Down);
    assert!(Direction::try_from(6).is_err());
}

/// Test edit mask interpretation
#[test]
fn test_edit_mask() {
    // 0x7F = all directions editable
    let full_edit = 0x7F_u8;
    assert_eq!(full_edit & 0x01, 0x01); // west
    assert_eq!(full_edit & 0x02, 0x02); // east
    assert_eq!(full_edit & 0x04, 0x04); // north
    assert_eq!(full_edit & 0x08, 0x08); // south
    assert_eq!(full_edit & 0x10, 0x10); // up
    assert_eq!(full_edit & 0x20, 0x20); // down
    assert_eq!(full_edit & 0x40, 0x40); // content editable

    // 0x00 = nothing editable (deleted/non-existent vertex)
    let no_edit = 0x00_u8;
    assert_eq!(no_edit & 0x7F, 0);
}

/// Test message type byte constants
#[test]
fn test_message_type_constants() {
    use crate::protocol::*;

    // Server messages are in 0x00-0x7F range
    assert!(MSG_SERVER_SET_CONTEXT < 0x80);
    assert!(MSG_SERVER_SET_EDGES < 0x80);
    assert!(MSG_SERVER_SET_VERTEX_LABEL < 0x80);
    assert!(MSG_SERVER_LOG_MESSAGE < 0x80);

    // Client messages are in 0x80-0xAF range
    assert!(MSG_CLIENT_WATCH_LANDMARK >= 0x80);
    assert!(MSG_CLIENT_SET_EDGES >= 0x80);
    assert!(MSG_CLIENT_CREATE_VERTEX >= 0x80);
    assert!(MSG_CLIENT_DELETE_VERTEX >= 0x80);

    // Elf messages are in 0xB0-0xBF range
    assert!(MSG_ELF_CONNECT >= 0xB0);
    assert!(MSG_ELF_OUTPUT >= 0xB0);
    assert!(MSG_ELF_COMPLETE >= 0xB0);
}

/// Test status code constants
#[test]
fn test_status_code_constants() {
    use crate::protocol::*;

    assert_eq!(STATUS_OK, 200);
    assert_eq!(STATUS_ACCEPTED, 202);
    assert_eq!(STATUS_BAD_REQUEST, 400);
    assert_eq!(STATUS_UNAUTHORIZED, 401);
    assert_eq!(STATUS_FORBIDDEN, 403);
    assert_eq!(STATUS_NOT_FOUND, 404);
    assert_eq!(STATUS_CONFLICT, 409);
    assert_eq!(STATUS_INTERNAL_ERROR, 500);
}
