//! Processing logic for text input, recording, and identification
//!
//! Handles keyboard shortcuts and state updates for input modes.

use bevy_egui::egui;

use crate::audio::AudioRecordingSignal;
use crate::graph::GraphState;
use crate::identity;
use crate::network::{NetEventsTx, ServerEvent, WsCommand, WsCommandTx};
use crate::state::{AppState, IdentificationAction, InputMode, PendingVertexCreation};
use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

/// Result of text input processing
#[derive(Clone, Debug, PartialEq)]
pub enum TextInputResult {
    None,
    Submitted,
    Cancelled,
}

/// Process text input keyboard shortcuts (Ctrl+Enter to submit, Escape to cancel)
pub fn process_text_input(
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    ws_cmd_tx: &WsCommandTx,
    net_events: &NetEventsTx,
) -> TextInputResult {
    if !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        return TextInputResult::None;
    }

    let (submit, cancel) = ctx.input(|i| (
        i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl,
        i.key_pressed(egui::Key::Escape)
    ));

    if submit {
        if let InputMode::TextInput { direction } = app_state.input_mode.clone() {
            let text = app_state.text_input_buffer.clone();
            if !text.is_empty() {
                if let Some(current_id) = app_state.current_vertex {
                    if let Some(ref tx) = ws_cmd_tx.0 {
                        if let Some(dir) = direction {
                            // Create new vertex
                            submit_new_vertex(tx, app_state, current_id, dir, text);
                        } else {
                            // Update existing vertex
                            submit_edit_vertex(tx, app_state, graph, net_events, current_id, text);
                        }
                    }
                }
            }
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
        }
        return TextInputResult::Submitted;
    }

    if cancel {
        app_state.input_mode = InputMode::Normal;
        app_state.text_input_buffer.clear();
        app_state.status = "Text input cancelled".to_string();
        return TextInputResult::Cancelled;
    }

    TextInputResult::None
}

fn submit_new_vertex(
    tx: &crossbeam_channel::Sender<WsCommand>,
    app_state: &mut AppState,
    current_id: u64,
    dir: usize,
    text: String,
) {
    let dir_byte = match dir {
        EDGE_WEST => 0,
        EDGE_EAST => 1,
        EDGE_NORTH => 2,
        EDGE_SOUTH => 3,
        EDGE_UP => 4,
        EDGE_DOWN => 5,
        _ => 0,
    };
    let action_id = app_state.next_action_id;
    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
    let text_bytes = text.into_bytes();
    let _ = tx.send(WsCommand::CreateVertex {
        action_id,
        from_vertex: current_id,
        direction: dir_byte,
        layer: 0,
        mime: "text/plain".to_string(),
        data: text_bytes.clone(),
    });
    app_state.pending_creations.insert(action_id, PendingVertexCreation {
        samples: Vec::new(),
        sample_rate: 0,
        data: text_bytes,
        mime: "text/plain".to_string(),
    });
    app_state.status = "Creating new note...".to_string();
}

fn submit_edit_vertex(
    tx: &crossbeam_channel::Sender<WsCommand>,
    app_state: &mut AppState,
    graph: &GraphState,
    net_events: &NetEventsTx,
    current_id: u64,
    text: String,
) {
    // Determine which layer to save to
    let layer = if let Some(vertex) = graph.vertices.get(&current_id) {
        let mime = vertex.mime.as_deref().unwrap_or("");
        if mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url" {
            0 // Primary is text, update layer 0
        } else {
            1 // Primary is not text, add/update as layer 1
        }
    } else {
        0 // Fallback to layer 0
    };

    let action_id = app_state.next_action_id;
    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
    let text_bytes = text.into_bytes();
    let _ = tx.send(WsCommand::SetVertexLabel {
        action_id,
        vertex_id: current_id,
        layer,
        mime: "text/plain".to_string(),
        data: text_bytes.clone(),
    });
    // Optimistically update local graph
    let _ = net_events.0.send(ServerEvent::SetVertexLabel {
        vertex_id: current_id,
        layer,
        mime: "text/plain".to_string(),
        data: text_bytes,
    });
    app_state.status = "Saving changes...".to_string();
}

/// Process recording cancel (Escape during recording)
pub fn process_recording_cancel(
    ctx: &egui::Context,
    app_state: &mut AppState,
    audio_signal: &AudioRecordingSignal,
) -> bool {
    if !matches!(app_state.input_mode, InputMode::Recording { .. }) {
        return false;
    }

    let escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));
    if !escape_pressed {
        return false;
    }

    // Signal to stop audio recording
    if let Ok(mut stop) = audio_signal.should_stop.lock() {
        *stop = true;
    }

    app_state.input_mode = InputMode::Normal;
    app_state.recording_start = None;
    if let Ok(mut samples) = app_state.audio_samples.lock() {
        samples.clear();
    }
    app_state.status = "Recording cancelled".to_string();
    true
}

