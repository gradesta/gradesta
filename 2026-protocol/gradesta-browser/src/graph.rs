//! Graph data structures and layout algorithms.
//!
//! This module contains the core graph types (Vertex, edges) and the algorithms
//! for computing 2D grid layouts from the 6-directional graph structure.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

/// Layer content for a vertex - each layer has its own MIME type and data
#[derive(Clone, Debug, Default)]
pub struct LayerContent {
    pub mime: String,
    pub data: Vec<u8>,
}

/// A vertex in the gradesta graph
#[derive(Clone, Debug, Default)]
pub struct Vertex {
    pub id: u64,
    /// Primary layer data (layer 0) for backwards compatibility
    pub label: Vec<u8>,
    /// Layer 0 MIME type
    pub mime: Option<String>,
    /// Edges: [West, East, North, South, Up, Down]
    pub edges: [u64; 6],
    /// Additional layers (layer_id -> content), each with its own MIME type
    pub layers: HashMap<u32, LayerContent>,
    /// Edit mask: bit 0-5 for edge editability, bit 6 for label editability
    /// 0x7F = all editable, 0 = read-only
    pub edit_mask: u8,
}

/// Graph state resource holding all vertices and context information
#[derive(Resource, Default)]
pub struct GraphState {
    pub vertices: HashMap<u64, Vertex>,
    pub context_uri: Option<String>,
    /// If set, jump to first non-portal vertex of this context
    pub pending_jump_context: Option<String>,
    /// Maps landmark URI -> vertices that belong to it
    pub landmark_vertices: HashMap<String, Vec<u64>>,
    /// Which landmark we're currently receiving data for
    pub current_receiving_landmark: Option<String>,
}

/// Information about a stack of vertices connected via up/down edges
pub struct StackInfo {
    /// All vertices in the stack (top to bottom)
    pub vertices: Vec<u64>,
    /// Current position in the stack (0-indexed)
    pub current_index: usize,
    /// Total number of vertices in the stack
    pub total: usize,
}

/// Compute the stack of vertices connected via up/down edges
pub fn compute_stack(graph: &GraphState, vertex_id: u64) -> StackInfo {
    let mut stack = Vec::new();
    let mut visited = HashSet::new();

    // Find the top of the stack
    let mut top_id = vertex_id;
    while let Some(v) = graph.vertices.get(&top_id) {
        if v.edges[EDGE_UP] != 0 && !visited.contains(&v.edges[EDGE_UP]) {
            visited.insert(top_id);
            top_id = v.edges[EDGE_UP];
        } else {
            break;
        }
    }

    // Now traverse down from top, collecting all vertices
    visited.clear();
    let mut current = top_id;
    let mut current_index = 0;
    let mut found_index = false;

    while let Some(v) = graph.vertices.get(&current) {
        if visited.contains(&current) {
            break;
        }
        visited.insert(current);
        stack.push(current);

        if current == vertex_id {
            current_index = stack.len() - 1;
            found_index = true;
        }

        if v.edges[EDGE_DOWN] != 0 {
            current = v.edges[EDGE_DOWN];
        } else {
            break;
        }
    }

    if !found_index && !stack.is_empty() {
        current_index = 0;
    }

    StackInfo {
        total: stack.len(),
        vertices: stack,
        current_index,
    }
}

/// A ghost edge represents a connection to a vertex that exists elsewhere in the grid
#[derive(Clone, Debug)]
pub struct GhostEdge {
    /// The direction of the edge (EDGE_WEST, EDGE_EAST, etc.)
    pub direction: usize,
    /// The target vertex id (which exists elsewhere in the grid)
    pub target_id: u64,
    /// The position where the target vertex actually is in the grid
    pub target_pos: (i32, i32),
}

/// 2D grid layout of the graph
#[derive(Default, Clone)]
pub struct GridView {
    /// Maps grid position to vertex ID
    pub cells: HashMap<(i32, i32), u64>,
    /// Maps vertex ID to grid position
    pub positions: HashMap<u64, (i32, i32)>,
    /// Distance from current vertex for each cell (used to resolve overlapping branches)
    pub distances: HashMap<(i32, i32), u32>,
    /// Ghost edges: vertex_id -> list of ghost edges from that vertex
    pub ghost_edges: HashMap<u64, Vec<GhostEdge>>,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// Direction offset for grid layout
const DIR_OFFSETS: [(i32, i32); 6] = [
    (-1, 0),  // West
    (1, 0),   // East
    (0, -1),  // North
    (0, 1),   // South
    (0, 0),   // Up (stacked, no grid offset)
    (0, 0),   // Down (stacked, no grid offset)
];

/// Get the expected neighbor position for a direction
pub fn expected_neighbor_pos(pos: (i32, i32), direction: usize) -> (i32, i32) {
    let offset = DIR_OFFSETS[direction];
    (pos.0 + offset.0, pos.1 + offset.1)
}

/// Detect ghost edges - connections where the target vertex is placed elsewhere in the grid
pub fn detect_ghost_edges(graph: &GraphState, grid: &GridView) -> HashMap<u64, Vec<GhostEdge>> {
    let mut ghost_edges: HashMap<u64, Vec<GhostEdge>> = HashMap::new();

    for (&vertex_id, &pos) in &grid.positions {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Check horizontal/vertical edges (not up/down which are stacked)
            for direction in [EDGE_WEST, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH] {
                let neighbor_id = vertex.edges[direction];
                if neighbor_id == 0 {
                    continue;
                }

                // Check if the neighbor exists in the grid
                if let Some(&neighbor_pos) = grid.positions.get(&neighbor_id) {
                    let expected = expected_neighbor_pos(pos, direction);
                    // If the neighbor is not where we expect it, it's a ghost edge
                    if neighbor_pos != expected {
                        ghost_edges.entry(vertex_id).or_default().push(GhostEdge {
                            direction,
                            target_id: neighbor_id,
                            target_pos: neighbor_pos,
                        });
                    }
                }
            }
        }
    }

