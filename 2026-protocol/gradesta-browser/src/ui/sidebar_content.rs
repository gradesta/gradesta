//! Sidebar content rendering for the right panel
//!
//! Renders the appropriate content based on current app state and sidebar mode.

use std::time::{Duration, Instant};

use bevy_egui::egui;

use crate::audio::AudioPlaybackState;
use crate::graph::{find_islands, GraphState};
use crate::keybindings;
use crate::media::{get_or_load_animated_gif, get_or_load_texture, MediaCache};
use crate::network::WsCommandTx;
use crate::rendering::{render_vertex_card, render_vertex_content};
use crate::sidebar;
use crate::state::{AppState, InputMode, EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};

/// Action returned from sidebar content rendering
#[derive(Clone, Debug)]
pub enum SidebarContentAction {
    None,
    CloseTextModal,
    CloseImageModal,
    CloseVideoModal,
    StopVideo { vertex_id: u64 },
    CancelNextcloudLogin,
    CancelIdentitySetup,
    CreateIdentity {
        nextcloud_url: String,
        username: String,
        app_password: String,
        display_name: String,
    },
    CloseIdentityPanel,
    RemoveIdentity(usize),
    InitiateNextcloudLogin(String),
    CancelTextInput,
    CloseNavPanel,
    JumpToVertex(u64),
    WatchLandmark(String),
    CloseBagPanel,
    ClearBag,
    RemoveFromBag(usize),
    CloseKeybindings,
    SaveKeybindings,
    ApplyPreset(keybindings::Preset),
}

/// Render the sidebar content panel
pub fn render_sidebar_content(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
    ws_cmd_tx: &WsCommandTx,
    playback_state: &AudioPlaybackState,
) -> SidebarContentAction {
    // Render content based on current mode
    if app_state.show_text_modal {
        render_text_modal(ui, app_state)
    } else if app_state.show_image_modal {
        render_image_modal(ui, ctx, app_state, graph, media_cache)
    } else if app_state.show_video_modal {
        render_video_modal(ui, ctx, app_state, media_cache)
    } else if app_state.nextcloud_login_state.is_some() {
        render_nextcloud_login(ui, ctx, app_state)
    } else if app_state.pending_identity_setup.is_some() {
        render_identity_setup(ui, app_state)
    } else if app_state.show_identity_panel {
        render_identity_panel(ui, app_state)
    } else if let InputMode::TextInput { direction } = app_state.input_mode.clone() {
        render_text_input_mode(ui, app_state, direction)
    } else if let InputMode::Recording { .. } = app_state.input_mode.clone() {
        render_recording_mode(ui, ctx, app_state)
    } else if app_state.pending_identification.is_some() {
        render_identification_request(ui, app_state)
    } else if app_state.show_nav_panel {
        render_nav_panel(ui, ctx, app_state, graph, media_cache, ws_cmd_tx)
    } else if app_state.show_bag_panel {
        render_bag_panel(ui, ctx, app_state, graph, media_cache)
    } else if matches!(app_state.sidebar.mode, sidebar::SidebarMode::Keybindings) {
        render_keybindings_mode(ui, app_state)
    } else {
        render_preview_mode(ui, ctx, app_state, graph, media_cache, playback_state)
    }
}

fn render_text_modal(ui: &mut egui::Ui, app_state: &AppState) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.horizontal(|ui| {
        ui.heading("Text Content");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = SidebarContentAction::CloseTextModal;
            }
        });
    });
    ui.separator();
    ui.label("Ctrl+Enter: Fullscreen | Escape: Close");
    ui.separator();

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app_state.text_modal_content.clone())
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false)
            );
        });

    action
}

