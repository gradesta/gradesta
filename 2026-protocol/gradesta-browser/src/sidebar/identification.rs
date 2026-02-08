//! Identification request handling (blocking)

use bevy_egui::egui;

/// Actions returned from the identification UI
pub enum IdentificationAction {
    None,
    Identify { index: usize, remember: bool },
    Refuse,
    OpenIdentityPanel,
}

/// Identity option for selection
pub struct IdentityOption {
    pub display_name: String,
    pub share_url: String,
}

/// Render the identification request dialog
///
/// This is shown in the sidebar when a server requests identification.
/// The rest of the UI should be greyed out (blocking mode).
///
/// # Arguments
/// * `ui` - egui UI context
/// * `server_url` - The server requesting identification
/// * `reason` - The reason given by the server
/// * `identities` - Available identities to choose from
/// * `selected_index` - Currently selected identity index
///
/// # Returns
/// Action to take
pub fn render_identification_request(
    ui: &mut egui::Ui,
    server_url: &str,
    reason: &str,
    identities: &[IdentityOption],
    selected_index: &mut usize,
) -> IdentificationAction {
    let mut action = IdentificationAction::None;

    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.heading("Identification Request");
    });

    ui.separator();

    ui.label(format!("Server: {}", server_url));
    ui.label(format!("Reason: {}", reason));

    ui.separator();

    if identities.is_empty() {
        ui.label("No identities configured.");
        ui.label("You need to connect a Nextcloud account to identify yourself.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Connect Nextcloud Account").clicked() {
                action = IdentificationAction::OpenIdentityPanel;
            }
            if ui.button("Refuse (Escape)").clicked() {
                action = IdentificationAction::Refuse;
            }
        });
    } else {
        ui.label("Identify as (Tab to cycle):");

        for (i, identity) in identities.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.radio_value(selected_index, i, &identity.display_name);
            });
            if *selected_index == i {
                ui.indent("identity_url", |ui| {
                    ui.label(format!("URL: {}", identity.share_url));
                });
            }
        }

        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("Identify (Enter)").clicked() {
                action = IdentificationAction::Identify {
                    index: *selected_index,
                    remember: false,
                };
            }
            if ui.button("Identify + Remember").clicked() {
                action = IdentificationAction::Identify {
                    index: *selected_index,
                    remember: true,
                };
            }
            if ui.button("Refuse (Escape)").clicked() {
                action = IdentificationAction::Refuse;
            }
        });
    }

    action
}
