//! Undo tree management for notes
//!
//! The undo system captures inverse operations for each mutation, storing them
//! in an undo tree structure. The tree can be served as a well-known landmark
//! (`gradesta://undo`) using the existing graph protocol.
//!
//! Tree structure uses existing 6-directional edges:
//! - West: Parent action (previous state)
//! - East: Child action (next state on main branch)
//! - North/South: Sibling branches (alternative timelines)

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::notes::{Edge, NotesIndex, Vertex};

/// Directory for undo data
pub const UNDO_DIR: &str = ".gradesta-notes/undo-snapshots";
/// Undo tree metadata file
pub const UNDO_FILE: &str = ".gradesta-notes/undo.toml";

/// Type of operation that was performed (for undo/redo)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UndoOperationType {
    /// A vertex was created
    CreateVertex {
        /// ID of the created vertex
        vertex_id: Uuid,
        /// Vertex it was connected from (if any)
        from_vertex: Option<Uuid>,
        /// Direction from from_vertex
        direction: Option<String>,
        /// Vertex that was displaced (if inserting into chain)
        displaced_vertex: Option<Uuid>,
        /// Snapshot of the created vertex (for redo after undo)
        #[serde(default)]
        snapshot_path: Option<String>,
    },
    /// A vertex was deleted
    DeleteVertex {
        /// ID of the deleted vertex
        vertex_id: Uuid,
        /// Path to snapshot file containing full vertex data
        snapshot_path: String,
        /// Edges that were connected to this vertex
        connected_edges: Vec<Edge>,
    },
    /// A vertex label was changed
    SetVertexLabel {
        /// ID of the vertex
        vertex_id: Uuid,
        /// Layer that was modified
        layer: u32,
        /// Path to snapshot file containing old content
        old_snapshot_path: String,
        /// Old MIME type
        old_mime: String,
    },
    /// Edges were modified
    SetEdges {
        /// ID of the vertex whose edges changed
        vertex_id: Uuid,
        /// Old edge state [west, east, north, south, up, down]
        old_edges: [Option<Uuid>; 6],
    },
}

/// A single action in the undo tree
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UndoAction {
    /// Unique identifier for this action
    pub id: Uuid,
    /// When the action was performed
    pub timestamp: DateTime<Utc>,
    /// Human-readable description (e.g., "Created note", "Edited: Meeting notes")
    pub description: String,
    /// The operation data needed to undo
    pub operation: UndoOperationType,
    /// Parent action ID (None for root)
    pub parent_id: Option<Uuid>,
    /// Child action IDs (branches)
    pub children: Vec<Uuid>,
    /// Identity of who performed the action (if known)
    pub identity: Option<String>,
    /// Extensible metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl UndoAction {
    /// Create a new undo action
    pub fn new(
        description: String,
        operation: UndoOperationType,
        parent_id: Option<Uuid>,
        identity: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            description,
            operation,
            parent_id,
            children: Vec::new(),
            identity,
            metadata: HashMap::new(),
        }
    }

    /// Get a short label for display (truncated description)
    pub fn short_label(&self) -> String {
        if self.description.len() > 50 {
            format!("{}...", &self.description[..47])
        } else {
            self.description.clone()
        }
    }
}

/// The undo tree structure
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UndoTree {
    /// All actions indexed by ID
    pub actions: HashMap<Uuid, UndoAction>,
    /// Root action ID (the initial state, or first action)
    pub root_id: Option<Uuid>,
    /// Current position in the tree (the "present")
    pub current_id: Option<Uuid>,
    /// Version for future compatibility
    pub version: u32,
}

