//! Command execution logic for UI system
//!
//! Executes captured keyboard commands and returns the results.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::audio::{
    generate_waveform_preview, run_audio_recording, set_audio_speed, spawn_audio_encoding_task,
    stop_audio, AudioPlaybackState, AudioProcessingChannel, AudioRecordingSignal,
};
use crate::commands::Command;
use crate::debug_log;
use crate::graph::GraphState;
use crate::media::MediaCache;
use crate::network::{WsCommand, WsCommandTx};
use crate::sidebar::SidebarMode;
use crate::state::{AppState, InputMode, PendingCell, PendingCellKind, PendingAudioStatus, PendingVertexCreation, PlaybackBoostState, TranscriptionMode};
use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};
use crate::state::{ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::tts;
use crate::voice_command::{
    self, AgentAction, AgentInterpretation, CellContext, VoiceCommandChannel,
    VoiceCommandEvent, VoiceCommandState, parse_script, ScriptInstruction,
};

use super::context_menu::{handle_context_menu_navigation, open_context_menu};
use super::input::CapturedCommands;

/// Results from command execution
#[derive(Clone, Debug, Default)]
pub struct CommandResults {
    /// Whether any command was processed
    pub any_command_processed: bool,
    /// Whether we should finalize recording this frame
    pub should_finalize_recording: bool,
    /// Whether voice command recording should be finalized
    pub should_finalize_voice_recording: bool,
    /// Whether to grant voice command permission (user selected Allow)
    pub should_grant_voice_permission: bool,
    /// Command selected from context menu (if any) - will be added to cmds next frame
    pub context_menu_command: Option<Command>,
}

/// Execute all captured commands and update state accordingly
///
/// This function handles all the command processing that was previously
/// inline in ui_system, including zoom, bag operations, paste, cut, delete,
/// recording, and modal control.
pub fn execute_commands(
    cmds: &CapturedCommands,
    app_state: &mut AppState,
    graph: &mut GraphState,
    ws_cmd_tx: &WsCommandTx,
    media_cache: &mut MediaCache,
    audio_signal: &AudioRecordingSignal,
    playback_state: &AudioPlaybackState,
    boost_state: &mut PlaybackBoostState,
    voice_channel: &VoiceCommandChannel,
    _ctx: &bevy_egui::egui::Context,
) -> CommandResults {
    let mut results = CommandResults::default();

    // GlobalCloseModal - close modals or exit fullscreen
    if cmds.has(Command::GlobalCloseModal) {
        results.any_command_processed = true;
        // First priority: close command bar
        if app_state.show_command_bar {
            app_state.show_command_bar = false;
            app_state.command_bar_input.clear();
            app_state.command_bar_selected = 0;
            app_state.command_bar_in_list = false;
            app_state.command_bar_llm_pending = false;
            app_state.command_bar_interpretations.clear();
            app_state.command_bar_interpretation_selected = 0;
        }
        // Next: exit fullscreen mode
        else if app_state.sidebar.fullscreen {
            app_state.sidebar.fullscreen = false;
        } else {
            // Close old-style modals (for backwards compat during transition)
            if app_state.show_text_modal {
                app_state.show_text_modal = false;
            }
            if app_state.show_image_modal {
                app_state.show_image_modal = false;
            }
            if app_state.show_video_modal {
                // Stop video player when closing
                if let Some(vertex_id) = app_state.video_modal_vertex_id {
                    if let Some(player) = media_cache.video_players.get(&vertex_id) {
                        player.stop();
                    }
                }
                app_state.show_video_modal = false;
            }
        }
    }

    // GlobalToggleFullscreen - toggle fullscreen or open modal with current content
    // (but not when in text input mode - that's for submitting)
    if cmds.has(Command::GlobalToggleFullscreen) && !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        results.any_command_processed = true;
        execute_toggle_fullscreen(app_state, graph);
    }

    // GraphClickVertex - "click" the current vertex (send click message to server)
    if cmds.has(Command::GraphClickVertex) && matches!(app_state.input_mode, InputMode::Normal) {
        results.any_command_processed = true;
        execute_click_vertex(app_state, ws_cmd_tx);
    }

    // Zoom commands
    if cmds.has(Command::GlobalZoomIn) {
        results.any_command_processed = true;
        app_state.zoom_level = (app_state.zoom_level + ZOOM_STEP).min(ZOOM_MAX);
    }
    if cmds.has(Command::GlobalZoomOut) {
        results.any_command_processed = true;
        app_state.zoom_level = (app_state.zoom_level - ZOOM_STEP).max(ZOOM_MIN);
    }
    if cmds.has(Command::GlobalZoomReset) {
        results.any_command_processed = true;
        app_state.zoom_level = 1.0;
    }
    if let Some(delta) = cmds.scroll_zoom {
        app_state.zoom_level = (app_state.zoom_level + delta).clamp(ZOOM_MIN, ZOOM_MAX);
    }
    if let Some(delta) = cmds.pinch_zoom {
        app_state.zoom_level = (app_state.zoom_level * delta).clamp(ZOOM_MIN, ZOOM_MAX);
    }

    // GlobalToggleBag
    if cmds.has(Command::GlobalToggleBag) {
        results.any_command_processed = true;
        app_state.show_bag_panel = !app_state.show_bag_panel;
        if app_state.show_bag_panel {
            app_state.show_nav_panel = false;
            app_state.show_elf_panel = false;
            app_state.show_debug_panel = false;
            app_state.show_identity_panel = false;
        }
    }

    // GlobalToggleNavPanel
    if cmds.has(Command::GlobalToggleNavPanel) {
        results.any_command_processed = true;
        app_state.show_nav_panel = !app_state.show_nav_panel;
        if app_state.show_nav_panel {
            app_state.show_bag_panel = false;
            app_state.show_elf_panel = false;
            app_state.show_debug_panel = false;
            app_state.show_identity_panel = false;
        }
    }

    // GlobalToggleElfPanel
    if cmds.has(Command::GlobalToggleElfPanel) {
        results.any_command_processed = true;
        app_state.show_elf_panel = !app_state.show_elf_panel;
        if app_state.show_elf_panel {
            app_state.show_bag_panel = false;
            app_state.show_nav_panel = false;
            app_state.show_debug_panel = false;
            app_state.show_identity_panel = false;
        }
    }

    // GlobalToggleDebugPanel
    if cmds.has(Command::GlobalToggleDebugPanel) {
        results.any_command_processed = true;
        app_state.show_debug_panel = !app_state.show_debug_panel;
        if app_state.show_debug_panel {
            app_state.show_bag_panel = false;
            app_state.show_nav_panel = false;
            app_state.show_elf_panel = false;
            app_state.show_identity_panel = false;
        }
    }

    // GlobalToggleIdentityPanel
    if cmds.has(Command::GlobalToggleIdentityPanel) {
        results.any_command_processed = true;
        app_state.show_identity_panel = !app_state.show_identity_panel;
        if app_state.show_identity_panel {
            app_state.show_bag_panel = false;
            app_state.show_nav_panel = false;
            app_state.show_elf_panel = false;
            app_state.show_debug_panel = false;
        }
    }

    // GraphYank - Yank (copy) current vertex to bag
    if cmds.has(Command::GraphYank) {
        results.any_command_processed = true;
        if let Some(current_id) = app_state.current_vertex {
            if app_state.bag.last() != Some(&current_id) {
                app_state.bag.push(current_id);
                app_state.status = format!("Yanked vertex to bag (depth: {})", app_state.bag.len());
            }
        }
    }

    // BagPop - Pop from bag (remove top without connecting)
    if cmds.has(Command::BagPop) {
        results.any_command_processed = true;
        if let Some(_popped) = app_state.bag.pop() {
            app_state.status = format!("Popped from bag (depth: {})", app_state.bag.len());
        } else {
            app_state.status = "Bag is empty".to_string();
        }
    }

    // GraphGoToBagTop - Go to top of bag (jump to that vertex)
    if cmds.has(Command::GraphGoToBagTop) && app_state.input_mode == InputMode::Normal {
        results.any_command_processed = true;
        if let Some(&top_id) = app_state.bag.last() {
            if let Some(current_id) = app_state.current_vertex {
                app_state.history.push(current_id);
            }
            app_state.current_vertex = Some(top_id);
            app_state.status = format!("Jumped to bag top (depth: {})", app_state.bag.len());
        } else {
            app_state.status = "Bag is empty".to_string();
        }
    }

    // GraphPaste - Paste from bag (connect bag vertex in insertion direction)
    if cmds.has(Command::GraphPaste) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_paste(app_state, graph, ws_cmd_tx);
    }

    // GraphCutEdge - Cut connection in the current navigation direction
    if cmds.has(Command::GraphCutEdge) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_cut_edge(app_state, graph, ws_cmd_tx);
    }

    // GraphDeleteVertex - Delete current vertex (if editable)
    if cmds.has(Command::GraphDeleteVertex) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_delete_vertex(app_state, graph, ws_cmd_tx);
    }

    // GraphEditText - Insert text at current vertex (edit)
    if cmds.has(Command::GraphEditText) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_edit_text(app_state, graph);
    }

    // GraphSetDirection* - Change last navigation direction without moving
    if app_state.input_mode == InputMode::Normal {
        let dir = if cmds.has(Command::GraphSetDirectionNorth) {
            Some(EDGE_NORTH)
        } else if cmds.has(Command::GraphSetDirectionSouth) {
            Some(EDGE_SOUTH)
        } else if cmds.has(Command::GraphSetDirectionWest) {
            Some(EDGE_WEST)
        } else if cmds.has(Command::GraphSetDirectionEast) {
            Some(EDGE_EAST)
        } else if cmds.has(Command::GraphSetDirectionUp) {
            Some(EDGE_UP)
        } else if cmds.has(Command::GraphSetDirectionDown) {
            Some(EDGE_DOWN)
        } else {
            None
        };
        if let Some(direction) = dir {
            results.any_command_processed = true;
            app_state.last_nav_direction = direction;
            app_state.status = format!("Direction set to {}", direction_name(direction));
        }
    }

    // GraphShowUndoTree - Navigate to undo tree view
    if cmds.has(Command::GraphShowUndoTree) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_show_undo_tree(app_state, graph, ws_cmd_tx);
    }

    // GraphReturnFromUndoTree - Return from undo tree to previous position
    if cmds.has(Command::GraphReturnFromUndoTree) && app_state.viewing_undo_tree {
        results.any_command_processed = true;
        execute_return_from_undo_tree(app_state, graph, ws_cmd_tx);
    }

    // Navigation commands
    if app_state.input_mode == InputMode::Normal {
        let nav_dir = if cmds.has(Command::GraphNavigateNorth) {
            Some(EDGE_NORTH)
        } else if cmds.has(Command::GraphNavigateSouth) {
            Some(EDGE_SOUTH)
        } else if cmds.has(Command::GraphNavigateEast) {
            Some(EDGE_EAST)
        } else if cmds.has(Command::GraphNavigateWest) {
            Some(EDGE_WEST)
        } else if cmds.has(Command::GraphNavigateUp) {
            Some(EDGE_UP)
        } else if cmds.has(Command::GraphNavigateDown) {
            Some(EDGE_DOWN)
        } else {
            None
        };

        if let Some(direction) = nav_dir {
            results.any_command_processed = true;
            if let Some(current_id) = app_state.current_vertex {
                if let Some(vertex) = graph.vertices.get(&current_id) {
                    let target_id = vertex.edges[direction];
                    if target_id != 0 && graph.vertices.contains_key(&target_id) {
                        app_state.history.push(current_id);
                        app_state.current_vertex = Some(target_id);
                        app_state.last_nav_direction = direction;
                    }
                }
            }
        }

        // History back
        if cmds.has(Command::GraphHistoryBack) {
            results.any_command_processed = true;
            if let Some(prev_id) = app_state.history.pop() {
                app_state.current_vertex = Some(prev_id);
            }
        }
    }

    // GraphNewTextVertex - Create new text vertex in last navigation direction
    // Creates a virtual placeholder locally and enters inline edit mode immediately
    // The actual CreateVertex is sent when user presses Ctrl+Enter to submit
    if cmds.has(Command::GraphNewTextVertex) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        let direction = app_state.last_nav_direction;

        if let Some(current_id) = app_state.current_vertex {
            // Create local placeholder ID
            let local_id = app_state.next_local_id;
            app_state.next_local_id = app_state.next_local_id.wrapping_sub(1);

            // Create virtual placeholder cell (not sent to server yet)
            let pending_cell = PendingCell {
                local_id,
                direction,
                from_vertex: current_id,
                created_at: Instant::now(),
                server_vertex_id: None,
                action_id: None,
                kind: PendingCellKind::Text,
            };

            app_state.pending_audio_cells.insert(local_id, pending_cell);

            eprintln!("GraphNewTextVertex: Created text placeholder local_id={} direction={}", local_id, direction);

            // Clear text buffer for editing
            app_state.text_input_buffer.clear();
            super::text_edit::reset_text_edit_state(app_state);

            // Enter inline edit mode with the local_id
            app_state.input_mode = InputMode::InlineEdit {
                vertex_id: local_id, // Using local_id as the "vertex_id" for new cells
                is_new: true,
                submitting: false,
                layer: 0, // New text cells always start at layer 0
            };

            app_state.status = "Editing new cell (Ctrl+Enter to save, Esc to cancel)".to_string();
        }
    }

    // GraphStartRecording - Push-to-talk recording
    if cmds.has(Command::GraphStartRecording) && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_start_recording(app_state, audio_signal, playback_state);
    }

    // GlobalCycleTranscriptionMode - Cycle through transcription modes
    if cmds.has(Command::GlobalCycleTranscriptionMode) {
        results.any_command_processed = true;
        app_state.transcription_mode = app_state.transcription_mode.next();
        app_state.status = format!("Transcription: {}", app_state.transcription_mode.label());
        // Persist to config
        let mut config = voice_command::VoiceCommandConfig::load();
        config.transcription_mode = app_state.transcription_mode;
        let _ = config.save();
    }

    // RecordingSave - Check for recording key release (push-to-talk stop)
    if let InputMode::Recording { .. } = &app_state.input_mode {
        if cmds.has(Command::RecordingSave) {
            results.any_command_processed = true;
            if let Ok(mut stop) = audio_signal.should_stop.lock() {
                *stop = true;
            }
        }
    }

    // GlobalOpenCommandBar - Open command bar (vim-style)
    if cmds.has(Command::GlobalOpenCommandBar) && app_state.input_mode == InputMode::Normal && !app_state.show_command_bar {
        results.any_command_processed = true;
        app_state.show_command_bar = true;
        app_state.command_bar_input.clear();
        app_state.command_bar_selected = 0;
        app_state.command_bar_in_list = false;
        app_state.command_bar_llm_pending = false;
        app_state.command_bar_interpretations.clear();
        app_state.command_bar_interpretation_selected = 0;
    }

    // GlobalOpenKeybindings - Open keybindings editor
    if cmds.has(Command::GlobalOpenKeybindings) {
        results.any_command_processed = true;
        app_state.sidebar.mode = SidebarMode::Keybindings;
    }

    // GlobalToggleTTS - Toggle text-to-speech mode
    if cmds.has(Command::GlobalToggleTTS) {
        results.any_command_processed = true;
        app_state.tts_mode = !app_state.tts_mode;
        if app_state.tts_mode {
            app_state.status = "TTS mode enabled (Ctrl+T to disable)".to_string();
        } else {
            tts::stop();
            app_state.status = "TTS mode disabled".to_string();
        }
    }

    // GlobalToggleGamepadHelp - Toggle gamepad help overlay
    if cmds.has(Command::GlobalToggleGamepadHelp) {
        results.any_command_processed = true;
        app_state.show_gamepad_help = !app_state.show_gamepad_help;
    }

    // GlobalToggleVoiceSettings - Toggle voice command settings dialog
    if cmds.has(Command::GlobalToggleVoiceSettings) {
        results.any_command_processed = true;
        app_state.show_voice_settings = !app_state.show_voice_settings;
    }

    // GlobalOpenContextMenu - Open the context menu (gamepad)
    if cmds.has(Command::GlobalOpenContextMenu) {
        eprintln!("GlobalOpenContextMenu triggered, input_mode: {:?}", app_state.input_mode);
        if app_state.input_mode == InputMode::Normal {
            results.any_command_processed = true;
            if app_state.context_menu.open {
                // Toggle off if already open
                app_state.context_menu.open = false;
            } else {
                open_context_menu(app_state);
            }
        }
    }

    // Context menu navigation
    if app_state.context_menu.open {
        if let Some(cmd) = handle_context_menu_navigation(
            app_state,
            cmds.context_menu_up,
            cmds.context_menu_down,
            cmds.context_menu_left,
            cmds.context_menu_right,
            cmds.context_menu_select,
            cmds.context_menu_back,
        ) {
            results.any_command_processed = true;
            // Store the selected command - will be processed next frame
            results.context_menu_command = Some(cmd);
        }
    }

    // Handle URL focus key - sets flag for main.rs to handle after TextEdit is rendered
    // (Focus must be requested AFTER the widget is rendered to ensure it's in used_ids)
    // Check both the special field (keyboard) and the command set (voice commands)
    if cmds.focus_url_down || cmds.has(Command::GlobalFocusUrl) {
        // Log only on first press (when transitioning from not pressed)
        if !app_state.focus_url_bar_next_frame {
            let key_info = app_state.keybindings.get_bindings(&Command::GlobalFocusUrl)
                .first()
                .map(|k| k.to_string());
            debug_log::log_command_triggered(app_state, Command::GlobalFocusUrl.slug(), key_info.as_deref());
        }
        app_state.focus_url_bar_next_frame = true;
    }
    // Note: focus_url_bar_next_frame is cleared in main.rs after focus is successfully applied

    // GlobalRefresh - trigger refresh (for voice commands; keyboard handled in main.rs)
    if cmds.has(Command::GlobalRefresh) {
        results.any_command_processed = true;
        // Set a flag that main.rs will check
        app_state.voice_refresh_pending = true;
    }

    // TextInputCancel - cancel text input mode or inline edit mode
    if cmds.has(Command::TextInputCancel) {
        if let InputMode::TextInput { .. } = app_state.input_mode {
            results.any_command_processed = true;
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
            app_state.status = "Text input cancelled".to_string();
        } else if let InputMode::InlineEdit { vertex_id, is_new, submitting, layer } = app_state.input_mode {
            // Don't allow cancel while submitting - wait for server response
            if submitting {
                return results;
            }
            results.any_command_processed = true;
            if is_new {
                // For new cells, just remove the local placeholder (never sent to server)
                app_state.pending_audio_cells.remove(&vertex_id);
                eprintln!("InlineEdit cancel: Removed local placeholder {}", vertex_id);
            } else {
                // Restore original content for existing vertex
                if let Some(original) = app_state.inline_edit_original.take() {
                    // Restore original content to the graph (no server call needed since we never saved)
                    if let Some(vertex) = graph.vertices.get_mut(&vertex_id) {
                        if let Some(layer_content) = vertex.layers.get_mut(&layer) {
                            layer_content.data = original.into_bytes();
                        }
                    }
                }
            }
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
            app_state.inline_edit_original = None;
            app_state.status = "Edit cancelled".to_string();
        }
    }

    // TextInputSubmit - submit text input or URL bar
    if cmds.has(Command::TextInputSubmit) {
        results.any_command_processed = true;
        // If URL bar was just focused/set, trigger refresh to connect
        if app_state.focus_url_bar_next_frame || app_state.url_bar_has_focus {
            app_state.voice_refresh_pending = true;
            app_state.focus_url_bar_next_frame = false;
        }
        // Handle text input mode submission
        else if let InputMode::TextInput { direction } = app_state.input_mode.clone() {
            let text = app_state.text_input_buffer.clone();
            if !text.is_empty() {
                if let Some(current_id) = app_state.current_vertex {
                    if let Some(ref tx) = ws_cmd_tx.0 {
                        if let Some(dir) = direction {
                            // Create new vertex
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
                                local_placeholder_id: None,
                                transcription_mode: TranscriptionMode::Off,
                                cloud_transcript: None,
                            });
                            app_state.status = "Creating new note...".to_string();
                        } else {
                            // Edit existing vertex - find which layer has editable text
                            let layer = if let Some(vertex) = graph.vertices.get(&current_id) {
                                // Find the first text layer that's not a portal or URL
                                let mut layer_nums: Vec<_> = vertex.layers.keys().copied().collect();
                                layer_nums.sort();
                                let mut found_layer = None;
                                for layer_num in layer_nums {
                                    if let Some(layer_content) = vertex.layers.get(&layer_num) {
                                        if layer_content.mime.starts_with("text/")
                                            && layer_content.mime != "text/gradesta-url"
                                            && layer_content.mime != "text/x-url"
                                        {
                                            found_layer = Some(layer_num);
                                            break;
                                        }
                                    }
                                }
                                // If no text layer found, create one in the first available layer
                                found_layer.unwrap_or_else(|| {
                                    // Find first unused layer number
                                    for i in 0..100 {
                                        if !vertex.layers.contains_key(&i) {
                                            return i;
                                        }
                                    }
                                    0
                                })
                            } else {
                                0 // Fallback
                            };
                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                            let text_bytes = text.into_bytes();
                            let _ = tx.send(WsCommand::SetVertexLabel {
                                action_id,
                                vertex_id: current_id,
                                layer,
                                mime: "text/plain".to_string(),
                                data: text_bytes,
                            });
                            app_state.status = "Saving changes...".to_string();
                        }
                    }
                }
            }
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
        }
        // Handle inline edit mode submission
        else if let InputMode::InlineEdit { vertex_id, is_new, submitting, layer } = app_state.input_mode {
            // Don't resubmit if already submitting
            if submitting {
                return results;
            }
            let text = app_state.text_input_buffer.clone();
            if let Some(ref tx) = ws_cmd_tx.0 {
                if is_new {
                    // New cell: send CreateVertex with the text content
                    // vertex_id is actually a local_id for new cells
                    let local_id = vertex_id;

                    if let Some(placeholder) = app_state.pending_audio_cells.get(&local_id) {
                        let direction = placeholder.direction;
                        let from_vertex = placeholder.from_vertex;

                        let dir_byte = match direction {
                            EDGE_WEST => 0,
                            EDGE_EAST => 1,
                            EDGE_NORTH => 2,
                            EDGE_SOUTH => 3,
                            EDGE_UP => 4,
                            EDGE_DOWN => 5,
                            _ => 3, // default south
                        };

                        let action_id = app_state.next_action_id;
                        app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                        let text_bytes = text.into_bytes();

                        // Send CreateVertex to server
                        let _ = tx.send(WsCommand::CreateVertex {
                            action_id,
                            from_vertex,
                            direction: dir_byte,
                            layer,
                            mime: "text/plain".to_string(),
                            data: text_bytes.clone(),
                        });

                        // Track pending creation so we can update when server responds
                        app_state.pending_creations.insert(action_id, PendingVertexCreation {
                            samples: Vec::new(),
                            sample_rate: 0,
                            data: text_bytes,
                            mime: "text/plain".to_string(),
                            local_placeholder_id: Some(local_id),
                            transcription_mode: TranscriptionMode::Off,
                            cloud_transcript: None,
                        });

                        // Update the placeholder with action_id so we can match server response
                        if let Some(cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                            cell.action_id = Some(action_id);
                        }

                        eprintln!("InlineEdit submit: Sending CreateVertex action_id={} local_id={}", action_id, local_id);
                        app_state.status = "Creating cell...".to_string();

                        // Stay in InlineEdit mode but mark as submitting - keeps grid centered
                        // Will switch to Normal when server responds with 202
                        app_state.input_mode = InputMode::InlineEdit {
                            vertex_id: local_id,
                            is_new: true,
                            submitting: true,
                            layer,
                        };
                        app_state.text_input_buffer.clear();
                        return results;
                    }
                } else {
                    // Existing cell: send SetVertexLabel to the correct layer
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    let text_bytes = text.into_bytes();
                    let _ = tx.send(WsCommand::SetVertexLabel {
                        action_id,
                        vertex_id,
                        layer,
                        mime: "text/plain".to_string(),
                        data: text_bytes.clone(),
                    });
                    // Update local graph state optimistically
                    if let Some(vertex) = graph.vertices.get_mut(&vertex_id) {
                        vertex.layers.insert(layer, crate::graph::LayerContent {
                            mime: "text/plain".to_string(),
                            data: text_bytes,
                        });
                    }
                    app_state.status = "Saved".to_string();
                }
            }
            // For existing cells or fallback, switch to normal immediately
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
            app_state.inline_edit_original = None;
        }
    }

    // GlobalPlaybackSpeedBoost - Boost playback speed for TTS and audio
    if cmds.has(Command::GlobalPlaybackSpeedBoost) {
        results.any_command_processed = true;
        let new_speed = boost_state.apply_boost();
        // Apply speed to both TTS and audio
        tts::set_rate(new_speed);
        set_audio_speed(new_speed);
        app_state.status = format!("Playback speed: {:.1}x", new_speed);
    }

    // Check if we should finalize recording (space was released)
    results.should_finalize_recording = if let InputMode::Recording { .. } = &app_state.input_mode {
        audio_signal.should_stop.lock().map(|s| *s).unwrap_or(false)
    } else {
        false
    };

    // Voice command mode handling
    // Start voice command when L2 held for 100ms+
    // Note: Does not require connection - voice commands can toggle UI, navigate locally, etc.
    if cmds.voice_command_start && app_state.input_mode == InputMode::Normal {
        results.any_command_processed = true;
        start_voice_command(app_state, playback_state, voice_channel);
    }

    // Handle voice command state machine
    if let InputMode::VoiceCommand(ref state) = app_state.input_mode.clone() {
        // Voice command cancel + burst - L2 released < 1 second
        // Cancel the voice command and trigger speed boost instead
        if cmds.voice_command_cancel_burst {
            if let VoiceCommandState::Recording { ref stop_signal, .. } = state {
                results.any_command_processed = true;
                // Stop the recording
                if let Ok(mut stop) = stop_signal.lock() {
                    *stop = true;
                }
                // Cancel voice command - return to normal mode
                app_state.input_mode = InputMode::Normal;
                app_state.recording_start = None;
                app_state.status = "Voice command cancelled".to_string();
                // Trigger speed boost
                let new_speed = boost_state.apply_boost();
                tts::set_rate(new_speed);
                set_audio_speed(new_speed);
                app_state.status = format!("Playback speed: {:.1}x", new_speed);
            }
        }
        // Voice command stop - triggers transition from Recording to Transcribing
        else if cmds.voice_command_stop {
            if let VoiceCommandState::Recording { ref stop_signal, .. } = state {
                results.any_command_processed = true;
                if let Ok(mut stop) = stop_signal.lock() {
                    *stop = true;
                }
                results.should_finalize_voice_recording = true;
            }
        }

        // Stop audio playback after 1 second of voice recording
        // (before 1 second, user might cancel and want burst instead)
        if let VoiceCommandState::Recording { .. } = state {
            if let Some(start) = app_state.recording_start {
                if start.elapsed() >= std::time::Duration::from_secs(1) {
                    stop_audio(playback_state);
                }
            }
        }

        // Selection navigation in Selecting state
        if let VoiceCommandState::Selecting { ref interpretations, selected, .. } = state {
            let max_idx = interpretations.len().saturating_sub(1);

            if cmds.voice_select_up {
                results.any_command_processed = true;
                let new_selected = selected.saturating_sub(1);
                if let InputMode::VoiceCommand(VoiceCommandState::Selecting { ref mut selected, .. }) = app_state.input_mode {
                    *selected = new_selected;
                }
            }

            if cmds.voice_select_down {
                results.any_command_processed = true;
                let new_selected = (selected + 1).min(max_idx);
                if let InputMode::VoiceCommand(VoiceCommandState::Selecting { ref mut selected, .. }) = app_state.input_mode {
                    *selected = new_selected;
                }
            }

            if cmds.voice_cancel {
                results.any_command_processed = true;
                app_state.input_mode = InputMode::Normal;
                app_state.status = "Voice command cancelled".to_string();
            }
        }

        // Permission response in AwaitingPermission state
        if let VoiceCommandState::AwaitingPermission { selected, .. } = state {
            // Left stick navigation between Allow (0) and Deny (1)
            if cmds.permission_select_left {
                results.any_command_processed = true;
                if let InputMode::VoiceCommand(VoiceCommandState::AwaitingPermission { ref mut selected, .. }) = app_state.input_mode {
                    *selected = 0; // Allow
                }
            }
            if cmds.permission_select_right {
                results.any_command_processed = true;
                if let InputMode::VoiceCommand(VoiceCommandState::AwaitingPermission { ref mut selected, .. }) = app_state.input_mode {
                    *selected = 1; // Deny
                }
            }

            // L3 confirms the selected button
            if cmds.permission_confirm {
                results.any_command_processed = true;
                if *selected == 0 {
                    // Allow - this will be handled in main.rs via grant_voice_permission
                    // Set a flag that main.rs checks
                    results.should_grant_voice_permission = true;
                } else {
                    // Deny - cancel voice command
                    app_state.input_mode = InputMode::Normal;
                    app_state.status = "Permission denied, voice command cancelled".to_string();
                }
            }

            // B button also denies (legacy)
            if cmds.voice_cancel {
                results.any_command_processed = true;
                app_state.input_mode = InputMode::Normal;
                app_state.status = "Permission denied, voice command cancelled".to_string();
            }
        }
    }

    results
}