fn render_image_modal(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.horizontal(|ui| {
        ui.heading("Image");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = SidebarContentAction::CloseImageModal;
            }
        });
    });
    ui.separator();
    ui.label("Ctrl+Enter: Fullscreen | Escape: Close");
    ui.separator();

    if let Some(vertex_id) = app_state.image_modal_vertex_id {
        if let Some(vertex) = graph.vertices.get(&vertex_id) {
            // Prefer layer 2 content (full image) over layer 0 (thumbnail/label)
            let (image_data, mime): (&[u8], &str) = if let Some(layer2) = vertex.layers.get(&2) {
                (&layer2.data, &layer2.mime)
            } else {
                (&vertex.label, vertex.mime.as_deref().unwrap_or(""))
            };

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if mime == "image/gif" {
                        if let Some(animated) = get_or_load_animated_gif(vertex_id, image_data, media_cache, ctx) {
                            let now = Instant::now();
                            if now.duration_since(animated.last_switch) >= animated.delays[animated.current_frame] {
                                animated.current_frame = (animated.current_frame + 1) % animated.frames.len();
                                animated.last_switch = now;
                            }
                            let tex = &animated.frames[animated.current_frame];
                            let size = tex.size_vec2();
                            let available = ui.available_size();
                            let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                            ui.image((tex.id(), size * scale));
                            ctx.request_repaint();
                        }
                    } else {
                        if let Some(tex) = get_or_load_texture(vertex_id, image_data, mime, media_cache, ctx) {
                            let size = tex.size_vec2();
                            let available = ui.available_size();
                            let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                            ui.image((tex.id(), size * scale));
                        }
                    }
                });
        }
    }

    action
}

fn render_video_modal(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &AppState,
    media_cache: &mut MediaCache,
) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    if let Some(vertex_id) = app_state.video_modal_vertex_id {
        // Update video texture from decoded frames
        let (latest_frame, is_playing) = {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                let mut latest = None;
                while let Ok(frame) = player.frame_rx.try_recv() {
                    latest = Some(frame);
                }
                (latest, player.is_playing())
            } else {
                (None, false)
            }
        };

        if let Some(frame) = latest_frame {
            if frame.width > 0 && frame.height > 0 && !frame.rgba.is_empty() {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [frame.width as usize, frame.height as usize],
                    &frame.rgba,
                );
                let handle = ctx.load_texture(
                    format!("video_{}", vertex_id),
                    image,
                    egui::TextureOptions::default(),
                );
                media_cache.video_textures.insert(vertex_id, handle);
            }
        }

        if is_playing {
            ctx.request_repaint();
        }

        ui.horizontal(|ui| {
            ui.heading("Video");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").clicked() {
                    action = SidebarContentAction::StopVideo { vertex_id };
                }
            });
        });
        ui.separator();
        ui.label("Ctrl+Enter: Fullscreen | Escape: Close");
        ui.separator();

        // Display video frame
        let video_width = if let Some(tex) = media_cache.video_textures.get(&vertex_id) {
            let available = ui.available_size() - egui::vec2(0.0, 60.0);
            let tex_size = tex.size_vec2();
            let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
            let display_size = tex_size * scale;
            ui.vertical_centered(|ui| {
                ui.image((tex.id(), display_size));
            });
            display_size.x
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.label("Loading video...");
                ui.spinner();
                ui.add_space(100.0);
            });
            ui.available_width()
        };

        ui.add_space(8.0);

        // Seek bar
        if let Some(player) = media_cache.video_players.get(&vertex_id) {
            let pos = player.get_position();
            let dur = player.duration;
            let dur_secs = dur.as_secs_f32().max(0.1);
            let mut pos_secs = pos.as_secs_f32();

            let available_width = ui.available_width();
            let left_padding = ((available_width - video_width) / 2.0).max(0.0);

            ui.horizontal(|ui| {
                ui.add_space(left_padding);
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(video_width, 20.0),
                    egui::Sense::click_and_drag()
                );

                if ui.is_rect_visible(rect) {
                    let progress = pos_secs / dur_secs;
                    let filled_width = rect.width() * progress;

                    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_gray(60));
                    let filled_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(filled_width, rect.height())
                    );
                    ui.painter().rect_filled(filled_rect, 4.0, egui::Color32::from_rgb(100, 150, 255));

                    if response.dragged() || response.clicked() {
                        if let Some(pointer_pos) = response.interact_pointer_pos() {
                            let relative_x = (pointer_pos.x - rect.left()) / rect.width();
                            let new_pos = (relative_x.clamp(0.0, 1.0) * dur_secs) as f32;
                            if (new_pos - pos_secs).abs() > 0.1 {
                                pos_secs = new_pos;
                            }
                        }
                    }

                    let indicator_x = rect.left() + filled_width;
                    let indicator_center = egui::pos2(indicator_x, rect.center().y);
                    ui.painter().circle_filled(indicator_center, 8.0, egui::Color32::WHITE);
                }
            });

            if pos_secs != pos.as_secs_f32() {
                player.seek(Duration::from_secs_f32(pos_secs));
            }
        }

        // Controls
        ui.horizontal(|ui| {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                if player.is_playing() {
                    if ui.button("⏸ Pause").clicked() {
                        player.pause();
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        player.play();
                    }
                }
                let pos = player.get_position();
                let dur = player.duration;
                ui.label(format!(
                    "{:02}:{:02} / {:02}:{:02}",
                    pos.as_secs() / 60,
                    pos.as_secs() % 60,
                    dur.as_secs() / 60,
                    dur.as_secs() % 60
                ));
            }
        });
    }

    action
}

