//! Connection manager for forwarding messages between browser and elf connections
//!
//! This module provides a registry of browser connection sender channels,
//! allowing elf handlers to forward vertex updates back to the originating browser.
//! It also tracks which connections are watching which vertices, enabling broadcast
//! of updates to all interested parties.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// Sender channel type for forwarding messages to browser connections
pub type BrowserSender = mpsc::UnboundedSender<Vec<u8>>;

/// Manages sender channels for active browser connections
#[derive(Default)]
pub struct ConnectionManager {
    /// Map from connection ID to sender channel
    senders: HashMap<u64, BrowserSender>,
    /// Map from vertex ID to set of connection IDs watching that vertex
    vertex_watchers: HashMap<u64, HashSet<u64>>,
    /// Map from connection ID to set of vertices it's watching (for cleanup)
    connection_watches: HashMap<u64, HashSet<u64>>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a browser connection's sender channel
    pub fn register(&mut self, conn_id: u64, sender: BrowserSender) {
        log::debug!("ConnectionManager: registered browser conn_id={}", conn_id);
        self.senders.insert(conn_id, sender);
    }

    /// Unregister a browser connection (on disconnect)
    pub fn unregister(&mut self, conn_id: u64) {
        log::debug!("ConnectionManager: unregistered browser conn_id={}", conn_id);
        self.senders.remove(&conn_id);

        // Clean up vertex watches for this connection
        if let Some(vertices) = self.connection_watches.remove(&conn_id) {
            for vertex_id in vertices {
                if let Some(watchers) = self.vertex_watchers.get_mut(&vertex_id) {
                    watchers.remove(&conn_id);
                    if watchers.is_empty() {
                        self.vertex_watchers.remove(&vertex_id);
                    }
                }
            }
        }
    }

    /// Register that a connection is watching a vertex
    pub fn add_vertex_watcher(&mut self, conn_id: u64, vertex_id: u64) {
        self.vertex_watchers
            .entry(vertex_id)
            .or_default()
            .insert(conn_id);
        self.connection_watches
            .entry(conn_id)
            .or_default()
            .insert(vertex_id);
    }

    /// Register that a connection is watching multiple vertices
    pub fn add_vertex_watchers(&mut self, conn_id: u64, vertex_ids: &[u64]) {
        for &vertex_id in vertex_ids {
            self.add_vertex_watcher(conn_id, vertex_id);
        }
    }

    /// Send a message to a specific browser connection
    /// Returns Ok(()) if sent successfully, Err(()) if connection not found or send failed
    pub fn send_to(&self, conn_id: u64, msg: Vec<u8>) -> Result<(), ()> {
        self.senders
            .get(&conn_id)
            .ok_or(())?
            .send(msg)
            .map_err(|_| ())
    }

    /// Broadcast a message to all connections watching a specific vertex
    /// Returns the number of connections that received the message
    pub fn broadcast_to_vertex_watchers(&self, vertex_id: u64, msg: &[u8]) -> usize {
        let mut sent_count = 0;
        if let Some(watchers) = self.vertex_watchers.get(&vertex_id) {
            for &conn_id in watchers {
                if let Some(sender) = self.senders.get(&conn_id) {
                    if sender.send(msg.to_vec()).is_ok() {
                        sent_count += 1;
                    }
                }
            }
        }
        sent_count
    }

    /// Get all connection IDs watching a specific vertex
    pub fn get_vertex_watchers(&self, vertex_id: u64) -> Vec<u64> {
        self.vertex_watchers
            .get(&vertex_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// Thread-safe shared connection manager
pub type SharedConnectionManager = Arc<Mutex<ConnectionManager>>;

/// Create a new shared connection manager
pub fn new_shared_connection_manager() -> SharedConnectionManager {
    Arc::new(Mutex::new(ConnectionManager::new()))
}
