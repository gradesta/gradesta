use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use crossbeam_channel::unbounded;
use std::thread;
use std::time::{Duration, Instant};

// Core modules
mod audio;
mod commands;
mod debug_log;
mod elf_http;
mod events;
mod export;
mod graph;
mod identity;
mod keybindings;
mod media;
mod network;
mod rendering;
mod sidebar;
mod state;
mod tts;
mod ui;
mod video_player;
mod whisper;

// Imports from refactored modules
use audio::{AudioPlaybackState, AudioRecordingSignal};
use audio::{play_audio, stop_audio};
use graph::GraphState;
use media::MediaCache;
use network::{run_ws, NetEventsTx, NetRx, ServerEvent, WsCommand, WsCommandTx};
use state::{AppState, InputMode, NextcloudLoginState};
use state::PendingIdentitySetup;
use state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};
use state::{KEY_REPEAT_DELAY, KEY_REPEAT_RATE};

// Existing module imports
use commands::Command;
use identity::Identity;

/// Determine the current keybinding context from app state
fn current_context(app_state: &AppState) -> commands::Context {
    match app_state.input_mode {
        InputMode::TextInput { .. } => commands::Context::TextInput,
        InputMode::Recording { .. } => commands::Context::Recording,
        InputMode::Normal => {
            if app_state.focus_url_bar_next_frame || app_state.url_bar_has_focus {
                // URL bar is focused - treat as text input context
                commands::Context::TextInput
            } else if app_state.pending_identification.is_some() {
                commands::Context::Authentication
            } else if app_state.show_bag_panel {
                commands::Context::Bag
            } else if app_state.show_nav_panel {
                commands::Context::NavPanel
            } else if app_state.show_command_bar {
                // Command bar has its own UI handling, use Global for fallback
                commands::Context::Global
            } else {
                commands::Context::Graph
            }
        }
    }
}

/// Sync all identities bidirectionally with Nextcloud on startup
fn sync_all_identities(app_state: &mut AppState) {
    eprintln!("Syncing identities with Nextcloud...");

    for identity in &mut app_state.identity_config.identities {
        let local_metadata = identity::IdentityMetadata {
            display_name: identity.display_name.clone(),
            share_url: identity.share_url.clone(),
            remembered_servers: identity.remembered_servers.clone(),
        };

        match identity::sync_identity_bidirectional(
            &identity.nextcloud_url,
            &identity.username,
            &identity.app_password,
            &local_metadata,
        ) {
            Ok(merged) => {
                // Update local identity with merged data
                if identity.remembered_servers != merged.remembered_servers {
                    eprintln!("  {} - synced {} remembered servers",
                        identity.display_name,
                        merged.remembered_servers.len());
                    identity.remembered_servers = merged.remembered_servers;
                } else {
                    eprintln!("  {} - up to date", identity.display_name);
                }
            }
            Err(e) => {
                eprintln!("  {} - sync failed: {}", identity.display_name, e);
            }
        }
    }

    // Save any changes
    if let Err(e) = app_state.identity_config.save() {
        eprintln!("Failed to save identity config after sync: {}", e);
    }
}