impl UndoTree {
    /// Create a new empty undo tree
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
            root_id: None,
            current_id: None,
            version: 1,
        }
    }

    /// Load undo tree from storage
    pub async fn load(nc: &crate::nextcloud::NextcloudClient) -> Result<Self> {
        eprintln!("DEBUG: UndoTree::load checking for {} (nc.url={})", UNDO_FILE, nc.url);
        let exists = nc.exists(UNDO_FILE).await;
        eprintln!("DEBUG: UndoTree::load - exists check returned: {}", exists);
        if !exists {
            eprintln!("DEBUG: UndoTree::load - file does not exist, returning empty tree");
            return Ok(Self::new());
        }

        eprintln!("DEBUG: UndoTree::load - downloading {}", UNDO_FILE);
        let data = nc.download(UNDO_FILE).await?;
        let content = String::from_utf8(data).context("Invalid UTF-8 in undo file")?;
        eprintln!("DEBUG: UndoTree::load - parsing {} bytes", content.len());
        let tree: Self = toml::from_str(&content).context("Failed to parse undo TOML")?;
        eprintln!("DEBUG: UndoTree::load - loaded {} actions, current={:?}", tree.actions.len(), tree.current_id);
        Ok(tree)
    }

    /// Save undo tree to storage
    pub async fn save(&self, nc: &crate::nextcloud::NextcloudClient) -> Result<()> {
        eprintln!("DEBUG: UndoTree::save - saving {} actions to {}", self.actions.len(), UNDO_FILE);
        // Ensure directory exists
        nc.mkdir(UNDO_DIR).await?;

        let content = toml::to_string_pretty(self).context("Failed to serialize undo tree")?;
        eprintln!("DEBUG: UndoTree::save - uploading {} bytes", content.len());
        nc.upload(UNDO_FILE, content.as_bytes()).await?;
        eprintln!("DEBUG: UndoTree::save - done");
        Ok(())
    }

    /// Add an action to the tree
    pub fn add_action(&mut self, mut action: UndoAction) {
        // Set parent to current position
        action.parent_id = self.current_id;

        // Update parent's children list
        if let Some(parent_id) = action.parent_id {
            if let Some(parent) = self.actions.get_mut(&parent_id) {
                parent.children.push(action.id);
            }
        }

        let action_id = action.id;

        // Set root if this is the first action
        if self.root_id.is_none() {
            self.root_id = Some(action_id);
        }

        // Store the action
        self.actions.insert(action_id, action);

        // Move current position to the new action
        self.current_id = Some(action_id);
    }

    /// Get an action by ID
    pub fn get_action(&self, id: Uuid) -> Option<&UndoAction> {
        self.actions.get(&id)
    }

    /// Get the current action
    pub fn current_action(&self) -> Option<&UndoAction> {
        self.current_id.and_then(|id| self.actions.get(&id))
    }

    /// Get the path from root to current position
    pub fn path_to_current(&self) -> Vec<Uuid> {
        let mut path = Vec::new();
        let mut current = self.current_id;

        while let Some(id) = current {
            path.push(id);
            current = self.actions.get(&id).and_then(|a| a.parent_id);
        }

        path.reverse();
        path
    }

    /// Navigate to a specific action (returns the path of actions to apply)
    /// Returns (actions_to_undo, actions_to_redo)
    pub fn navigate_to(&self, target_id: Uuid) -> Option<(Vec<Uuid>, Vec<Uuid>)> {
        // Find common ancestor between current and target
        let current_path = self.path_to_current();
        let target_path = self.path_to(target_id)?;

        // Find divergence point
        let mut common_prefix_len = 0;
        for (i, (a, b)) in current_path.iter().zip(target_path.iter()).enumerate() {
            if a == b {
                common_prefix_len = i + 1;
            } else {
                break;
            }
        }

        // Actions to undo: from current back to common ancestor (exclusive of ancestor)
        let undo_actions: Vec<Uuid> = current_path[common_prefix_len..].iter().rev().copied().collect();

        // Actions to redo: from common ancestor to target (exclusive of ancestor)
        let redo_actions: Vec<Uuid> = target_path[common_prefix_len..].to_vec();

        Some((undo_actions, redo_actions))
    }

    /// Get path from root to a specific action
    fn path_to(&self, target_id: Uuid) -> Option<Vec<Uuid>> {
        let mut path = Vec::new();
        let mut current = Some(target_id);

        while let Some(id) = current {
            path.push(id);
            current = self.actions.get(&id).and_then(|a| a.parent_id);
        }

        path.reverse();
        Some(path)
    }

    /// Set current position (after navigation)
    pub fn set_current(&mut self, id: Uuid) {
        if self.actions.contains_key(&id) {
            self.current_id = Some(id);
        }
    }

    /// Get sibling actions (same parent, different branches)
    pub fn get_siblings(&self, action_id: Uuid) -> Vec<Uuid> {
        let action = match self.actions.get(&action_id) {
            Some(a) => a,
            None => return Vec::new(),
        };

        let parent_id = match action.parent_id {
            Some(p) => p,
            None => return Vec::new(), // Root has no siblings
        };

        let parent = match self.actions.get(&parent_id) {
            Some(p) => p,
            None => return Vec::new(),
        };

        parent
            .children
            .iter()
            .filter(|&&id| id != action_id)
            .copied()
            .collect()
    }

    /// Get the first child (main branch continuation)
    pub fn get_main_child(&self, action_id: Uuid) -> Option<Uuid> {
        self.actions
            .get(&action_id)
            .and_then(|a| a.children.first().copied())
    }
}

/// Convert undo action ID to vertex hash (for protocol)
pub fn action_to_hash(id: Uuid) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    // Add prefix to avoid collision with note vertex hashes
    "undo:".hash(&mut hasher);
    id.hash(&mut hasher);
    hasher.finish()
}

