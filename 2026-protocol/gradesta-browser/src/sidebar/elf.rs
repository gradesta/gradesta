//! Elf management panel in the sidebar
//!
//! This panel allows users to:
//! - Add/remove trusted elf URLs
//! - View elf manifests and available commands
//! - Configure region and permissions for summoning
//! - Summon elves and view task output

use bevy_egui::egui;

use crate::network::{
    DIRECTION_WEST, DIRECTION_EAST, DIRECTION_NORTH, DIRECTION_SOUTH,
    DIRECTION_UP, DIRECTION_DOWN, PERM_READ, PERM_WRITE, PERM_CREATE, PERM_DELETE,
};
use crate::state::{AppState, ElfTask};

/// Actions that can be taken from the elf panel
#[derive(Clone, Debug)]
pub enum ElfAction {
    /// Add a new trusted elf URL
    AddElf(String),
    /// Remove an elf by index
    RemoveElf(usize),
    /// Refresh the manifest for an elf
    RefreshManifest(usize),
    /// Summon an elf with the selected command
    Summon {
        elf_index: usize,
        command_index: usize,
        directions: u8,
        permissions: u8,
    },
    /// Close the panel
    Close,
}

/// Render the elf management panel
pub fn render_elf_panel(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    current_vertex: Option<u64>,
    _current_landmark: &str,
) -> Option<ElfAction> {
    let mut action = None;

    ui.heading("Elves");
    ui.separator();

    // URL input for adding new elves
    ui.horizontal(|ui| {
        ui.label("URL:");
        let response = ui.text_edit_singleline(&mut app_state.elf_panel.url_input);
        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if !app_state.elf_panel.url_input.is_empty() {
                action = Some(ElfAction::AddElf(app_state.elf_panel.url_input.clone()));
                app_state.elf_panel.url_input.clear();
            }
        }
        if ui.button("Add").clicked() && !app_state.elf_panel.url_input.is_empty() {
            action = Some(ElfAction::AddElf(app_state.elf_panel.url_input.clone()));
            app_state.elf_panel.url_input.clear();
        }
    });

    ui.separator();

    // List of trusted elves
    if app_state.trusted_elves.is_empty() {
        ui.label("No trusted elves configured.");
        ui.label("Add an elf URL above to get started.");
    } else {
        ui.label("Trusted Elves:");
        ui.add_space(4.0);

        let mut remove_index = None;
        let mut refresh_index = None;

        for (idx, elf) in app_state.trusted_elves.iter().enumerate() {
            let selected = idx == app_state.elf_panel.selected_elf_index;

            egui::Frame::new()
                .fill(if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    egui::Color32::TRANSPARENT
                })
                .inner_margin(4.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Elf name/URL
                        let name = elf.manifest
                            .as_ref()
                            .map(|m| m.name.as_str())
                            .unwrap_or(&elf.url);

                        let status_icon = if elf.is_reachable { "✓" } else { "✗" };
                        let status_color = if elf.is_reachable {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::RED
                        };

                        ui.colored_label(status_color, status_icon);

                        if ui.selectable_label(selected, name).clicked() {
                            app_state.elf_panel.selected_elf_index = idx;
                            app_state.elf_panel.selected_command_index = 0;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("×").clicked() {
                                remove_index = Some(idx);
                            }
                            if ui.small_button("↻").clicked() {
                                refresh_index = Some(idx);
                            }
                        });
                    });
                });
        }

        if let Some(idx) = remove_index {
            action = Some(ElfAction::RemoveElf(idx));
        }
        if let Some(idx) = refresh_index {
            action = Some(ElfAction::RefreshManifest(idx));
        }
    }

    ui.separator();

    // Selected elf details
    if let Some(elf) = app_state.trusted_elves.get(app_state.elf_panel.selected_elf_index) {
        if let Some(manifest) = &elf.manifest {
            ui.label(format!("Elf: {}", manifest.name));
            ui.label(&manifest.description);
            ui.add_space(4.0);

            // Command selection
            ui.label("Commands:");
            for (cmd_idx, cmd) in manifest.commands.iter().enumerate() {
                let selected = cmd_idx == app_state.elf_panel.selected_command_index;
                if ui.selectable_label(selected, &cmd.name).clicked() {
                    app_state.elf_panel.selected_command_index = cmd_idx;
                }
                if selected {
                    ui.indent("cmd_desc", |ui| {
                        ui.label(&cmd.description);
                    });
                }
            }

            ui.separator();

            // Region direction selection
            ui.label("Region Directions:");
            ui.horizontal(|ui| {
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_WEST, "W");
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_EAST, "E");
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_NORTH, "N");
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_SOUTH, "S");
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_UP, "U");
                direction_checkbox(ui, &mut app_state.elf_panel.selected_directions, DIRECTION_DOWN, "D");
            });

            // Permission selection
            ui.label("Permissions:");
            ui.horizontal(|ui| {
                perm_checkbox(ui, &mut app_state.elf_panel.selected_permissions, PERM_READ, "Read");
                perm_checkbox(ui, &mut app_state.elf_panel.selected_permissions, PERM_WRITE, "Write");
                perm_checkbox(ui, &mut app_state.elf_panel.selected_permissions, PERM_CREATE, "Create");
                perm_checkbox(ui, &mut app_state.elf_panel.selected_permissions, PERM_DELETE, "Delete");
            });

            ui.add_space(8.0);

            // Summon button
            let can_summon = current_vertex.is_some() && elf.is_reachable;
            ui.add_enabled_ui(can_summon, |ui| {
                if ui.button("Summon Elf").clicked() {
                    action = Some(ElfAction::Summon {
                        elf_index: app_state.elf_panel.selected_elf_index,
                        command_index: app_state.elf_panel.selected_command_index,
                        directions: app_state.elf_panel.selected_directions,
                        permissions: app_state.elf_panel.selected_permissions,
                    });
                }
            });

            if current_vertex.is_none() {
                ui.label("(Select a vertex first)");
            }
        } else {
            ui.label("Loading manifest...");
        }
    }

    ui.separator();

    // Active tasks
    if !app_state.active_elf_tasks.is_empty() {
        ui.collapsing("Active Tasks", |ui| {
            for (action_id, task) in &app_state.active_elf_tasks {
                render_task(ui, *action_id, task);
            }
        });
    }

    ui.separator();

    if ui.button("Close").clicked() {
        action = Some(ElfAction::Close);
    }

    action
}

fn direction_checkbox(ui: &mut egui::Ui, directions: &mut u8, flag: u8, label: &str) {
    let mut checked = (*directions & flag) != 0;
    if ui.checkbox(&mut checked, label).changed() {
        if checked {
            *directions |= flag;
        } else {
            *directions &= !flag;
        }
    }
}

fn perm_checkbox(ui: &mut egui::Ui, permissions: &mut u8, flag: u8, label: &str) {
    let mut checked = (*permissions & flag) != 0;
    if ui.checkbox(&mut checked, label).changed() {
        if checked {
            *permissions |= flag;
        } else {
            *permissions &= !flag;
        }
    }
}

fn render_task(ui: &mut egui::Ui, action_id: u64, task: &ElfTask) {
    let status_text = if task.completed {
        format!("Completed (status: {})", task.status.unwrap_or(0))
    } else {
        "Running...".to_string()
    };

    ui.collapsing(format!("{} - {}", task.command, status_text), |ui| {
        ui.label(format!("Elf: {}", task.elf_url));
        ui.label(format!("Action ID: {}", action_id));

        if let Some(ref msg) = task.message {
            ui.label(format!("Message: {}", msg));
        }
    });
}
