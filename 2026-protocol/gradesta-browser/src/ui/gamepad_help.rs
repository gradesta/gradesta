//! Gamepad help overlay - shows PS2 controller layout with keybindings

use bevy_egui::egui;

/// Render the gamepad help overlay
pub fn render_gamepad_help_overlay(ctx: &egui::Context) {
    egui::Area::new(egui::Id::new("gamepad_help_overlay"))
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(egui::Color32::from_rgba_unmultiplied(20, 20, 30, 240))
                .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237)))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.heading("Gamepad Controls (Ctrl+G or L3 to close)");
                    ui.add_space(10.0);

                    // Use monospace font for the diagram
                    let mono_style = egui::TextStyle::Monospace;

                    ui.horizontal(|ui| {
                        // Left side - shoulders and d-pad
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("     SHOULDER BUTTONS").strong());
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(
"  [L2 Yank]               [R2 RECORD]
  [L1 Up]                  [R1 Down]"
                            ).text_style(mono_style.clone()));

                            ui.add_space(15.0);
                            ui.label(egui::RichText::new("        D-PAD / LEFT STICK").strong());
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(
"                 North
                   ^
                   |
      West  <------+------> East
                   |
                   v
                 South"
                            ).text_style(mono_style.clone()));
                        });

                        ui.add_space(40.0);

                        // Right side - face buttons and special
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("      FACE BUTTONS").strong());
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(
"              [/\\] New Text

       [ ] Edit         [O] Click

              [X] Delete"
                            ).text_style(mono_style.clone()));

                            ui.add_space(15.0);
                            ui.label(egui::RichText::new("      SPECIAL BUTTONS").strong());
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(
"  [SELECT] History Back
  [START]  Toggle Bag
  [L3]     This Help"
                            ).text_style(mono_style.clone()));
                        });
                    });

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Summary table
                    ui.label(egui::RichText::new("Quick Reference").strong());
                    ui.add_space(5.0);

                    egui::Grid::new("gamepad_help_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("D-Pad / L-Stick:");
                            ui.label("Navigate graph (N/S/E/W)");
                            ui.end_row();

                            ui.label("L1 / R1:");
                            ui.label("Navigate layers (Up/Down)");
                            ui.end_row();

                            ui.label("Circle (O):");
                            ui.label("Click/Activate vertex");
                            ui.end_row();

                            ui.label("Cross (X):");
                            ui.label("Delete vertex");
                            ui.end_row();

                            ui.label("Square:");
                            ui.label("Edit text at cursor");
                            ui.end_row();

                            ui.label("Triangle:");
                            ui.label("Create new text vertex");
                            ui.end_row();

                            ui.label("R2 (hold):");
                            ui.label("Record audio (release to save)");
                            ui.end_row();

                            ui.label("L2:");
                            ui.label("Yank vertex to bag");
                            ui.end_row();

                            ui.label("Start:");
                            ui.label("Toggle bag panel");
                            ui.end_row();

                            ui.label("Select:");
                            ui.label("Go back in history");
                            ui.end_row();
                        });
                });
        });
}