fn render_nextcloud_login(ui: &mut egui::Ui, ctx: &egui::Context, app_state: &AppState) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;
    let nc_url = app_state.nextcloud_login_state.as_ref().unwrap().nextcloud_url.clone();

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.heading("Awaiting Nextcloud Activation");
        ui.add_space(10.0);
        ui.label(format!("Connecting to: {}", nc_url));
        ui.add_space(10.0);
        ui.label("Please complete the login in your web browser.");
        ui.label("This will close automatically when done.");
        ui.add_space(20.0);
        ui.spinner();
        ui.add_space(20.0);
        if ui.button("Cancel").clicked() {
            action = SidebarContentAction::CancelNextcloudLogin;
        }
    });
    ctx.request_repaint();

    action
}

fn render_identity_setup(ui: &mut egui::Ui, app_state: &mut AppState) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.vertical_centered(|ui| {
        ui.add_space(10.0);
        ui.heading("Choose Display Name");
    });
    ui.add_space(10.0);
    ui.label("Logged in successfully!");
    ui.add_space(5.0);
    ui.label("Choose a display name for this identity.");
    ui.label("(This is just a label - your real identity is the public key URL)");
    ui.add_space(10.0);

    let mut create_action = false;
    let mut cancel_action = false;

    if let Some(ref mut pending_setup) = app_state.pending_identity_setup {
        ui.horizontal(|ui| {
            ui.label("Display Name:");
            ui.text_edit_singleline(&mut pending_setup.display_name_input);
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Create Identity").clicked() {
                create_action = true;
            }
            if ui.button("Cancel").clicked() {
                cancel_action = true;
            }
        });
    }

    if cancel_action {
        action = SidebarContentAction::CancelIdentitySetup;
    }
    if create_action {
        if let Some(pending_setup) = app_state.pending_identity_setup.as_ref() {
            let display_name = pending_setup.display_name_input.trim().to_string();
            let display_name = if display_name.is_empty() {
                pending_setup.username.clone()
            } else {
                display_name
            };
            action = SidebarContentAction::CreateIdentity {
                nextcloud_url: pending_setup.nextcloud_url.clone(),
                username: pending_setup.username.clone(),
                app_password: pending_setup.app_password.clone(),
                display_name,
            };
        }
    }

    action
}

