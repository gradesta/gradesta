//! Command execution logic for UI system
//!
//! Executes captured keyboard commands and returns the results.

use std::thread;
use std::time::{Duration, Instant};

use crate::audio::{encode_ogg_vorbis, run_audio_recording, stop_audio, AudioPlaybackState, AudioRecordingSignal};
use crate::graph::GraphState;
use crate::media::MediaCache;
use crate::network::{WsCommand, WsCommandTx};
use crate::sidebar::SidebarMode;
use crate::state::{AppState, InputMode, PendingVertexCreation};
use crate::state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};
use crate::state::{ZOOM_MAX, ZOOM_MIN, ZOOM_STEP};
use crate::tts;

use super::input::CapturedCommands;

/// Results from command execution
#[derive(Clone, Debug, Default)]
pub struct CommandResults {
    /// Whether any command was processed
    pub any_command_processed: bool,
    /// Whether we should finalize recording this frame
    pub should_finalize_recording: bool,
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
    ctx: &bevy_egui::egui::Context,
) -> CommandResults {
    let mut results = CommandResults::default();

    // GlobalCloseModal - close modals or exit fullscreen
    if cmds.close_modal {
        results.any_command_processed = true;
        // First priority: close command bar
        if app_state.show_command_bar {
            app_state.show_command_bar = false;
            app_state.command_bar_input.clear();
            app_state.command_bar_selected = 0;
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
    if cmds.toggle_fullscreen && !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        results.any_command_processed = true;
        execute_toggle_fullscreen(app_state, graph);
    }

    // GraphClickVertex - "click" the current vertex (send click message to server)
    if cmds.click_vertex && matches!(app_state.input_mode, InputMode::Normal) {
        results.any_command_processed = true;
        execute_click_vertex(app_state, ws_cmd_tx);
    }

    // Zoom commands
    if cmds.zoom_in {
        results.any_command_processed = true;
        app_state.zoom_level = (app_state.zoom_level + ZOOM_STEP).min(ZOOM_MAX);
    }
    if cmds.zoom_out {
        results.any_command_processed = true;
        app_state.zoom_level = (app_state.zoom_level - ZOOM_STEP).max(ZOOM_MIN);
    }
    if cmds.zoom_reset {
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
    if cmds.toggle_bag {
        results.any_command_processed = true;
        app_state.show_bag_panel = !app_state.show_bag_panel;
        if app_state.show_bag_panel {
            app_state.show_nav_panel = false;
        }
    }

    // GlobalToggleNavPanel
    if cmds.toggle_nav_panel {
        results.any_command_processed = true;
        app_state.show_nav_panel = !app_state.show_nav_panel;
        if app_state.show_nav_panel {
            app_state.show_bag_panel = false;
        }
    }

    // GraphYank - Yank (copy) current vertex to bag
    if cmds.yank {
        results.any_command_processed = true;
        if let Some(current_id) = app_state.current_vertex {
            if app_state.bag.last() != Some(&current_id) {
                app_state.bag.push(current_id);
                app_state.status = format!("Yanked vertex to bag (depth: {})", app_state.bag.len());
            }
        }
    }

    // BagPop - Pop from bag (remove top without connecting)
    if cmds.bag_pop {
        results.any_command_processed = true;
        if let Some(_popped) = app_state.bag.pop() {
            app_state.status = format!("Popped from bag (depth: {})", app_state.bag.len());
        } else {
            app_state.status = "Bag is empty".to_string();
        }
    }

    // GraphGoToBagTop - Go to top of bag (jump to that vertex)
    if cmds.go_to_bag_top && app_state.input_mode == InputMode::Normal {
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
    if cmds.paste && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_paste(app_state, graph, ws_cmd_tx);
    }

    // GraphCutEdge - Cut connection in the current navigation direction
    if cmds.cut_edge && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_cut_edge(app_state, graph, ws_cmd_tx);
    }

    // GraphDeleteVertex - Delete current vertex (if editable)
    if cmds.delete_vertex && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_delete_vertex(app_state, graph, ws_cmd_tx);
    }

    // GraphEditText - Insert text at current vertex (edit)
    if cmds.edit_text && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_edit_text(app_state, graph);
    }

    // GraphSetDirection* - Change last navigation direction without moving
    if app_state.input_mode == InputMode::Normal {
        let dir = if cmds.set_dir_north {
            Some(EDGE_NORTH)
        } else if cmds.set_dir_south {
            Some(EDGE_SOUTH)
        } else if cmds.set_dir_west {
            Some(EDGE_WEST)
        } else if cmds.set_dir_east {
            Some(EDGE_EAST)
        } else if cmds.set_dir_up {
            Some(EDGE_UP)
        } else if cmds.set_dir_down {
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

    // GraphNewTextVertex - Create new text vertex in last navigation direction
    if cmds.new_text_vertex && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        let direction = app_state.last_nav_direction;
        app_state.text_input_buffer.clear();
        super::text_edit::reset_text_edit_state(app_state);
        app_state.input_mode = InputMode::TextInput { direction: Some(direction) };
        app_state.status = format!("Text input mode (new vertex {})", direction_name(direction));
    }

    // GraphStartRecording - Push-to-talk recording
    if cmds.start_recording && app_state.input_mode == InputMode::Normal && app_state.connected {
        results.any_command_processed = true;
        execute_start_recording(app_state, audio_signal, playback_state);
    }

    // RecordingSave - Check for recording key release (push-to-talk stop)
    if let InputMode::Recording { .. } = &app_state.input_mode {
        if cmds.recording_save {
            results.any_command_processed = true;
            if let Ok(mut stop) = audio_signal.should_stop.lock() {
                *stop = true;
            }
        }
    }

    // GlobalOpenCommandBar - Open command bar (vim-style)
    if cmds.open_command_bar && app_state.input_mode == InputMode::Normal && !app_state.show_command_bar {
        results.any_command_processed = true;
        app_state.show_command_bar = true;
        app_state.command_bar_input.clear();
        app_state.command_bar_selected = 0;
    }

    // GlobalOpenKeybindings - Open keybindings editor
    if cmds.open_keybindings {
        results.any_command_processed = true;
        app_state.sidebar.mode = SidebarMode::Keybindings;
    }

    // GlobalToggleTTS - Toggle text-to-speech mode
    if cmds.toggle_tts {
        results.any_command_processed = true;
        app_state.tts_mode = !app_state.tts_mode;
        if app_state.tts_mode {
            app_state.status = "TTS mode enabled (Ctrl+T to disable)".to_string();
        } else {
            tts::stop();
            app_state.status = "TTS mode disabled".to_string();
        }
    }

    // Handle URL focus key
    let url_bar_id = bevy_egui::egui::Id::new("url_bar");
    if cmds.focus_url_down {
        app_state.focus_url_bar_next_frame = true;
    } else if app_state.focus_url_bar_next_frame && cmds.focus_url_released {
        app_state.focus_url_bar_next_frame = false;
        ctx.memory_mut(|mem| mem.request_focus(url_bar_id));
    }

    // GlobalCopyUrl - but not in TextInput mode (let egui handle Ctrl+C for text copy)
    if cmds.copy_url && !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        results.any_command_processed = true;
        ctx.copy_text(app_state.url_input.clone());
        app_state.status = "Copied URL to clipboard".to_string();
    }

    // Check if we should finalize recording (space was released)
    results.should_finalize_recording = if let InputMode::Recording { .. } = &app_state.input_mode {
        audio_signal.should_stop.lock().map(|s| *s).unwrap_or(false)
    } else {
        false
    };

    results
}

