//! Command bar (vim-style command palette) rendering
//!
//! Renders the command bar overlay for fuzzy command search and LLM natural language commands.
//! - Regular input: fuzzy matches against Command slugs
//! - Input starting with `"`: sends to LLM for natural language interpretation

use bevy_egui::egui;

use crate::commands::Command;
use crate::state::AppState;
use crate::voice_command::{self, AgentAction, VoiceCommandChannel, VoiceCommandConfig};

/// Action from the command bar
#[derive(Clone, Debug)]
pub enum CommandBarAction {
    None,
    Execute(Command),
    ExecuteScript(AgentAction),
}

/// Render the command bar overlay
///
/// Returns an action if the user selected a command or closed the bar.
pub fn render_command_bar(
    ctx: &egui::Context,
    app_state: &mut AppState,
    voice_channel: &VoiceCommandChannel,
    voice_config: &VoiceCommandConfig,
) -> CommandBarAction {
    if !app_state.show_command_bar {
        return CommandBarAction::None;
    }

    let mut action = CommandBarAction::None;

    #[allow(deprecated)]
    let screen_rect = ctx.screen_rect();
    let command_bar_width = (screen_rect.width() * 0.6).min(800.0).max(400.0);
    let command_bar_x = (screen_rect.width() - command_bar_width) / 2.0;

    // Detect LLM mode: input starts with "
    let is_llm_mode = app_state.command_bar_input.starts_with('"');
    let llm_query = if is_llm_mode {
        app_state.command_bar_input[1..].to_string()
    } else {
        String::new()
    };

    // Fixed height for stable positioning
    let fixed_height = 400.0;
    let command_bar_y = screen_rect.height() - fixed_height - 20.0;

    egui::Window::new("Command Bar")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_pos(egui::pos2(command_bar_x, command_bar_y))
        .fixed_size(egui::vec2(command_bar_width, fixed_height))
        .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(30, 30, 35)))
        .show(ctx, |ui| {
            // Check for arrow keys BEFORE rendering TextEdit (so we can intercept them)
            let arrow_down = ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowDown));
            let arrow_up = ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowUp));

            // Track if we were already in list mode (to avoid double-processing arrow keys)
            let was_in_list = app_state.command_bar_in_list;

            // Input bar
            let text_edit_id = egui::Id::new("command_bar_input");
            ui.horizontal(|ui| {
                ui.label(":");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app_state.command_bar_input)
                        .desired_width(command_bar_width - 30.0)
                        .hint_text("Type command or \"natural language query")
                        .id(text_edit_id),
                );

                // Focus the input on first show (only if not in list mode)
                if !app_state.command_bar_in_list && (response.gained_focus() || app_state.command_bar_input.is_empty()) {
                    response.request_focus();
                }

                // Handle arrow down: move from text input to list
                if response.has_focus() && arrow_down {
                    app_state.command_bar_in_list = true;
                    app_state.command_bar_selected = 0;
                    response.surrender_focus();
                }
            });

            // Handle arrow up at top of list: return to text input
            if app_state.command_bar_in_list && arrow_up && app_state.command_bar_selected == 0 {
                app_state.command_bar_in_list = false;
                ui.ctx().memory_mut(|mem| mem.request_focus(text_edit_id));
            }

            // Handle arrow navigation within the list (only if we were already in list mode)
            if was_in_list && app_state.command_bar_in_list {
                if arrow_down {
                    app_state.command_bar_selected = app_state.command_bar_selected.saturating_add(1);
                }
                if arrow_up && app_state.command_bar_selected > 0 {
                    app_state.command_bar_selected = app_state.command_bar_selected.saturating_sub(1);
                }
            }

            ui.add_space(4.0);
            ui.separator();

            // Show loading spinner if waiting for LLM
            if app_state.command_bar_llm_pending {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("Interpreting: {}", &llm_query));
                });
            }
            // If we have LLM interpretations, show them
            else if !app_state.command_bar_interpretations.is_empty() {
                render_llm_interpretations(ui, app_state, &mut action);
            }
            // Regular mode: fuzzy matching
            else if !is_llm_mode || llm_query.is_empty() {
                render_command_matches(ctx, ui, app_state, command_bar_width, &mut action);
            }
            // LLM mode but no query submitted yet
            else {
                ui.label(egui::RichText::new("Press Enter to interpret with LLM").weak());
            }

            // Handle keyboard input for command bar
            handle_command_bar_input(
                ctx,
                app_state,
                is_llm_mode,
                &llm_query,
                voice_channel,
                voice_config,
            );
        });

    action
}

