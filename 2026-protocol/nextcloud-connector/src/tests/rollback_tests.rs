//! Rollback tests for failure scenarios
//!
//! Tests that verify operations are properly rolled back when failures occur,
//! including WebDAV upload failures and index save failures.

use tokio::time::{sleep, Duration};

use crate::protocol::{Direction, STATUS_CONFLICT};
use crate::sync_worker::{SyncOperation, SyncWorkItem};

use super::harness::{add_test_edge, create_test_vertex, TestHarness};

/// Test that create operation sends 409 on upload failure
#[tokio::test]
async fn test_create_rollback_on_upload_fail() {
    let mut harness = TestHarness::new().await;

    // Configure WebDAV to fail uploads
    harness
        .webdav
        .fail_upload(".gradesta-notes/content/fail.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    // Create vertex in index
    let (uuid, hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/fail.txt",
        Some("Will fail".to_string()),
    )
    .await;

    // Queue work item for a file that will fail to upload
    let work_item = SyncWorkItem {
        action_id: 100,
        vertex_id: hash,
        operation: SyncOperation::CreateVertex {
            uuid,
            content: b"Will fail".to_vec(),
            content_hash: "abc123".to_string(),
            ext: "txt".to_string(),
            mime: "text/plain".to_string(),
            from_vertex: 0,
            direction: Direction::East,
            original_source_edges: None,
            displaced_vertex: None,
        },
    };

    sync_tx.send(work_item).await.unwrap();

    // Wait for processing
    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    // Should have received 409 response
    let messages = harness.drain_messages();

    // Find the log message
    let log_msg = messages.iter().find_map(|m| TestHarness::parse_log_message(m));
    assert!(log_msg.is_some(), "Expected log message");

    let (action_id, status, vertex_id, _message) = log_msg.unwrap();
    assert_eq!(action_id, 100);
    assert_eq!(status, STATUS_CONFLICT);
    assert_eq!(vertex_id, hash);
}

/// Test that create rollback restores source vertex edges
#[tokio::test]
async fn test_create_rollback_restores_source_edges() {
    let mut harness = TestHarness::new().await;

    // Create source vertex with existing edges
    let (uuid_source, hash_source) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/source.txt",
        None,
    )
    .await;

    let (uuid_existing, hash_existing) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/existing.txt",
        None,
    )
    .await;

    // Source already has east edge
    add_test_edge(&harness.index, uuid_source, uuid_existing, Direction::East).await;

    // Get original edges
    let original_source_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_source)
    };

    // Configure failure
    harness
        .webdav
        .fail_upload(".gradesta-notes/content/new.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    // Create new vertex (which will fail)
    let (uuid_new, hash_new) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/new.txt",
        None,
    )
    .await;

    let work_item = SyncWorkItem {
        action_id: 101,
        vertex_id: hash_new,
        operation: SyncOperation::CreateVertex {
            uuid: uuid_new,
            content: b"New content".to_vec(),
            content_hash: "def456".to_string(),
            ext: "txt".to_string(),
            mime: "text/plain".to_string(),
            from_vertex: hash_source,
            direction: Direction::East,
            original_source_edges: Some((hash_source, original_source_edges)),
            displaced_vertex: Some((hash_existing, original_source_edges)),
        },
    };

    sync_tx.send(work_item).await.unwrap();

    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    let messages = harness.drain_messages();

    // Should have SetEdges to restore source edges
    let edges_msgs: Vec<_> = messages
        .iter()
        .filter_map(|m| TestHarness::parse_set_edges(m))
        .collect();

    // Should have at least the source vertex restore
    let source_restore = edges_msgs.iter().find(|(_, vid, _, _)| *vid == hash_source);
    assert!(
        source_restore.is_some(),
        "Expected SetEdges for source vertex"
    );
}

/// Test that create rollback restores displaced vertex edges
#[tokio::test]
async fn test_create_rollback_restores_displaced_edges() {
    let mut harness = TestHarness::new().await;

    // Create chain: A -> B (B is displaced)
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

    let original_a_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_a)
    };
    let original_b_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_b)
    };

    // Configure failure for new vertex
    harness
        .webdav
        .fail_upload(".gradesta-notes/content/x.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    // Try to insert X between A and B (will fail)
    let (uuid_x, hash_x) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/x.txt",
        None,
    )
    .await;

    let work_item = SyncWorkItem {
        action_id: 102,
        vertex_id: hash_x,
        operation: SyncOperation::CreateVertex {
            uuid: uuid_x,
            content: b"X content".to_vec(),
            content_hash: "ghi789".to_string(),
            ext: "txt".to_string(),
            mime: "text/plain".to_string(),
            from_vertex: hash_a,
            direction: Direction::East,
            original_source_edges: Some((hash_a, original_a_edges)),
            displaced_vertex: Some((hash_b, original_b_edges)),
        },
    };

    sync_tx.send(work_item).await.unwrap();

    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    let messages = harness.drain_messages();

    // Should have SetEdges for displaced vertex B
    let edges_msgs: Vec<_> = messages
        .iter()
        .filter_map(|m| TestHarness::parse_set_edges(m))
        .collect();

    let b_restore = edges_msgs.iter().find(|(_, vid, _, _)| *vid == hash_b);
    assert!(
        b_restore.is_some(),
        "Expected SetEdges for displaced vertex B"
    );
}