/// Finalize recording after space key was released
pub fn finalize_recording(
    app_state: &mut AppState,
    audio_signal: &AudioRecordingSignal,
    ws_cmd_tx: &WsCommandTx,
) {
    // Signal to stop recording
    if let Ok(mut stop) = audio_signal.should_stop.lock() {
        *stop = true;
    }
    // Small delay to let the recording thread notice
    thread::sleep(Duration::from_millis(50));

    if let InputMode::Recording { direction } = app_state.input_mode.clone() {
        // Get samples and encode
        let samples = if let Ok(s) = app_state.audio_samples.lock() {
            s.clone()
        } else {
            Vec::new()
        };

        // Get actual sample rate from recording
        let sample_rate = audio_signal.actual_sample_rate.lock()
            .map(|sr| *sr)
            .unwrap_or(44100);

        eprintln!("Recording finished: {} samples at {} Hz", samples.len(), sample_rate);

        if samples.len() > 1000 { // At least some audio
            let duration_secs = samples.len() as f32 / sample_rate as f32;

            let current_id = app_state.current_vertex;
            let dir_byte = match direction {
                EDGE_WEST => 0,
                EDGE_EAST => 1,
                EDGE_NORTH => 2,
                EDGE_SOUTH => 3,
                EDGE_UP => 4,
                EDGE_DOWN => 5,
                _ => 3, // default south
            };

            // Allocate action_id for the CreateVertex
            let action_id = app_state.next_action_id;
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

            // Encode audio immediately (no transcript in audio - that goes to layer 1)
            match encode_ogg_vorbis(&samples, sample_rate, None) {
                Ok(audio_data) => {
                    // Send audio to server immediately (layer 0)
                    if let (Some(current_id), Some(ref tx)) = (current_id, &ws_cmd_tx.0) {
                        let _ = tx.send(WsCommand::CreateVertex {
                            action_id,
                            from_vertex: current_id,
                            direction: dir_byte,
                            layer: 0,
                            mime: "audio/ogg".to_string(),
                            data: audio_data.clone(),
                        });

                        // Store samples and encoded data - will be processed when we get the 200 response
                        app_state.pending_creations.insert(action_id, PendingVertexCreation {
                            samples: samples.clone(),
                            sample_rate,
                            data: audio_data,
                            mime: "audio/ogg".to_string(),
                        });
                    }
                    app_state.status = format!("Saving audio ({:.1}s)...", duration_secs);
                }
                Err(e) => {
                    eprintln!("Failed to encode audio: {}", e);
                    app_state.status = format!("Audio encode failed: {}", e);
                }
            }
        } else {
            app_state.status = "Recording too short (hold Space longer)".to_string();
        }

        app_state.input_mode = InputMode::Normal;
        app_state.recording_start = None;
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
            let mime = vertex.mime.as_deref().unwrap_or("");
            let primary_is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
            let primary_is_text = mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url";

            let has_layer_image = vertex.layers.values()
                .any(|l| l.mime.starts_with("image/") || is_image_data(&l.data));

            if primary_is_text && !has_layer_image {
                app_state.text_modal_content = String::from_utf8_lossy(&vertex.label).to_string();
                app_state.show_text_modal = true;
                app_state.sidebar.fullscreen = true;
            } else if primary_is_image || has_layer_image {
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
                    for vertices in graph.landmark_vertices.values_mut() {
                        vertices.retain(|&id| id != current_id);
                    }
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
            let mime = vertex.mime.as_deref().unwrap_or("");
            if let Some(layer1) = vertex.layers.get(&1) {
                if layer1.mime.starts_with("text/") {
                    app_state.text_input_buffer = String::from_utf8_lossy(&layer1.data).to_string();
                } else {
                    app_state.text_input_buffer.clear();
                }
            } else if mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url" {
                app_state.text_input_buffer = String::from_utf8_lossy(&vertex.label).to_string();
            } else {
                app_state.text_input_buffer.clear();
            }
        }
        super::text_edit::reset_text_edit_state(app_state);
        app_state.input_mode = InputMode::TextInput { direction: None };
        app_state.status = "Text input mode (editing current vertex)".to_string();
    }
}

fn execute_start_recording(
    app_state: &mut AppState,
    audio_signal: &AudioRecordingSignal,
    playback_state: &AudioPlaybackState,
) {
    // Stop any currently playing audio before recording
    stop_audio(playback_state);

    let direction = app_state.last_nav_direction;

    // Clear samples and reset stop signal
    if let Ok(mut samples) = app_state.audio_samples.lock() {
        samples.clear();
    }
    if let Ok(mut stop) = audio_signal.should_stop.lock() {
        *stop = false;
    }

    // Start audio recording in a separate thread
    let samples_clone = app_state.audio_samples.clone();
    let stop_signal = audio_signal.should_stop.clone();
    let sample_rate_out = audio_signal.actual_sample_rate.clone();
    thread::spawn(move || {
        if let Err(e) = run_audio_recording(samples_clone, stop_signal, sample_rate_out) {
            eprintln!("Audio recording error: {}", e);
        }
    });

    app_state.recording_start = Some(Instant::now());
    app_state.input_mode = InputMode::Recording { direction };
    app_state.status = "🔴 Recording... (release key to save)".to_string();
}
