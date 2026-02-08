//! Top and bottom panel rendering for UI system
//!
//! Renders the URL bar (top) and help/status bar (bottom).

use bevy_egui::egui;

use crate::graph::GraphState;
use crate::sidebar::SidebarMode;
use crate::state::AppState;
use crate::tts;

/// Action from the top panel
#[derive(Clone, Debug, PartialEq)]
pub enum TopPanelAction {
    None,
    Connect { url: String },
    Refresh,
}

/// Action from the bottom panel
#[derive(Clone, Debug, PartialEq)]
pub enum BottomPanelAction {
    None,
    ToggleTTS,
    OpenKeybindings,
    ToggleIdentities,
    ToggleBag,
}

/// Render the top panel with URL bar (server + landmark) and connection controls
///
/// Returns an action if the user requested a connection or refresh.
pub fn render_top_panel(
    ctx: &egui::Context,
    app_state: &mut AppState,
    _graph: &GraphState,
    cmd_refresh: bool,
) -> TopPanelAction {
    let mut action = TopPanelAction::None;

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Server input
            ui.label("Server:");
            let server_bar_id = egui::Id::new("server_bar");
            let lock_input = app_state.focus_url_bar_next_frame;
            let server_edit = egui::TextEdit::singleline(&mut app_state.server_input)
                .id(server_bar_id)
                .desired_width(250.0)
                .lock_focus(lock_input)
                .hint_text("ws://localhost:8080");
            let server_response = ui.add(server_edit);
            app_state.server_bar_has_focus = server_response.has_focus();

            ui.add_space(8.0);

            // Landmark input
            ui.label("Landmark:");
            let landmark_bar_id = egui::Id::new("landmark_bar");
            let landmark_edit = egui::TextEdit::singleline(&mut app_state.landmark_input)
                .id(landmark_bar_id)
                .desired_width(300.0)
                .hint_text("/");
            let landmark_response = ui.add(landmark_edit);
            app_state.landmark_bar_has_focus = landmark_response.has_focus();

            // Track combined URL bar focus state
            app_state.url_bar_has_focus = app_state.server_bar_has_focus || app_state.landmark_bar_has_focus;

            let button_label = if app_state.connected { "Refresh" } else { "Connect" };
            let button_clicked = ui.button(button_label).clicked();

            // Copy URL button (clipboard icon)
            let copy_clicked = ui.button("📋").clicked();
            if copy_clicked {
                let full_url = super::url_utils::construct_full_url(&app_state.server_input, &app_state.landmark_input);
                super::url_utils::set_clipboard_text(&full_url);
                app_state.status = "Copied URL to clipboard".to_string();
            }

            let server_enter = server_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let landmark_enter = landmark_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if button_clicked || server_enter || landmark_enter || cmd_refresh {
                let server = app_state.server_input.trim().to_string();
                let landmark = app_state.landmark_input.trim().to_string();
                if server.is_empty() {
                    app_state.status = "Server is empty!".to_string();
                } else if landmark.is_empty() {
                    app_state.status = "Landmark is empty!".to_string();
                } else {
                    let url = super::url_utils::construct_full_url(&server, &landmark);
                    if app_state.connected {
                        action = TopPanelAction::Refresh;
                    } else {
                        action = TopPanelAction::Connect { url };
                    }
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(&app_state.status);
            ui.separator();
            ui.label(format!("Zoom: {:.0}%", app_state.zoom_level * 100.0));
        });
        ui.add_space(8.0);
    });

    action
}

/// Render the bottom panel with navigation help and quick actions
///
/// Returns an action if the user clicked a button.
pub fn render_bottom_panel(
    ctx: &egui::Context,
    app_state: &mut AppState,
) -> BottomPanelAction {
    let mut action = BottomPanelAction::None;

    egui::TopBottomPanel::bottom("help_panel").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("↑↓←→ Nav | Enter=Click | Space=Record | I=Edit | Y=Yank | Ctrl+K=Keybindings");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // TTS mode indicator
                let tts_label = if app_state.tts_mode { "🔊 TTS ON" } else { "🔇 TTS" };
                if ui.button(tts_label).on_hover_text("Toggle text-to-speech (Ctrl+T)").clicked() {
                    app_state.tts_mode = !app_state.tts_mode;
                    if !app_state.tts_mode {
                        tts::stop();
                    }
                    action = BottomPanelAction::ToggleTTS;
                }
                ui.separator();
                // Keybindings button
                if ui.button("⌨ Keybindings (Ctrl+K)").clicked() {
                    app_state.sidebar.mode = SidebarMode::Keybindings;
                    action = BottomPanelAction::OpenKeybindings;
                }
                ui.separator();
                if ui.button("🔑 Identities").clicked() {
                    app_state.show_identity_panel = !app_state.show_identity_panel;
                    action = BottomPanelAction::ToggleIdentities;
                }
                let id_count = app_state.identity_config.identities.len();
                if id_count > 0 {
                    ui.label(format!("{} id", id_count));
                }
                ui.separator();
                // Bag indicator
                let bag_count = app_state.bag.len();
                let bag_label = if bag_count > 0 {
                    format!("📋 {}", bag_count)
                } else {
                    "📋".to_string()
                };
                if ui.button(&bag_label).clicked() {
                    app_state.show_bag_panel = !app_state.show_bag_panel;
                    action = BottomPanelAction::ToggleBag;
                }
            });
        });
        ui.add_space(4.0);
    });

    action
}
