//! Vertex CRUD operation tests
//!
//! Tests for create, read, update, and delete operations on vertices,
//! including chain handling and edge updates.

use uuid::Uuid;

use crate::notes::{uuid_to_hash, CONTENT_DIR};
use crate::protocol::Direction;
use crate::webdav::WebDavClient;

use super::harness::{add_test_edge, create_test_vertex, TestHarness};

/// Test basic vertex creation
#[tokio::test]
async fn test_create_vertex_basic() {
    let harness = TestHarness::new().await;

    // Create a vertex
    let (uuid, hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/test.txt",
        Some("Test content".to_string()),
    )
    .await;

    // Verify index updated
    let index = harness.index.lock().await;
    assert_eq!(index.vertices.len(), 1);

    let vertex = index.get_vertex(uuid).unwrap();
    assert_eq!(vertex.mime, "text/plain");
    assert_eq!(vertex.transcript, Some("Test content".to_string()));

    // Verify hash lookup works
    let by_hash = index.get_vertex_by_hash(hash);
    assert!(by_hash.is_some());
    assert_eq!(by_hash.unwrap().id, uuid);
}

/// Test vertex creation with chain linking
#[tokio::test]
async fn test_create_vertex_with_chain() {
    let harness = TestHarness::new().await;

    // Create first vertex (will be the "from" vertex)
    let (uuid_a, hash_a) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/a.txt",
        Some("Vertex A".to_string()),
    )
    .await;

    // Create second vertex
    let (uuid_b, hash_b) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/b.txt",
        Some("Vertex B".to_string()),
    )
    .await;

    // Add edge A -> B (east)
    add_test_edge(&harness.index, uuid_a, uuid_b, Direction::East).await;

    // Verify edges
    let index = harness.index.lock().await;
    let edges = index.build_edge_array(uuid_a);
    assert_eq!(edges[1], hash_b); // East edge points to B

    let edges_b = index.build_edge_array(uuid_b);
    assert_eq!(edges_b[0], hash_a); // West edge points back to A
}

/// Test vertex creation with displacement
#[tokio::test]
async fn test_create_vertex_displacement() {
    let harness = TestHarness::new().await;

    // Create chain: A -> B
    let (uuid_a, hash_a) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/a.txt",
        None,
    )
    .await;

    let (uuid_b, hash_b) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/b.txt",
        None,
    )
    .await;

    add_test_edge(&harness.index, uuid_a, uuid_b, Direction::East).await;

    // Now insert X between A and B
    let (uuid_x, hash_x) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/x.txt",
        None,
    )
    .await;

    // Use insert_vertex to properly handle displacement
    {
        let mut index = harness.index.lock().await;
        let displaced = index.insert_vertex(uuid_a, uuid_x, Direction::East);
        assert_eq!(displaced, Some(uuid_b)); // B was displaced
    }

    // Verify final chain: A -> X -> B
    let index = harness.index.lock().await;

    let edges_a = index.build_edge_array(uuid_a);
    assert_eq!(edges_a[1], hash_x); // A's east now points to X

    let edges_x = index.build_edge_array(uuid_x);
    assert_eq!(edges_x[0], hash_a); // X's west points to A
    assert_eq!(edges_x[1], hash_b); // X's east points to B

    let edges_b = index.build_edge_array(uuid_b);
    assert_eq!(edges_b[0], hash_x); // B's west now points to X
}

/// Test that content file path is correctly formatted
#[tokio::test]
async fn test_create_vertex_file_path() {
    let harness = TestHarness::new().await;

    let uuid = Uuid::new_v4();
    let expected_file = format!("{}/{}.txt", CONTENT_DIR, uuid);

    let mut index = harness.index.lock().await;
    index.vertices.push(crate::notes::Vertex {
        id: uuid,
        mime: "text/plain".to_string(),
        file: expected_file.clone(),
        transcript: None,
        layers: std::collections::HashMap::new(),
        created: chrono::Utc::now(),
        modified: None,
    });

    let vertex = index.get_vertex(uuid).unwrap();
    assert!(vertex.file.starts_with(CONTENT_DIR));
    assert!(vertex.file.ends_with(".txt"));
}

/// Test basic vertex deletion
#[tokio::test]
async fn test_delete_vertex_basic() {
    let harness = TestHarness::new().await;

    // Create a vertex
    let (uuid, _hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/test.txt",
        None,
    )
    .await;

    // Verify it exists
    {
        let index = harness.index.lock().await;
        assert!(index.get_vertex(uuid).is_some());
    }

    // Delete it
    {
        let mut index = harness.index.lock().await;
        let (files, neighbors) = index.delete_vertex(uuid).unwrap();
        assert_eq!(files.len(), 1);
        assert!(neighbors.is_empty()); // No neighbors
    }

    // Verify it's gone
    {
        let index = harness.index.lock().await;
        assert!(index.get_vertex(uuid).is_none());
    }
}

/// Test deletion with chain reconnection (delete middle vertex)
#[tokio::test]
async fn test_delete_vertex_chain_reconnection() {
    let harness = TestHarness::new().await;

    // Create chain: A -> X -> B
    let (uuid_a, hash_a) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/a.txt",
        None,
    )
    .await;

    let (uuid_x, _hash_x) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/x.txt",
        None,
    )
    .await;

    let (uuid_b, hash_b) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/b.txt",
        None,
    )
    .await;

    // Set up edges
    add_test_edge(&harness.index, uuid_a, uuid_x, Direction::East).await;
    add_test_edge(&harness.index, uuid_x, uuid_b, Direction::East).await;

    // Delete X
    {
        let mut index = harness.index.lock().await;
        let (files, neighbors) = index.delete_vertex(uuid_x).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(neighbors.len(), 2); // A and B were affected
    }

    // Verify chain is now: A -> B
    let index = harness.index.lock().await;

    let edges_a = index.build_edge_array(uuid_a);
    assert_eq!(edges_a[1], hash_b); // A's east now points to B

    let edges_b = index.build_edge_array(uuid_b);
    assert_eq!(edges_b[0], hash_a); // B's west now points to A
}

