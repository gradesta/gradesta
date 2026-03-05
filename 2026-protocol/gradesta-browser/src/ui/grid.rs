//! Central grid view rendering for UI system
//!
//! Renders the main navigation grid showing vertices and their connections.

use bevy_egui::egui;

use std::collections::HashMap;

use crate::graph::{build_grid_view, GraphState, GridView, PlaceholderCell, PortalShadow, Vertex};
use crate::media::{is_image_data, MediaCache};
use crate::network::{WsCommand, WsCommandTx};
use crate::rendering::render_vertex_card;
use crate::state::{AppState, PendingAudioStatus};
use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

/// Action from grid rendering
#[derive(Clone, Debug)]
pub enum GridAction {
    None,
    ClickVertex,
}

/// Calculate the appropriate height for a cell based on its content
/// Uses cached textures only to avoid expensive loading during layout
fn calculate_cell_height(
    vertex: &Vertex,
    vertex_id: u64,
    cell_width: f32,
    max_height: f32,
    zoom: f32,
    media_cache: &MediaCache,
) -> f32 {
    let min_height = 80.0 * zoom;
    let padding = 8.0 * zoom;
    let content_width = cell_width - padding * 2.0;

    let mime = vertex.mime.as_deref().unwrap_or("");
    let primary_is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
    let primary_is_audio = mime.starts_with("audio/");
    let primary_is_text = mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url";

    // Count content sections (same logic as rendering.rs)
    let mut has_image = primary_is_image;
    let mut has_audio = primary_is_audio;
    let mut has_text = primary_is_text;
    let mut text_content: Option<String> = None;

    // Check additional layers
    for layer in vertex.layers.values() {
        if layer.mime.starts_with("image/") || is_image_data(&layer.data) {
            has_image = true;
        } else if layer.mime.starts_with("audio/") {
            has_audio = true;
        } else if layer.mime.starts_with("text/") && !layer.mime.contains("gradesta-url") {
            has_text = true;
            if text_content.is_none() {
                text_content = String::from_utf8(layer.data.clone()).ok();
            }
        }
    }

    // For primary text content
    if primary_is_text && text_content.is_none() {
        text_content = String::from_utf8(vertex.label.clone()).ok();
    }

    let num_sections = (has_image as usize) + (has_audio as usize) + (has_text as usize);
    if num_sections == 0 {
        return min_height;
    }

    let mut total_height = padding * 2.0; // Top and bottom padding

    // Image section height - use cached texture if available, otherwise estimate
    if has_image {
        if let Some(tex) = media_cache.textures.get(&vertex_id) {
            let aspect = tex.size_vec2().x / tex.size_vec2().y;
            let image_height = content_width / aspect;
            total_height += image_height.min(max_height * 0.6);
        } else {
            // Default to square-ish aspect for uncached images
            total_height += content_width * 0.75;
        }
    }

    // Audio waveform section - fixed height
    if has_audio {
        total_height += 60.0 * zoom;
    }

    // Text section height
    if has_text {
        if let Some(ref text) = text_content {
            let font_size = 13.0 * zoom * 0.9;
            let char_width = font_size * 0.5;
            let line_height = font_size * 1.2;
            let chars_per_line = (content_width / char_width) as usize;
            let chars_per_line = chars_per_line.max(5);

            // Count wrapped lines
            let mut line_count = 0usize;
            let mut current_line_len = 0usize;
            for word in text.split_whitespace() {
                let word_len = word.chars().count();
                if current_line_len == 0 {
                    current_line_len = word_len;
                } else if current_line_len + 1 + word_len <= chars_per_line {
                    current_line_len += 1 + word_len;
                } else {
                    line_count += 1;
                    current_line_len = word_len;
                }
            }
            if current_line_len > 0 {
                line_count += 1;
            }

            // Cap at 20 lines for reasonable sizing
            let line_count = line_count.min(20).max(1);
            total_height += line_count as f32 * line_height;
        } else {
            // Fallback for text without content
            total_height += 40.0 * zoom;
        }
    }

    total_height.min(max_height).max(min_height)
}

