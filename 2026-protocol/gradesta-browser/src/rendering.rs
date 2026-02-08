//! Vertex rendering functions.
//!
//! This module contains functions for rendering vertex content and cards.

use std::time::Instant;

use bevy_egui::egui;

use crate::audio::{extract_transcript, play_audio, AudioPlaybackState};
use crate::graph::{compute_stack, GraphState, Vertex};
use crate::media::{
    draw_waveform, get_or_generate_waveform, get_or_load_animated_gif, get_or_load_texture,
    is_image_data, open_with_external, MediaCache,
};

/// Render vertex content in the sidebar panel
pub fn render_vertex_content(
    ui: &mut egui::Ui,
    vertex: &Vertex,
    vertex_id: u64,
    media_cache: &mut MediaCache,
    ctx: &egui::Context,
    playback_state: &AudioPlaybackState,
) {
    let label = String::from_utf8_lossy(&vertex.label);

    if let Some(mime) = &vertex.mime {
        // Show type and size for all content
        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.monospace(mime);
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.label(format!("{} bytes", vertex.label.len()));
        });

        ui.separator();

        if mime == "text/gradesta-url" {
            // Portal to another landmark - should be auto-loading
            ui.heading("🌀 Loading...");
            let url = String::from_utf8_lossy(&vertex.label);
            ui.spinner();
            ui.add_space(8.0);
            let url_display: String = url.chars().take(60).collect();
            ui.label(format!("Loading: {}", url_display));

        } else if mime == "text/x-url" {
            // External URL - file too large to inline, open externally
            ui.heading("📎 External Content");
            ui.add_space(8.0);
            let url = String::from_utf8_lossy(&vertex.label);
            ui.label("This file is too large to display inline.");
            ui.add_space(4.0);
            ui.monospace(&*url);
            ui.add_space(8.0);
            if ui.button("🔗 Open with system handler").clicked() {
                let url_str = url.to_string();
                let _ = std::process::Command::new("xdg-open")
                    .arg(&url_str)
                    .spawn();
            }

        } else if mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url" {
            // Text content - show in constrained scroll area
            ui.heading("📄 Text Content");
            ui.add_space(4.0);
            ui.label("Ctrl+Enter to open in modal");
            ui.add_space(4.0);

            let text = String::from_utf8_lossy(&vertex.label);
            let available_height = ui.available_height().min(400.0).max(100.0);

            egui::ScrollArea::vertical()
                .max_height(available_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut text.to_string())
                            .desired_width(ui.available_width())
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                    );
                });

        } else if mime.starts_with("image/") || is_image_data(&vertex.label) {
            // Image content
            ui.heading("🖼 Image");
            ui.add_space(4.0);
            ui.label("Ctrl+Enter to view full size");
            ui.add_space(4.0);

            let is_gif = mime == "image/gif" || (vertex.label.len() >= 6 && &vertex.label[0..6] == b"GIF89a" || vertex.label.len() >= 6 && &vertex.label[0..6] == b"GIF87a");

            if is_gif {
                // Animated GIF
                if let Some(animated) = get_or_load_animated_gif(vertex_id, &vertex.label, media_cache, ctx) {
                    // Update animation
                    let now = Instant::now();
                    if now.duration_since(animated.last_switch) >= animated.delays[animated.current_frame] {
                        animated.current_frame = (animated.current_frame + 1) % animated.frames.len();
                        animated.last_switch = now;
                    }

                    let tex = &animated.frames[animated.current_frame];
                    let size = tex.size_vec2();
                    let max_size = egui::vec2(380.0, 400.0);
                    let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                    ui.image((tex.id(), size * scale));

                    ui.label(format!("Frame {}/{}", animated.current_frame + 1, animated.frames.len()));
                    ctx.request_repaint(); // Keep animating
                }
            } else {
                // Static image
                match get_or_load_texture(vertex_id, &vertex.label, mime, media_cache, ctx) {
                    Some(tex) => {
                        let size = tex.size_vec2();
                        let max_size = egui::vec2(380.0, 400.0);
                        let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                        ui.image((tex.id(), size * scale));
                        ui.label(format!("{}x{}", size.x as u32, size.y as u32));
                    }
                    None => {
                        ui.label("Failed to decode image");
                    }
                }
            }

        } else if mime.starts_with("audio/") {
            // Audio content - inline playback
            ui.heading("🔊 Audio");
            ui.add_space(4.0);
            ui.label(format!("Type: {}", mime));

            // Show file size
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(8.0);

            // Play button
            if ui.button("▶ Play").clicked() {
                play_audio(&vertex.label, mime, vertex_id, playback_state);
            }

            // Check for transcript
            let transcript = vertex.layers.values()
                .find(|layer| layer.mime == "text/plain")
                .and_then(|layer| String::from_utf8(layer.data.clone()).ok())
                .or_else(|| extract_transcript(&vertex.label, mime));

            if let Some(transcript) = transcript {
                ui.add_space(8.0);
                ui.separator();
                ui.heading("📝 Transcript");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.label(&transcript);
                    });
            }

            ui.add_space(8.0);
            if ui.button("📂 Open with external player").clicked() {
                open_with_external(&vertex.label, mime);
            }

        } else if mime.starts_with("video/") {
            // Video - offer to open externally
            ui.heading("🎬 Video");
            ui.add_space(8.0);
            ui.label("Video playback requires external player.");
            ui.label(format!("Type: {}", mime));
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(16.0);

            if ui.button("📂 Open with external player").clicked() {
                open_with_external(&vertex.label, mime);
            }

        } else {
            // Other binary content
            ui.heading("📦 Binary Content");
            ui.add_space(8.0);
            ui.label(format!("Type: {}", mime));
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(16.0);

            if ui.button("📂 Open with system default").clicked() {
                open_with_external(&vertex.label, mime);
            }

            // Show hex preview
            ui.add_space(16.0);
            ui.label("Hex preview:");
            let hex_preview: String = vertex.label.iter()
                .take(64)
                .map(|b| format!("{:02x} ", b))
                .collect();
            ui.monospace(&hex_preview);
            if vertex.label.len() > 64 {
                ui.label("...");
            }
        }
    } else {
        ui.label("No content type specified");
        ui.add_space(8.0);
        ui.label(&*label);
    }
}