fn render_identity_panel(ui: &mut egui::Ui, app_state: &mut AppState) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.horizontal(|ui| {
        ui.heading("Identity Management");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = SidebarContentAction::CloseIdentityPanel;
            }
        });
    });
    ui.separator();

    // List existing identities
    ui.label("Your Identities:");
    if app_state.identity_config.identities.is_empty() {
        ui.label("No identities configured. Add a Nextcloud account below.");
    } else {
        let mut to_remove = None;
        for (i, identity) in app_state.identity_config.identities.iter().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(&identity.display_name);
                    if ui.small_button("Remove").clicked() {
                        to_remove = Some(i);
                    }
                });
                ui.monospace(&identity.share_url);
            });
        }
        if let Some(i) = to_remove {
            action = SidebarContentAction::RemoveIdentity(i);
        }
    }

    ui.separator();
    ui.label("Add Nextcloud Account:");
    ui.horizontal(|ui| {
        ui.label("URL:");
        ui.text_edit_singleline(&mut app_state.nextcloud_url_input);
    });

    if ui.button("Connect Nextcloud Account").clicked() {
        let nc_url = app_state.nextcloud_url_input.trim().to_string();
        if !nc_url.is_empty() {
            action = SidebarContentAction::InitiateNextcloudLogin(nc_url);
        }
    }

    action
}

fn render_text_input_mode(ui: &mut egui::Ui, app_state: &mut AppState, direction: Option<usize>) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    let title = if direction.is_some() { "New Text Note" } else { "Edit Text" };
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕ Cancel").clicked() {
                action = SidebarContentAction::CancelTextInput;
            }
        });
    });
    ui.separator();

    if let Some(dir) = direction {
        let dir_name = match dir {
            EDGE_NORTH => "North ↑",
            EDGE_SOUTH => "South ↓",
            EDGE_WEST => "West ←",
            EDGE_EAST => "East →",
            EDGE_UP => "Up ⬆",
            EDGE_DOWN => "Down ⬇",
            _ => "?",
        };
        ui.label(format!("Creating new note to the {}", dir_name));
    } else {
        ui.label("Editing current vertex");
    }

    if ui.button("Save (Ctrl+Enter)").clicked() {
        // Signal submit - handled in the existing submit_text logic
    }

    ui.separator();

    egui::ScrollArea::vertical()
        .max_height(ui.available_height() - 20.0)
        .show(ui, |ui| {
            // Use .show() to get cursor/selection info for our clipboard
            let output = egui::TextEdit::multiline(&mut app_state.text_input_buffer)
                .desired_width(f32::INFINITY)
                .desired_rows(10)
                .font(egui::TextStyle::Monospace)
                .show(ui);

            output.response.request_focus();

            // Update our selection state from egui's cursor state
            if let Some(cursor_range) = output.cursor_range {
                use egui::text::CursorRange;
                let range: CursorRange = cursor_range;
                let primary = range.primary.ccursor.index;
                let secondary = range.secondary.ccursor.index;

                if primary != secondary {
                    // There's a selection
                    app_state.text_selection_start = Some(secondary.min(primary));
                    app_state.text_cursor_pos = secondary.max(primary);
                } else {
                    // Just cursor, no selection
                    app_state.text_cursor_pos = primary;
                    app_state.text_selection_start = None;
                }
            }
        });

    action
}

fn render_recording_mode(ui: &mut egui::Ui, ctx: &egui::Context, app_state: &AppState) -> SidebarContentAction {
    let elapsed = app_state.recording_start
        .map(|s| s.elapsed())
        .unwrap_or(Duration::ZERO);

    ui.vertical_centered(|ui| {
        ui.add_space(20.0);
        ui.heading("Recording Audio");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.heading("🔴");
            ui.heading(format!("{:.1}s", elapsed.as_secs_f32()));
        });
        ui.add_space(10.0);
        ui.label("Release Space to save");
        ui.add_space(10.0);

        let level = if let Ok(samples) = app_state.audio_samples.lock() {
            if samples.len() > 1000 {
                samples.iter().rev().take(1000).map(|s| s.abs()).sum::<f32>() / 1000.0 * 10.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        ui.add(egui::ProgressBar::new(level.min(1.0)));

        ui.add_space(20.0);
        if ui.button("Cancel (Escape)").clicked() {
            // Will be handled by existing cancel_recording logic
        }
    });
    ctx.request_repaint();

    SidebarContentAction::None
}

