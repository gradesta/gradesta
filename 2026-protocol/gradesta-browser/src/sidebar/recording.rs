//! Audio recording mode (blocking)

use bevy_egui::egui;
use std::time::Duration;

use super::Direction;

/// Actions returned from the recording UI
pub enum RecordingAction {
    None,
    Cancel,
}

/// Render the recording indicator
///
/// This is shown in the sidebar during audio recording.
/// The rest of the UI should be greyed out (blocking mode).
///
/// # Arguments
/// * `ui` - egui UI context
/// * `direction` - Direction the recording will create a new vertex in
/// * `elapsed` - Time elapsed since recording started
/// * `audio_level` - Current audio level (0.0 - 1.0)
///
/// # Returns
/// Action to take (cancel, or none)
pub fn render_recording(
    ui: &mut egui::Ui,
    direction: &Direction,
    elapsed: Duration,
    audio_level: f32,
) -> RecordingAction {
    let mut action = RecordingAction::None;

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.heading("Recording Audio");
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.heading("🔴");
            ui.heading(format!("{:.1}s", elapsed.as_secs_f32()));
        });

        ui.add_space(10.0);
        ui.label(format!(
            "Creating note to the {} {}",
            direction.name(),
            direction.arrow()
        ));

        ui.add_space(10.0);
        ui.label("Release Space to save");

        ui.add_space(10.0);

        // Audio level indicator
        ui.add(egui::ProgressBar::new(audio_level.min(1.0)));

        ui.add_space(20.0);

        if ui.button("Cancel (Escape)").clicked() {
            action = RecordingAction::Cancel;
        }
    });

    action
}