    ghost_edges
}

/// Find disconnected vertex clusters (islands) in the graph
pub fn find_islands(graph: &GraphState, starting_vertex: Option<u64>) -> Vec<Vec<u64>> {
    let mut visited: HashSet<u64> = HashSet::new();
    let mut islands: Vec<Vec<u64>> = Vec::new();

    // Get all vertex IDs
    let all_vertices: Vec<u64> = graph.vertices.keys().copied().collect();

    // Start with the current vertex's island if specified
    if let Some(start) = starting_vertex {
        if graph.vertices.contains_key(&start) {
            let island = collect_island(graph, start, &mut visited);
            if !island.is_empty() {
                islands.push(island);
            }
        }
    }

    // Find remaining islands
    for &vertex_id in &all_vertices {
        if !visited.contains(&vertex_id) {
            let island = collect_island(graph, vertex_id, &mut visited);
            if !island.is_empty() {
                islands.push(island);
            }
        }
    }

    islands
}

/// Collect all vertices in an island starting from a given vertex
fn collect_island(graph: &GraphState, start: u64, visited: &mut HashSet<u64>) -> Vec<u64> {
    let mut island = Vec::new();
    let mut stack = vec![start];

    while let Some(vertex_id) = stack.pop() {
        if visited.contains(&vertex_id) {
            continue;
        }
        visited.insert(vertex_id);

        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            island.push(vertex_id);

            // Add all connected vertices to the stack
            for &neighbor_id in &vertex.edges {
                if neighbor_id != 0 && !visited.contains(&neighbor_id) {
                    stack.push(neighbor_id);
                }
            }
        }
    }

    island
}

/// Build a 2D grid view from the graph, starting from the current vertex
pub fn build_grid_view(graph: &GraphState, current_vertex: Option<u64>) -> GridView {
    let mut grid = GridView::default();

    let start = match current_vertex {
        Some(id) if graph.vertices.contains_key(&id) => id,
        _ => {
            // No current vertex, just pick any vertex
            if let Some(&id) = graph.vertices.keys().next() {
                id
            } else {
                return grid;
            }
        }
    };

    // Place the starting vertex at origin
    grid.cells.insert((0, 0), start);
    grid.positions.insert(start, (0, 0));
    grid.distances.insert((0, 0), 0);

    // Expand outward using BFS
    let mut queue: Vec<(u64, (i32, i32), u32)> = vec![(start, (0, 0), 0)];
    let mut visited: HashSet<u64> = HashSet::new();
    visited.insert(start);

    while let Some((vertex_id, pos, distance)) = queue.pop() {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Process horizontal and vertical edges
            for direction in [EDGE_WEST, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH] {
                let neighbor_id = vertex.edges[direction];
                if neighbor_id == 0 || visited.contains(&neighbor_id) {
                    continue;
                }

                let new_pos = expected_neighbor_pos(pos, direction);
                let new_distance = distance + 1;

                // Check if position is already occupied
                if let Some(&existing_distance) = grid.distances.get(&new_pos) {
                    // Only replace if we're closer
                    if new_distance >= existing_distance {
                        continue;
                    }
                    // Remove the old vertex from this position
                    if let Some(&old_id) = grid.cells.get(&new_pos) {
                        grid.positions.remove(&old_id);
                    }
                }

                visited.insert(neighbor_id);
                grid.cells.insert(new_pos, neighbor_id);
                grid.positions.insert(neighbor_id, new_pos);
                grid.distances.insert(new_pos, new_distance);
                queue.push((neighbor_id, new_pos, new_distance));
            }
        }
    }

    // Compute bounds
    if !grid.cells.is_empty() {
        grid.min_x = grid.cells.keys().map(|p| p.0).min().unwrap_or(0);
        grid.max_x = grid.cells.keys().map(|p| p.0).max().unwrap_or(0);
        grid.min_y = grid.cells.keys().map(|p| p.1).min().unwrap_or(0);
        grid.max_y = grid.cells.keys().map(|p| p.1).max().unwrap_or(0);
    }

    // Detect ghost edges
    grid.ghost_edges = detect_ghost_edges(graph, &grid);

    grid
}
