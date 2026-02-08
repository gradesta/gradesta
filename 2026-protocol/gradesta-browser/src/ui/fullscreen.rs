//! Fullscreen content rendering for UI system
//!
//! Renders content in fullscreen mode (text, image, video, preview).

use std::time::{Duration, Instant};

use bevy_egui::egui;

use crate::audio::AudioPlaybackState;
use crate::graph::GraphState;
use crate::media::{get_or_load_animated_gif, get_or_load_texture, MediaCache};
use crate::rendering::render_vertex_content;
use crate::state::AppState;

/// Action from fullscreen rendering
#[derive(Clone, Debug, PartialEq)]
pub enum FullscreenAction {
    None,
    ExitFullscreen,
}

/// Render fullscreen content
///
/// Returns true if fullscreen mode should continue, false if it should exit.
/// Also returns an action if the user clicked the exit button.
pub fn render_fullscreen_content(
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
    playback_state: &AudioPlaybackState,
) -> FullscreenAction {
    if !app_state.sidebar.fullscreen {
        return FullscreenAction::None;
    }

    let mut action = FullscreenAction::None;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Add a minimal toolbar at the top for closing
        ui.horizontal(|ui| {
            if ui.button("✕ Exit Fullscreen (Esc)").clicked() {
                action = FullscreenAction::ExitFullscreen;
            }
            ui.separator();
            ui.label("Ctrl+Enter to toggle");
        });
        ui.separator();

        // Render the appropriate content based on what modal is active
        if app_state.show_text_modal {
            render_fullscreen_text(ui, &app_state.text_modal_content);
        } else if app_state.show_image_modal {
            render_fullscreen_image(ui, ctx, app_state, graph, media_cache);
        } else if app_state.show_video_modal {
            render_fullscreen_video(ui, ctx, app_state, media_cache);
        } else {
            // No specific modal - show preview content in fullscreen
            if let Some(current_id) = app_state.current_vertex {
                if let Some(vertex) = graph.vertices.get(&current_id) {
                    render_vertex_content(ui, vertex, current_id, media_cache, ctx, playback_state);
                }
            } else {
                ui.label("No content to display");
            }
        }
    });

    action
}

fn render_fullscreen_text(ui: &mut egui::Ui, content: &str) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut content.to_string())
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false)
            );
        });
}

fn render_fullscreen_image(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
) {
    if let Some(vertex_id) = app_state.image_modal_vertex_id {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Prefer layer 2 content (full image) over layer 0 (thumbnail/label)
            let (image_data, mime): (&[u8], &str) = if let Some(layer2) = vertex.layers.get(&2) {
                (&layer2.data, &layer2.mime)
            } else {
                (&vertex.label, vertex.mime.as_deref().unwrap_or(""))
            };

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if mime == "image/gif" {
                        if let Some(animated) = get_or_load_animated_gif(vertex_id, image_data, media_cache, ctx) {
                            let now = Instant::now();
                            if now.duration_since(animated.last_switch) >= animated.delays[animated.current_frame] {
                                animated.current_frame = (animated.current_frame + 1) % animated.frames.len();
                                animated.last_switch = now;
                            }
                            let tex = &animated.frames[animated.current_frame];
                            let size = tex.size_vec2();
                            let available = ui.available_size();
                            let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                            ui.vertical_centered(|ui| {
                                ui.image((tex.id(), size * scale));
                            });
                            ctx.request_repaint();
                        }
                    } else {
                        if let Some(tex) = get_or_load_texture(vertex_id, image_data, mime, media_cache, ctx) {
                            let size = tex.size_vec2();
                            let available = ui.available_size();
                            let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                            ui.vertical_centered(|ui| {
                                ui.image((tex.id(), size * scale));
                            });
                        }
                    }
                });
        }
    }
}

fn render_fullscreen_video(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &AppState,
    media_cache: &mut MediaCache,
) {
    if let Some(vertex_id) = app_state.video_modal_vertex_id {
        // Update video texture from decoded frames
        let (latest_frame, is_playing, position, duration) = {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                let mut latest = None;
                while let Ok(frame) = player.frame_rx.try_recv() {
                    latest = Some(frame);
                }
                (latest, player.is_playing(), player.get_position(), player.duration)
            } else {
                (None, false, Duration::ZERO, Duration::ZERO)
            }
        };

        // Update texture
        if let Some(frame) = latest_frame {
            if frame.width > 0 && frame.height > 0 && !frame.rgba.is_empty() {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.rgba,
                );
                let handle = ctx.load_texture(
                    format!("video_{}", vertex_id),
                    image,
                    egui::TextureOptions::default(),
                );
                media_cache.video_textures.insert(vertex_id, handle);
            }
        }

        if is_playing {
            ctx.request_repaint();
        }

        // Render video player in fullscreen
        let video_width = if let Some(tex) = media_cache.video_textures.get(&vertex_id) {
            let tex_size = tex.size_vec2();
            let available = ui.available_size() - egui::vec2(0.0, 60.0);
            let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
            let display_size = tex_size * scale;
            ui.vertical_centered(|ui| {
                ui.image((tex.id(), display_size));
            });
            display_size.x
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Loading video...");
                ui.spinner();
                ui.add_space(100.0);
            });
            ui.available_width()
        };

        ui.add_space(8.0);

        // Seek bar
        let dur_secs = duration.as_secs_f32().max(0.1);
        let mut pos_secs = position.as_secs_f32();
        let available_width = ui.available_width();
        let left_padding = ((available_width - video_width) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(left_padding);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(video_width, 20.0),
                egui::Sense::click_and_drag()
            );

            if ui.is_rect_visible(rect) {
                let progress = pos_secs / dur_secs;
                let filled_width = rect.width() * progress;

                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(60));
                let filled_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(filled_width, rect.height())
                );
                ui.painter().rect_filled(filled_rect, 4.0, egui::Color32::from_rgb(100, 150, 255));

                if response.dragged() || response.clicked() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let relative_x = (pointer_pos.x - rect.left()) / rect.width();
                        let new_pos = (relative_x.clamp(0.0, 1.0) * dur_secs) as f32;
                        if (new_pos - pos_secs).abs() > 0.1 {
                            pos_secs = new_pos;
                        }
                    }
                }

                let indicator_x = rect.left() + filled_width;
                let indicator_center = egui::pos2(indicator_x, rect.center().y);
                ui.painter().circle_filled(indicator_center, 8.0, egui::Color32::WHITE);
            }
        });

        if pos_secs != position.as_secs_f32() {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                player.seek(Duration::from_secs_f32(pos_secs));
            }
        }

        // Controls
        ui.horizontal(|ui| {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                if player.is_playing() {
                    if ui.button("⏸ Pause").clicked() {
                        player.pause();
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        player.play();
                    }
                }
                ui.label(format!(
                    "{:02}:{:02} / {:02}:{:02}",
                    position.as_secs() / 60,
                    position.as_secs() % 60,
                    duration.as_secs() / 60,
                    duration.as_secs() % 60
                ));
            }
        });
    }
}
