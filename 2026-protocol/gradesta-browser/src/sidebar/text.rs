//! Text viewing mode - displays text content in the sidebar

use bevy_egui::egui;

/// Render text content (viewing, not editing)
///
/// # Arguments
/// * `ui` - egui UI context
/// * `content` - The text content to display
/// * `fullscreen` - Whether to render in fullscreen mode
pub fn render_text_view(
    ui: &mut egui::Ui,
    content: &str,
    fullscreen: bool,
) {
    if !fullscreen {
        ui.heading("Text Content");
        ui.separator();
    }

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut content.to_string())
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false),
            );
        });
}
