//! Export panel sidebar for HTML export direction selection

use bevy_egui::egui;

use crate::export::ExportState;
use crate::state::Direction;

/// Action returned from the export panel
#[derive(Clone, Debug, PartialEq)]
pub enum ExportAction {
    None,
    ToggleDirection(Direction),
    Confirm,
    Cancel,
}

/// Render the export panel sidebar
pub fn render_export_panel(
    ui: &mut egui::Ui,
    export_state: &ExportState,
) -> ExportAction {
    let mut action = ExportAction::None;

    ui.heading("Export to HTML");
    ui.add_space(10.0);

    ui.label("Select directions to include in export:");
    ui.label("(from current vertex)");
    ui.add_space(10.0);

    // Direction toggles in a grid layout
    egui::Grid::new("export_directions")
        .num_columns(2)
        .spacing([20.0, 10.0])
        .show(ui, |ui| {
            // Cardinal directions
            for dir in Direction::cardinal() {
                let enabled = export_state.is_direction_enabled(*dir);
                let label = format!("{} {}", dir.arrow(), dir.name());
                if ui.checkbox(&mut enabled.clone(), label).changed() {
                    action = ExportAction::ToggleDirection(*dir);
                }
                if *dir == Direction::East || *dir == Direction::South {
                    ui.end_row();
                }
            }
        });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(5.0);

    // Stack directions
    ui.label("Stack directions:");
    ui.horizontal(|ui| {
        let up_enabled = export_state.is_direction_enabled(Direction::Up);
        if ui.checkbox(&mut up_enabled.clone(), format!("{} Up", Direction::Up.arrow())).changed() {
            action = ExportAction::ToggleDirection(Direction::Up);
        }
        let down_enabled = export_state.is_direction_enabled(Direction::Down);
        if ui.checkbox(&mut down_enabled.clone(), format!("{} Down", Direction::Down.arrow())).changed() {
            action = ExportAction::ToggleDirection(Direction::Down);
        }
    });

    ui.add_space(20.0);

    // Summary
    let dir_count = export_state.directions.len();
    if dir_count == 0 {
        ui.colored_label(egui::Color32::YELLOW, "Select at least one direction");
    } else {
        let dirs: Vec<&str> = export_state.directions.iter()
            .map(|d| d.name())
            .collect();
        ui.label(format!("Will export: {}", dirs.join(", ")));
    }

    ui.add_space(20.0);

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("Export HTML").clicked() && dir_count > 0 {
            action = ExportAction::Confirm;
        }
        if ui.button("Cancel").clicked() {
            action = ExportAction::Cancel;
        }
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(5.0);

    // Keyboard hints
    ui.small("Keyboard shortcuts:");
    ui.small("E/W/N/S/U/D - Toggle directions");
    ui.small("Enter - Export | Escape - Cancel");

    action
}