/// Calculate row heights based on maximum cell height in each row
fn calculate_row_heights(
    grid: &GridView,
    graph: &GraphState,
    cell_width: f32,
    max_height: f32,
    zoom: f32,
    media_cache: &MediaCache,
) -> HashMap<i32, f32> {
    let mut row_heights: HashMap<i32, f32> = HashMap::new();
    let min_height = 80.0 * zoom;

    // For each cell in the grid, calculate its height and track max per row
    for ((_, y), &vertex_id) in &grid.cells {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            let height = calculate_cell_height(vertex, vertex_id, cell_width, max_height, zoom, media_cache);
            let current_max = row_heights.get(y).copied().unwrap_or(min_height);
            row_heights.insert(*y, current_max.max(height));
        }
    }

    // Ensure all rows have at least minimum height
    for y in grid.min_y..=grid.max_y {
        row_heights.entry(y).or_insert(min_height);
    }

    row_heights
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

        let grid = build_grid_view(graph, Some(current_id), &app_state.pending_audio_cells, app_state.loading_portal_cell.as_ref());

        let zoom = app_state.zoom_level;
        let cell_width = 320.0f32 * zoom;
        let padding = 4.0f32 * zoom;
        let font_size = 13.0f32 * zoom;

        let available = ui.available_size();
        let panel_min = ui.min_rect().min;

        // Calculate max cell height: 2/3 of available screen height at zoom=1.0
        let max_cell_height = (available.y * 2.0 / 3.0) / zoom * zoom; // Normalize to current zoom

        // Calculate row heights based on content
        let row_heights = calculate_row_heights(&grid, graph, cell_width, max_cell_height, zoom, media_cache);

        // Helper to calculate Y position for a given row
        let row_y_position = |row: i32| -> f32 {
            let mut y = 0.0;
            for r in grid.min_y..row {
                y += row_heights.get(&r).copied().unwrap_or(80.0 * zoom) + padding;
            }
            y
        };

        // Determine which cell is "current" - either the recording placeholder or current_vertex
        // Placeholders are added to grid.positions so the same lookup works for both
        let effective_current_id = app_state.recording_placeholder_id.unwrap_or(current_id);
        let selected_pos = grid.positions.get(&effective_current_id).copied().unwrap_or((0, 0));

        // Calculate where the selected cell would be in grid-local coordinates
        let selected_cell_x = (selected_pos.0 - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
        let selected_row_height = row_heights.get(&selected_pos.1).copied().unwrap_or(80.0 * zoom);
        let selected_cell_y = row_y_position(selected_pos.1) + selected_row_height / 2.0;

        // Calculate offset to center the selected cell in the available space
        let offset_x = available.x / 2.0 - selected_cell_x;
        let offset_y = available.y / 2.0 - selected_cell_y;

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
                let from_cell_y = row_y_position(*y);
                let from_cell_height = row_heights.get(y).copied().unwrap_or(80.0 * zoom);

                // Draw line to south neighbor
                if vertex.edges[EDGE_SOUTH] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_SOUTH]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = row_y_position(ty);

                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width / 2.0, from_cell_y + from_cell_height);
                        let to_edge = base_pos + egui::vec2(to_cell_x + cell_width / 2.0, to_cell_y);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ns));
                    }
                }

                // Draw line to east neighbor
                if vertex.edges[EDGE_EAST] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_EAST]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = row_y_position(ty);
                        let to_cell_height = row_heights.get(&ty).copied().unwrap_or(80.0 * zoom);

                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width, from_cell_y + from_cell_height / 2.0);
                        let to_edge = base_pos + egui::vec2(to_cell_x, to_cell_y + to_cell_height / 2.0);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ew));
                    }
                }
            }
        }

        // Draw cells
        for y in grid.min_y..=grid.max_y {
            let cell_height = row_heights.get(&y).copied().unwrap_or(80.0 * zoom);
            for x in grid.min_x..=grid.max_x {
                if let Some(&vertex_id) = grid.cells.get(&(x, y)) {
                    let cell_x = (x - grid.min_x) as f32 * (cell_width + padding);
                    let cell_y = row_y_position(y);

                    let rect = egui::Rect::from_min_size(
                        base_pos + egui::vec2(cell_x, cell_y),
                        egui::vec2(cell_width, cell_height),
                    );

                    // Same is_current logic for all cells
                    let is_current = vertex_id == effective_current_id;

                    if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        render_vertex_card(&painter, vertex, vertex_id, rect, is_current, zoom, font_size, media_cache, ctx, graph);

                        // Draw direction arrow on current cell
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
                        painter.rect_stroke(rect, corner_radius, egui::Stroke::new(2.0 * zoom, egui::Color32::from_rgb(80, 80, 90)), egui::StrokeKind::Outside);
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
                        action = GridAction::ClickVertex;
                    }
                }
            }
        }

        // Draw placeholder cells for pending audio recordings
        for placeholder in &grid.placeholder_cells {
            let (x, y) = placeholder.position;
            let cell_x = (x - grid.min_x) as f32 * (cell_width + padding);
            let cell_y = row_y_position(y);
            let cell_height = row_heights.get(&y).copied().unwrap_or(80.0 * zoom);

            let rect = egui::Rect::from_min_size(
                base_pos + egui::vec2(cell_x, cell_y),
                egui::vec2(cell_width, cell_height),
            );

            // Same is_current logic for all cells
            let is_current = placeholder.local_id == effective_current_id;

            render_placeholder_cell(&painter, rect, placeholder, zoom, font_size, is_current);

            // Draw direction arrow on current cell (same as regular vertices)
            if is_current {
                draw_direction_arrow(&painter, rect, app_state.last_nav_direction, zoom);
            }
        }

        // Draw portal shadow cells (unloaded portal destinations)
        for shadow in &grid.portal_shadows {
            let (x, y) = shadow.position;
            let cell_x = (x - grid.min_x) as f32 * (cell_width + padding);
            let cell_y = row_y_position(y);
            let cell_height = row_heights.get(&y).copied().unwrap_or(80.0 * zoom);

            let rect = egui::Rect::from_min_size(
                base_pos + egui::vec2(cell_x, cell_y),
                egui::vec2(cell_width, cell_height),
            );

            render_portal_shadow(&painter, rect, shadow, zoom, font_size, ctx);
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

/// Draw a loading spinner overlay on a cell (for portal loading)
/// Render a portal shadow cell (unloaded portal destination)
fn render_portal_shadow(
    painter: &egui::Painter,
    rect: egui::Rect,
    shadow: &PortalShadow,
    zoom: f32,
    _font_size: f32,
    ctx: &egui::Context,
) {
    let corner_radius = 8.0 * zoom;

    if shadow.is_loading {
        // Loading state - animated spinner
        ctx.request_repaint();

        // Slightly brighter background for loading
        painter.rect_filled(
            rect,
            corner_radius,
            egui::Color32::from_rgba_unmultiplied(40, 50, 70, 200),
        );

        // Animated border
        painter.rect_stroke(
            rect,
            corner_radius,
            egui::Stroke::new(2.0 * zoom, egui::Color32::from_rgb(80, 140, 200)),
            egui::StrokeKind::Outside,
        );

        let center = rect.center();
        let radius = 18.0 * zoom;
        let thickness = 3.0 * zoom;

        // Animated spinner
        let time = ctx.input(|i| i.time);
        let angle = (time * 4.0) as f32;

        let segments = 8;
        for i in 0..segments {
            let seg_angle = angle + (i as f32 * std::f32::consts::TAU / segments as f32);
            let alpha = ((i as f32 / segments as f32) * 180.0) as u8 + 75;
            let color = egui::Color32::from_rgba_unmultiplied(100, 160, 255, alpha);

            let inner = center + egui::vec2(seg_angle.cos(), seg_angle.sin()) * (radius - thickness);
            let outer = center + egui::vec2(seg_angle.cos(), seg_angle.sin()) * radius;

            painter.line_segment([inner, outer], egui::Stroke::new(thickness * 0.8, color));
        }

        painter.text(
            egui::pos2(center.x, rect.max.y - 14.0 * zoom),
            egui::Align2::CENTER_CENTER,
            "Loading...",
            egui::FontId::proportional(10.0 * zoom),
            egui::Color32::from_rgb(150, 170, 200),
        );
    } else {
        // Shadow state - dimmed placeholder
        painter.rect_filled(
            rect,
            corner_radius,
            egui::Color32::from_rgba_unmultiplied(30, 35, 45, 150),
        );

        // Dashed-style border (using dotted pattern)
        painter.rect_stroke(
            rect,
            corner_radius,
            egui::Stroke::new(1.5 * zoom, egui::Color32::from_rgba_unmultiplied(80, 90, 110, 150)),
            egui::StrokeKind::Outside,
        );

        // Question mark or ellipsis to indicate unknown content
        let center = rect.center();
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            "?",
            egui::FontId::proportional(24.0 * zoom),
            egui::Color32::from_rgba_unmultiplied(100, 110, 130, 180),
        );
    }
}

