//! Keybindings editor sidebar

use bevy_egui::egui;

use crate::commands::{Command, Context};
use crate::keybindings::{KeyBinding, KeybindingResolver, Preset};

/// State for the keybindings editor
pub struct KeybindingsEditorState {
    /// Command being edited (if any)
    pub editing_command: Option<Command>,
    /// Text input for new keybinding
    pub new_binding_input: String,
    /// Filter text for searching commands
    pub filter: String,
    /// Currently selected context filter
    pub context_filter: Option<Context>,
    /// Recording new key binding
    pub recording: bool,
    /// Pending preset to apply (for confirmation)
    pub pending_preset: Option<Preset>,
}

impl Default for KeybindingsEditorState {
    fn default() -> Self {
        Self {
            editing_command: None,
            new_binding_input: String::new(),
            filter: String::new(),
            context_filter: None,
            recording: false,
            pending_preset: None,
        }
    }
}

/// Render the keybindings editor sidebar
pub fn render_keybindings_editor(
    ui: &mut egui::Ui,
    resolver: &mut KeybindingResolver,
    state: &mut KeybindingsEditorState,
) -> KeybindingsAction {
    let mut action = KeybindingsAction::None;

    ui.horizontal(|ui| {
        ui.heading("⌨ Keybindings");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕ Close").clicked() {
                action = KeybindingsAction::Close;
            }
            if ui.button("↺ Reset All").clicked() {
                resolver.reset_all_to_defaults();
                action = KeybindingsAction::Save;
            }
        });
    });
    ui.separator();

    // Preset selector
    ui.horizontal(|ui| {
        ui.label("Presets:");
        for preset in Preset::all() {
            if ui.button(preset.name()).on_hover_text(preset.description()).clicked() {
                state.pending_preset = Some(preset.clone());
            }
        }
    });

    // Preset confirmation dialog
    if let Some(ref preset) = state.pending_preset.clone() {
        ui.add_space(4.0);
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Apply {} preset?", preset.name()));
                if ui.button("Apply").clicked() {
                    action = KeybindingsAction::ApplyPreset(preset.clone());
                    state.pending_preset = None;
                }
                if ui.button("Cancel").clicked() {
                    state.pending_preset = None;
                }
            });
            ui.label(egui::RichText::new(preset.description()).weak().small());
        });
    }

    ui.separator();

    // Search/filter bar
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.text_edit_singleline(&mut state.filter);
    });

    // Context filter tabs
    ui.horizontal_wrapped(|ui| {
        let contexts = [
            (None, "All"),
            (Some(Context::Global), "Global"),
            (Some(Context::Graph), "Graph"),
            (Some(Context::Bag), "Bag"),
            (Some(Context::TextInput), "Text"),
            (Some(Context::Recording), "Record"),
            (Some(Context::Authentication), "Auth"),
            (Some(Context::NavPanel), "Nav"),
        ];
        for (ctx, name) in contexts {
            let selected = state.context_filter == ctx;
            if ui.selectable_label(selected, name).clicked() {
                state.context_filter = ctx;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();

    // List of commands with their keybindings
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let all_commands = Command::all();
            let filter_lower = state.filter.to_lowercase();

            for cmd in all_commands {
                // Apply context filter
                if let Some(ctx) = state.context_filter {
                    if cmd.context() != ctx {
                        continue;
                    }
                }

                // Apply text filter
                if !filter_lower.is_empty() {
                    let slug_lower = cmd.slug().to_lowercase();
                    let desc_lower = cmd.description().to_lowercase();
                    if !slug_lower.contains(&filter_lower) && !desc_lower.contains(&filter_lower) {
                        continue;
                    }
                }

                let bindings = resolver.get_bindings(&cmd);
                let is_editing = state.editing_command.as_ref() == Some(&cmd);

                // Command row
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(cmd.slug()).strong().monospace());
                            ui.label(egui::RichText::new(cmd.description()).weak().small());
                        });
                    });

                    // Show current bindings
                    ui.horizontal_wrapped(|ui| {
                        if bindings.is_empty() {
                            ui.label(egui::RichText::new("(no binding)").weak().italics());
                        } else {
                            for binding in &bindings {
                                ui.label(
                                    egui::RichText::new(format!("  {} ", binding))
                                        .background_color(egui::Color32::from_rgb(50, 50, 70))
                                        .monospace()
                                );
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_editing {
                                // Editing mode
                                if ui.button("Done").clicked() {
                                    state.editing_command = None;
                                    state.new_binding_input.clear();
                                }
                                if ui.button("Reset").clicked() {
                                    resolver.reset_to_default(&cmd);
                                    state.editing_command = None;
                                    action = KeybindingsAction::Save;
                                }
                            } else {
                                if ui.button("Edit").clicked() {
                                    state.editing_command = Some(cmd.clone());
                                    state.new_binding_input.clear();
                                }
                            }
                        });
                    });

                    // Editing UI
                    if is_editing {
                        ui.add_space(4.0);
                        ui.separator();

                        // Show existing bindings with remove buttons
                        for binding in &bindings {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(binding.to_string())
                                        .monospace()
                                );
                                if ui.small_button("✕").clicked() {
                                    // Remove this binding
                                    let new_bindings: Vec<_> = bindings.iter()
                                        .filter(|b| *b != binding)
                                        .cloned()
                                        .collect();
                                    resolver.rebind(&cmd, new_bindings);
                                    action = KeybindingsAction::Save;
                                }
                            });
                        }

                        // Add new binding
                        ui.horizontal(|ui| {
                            ui.label("Add:");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut state.new_binding_input)
                                    .hint_text("e.g., Ctrl+Shift+N")
                                    .desired_width(120.0)
                            );

                            if ui.button("➕").clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                                if let Ok(new_binding) = KeyBinding::parse(&state.new_binding_input) {
                                    // Check for conflicts
                                    if let Some(conflict_cmd) = resolver.find_conflict(cmd.context(), &new_binding) {
                                        if conflict_cmd != cmd {
                                            // There's a conflict - warn but allow
                                            ui.colored_label(
                                                egui::Color32::YELLOW,
                                                format!("Warning: conflicts with {}", conflict_cmd.slug())
                                            );
                                        }
                                    }

                                    // Add the binding
                                    let mut new_bindings = bindings.clone();
                                    new_bindings.push(new_binding);
                                    resolver.rebind(&cmd, new_bindings);
                                    state.new_binding_input.clear();
                                    action = KeybindingsAction::Save;
                                }
                            }
                        });
                    }
                });

                ui.add_space(4.0);
            }
        });

    action
}

/// Actions returned by the keybindings editor
#[derive(Debug, Clone, PartialEq)]
pub enum KeybindingsAction {
    None,
    Close,
    Save,
    ApplyPreset(Preset),
}
