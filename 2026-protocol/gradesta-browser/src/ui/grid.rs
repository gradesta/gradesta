//! Central grid view rendering for UI system
//!
//! Renders the main navigation grid showing vertices and their connections.

use bevy_egui::egui;

use crate::graph::{build_grid_view, GraphState};
use crate::media::MediaCache;
use crate::network::{WsCommand, WsCommandTx};
use crate::rendering::render_vertex_card;
use crate::state::AppState;
use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

/// Action from grid rendering
#[derive(Clone, Debug)]
pub enum GridAction {
    None,
    ClickVertex { vertex_id: u64 },
}

/// Render the central grid view
///
/// Returns an action if the user clicked a vertex.
pub fn render_grid_view(
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
    ws_cmd_tx: &WsCommandTx,
) -> GridAction {
    let mut action = GridAction::None;

    egui::CentralPanel::default().show(ctx, |ui| {
        if !app_state.connected {
            ui.centered_and_justified(|ui| {
                ui.heading("Enter a WebSocket URL above and click Connect");
            });
            return;
        }

        let Some(current_id) = app_state.current_vertex else {
            ui.centered_and_justified(|ui| {
                if app_state.pending_identification.is_some() {
                    ui.heading("Server requires authentication");
                    ui.label("Please respond to the identification request in the sidebar.");
                } else {
                    ui.heading("Waiting for data from server...");
                }
            });
            return;
        };

        let grid = build_grid_view(graph, Some(current_id));

        let zoom = app_state.zoom_level;
        let cell_width = 160.0f32 * zoom;
        let cell_height = 140.0f32 * zoom;
        let padding = 4.0f32 * zoom;
        let font_size = 13.0f32 * zoom;

        let available = ui.available_size();
        let panel_min = ui.min_rect().min;

        // Find the position of the current vertex in the grid
        let current_pos = grid.positions.get(&current_id).copied().unwrap_or((0, 0));

        // Calculate where the current cell would be in grid-local coordinates
        let current_cell_x = (current_pos.0 - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
        let current_cell_y = (current_pos.1 - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;

        // Calculate offset to center the current cell in the available space
        let offset_x = available.x / 2.0 - current_cell_x;
        let offset_y = available.y / 2.0 - current_cell_y;

        // Get the actual CentralPanel bounds - ui.max_rect() is the panel's allocated area
        // ui.clip_rect() returns the full content_rect which includes other panels
        let panel_rect = ui.max_rect();
        // Create a painter that clips to the CentralPanel area only
        let painter = ui.painter().with_clip_rect(panel_rect);
        let base_pos = panel_min + egui::vec2(offset_x, offset_y);

        // Draw edge lines first (behind cells)
        let line_color_ns = egui::Color32::from_rgb(80, 120, 100); // North-South (vertical)
        let line_color_ew = egui::Color32::from_rgb(100, 80, 120); // East-West (horizontal)
        let line_thickness = 2.5 * zoom;

        for ((x, y), &vertex_id) in &grid.cells {
            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                let from_cell_x = (*x - grid.min_x) as f32 * (cell_width + padding);
                let from_cell_y = (*y - grid.min_y) as f32 * (cell_height + padding);

                // Draw line to south neighbor
                if vertex.edges[EDGE_SOUTH] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_SOUTH]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = (ty - grid.min_y) as f32 * (cell_height + padding);

                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width / 2.0, from_cell_y + cell_height);
                        let to_edge = base_pos + egui::vec2(to_cell_x + cell_width / 2.0, to_cell_y);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ns));
                    }
                }

                // Draw line to east neighbor
                if vertex.edges[EDGE_EAST] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_EAST]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = (ty - grid.min_y) as f32 * (cell_height + padding);

                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width, from_cell_y + cell_height / 2.0);
                        let to_edge = base_pos + egui::vec2(to_cell_x, to_cell_y + cell_height / 2.0);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ew));
                    }
                }
            }
        }

        // Draw cells
        for y in grid.min_y..=grid.max_y {
            for x in grid.min_x..=grid.max_x {
                if let Some(&vertex_id) = grid.cells.get(&(x, y)) {
                    let cell_x = (x - grid.min_x) as f32 * (cell_width + padding);
                    let cell_y = (y - grid.min_y) as f32 * (cell_height + padding);

                    let rect = egui::Rect::from_min_size(
                        base_pos + egui::vec2(cell_x, cell_y),
                        egui::vec2(cell_width, cell_height),
                    );

                    let is_current = vertex_id == current_id;

                    if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        render_vertex_card(&painter, vertex, vertex_id, rect, is_current, zoom, font_size, media_cache, ctx, graph);

                        // Draw direction arrow indicator on current cell
                        if is_current {
                            draw_direction_arrow(&painter, rect, app_state.last_nav_direction, zoom);
                        }

                        // Draw ghost edge indicators
                        if let Some(ghosts) = grid.ghost_edges.get(&vertex_id) {
                            draw_ghost_indicators(&painter, rect, ghosts, zoom);
                        }
                    } else {
                        // No vertex data - just draw empty cell
                        let corner_radius = 4.0 * zoom;
                        painter.rect_filled(rect, corner_radius, egui::Color32::from_rgb(50, 50, 55));
                        painter.rect_stroke(rect, corner_radius, egui::Stroke::new(2.0 * zoom, egui::Color32::from_rgb(80, 80, 90)));
                    }

                    // Detect clicks on cells
                    let response = ui.interact(rect, egui::Id::new(("cell", vertex_id)), egui::Sense::click());
                    if response.clicked() && !is_current {
                        if let Some(ref tx) = ws_cmd_tx.0 {
                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                            let _ = tx.send(WsCommand::ClickVertex { action_id, vertex_id });
                            app_state.status = format!("Clicked vertex {}", vertex_id);
                        }
                        action = GridAction::ClickVertex { vertex_id };
                    }
                }
            }
        }
    });

    action
}