fn main() {
    // Check if Whisper model needs to be downloaded
    if !whisper::is_model_available() {
        run_startup_download();
    }

    // Preload Whisper model in background so it's ready when needed
    whisper::preload_model();

    // Sync identities from Nextcloud on startup
    let mut app_state = AppState::default();
    sync_all_identities(&mut app_state);

    // Initialize debug log file
    let debug_log_path = debug_log::init_debug_log();
    eprintln!("Debug log: {}", debug_log_path.display());
    app_state.debug_log_file = Some(debug_log_path);

    let (net_tx, net_rx) = unbounded::<ServerEvent>();

    App::new()
        .insert_resource(NetRx(net_rx))
        .insert_resource(NetEventsTx(net_tx))
        .insert_resource(WsCommandTx(None))
        .insert_resource(GraphState::default())
        .insert_resource(app_state)
        .insert_resource(MediaCache::default())
        .insert_resource(AudioRecordingSignal::default())
        .insert_resource(AudioPlaybackState::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gradesta Browser".to_string(),
                resolution: (1400.0, 900.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (
            ui_system,
            events::ingest_server_events,
            handle_navigation,
            auto_play_audio_on_navigate,
            auto_expand_nearby_links,
        ))
        .run();
}

/// Run the startup download dialog using eframe
fn run_startup_download() {
    use eframe::egui;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    let progress = Arc::new(whisper::DownloadProgress::default());
    let progress_clone = Arc::clone(&progress);

    // Start download in background thread
    std::thread::spawn(move || {
        if let Err(e) = whisper::download_model_with_progress(progress_clone) {
            eprintln!("Download failed: {}", e);
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 150.0])
            .with_title("Gradesta Browser")
            .with_resizable(false),
        ..Default::default()
    };

    struct DownloadApp {
        progress: Arc<whisper::DownloadProgress>,
    }

    impl eframe::App for DownloadApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            let downloaded = self.progress.downloaded.load(Ordering::SeqCst);
            let total = self.progress.total.load(Ordering::SeqCst);
            let complete = self.progress.complete.load(Ordering::SeqCst);
            let error = self.progress.error.lock().unwrap().clone();

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.heading("Gradesta Browser");
                    ui.add_space(10.0);

                    if let Some(err) = error {
                        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                        ui.add_space(10.0);
                        if ui.button("Retry").clicked() {
                            // Clear error and restart
                            *self.progress.error.lock().unwrap() = None;
                            self.progress.downloaded.store(0, Ordering::SeqCst);
                            let progress_clone = Arc::clone(&self.progress);
                            std::thread::spawn(move || {
                                let _ = whisper::download_model_with_progress(progress_clone);
                            });
                        }
                    } else if complete {
                        ui.label("Setup complete!");
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else if total > 0 {
                        ui.label("Downloading speech recognition model...");
                        ui.add_space(10.0);

                        let fraction = downloaded as f32 / total as f32;
                        let progress_bar = egui::ProgressBar::new(fraction)
                            .show_percentage()
                            .animate(true);
                        ui.add(progress_bar);

                        ui.add_space(5.0);
                        ui.label(format!(
                            "{:.1} MB / {:.1} MB",
                            downloaded as f64 / 1_000_000.0,
                            total as f64 / 1_000_000.0
                        ));
                    } else {
                        ui.label("Connecting to download server...");
                        ui.add_space(10.0);
                        ui.spinner();
                    }
                });
            });

            // Keep refreshing
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    let _ = eframe::run_native(
        "Gradesta Browser Setup",
        options,
        Box::new(|_cc| Box::new(DownloadApp { progress })),
    );
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn ui_system(
    mut contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    mut graph: ResMut<GraphState>,
    net_events: Res<NetEventsTx>,
    mut ws_cmd_tx: ResMut<WsCommandTx>,
    mut media_cache: ResMut<MediaCache>,
    audio_signal: Res<AudioRecordingSignal>,
    playback_state: Res<AudioPlaybackState>,
) {
    let ctx = contexts.ctx_mut();

    // Process at most ONE decoded image per frame to avoid GPU upload stalls
    if let Ok(decoded) = media_cache.decoded_rx.try_recv() {
        // For very large images, downsample to avoid GPU memory issues and upload stalls
        // Max dimension of 2048 is reasonable for most displays
        const MAX_DIM: u32 = 2048;
        let (final_width, final_height, final_rgba) = if decoded.width > MAX_DIM || decoded.height > MAX_DIM {
            let scale = (MAX_DIM as f32 / decoded.width.max(decoded.height) as f32).min(1.0);
            let new_width = (decoded.width as f32 * scale) as u32;
            let new_height = (decoded.height as f32 * scale) as u32;

            // Simple bilinear downsampling
            let mut downsampled = vec![0u8; (new_width * new_height * 4) as usize];
            for y in 0..new_height {
                for x in 0..new_width {
                    let src_x = (x as f32 / scale) as u32;
                    let src_y = (y as f32 / scale) as u32;
                    let src_idx = ((src_y * decoded.width + src_x) * 4) as usize;
                    let dst_idx = ((y * new_width + x) * 4) as usize;
                    if src_idx + 3 < decoded.rgba.len() {
                        downsampled[dst_idx..dst_idx + 4].copy_from_slice(&decoded.rgba[src_idx..src_idx + 4]);
                    }
                }
            }
            (new_width, new_height, downsampled)
        } else {
            (decoded.width, decoded.height, decoded.rgba)
        };

        let image = egui::ColorImage::from_rgba_unmultiplied(
            [final_width as usize, final_height as usize],
            &final_rgba,
        );
        let handle = ctx.load_texture(
            format!("vertex_{}", decoded.id),
            image,
            egui::TextureOptions::default(),
        );
        media_cache.textures.insert(decoded.id, handle);
        media_cache.pending_decodes.remove(&decoded.id);
    }

    // Handle modal keyboard shortcuts and zoom

    // Determine current keybinding context and capture keyboard commands
    let kb_context = current_context(&app_state);

    // Log context changes
    let context_name = debug_log::context_name(kb_context);
    if app_state.debug_last_context.as_deref() != Some(context_name) {
        if let Some(old_ctx) = app_state.debug_last_context.take() {
            debug_log::log_context_change(&mut app_state, &old_ctx, context_name);
        }
        app_state.debug_last_context = Some(context_name.to_string());
    }

    let cmds = ui::capture_keyboard_commands(ctx, &app_state.keybindings, kb_context);
    // Log triggered commands to debug log (separated to avoid borrow conflicts)
    ui::log_triggered_commands_to_debug(&cmds, &mut app_state);
    let cmd_refresh = cmds.refresh;  // Used later for refresh logic

    // IMPORTANT: Consume text edit events BEFORE any UI rendering
    // This prevents egui's TextEdit from trying to use the broken system clipboard
    // and inserting raw characters like 'v' when Ctrl+V is pressed
    ui::consume_text_edit_events(ctx, &app_state);

    // Handle Ctrl+Shift+C to copy full URL when URL bars are focused
    ui::handle_url_bar_copy_shortcut(ctx, &mut app_state);

    // Handle smart paste for URL bars (detects full URLs and distributes to server/landmark)
    ui::handle_url_bar_smart_paste(ctx, &mut app_state);

    // Execute all keyboard commands using the ui module
    let cmd_results = ui::execute_commands(
        &cmds,
        &mut app_state,
        &mut graph,
        &ws_cmd_tx,
        &mut media_cache,
        &audio_signal,
        &playback_state,
        ctx,
    );

    // Process text editing commands (copy, cut, paste, undo, redo)
    ui::process_text_edit_commands(&cmds, &mut app_state);

    // Finalize recording if needed
    if cmd_results.should_finalize_recording {
        ui::finalize_recording(&mut app_state, &audio_signal, &ws_cmd_tx);
    }

    // Apply zoom by scaling the UI - we do this manually in rendering instead of using pixels_per_point
    // because set_pixels_per_point causes layout issues

    // GlobalRefresh is already computed above as cmd_refresh

    // FULLSCREEN MODE: When fullscreen, skip the normal UI and render content directly
    if app_state.sidebar.fullscreen {
        let action = ui::render_fullscreen_content(ctx, &mut app_state, &graph, &mut media_cache, &playback_state);
        if matches!(action, ui::FullscreenAction::ExitFullscreen) {
            app_state.sidebar.fullscreen = false;
        }
        return; // Skip the rest of the normal UI
    }

    // NORMAL MODE: Render full UI with panels
    // Top panel with URL bar (split into server and landmark)
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

            // Show different button based on connection state
            let button_label = if app_state.connected { "Refresh" } else { "Connect" };
            let button_clicked = ui.button(button_label).clicked();

            // Copy URL button (clipboard icon)
            let copy_clicked = ui.button("📋").clicked();
            if copy_clicked {
                let full_url = ui::url_utils::construct_full_url(&app_state.server_input, &app_state.landmark_input);
                ui::url_utils::set_clipboard_text(&full_url);
                app_state.status = "Copied URL to clipboard".to_string();
            }

            // Enter pressed in either input triggers connection
            let server_enter = server_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let landmark_enter = landmark_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            // Connect/refresh on button click, Enter, or GlobalRefresh command
            if button_clicked || server_enter || landmark_enter || cmd_refresh {
                let server = app_state.server_input.trim().to_string();
                let landmark = app_state.landmark_input.trim().to_string();
                if server.is_empty() {
                    app_state.status = "Server is empty!".to_string();
                } else if landmark.is_empty() {
                    app_state.status = "Landmark is empty!".to_string();
                } else {
                    // Construct full URL from server and landmark
                    let url = ui::url_utils::construct_full_url(&server, &landmark);

                    // Disconnect existing connection first
                    if app_state.connected {
                        app_state.connected = false;
                        ws_cmd_tx.0 = None; // Drop the sender, which will cause the WS thread to stop
                    }

                    // Clear graph state for fresh connection
                    graph.vertices.clear();
                    graph.context_uri = None;
                    graph.current_receiving_landmark = None;
                    graph.landmark_vertices.clear();
                    app_state.current_vertex = None;
                    app_state.history.clear();
                    app_state.requested_landmarks.clear();

                    app_state.status = "Connecting...".to_string();
                    let tx = net_events.0.clone();
                    let url_clone = url.clone();
                    // Create command channel for sending watch requests to the WS thread
                    let (cmd_tx, cmd_rx) = unbounded::<WsCommand>();
                    ws_cmd_tx.0 = Some(cmd_tx);
                    thread::spawn(move || {
                        match url::Url::parse(&url_clone) {
                            Ok(parsed_url) => {
                                if let Err(err) = run_ws(parsed_url, cmd_rx, tx.clone()) {
                                    let _ = tx.send(ServerEvent::Error {
                                        message: format!("{err:#}"),
                                    });
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(ServerEvent::Error {
                                    message: format!("Invalid URL: {}", e),
                                });
                            }
                        }
                    });
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

    // Bottom panel with navigation help
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
                }
                ui.separator();
                // Debug panel toggle
                let debug_label = if app_state.show_debug_panel { "🐛 Debug ON" } else { "🐛 Debug" };
                if ui.button(debug_label).on_hover_text("Toggle debug log panel").clicked() {
                    app_state.show_debug_panel = !app_state.show_debug_panel;
                }
                ui.separator();
                // Keybindings button (prominent)
                if ui.button("⌨ Keybindings (Ctrl+K)").clicked() {
                    app_state.sidebar.mode = sidebar::SidebarMode::Keybindings;
                }
                ui.separator();
                if ui.button("🔑 Identities").clicked() {
                    app_state.show_identity_panel = !app_state.show_identity_panel;
                }
                let id_count = app_state.identity_config.identities.len();
                if id_count > 0 {
                    ui.label(format!("{} id", id_count));
                }
                ui.separator();
                // Export button
                if ui.button("📤 Export").on_hover_text("Export graph section to HTML").clicked() {
                    app_state.sidebar.mode = sidebar::SidebarMode::Export;
                    app_state.export_state = export::ExportState::new();
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
                }
            });
        });
        ui.add_space(4.0);
    });

    // Command bar overlay (vim-style ':' command)
    if let ui::CommandBarAction::Execute(cmd) = ui::render_command_bar(ctx, &mut app_state) {
        ui::execute_command_bar_command(cmd, &mut app_state);
    }

    // Right panel for content - renders based on sidebar mode
    let sidebar_action = egui::SidePanel::right("preview_panel").min_width(400.0).show(ctx, |ui| {
        ui::render_sidebar_content(ui, ctx, &mut app_state, &graph, &mut media_cache, &ws_cmd_tx, &playback_state)
    }).inner;

    // Sync any copied text to system clipboard (after all UI rendering)
    ui::sync_copy_to_system_clipboard(ctx);

    // Handle sidebar actions
    match sidebar_action {
        ui::SidebarContentAction::CloseTextModal => {
            app_state.show_text_modal = false;
        }
        ui::SidebarContentAction::CloseImageModal => {
            app_state.show_image_modal = false;
        }
        ui::SidebarContentAction::CloseVideoModal => {
            app_state.show_video_modal = false;
        }
        ui::SidebarContentAction::StopVideo { vertex_id } => {
            if let Some(player) = media_cache.video_players.get(&vertex_id) {
                player.stop();
            }
            app_state.show_video_modal = false;
        }
        ui::SidebarContentAction::CancelNextcloudLogin => {
            app_state.nextcloud_login_state = None;
        }
        ui::SidebarContentAction::CancelIdentitySetup => {
            app_state.pending_identity_setup = None;
            app_state.status = "Identity creation cancelled.".to_string();
        }
        ui::SidebarContentAction::CreateIdentity { nextcloud_url, username, app_password, display_name } => {
            app_state.pending_identity_setup = None;
            match identity::setup_identity(&nextcloud_url, &username, &app_password, &display_name) {
                Ok((signing_key, share_url)) => {
                    let new_identity = Identity {
                        display_name: display_name.clone(),
                        nextcloud_url,
                        username,
                        app_password,
                        share_url,
                        remembered_servers: Vec::new(),
                        signing_key: Some(signing_key),
                    };
                    app_state.identity_config.identities.push(new_identity);
                    let _ = app_state.identity_config.save();
                    app_state.status = format!("Identity '{}' created successfully!", display_name);
                }
                Err(e) => {
                    app_state.status = format!("Failed to setup identity: {}", e);
                }
            }
        }
        ui::SidebarContentAction::CloseIdentityPanel => {
            app_state.show_identity_panel = false;
        }
        ui::SidebarContentAction::RemoveIdentity(i) => {
            app_state.identity_config.identities.remove(i);
            let _ = app_state.identity_config.save();
        }
        ui::SidebarContentAction::InitiateNextcloudLogin(nc_url) => {
            if !nc_url.is_empty() {
                match identity::initiate_nextcloud_login(&nc_url) {
                    Ok((login_url, poll_endpoint, poll_token)) => {
                        let login_url_clone = login_url.clone();
                        thread::spawn(move || {
                            let _ = open::that(&login_url_clone);
                        });
                        app_state.nextcloud_login_state = Some(NextcloudLoginState {
                            nextcloud_url: nc_url,
                            poll_endpoint,
                            poll_token,
                            started: Instant::now(),
                        });
                        app_state.status = "Opening browser for Nextcloud login...".to_string();
                    }
                    Err(e) => {
                        app_state.status = format!("Failed to initiate login: {}", e);
                    }
                }
            } else {
                // Empty URL means show identity panel (from identification request)
                app_state.show_identity_panel = true;
            }
        }
        ui::SidebarContentAction::CancelTextInput => {
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
            app_state.status = "Text input cancelled".to_string();
        }
        ui::SidebarContentAction::CloseNavPanel => {
            app_state.show_nav_panel = false;
        }
        ui::SidebarContentAction::JumpToVertex(vid) => {
            if let Some(current_id) = app_state.current_vertex {
                app_state.history.push(current_id);
            }
            app_state.current_vertex = Some(vid);
        }
        ui::SidebarContentAction::WatchLandmark(landmark) => {
            if let Some(ref tx) = ws_cmd_tx.0 {
                if !app_state.requested_landmarks.contains(&landmark) {
                    app_state.requested_landmarks.insert(landmark.clone());
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    let _ = tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark.clone() });
                }
                app_state.following_portal = Some(landmark.clone());
                app_state.status = format!("Loading: {}", landmark);
            }
        }
        ui::SidebarContentAction::CloseBagPanel => {
            app_state.show_bag_panel = false;
        }
        ui::SidebarContentAction::ClearBag => {
            app_state.bag.clear();
            app_state.status = "Bag cleared".to_string();
        }
        ui::SidebarContentAction::RemoveFromBag(idx) => {
            app_state.bag.remove(idx);
        }
        ui::SidebarContentAction::CloseKeybindings => {
            app_state.sidebar.mode = sidebar::SidebarMode::Preview;
        }
        ui::SidebarContentAction::SaveKeybindings => {
            if let Err(e) = app_state.keybindings.save_to_config() {
                app_state.status = format!("Failed to save keybindings: {}", e);
            } else {
                app_state.status = "Keybindings saved".to_string();
            }
        }
        ui::SidebarContentAction::ApplyPreset(preset) => {
            match keybindings::KeybindingsConfig::load() {
                Ok(mut config) => {
                    keybindings::presets::apply_preset(&mut config, &preset);
                    if let Err(e) = config.save() {
                        app_state.status = format!("Failed to save preset: {}", e);
                    } else {
                        app_state.keybindings = keybindings::KeybindingResolver::new(&config);
                        app_state.status = format!("{} preset applied", preset.name());
                    }
                }
                Err(e) => {
                    app_state.status = format!("Failed to load config: {}", e);
                }
            }
        }
        ui::SidebarContentAction::CloseDebugPanel => {
            app_state.show_debug_panel = false;
        }
        ui::SidebarContentAction::ClearDebugLog => {
            debug_log::clear_log(&mut app_state);
            app_state.status = "Debug log cleared".to_string();
        }
        ui::SidebarContentAction::ExportToggleDirection(dir) => {
            app_state.export_state.toggle_direction(dir);
        }
        ui::SidebarContentAction::ExportConfirm => {
            if let Some(current_id) = app_state.current_vertex {
                let data = export::collect_vertices_for_export(
                    &graph,
                    current_id,
                    &app_state.export_state.directions,
                );
                let title = format!("Gradesta Export - {}",
                    graph.context_uri.as_deref().unwrap_or("Unknown"));
                let html = export::generate_html(&data, &title);
                match export::save_html_file(&html) {
                    Ok(path) => {
                        app_state.status = format!("Exported to: {}", path);
                        app_state.sidebar.mode = sidebar::SidebarMode::Preview;
                        app_state.export_state = export::ExportState::new();
                    }
                    Err(e) => {
                        app_state.status = format!("Export failed: {}", e);
                    }
                }
            } else {
                app_state.status = "No vertex selected for export".to_string();
            }
        }
        ui::SidebarContentAction::ExportCancel => {
            app_state.sidebar.mode = sidebar::SidebarMode::Preview;
            app_state.export_state = export::ExportState::new();
            app_state.status = "Export cancelled".to_string();
        }
        ui::SidebarContentAction::None => {}
    }

    // Poll for Nextcloud login completion (runs every frame, independent of UI panels)
    if let Some(ref login_state) = app_state.nextcloud_login_state.clone() {
        if login_state.started.elapsed() > Duration::from_millis(500) {
            match identity::poll_login_completion(&login_state.poll_endpoint, &login_state.poll_token) {
                Ok(Some((server, username, app_password))) => {
                    eprintln!("Login successful: {}@{}", username, server);

                    // Check if identity already exists on this Nextcloud account
                    match identity::sync_identity_from_nextcloud(&server, &username, &app_password) {
                        Ok(Some((signing_key, metadata))) => {
                            // Existing identity found - sync it
                            eprintln!("Found existing identity on Nextcloud: {}", metadata.display_name);
                            let new_identity = Identity {
                                display_name: metadata.display_name.clone(),
                                nextcloud_url: server,
                                username,
                                app_password,
                                share_url: metadata.share_url,
                                remembered_servers: metadata.remembered_servers,
                                signing_key: Some(signing_key),
                            };
                            // Check if we already have this identity locally (by share_url)
                            let exists = app_state.identity_config.identities.iter()
                                .any(|id| id.share_url == new_identity.share_url);
                            if !exists {
                                app_state.identity_config.identities.push(new_identity.clone());
                                let _ = app_state.identity_config.save();
                            }
                            app_state.status = format!("Identity '{}' synced from Nextcloud!", metadata.display_name);
                            app_state.nextcloud_login_state = None;
                        }
                        Ok(None) => {
                            // No existing identity - show display name prompt for new identity
                            app_state.status = format!("Logged in as {}. Please choose a display name.", username);
                            app_state.pending_identity_setup = Some(PendingIdentitySetup {
                                nextcloud_url: server,
                                username: username.clone(),
                                app_password,
                                display_name_input: username, // Default to username
                            });
                            app_state.nextcloud_login_state = None;
                        }
                        Err(e) => {
                            // Error checking - could be network issue, proceed with new identity flow
                            eprintln!("Error checking for existing identity: {}", e);
                            app_state.status = format!("Logged in as {}. Please choose a display name.", username);
                            app_state.pending_identity_setup = Some(PendingIdentitySetup {
                                nextcloud_url: server,
                                username: username.clone(),
                                app_password,
                                display_name_input: username,
                            });
                            app_state.nextcloud_login_state = None;
                        }
                    }
                }
                Ok(None) => {
                    // Still waiting - update the started time to throttle polling
                    if let Some(ref mut state) = app_state.nextcloud_login_state {
                        state.started = Instant::now();
                    }
                }
                Err(e) => {
                    eprintln!("Login poll error: {}", e);
                    app_state.status = format!("Login failed: {}", e);
                    app_state.nextcloud_login_state = None;
                }
            }
        }
    }

    // Process text input, recording cancellation, and identification using ui module
    ui::process_text_input(ctx, &mut app_state, &graph, &ws_cmd_tx, &net_events);
    ui::process_recording_cancel(ctx, &mut app_state, &audio_signal);
    ui::process_identification(ctx, &mut app_state, &ws_cmd_tx);

    // Central panel showing grid view
    ui::render_grid_view(ctx, &mut app_state, &graph, &mut media_cache, &ws_cmd_tx);
}