/// Render a vertex as a card preview in the given rect
/// This uses the exact same rendering logic as the main grid cells
pub fn render_vertex_card(
    painter: &egui::Painter,
    vertex: &Vertex,
    vertex_id: u64,
    rect: egui::Rect,
    is_current: bool,
    zoom: f32,
    font_size: f32,
    media_cache: &mut MediaCache,
    ctx: &egui::Context,
    graph: &GraphState,
) {
    let mime = vertex.mime.as_deref().unwrap_or("");
    let primary_is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
    let primary_is_audio = mime.starts_with("audio/");

    // Collect all content types present
    let mut has_image = primary_is_image;
    let mut has_audio = primary_is_audio;
    let mut has_text = mime.starts_with("text/") && !mime.contains("gradesta-url");
    let mut text_content: Option<String> = None;

    // Check additional layers for images, audio, text
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
    if mime.starts_with("text/") && !mime.contains("gradesta-url") && text_content.is_none() {
        text_content = String::from_utf8(vertex.label.clone()).ok();
    }

    // Different colors based on content type (same as grid cells)
    let (bg_color, border_color) = if is_current {
        (egui::Color32::from_rgb(50, 100, 70), egui::Color32::from_rgb(100, 200, 120))
    } else if mime == "text/gradesta-url" {
        (egui::Color32::from_rgb(60, 60, 90), egui::Color32::from_rgb(100, 100, 150))
    } else if has_image {
        (egui::Color32::from_rgb(40, 40, 45), egui::Color32::from_rgb(140, 100, 140))
    } else if has_audio {
        (egui::Color32::from_rgb(70, 60, 50), egui::Color32::from_rgb(140, 120, 100))
    } else if has_text {
        (egui::Color32::from_rgb(50, 60, 70), egui::Color32::from_rgb(100, 120, 140))
    } else {
        (egui::Color32::from_rgb(50, 50, 55), egui::Color32::from_rgb(80, 80, 90))
    };

    let corner_radius = 4.0 * zoom;

    // Check for stacked cards (up/down connections)
    let stack = compute_stack(graph, vertex_id);
    let has_stack = stack.total > 1;

    // Draw stacked card shadows behind the main card
    if has_stack {
        let shadow_color = egui::Color32::from_rgba_unmultiplied(30, 30, 35, 180);
        let cards_to_show = (stack.total - stack.current_index - 1).min(3); // Cards below current
        for i in (1..=cards_to_show).rev() {
            let offset = i as f32 * 4.0 * zoom;
            let shadow_rect = rect.translate(egui::vec2(offset, offset));
            painter.rect_filled(shadow_rect, corner_radius, shadow_color);
            painter.rect_stroke(shadow_rect, corner_radius, egui::Stroke::new(1.0 * zoom, egui::Color32::from_rgb(60, 60, 65)));
        }
        // Also draw cards above (offset in opposite direction)
        let cards_above = stack.current_index.min(3);
        for i in (1..=cards_above).rev() {
            let offset = i as f32 * 4.0 * zoom;
            let shadow_rect = rect.translate(egui::vec2(-offset, -offset));
            painter.rect_filled(shadow_rect, corner_radius, shadow_color);
            painter.rect_stroke(shadow_rect, corner_radius, egui::Stroke::new(1.0 * zoom, egui::Color32::from_rgb(60, 60, 65)));
        }
    }

    painter.rect_filled(rect, corner_radius, bg_color);
    painter.rect_stroke(rect, corner_radius, egui::Stroke::new(if is_current { 3.0 * zoom } else { 2.0 * zoom }, border_color));

    // Draw stack position badge if this vertex is part of a stack
    if has_stack {
        let badge_text = format!("{}/{}", stack.current_index + 1, stack.total);
        let badge_font = egui::FontId::proportional(font_size * 0.6);
        let badge_pos = egui::pos2(rect.right() - 4.0 * zoom, rect.top() + 4.0 * zoom);
        // Draw badge background
        let badge_rect = egui::Rect::from_center_size(
            badge_pos + egui::vec2(-12.0 * zoom, 6.0 * zoom),
            egui::vec2(28.0 * zoom, 14.0 * zoom),
        );
        painter.rect_filled(badge_rect, 3.0 * zoom, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
        painter.text(
            badge_pos,
            egui::Align2::RIGHT_TOP,
            badge_text,
            badge_font,
            egui::Color32::from_rgb(200, 200, 255),
        );
    }

    // Calculate layout sections based on what content we have
    let inner_rect = rect.shrink(4.0 * zoom);
    let num_sections = (has_image as usize) + (has_audio as usize) + (has_text as usize);
    let section_height = if num_sections > 0 { inner_rect.height() / num_sections as f32 } else { inner_rect.height() };

    let mut y_offset = 0.0;

    // Render image section
    if has_image {
        let section_rect = egui::Rect::from_min_size(
            inner_rect.min + egui::vec2(0.0, y_offset),
            egui::vec2(inner_rect.width(), section_height),
        );

        // Try primary layer first, then additional layers
        let (img_data, img_mime) = if primary_is_image {
            (&vertex.label, mime)
        } else {
            vertex.layers.values()
                .find(|l| l.mime.starts_with("image/") || is_image_data(&l.data))
                .map(|l| (&l.data, l.mime.as_str()))
                .unwrap_or((&vertex.label, mime))
        };

        if let Some(tex) = get_or_load_texture(vertex_id, img_data, img_mime, media_cache, ctx) {
            let tex_size = tex.size_vec2();
            let scale = (section_rect.width() / tex_size.x).min(section_rect.height() / tex_size.y);
            let scaled_size = tex_size * scale;
            let img_rect = egui::Rect::from_center_size(section_rect.center(), scaled_size);
            painter.image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
        }
        y_offset += section_height;
    }

    // Render audio waveform section
    if has_audio {
        let section_rect = egui::Rect::from_min_size(
            inner_rect.min + egui::vec2(0.0, y_offset),
            egui::vec2(inner_rect.width(), section_height),
        ).shrink(2.0 * zoom);

        let num_bars = (section_rect.width() / (3.0 * zoom)) as usize;
        let num_bars = num_bars.max(10).min(64);

        // Get audio data from primary or layers
        let audio_data = if primary_is_audio {
            &vertex.label
        } else {
            vertex.layers.values()
                .find(|l| l.mime.starts_with("audio/"))
                .map(|l| &l.data)
                .unwrap_or(&vertex.label)
        };

        if let Some(waveform) = get_or_generate_waveform(vertex_id, audio_data, media_cache, num_bars) {
            let waveform_color = if is_current {
                egui::Color32::from_rgb(100, 200, 255)
            } else {
                egui::Color32::from_rgb(140, 120, 100)
            };
            draw_waveform(painter, section_rect, waveform, waveform_color, egui::Color32::TRANSPARENT);
        }

        // Draw small speaker icon
        painter.text(
            egui::pos2(section_rect.left() + 2.0 * zoom, section_rect.top() + 2.0 * zoom),
            egui::Align2::LEFT_TOP,
            "🔊",
            egui::FontId::proportional(font_size * 0.6),
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 150),
        );
        y_offset += section_height;
    }

    // Render text section
    if has_text {
        let section_rect = egui::Rect::from_min_size(
            inner_rect.min + egui::vec2(0.0, y_offset),
            egui::vec2(inner_rect.width(), section_height),
        );

        if let Some(ref text) = text_content {
            let text_font_size = font_size * 0.9;
            let char_width = text_font_size * 0.5;
            let line_height = text_font_size * 1.2;
            let chars_per_line = ((section_rect.width() - 4.0 * zoom) / char_width) as usize;
            let chars_per_line = chars_per_line.max(5);
            let max_lines = ((section_rect.height() - 4.0 * zoom) / line_height) as usize;
            let max_lines = max_lines.max(1);

            // Word wrap the text, up to tweet length (280 chars) before truncating
            let max_chars = 280.min(chars_per_line * max_lines);
            let text_to_wrap: String = text.chars().take(max_chars).collect();
            let needs_ellipsis = text.chars().count() > max_chars;

            // Simple word wrap
            let mut lines: Vec<String> = Vec::new();
            let mut current_line = String::new();

            for word in text_to_wrap.split_whitespace() {
                if current_line.is_empty() {
                    current_line = word.to_string();
                } else if current_line.chars().count() + 1 + word.chars().count() <= chars_per_line {
                    current_line.push(' ');
                    current_line.push_str(word);
                } else {
                    lines.push(current_line);
                    current_line = word.to_string();
                    if lines.len() >= max_lines {
                        break;
                    }
                }
            }
            if !current_line.is_empty() && lines.len() < max_lines {
                lines.push(current_line);
            }

            // Add ellipsis to last line if truncated
            if needs_ellipsis && !lines.is_empty() {
                let last = lines.last_mut().unwrap();
                if last.chars().count() + 1 <= chars_per_line {
                    last.push('…');
                } else {
                    // Truncate last word to make room
                    let truncated: String = last.chars().take(chars_per_line - 1).collect();
                    *last = format!("{}…", truncated);
                }
            }

            // Draw each line
            let total_text_height = lines.len() as f32 * line_height;
            let start_y = section_rect.center().y - total_text_height / 2.0 + line_height / 2.0;

            for (i, line) in lines.iter().enumerate() {
                painter.text(
                    egui::pos2(section_rect.center().x, start_y + i as f32 * line_height),
                    egui::Align2::CENTER_CENTER,
                    line,
                    egui::FontId::proportional(text_font_size),
                    egui::Color32::WHITE,
                );
            }
        }
    }

    // If no content at all, show placeholder
    if !has_image && !has_audio && !has_text {
        let icon = if mime == "text/gradesta-url" {
            "🌀"
        } else if mime == "text/x-url" {
            "📎"
        } else if mime.starts_with("video/") {
            "🎬"
        } else if !mime.is_empty() {
            "📦"
        } else {
            "◻"
        };

        // For portals, also show destination
        let display = if mime == "text/gradesta-url" {
            let url = String::from_utf8_lossy(&vertex.label);
            let short_url: String = url.chars().take(20).collect();
            format!("{} {}", icon, if url.len() > 20 { format!("{}…", short_url) } else { short_url })
        } else {
            icon.to_string()
        };

        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            display,
            egui::FontId::proportional(font_size),
            egui::Color32::WHITE,
        );
    }
}
