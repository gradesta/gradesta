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

/// Build a 2D grid view from the graph, starting from the current vertex.
///
/// The algorithm:
/// 1. Place current vertex at origin (0, 0)
/// 2. Expand north/south along the center column (x=0)
/// 3. From each vertex in the center column, expand west/east horizontally only
///    (no vertical expansion from non-center columns)
///
/// This prevents unrelated subgraphs from appearing in the view while still
/// allowing navigation to connected vertices.
pub fn build_grid_view(graph: &GraphState, current_vertex: Option<u64>) -> GridView {
    let mut grid = GridView::default();
    let mut visited = HashSet::new();

    let current_id = match current_vertex {
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

    // Always ensure the current vertex is in the grid at (0, 0) with distance 0
    // This guarantees centering works correctly
    grid.cells.insert((0, 0), current_id);
    grid.positions.insert(current_id, (0, 0));
    grid.distances.insert((0, 0), 0);
    visited.insert(current_id);

    // If the current vertex exists in the graph, expand from it
    if let Some(current_vertex) = graph.vertices.get(&current_id) {
        // Expand north from current vertex
        let mut y = -1i32;
        let mut north_id = current_vertex.edges[EDGE_NORTH];
        while north_id != 0 && graph.vertices.contains_key(&north_id) && !visited.contains(&north_id) {
            let distance = (-y) as u32;
            grid.cells.insert((0, y), north_id);
            grid.positions.insert(north_id, (0, y));
            grid.distances.insert((0, y), distance);
            visited.insert(north_id);
            grid.min_y = grid.min_y.min(y);

            if let Some(v) = graph.vertices.get(&north_id) {
                north_id = v.edges[EDGE_NORTH];
            } else {
                break;
            }
            y -= 1;
        }

        // Expand south from current vertex
        let mut y = 1i32;
        let mut south_id = current_vertex.edges[EDGE_SOUTH];
        while south_id != 0 && graph.vertices.contains_key(&south_id) && !visited.contains(&south_id) {
            let distance = y as u32;
            grid.cells.insert((0, y), south_id);
            grid.positions.insert(south_id, (0, y));
            grid.distances.insert((0, y), distance);
            visited.insert(south_id);
            grid.max_y = grid.max_y.max(y);

            if let Some(v) = graph.vertices.get(&south_id) {
                south_id = v.edges[EDGE_SOUTH];
            } else {
                break;
            }
            y += 1;
        }
    }

    // Now expand west/east from all vertices in the center column
    // Sort by distance so closer vertices expand first and claim shared targets
    let mut center_vertices: Vec<(i32, u64, u32)> = grid.cells.iter()
        .filter(|((x, _), _)| *x == 0)
        .map(|((_, y), id)| (*y, *id, *grid.distances.get(&(0, *y)).unwrap_or(&u32::MAX)))
        .collect();
    center_vertices.sort_by_key(|(_, _, dist)| *dist);

    for (y, id, base_dist) in center_vertices {
        if let Some(v) = graph.vertices.get(&id) {
            if v.edges[EDGE_WEST] != 0 && !visited.contains(&v.edges[EDGE_WEST]) {
                expand_column_horizontal(&mut grid, graph, v.edges[EDGE_WEST], -1, y, base_dist + 1, &mut visited);
            }
            if v.edges[EDGE_EAST] != 0 && !visited.contains(&v.edges[EDGE_EAST]) {
                expand_column_horizontal(&mut grid, graph, v.edges[EDGE_EAST], 1, y, base_dist + 1, &mut visited);
            }
        }
    }

    // Detect ghost edges (connections to vertices that exist elsewhere in the grid)
    grid.ghost_edges = detect_ghost_edges(graph, &grid);

    grid
}

/// Expand horizontally from a vertex, placing it and continuing in the same direction.
/// Does not expand vertically from non-center columns to prevent unrelated graphs
/// from appearing.
fn expand_column_horizontal(
    grid: &mut GridView,
    graph: &GraphState,
    start_id: u64,
    x: i32,
    start_y: i32,
    base_distance: u32,
    visited: &mut HashSet<u64>,
) {
    if !graph.vertices.contains_key(&start_id) {
        return;
    }

    // If this vertex is already placed somewhere in the grid, check if we're offering
    // a closer position. If so, move it. If not, skip.
    if let Some(&existing_pos) = grid.positions.get(&start_id) {
        let existing_dist = *grid.distances.get(&existing_pos).unwrap_or(&u32::MAX);
        if base_distance < existing_dist {
            // This is a closer path to this vertex - move it to the new position
            grid.cells.remove(&existing_pos);
            grid.distances.remove(&existing_pos);
            // Don't remove from positions yet - will be updated below
        } else {
            // Already placed at a closer or equal distance, don't expand from here
            return;
        }
    }

    let pos = (x, start_y);

    // Check if the target cell already has a vertex at a closer distance
    let should_insert = match grid.distances.get(&pos) {
        None => true,
        Some(&existing_dist) => base_distance < existing_dist,
    };

    if !should_insert {
        // Cell already has a closer vertex, don't expand from here
        return;
    }

    // Remove old vertex from this cell if any
    if let Some(&old_vertex) = grid.cells.get(&pos) {
        if old_vertex != start_id {
            grid.positions.remove(&old_vertex);
        }
    }

    visited.insert(start_id);
    grid.cells.insert(pos, start_id);
    grid.positions.insert(start_id, pos);
    grid.distances.insert(pos, base_distance);

    grid.min_x = grid.min_x.min(x);
    grid.max_x = grid.max_x.max(x);
    grid.min_y = grid.min_y.min(start_y);
    grid.max_y = grid.max_y.max(start_y);

    // Continue expanding horizontally (but not vertically)
    if let Some(v) = graph.vertices.get(&start_id) {
        if x < 0 && v.edges[EDGE_WEST] != 0 {
            expand_column_horizontal(grid, graph, v.edges[EDGE_WEST], x - 1, start_y, base_distance + 1, visited);
        }
        if x > 0 && v.edges[EDGE_EAST] != 0 {
            expand_column_horizontal(grid, graph, v.edges[EDGE_EAST], x + 1, start_y, base_distance + 1, visited);
        }
    }
}