fn draw_direction_arrow(painter: &egui::Painter, rect: egui::Rect, direction: usize, zoom: f32) {
    let arrow_color = egui::Color32::from_rgba_unmultiplied(100, 255, 150, 200);
    let arrow_size = 12.0 * zoom;
    let arrow_thickness = 3.0 * zoom;
    let gap = 4.0 * zoom;

    let (arrow_start, arrow_end) = match direction {
        EDGE_EAST => {
            let mid_y = rect.center().y;
            let start = egui::pos2(rect.right() + gap, mid_y);
            let end = egui::pos2(rect.right() + gap + arrow_size, mid_y);
            (start, end)
        }
        EDGE_WEST => {
            let mid_y = rect.center().y;
            let start = egui::pos2(rect.left() - gap, mid_y);
            let end = egui::pos2(rect.left() - gap - arrow_size, mid_y);
            (start, end)
        }
        EDGE_NORTH => {
            let mid_x = rect.center().x;
            let start = egui::pos2(mid_x, rect.top() - gap);
            let end = egui::pos2(mid_x, rect.top() - gap - arrow_size);
            (start, end)
        }
        EDGE_SOUTH => {
            let mid_x = rect.center().x;
            let start = egui::pos2(mid_x, rect.bottom() + gap);
            let end = egui::pos2(mid_x, rect.bottom() + gap + arrow_size);
            (start, end)
        }
        EDGE_UP => {
            let start = egui::pos2(rect.right() - 8.0 * zoom, rect.top() - gap);
            let end = egui::pos2(rect.right() - 8.0 * zoom, rect.top() - gap - arrow_size);
            (start, end)
        }
        EDGE_DOWN => {
            let start = egui::pos2(rect.right() - 8.0 * zoom, rect.bottom() + gap);
            let end = egui::pos2(rect.right() - 8.0 * zoom, rect.bottom() + gap + arrow_size);
            (start, end)
        }
        _ => {
            let mid_x = rect.center().x;
            let start = egui::pos2(mid_x, rect.bottom() + gap);
            let end = egui::pos2(mid_x, rect.bottom() + gap + arrow_size);
            (start, end)
        }
    };

    painter.line_segment([arrow_start, arrow_end], egui::Stroke::new(arrow_thickness, arrow_color));

    // Draw arrow head
    let dir = (arrow_end - arrow_start).normalized();
    let perp = egui::vec2(-dir.y, dir.x);
    let head_size = 6.0 * zoom;
    let head_base = arrow_end - dir * head_size;
    let head_left = head_base + perp * head_size * 0.5;
    let head_right = head_base - perp * head_size * 0.5;
    painter.add(egui::Shape::convex_polygon(
        vec![arrow_end, head_left, head_right],
        arrow_color,
        egui::Stroke::NONE,
    ));
}

fn draw_ghost_indicators(painter: &egui::Painter, rect: egui::Rect, ghosts: &[crate::graph::GhostEdge], zoom: f32) {
    let ghost_color = egui::Color32::from_rgba_unmultiplied(180, 100, 255, 180);
    let ghost_size = 8.0 * zoom;

    for ghost in ghosts {
        let (indicator_pos, icon) = match ghost.direction {
            EDGE_WEST => (egui::pos2(rect.left() + 4.0 * zoom, rect.center().y), "◀"),
            EDGE_EAST => (egui::pos2(rect.right() - 4.0 * zoom, rect.center().y), "▶"),
            EDGE_NORTH => (egui::pos2(rect.center().x, rect.top() + 4.0 * zoom), "▲"),
            EDGE_SOUTH => (egui::pos2(rect.center().x, rect.bottom() - 4.0 * zoom), "▼"),
            _ => continue,
        };

        painter.circle_filled(indicator_pos, ghost_size, egui::Color32::from_rgba_unmultiplied(100, 50, 150, 100));
        painter.circle_stroke(indicator_pos, ghost_size, egui::Stroke::new(2.0 * zoom, ghost_color));
        painter.text(
            indicator_pos,
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(ghost_size * 0.8),
            ghost_color,
        );
    }
}