fn handle_navigation(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    keys: Res<ButtonInput<bevy::prelude::KeyCode>>,
    mut contexts: EguiContexts,
) {
    // Don't handle navigation when egui wants keyboard input (e.g., URL bar focused)
    let ctx = contexts.ctx_mut();
    if ctx.wants_keyboard_input() {
        return;
    }

    // Don't handle navigation when in text input mode
    if matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        return;
    }

    // Don't move when shift is held - shift+arrow only changes direction (handled in ui_system)
    let shift_held = keys.pressed(bevy::prelude::KeyCode::ShiftLeft) || keys.pressed(bevy::prelude::KeyCode::ShiftRight);
    if shift_held {
        return;
    }

    let Some(current_id) = app_state.current_vertex else { return };
    let Some(vertex) = graph.vertices.get(&current_id) else { return };

    // Use resolver to check which navigation command is active
    let context = commands::Context::Graph;
    let resolver = &app_state.keybindings;

    // Check which navigation key is held (if any)
    let held_edge: Option<usize> = if resolver.command_pressed_bevy(context, &Command::GraphNavigateNorth, &keys) {
        Some(EDGE_NORTH)
    } else if resolver.command_pressed_bevy(context, &Command::GraphNavigateSouth, &keys) {
        Some(EDGE_SOUTH)
    } else if resolver.command_pressed_bevy(context, &Command::GraphNavigateWest, &keys) {
        Some(EDGE_WEST)
    } else if resolver.command_pressed_bevy(context, &Command::GraphNavigateEast, &keys) {
        Some(EDGE_EAST)
    } else if resolver.command_pressed_bevy(context, &Command::GraphNavigateUp, &keys) {
        Some(EDGE_UP)
    } else if resolver.command_pressed_bevy(context, &Command::GraphNavigateDown, &keys) {
        Some(EDGE_DOWN)
    } else {
        None
    };

    // Check for just pressed (initial press)
    let just_pressed_edge: Option<usize> = if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateNorth, &keys) {
        Some(EDGE_NORTH)
    } else if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateSouth, &keys) {
        Some(EDGE_SOUTH)
    } else if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateWest, &keys) {
        Some(EDGE_WEST)
    } else if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateEast, &keys) {
        Some(EDGE_EAST)
    } else if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateUp, &keys) {
        Some(EDGE_UP)
    } else if resolver.command_just_pressed_bevy(context, &Command::GraphNavigateDown, &keys) {
        Some(EDGE_DOWN)
    } else {
        None
    };

    // Check for history back command
    if resolver.command_just_pressed_bevy(context, &Command::GraphHistoryBack, &keys) {
        if let Some(prev_id) = app_state.history.pop() {
            app_state.current_vertex = Some(prev_id);
        }
        return;
    }

    // Determine if we should move
    let should_move = if just_pressed_edge.is_some() {
        // Initial key press - always move
        app_state.key_repeat_last_move = Some(Instant::now());
        app_state.key_repeat_started = false;
        true
    } else if let Some(_edge) = held_edge {
        // Key is held - check repeat timing
        if let Some(last_move) = app_state.key_repeat_last_move {
            let elapsed = last_move.elapsed();
            if !app_state.key_repeat_started {
                // Haven't started repeating yet - check initial delay
                if elapsed >= KEY_REPEAT_DELAY {
                    app_state.key_repeat_started = true;
                    app_state.key_repeat_last_move = Some(Instant::now());
                    true
                } else {
                    false
                }
            } else {
                // Already repeating - check repeat rate
                if elapsed >= KEY_REPEAT_RATE {
                    app_state.key_repeat_last_move = Some(Instant::now());
                    true
                } else {
                    false
                }
            }
        } else {
            false
        }
    } else {
        // No key held - reset state
        app_state.key_repeat_last_move = None;
        app_state.key_repeat_started = false;
        false
    };

    let target_edge = just_pressed_edge.or(held_edge);

    // Update direction on any key press, even if we can't move
    if let Some(edge_idx) = just_pressed_edge {
        app_state.last_nav_direction = edge_idx;
    }

    if should_move {
        if let Some(edge_idx) = target_edge {
            let target_id = vertex.edges[edge_idx];
            if target_id != 0 {
                // Move cursor to target (even if it's a portal - auto_expand will handle following it)
                app_state.history.push(current_id);
                app_state.current_vertex = Some(target_id);
            } else {
                // Provide feedback for up/down navigation at stack edges
                if edge_idx == EDGE_UP {
                    app_state.status = "Top of stack".to_string();
                } else if edge_idx == EDGE_DOWN {
                    app_state.status = "Bottom of stack".to_string();
                }
            }
        }
    }
}