/// Check if a vertex hash is an undo action (vs a regular vertex)
/// We use a simple heuristic: undo vertices have hashes generated with "undo:" prefix
pub fn is_undo_vertex(hash: u64, tree: &UndoTree) -> bool {
    tree.actions.keys().any(|id| action_to_hash(*id) == hash)
}

/// Find action by vertex hash
pub fn hash_to_action(tree: &UndoTree, hash: u64) -> Option<Uuid> {
    tree.actions
        .keys()
        .find(|id| action_to_hash(**id) == hash)
        .copied()
}

/// Snapshot content for restoration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentSnapshot {
    /// The vertex this content belonged to
    pub vertex_id: Uuid,
    /// Layer number
    pub layer: u32,
    /// MIME type
    pub mime: String,
    /// File path where content is stored
    pub file_path: String,
}

/// Vertex snapshot for deleted vertex restoration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VertexSnapshot {
    /// The original vertex data
    pub vertex: Vertex,
    /// Edges connected to this vertex
    pub edges: Vec<Edge>,
}

impl VertexSnapshot {
    /// Save snapshot to storage
    pub async fn save(&self, nc: &crate::nextcloud::NextcloudClient, action_id: Uuid) -> Result<String> {
        let snapshot_path = format!("{}/{}_vertex.toml", UNDO_DIR, action_id);
        let content = toml::to_string_pretty(self).context("Failed to serialize vertex snapshot")?;
        nc.upload(&snapshot_path, content.as_bytes()).await?;
        Ok(snapshot_path)
    }

    /// Load snapshot from storage
    pub async fn load(nc: &crate::nextcloud::NextcloudClient, snapshot_path: &str) -> Result<Self> {
        let data = nc.download(snapshot_path).await?;
        let content = String::from_utf8(data).context("Invalid UTF-8 in snapshot")?;
        toml::from_str(&content).context("Failed to parse vertex snapshot")
    }
}

/// Save content to a snapshot file
pub async fn save_content_snapshot(
    nc: &crate::nextcloud::NextcloudClient,
    action_id: Uuid,
    layer: u32,
    content: &[u8],
) -> Result<String> {
    let snapshot_path = format!("{}/{}_layer{}.bin", UNDO_DIR, action_id, layer);
    nc.upload(&snapshot_path, content).await?;
    Ok(snapshot_path)
}

/// Load content from a snapshot file
pub async fn load_content_snapshot(
    nc: &crate::nextcloud::NextcloudClient,
    snapshot_path: &str,
) -> Result<Vec<u8>> {
    nc.download(snapshot_path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_tree_navigation() {
        let mut tree = UndoTree::new();

        // Create a linear sequence: A -> B -> C
        let action_a = UndoAction::new(
            "Action A".to_string(),
            UndoOperationType::CreateVertex {
                vertex_id: Uuid::new_v4(),
                from_vertex: None,
                direction: None,
                displaced_vertex: None,
            },
            None,
            None,
        );
        let id_a = action_a.id;
        tree.add_action(action_a);

        let action_b = UndoAction::new(
            "Action B".to_string(),
            UndoOperationType::CreateVertex {
                vertex_id: Uuid::new_v4(),
                from_vertex: None,
                direction: None,
                displaced_vertex: None,
            },
            None,
            None,
        );
        let id_b = action_b.id;
        tree.add_action(action_b);

        let action_c = UndoAction::new(
            "Action C".to_string(),
            UndoOperationType::CreateVertex {
                vertex_id: Uuid::new_v4(),
                from_vertex: None,
                direction: None,
                displaced_vertex: None,
            },
            None,
            None,
        );
        let id_c = action_c.id;
        tree.add_action(action_c);

        // Current should be C
        assert_eq!(tree.current_id, Some(id_c));

        // Navigate from C to A
        let (undo, redo) = tree.navigate_to(id_a).unwrap();
        assert_eq!(undo, vec![id_c, id_b]); // Undo C and B
        assert!(redo.is_empty()); // No redo needed

        // Navigate from C to B
        let (undo, redo) = tree.navigate_to(id_b).unwrap();
        assert_eq!(undo, vec![id_c]); // Only undo C
        assert!(redo.is_empty());
    }

    #[test]
    fn test_action_to_hash() {
        let id = Uuid::new_v4();
        let hash1 = action_to_hash(id);
        let hash2 = action_to_hash(id);
        assert_eq!(hash1, hash2);

        let other_id = Uuid::new_v4();
        let other_hash = action_to_hash(other_id);
        assert_ne!(hash1, other_hash);
    }
}