/// Render a placeholder cell for a pending audio recording
fn render_placeholder_cell(
    painter: &egui::Painter,
    rect: egui::Rect,
    placeholder: &PlaceholderCell,
    zoom: f32,
    font_size: f32,
    is_current: bool,
) {
    let corner_radius = 8.0 * zoom;
    let is_recording = placeholder.pending_cell.status == PendingAudioStatus::Recording;

    // Background color based on status
    let bg_color = match placeholder.pending_cell.status {
        PendingAudioStatus::Recording => egui::Color32::from_rgba_unmultiplied(80, 40, 40, 230),
        PendingAudioStatus::Encoding => egui::Color32::from_rgba_unmultiplied(60, 60, 80, 220),
        PendingAudioStatus::Uploading => egui::Color32::from_rgba_unmultiplied(60, 80, 60, 220),
        PendingAudioStatus::Transcribing => egui::Color32::from_rgba_unmultiplied(80, 60, 80, 220),
        PendingAudioStatus::Complete => egui::Color32::from_rgba_unmultiplied(60, 80, 80, 220),
    };

    // Border color - current cell gets highlight, recording pulses red
    let elapsed = placeholder.pending_cell.created_at.elapsed().as_secs_f32();
    let pulse = ((elapsed * 3.0).sin() * 0.5 + 0.5) as u8;
    let border_color = if is_recording {
        egui::Color32::from_rgba_unmultiplied(220 + pulse / 8, 100 + pulse / 2, 100, 255)
    } else if is_current {
        egui::Color32::from_rgb(100, 200, 255) // Same highlight as current vertex cells
    } else {
        egui::Color32::from_rgba_unmultiplied(100 + pulse / 2, 150 + pulse / 2, 200, 200)
    };

    // Border thickness - thicker when current (same as vertex cards)
    let border_thickness = if is_current { 3.0 } else { 2.0 };

    // Draw background
    painter.rect_filled(rect, corner_radius, bg_color);
    painter.rect_stroke(
        rect,
        corner_radius,
        egui::Stroke::new(border_thickness * zoom, border_color),
        egui::StrokeKind::Outside,
    );

    if is_recording {
        // Draw live audio level meter for recording
        draw_audio_level_meter(
            painter,
            rect,
            placeholder.pending_cell.current_audio_level,
            zoom,
        );
    } else {
        // Draw waveform visualization for processing states
        let waveform = &placeholder.pending_cell.waveform;
        if !waveform.is_empty() {
            let waveform_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(8.0 * zoom, rect.height() * 0.3),
                egui::vec2(rect.width() - 16.0 * zoom, rect.height() * 0.4),
            );
            draw_waveform(painter, waveform_rect, waveform, zoom, border_color);
        }
    }

    // Draw status text
    let status_text = match placeholder.pending_cell.status {
        PendingAudioStatus::Recording => "🔴 Recording...",
        PendingAudioStatus::Encoding => "Encoding...",
        PendingAudioStatus::Uploading => "Uploading...",
        PendingAudioStatus::Transcribing => "Transcribing...",
        PendingAudioStatus::Complete => "Complete",
    };

    let status_pos = egui::pos2(rect.center().x, rect.max.y - 12.0 * zoom);
    let status_color = if is_recording {
        egui::Color32::from_rgb(255, 150, 150)
    } else {
        egui::Color32::from_rgb(180, 180, 200)
    };
    painter.text(
        status_pos,
        egui::Align2::CENTER_CENTER,
        status_text,
        egui::FontId::proportional(font_size * 0.85),
        status_color,
    );

    // Draw audio icon at top
    let icon_pos = egui::pos2(rect.center().x, rect.min.y + 16.0 * zoom);
    painter.text(
        icon_pos,
        egui::Align2::CENTER_CENTER,
        "🎤",
        egui::FontId::proportional(font_size * 1.2),
        egui::Color32::WHITE,
    );
}

