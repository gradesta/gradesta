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
use crate::state::{AppState, ElfTask, ElfPanelFocus};
use crate::ui::SidebarGamepadInput;

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
    gamepad_input: &SidebarGamepadInput,
) -> Option<ElfAction> {
    let mut action = None;

    // Handle gamepad back button to close
    if gamepad_input.back {
        return Some(ElfAction::Close);
    }

    ui.heading("Elves");
    ui.separator();

    // Gamepad navigation help
    ui.label("↑↓=Select | ←→=Section | R3=Activate | ○=Close");
    ui.separator();

    // Handle section navigation (left/right changes focus section)
    let focus = app_state.elf_panel.gamepad_focus;
    let elf_count = app_state.trusted_elves.len();
    let command_count = app_state.trusted_elves
        .get(app_state.elf_panel.selected_elf_index)
        .and_then(|e| e.manifest.as_ref())
        .map(|m| m.commands.len())
        .unwrap_or(0);

    // Section order: ElfList -> CommandList -> Directions -> Permissions -> Summon
    if gamepad_input.nav_right {
        app_state.elf_panel.gamepad_focus = match focus {
            ElfPanelFocus::ElfList if command_count > 0 => ElfPanelFocus::CommandList,
            ElfPanelFocus::ElfList => ElfPanelFocus::Directions,
            ElfPanelFocus::CommandList => ElfPanelFocus::Directions,
            ElfPanelFocus::Directions => ElfPanelFocus::Permissions,
            ElfPanelFocus::Permissions => ElfPanelFocus::Summon,
            ElfPanelFocus::Summon => ElfPanelFocus::Summon,
        };
    }
    if gamepad_input.nav_left {
        app_state.elf_panel.gamepad_focus = match focus {
            ElfPanelFocus::ElfList => ElfPanelFocus::ElfList,
            ElfPanelFocus::CommandList => ElfPanelFocus::ElfList,
            ElfPanelFocus::Directions if command_count > 0 => ElfPanelFocus::CommandList,
            ElfPanelFocus::Directions => ElfPanelFocus::ElfList,
            ElfPanelFocus::Permissions => ElfPanelFocus::Directions,
            ElfPanelFocus::Summon => ElfPanelFocus::Permissions,
        };
    }

    // Handle up/down navigation within current section
    match app_state.elf_panel.gamepad_focus {
        ElfPanelFocus::ElfList => {
            if gamepad_input.nav_up && app_state.elf_panel.selected_elf_index > 0 {
                app_state.elf_panel.selected_elf_index -= 1;
                app_state.elf_panel.selected_command_index = 0;
            }
            if gamepad_input.nav_down && app_state.elf_panel.selected_elf_index < elf_count.saturating_sub(1) {
                app_state.elf_panel.selected_elf_index += 1;
                app_state.elf_panel.selected_command_index = 0;
            }
        }
        ElfPanelFocus::CommandList => {
            if gamepad_input.nav_up && app_state.elf_panel.selected_command_index > 0 {
                app_state.elf_panel.selected_command_index -= 1;
            }
            if gamepad_input.nav_down && app_state.elf_panel.selected_command_index < command_count.saturating_sub(1) {
                app_state.elf_panel.selected_command_index += 1;
            }
        }
        ElfPanelFocus::Directions => {
            // 6 directions: W, E, N, S, U, D (indices 0-5)
            if gamepad_input.nav_up && app_state.elf_panel.direction_cursor > 0 {
                app_state.elf_panel.direction_cursor -= 1;
            }
            if gamepad_input.nav_down && app_state.elf_panel.direction_cursor < 5 {
                app_state.elf_panel.direction_cursor += 1;
            }
            // Toggle on select
            if gamepad_input.select || gamepad_input.cross {
                let flag = match app_state.elf_panel.direction_cursor {
                    0 => DIRECTION_WEST,
                    1 => DIRECTION_EAST,
                    2 => DIRECTION_NORTH,
                    3 => DIRECTION_SOUTH,
                    4 => DIRECTION_UP,
                    _ => DIRECTION_DOWN,
                };
                app_state.elf_panel.selected_directions ^= flag;
            }
        }
        ElfPanelFocus::Permissions => {
            // 4 permissions: Read, Write, Create, Delete (indices 0-3)
            if gamepad_input.nav_up && app_state.elf_panel.permission_cursor > 0 {
                app_state.elf_panel.permission_cursor -= 1;
            }
            if gamepad_input.nav_down && app_state.elf_panel.permission_cursor < 3 {
                app_state.elf_panel.permission_cursor += 1;
            }
            // Toggle on select
            if gamepad_input.select || gamepad_input.cross {
                let flag = match app_state.elf_panel.permission_cursor {
                    0 => PERM_READ,
                    1 => PERM_WRITE,
                    2 => PERM_CREATE,
                    _ => PERM_DELETE,
                };
                app_state.elf_panel.selected_permissions ^= flag;
            }
        }
        ElfPanelFocus::Summon => {
            // Select triggers summon
            if gamepad_input.select || gamepad_input.cross {
                let can_summon = current_vertex.is_some()
                    && app_state.trusted_elves.get(app_state.elf_panel.selected_elf_index)
                        .map(|e| e.is_reachable)
                        .unwrap_or(false);
                if can_summon {
                    action = Some(ElfAction::Summon {
                        elf_index: app_state.elf_panel.selected_elf_index,
                        command_index: app_state.elf_panel.selected_command_index,
                        directions: app_state.elf_panel.selected_directions,
                        permissions: app_state.elf_panel.selected_permissions,
                    });
                }
            }
        }
    }

    let focus = app_state.elf_panel.gamepad_focus;

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

    // List of trusted elves with focus indicator
    let elf_list_focused = focus == ElfPanelFocus::ElfList;
    let section_header = if elf_list_focused { "▶ Trusted Elves:" } else { "  Trusted Elves:" };

    if app_state.trusted_elves.is_empty() {
        ui.label("No trusted elves configured.");
        ui.label("Add an elf URL above to get started.");
    } else {
        ui.label(section_header);
        ui.add_space(4.0);

        let mut remove_index = None;
        let mut refresh_index = None;

        for (idx, elf) in app_state.trusted_elves.iter().enumerate() {
            let selected = idx == app_state.elf_panel.selected_elf_index;
            let is_focused_item = elf_list_focused && selected;

            let frame = egui::Frame::new()
                .fill(if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    egui::Color32::TRANSPARENT
                })
                .stroke(if is_focused_item {
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100))
                } else {
                    egui::Stroke::NONE
                })
                .inner_margin(4.0);

            frame.show(ui, |ui| {
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

            // Command selection with focus indicator
            let cmd_list_focused = focus == ElfPanelFocus::CommandList;
            let cmd_header = if cmd_list_focused { "▶ Commands:" } else { "  Commands:" };
            ui.label(cmd_header);

            for (cmd_idx, cmd) in manifest.commands.iter().enumerate() {
                let selected = cmd_idx == app_state.elf_panel.selected_command_index;
                let is_focused_item = cmd_list_focused && selected;

                let label = if is_focused_item {
                    egui::RichText::new(&cmd.name).color(egui::Color32::from_rgb(100, 200, 100))
                } else {
                    egui::RichText::new(&cmd.name)
                };

                if ui.selectable_label(selected, label).clicked() {
                    app_state.elf_panel.selected_command_index = cmd_idx;
                }
                if selected {
                    ui.indent("cmd_desc", |ui| {
                        ui.label(&cmd.description);
                    });
                }
            }

            ui.separator();

            // Region direction selection with focus indicator
            let dir_focused = focus == ElfPanelFocus::Directions;
            let dir_header = if dir_focused { "▶ Region Directions:" } else { "  Region Directions:" };
            ui.label(dir_header);

            let directions = [
                (DIRECTION_WEST, "W", 0),
                (DIRECTION_EAST, "E", 1),
                (DIRECTION_NORTH, "N", 2),
                (DIRECTION_SOUTH, "S", 3),
                (DIRECTION_UP, "U", 4),
                (DIRECTION_DOWN, "D", 5),
            ];

            ui.horizontal(|ui| {
                for (flag, label, idx) in directions {
                    let is_cursor = dir_focused && app_state.elf_panel.direction_cursor == idx;
                    direction_checkbox_with_focus(ui, &mut app_state.elf_panel.selected_directions, flag, label, is_cursor);
                }
            });

            // Permission selection with focus indicator
            let perm_focused = focus == ElfPanelFocus::Permissions;
            let perm_header = if perm_focused { "▶ Permissions:" } else { "  Permissions:" };
            ui.label(perm_header);

            let permissions = [
                (PERM_READ, "Read", 0),
                (PERM_WRITE, "Write", 1),
                (PERM_CREATE, "Create", 2),
                (PERM_DELETE, "Delete", 3),
            ];

            ui.horizontal(|ui| {
                for (flag, label, idx) in permissions {
                    let is_cursor = perm_focused && app_state.elf_panel.permission_cursor == idx;
                    perm_checkbox_with_focus(ui, &mut app_state.elf_panel.selected_permissions, flag, label, is_cursor);
                }
            });

            ui.add_space(8.0);

            // Summon button with focus indicator
            let summon_focused = focus == ElfPanelFocus::Summon;
            let can_summon = current_vertex.is_some() && elf.is_reachable;

            ui.add_enabled_ui(can_summon, |ui| {
                let btn = if summon_focused {
                    egui::Button::new(egui::RichText::new("▶ Summon Elf").color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(60, 100, 60))
                } else {
                    egui::Button::new("Summon Elf")
                };
                if ui.add(btn).clicked() {
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

fn direction_checkbox_with_focus(ui: &mut egui::Ui, directions: &mut u8, flag: u8, label: &str, is_focused: bool) {
    let mut checked = (*directions & flag) != 0;
    let display_label = if is_focused {
        egui::RichText::new(label).color(egui::Color32::from_rgb(100, 200, 100)).strong()
    } else {
        egui::RichText::new(label)
    };
    if ui.checkbox(&mut checked, display_label).changed() {
        if checked {
            *directions |= flag;
        } else {
            *directions &= !flag;
        }
    }
}

fn perm_checkbox_with_focus(ui: &mut egui::Ui, permissions: &mut u8, flag: u8, label: &str, is_focused: bool) {
    let mut checked = (*permissions & flag) != 0;
    let display_label = if is_focused {
        egui::RichText::new(label).color(egui::Color32::from_rgb(100, 200, 100)).strong()
    } else {
        egui::RichText::new(label)
    };
    if ui.checkbox(&mut checked, display_label).changed() {
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