/// Finalize recording after space key was released
///
/// This function:
/// 1. Finds the existing placeholder cell (created on recording start)
/// 2. Generates a waveform preview for visualization
/// 3. Updates status to Encoding and spawns background encoding task
/// 4. Returns to Normal mode instantly
pub fn finalize_recording(
    app_state: &mut AppState,
    audio_signal: &AudioRecordingSignal,
    audio_processing: &AudioProcessingChannel,
) {
    // Signal to stop recording
    if let Ok(mut stop) = audio_signal.should_stop.lock() {
        *stop = true;
    }
    // Small delay to let the recording thread notice
    thread::sleep(Duration::from_millis(50));

    if let InputMode::Recording { direction } = app_state.input_mode.clone() {
        // Get samples
        let samples = if let Ok(s) = app_state.audio_samples.lock() {
            s.clone()
        } else {
            Vec::new()
        };

        // Get actual sample rate from recording
        let sample_rate = audio_signal
            .actual_sample_rate
            .lock()
            .map(|sr| *sr)
            .unwrap_or(44100);

        eprintln!(
            "Recording finished: {} samples at {} Hz",
            samples.len(),
            sample_rate
        );

        // Find the existing placeholder cell (in Recording status)
        let recording_cell_id = app_state
            .pending_audio_cells
            .iter()
            .find(|(_, cell)| {
                cell.is_recording() && cell.direction == direction
            })
            .map(|(id, _)| *id);

        if samples.len() > 1000 {
            // At least some audio
            let duration_secs = samples.len() as f32 / sample_rate as f32;

            if let Some(local_id) = recording_cell_id {
                // Generate waveform preview for visualization (50 points)
                let waveform = generate_waveform_preview(&samples, 50);

                // Update existing placeholder cell
                if let Some(cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                    cell.set_audio_status(PendingAudioStatus::Encoding);
                    cell.set_waveform(waveform);
                    cell.set_audio_level(0.0);
                }

                app_state.status = format!("Recording saved ({:.1}s) - encoding...", duration_secs);

                eprintln!(
                    "Updated placeholder to encoding: local_id={} direction={}",
                    local_id, direction
                );

                // Spawn background encoding task
                spawn_audio_encoding_task(
                    local_id,
                    samples,
                    sample_rate,
                    audio_processing.tx.clone(),
                );
            }
        } else {
            // Recording too short - remove the placeholder
            if let Some(local_id) = recording_cell_id {
                app_state.pending_audio_cells.remove(&local_id);
            }
            app_state.status = "Recording too short (hold Space longer)".to_string();
        }

        app_state.input_mode = InputMode::Normal;
        app_state.recording_start = None;
        // Keep recording_placeholder_id set - placeholder stays selected until server responds
    }
}