fn render_identification_request(ui: &mut egui::Ui, app_state: &mut AppState) -> SidebarContentAction {
    let pending = app_state.pending_identification.clone().unwrap();

    // Check if this is a remembered server (auto-identify handled elsewhere)
    let is_remembered = app_state.identity_config.identities.get(app_state.selected_identity_index)
        .map(|id| id.remembered_servers.contains(&pending.server_url))
        .unwrap_or(false);

    if !is_remembered {
        ui.heading("Identification Request");
        ui.separator();

        ui.label(format!("Server: {}", pending.server_url));
        ui.label(format!("Reason: {}", pending.reason));
        ui.separator();

        if app_state.identity_config.identities.is_empty() {
            ui.label("No identities configured.");
            ui.label("Connect a Nextcloud account to identify yourself.");
            ui.add_space(8.0);
            if ui.button("Connect Nextcloud Account").clicked() {
                return SidebarContentAction::InitiateNextcloudLogin(String::new());
            }
            if ui.button("Refuse (Escape)").clicked() {
                // Will be handled by keyboard
            }
        } else {
            ui.label("Identify as (Tab to cycle):");
            let identity_info: Vec<(String, String)> = app_state.identity_config.identities
                .iter()
                .map(|id| (id.display_name.clone(), id.share_url.clone()))
                .collect();
            for (i, (name, url)) in identity_info.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.radio_value(&mut app_state.selected_identity_index, i, name);
                });
                if app_state.selected_identity_index == i {
                    ui.indent("identity_url", |ui| {
                        ui.monospace(url);
                    });
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Identify (Enter)").clicked() {
                    // Will be handled by keyboard
                }
                if ui.button("Remember").clicked() {
                    // Will be handled by keyboard
                }
                if ui.button("Refuse (Esc)").clicked() {
                    // Will be handled by keyboard
                }
            });
        }
    }

    SidebarContentAction::None
}

