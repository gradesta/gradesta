//! Text input mode for creating/editing text vertices

use bevy_egui::egui;

use super::Direction;

/// Actions returned from the text input UI
pub enum TextInputAction {
    None,
    Submit(String),
    Cancel,
}

/// Render the text input panel
///
/// # Arguments
/// * `ui` - egui UI context
/// * `direction` - Direction to create new vertex (None = editing current)
/// * `content` - Mutable reference to the text content
///
/// # Returns
/// Action to take
pub fn render_text_input(
    ui: &mut egui::Ui,
    direction: Option<&Direction>,
    content: &mut String,
) -> TextInputAction {
    let mut action = TextInputAction::None;

    let title = if direction.is_some() {
        "New Text Note"
    } else {
        "Edit Text"
    };

    ui.heading(title);
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Save (Ctrl+Enter)").clicked() {
            action = TextInputAction::Submit(content.clone());
        }
        if ui.button("Cancel (Escape)").clicked() {
            action = TextInputAction::Cancel;
        }
    });

    if let Some(dir) = direction {
        ui.label(format!(
            "Creating new note to the {} {}",
            dir.name(),
            dir.arrow()
        ));
    } else {
        ui.label("Editing current vertex");
    }

    ui.separator();

    // Text input area
    egui::ScrollArea::vertical()
        .max_height(ui.available_height() - 20.0)
        .show(ui, |ui| {
            let response = ui.add(
                egui::TextEdit::multiline(content)
                    .desired_width(f32::INFINITY)
                    .desired_rows(10)
                    .font(egui::TextStyle::Monospace),
            );
            // Request focus on the text input
            response.request_focus();
        });

    action
}