// Helper functions

fn direction_name(direction: usize) -> &'static str {
    match direction {
        EDGE_NORTH => "north",
        EDGE_SOUTH => "south",
        EDGE_WEST => "west",
        EDGE_EAST => "east",
        EDGE_UP => "up",
        EDGE_DOWN => "down",
        _ => "?",
    }
}

fn opposite_direction(direction: usize) -> usize {
    match direction {
        EDGE_WEST => EDGE_EAST,
        EDGE_EAST => EDGE_WEST,
        EDGE_NORTH => EDGE_SOUTH,
        EDGE_SOUTH => EDGE_NORTH,
        EDGE_UP => EDGE_DOWN,
        EDGE_DOWN => EDGE_UP,
        _ => EDGE_NORTH,
    }
}

fn execute_toggle_fullscreen(app_state: &mut AppState, graph: &GraphState) {
    use crate::media::is_image_data;

    if app_state.sidebar.fullscreen {
        app_state.sidebar.fullscreen = false;
    } else if app_state.show_text_modal || app_state.show_image_modal || app_state.show_video_modal {
        app_state.sidebar.fullscreen = true;
    } else if let Some(current_id) = app_state.current_vertex {
        if let Some(vertex) = graph.vertices.get(&current_id) {
            // Check all layers for content types
            let has_image = vertex.layers.values()
                .any(|l| l.mime.starts_with("image/") || is_image_data(&l.data));

            // Check for text in any layer (excluding portals/URLs)
            let text_content = vertex.layers.values()
                .find(|l| l.mime.starts_with("text/") && l.mime != "text/gradesta-url" && l.mime != "text/x-url")
                .and_then(|l| String::from_utf8(l.data.clone()).ok());

            if let Some(text) = text_content {
                // Found text content - expand it
                app_state.text_modal_content = text;
                app_state.show_text_modal = true;
                app_state.sidebar.fullscreen = true;
            } else if has_image {
                app_state.image_modal_vertex_id = Some(current_id);
                app_state.show_image_modal = true;
                app_state.sidebar.fullscreen = true;
            }
        }
    }
}