fn render_nav_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
    ws_cmd_tx: &WsCommandTx,
) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.horizontal(|ui| {
        ui.heading("Navigation");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = SidebarContentAction::CloseNavPanel;
            }
        });
    });
    ui.separator();

    ui.label("Ctrl+N to toggle | Click to jump");
    ui.separator();

    // Landmark History section
    ui.heading("Landmark History");
    if app_state.landmark_history.is_empty() {
        ui.label("No landmarks visited yet.");
    } else {
        let landmark_vertices: Vec<(String, Option<u64>)> = app_state.landmark_history.iter().rev()
            .map(|landmark| {
                let vertex_id = graph.landmark_vertices.get(landmark)
                    .and_then(|vertices| vertices.first().copied());
                (landmark.clone(), vertex_id)
            })
            .collect();

        egui::ScrollArea::vertical()
            .id_source("landmark_history")
            .max_height(200.0)
            .show(ui, |ui| {
                let zoom = 1.0f32;
                let card_width = ui.available_width() - 16.0;
                let card_height = 100.0 * zoom;
                let font_size = 13.0 * zoom;

                for (landmark, vertex_id_opt) in landmark_vertices.iter() {
                    let is_current = graph.context_uri.as_ref() == Some(landmark);

                    ui.add_space(4.0);

                    if is_current {
                        ui.horizontal(|ui| {
                            ui.label("→ Current:");
                        });
                    }

                    let (card_rect, response) = ui.allocate_exact_size(
                        egui::vec2(card_width, card_height),
                        egui::Sense::click(),
                    );

                    let painter = ui.painter();
                    if let Some(vertex_id) = vertex_id_opt {
                        if let Some(vertex) = graph.vertices.get(vertex_id) {
                            render_vertex_card(
                                painter,
                                vertex,
                                *vertex_id,
                                card_rect,
                                is_current,
                                zoom,
                                font_size,
                                media_cache,
                                ctx,
                                graph,
                            );
                        } else {
                            render_landmark_placeholder(painter, card_rect, landmark);
                        }
                    } else {
                        render_landmark_placeholder(painter, card_rect, landmark);
                    }

                    if response.clicked() && !is_current {
                        if let Some(vertex_id) = vertex_id_opt {
                            if graph.vertices.contains_key(vertex_id) {
                                action = SidebarContentAction::JumpToVertex(*vertex_id);
                            } else {
                                action = SidebarContentAction::WatchLandmark(landmark.clone());
                            }
                        } else {
                            action = SidebarContentAction::WatchLandmark(landmark.clone());
                        }
                    }

                    ui.add_space(4.0);
                }
            });
    }

    ui.separator();

    // Islands section
    ui.heading("Islands");
    let islands = find_islands(graph, app_state.current_vertex);

    if islands.len() <= 1 {
        ui.label("No disconnected islands.");
    } else {
        ui.label(format!("{} islands found:", islands.len()));

        egui::ScrollArea::vertical()
            .id_source("islands")
            .max_height(ui.available_height() - 20.0)
            .show(ui, |ui| {
                let zoom = 1.0f32;
                let card_width = ui.available_width() - 16.0;
                let card_height = 100.0 * zoom;
                let font_size = 13.0 * zoom;

                for (island_idx, island) in islands.iter().enumerate() {
                    let is_current_island = island_idx == 0;
                    let header = if is_current_island {
                        format!("Current ({} vertices)", island.len())
                    } else {
                        format!("Island {} ({} vertices)", island_idx, island.len())
                    };

                    ui.collapsing(header, |ui| {
                        let display_count = island.len().min(5);
                        for &vertex_id in island.iter().take(display_count) {
                            ui.add_space(4.0);

                            let (card_rect, response) = ui.allocate_exact_size(
                                egui::vec2(card_width, card_height),
                                egui::Sense::click(),
                            );

                            let painter = ui.painter();
                            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                                let is_current = app_state.current_vertex == Some(vertex_id);
                                render_vertex_card(
                                    painter,
                                    vertex,
                                    vertex_id,
                                    card_rect,
                                    is_current,
                                    zoom,
                                    font_size,
                                    media_cache,
                                    ctx,
                                    graph,
                                );
                            } else {
                                render_vertex_placeholder(painter, card_rect, vertex_id);
                            }

                            if response.clicked() {
                                action = SidebarContentAction::JumpToVertex(vertex_id);
                            }

                            ui.add_space(4.0);
                        }
                        if island.len() > display_count {
                            ui.label(format!("... and {} more", island.len() - display_count));
                        }
                    });
                }
            });
    }

    action
}

fn render_landmark_placeholder(painter: &egui::Painter, rect: egui::Rect, landmark: &str) {
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 50, 55));
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 90)));
    let short_landmark: String = if landmark.len() > 30 {
        format!("...{}", &landmark[landmark.len()-27..])
    } else {
        landmark.to_string()
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        short_landmark,
        egui::FontId::proportional(12.0),
        egui::Color32::GRAY,
    );
}

fn render_vertex_placeholder(painter: &egui::Painter, rect: egui::Rect, vertex_id: u64) {
    painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(50, 50, 55));
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(80, 80, 90)));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("Vertex {}", vertex_id),
        egui::FontId::proportional(12.0),
        egui::Color32::GRAY,
    );
}

