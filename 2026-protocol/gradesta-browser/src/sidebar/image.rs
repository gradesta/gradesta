//! Image viewing mode - displays images in the sidebar

use bevy_egui::egui;

/// Render image content
///
/// # Arguments
/// * `ui` - egui UI context
/// * `texture` - The texture handle for the image (if loaded)
/// * `fullscreen` - Whether to render in fullscreen mode
pub fn render_image_view(
    ui: &mut egui::Ui,
    texture: Option<&egui::TextureHandle>,
    fullscreen: bool,
) {
    if !fullscreen {
        ui.heading("Image");
        ui.separator();
    }

    if let Some(tex) = texture {
        let size = tex.size_vec2();
        let available = ui.available_size();

        // Scale to fit available space while maintaining aspect ratio
        let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
        let display_size = size * scale;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.image((tex.id(), display_size));
                });
            });

        if !fullscreen {
            ui.label(format!("{}x{}", size.x as u32, size.y as u32));
        }
    } else {
        ui.label("Loading image...");
        ui.spinner();
    }
}