fn execute_click_vertex(app_state: &mut AppState, ws_cmd_tx: &WsCommandTx) {
    if let Some(current_id) = app_state.current_vertex {
        if let Some(ref tx) = ws_cmd_tx.0 {
            let action_id = app_state.next_action_id;
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            let _ = tx.send(WsCommand::ClickVertex { action_id, vertex_id: current_id });
            app_state.status = format!("Clicked vertex {}", current_id);
        }
    }
}

fn execute_paste(app_state: &mut AppState, graph: &mut GraphState, ws_cmd_tx: &WsCommandTx) {
    if let Some(paste_id) = app_state.bag.pop() {
        if let Some(current_id) = app_state.current_vertex {
            if current_id == paste_id {
                app_state.bag.push(paste_id);
                app_state.status = "Cannot paste: vertex is already current".to_string();
            } else if let Some(ref tx) = ws_cmd_tx.0 {
                let direction = app_state.last_nav_direction;
                let opposite = opposite_direction(direction);

                let paste_edges = graph.vertices.get(&paste_id)
                    .map(|v| v.edges)
                    .unwrap_or([0; 6]);

                let mut new_current_edges = [u64::MAX; 6];
                new_current_edges[direction] = paste_id;

                let mut new_paste_edges = paste_edges;
                new_paste_edges[opposite] = current_id;

                let action_id1 = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = tx.send(WsCommand::SetEdges {
                    action_id: action_id1,
                    vertex_id: current_id,
                    edges: new_current_edges,
                });

                let action_id2 = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = tx.send(WsCommand::SetEdges {
                    action_id: action_id2,
                    vertex_id: paste_id,
                    edges: new_paste_edges,
                });

                // Optimistically update local graph
                if let Some(current_vertex) = graph.vertices.get_mut(&current_id) {
                    current_vertex.edges[direction] = paste_id;
                }
                if let Some(paste_vertex) = graph.vertices.get_mut(&paste_id) {
                    paste_vertex.edges[opposite] = current_id;
                }

                app_state.status = format!("Pasted vertex {} (bag: {})", direction_name(direction), app_state.bag.len());

                // Navigate to the pasted vertex
                app_state.history.push(current_id);
                app_state.current_vertex = Some(paste_id);
            }
        } else {
            app_state.bag.push(paste_id);
            app_state.status = "Cannot paste: no current vertex".to_string();
        }
    } else {
        app_state.status = "Bag is empty".to_string();
    }
}

