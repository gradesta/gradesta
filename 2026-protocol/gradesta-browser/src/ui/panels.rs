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

/// Render the top panel with URL bar and connection controls
///
/// Returns an action if the user requested a connection or refresh.
pub fn render_top_panel(
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    url_bar_id: egui::Id,
    cmd_refresh: bool,
) -> TopPanelAction {
    let mut action = TopPanelAction::None;

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("URL:");
            // Lock keyboard input while Ctrl+L is being pressed
            let lock_input = app_state.focus_url_bar_next_frame;
            let text_edit = egui::TextEdit::singleline(&mut app_state.url_input)
                .id(url_bar_id)
                .desired_width(600.0)
                .lock_focus(lock_input)
                .hint_text("ws://localhost:8080/ws?landmark=/home/");
            let response = ui.add(text_edit);

            let button_label = if app_state.connected { "Refresh" } else { "Connect" };
            let button_clicked = ui.button(button_label).clicked();
            let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if button_clicked || enter_pressed || cmd_refresh {
                let url = app_state.url_input.trim().to_string();
                if url.is_empty() {
                    app_state.status = "URL is empty!".to_string();
                } else if app_state.connected {
                    action = TopPanelAction::Refresh;
                } else {
                    action = TopPanelAction::Connect { url };
                }
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(&app_state.status);
            if let Some(uri) = &graph.context_uri {
                ui.separator();
                ui.label(format!("Landmark: {}", uri));
            }
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