/// Auto-play audio when navigating to an audio cell, stop when leaving
/// Auto-play audio when navigating to an audio cell, stop when leaving
fn auto_play_audio_on_navigate(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    playback_state: Res<AudioPlaybackState>,
) {
    let current_id = app_state.current_vertex;
    let last_id = app_state.last_vertex;

    // Update last_vertex tracking
    if current_id != last_id {
        // Stop any playing audio when leaving a cell
        stop_audio(&playback_state);

        app_state.last_vertex = current_id;

        // If we navigated to a new vertex, check if it's audio
        if let Some(vertex_id) = current_id {
            // Skip auto-play for vertices we just recorded
            if app_state.skip_autoplay_vertex == Some(vertex_id) {
                app_state.skip_autoplay_vertex = None;
                return;
            }

            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                if let Some(mime) = &vertex.mime {
                    if mime.starts_with("audio/") && !vertex.label.is_empty() {
                        // Auto-play the audio
                        play_audio(&vertex.label, mime, vertex_id, &playback_state);
                    } else if app_state.tts_mode
                        && mime.starts_with("text/")
                        && mime != "text/gradesta-url"
                        && !vertex.label.is_empty()
                    {
                        // TTS mode: read text cells aloud
                        if let Ok(text) = String::from_utf8(vertex.label.clone()) {
                            let text = text.trim();
                            if !text.is_empty() {
                                tts::speak(text);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Automatically expand gradesta-url portals that are exactly 2 steps from cursor
/// This pre-loads content so it's ready when user navigates there
fn auto_expand_nearby_links(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    ws_cmd_tx: Res<WsCommandTx>,
) {
    let Some(current_id) = app_state.current_vertex else { return };
    let Some(cmd_tx) = &ws_cmd_tx.0 else { return };

    // FIRST: Check if current vertex doesn't exist locally - request it via WatchLandmark
    if !graph.vertices.contains_key(&current_id) {
        // We navigated to a vertex that doesn't exist in our local graph
        // This happens at landmark boundaries - request the data
        let landmark_url = if let Some(ref base_url) = app_state.base_ws_url {
            // Extract the base landmark and append vertex ID
            // The base_ws_url looks like "ws://localhost:8083/ws?landmark=notes://identity/"
            if let Some(landmark_start) = base_url.find("landmark=") {
                let landmark_base = &base_url[landmark_start + 9..];
                // Remove trailing parts after the landmark
                let landmark_base = landmark_base.split('&').next().unwrap_or(landmark_base);
                // Append vertex ID to landmark
                format!("{}{}", landmark_base.trim_end_matches('/'), current_id)
            } else {
                format!("vertex/{}", current_id)
            }
        } else {
            format!("vertex/{}", current_id)
        };

        if !app_state.requested_landmarks.contains(&landmark_url) {
            eprintln!("Requesting landmark for unknown vertex {}: {}", current_id, landmark_url);
            app_state.requested_landmarks.insert(landmark_url.clone());
            let action_id = app_state.next_action_id;
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url });
        }
        return;
    }

    let Some(current) = graph.vertices.get(&current_id) else { return };

    // Helper to build landmark URL for a vertex ID
    let build_landmark_url = |vertex_id: u64, base_url: &Option<String>| -> String {
        if let Some(ref base_url) = base_url {
            if let Some(landmark_start) = base_url.find("landmark=") {
                let landmark_base = &base_url[landmark_start + 9..];
                let landmark_base = landmark_base.split('&').next().unwrap_or(landmark_base);
                return format!("{}{}", landmark_base.trim_end_matches('/'), vertex_id);
            }
        }
        format!("vertex/{}", vertex_id)
    };

    // FIRST: If we're sitting on a portal, handle it
    // Layer 0 portals (text/gradesta-url as primary): auto-follow immediately (no visible label)
    // Layer 1 portals (text/plain primary, gradesta-url on layer 1): just preload, don't auto-follow (has visible label)
    let is_layer0_portal = current.mime.as_deref() == Some("text/gradesta-url");
    let layer1_portal_url = current.layers.get(&1)
        .filter(|l| l.mime == "text/gradesta-url")
        .map(|l| String::from_utf8_lossy(&l.data).to_string());

    if is_layer0_portal {
        // Layer 0 portal - auto-follow
        let landmark_url = String::from_utf8_lossy(&current.label).to_string();

        // Check if this landmark was already loaded by looking up vertices associated with it
        if let Some(vertices) = graph.landmark_vertices.get(&landmark_url) {
            // First, try to find the east neighbor of the portal (preferred direction for content)
            let east_id = current.edges[EDGE_EAST];
            if east_id != 0 {
                if let Some(vertex) = graph.vertices.get(&east_id) {
                    if vertex.mime.as_deref() != Some("text/gradesta-url") {
                        // Found content vertex to the east - jump to it
                        app_state.history.push(current_id);
                        app_state.current_vertex = Some(east_id);
                        app_state.following_portal = None;
                        return;
                    }
                }
            }

            // Fallback: find the first non-portal vertex in this landmark
            for &vid in vertices {
                if let Some(vertex) = graph.vertices.get(&vid) {
                    if vertex.mime.as_deref() != Some("text/gradesta-url") {
                        // Found a content vertex - jump to it
                        app_state.history.push(current_id);
                        app_state.current_vertex = Some(vid);
                        app_state.following_portal = None;
                        return;
                    }
                }
            }
        }

        // Not loaded yet - request it
        if !app_state.requested_landmarks.contains(&landmark_url) {
            app_state.requested_landmarks.insert(landmark_url.clone());
            let action_id = app_state.next_action_id;
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url.clone() });
        }

        // Set up to jump when it loads
        if app_state.following_portal.is_none() {
            app_state.following_portal = Some(landmark_url);
        }
        return;
    } else if let Some(landmark_url) = layer1_portal_url {
        // Layer 1 portal - just preload the landmark, don't auto-follow
        // This allows the user to see the text label and navigate east manually
        if !app_state.requested_landmarks.contains(&landmark_url) {
            app_state.requested_landmarks.insert(landmark_url.clone());
            let action_id = app_state.next_action_id;
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url });
        }
        // Don't return - continue to preload neighbors
    }

    // SECOND: Preload immediate neighbors (1 step away) that we don't have
    for edge in current.edges {
        if edge == 0 {
            continue;
        }
        if !graph.vertices.contains_key(&edge) {
            // This edge points to a vertex we don't have - request it
            let landmark_url = build_landmark_url(edge, &app_state.base_ws_url);
            if !app_state.requested_landmarks.contains(&landmark_url) {
                eprintln!("Preloading nearby unknown vertex {}: {}", edge, landmark_url);
                app_state.requested_landmarks.insert(landmark_url.clone());
                let action_id = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url });
                return; // Only one per frame
            }
        }
    }

    // THIRD: Collect vertices that are exactly 2 steps away
    let mut two_steps_away: Vec<u64> = Vec::new();

    // For each immediate neighbor (1 step)
    for edge1 in current.edges {
        if edge1 == 0 {
            continue;
        }
        if let Some(neighbor) = graph.vertices.get(&edge1) {
            // For each of that neighbor's neighbors (2 steps)
            for edge2 in neighbor.edges {
                if edge2 != 0 && edge2 != current_id {
                    two_steps_away.push(edge2);
                }
            }
        }
    }

    // Request unknown vertices that are 2 steps away
    for vid in &two_steps_away {
        if !graph.vertices.contains_key(vid) {
            let landmark_url = build_landmark_url(*vid, &app_state.base_ws_url);
            if !app_state.requested_landmarks.contains(&landmark_url) {
                eprintln!("Preloading 2-step unknown vertex {}: {}", vid, landmark_url);
                app_state.requested_landmarks.insert(landmark_url.clone());
                let action_id = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url });
                return; // Only one per frame
            }
        }
    }

    // Also check for portal vertices 2 steps away
    for vid in two_steps_away {
        if let Some(vertex) = graph.vertices.get(&vid) {
            // Check both layer 0 and layer 1 for gradesta-url
            let landmark_url = if vertex.mime.as_deref() == Some("text/gradesta-url") {
                Some(String::from_utf8_lossy(&vertex.label).to_string())
            } else if let Some(layer1) = vertex.layers.get(&1) {
                if layer1.mime == "text/gradesta-url" {
                    Some(String::from_utf8_lossy(&layer1.data).to_string())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(landmark_url) = landmark_url {
                // Skip if already requested
                if app_state.requested_landmarks.contains(&landmark_url) {
                    continue;
                }

                // Mark as requested and send - only one per frame
                app_state.requested_landmarks.insert(landmark_url.clone());
                let action_id = app_state.next_action_id;
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                let _ = cmd_tx.send(WsCommand::WatchLandmark { action_id, landmark: landmark_url });
                return; // Only one per frame
            }
        }
    }
}