fn execute_cut_edge(app_state: &mut AppState, graph: &mut GraphState, ws_cmd_tx: &WsCommandTx) {
    if let Some(current_id) = app_state.current_vertex {
        if let Some(ref tx) = ws_cmd_tx.0 {
            let direction = app_state.last_nav_direction;
            let opposite = opposite_direction(direction);

            let neighbor_id = graph.vertices.get(&current_id)
                .map(|v| v.edges[direction])
                .unwrap_or(0);

            if neighbor_id != 0 {
                let mut current_edges = [u64::MAX; 6];
                current_edges[direction] = 0;

                let action_id1 = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = tx.send(WsCommand::SetEdges {
                    action_id: action_id1,
                    vertex_id: current_id,
                    edges: current_edges,
                });

                let mut neighbor_edges = [u64::MAX; 6];
                neighbor_edges[opposite] = 0;

                let action_id2 = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = tx.send(WsCommand::SetEdges {
                    action_id: action_id2,
                    vertex_id: neighbor_id,
                    edges: neighbor_edges,
                });

                // Optimistically update local graph
                if let Some(current_vertex) = graph.vertices.get_mut(&current_id) {
                    current_vertex.edges[direction] = 0;
                }
                if let Some(neighbor_vertex) = graph.vertices.get_mut(&neighbor_id) {
                    neighbor_vertex.edges[opposite] = 0;
                }

                app_state.status = format!("Cut connection {}", direction_name(direction));
            } else {
                app_state.status = format!("No connection {} to cut", direction_name(direction));
            }
        }
    }
}

fn execute_delete_vertex(app_state: &mut AppState, graph: &mut GraphState, ws_cmd_tx: &WsCommandTx) {
    if let Some(current_id) = app_state.current_vertex {
        if let Some(vertex) = graph.vertices.get(&current_id).cloned() {
            if vertex.edit_mask != 0 {
                let next_vertex = vertex.edges.iter()
                    .find(|&&e| e != 0)
                    .copied();

                if let Some(ref tx) = ws_cmd_tx.0 {
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    let _ = tx.send(WsCommand::DeleteVertex { action_id, vertex_id: current_id });
                    app_state.status = "Deleting vertex...".to_string();

                    // Optimistic delete
                    graph.vertices.remove(&current_id);
                    graph.landmark_mgr.remove_vertex(current_id);
                    app_state.history.retain(|&id| id != current_id);

                    if let Some(next) = next_vertex {
                        app_state.current_vertex = Some(next);
                    } else if let Some(prev) = app_state.history.pop() {
                        app_state.current_vertex = Some(prev);
                    } else {
                        app_state.current_vertex = None;
                    }
                }
            } else {
                app_state.status = "Cannot delete: vertex is read-only".to_string();
            }
        }
    }
}

fn execute_edit_text(app_state: &mut AppState, graph: &GraphState) {
    if let Some(current_id) = app_state.current_vertex {
        if let Some(vertex) = graph.vertices.get(&current_id) {
            // Find which layer has editable text content (sorted by layer number)
            let mut layer_nums: Vec<_> = vertex.layers.keys().copied().collect();
            layer_nums.sort();

            let mut found: Option<(u32, String)> = None;
            for layer_num in layer_nums {
                if let Some(layer_data) = vertex.layers.get(&layer_num) {
                    if layer_data.mime.starts_with("text/")
                        && layer_data.mime != "text/gradesta-url"
                        && layer_data.mime != "text/x-url"
                    {
                        found = Some((layer_num, String::from_utf8_lossy(&layer_data.data).to_string()));
                        break;
                    }
                }
            }

            let Some((layer, text)) = found else {
                return; // No editable text
            };

            app_state.text_input_buffer = text;
            app_state.inline_edit_original = Some(app_state.text_input_buffer.clone());
            super::text_edit::reset_text_edit_state(app_state);
            app_state.input_mode = InputMode::InlineEdit {
                vertex_id: current_id,
                is_new: false,
                submitting: false,
                layer,
            };
            app_state.status = "Editing cell (Ctrl+Enter to save, Esc to cancel)".to_string();
        }
    }
}