/// Process identification dialog
pub fn process_identification(
    ctx: &egui::Context,
    app_state: &mut AppState,
    ws_cmd_tx: &WsCommandTx,
) {
    // Handle identification in a separate pass to avoid borrow conflicts
    let mut id_action: Option<IdentificationAction> = None;

    if let Some(ref pending) = app_state.pending_identification.clone() {
        let selected_idx = app_state.selected_identity_index;

        // Check if this is a remembered server (auto-identify)
        let is_remembered = app_state.identity_config.identities.get(selected_idx)
            .map(|id| id.remembered_servers.contains(&pending.server_url))
            .unwrap_or(false);

        if is_remembered {
            id_action = Some(IdentificationAction::Identify { remember: false });
        } else {
            // Handle keyboard shortcuts for identification dialog
            let id_enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            let id_tab_pressed = ctx.input(|i| i.key_pressed(egui::Key::Tab));
            let id_escape_pressed = ctx.input(|i| i.key_pressed(egui::Key::Escape));

            // Tab cycles through identities
            if id_tab_pressed && !app_state.identity_config.identities.is_empty() {
                let num_identities = app_state.identity_config.identities.len();
                app_state.selected_identity_index = (app_state.selected_identity_index + 1) % num_identities;
            }

            // Enter confirms identification
            if id_enter_pressed && !app_state.identity_config.identities.is_empty() {
                id_action = Some(IdentificationAction::Identify { remember: false });
            }

            // Escape refuses
            if id_escape_pressed {
                id_action = Some(IdentificationAction::Refuse);
            }
        }
    }

    // Process identification action
    if let Some(action) = id_action {
        if let Some(pending) = app_state.pending_identification.take() {
            match action {
                IdentificationAction::Identify { remember } => {
                    execute_identification(app_state, ws_cmd_tx, pending, remember);
                }
                IdentificationAction::Refuse => {
                    if let Some(ref tx) = ws_cmd_tx.0 {
                        let _ = tx.send(WsCommand::IdentificationRefused {
                            action_id: pending.action_id,
                        });
                    }
                    app_state.status = "Identification refused".to_string();
                }
            }
        }
    }
}

fn execute_identification(
    app_state: &mut AppState,
    ws_cmd_tx: &WsCommandTx,
    pending: crate::state::PendingIdentification,
    remember: bool,
) {
    let idx = app_state.selected_identity_index;
    let mut success = false;
    let mut display_name = String::new();

    // First, load the signing key if needed
    if let Some(identity) = app_state.identity_config.identities.get(idx) {
        if identity.signing_key.is_none() {
            match identity::load_signing_key(&identity.nextcloud_url, &identity.username, &identity.app_password) {
                Ok(key) => {
                    if let Some(id) = app_state.identity_config.identities.get_mut(idx) {
                        id.signing_key = Some(key);
                    }
                }
                Err(e) => {
                    app_state.status = format!("Failed to load signing key: {}", e);
                }
            }
        }
    }

    // Now sign and send
    if let Some(identity) = app_state.identity_config.identities.get(idx) {
        if let Some(ref signing_key) = identity.signing_key {
            let signature = identity::sign_challenge(signing_key, &pending.nonce, pending.timestamp);
            if let Some(ref tx) = ws_cmd_tx.0 {
                let _ = tx.send(WsCommand::IdentificationResponse {
                    action_id: pending.action_id,
                    identity_url: identity.share_url.clone(),
                    signature,
                });
            }
            display_name = identity.display_name.clone();
            success = true;
        }
    }

    if success {
        if remember {
            // Clone what we need for Nextcloud sync before mutable borrow
            let sync_info = app_state.identity_config.identities.get(idx).map(|id| {
                (
                    id.nextcloud_url.clone(),
                    id.username.clone(),
                    id.app_password.clone(),
                    id.display_name.clone(),
                    id.share_url.clone(),
                )
            });

            if let Some(identity) = app_state.identity_config.identities.get_mut(idx) {
                if !identity.remembered_servers.contains(&pending.server_url) {
                    identity.remembered_servers.push(pending.server_url.clone());
                }
            }

            // Save and sync after mutable borrow is done
            let _ = app_state.identity_config.save();
            if let Some((nc_url, nc_user, nc_pass, disp_name, share_url)) = sync_info {
                if let Some(identity) = app_state.identity_config.identities.get(idx) {
                    let metadata = identity::IdentityMetadata {
                        display_name: disp_name,
                        share_url,
                        remembered_servers: identity.remembered_servers.clone(),
                    };
                    let _ = identity::upload_identity_metadata(&nc_url, &nc_user, &nc_pass, &metadata);
                }
            }
            app_state.status = format!("Identified as {} (remembered)", display_name);
        } else {
            app_state.status = format!("Identified as {}", display_name);
        }
    }
}
