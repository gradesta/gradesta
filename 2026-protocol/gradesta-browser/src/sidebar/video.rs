//! Video playback mode - displays video player in the sidebar

use bevy_egui::egui;
use std::time::Duration;

/// Video control actions returned from the UI
pub enum VideoAction {
    None,
    Play,
    Pause,
    Seek(Duration),
    Close,
}

/// Render video player
///
/// # Arguments
/// * `ui` - egui UI context
/// * `texture` - The current video frame texture (if available)
/// * `is_playing` - Whether the video is currently playing
/// * `position` - Current playback position
/// * `duration` - Total video duration
/// * `fullscreen` - Whether to render in fullscreen mode
///
/// # Returns
/// The action to take (play, pause, seek, etc.)
pub fn render_video(
    ui: &mut egui::Ui,
    texture: Option<&egui::TextureHandle>,
    is_playing: bool,
    position: Duration,
    duration: Duration,
    fullscreen: bool,
) -> VideoAction {
    let mut action = VideoAction::None;

    // Display video frame
    let video_width = if let Some(tex) = texture {
        let tex_size = tex.size_vec2();
        let available = ui.available_size() - egui::vec2(0.0, 60.0); // Reserve space for controls
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

    // Seek bar (custom drawn to match video width)
    let dur_secs = duration.as_secs_f32().max(0.1);
    let pos_secs = position.as_secs_f32();

    // Center the seek bar
    let available_width = ui.available_width();
    let left_padding = ((available_width - video_width) / 2.0).max(0.0);

    ui.horizontal(|ui| {
        ui.add_space(left_padding);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(video_width, 20.0),
            egui::Sense::click_and_drag(),
        );

        if ui.is_rect_visible(rect) {
            let progress = pos_secs / dur_secs;
            let filled_width = rect.width() * progress;

            // Background
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(60));

            // Filled portion
            let filled_rect =
                egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
            ui.painter()
                .rect_filled(filled_rect, 4.0, egui::Color32::from_rgb(100, 150, 255));

            // Handle dragging
            if response.dragged() || response.clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    let relative_x = (pointer_pos.x - rect.left()) / rect.width();
                    let new_pos_secs = relative_x.clamp(0.0, 1.0) * dur_secs;
                    if (new_pos_secs - pos_secs).abs() > 0.1 {
                        action = VideoAction::Seek(Duration::from_secs_f32(new_pos_secs));
                    }
                }
            }

            // Position indicator
            let indicator_x = rect.left() + filled_width;
            let indicator_center = egui::pos2(indicator_x, rect.center().y);
            ui.painter()
                .circle_filled(indicator_center, 8.0, egui::Color32::WHITE);
        }
    });

    // Controls bar
    ui.horizontal(|ui| {
        // Play/Pause button
        if is_playing {
            if ui.button("⏸ Pause").clicked() {
                action = VideoAction::Pause;
            }
        } else {
            if ui.button("▶ Play").clicked() {
                action = VideoAction::Play;
            }
        }

        // Time display
        ui.label(format!(
            "{:02}:{:02} / {:02}:{:02}",
            position.as_secs() / 60,
            position.as_secs() % 60,
            duration.as_secs() / 60,
            duration.as_secs() % 60
        ));

        if !fullscreen {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Close").clicked() {
                    action = VideoAction::Close;
                }
            });
        }
    });

    action
}