fn execute_show_undo_tree(app_state: &mut AppState, graph: &mut GraphState, ws_cmd_tx: &WsCommandTx) {
    // Don't enter undo tree if already viewing it
    if app_state.viewing_undo_tree {
        return;
    }

    // Save current position before navigating to undo tree
    // Use landmark_input which contains the current landmark path
    let current_landmark = app_state.landmark_input.clone();
    let current_vertex = app_state.current_vertex;
    app_state.pre_undo_position = Some((current_landmark, current_vertex));

    // Mark that we're viewing the undo tree
    app_state.viewing_undo_tree = true;

    let undo_landmark = "gradesta://undo".to_string();

    // Send WatchLandmark for the undo tree with auto-follow
    if let Some(ref tx) = ws_cmd_tx.0 {
        let action_id = app_state.next_action_id;
        if graph.landmark_mgr.watch_and_follow(&undo_landmark, action_id, tx) {
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
        }
    }

    app_state.status = "Viewing undo history (Escape to return)".to_string();
}

fn execute_return_from_undo_tree(app_state: &mut AppState, graph: &mut GraphState, ws_cmd_tx: &WsCommandTx) {
    // Only process if we're actually in the undo tree view
    if !app_state.viewing_undo_tree {
        return;
    }

    // Restore previous position
    if let Some((landmark, vertex)) = app_state.pre_undo_position.take() {
        // Send WatchLandmark to return to previous landmark
        if let Some(ref tx) = ws_cmd_tx.0 {
            let action_id = app_state.next_action_id;
            if graph.landmark_mgr.watch_if_needed(&landmark, action_id, tx) {
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            }
        }

        // Restore current vertex (will be updated when server responds, but set optimistically)
        if let Some(vid) = vertex {
            app_state.current_vertex = Some(vid);
        }

        app_state.status = format!("Returned to {}", landmark);
    }

    // Mark that we're no longer viewing the undo tree
    app_state.viewing_undo_tree = false;
}

fn execute_start_recording(
    app_state: &mut AppState,
    audio_signal: &AudioRecordingSignal,
    playback_state: &AudioPlaybackState,
) {
    // Stop any currently playing audio before recording
    stop_audio(playback_state);

    let direction = app_state.last_nav_direction;
    let transcription_mode = app_state.transcription_mode;

    // Clear samples and reset stop signal
    if let Ok(mut samples) = app_state.audio_samples.lock() {
        samples.clear();
    }
    if let Ok(mut stop) = audio_signal.should_stop.lock() {
        *stop = false;
    }

    // For Cloud mode, set up live transcription (no voice command events)
    let live_transcript = if transcription_mode == TranscriptionMode::Cloud {
        let transcript = Arc::new(Mutex::new(String::new()));
        // Start Soniox WebSocket for audio note transcription (no voice command modal)
        let ws_audio_tx = voice_command::start_audio_note_transcription(transcript.clone());
        if ws_audio_tx.is_none() {
            eprintln!("Failed to start cloud transcription - no Soniox API key");
            None
        } else {
            Some((transcript, ws_audio_tx))
        }
    } else {
        None
    };

    // Create placeholder cell immediately when recording starts
    if let Some(current_id) = app_state.current_vertex {
        let local_id = app_state.next_local_id;
        app_state.next_local_id = app_state.next_local_id.wrapping_sub(1);

        let pending_cell = PendingCell {
            local_id,
            direction,
            from_vertex: current_id,
            created_at: Instant::now(),
            server_vertex_id: None,
            action_id: None,
            kind: PendingCellKind::Audio {
                status: PendingAudioStatus::Recording,
                waveform: Vec::new(),
                current_audio_level: 0.0,
                live_transcript: live_transcript.as_ref().map(|(t, _)| t.clone()),
            },
        };

        app_state.pending_audio_cells.insert(local_id, pending_cell);
        app_state.recording_placeholder_id = Some(local_id);
        eprintln!(
            "Created recording placeholder: local_id={} direction={} from_vertex={} cloud={}",
            local_id, direction, current_id, live_transcript.is_some()
        );
    }

    // Start audio recording in a separate thread
    let samples_clone = app_state.audio_samples.clone();
    let stop_signal = audio_signal.should_stop.clone();
    let sample_rate_out = audio_signal.actual_sample_rate.clone();

    if let Some((_, ws_audio_tx)) = live_transcript {
        // Cloud mode: stream audio to WebSocket for live transcription
        let audio_level = Arc::new(Mutex::new(0.0f32));
        thread::spawn(move || {
            if let Err(e) = run_voice_recording_with_streaming(
                samples_clone,
                stop_signal,
                sample_rate_out,
                ws_audio_tx,
                audio_level,
            ) {
                eprintln!("Audio recording error: {}", e);
            }
        });
    } else {
        // Local or Off mode: standard recording
        thread::spawn(move || {
            if let Err(e) = run_audio_recording(samples_clone, stop_signal, sample_rate_out) {
                eprintln!("Audio recording error: {}", e);
            }
        });
    }

    app_state.recording_start = Some(Instant::now());
    app_state.input_mode = InputMode::Recording { direction };
    let mode_label = match transcription_mode {
        TranscriptionMode::Off => "(no transcription)",
        TranscriptionMode::Local => "(Whisper after)",
        TranscriptionMode::Cloud => "(live transcription)",
    };
    app_state.status = format!("🔴 Recording {} (release key to save)", mode_label);
}

// ============================================================================
// Voice Command Functions
// ============================================================================

/// Start voice command recording with real-time transcription (L2+R2 held)
fn start_voice_command(
    app_state: &mut AppState,
    _playback_state: &AudioPlaybackState,
    voice_channel: &VoiceCommandChannel,
) {
    // Don't stop audio here - wait until 1 second to allow burst cancel
    // Audio will be stopped in execute_commands after VOICE_COMMAND_MIN_DURATION

    // Create shared state for recording
    let samples = Arc::new(Mutex::new(Vec::new()));
    let stop_signal = Arc::new(Mutex::new(false));
    let live_transcript = Arc::new(Mutex::new(String::new()));
    let audio_level = Arc::new(Mutex::new(0.0f32));

    // Start real-time transcription WebSocket
    let ws_audio_tx = voice_command::start_realtime_transcription(
        live_transcript.clone(),
        voice_channel.tx.clone(),
    );

    if ws_audio_tx.is_none() {
        app_state.status = "No Soniox API key found. Add key to ~/.config/gradesta/elves/simple-llm/soniox.com/secret.key".to_string();
        return;
    }

    let ws_audio_tx_for_thread = ws_audio_tx.clone();
    let audio_level_for_thread = audio_level.clone();

    // Start recording in background thread, streaming to WebSocket
    let samples_clone = samples.clone();
    let stop_clone = stop_signal.clone();
    thread::spawn(move || {
        // Record audio and stream chunks to WebSocket for real-time transcription
        let sample_rate_out = Arc::new(Mutex::new(16000u32)); // Target sample rate for speech
        if let Err(e) = run_voice_recording_with_streaming(
            samples_clone,
            stop_clone,
            sample_rate_out,
            ws_audio_tx_for_thread,
            audio_level_for_thread,
        ) {
            eprintln!("Voice command recording error: {}", e);
        }
    });

    app_state.recording_start = Some(Instant::now());
    app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Recording {
        samples,
        stop_signal,
        live_transcript,
        ws_audio_tx,
        audio_level,
    });
    app_state.status = "🎤 Voice command... (release triggers to stop)".to_string();
}

