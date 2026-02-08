//! Identity management panel

use bevy_egui::egui;

/// Actions returned from the identity management UI
pub enum IdentityAction {
    None,
    Close,
    Remove(usize),
    ConnectNextcloud(String),
}

/// Identity display info
pub struct IdentityInfo {
    pub display_name: String,
    pub share_url: String,
}

/// Render the identity management panel
///
/// # Arguments
/// * `ui` - egui UI context
/// * `identities` - List of configured identities
/// * `nextcloud_url_input` - Mutable reference to the Nextcloud URL input field
///
/// # Returns
/// Action to take
pub fn render_identity_management(
    ui: &mut egui::Ui,
    identities: &[IdentityInfo],
    nextcloud_url_input: &mut String,
) -> IdentityAction {
    let mut action = IdentityAction::None;

    ui.heading("Identity Management");
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Close").clicked() {
            action = IdentityAction::Close;
        }
    });

    ui.separator();

    // List existing identities
    ui.heading("Your Identities");
    if identities.is_empty() {
        ui.label("No identities configured. Add a Nextcloud account below.");
    } else {
        for (i, identity) in identities.iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(&identity.display_name);
                    ui.label("(display name)");
                    if ui.small_button("Remove").clicked() {
                        action = IdentityAction::Remove(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Identity URL:");
                    ui.monospace(&identity.share_url);
                });
            });
        }
    }

    ui.separator();
    ui.heading("Add Nextcloud Account");

    ui.horizontal(|ui| {
        ui.label("Nextcloud URL:");
        ui.text_edit_singleline(nextcloud_url_input);
    });

    if ui.button("Connect Nextcloud Account").clicked() {
        let url = nextcloud_url_input.trim().to_string();
        if !url.is_empty() {
            action = IdentityAction::ConnectNextcloud(url);
        }
    }

    action
}
