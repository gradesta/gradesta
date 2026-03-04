//! Graph data structures and layout algorithms.
//!
//! This module contains the core graph types (Vertex, edges) and the algorithms
//! for computing 2D grid layouts from the 6-directional graph structure.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::state::{LoadingPortalCell, PendingAudioCell, EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

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
        current_index,
    }
}

/// A ghost edge represents a connection to a vertex that exists elsewhere in the grid
#[derive(Clone, Debug)]
pub struct GhostEdge {
    /// The direction of the edge (EDGE_WEST, EDGE_EAST, etc.)
    pub direction: usize,
}

/// A placeholder cell for a pending audio recording
#[derive(Clone, Debug)]
pub struct PlaceholderCell {
    /// The local ID of the pending audio cell
    pub local_id: u64,
    /// Grid position
    pub position: (i32, i32),
    /// Reference to the pending cell data (cloned for rendering)
    pub pending_cell: PendingAudioCell,
}

/// A placeholder cell for a loading portal
#[derive(Clone, Debug)]
pub struct LoadingPortalPlaceholder {
    /// Grid position
    pub position: (i32, i32),
    /// When loading started (for animation)
    pub created_at: std::time::Instant,
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
    /// Placeholder cells for pending audio recordings
    pub placeholder_cells: Vec<PlaceholderCell>,
    /// Placeholder for loading portal (if any)
    pub loading_portal: Option<LoadingPortalPlaceholder>,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// Get the opposite direction for bidirectional edge checking
fn opposite_direction(dir: usize) -> usize {
    match dir {
        EDGE_WEST => EDGE_EAST,
        EDGE_EAST => EDGE_WEST,
        EDGE_NORTH => EDGE_SOUTH,
        EDGE_SOUTH => EDGE_NORTH,
        EDGE_UP => EDGE_DOWN,
        EDGE_DOWN => EDGE_UP,
        _ => dir,
    }
}

/// Check if a vertex has a reverse edge back to the source
fn has_reverse_edge(graph: &GraphState, from_id: u64, to_id: u64, direction: usize) -> bool {
    if let Some(target) = graph.vertices.get(&to_id) {
        target.edges[opposite_direction(direction)] == from_id
    } else {
        false
    }
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

/// Build a 2D grid view from the graph, starting from the current vertex.
///
/// The algorithm uses center-out BFS:
/// 1. Place current vertex at origin (0, 0)
/// 2. BFS outward, for each placed cell check its 4 neighbors (W, E, N, S)
/// 3. Only place a neighbor if:
///    - It has a bidirectional edge back to the source
///    - The target grid position is not already occupied
///
/// This prevents ghost cells from appearing when orphan vertices have
/// one-way edges pointing to unrelated parts of the graph.
///
/// Also includes pending audio cells as placeholder cells in the appropriate positions.
pub fn build_grid_view(
    graph: &GraphState,
    current_vertex: Option<u64>,
    pending_audio_cells: &HashMap<u64, PendingAudioCell>,
    loading_portal_cell: Option<&LoadingPortalCell>,
) -> GridView {
    use std::collections::VecDeque;

    let mut grid = GridView::default();

    let current_id = match current_vertex {
        Some(id) if graph.vertices.contains_key(&id) => id,
        _ => {
            // No valid current vertex - return empty grid rather than
            // picking a random vertex which could be from any landmark
            return grid;
        }
    };

    // Place center vertex at origin
    grid.cells.insert((0, 0), current_id);
    grid.positions.insert(current_id, (0, 0));
    grid.distances.insert((0, 0), 0);

    // BFS queue: (vertex_id, position, distance)
    let mut queue: VecDeque<(u64, (i32, i32), u32)> = VecDeque::new();
    queue.push_back((current_id, (0, 0), 0));

    // Directions to expand: W, E, N, S (indices 0-3)
    let directions = [EDGE_WEST, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH];

    while let Some((vertex_id, pos, distance)) = queue.pop_front() {
        let vertex = match graph.vertices.get(&vertex_id) {
            Some(v) => v,
            None => continue,
        };

        for &dir in &directions {
            let neighbor_id = vertex.edges[dir];
            if neighbor_id == 0 {
                continue;
            }

            // Check bidirectional edge - neighbor must point back to us
            if !has_reverse_edge(graph, vertex_id, neighbor_id, dir) {
                continue;
            }

            // Calculate target position
            let offset = DIR_OFFSETS[dir];
            let target_pos = (pos.0 + offset.0, pos.1 + offset.1);

            // Skip if position already occupied
            if grid.cells.contains_key(&target_pos) {
                continue;
            }

            // Skip if vertex already placed elsewhere
            if grid.positions.contains_key(&neighbor_id) {
                continue;
            }

            // Place the vertex
            let new_distance = distance + 1;
            grid.cells.insert(target_pos, neighbor_id);
            grid.positions.insert(neighbor_id, target_pos);
            grid.distances.insert(target_pos, new_distance);

            // Update bounds
            grid.min_x = grid.min_x.min(target_pos.0);
            grid.max_x = grid.max_x.max(target_pos.0);
            grid.min_y = grid.min_y.min(target_pos.1);
            grid.max_y = grid.max_y.max(target_pos.1);

            // Add to queue for further expansion
            queue.push_back((neighbor_id, target_pos, new_distance));
        }
    }

    // Add placeholder cells for pending audio recordings
    for (local_id, pending_cell) in pending_audio_cells {
        // Only show placeholders for cells connected to vertices currently in the grid
        if let Some(&from_pos) = grid.positions.get(&pending_cell.from_vertex) {
            // Calculate the position where this pending cell should appear
            let offset = DIR_OFFSETS[pending_cell.direction];
            let placeholder_pos = (from_pos.0 + offset.0, from_pos.1 + offset.1);

            // Always add to positions map so centering works (even if cell can't be shown)
            grid.positions.insert(*local_id, placeholder_pos);

            // Only add visible placeholder if the position is not already occupied
            if !grid.cells.contains_key(&placeholder_pos) {
                grid.placeholder_cells.push(PlaceholderCell {
                    local_id: *local_id,
                    position: placeholder_pos,
                    pending_cell: pending_cell.clone(),
                });

                // Update grid bounds
                grid.min_x = grid.min_x.min(placeholder_pos.0);
                grid.max_x = grid.max_x.max(placeholder_pos.0);
                grid.min_y = grid.min_y.min(placeholder_pos.1);
                grid.max_y = grid.max_y.max(placeholder_pos.1);
            }
        }
    }

    // Add loading portal placeholder if present
    if let Some(loading_cell) = loading_portal_cell {
        if let Some(&from_pos) = grid.positions.get(&loading_cell.from_vertex) {
            let offset = DIR_OFFSETS[loading_cell.direction];
            let placeholder_pos = (from_pos.0 + offset.0, from_pos.1 + offset.1);

            // Only add if position is not occupied
            if !grid.cells.contains_key(&placeholder_pos) {
                grid.loading_portal = Some(LoadingPortalPlaceholder {
                    position: placeholder_pos,
                    created_at: loading_cell.created_at,
                });

                // Update grid bounds
                grid.min_x = grid.min_x.min(placeholder_pos.0);
                grid.max_x = grid.max_x.max(placeholder_pos.0);
                grid.min_y = grid.min_y.min(placeholder_pos.1);
                grid.max_y = grid.max_y.max(placeholder_pos.1);
            }
        }
    }

    // Detect ghost edges (connections to vertices that exist elsewhere in the grid)
    grid.ghost_edges = detect_ghost_edges(graph, &grid);

    grid
}
