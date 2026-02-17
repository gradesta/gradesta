//! File-based storage for the test server

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

const INDEX_FILE: &str = "index.toml";

/// A vertex in the graph
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub id: Uuid,
    pub mime: String,
    pub file: String,
    pub created: DateTime<Utc>,
}

impl Vertex {
    pub fn new(id: Uuid, mime: &str, file: &str) -> Self {
        Self {
            id,
            mime: mime.to_string(),
            file: file.to_string(),
            created: Utc::now(),
        }
    }

    /// Hash the UUID to a u64 for protocol
    pub fn id_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        hasher.finish()
    }
}

/// An edge connecting two vertices
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: Uuid,
    pub to: Uuid,
    pub direction: u8, // 0=W, 1=E, 2=N, 3=S, 4=U, 5=D
}

/// Notes index
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotesIndex {
    pub version: u32,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
}

impl Default for NotesIndex {
    fn default() -> Self {
        Self {
            version: 1,
            vertices: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl NotesIndex {
    /// Find a vertex by its hash
    pub fn find_vertex(&self, hash: u64) -> Option<&Vertex> {
        self.vertices.iter().find(|v| v.id_hash() == hash)
    }

    /// Find a vertex by its hash (mutable)
    pub fn find_vertex_mut(&mut self, hash: u64) -> Option<&mut Vertex> {
        self.vertices.iter_mut().find(|v| v.id_hash() == hash)
    }

    /// Find UUID from hash
    pub fn find_uuid(&self, hash: u64) -> Option<Uuid> {
        self.find_vertex(hash).map(|v| v.id)
    }

    /// Add an edge
    pub fn add_edge(&mut self, from: Uuid, to: Uuid, direction: u8) {
        // Remove existing edge in this direction
        self.edges.retain(|e| !(e.from == from && e.direction == direction));
        self.edges.push(Edge { from, to, direction });
    }

    /// Build edges array for a vertex
    pub fn build_edges(&self, vertex_id: Uuid) -> [u64; 6] {
        let mut edges = [0u64; 6];

        for edge in &self.edges {
            if edge.from == vertex_id && (edge.direction as usize) < 6 {
                if let Some(to_vertex) = self.vertices.iter().find(|v| v.id == edge.to) {
                    edges[edge.direction as usize] = to_vertex.id_hash();
                }
            }
            // Reverse edge
            if edge.to == vertex_id && (edge.direction as usize) < 6 {
                let reverse_dir = match edge.direction {
                    0 => 1, // W -> E
                    1 => 0, // E -> W
                    2 => 3, // N -> S
                    3 => 2, // S -> N
                    4 => 5, // U -> D
                    5 => 4, // D -> U
                    _ => continue,
                };
                if let Some(from_vertex) = self.vertices.iter().find(|v| v.id == edge.from) {
                    edges[reverse_dir] = from_vertex.id_hash();
                }
            }
        }

        edges
    }
}

/// File-based storage client
#[derive(Clone)]
pub struct StorageClient {
    base_path: PathBuf,
}

impl StorageClient {
    pub fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
        }
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.base_path.join(path.trim_start_matches('/'))
    }

    pub async fn upload(&self, path: &str, content: &[u8]) -> Result<()> {
        let full_path = self.full_path(path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&full_path, content).await?;
        Ok(())
    }

    pub async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let full_path = self.full_path(path);
        Ok(fs::read(&full_path).await?)
    }

    pub async fn load_index(&self) -> Result<NotesIndex> {
        let index_path = self.full_path(INDEX_FILE);

        if !index_path.exists() {
            return Ok(NotesIndex::default());
        }

        let content = fs::read_to_string(&index_path).await?;
        Ok(toml::from_str(&content)?)
    }

    pub async fn save_index(&self, index: &NotesIndex) -> Result<()> {
        let content = toml::to_string_pretty(index)?;
        let index_path = self.full_path(INDEX_FILE);
        fs::write(&index_path, content).await?;
        Ok(())
    }
}