fn render_bag_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &mut AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    ui.horizontal(|ui| {
        ui.heading("Bag");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                action = SidebarContentAction::CloseBagPanel;
            }
        });
    });
    ui.separator();

    ui.horizontal(|ui| {
        if ui.button("Clear All").clicked() {
            action = SidebarContentAction::ClearBag;
        }
    });

    ui.label("Y=Yank | P=Paste | G=Go | Ctrl+Y=Pop");
    ui.separator();

    if app_state.bag.is_empty() {
        ui.label("Bag is empty. Press Y to yank current vertex.");
    } else {
        ui.label(format!("{} item(s):", app_state.bag.len()));

        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 20.0)
            .show(ui, |ui| {
                let mut remove_idx = None;
                let mut jump_to = None;

                let zoom = 1.0f32;
                let card_width = ui.available_width() - 16.0;
                let card_height = 140.0 * zoom;
                let font_size = 13.0 * zoom;

                for (i, &vertex_id) in app_state.bag.iter().rev().enumerate() {
                    let stack_idx = app_state.bag.len() - 1 - i;
                    let is_top = i == 0;

                    ui.add_space(4.0);

                    if is_top {
                        ui.horizontal(|ui| {
                            ui.label("→ Next to paste:");
                        });
                    }

                    let (card_rect, response) = ui.allocate_exact_size(
                        egui::vec2(card_width, card_height),
                        egui::Sense::click(),
                    );

                    let painter = ui.painter();
                    if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        render_vertex_card(
                            painter,
                            vertex,
                            vertex_id,
                            card_rect,
                            is_top,
                            zoom,
                            font_size,
                            media_cache,
                            ctx,
                            graph,
                        );
                    } else {
                        render_vertex_placeholder(painter, card_rect, vertex_id);
                    }

                    if response.clicked() {
                        jump_to = Some(vertex_id);
                    }

                    ui.horizontal(|ui| {
                        if ui.small_button("Go").clicked() {
                            jump_to = Some(vertex_id);
                        }
                        if ui.small_button("Remove").clicked() {
                            remove_idx = Some(stack_idx);
                        }
                    });

                    ui.add_space(4.0);
                    if i < app_state.bag.len() - 1 {
                        ui.separator();
                    }
                }

                if let Some(idx) = remove_idx {
                    action = SidebarContentAction::RemoveFromBag(idx);
                }
                if let Some(vid) = jump_to {
                    action = SidebarContentAction::JumpToVertex(vid);
                }
            });
    }

    action
}

fn render_keybindings_mode(ui: &mut egui::Ui, app_state: &mut AppState) -> SidebarContentAction {
    let mut action = SidebarContentAction::None;

    // We need to avoid borrowing app_state twice, so we take out the editor state temporarily
    let mut editor_state = std::mem::take(&mut app_state.keybindings_editor);
    let kb_action = sidebar::render_keybindings_editor(
        ui,
        &mut app_state.keybindings,
        &mut editor_state,
    );
    app_state.keybindings_editor = editor_state;

    match kb_action {
        sidebar::KeybindingsAction::Close => {
            action = SidebarContentAction::CloseKeybindings;
        }
        sidebar::KeybindingsAction::Save => {
            action = SidebarContentAction::SaveKeybindings;
        }
        sidebar::KeybindingsAction::ApplyPreset(preset) => {
            action = SidebarContentAction::ApplyPreset(preset);
        }
        sidebar::KeybindingsAction::None => {}
    }

    action
}

fn render_preview_mode(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app_state: &AppState,
    graph: &GraphState,
    media_cache: &mut MediaCache,
    playback_state: &AudioPlaybackState,
) -> SidebarContentAction {
    ui.heading("Content Preview");
    ui.separator();

    if let Some(current_id) = app_state.current_vertex {
        if let Some(vertex) = graph.vertices.get(&current_id) {
            render_vertex_content(ui, vertex, current_id, media_cache, ctx, playback_state);
        }
    } else {
        ui.label("No vertex selected");
    }

    SidebarContentAction::None
}