/// Test deletion of vertex with multiple neighbors
#[tokio::test]
async fn test_delete_vertex_multiple_neighbors() {
    let harness = TestHarness::new().await;

    // Create a cross pattern:
    //       N
    //       |
    //   W - X - E
    //       |
    //       S

    let (uuid_x, _hash_x) = create_test_vertex(&harness.index, "text/plain", "x.txt", None).await;
    let (uuid_n, hash_n) = create_test_vertex(&harness.index, "text/plain", "n.txt", None).await;
    let (uuid_s, hash_s) = create_test_vertex(&harness.index, "text/plain", "s.txt", None).await;
    let (uuid_e, hash_e) = create_test_vertex(&harness.index, "text/plain", "e.txt", None).await;
    let (uuid_w, hash_w) = create_test_vertex(&harness.index, "text/plain", "w.txt", None).await;

    // Set up edges from X to neighbors
    add_test_edge(&harness.index, uuid_x, uuid_n, Direction::North).await;
    add_test_edge(&harness.index, uuid_x, uuid_s, Direction::South).await;
    add_test_edge(&harness.index, uuid_x, uuid_e, Direction::East).await;
    add_test_edge(&harness.index, uuid_x, uuid_w, Direction::West).await;

    // Delete X
    {
        let mut index = harness.index.lock().await;
        let (files, neighbors) = index.delete_vertex(uuid_x).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(neighbors.len(), 4); // All four neighbors affected
    }

    // Verify reconnections: N-S and W-E should be connected
    let index = harness.index.lock().await;

    // North and South should be connected
    let edges_n = index.build_edge_array(uuid_n);
    assert_eq!(edges_n[3], hash_s); // N's south points to S

    let edges_s = index.build_edge_array(uuid_s);
    assert_eq!(edges_s[2], hash_n); // S's north points to N

    // West and East should be connected
    let edges_w = index.build_edge_array(uuid_w);
    assert_eq!(edges_w[1], hash_e); // W's east points to E

    let edges_e = index.build_edge_array(uuid_e);
    assert_eq!(edges_e[0], hash_w); // E's west points to W
}

/// Test editing vertex content
#[tokio::test]
async fn test_edit_vertex_content() {
    let harness = TestHarness::new().await;

    // Create a vertex
    let (uuid, _hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/test.txt",
        Some("Original".to_string()),
    )
    .await;

    // Edit the transcript
    {
        let mut index = harness.index.lock().await;
        index
            .update_vertex(uuid, Some("Updated content".to_string()))
            .unwrap();
    }

    // Verify update
    let index = harness.index.lock().await;
    let vertex = index.get_vertex(uuid).unwrap();
    assert_eq!(vertex.transcript, Some("Updated content".to_string()));
    assert!(vertex.modified.is_some());
}

/// Test editing vertex layer 1 (transcript layer)
#[tokio::test]
async fn test_edit_vertex_layer1() {
    let harness = TestHarness::new().await;

    // Create a vertex
    let (uuid, _hash) = create_test_vertex(
        &harness.index,
        "audio/ogg",
        ".gradesta-notes/content/audio.ogg",
        None,
    )
    .await;

    // Set layer 1 (transcript)
    {
        let mut index = harness.index.lock().await;
        index
            .set_vertex_layer(
                uuid,
                1,
                "text/plain",
                ".gradesta-notes/content/audio_transcript.txt",
            )
            .unwrap();
    }

    // Verify layer was added
    let index = harness.index.lock().await;
    let (mime, file) = index.get_vertex_layer(uuid, 1).unwrap();
    assert_eq!(mime, "text/plain");
    assert!(file.contains("transcript"));
}

/// Test WebDAV upload is called for create
#[tokio::test]
async fn test_create_vertex_webdav_upload() {
    let harness = TestHarness::new().await;

    let content = b"Test file content";
    let file_path = ".gradesta-notes/content/test.txt";

    // Upload via mock
    harness.webdav.upload(file_path, content).await.unwrap();

    // Verify upload was recorded
    let ops = harness.webdav.get_operations().await;
    assert_eq!(ops.len(), 1);
    assert!(matches!(
        &ops[0],
        super::mock_webdav::WebDavOperation::Upload { path, .. } if path == file_path
    ));

    // Verify file exists in mock
    let stored = harness.webdav.get_file(file_path).await.unwrap();
    assert_eq!(stored, content);
}

#[cfg(test)]
mod hash_tests {
    use super::*;

    /// Test that UUID to hash conversion is consistent
    #[test]
    fn test_uuid_to_hash_consistency() {
        let uuid = Uuid::new_v4();
        let hash1 = uuid_to_hash(uuid);
        let hash2 = uuid_to_hash(uuid);

        assert_eq!(hash1, hash2);
    }

    /// Test that different UUIDs produce different hashes
    #[test]
    fn test_uuid_to_hash_uniqueness() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let hash1 = uuid_to_hash(uuid1);
        let hash2 = uuid_to_hash(uuid2);

        assert_ne!(hash1, hash2);
    }
}