/// Render the LLM interpretation results
fn render_llm_interpretations(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    action: &mut CommandBarAction,
) {
    let selected = app_state.command_bar_interpretation_selected;
    let interps_len = app_state.command_bar_interpretations.len();

    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        for (i, interp) in app_state.command_bar_interpretations.iter().enumerate() {
            let is_selected = i == selected;
            let bg = if is_selected {
                egui::Color32::from_rgb(60, 60, 100)
            } else {
                egui::Color32::TRANSPARENT
            };

            ui.horizontal(|ui| {
                let rect = ui.available_rect_before_wrap();
                let rect = egui::Rect::from_min_size(rect.min, egui::vec2(ui.available_width(), 22.0));
                ui.painter().rect_filled(rect, 2.0, bg);

                // Confidence indicator
                let confidence_pct = (interp.confidence * 100.0) as u32;
                ui.label(egui::RichText::new(format!("{}%", confidence_pct)).weak().monospace());

                // Action indicator
                match &interp.action {
                    AgentAction::Script(script) => {
                        // Show first command from script
                        let first_cmd = script.lines().next().unwrap_or("");
                        ui.label(egui::RichText::new(first_cmd).strong());
                    }
                    AgentAction::Cancel => {
                        ui.label(egui::RichText::new("Cancel").weak());
                    }
                }

                ui.label("-");
                ui.label(&interp.explanation);
            });
        }
    });

    // Handle selection - capture values before mutable borrow
    let key_down = ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowDown));
    let key_up = ui.ctx().input(|i| i.key_pressed(egui::Key::ArrowUp));
    let key_enter = ui.ctx().input(|i| i.key_pressed(egui::Key::Enter));

    if key_down {
        app_state.command_bar_interpretation_selected =
            (selected + 1).min(interps_len.saturating_sub(1));
    }
    if key_up {
        app_state.command_bar_interpretation_selected = selected.saturating_sub(1);
    }
    if key_enter && interps_len > 0 {
        let selected_action = app_state.command_bar_interpretations
            .get(selected.min(interps_len - 1))
            .map(|interp| interp.action.clone());

        if let Some(a) = selected_action {
            *action = CommandBarAction::ExecuteScript(a);
            // Clear state
            app_state.show_command_bar = false;
            app_state.command_bar_interpretations.clear();
            app_state.command_bar_interpretation_selected = 0;
            app_state.command_bar_input.clear();
        }
    }
}

/// Render the fuzzy-matched command list
fn render_command_matches(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    command_bar_width: f32,
    action: &mut CommandBarAction,
) {
    let input = &app_state.command_bar_input;
    let matches: Vec<_> = if input.is_empty() {
        Command::all().into_iter().take(15).collect()
    } else {
        Command::all()
            .into_iter()
            .filter(|cmd| cmd.fuzzy_matches(input))
            .take(15)
            .collect()
    };

    // Ensure selected index is valid
    if app_state.command_bar_selected >= matches.len() {
        app_state.command_bar_selected = matches.len().saturating_sub(1);
    }

    // Display matches
    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        for (i, cmd) in matches.iter().enumerate() {
            let is_selected = i == app_state.command_bar_selected && app_state.command_bar_in_list;
            let bg = if is_selected {
                egui::Color32::from_rgb(60, 60, 100)
            } else {
                egui::Color32::TRANSPARENT
            };

            ui.horizontal(|ui| {
                let rect = ui.available_rect_before_wrap();
                let rect =
                    egui::Rect::from_min_size(rect.min, egui::vec2(command_bar_width - 20.0, 22.0));
                ui.painter().rect_filled(rect, 2.0, bg);

                ui.label(egui::RichText::new(cmd.slug()).strong());
                ui.label("-");
                ui.label(cmd.description());

                // Show keybinding if any
                let bindings = app_state.keybindings.get_bindings(cmd);
                if !bindings.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(bindings[0].to_string()).weak());
                    });
                }
            });
        }
    });

    // Handle keyboard input (arrow navigation is in render_command_bar to avoid double-processing)
    let key_enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
    let key_tab = ctx.input(|i| i.key_pressed(egui::Key::Tab));

    // Enter executes the selected command (works both in list mode and when typing)
    if key_enter && !matches.is_empty() {
        let idx = if app_state.command_bar_in_list {
            app_state.command_bar_selected.min(matches.len() - 1)
        } else {
            0 // Execute first match when not in list mode
        };
        let cmd = matches[idx].clone();
        app_state.show_command_bar = false;
        app_state.command_bar_input.clear();
        app_state.command_bar_selected = 0;
        app_state.command_bar_in_list = false;
        app_state.command_bar_llm_pending = false;
        app_state.command_bar_interpretations.clear();
        app_state.command_bar_interpretation_selected = 0;
        *action = CommandBarAction::Execute(cmd);
    }

    if key_tab && !matches.is_empty() {
        // Autocomplete with selected command
        let cmd = &matches[app_state.command_bar_selected.min(matches.len() - 1)];
        app_state.command_bar_input = cmd.slug().to_string();
    }
}

/// Handle keyboard input for command bar
fn handle_command_bar_input(
    ctx: &egui::Context,
    app_state: &mut AppState,
    is_llm_mode: bool,
    llm_query: &str,
    voice_channel: &VoiceCommandChannel,
    voice_config: &VoiceCommandConfig,
) {
    let key_escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    let key_enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));

    // Escape closes command bar
    if key_escape {
        app_state.show_command_bar = false;
        app_state.command_bar_input.clear();
        app_state.command_bar_selected = 0;
        app_state.command_bar_in_list = false;
        app_state.command_bar_llm_pending = false;
        app_state.command_bar_interpretations.clear();
        app_state.command_bar_interpretation_selected = 0;
    }

    // In LLM mode, Enter triggers LLM query (if not already pending and no interpretations)
    if is_llm_mode
        && !llm_query.is_empty()
        && !app_state.command_bar_llm_pending
        && app_state.command_bar_interpretations.is_empty()
        && !app_state.command_bar_in_list
        && key_enter
    {
        if let Some(api_key) = voice_command::load_api_key() {
            app_state.command_bar_llm_pending = true;
            voice_command::query_llm_for_command_bar(
                llm_query,
                voice_channel.tx.clone(),
                api_key,
                voice_config.model.clone(),
            );
        } else {
            app_state.status = "No API key configured for LLM".to_string();
        }
    }
}