/// Draw a live audio level meter (vertical bar that responds to microphone input)
fn draw_audio_level_meter(
    painter: &egui::Painter,
    rect: egui::Rect,
    level: f32,
    zoom: f32,
) {
    let meter_width = 30.0 * zoom;
    let meter_height = rect.height() * 0.5;
    let meter_rect = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.center().y - 5.0 * zoom),
        egui::vec2(meter_width, meter_height),
    );

    // Background
    painter.rect_filled(
        meter_rect,
        4.0 * zoom,
        egui::Color32::from_rgb(40, 40, 50),
    );

    // Level bar (grows from bottom)
    let level_height = meter_height * level.clamp(0.0, 1.0);
    if level_height > 0.0 {
        let level_rect = egui::Rect::from_min_max(
            egui::pos2(meter_rect.min.x, meter_rect.max.y - level_height),
            meter_rect.max,
        );

        // Color gradient based on level (green -> yellow -> red)
        let color = if level < 0.5 {
            egui::Color32::from_rgb(80, 200, 80)
        } else if level < 0.8 {
            egui::Color32::from_rgb(200, 200, 80)
        } else {
            egui::Color32::from_rgb(200, 80, 80)
        };

        painter.rect_filled(level_rect, 4.0 * zoom, color);
    }

    // Border
    painter.rect_stroke(
        meter_rect,
        4.0 * zoom,
        egui::Stroke::new(1.0 * zoom, egui::Color32::from_rgb(100, 100, 120)),
        egui::StrokeKind::Outside,
    );
}

/// Draw a waveform visualization from amplitude data
fn draw_waveform(
    painter: &egui::Painter,
    rect: egui::Rect,
    waveform: &[f32],
    zoom: f32,
    color: egui::Color32,
) {
    if waveform.is_empty() {
        return;
    }

    let bar_width = rect.width() / waveform.len() as f32;
    let center_y = rect.center().y;
    let max_height = rect.height() / 2.0;

    // Find max amplitude for normalization
    let max_amp = waveform.iter().cloned().fold(0.0f32, f32::max).max(0.01);

    for (i, &amp) in waveform.iter().enumerate() {
        let x = rect.min.x + (i as f32 + 0.5) * bar_width;
        let normalized = (amp / max_amp).min(1.0);
        let height = normalized * max_height;

        // Draw symmetric bar around center
        let bar_rect = egui::Rect::from_center_size(
            egui::pos2(x, center_y),
            egui::vec2(bar_width * 0.7, height * 2.0),
        );

        painter.rect_filled(bar_rect, 1.0 * zoom, color);
    }
}