/// Record audio and stream chunks to WebSocket for real-time transcription
fn run_voice_recording_with_streaming(
    samples: Arc<Mutex<Vec<f32>>>,
    stop_signal: Arc<Mutex<bool>>,
    sample_rate_out: Arc<Mutex<u32>>,
    ws_audio_tx: Option<crossbeam_channel::Sender<Vec<u8>>>,
    audio_level: Arc<Mutex<f32>>,
) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or("No input device available")?;

    // Request 16kHz mono for speech recognition
    let config = cpal::StreamConfig {
        channels: 1,
        sample_rate: cpal::SampleRate(16000),
        buffer_size: cpal::BufferSize::Default,
    };

    if let Ok(mut sr) = sample_rate_out.lock() {
        *sr = 16000;
    }

    let samples_for_callback = samples.clone();
    let stop_for_callback = stop_signal.clone();
    let ws_tx = ws_audio_tx.clone();
    let audio_level_for_callback = audio_level.clone();

    // Buffer for accumulating samples before sending (send every ~100ms = 1600 samples at 16kHz)
    let chunk_buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(1600)));
    let chunk_buffer_for_callback = chunk_buffer.clone();

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Calculate RMS audio level for the meter
            if !data.is_empty() {
                let sum_sq: f32 = data.iter().map(|&s| s * s).sum();
                let rms = (sum_sq / data.len() as f32).sqrt();
                // Scale to 0-1 range (typical speech is around 0.1-0.3 RMS)
                let level = (rms * 5.0).min(1.0);
                if let Ok(mut lvl) = audio_level_for_callback.lock() {
                    // Smooth the level a bit
                    *lvl = *lvl * 0.7 + level * 0.3;
                }
            }

            // Accumulate samples
            if let Ok(mut s) = samples_for_callback.lock() {
                s.extend_from_slice(data);
            }

            // Check if we should stop - but only AFTER accumulating samples
            // This ensures we don't lose any audio data
            if let Ok(stop) = stop_for_callback.lock() {
                if *stop {
                    // Still need to buffer for WebSocket even when stopping
                    if let Ok(mut buf) = chunk_buffer_for_callback.lock() {
                        buf.extend_from_slice(data);
                    }
                    return;
                }
            }

            // Buffer for WebSocket streaming
            if let Ok(mut buf) = chunk_buffer_for_callback.lock() {
                buf.extend_from_slice(data);

                // Send chunk when we have enough samples (~100ms of audio)
                if buf.len() >= 1600 {
                    if let Some(ref tx) = ws_tx {
                        // Convert f32 samples to s16le bytes
                        let pcm: Vec<u8> = buf.iter()
                            .flat_map(|&s| {
                                let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                                sample.to_le_bytes()
                            })
                            .collect();
                        let _ = tx.send(pcm);
                    }
                    buf.clear();
                }
            }
        },
        |err| {
            eprintln!("Audio stream error: {}", err);
        },
        None,
    ).map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream.play().map_err(|e| format!("Failed to start recording: {}", e))?;

    // Wait for stop signal
    loop {
        if let Ok(stop) = stop_signal.lock() {
            if *stop {
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Drop the stream FIRST to stop new audio callbacks
    drop(stream);

    // Small delay to ensure any in-flight callbacks complete
    thread::sleep(Duration::from_millis(20));

    // Now send any remaining buffered samples
    if let Some(ref tx) = ws_audio_tx {
        if let Ok(buf) = chunk_buffer.lock() {
            if !buf.is_empty() {
                eprintln!("Sending {} remaining audio samples to transcription", buf.len());
                let pcm: Vec<u8> = buf.iter()
                    .flat_map(|&s| {
                        let sample = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
                        sample.to_le_bytes()
                    })
                    .collect();
                let _ = tx.send(pcm);
            }
        }
    }

    Ok(())
}

/// Finalize voice command recording - signals stop and waits for WebSocket to complete
pub fn finalize_voice_recording(
    app_state: &mut AppState,
    _voice_channel: &VoiceCommandChannel,
    _voice_config: &voice_command::VoiceCommandConfig,
) {
    if let InputMode::VoiceCommand(VoiceCommandState::Recording {
        ref samples,
        ref stop_signal,
        ref live_transcript,
        ..
    }) = app_state.input_mode {
        // Signal recording to stop - this will close the audio channel
        // which will cause the WebSocket thread to finish and send TranscriptionComplete
        if let Ok(mut stop) = stop_signal.lock() {
            *stop = true;
        }

        // Get the current live transcript
        let current_transcript = if let Ok(t) = live_transcript.lock() {
            t.clone()
        } else {
            String::new()
        };

        // Small delay to let recording thread finish
        thread::sleep(Duration::from_millis(100));

        // Get recorded samples count for logging
        let sample_count = if let Ok(s) = samples.lock() {
            s.len()
        } else {
            0
        };

        eprintln!("Voice recording finished: {} samples, transcript so far: \"{}\"", sample_count, current_transcript);

        if sample_count < 1000 {
            // Too short
            app_state.input_mode = InputMode::Normal;
            app_state.status = "Voice command too short".to_string();
            return;
        }

        // Transition to Transcribing state while WebSocket finalizes
        // The TranscriptionComplete event will arrive shortly via the channel
        app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Transcribing);
        app_state.status = format!("Finalizing: \"{}\"", current_transcript);
    }
}

/// Process voice command events from the channel
pub fn process_voice_command_events(
    app_state: &mut AppState,
    graph: &GraphState,
    voice_channel: &VoiceCommandChannel,
    voice_config: &voice_command::VoiceCommandConfig,
) {
    while let Ok(event) = voice_channel.rx.try_recv() {
        match event {
            VoiceCommandEvent::TranscriptionComplete { transcript } => {
                eprintln!("Transcription complete: \"{}\"", transcript);

                if transcript.is_empty() {
                    app_state.input_mode = InputMode::Normal;
                    app_state.status = "No speech detected".to_string();
                    continue;
                }

                // Start LLM interpretation
                app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Interpreting {
                    transcript: transcript.clone(),
                    cell_context: None,
                });
                app_state.status = "Interpreting command...".to_string();

                // Query LLM
                if let Some(api_key) = voice_command::load_api_key() {
                    let context = "Normal";
                    let mime_type = get_current_cell_mime(app_state, graph);
                    let direction = direction_name(app_state.last_nav_direction);

                    voice_command::query_llm(
                        &transcript,
                        context,
                        &mime_type,
                        direction,
                        None,
                        &api_key,
                        voice_config,
                        voice_channel.tx.clone(),
                    );
                } else {
                    app_state.input_mode = InputMode::Normal;
                    app_state.status = "API key not found. Place key at ~/.config/gradesta/elves/simple-llm/requesty.ai/secret.key".to_string();
                }
            }

            VoiceCommandEvent::TranscriptionFailed { error } => {
                eprintln!("Transcription failed: {}", error);
                // Only exit recording if we're not still holding the trigger
                // If still in Recording state, just update status but stay in that state
                // so the user can see what's happening
                if !matches!(app_state.input_mode, InputMode::VoiceCommand(VoiceCommandState::Recording { .. })) {
                    app_state.input_mode = InputMode::Normal;
                }
                app_state.status = format!("Transcription failed: {}", error);
            }

            VoiceCommandEvent::TranscriptUpdate { text, is_final: _ } => {
                // Real-time transcript updates are handled via the Arc<Mutex<String>> in the state
                // This event is just for logging/debugging
                eprintln!("Live transcript update: \"{}\"", text);
            }

            VoiceCommandEvent::LlmResponse { mut interpretations, generated_image } => {
                eprintln!("LLM response: {} interpretations", interpretations.len());

                // Store generated image if present
                if let Some(img) = generated_image {
                    eprintln!("Storing generated image: {} bytes, {}", img.data.len(), img.mime);
                    app_state.generated_image_buffer = Some(crate::state::GeneratedImage {
                        mime: img.mime,
                        data: img.data,
                    });
                }

                // Get the transcript from current state
                let transcript = if let InputMode::VoiceCommand(VoiceCommandState::Interpreting { ref transcript, .. }) = app_state.input_mode {
                    transcript.clone()
                } else {
                    "".to_string()
                };

                // Always add Cancel as the last option
                interpretations.push(AgentInterpretation {
                    action: AgentAction::Cancel,
                    confidence: 1.0,
                    explanation: "Cancel voice command".to_string(),
                });

                // Show selection menu
                app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Selecting {
                    transcript,
                    interpretations,
                    selected: 0,
                });
                app_state.status = "Select action with R3/A, navigate with right stick".to_string();
            }

            VoiceCommandEvent::LlmRequestsView { targets, reason } => {
                eprintln!("LLM requests view: {:?} - {}", targets, reason);

                // Get the transcript from current state
                let transcript = if let InputMode::VoiceCommand(VoiceCommandState::Interpreting { ref transcript, .. }) = app_state.input_mode {
                    transcript.clone()
                } else {
                    "".to_string()
                };

                // Show permission prompt (default selection: Allow)
                app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::AwaitingPermission {
                    transcript,
                    requested_targets: targets,
                    reason,
                    selected: 0, // Default to Allow button
                });
                app_state.status = "Agent requests permission".to_string();
            }

            VoiceCommandEvent::LlmFailed { error } => {
                eprintln!("LLM failed: {}", error);
                app_state.input_mode = InputMode::Normal;
                app_state.status = format!("Interpretation failed: {}", error);
            }

            // Command bar LLM events
            VoiceCommandEvent::CommandBarLlmResponse { interpretations } => {
                eprintln!("Command bar LLM response: {} interpretations", interpretations.len());
                app_state.command_bar_llm_pending = false;
                app_state.command_bar_interpretations = interpretations;
                app_state.command_bar_interpretation_selected = 0;
            }
            VoiceCommandEvent::CommandBarLlmFailed { error } => {
                eprintln!("Command bar LLM failed: {}", error);
                app_state.command_bar_llm_pending = false;
                app_state.status = format!("LLM error: {}", error);
            }
        }
    }
}

