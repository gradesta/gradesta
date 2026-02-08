//! Command bar (vim-style command palette) rendering
//!
//! Renders the command bar overlay for fuzzy command search.

use bevy_egui::egui;

use crate::commands::Command;
use crate::sidebar::SidebarMode;
use crate::state::AppState;
use crate::state::{ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::tts;

/// Action from the command bar
#[derive(Clone, Debug)]
pub enum CommandBarAction {
    None,
    Close,
    Execute(Command),
}

/// Render the command bar overlay
///
/// Returns an action if the user selected a command or closed the bar.
pub fn render_command_bar(
    ctx: &egui::Context,
    app_state: &mut AppState,
) -> CommandBarAction {
    if !app_state.show_command_bar {
        return CommandBarAction::None;
    }

    let mut action = CommandBarAction::None;

    let screen_rect = ctx.screen_rect();
    let command_bar_width = (screen_rect.width() * 0.6).min(800.0).max(400.0);
    let command_bar_x = (screen_rect.width() - command_bar_width) / 2.0;

    egui::Window::new("Command Bar")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .fixed_pos(egui::pos2(command_bar_x, screen_rect.height() - 150.0))
        .fixed_size(egui::vec2(command_bar_width, 130.0))
        .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(30, 30, 35)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(":");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app_state.command_bar_input)
                        .desired_width(command_bar_width - 30.0)
                        .hint_text("Type command (e.g., graph.yank, global.zoom_in)")
                        .id(egui::Id::new("command_bar_input"))
                );

                // Focus the input on first show
                if response.gained_focus() || app_state.command_bar_input.is_empty() {
                    response.request_focus();
                }
            });

            ui.add_space(4.0);
            ui.separator();

            // Get matching commands
            let matches: Vec<_> = if app_state.command_bar_input.is_empty() {
                Command::all().into_iter().take(5).collect()
            } else {
                Command::all()
                    .into_iter()
                    .filter(|cmd| cmd.fuzzy_matches(&app_state.command_bar_input))
                    .take(5)
                    .collect()
            };

            // Display matches
            egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                for (i, cmd) in matches.iter().enumerate() {
                    let is_selected = i == app_state.command_bar_selected;
                    let bg = if is_selected {
                        egui::Color32::from_rgb(60, 60, 100)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    ui.horizontal(|ui| {
                        let rect = ui.available_rect_before_wrap();
                        let rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(command_bar_width - 20.0, 18.0)
                        );
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

            // Handle keyboard navigation in command bar
            ctx.input(|i| {
                if i.key_pressed(egui::Key::ArrowDown) {
                    app_state.command_bar_selected = (app_state.command_bar_selected + 1).min(matches.len().saturating_sub(1));
                }
                if i.key_pressed(egui::Key::ArrowUp) {
                    app_state.command_bar_selected = app_state.command_bar_selected.saturating_sub(1);
                }
                if i.key_pressed(egui::Key::Enter) && !matches.is_empty() {
                    let cmd = matches[app_state.command_bar_selected.min(matches.len() - 1)].clone();
                    app_state.show_command_bar = false;
                    app_state.command_bar_input.clear();
                    app_state.command_bar_selected = 0;
                    action = CommandBarAction::Execute(cmd);
                }
                if i.key_pressed(egui::Key::Tab) {
                    // Autocomplete with selected command
                    if !matches.is_empty() {
                        let cmd = &matches[app_state.command_bar_selected.min(matches.len() - 1)];
                        app_state.command_bar_input = cmd.slug().to_string();
                    }
                }
            });
        });

    action
}

/// Execute a command from the command bar
pub fn execute_command_bar_command(cmd: Command, app_state: &mut AppState) {
    match cmd {
        Command::GlobalZoomIn => {
            app_state.zoom_level = (app_state.zoom_level + ZOOM_STEP).min(ZOOM_MAX);
            app_state.status = format!("Zoom: {:.0}%", app_state.zoom_level * 100.0);
        }
        Command::GlobalZoomOut => {
            app_state.zoom_level = (app_state.zoom_level - ZOOM_STEP).max(ZOOM_MIN);
            app_state.status = format!("Zoom: {:.0}%", app_state.zoom_level * 100.0);
        }
        Command::GlobalZoomReset => {
            app_state.zoom_level = 1.0;
            app_state.status = "Zoom reset".to_string();
        }
        Command::GlobalToggleBag => {
            app_state.show_bag_panel = !app_state.show_bag_panel;
            if app_state.show_bag_panel {
                app_state.show_nav_panel = false;
            }
        }
        Command::GlobalToggleNavPanel => {
            app_state.show_nav_panel = !app_state.show_nav_panel;
            if app_state.show_nav_panel {
                app_state.show_bag_panel = false;
            }
        }
        Command::GlobalCloseModal => {
            app_state.sidebar.fullscreen = false;
            app_state.show_text_modal = false;
            app_state.show_image_modal = false;
            app_state.show_video_modal = false;
        }
        Command::GlobalOpenKeybindings => {
            app_state.sidebar.mode = SidebarMode::Keybindings;
        }
        Command::GlobalToggleTTS => {
            app_state.tts_mode = !app_state.tts_mode;
            if app_state.tts_mode {
                app_state.status = "TTS mode enabled".to_string();
            } else {
                tts::stop();
                app_state.status = "TTS mode disabled".to_string();
            }
        }
        Command::GlobalTTSSpeedUp => {
            let rate = tts::get_rate();
            let new_rate = (rate + 0.25).min(3.0);
            tts::set_rate(new_rate);
            app_state.status = format!("TTS speed: {:.2}x", new_rate);
        }
        Command::GlobalTTSSpeedDown => {
            let rate = tts::get_rate();
            let new_rate = (rate - 0.25).max(0.5);
            tts::set_rate(new_rate);
            app_state.status = format!("TTS speed: {:.2}x", new_rate);
        }
        _ => {
            app_state.status = format!("Command not yet wired: {}", cmd.slug());
        }
    }
}
