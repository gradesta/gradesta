//! Centralized landmark watching state management.
//!
//! This module consolidates all landmark-related state and provides methods
//! to prevent re-watching already watched landmarks.

use std::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use crate::network::WsCommand;

/// State of a landmark watch request
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LandmarkWatchState {
    /// Watch request sent, awaiting SetContext response
    Requested { action_id: u64 },
    /// Currently receiving vertices for this landmark
    Receiving,
    /// Landmark fully loaded
    Complete,
    /// Watch request failed
    Failed { reason: String },
}

/// Centralized manager for landmark watching state.
///
/// Consolidates:
/// - `AppState.requested_landmarks` -> `watches`
/// - `GraphState.landmark_vertices` -> `landmark_vertices`
/// - `GraphState.current_receiving_landmark` -> `current_receiving_landmark`
/// - `AppState.following_portal` -> `pending_navigation`
#[derive(Default)]
pub struct LandmarkWatchManager {
    /// Maps landmark URL to its watch state
    watches: HashMap<String, LandmarkWatchState>,
    /// Maps landmark URL to the vertex IDs that belong to it
    pub landmark_vertices: HashMap<String, Vec<u64>>,
    /// Which landmark we're currently receiving data for
    pub current_receiving_landmark: Option<String>,
    /// Landmarks that should trigger auto-navigation when their first content vertex arrives
    pending_navigation: HashSet<String>,
}

impl LandmarkWatchManager {
    /// Watch a landmark if it hasn't been watched yet.
    ///
    /// Returns `true` if a new watch was initiated, `false` if already watched.
    pub fn watch_if_needed(
        &mut self,
        landmark: &str,
        action_id: u64,
        cmd_tx: &Sender<WsCommand>,
    ) -> bool {
        if self.is_watched(landmark) {
            return false;
        }

        eprintln!("Requesting landmark: {}", landmark);
        self.watches.insert(
            landmark.to_string(),
            LandmarkWatchState::Requested { action_id },
        );

        let _ = cmd_tx.send(WsCommand::WatchLandmark {
            action_id,
            landmark: landmark.to_string(),
        });

        true
    }

    /// Watch a landmark and set up auto-navigation to it.
    ///
    /// Returns `true` if a new watch was initiated, `false` if already watched.
    /// If the landmark is already loaded, navigation should be handled by the caller.
    pub fn watch_and_follow(
        &mut self,
        landmark: &str,
        action_id: u64,
        cmd_tx: &Sender<WsCommand>,
    ) -> bool {
        let is_new = self.watch_if_needed(landmark, action_id, cmd_tx);

        // Always add to pending navigation if not already loaded
        if !self.is_loaded(landmark) {
            self.pending_navigation.insert(landmark.to_string());
        }

        is_new
    }

    /// Check if a landmark has been watched (any state except Failed).
    pub fn is_watched(&self, landmark: &str) -> bool {
        matches!(
            self.watches.get(landmark),
            Some(LandmarkWatchState::Requested { .. })
                | Some(LandmarkWatchState::Receiving)
                | Some(LandmarkWatchState::Complete)
        )
    }

    /// Check if a landmark is fully loaded (has received vertices).
    pub fn is_loaded(&self, landmark: &str) -> bool {
        self.landmark_vertices
            .get(landmark)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Handle SetContext response from server.
    ///
    /// Updates the watch state and prepares to receive vertices.
    pub fn on_set_context(&mut self, landmark: &str) {
        // Update state to Receiving
        self.watches.insert(
            landmark.to_string(),
            LandmarkWatchState::Receiving,
        );

        // Set current receiving landmark
        self.current_receiving_landmark = Some(landmark.to_string());

        // Initialize vertex list if not exists
        self.landmark_vertices
            .entry(landmark.to_string())
            .or_insert_with(Vec::new);
    }

    /// Track a vertex as belonging to the current receiving landmark.
    ///
    /// Call this when a SetVertexLabel/SetVertexPreview is received.
    pub fn track_vertex(&mut self, vertex_id: u64) {
        if let Some(landmark) = self.current_receiving_landmark.clone() {
            if let Some(vertices) = self.landmark_vertices.get_mut(&landmark) {
                if !vertices.contains(&vertex_id) {
                    vertices.push(vertex_id);
                }
            }
        }
    }

    /// Get vertices belonging to a landmark.
    pub fn get_landmark_vertices(&self, landmark: &str) -> Option<&Vec<u64>> {
        self.landmark_vertices.get(landmark)
    }

    /// Check if a landmark has any non-portal content loaded.
    ///
    /// Used to determine if a portal's destination has content available.
    pub fn has_content<F>(&self, landmark: &str, is_portal: F) -> bool
    where
        F: Fn(u64) -> bool,
    {
        self.landmark_vertices
            .get(landmark)
            .map(|vertices| vertices.iter().any(|&vid| !is_portal(vid)))
            .unwrap_or(false)
    }

    /// Check if a landmark is pending navigation.
    ///
    /// When `true`, the browser should auto-navigate to the first content vertex
    /// when it arrives.
    pub fn should_follow(&self, landmark: &str) -> bool {
        self.pending_navigation.contains(landmark)
    }

    /// Clear pending navigation for a landmark.
    ///
    /// Call this after successfully navigating to the landmark's content.
    pub fn clear_follow(&mut self, landmark: &str) {
        self.pending_navigation.remove(landmark);
    }

    /// Get the landmark we should follow (if any).
    ///
    /// Returns the first pending navigation landmark.
    pub fn get_following(&self) -> Option<&String> {
        self.pending_navigation.iter().next()
    }

    /// Clear all state (on disconnect).
    pub fn clear(&mut self) {
        self.watches.clear();
        self.landmark_vertices.clear();
        self.current_receiving_landmark = None;
        self.pending_navigation.clear();
    }

    /// Remove a vertex from all landmark vertex lists.
    ///
    /// Call this when a vertex is deleted.
    pub fn remove_vertex(&mut self, vertex_id: u64) {
        for vertices in self.landmark_vertices.values_mut() {
            vertices.retain(|&id| id != vertex_id);
        }
    }
}

/// Build a landmark URL for a vertex ID based on the connection's base URL.
///
/// The base_ws_url looks like "ws://localhost:8083/ws?landmark=notes://identity/"
/// This extracts the landmark base and appends the vertex ID.
pub fn build_landmark_url(vertex_id: u64, base_url: &Option<String>) -> String {
    if let Some(ref base_url) = base_url {
        if let Some(landmark_start) = base_url.find("landmark=") {
            let landmark_base = &base_url[landmark_start + 9..];
            let landmark_base = landmark_base.split('&').next().unwrap_or(landmark_base);
            return format!("{}{}", landmark_base.trim_end_matches('/'), vertex_id);
        }
    }
    format!("vertex/{}", vertex_id)
}
