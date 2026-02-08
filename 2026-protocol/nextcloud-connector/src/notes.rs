//! Note storage and graph management

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::nextcloud::NextcloudClient;
use crate::protocol::Direction;

const NOTES_DIR: &str = ".gradesta-notes";
const INDEX_FILE: &str = ".gradesta-notes/index.toml";
pub const CONTENT_DIR: &str = ".gradesta-notes/content";

/// Layer content - each layer has its own MIME type and file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerContent {
    pub mime: String,
    pub file: String,
}

/// Vertex in the note graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub id: Uuid,
    /// Primary layer mime type (for backwards compatibility)
    pub mime: String,
    /// Primary layer file (for backwards compatibility)
    pub file: String,
    /// Legacy transcript field (migrated to layers)
    #[serde(default)]
    pub transcript: Option<String>,
    /// Additional layers (layer_id -> content)
    /// Layer 0 is implicit (mime, file fields)
    /// Layer 1+ stored here
    #[serde(default)]
    pub layers: HashMap<u32, LayerContent>,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub modified: Option<DateTime<Utc>>,
}

/// Edge connecting two vertices
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: Uuid,
    pub to: Uuid,
    pub direction: String, // "north", "south", "east", "west", "up", "down"
}

/// Notes index metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexMeta {
    pub version: u32,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

/// Notes index structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotesIndex {
    pub meta: IndexMeta,
    #[serde(default)]
    pub vertices: Vec<Vertex>,
    #[serde(default)]
    pub edges: Vec<Edge>,
}