/// Grant permission for LLM to view cell content and re-query
pub fn grant_voice_permission(
    app_state: &mut AppState,
    graph: &GraphState,
    voice_channel: &VoiceCommandChannel,
    voice_config: &voice_command::VoiceCommandConfig,
) {
    if let InputMode::VoiceCommand(VoiceCommandState::AwaitingPermission {
        ref transcript,
        ref requested_targets,
        ..
    }) = app_state.input_mode
    {
        // Build cell context
        let mut cell_context = CellContext::default();

        for target in requested_targets {
            let content = match target.as_str() {
                "current" => get_cell_content_at(app_state, graph, None),
                "north" => get_cell_content_at(app_state, graph, Some(EDGE_NORTH)),
                "south" => get_cell_content_at(app_state, graph, Some(EDGE_SOUTH)),
                "east" => get_cell_content_at(app_state, graph, Some(EDGE_EAST)),
                "west" => get_cell_content_at(app_state, graph, Some(EDGE_WEST)),
                "up" => get_cell_content_at(app_state, graph, Some(EDGE_UP)),
                "down" => get_cell_content_at(app_state, graph, Some(EDGE_DOWN)),
                _ => None,
            };

            if target == "current" {
                cell_context.current = content;
            } else if let Some(c) = content {
                cell_context.directions.insert(target.clone(), c);
            }
        }

        let transcript = transcript.clone();

        // Transition back to interpreting with context
        app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Interpreting {
            transcript: transcript.clone(),
            cell_context: Some(cell_context.clone()),
        });
        app_state.status = "Re-interpreting with cell data...".to_string();

        // Re-query LLM with cell context
        if let Some(api_key) = voice_command::load_api_key() {
            let context = "Normal";
            let mime_type = get_current_cell_mime(app_state, graph);
            let direction = direction_name(app_state.last_nav_direction);

            voice_command::query_llm(
                &transcript,
                context,
                &mime_type,
                direction,
                Some(&cell_context),
                &api_key,
                voice_config,
                voice_channel.tx.clone(),
            );
        }
    }
}

/// Execute the selected voice command action
/// Uses the same execute_commands path as keybindings for consistency
pub fn execute_voice_action(
    app_state: &mut AppState,
    graph: &mut GraphState,
    ws_cmd_tx: &WsCommandTx,
    media_cache: &mut MediaCache,
    audio_signal: &AudioRecordingSignal,
    playback_state: &AudioPlaybackState,
    boost_state: &mut PlaybackBoostState,
    voice_channel: &VoiceCommandChannel,
    ctx: &bevy_egui::egui::Context,
) {
    if let InputMode::VoiceCommand(VoiceCommandState::Selecting {
        ref interpretations,
        selected,
        ..
    }) = app_state.input_mode
    {
        if let Some(interp) = interpretations.get(selected) {
            let action = interp.action.clone();
            let explanation = interp.explanation.clone();

            match action {
                AgentAction::Script(script) => {
                    let instructions = parse_script(&script);

                    // Exit voice command mode BEFORE executing commands
                    // Commands check for InputMode::Normal, so we need to set it first
                    app_state.input_mode = InputMode::Normal;

                    for instruction in instructions {
                        match instruction {
                            ScriptInstruction::Command(cmd) => {
                                // Create a CapturedCommands with just this command
                                let mut cmds = super::input::CapturedCommands::default();
                                cmds.add(cmd);
                                // Execute through the standard command path
                                execute_commands(
                                    &cmds,
                                    app_state,
                                    graph,
                                    ws_cmd_tx,
                                    media_cache,
                                    audio_signal,
                                    playback_state,
                                    boost_state,
                                    voice_channel,
                                    ctx,
                                );
                            }
                            ScriptInstruction::InsertText(content) => {
                                // If URL bar is focused/pending focus, set server input and refresh
                                // Otherwise set text input buffer and auto-submit
                                if app_state.focus_url_bar_next_frame || app_state.url_bar_has_focus {
                                    app_state.server_input = content;
                                    // Auto-trigger refresh for URL bar
                                    app_state.voice_refresh_pending = true;
                                } else {
                                    // Set the text buffer
                                    app_state.text_input_buffer = content;
                                    super::text_edit::reset_text_edit_state(app_state);
                                    // Auto-submit by executing TextInputSubmit command
                                    let mut cmds = super::input::CapturedCommands::default();
                                    cmds.add(Command::TextInputSubmit);
                                    execute_commands(
                                        &cmds,
                                        app_state,
                                        graph,
                                        ws_cmd_tx,
                                        media_cache,
                                        audio_signal,
                                        playback_state,
                                        boost_state,
                                        voice_channel,
                                        ctx,
                                    );
                                }
                            }
                            ScriptInstruction::InsertGeneratedImage => {
                                // Create a new cell with the generated image
                                if let Some(ref img) = app_state.generated_image_buffer {
                                    if let Some(current_id) = app_state.current_vertex {
                                        if let Some(ref tx) = ws_cmd_tx.0 {
                                            let direction = app_state.last_nav_direction;
                                            let dir_byte = match direction {
                                                EDGE_WEST => 0,
                                                EDGE_EAST => 1,
                                                EDGE_NORTH => 2,
                                                EDGE_SOUTH => 3,
                                                EDGE_UP => 4,
                                                EDGE_DOWN => 5,
                                                _ => 3, // default south
                                            };
                                            let action_id = app_state.next_action_id;
                                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                                            let mime = img.mime.clone();
                                            let data = img.data.clone();

                                            eprintln!("Creating image vertex: {} bytes, {}", data.len(), mime);

                                            let _ = tx.send(WsCommand::CreateVertex {
                                                action_id,
                                                from_vertex: current_id,
                                                direction: dir_byte,
                                                layer: 0,
                                                mime: mime.clone(),
                                                data: data.clone(),
                                            });

                                            app_state.pending_creations.insert(action_id, PendingVertexCreation {
                                                samples: Vec::new(),
                                                sample_rate: 0,
                                                data,
                                                mime,
                                                local_placeholder_id: None,
                                                transcription_mode: TranscriptionMode::Off,
                                                cloud_transcript: None,
                                            });

                                            app_state.status = format!("Creating image cell {}...", direction_name(direction));
                                        }
                                    }
                                    // Clear the buffer after use
                                    app_state.generated_image_buffer = None;
                                } else {
                                    app_state.status = "No generated image in buffer".to_string();
                                }
                            }
                        }
                    }

                    app_state.status = format!("Executed: {}", explanation);

                    // Only return to Normal if not in a text input mode
                    if !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
                        app_state.input_mode = InputMode::Normal;
                    }
                }

                AgentAction::Cancel => {
                    app_state.status = "Voice command cancelled".to_string();
                    app_state.input_mode = InputMode::Normal;
                }
            }
        }
    }
}


fn get_current_cell_mime(app_state: &AppState, graph: &GraphState) -> String {
    // Get the first mime type from any layer
    app_state
        .current_vertex
        .and_then(|id| graph.vertices.get(&id))
        .and_then(|v| v.layers.values().next())
        .map(|l| l.mime.clone())
        .unwrap_or_else(|| "text/plain".to_string())
}

fn get_cell_content_at(app_state: &AppState, graph: &GraphState, direction: Option<usize>) -> Option<String> {
    let vertex_id = if let Some(dir) = direction {
        app_state
            .current_vertex
            .and_then(|id| graph.vertices.get(&id))
            .map(|v| v.edges[dir])
            .filter(|&id| id != 0)?
    } else {
        app_state.current_vertex?
    };

    let vertex = graph.vertices.get(&vertex_id)?;

    // Find text content in any layer (excluding portals)
    for layer in vertex.layers.values() {
        if layer.mime.starts_with("text/") && layer.mime != "text/gradesta-url" {
            return Some(String::from_utf8_lossy(&layer.data).to_string());
        }
    }
    None
}