/// Test that delete rollback sends SetVertexLabel with original content
#[tokio::test]
async fn test_delete_rollback_restores_content() {
    let mut harness = TestHarness::new().await;

    // Create a vertex to delete
    let (uuid, hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/restore.txt",
        Some("Restore me".to_string()),
    )
    .await;

    let original_content = b"Restore me content";
    let original_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid)
    };

    // Delete from index
    {
        let mut index = harness.index.lock().await;
        index.delete_vertex(uuid).unwrap();
    }

    // Configure delete to fail
    harness
        .webdav
        .fail_delete(".gradesta-notes/content/restore.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    let work_item = SyncWorkItem {
        action_id: 103,
        vertex_id: hash,
        operation: SyncOperation::DeleteVertex {
            uuid,
            content_hashes_to_gc: "jkl012".to_string(),
            original_mime: "text/plain".to_string(),
            original_hash: "jkl012".to_string(),
            original_edges,
            affected_neighbors: vec![],
        },
    };

    sync_tx.send(work_item).await.unwrap();

    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    let messages = harness.drain_messages();

    // Should have SetVertexLabel to restore content
    let label_msg = messages
        .iter()
        .find_map(|m| TestHarness::parse_set_vertex_label(m));

    assert!(label_msg.is_some(), "Expected SetVertexLabel message");
    let (action_id, vertex_id, layer, mime, content) = label_msg.unwrap();
    assert_eq!(action_id, 103);
    assert_eq!(vertex_id, hash);
    assert_eq!(layer, 0);
    assert_eq!(mime, "text/plain");
    assert_eq!(content, original_content.to_vec());
}

/// Test that delete rollback restores all neighbor edges
#[tokio::test]
async fn test_delete_rollback_restores_neighbor_edges() {
    let mut harness = TestHarness::new().await;

    // Create chain: A -> X -> B
    let (uuid_a, hash_a) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/a.txt",
        None,
    )
    .await;

    let (uuid_x, hash_x) = create_test_vertex(
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

    add_test_edge(&harness.index, uuid_a, uuid_x, Direction::East).await;
    add_test_edge(&harness.index, uuid_x, uuid_b, Direction::East).await;

    // Get original edges before deletion
    let original_x_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_x)
    };
    let original_a_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_a)
    };
    let original_b_edges = {
        let index = harness.index.lock().await;
        index.build_edge_array(uuid_b)
    };

    // Delete X from index
    {
        let mut index = harness.index.lock().await;
        index.delete_vertex(uuid_x).unwrap();
    }

    // Configure failure
    harness
        .webdav
        .fail_delete(".gradesta-notes/content/x.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    let work_item = SyncWorkItem {
        action_id: 104,
        vertex_id: hash_x,
        operation: SyncOperation::DeleteVertex {
            uuid: uuid_x,
            content_hashes_to_gc: "mno345".to_string(),
            original_mime: "text/plain".to_string(),
            original_hash: "mno345".to_string(),
            original_edges: original_x_edges,
            affected_neighbors: vec![(uuid_a, original_a_edges), (uuid_b, original_b_edges)],
        },
    };

    sync_tx.send(work_item).await.unwrap();

    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    let messages = harness.drain_messages();

    // Should have SetEdges for both neighbors
    let edges_msgs: Vec<_> = messages
        .iter()
        .filter_map(|m| TestHarness::parse_set_edges(m))
        .collect();

    // Look for A and B restores
    let a_restore = edges_msgs.iter().find(|(_, vid, _, _)| *vid == hash_a);
    let b_restore = edges_msgs.iter().find(|(_, vid, _, _)| *vid == hash_b);

    assert!(a_restore.is_some(), "Expected SetEdges for neighbor A");
    assert!(b_restore.is_some(), "Expected SetEdges for neighbor B");
}

/// Test that edit rollback sends original content back
#[tokio::test]
async fn test_edit_rollback_restores_content() {
    let mut harness = TestHarness::new().await;

    // Create vertex
    let (uuid, hash) = create_test_vertex(
        &harness.index,
        "text/plain",
        ".gradesta-notes/content/edit.txt",
        Some("Original text".to_string()),
    )
    .await;

    let original_content = b"Original text";

    // Configure upload to fail
    harness
        .webdav
        .fail_upload(".gradesta-notes/content/edit.txt")
        .await;

    harness.spawn_sync_worker("test-user");

    let sync_tx = harness.sync_tx.as_ref().unwrap().clone();

    let work_item = SyncWorkItem {
        action_id: 105,
        vertex_id: hash,
        operation: SyncOperation::EditVertex {
            uuid,
            content: b"New text".to_vec(),
            new_hash: "pqr678".to_string(),
            ext: "txt".to_string(),
            mime: "text/plain".to_string(),
            layer: 0,
            original_hash: "stu901".to_string(),
            original_mime: "text/plain".to_string(),
        },
    };

    sync_tx.send(work_item).await.unwrap();

    drop(sync_tx);
    harness.sync_tx = None;
    sleep(Duration::from_millis(200)).await;

    let messages = harness.drain_messages();

    // Should have SetVertexLabel with original content
    let label_msg = messages
        .iter()
        .find_map(|m| TestHarness::parse_set_vertex_label(m));

    assert!(label_msg.is_some(), "Expected SetVertexLabel message");
    let (action_id, vertex_id, layer, _mime, content) = label_msg.unwrap();
    assert_eq!(action_id, 105);
    assert_eq!(vertex_id, hash);
    assert_eq!(layer, 0);
    assert_eq!(content, original_content.to_vec());
}

/// Test delete returns 404 for non-existent vertex
#[tokio::test]
async fn test_delete_vertex_not_found() {
    let harness = TestHarness::new().await;

    let fake_uuid = uuid::Uuid::new_v4();

    // Try to delete non-existent vertex
    let result = {
        let mut index = harness.index.lock().await;
        index.delete_vertex(fake_uuid)
    };

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found"));
}