impl Default for NotesIndex {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            meta: IndexMeta {
                version: 1,
                created: now,
                modified: now,
            },
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl NotesIndex {
    /// Load index from Nextcloud
    pub async fn load(nc: &NextcloudClient) -> Result<Self> {
        if !nc.exists(INDEX_FILE).await {
            // Create empty index
            let index = Self::default();
            index.save(nc).await?;
            return Ok(index);
        }

        let data = nc.download(INDEX_FILE).await?;
        let content = String::from_utf8(data).context("Invalid UTF-8 in index")?;
        toml::from_str(&content).context("Failed to parse index TOML")
    }

    /// Save index to Nextcloud
    pub async fn save(&self, nc: &NextcloudClient) -> Result<()> {
        // Ensure directories exist
        nc.mkdir(NOTES_DIR).await?;
        nc.mkdir(CONTENT_DIR).await?;

        let content = toml::to_string_pretty(self).context("Failed to serialize index")?;
        nc.upload(INDEX_FILE, content.as_bytes()).await
    }

    /// Get vertex by ID
    pub fn get_vertex(&self, id: Uuid) -> Option<&Vertex> {
        self.vertices.iter().find(|v| v.id == id)
    }

    /// Get vertex by numeric ID (hash)
    pub fn get_vertex_by_hash(&self, hash: u64) -> Option<&Vertex> {
        self.vertices.iter().find(|v| uuid_to_hash(v.id) == hash)
    }

    /// Get edges from a vertex
    pub fn get_edges_from(&self, vertex_id: Uuid) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from == vertex_id).collect()
    }

    /// Get edges to a vertex
    pub fn get_edges_to(&self, vertex_id: Uuid) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to == vertex_id).collect()
    }

    /// Get neighbor in a direction
    pub fn get_neighbor(&self, vertex_id: Uuid, direction: &str) -> Option<Uuid> {
        // Check outgoing edges
        for edge in &self.edges {
            if edge.from == vertex_id && edge.direction == direction {
                return Some(edge.to);
            }
        }
        // Check incoming edges (reverse direction)
        let reverse = match direction {
            "north" => "south",
            "south" => "north",
            "east" => "west",
            "west" => "east",
            "up" => "down",
            "down" => "up",
            _ => return None,
        };
        for edge in &self.edges {
            if edge.to == vertex_id && edge.direction == reverse {
                return Some(edge.from);
            }
        }
        None
    }

    /// Create a new vertex
    pub fn create_vertex(&mut self, mime: &str, file: &str, transcript: Option<String>) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        self.vertices.push(Vertex {
            id,
            mime: mime.to_string(),
            file: file.to_string(),
            transcript,
            layers: HashMap::new(),
            created: now,
            modified: None,
        });
        self.meta.modified = now;
        id
    }

    /// Set a layer on a vertex
    /// Layer 0 updates mime/file, Layer 1+ goes into layers map
    pub fn set_vertex_layer(&mut self, id: Uuid, layer: u32, mime: &str, file: &str) -> Result<()> {
        let vertex = self
            .vertices
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| anyhow!("Vertex not found"))?;

        if layer == 0 {
            vertex.mime = mime.to_string();
            vertex.file = file.to_string();
        } else {
            vertex.layers.insert(layer, LayerContent {
                mime: mime.to_string(),
                file: file.to_string(),
            });
        }
        vertex.modified = Some(Utc::now());
        self.meta.modified = Utc::now();
        Ok(())
    }

    /// Get a layer from a vertex
    pub fn get_vertex_layer(&self, id: Uuid, layer: u32) -> Option<(&str, &str)> {
        let vertex = self.vertices.iter().find(|v| v.id == id)?;
        if layer == 0 {
            Some((&vertex.mime, &vertex.file))
        } else {
            vertex.layers.get(&layer).map(|l| (l.mime.as_str(), l.file.as_str()))
        }
    }

    /// Add an edge (simple - replaces any existing edge in that direction)
    pub fn add_edge(&mut self, from: Uuid, to: Uuid, direction: Direction) {
        let dir_str = match direction {
            Direction::West => "west",
            Direction::East => "east",
            Direction::North => "north",
            Direction::South => "south",
            Direction::Up => "up",
            Direction::Down => "down",
        };

        // Remove any existing edge in this direction from this vertex
        self.edges.retain(|e| !(e.from == from && e.direction == dir_str));

        self.edges.push(Edge {
            from,
            to,
            direction: dir_str.to_string(),
        });
        self.meta.modified = Utc::now();
    }

    /// Insert a vertex into a chain, maintaining connectivity
    /// If `from` already has an edge in `direction` to some vertex `displaced`,
    /// create: from -> new_vertex -> displaced (preserving the chain)
    /// Returns the displaced vertex if there was one
    pub fn insert_vertex(&mut self, from: Uuid, new_vertex: Uuid, direction: Direction) -> Option<Uuid> {
        let dir_str = match direction {
            Direction::West => "west",
            Direction::East => "east",
            Direction::North => "north",
            Direction::South => "south",
            Direction::Up => "up",
            Direction::Down => "down",
        };

        // Find existing edge in this direction (if any)
        let displaced = self.edges.iter()
            .find(|e| e.from == from && e.direction == dir_str)
            .map(|e| e.to);

        // Remove the old edge from `from` in this direction
        self.edges.retain(|e| !(e.from == from && e.direction == dir_str));

        // Add edge: from -> new_vertex
        self.edges.push(Edge {
            from,
            to: new_vertex,
            direction: dir_str.to_string(),
        });

        // If there was a displaced vertex, add edge: new_vertex -> displaced
        if let Some(displaced_id) = displaced {
            self.edges.push(Edge {
                from: new_vertex,
                to: displaced_id,
                direction: dir_str.to_string(),
            });
        }

        self.meta.modified = Utc::now();
        displaced
    }

    /// Update vertex content
    pub fn update_vertex(&mut self, id: Uuid, transcript: Option<String>) -> Result<()> {
        let vertex = self
            .vertices
            .iter_mut()
            .find(|v| v.id == id)
            .ok_or_else(|| anyhow!("Vertex not found"))?;

        if let Some(t) = transcript {
            vertex.transcript = Some(t);
        }
        vertex.modified = Some(Utc::now());
        self.meta.modified = Utc::now();
        Ok(())
    }

    /// Build edge array for a vertex (for protocol)
    pub fn build_edge_array(&self, vertex_id: Uuid) -> [u64; 6] {
        let mut edges = [0u64; 6]; // [west, east, north, south, up, down]

        for (i, dir) in ["west", "east", "north", "south", "up", "down"].iter().enumerate() {
            if let Some(neighbor_id) = self.get_neighbor(vertex_id, dir) {
                edges[i] = uuid_to_hash(neighbor_id);
            }
        }

        edges
    }
}

/// Get file extension for MIME type
pub fn mime_to_extension(mime: &str) -> &'static str {
    match mime {
        "text/plain" => "txt",
        "text/markdown" => "md",
        "audio/ogg" => "ogg",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        _ => "bin",
    }
}

/// Convert UUID to u64 hash for protocol
pub fn uuid_to_hash(id: Uuid) -> u64 {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

/// Convert hash back to UUID (requires lookup in index)
pub fn hash_to_uuid(index: &NotesIndex, hash: u64) -> Option<Uuid> {
    index
        .vertices
        .iter()
        .find(|v| uuid_to_hash(v.id) == hash)
        .map(|v| v.id)
}

impl NotesIndex {
    /// Count total edges (in any direction) connected to a vertex
    pub fn count_edges(&self, vertex_id: Uuid) -> usize {
        let mut count = 0;
        for dir in ["west", "east", "north", "south", "up", "down"] {
            if self.get_neighbor(vertex_id, dir).is_some() {
                count += 1;
            }
        }
        count
    }

    /// Check if a vertex is a "fork" (has more than 2 edges, meaning it branches)
    pub fn is_fork(&self, vertex_id: Uuid) -> bool {
        self.count_edges(vertex_id) > 2
    }

    /// Get all neighbors of a vertex (in all directions)
    pub fn get_all_neighbors(&self, vertex_id: Uuid) -> Vec<(Uuid, &str)> {
        let mut neighbors = Vec::new();
        for dir in ["west", "east", "north", "south", "up", "down"] {
            if let Some(neighbor) = self.get_neighbor(vertex_id, dir) {
                neighbors.push((neighbor, dir));
            }
        }
        neighbors
    }

    /// Get vertices reachable from a starting vertex, stopping at landmarks.
    /// A vertex becomes a landmark boundary if:
    /// - It's a fork (more than 2 edges)
    /// - It's been `max_chain_length` vertices since the last landmark
    ///
    /// Returns: (vertices to send, landmark vertices at boundaries)
    pub fn get_vertices_within_landmark(
        &self,
        start_vertex: Uuid,
        max_chain_length: usize,
    ) -> (Vec<Uuid>, Vec<Uuid>) {
        use std::collections::{HashSet, VecDeque};

        let mut to_send: Vec<Uuid> = Vec::new();
        let mut landmarks: Vec<Uuid> = Vec::new();
        let mut visited: HashSet<Uuid> = HashSet::new();

        // BFS with chain length tracking
        // Each item is (vertex_id, distance_from_last_landmark)
        let mut queue: VecDeque<(Uuid, usize)> = VecDeque::new();
        queue.push_back((start_vertex, 0));

        while let Some((vertex_id, distance)) = queue.pop_front() {
            if visited.contains(&vertex_id) {
                continue;
            }
            visited.insert(vertex_id);

            let is_fork = self.is_fork(vertex_id);
            let at_chain_limit = distance >= max_chain_length;

            // Check if this vertex should be a landmark boundary
            if vertex_id != start_vertex && (is_fork || at_chain_limit) {
                // This is a landmark boundary - include it but don't traverse beyond
                landmarks.push(vertex_id);
                to_send.push(vertex_id);
                continue;
            }

            // Include this vertex
            to_send.push(vertex_id);

            // Continue traversal to neighbors
            let new_distance = if is_fork { 1 } else { distance + 1 };
            for (neighbor, _dir) in self.get_all_neighbors(vertex_id) {
                if !visited.contains(&neighbor) {
                    queue.push_back((neighbor, new_distance));
                }
            }
        }

        (to_send, landmarks)
    }

    /// Find the first vertex (by creation time) - used as root if no specific landmark
    pub fn get_root_vertex(&self) -> Option<Uuid> {
        self.vertices.first().map(|v| v.id)
    }

    /// Delete a vertex and reconnect its neighbors to maintain chains.
    /// For each axis (E/W, N/S, U/D), if the vertex has neighbors on both sides,
    /// connect them to each other. Returns the files to delete and the affected neighbor IDs.
    ///
    /// Returns: (files_to_delete, affected_neighbors)
    pub fn delete_vertex(&mut self, vertex_id: Uuid) -> Result<(Vec<String>, Vec<Uuid>)> {
        // Find the vertex
        let vertex = self.vertices.iter()
            .find(|v| v.id == vertex_id)
            .ok_or_else(|| anyhow!("Vertex not found"))?
            .clone();

        let mut files_to_delete = Vec::new();
        let mut affected_neighbors = Vec::new();

        // Collect files to delete
        files_to_delete.push(vertex.file.clone());
        for layer_content in vertex.layers.values() {
            files_to_delete.push(layer_content.file.clone());
        }

        // For each axis, get the two neighbors and reconnect them
        let axis_pairs = [
            ("west", "east"),
            ("north", "south"),
            ("up", "down"),
        ];

        for (dir_a, dir_b) in axis_pairs {
            let neighbor_a = self.get_neighbor(vertex_id, dir_a);
            let neighbor_b = self.get_neighbor(vertex_id, dir_b);

            // Track affected neighbors
            if let Some(n) = neighbor_a {
                if !affected_neighbors.contains(&n) {
                    affected_neighbors.push(n);
                }
            }
            if let Some(n) = neighbor_b {
                if !affected_neighbors.contains(&n) {
                    affected_neighbors.push(n);
                }
            }

            // If both neighbors exist, connect them
            if let (Some(na), Some(nb)) = (neighbor_a, neighbor_b) {
                // Connect neighbor_a's dir_b side to neighbor_b
                // (e.g., if deleting X in A-X-B, connect A's east to B)
                let direction = match dir_b {
                    "west" => Direction::West,
                    "east" => Direction::East,
                    "north" => Direction::North,
                    "south" => Direction::South,
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    _ => continue,
                };
                self.add_edge(na, nb, direction);
            }
        }

        // Remove all edges involving this vertex
        self.edges.retain(|e| e.from != vertex_id && e.to != vertex_id);

        // Remove the vertex
        self.vertices.retain(|v| v.id != vertex_id);

        self.meta.modified = Utc::now();

        Ok((files_to_delete, affected_neighbors))
    }
}
