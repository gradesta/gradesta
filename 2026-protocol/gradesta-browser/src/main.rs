use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts, EguiContextSettings, EguiPlugin, EguiStartupSet, EguiPrimaryContextPass};
use crossbeam_channel::{unbounded, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

// Core modules
mod audio;
mod audio_processing;
mod commands;
mod debug_log;
mod elf_http;
mod events;
mod export;
mod gamepad;
mod graph;
mod identity;
mod keybindings;
mod landmark;
mod local_services;
mod media;
mod network;
mod rendering;
mod sidebar;
mod state;
mod tts;
mod ui;
mod video_player;
mod voice_command;
mod whisper;

// Imports from refactored modules
use audio::{AudioPlaybackState, AudioPreloadCache, AudioProcessingChannel, AudioRecordingSignal};
use audio::{play_audio_fast, predecode_audio_async, stop_audio};
use audio_processing::set_audio_speed;
use graph::{direction_priority_order, GraphState};
use landmark::build_landmark_url;
use media::MediaCache;
use network::{run_ws, NetEventsTx, NetRx, ServerEvent, WsCommand, WsCommandTx};
use state::{AppState, InputMode, NextcloudLoginState, PlaybackBoostState};
use state::PendingIdentitySetup;
use state::{EDGE_DOWN, EDGE_EAST, EDGE_NORTH, EDGE_SOUTH, EDGE_UP, EDGE_WEST};
use state::{KEY_REPEAT_DELAY, KEY_REPEAT_RATE};
use voice_command::{ModelFetchChannel, VoiceCommandChannel, VoiceCommandConfig, VoiceCommandConfigRes, VoiceCommandState};

// Existing module imports
use commands::Command;
use identity::Identity;

/// Resource for sending elf HTTP events
#[derive(Resource)]
pub struct ElfHttpTx(pub Sender<elf_http::ElfHttpEvent>);

/// Resource for receiving elf HTTP events
#[derive(Resource)]
pub struct ElfHttpRx(pub Receiver<elf_http::ElfHttpEvent>);

/// Determine the current keybinding context from app state
fn current_context(app_state: &AppState) -> commands::Context {
    match app_state.input_mode {
        InputMode::TextInput { .. } => commands::Context::TextInput,
        InputMode::InlineEdit { .. } => commands::Context::TextInput, // Inline edit uses text input context
        InputMode::Recording { .. } => commands::Context::Recording,
        InputMode::VoiceCommand(_) => commands::Context::Global, // Voice command has its own handling
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

    // Initialize gamepad support
    gamepad::init_global_gamepad();

    let (net_tx, net_rx) = unbounded::<ServerEvent>();
    let (elf_tx, elf_rx) = unbounded::<elf_http::ElfHttpEvent>();

    // Create audio resources with shared playing_vertex
    let audio_playback_state = AudioPlaybackState::default();
    let audio_preload_cache = AudioPreloadCache::new(audio_playback_state.playing_vertex.clone());

    // Fetch manifests for all pre-loaded trusted elves
    for elf in &app_state.trusted_elves {
        eprintln!("Fetching manifest for pre-loaded elf: {}", elf.url);
        elf_http::fetch_manifest_async(elf.url.clone(), elf_tx.clone());
    }

    App::new()
        .insert_resource(NetRx(net_rx))
        .insert_resource(NetEventsTx(net_tx))
        .insert_resource(WsCommandTx(None))
        .insert_resource(ElfHttpTx(elf_tx))
        .insert_resource(ElfHttpRx(elf_rx))
        .insert_resource(GraphState::default())
        .insert_resource(app_state)
        .insert_resource(MediaCache::default())
        .insert_resource(AudioRecordingSignal::default())
        .insert_resource(audio_playback_state)
        .insert_resource(audio_preload_cache)
        .insert_resource(AudioProcessingChannel::default())
        .insert_resource(PlaybackBoostState::default())
        .insert_resource(VoiceCommandChannel::default())
        .insert_resource(VoiceCommandConfigRes(VoiceCommandConfig::load()))
        .insert_resource(ModelFetchChannel::default())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gradesta Browser".to_string(),
                resolution: bevy::window::WindowResolution::new(1400, 900),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_systems(Startup, setup.after(EguiStartupSet::InitContexts))
        .add_systems(PreUpdate, sync_egui_scale_factor)
        .add_systems(EguiPrimaryContextPass, ui_system)
        .add_systems(Update, (
            events::ingest_server_events,
            events::process_audio_results,
            events::update_recording_audio_levels,
            events::request_content_for_visible_cells,
            process_elf_http_events,
            (handle_navigation, auto_play_audio_on_navigate).chain(),
            auto_expand_nearby_links,
            preload_nearby_audio,
            playback_boost_decay,
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
        Box::new(|_cc| Ok(Box::new(DownloadApp { progress }))),
    );
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Sync bevy_egui's scale_factor with the window's DPI scale factor
/// This fixes pointer coordinate mismatch on HiDPI displays
fn sync_egui_scale_factor(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut egui_settings: Query<&mut EguiContextSettings>,
) {
    let Ok(window) = windows.single() else { return };
    let window_scale = window.scale_factor();

    for mut settings in egui_settings.iter_mut() {
        if (settings.scale_factor - window_scale).abs() > 0.01 {
            eprintln!("Syncing egui scale_factor: {} -> {}", settings.scale_factor, window_scale);
            settings.scale_factor = window_scale;
        }
    }
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
    elf_http_tx: Res<ElfHttpTx>,
    audio_processing: Res<AudioProcessingChannel>,
    mut boost_state: ResMut<PlaybackBoostState>,
    voice_channel: Res<VoiceCommandChannel>,
    mut voice_config: ResMut<VoiceCommandConfigRes>,
    model_fetch_channel: Res<ModelFetchChannel>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    // Clone the model fetch tx for use in button handlers
    let model_fetch_tx = model_fetch_channel.tx.clone();

    // Process model fetch results
    while let Ok(result) = model_fetch_channel.rx.try_recv() {
        app_state.model_fetch_state.loading = false;
        match result {
            Ok(models) => {
                app_state.model_fetch_state.models = models;
                app_state.model_fetch_state.error = None;
            }
            Err(e) => {
                app_state.model_fetch_state.error = Some(e);
            }
        }
    }

    // Debug: check input state periodically and on pointer activity
    static DEBUG_FRAME: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = DEBUG_FRAME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if frame == 0 {
        // Print screen info once at startup
        let screen = ctx.screen_rect();
        let ppp = ctx.pixels_per_point();
        eprintln!("Screen rect: {:?}, pixels_per_point: {}", screen, ppp);
    }
    ctx.input(|i| {
        let show_debug = frame % 60 == 0 || i.pointer.any_down() || i.pointer.any_click() || i.pointer.any_released();
        if show_debug {
            eprintln!("Frame {}: down={}, click={}, released={}, pos={:?}, hover_pos={:?}",
                frame, i.pointer.any_down(), i.pointer.any_click(), i.pointer.any_released(),
                i.pointer.interact_pos(), i.pointer.hover_pos());
        }
    });

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

    // Update gamepad state
    gamepad::update_global_gamepad();
    let gamepad_snapshot = gamepad::get_gamepad_snapshot();

    let mut cmds = ui::capture_keyboard_commands(ctx, &app_state.keybindings, kb_context);

    // Check if we're in voice command mode for gamepad handling
    let is_in_voice_command_mode = matches!(app_state.input_mode, InputMode::VoiceCommand(_));

    // Merge gamepad commands (only in graph context, not during text input)
    if kb_context == commands::Context::Graph || kb_context == commands::Context::Recording || is_in_voice_command_mode {
        ui::capture_gamepad_commands(&mut cmds, &gamepad_snapshot, &app_state.keybindings);
    }

    // Capture text input gamepad commands (Circle to cancel, Cross to submit)
    if kb_context == commands::Context::TextInput {
        ui::capture_text_input_gamepad(&mut cmds, &gamepad_snapshot);
    }

    // Always capture context menu open command (works globally like a menu button)
    if let Some(ref gp) = gamepad_snapshot {
        // Debug: log any button presses
        if !gp.pressed_this_frame.is_empty() {
            eprintln!("Gamepad buttons pressed this frame: {:?}", gp.pressed_this_frame);
        }
        if app_state.keybindings.command_pressed_gamepad(&commands::Command::GlobalOpenContextMenu, gp) {
            eprintln!("Captured GlobalOpenContextMenu from gamepad");
            cmds.add(commands::Command::GlobalOpenContextMenu);
        }
    }

    // Capture voice command specific gamepad inputs (L2 hold for voice, right stick for selection)
    let recording_start = app_state.recording_start;
    ui::capture_voice_command_gamepad(&mut cmds, &gamepad_snapshot, is_in_voice_command_mode, &mut app_state.l2_press_start, recording_start);

    // Capture context menu specific gamepad inputs (right stick + face buttons when menu open)
    ui::capture_context_menu_gamepad(&mut cmds, &gamepad_snapshot, app_state.context_menu.open);

    // Capture sidebar gamepad inputs (for sidebar panel navigation)
    let sidebar_gamepad_input = ui::capture_sidebar_gamepad(&gamepad_snapshot);

    // Log triggered commands to debug log (separated to avoid borrow conflicts)
    ui::log_triggered_commands_to_debug(&cmds, &mut app_state);
    let cmd_refresh = cmds.has(commands::Command::GlobalRefresh);  // Used later for refresh logic

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
        &mut boost_state,
        &voice_channel,
        ctx,
    );

    // Process text editing commands (copy, cut, paste, undo, redo)
    ui::process_text_edit_commands(&cmds, &mut app_state);

    // Finalize recording if needed
    if cmd_results.should_finalize_recording {
        ui::finalize_recording(&mut app_state, &audio_signal, &audio_processing);
    }

    // Finalize voice command recording if needed
    if cmd_results.should_finalize_voice_recording {
        ui::finalize_voice_recording(&mut app_state, &voice_channel, &voice_config.0);
    }

    // Grant voice permission if user confirmed with L3
    if cmd_results.should_grant_voice_permission {
        ui::grant_voice_permission(&mut app_state, &graph, &voice_channel, &voice_config.0);
    }

    // Handle context menu command if one was selected
    // This creates a new CapturedCommands with just that command and processes it
    if let Some(cmd) = cmd_results.context_menu_command {
        let mut menu_cmds = ui::CapturedCommands::default();
        menu_cmds.commands.insert(cmd);
        let _ = ui::execute_commands(
            &menu_cmds,
            &mut app_state,
            &mut graph,
            &ws_cmd_tx,
            &mut media_cache,
            &audio_signal,
            &playback_state,
            &mut boost_state,
            &voice_channel,
            ctx,
        );
    }

    // Process voice command events from async operations
    ui::process_voice_command_events(&mut app_state, &graph, &voice_channel, &voice_config.0);

    // Handle voice command confirm/action execution (R3/A button)
    if cmds.voice_confirm {
        eprintln!("voice_confirm triggered, input_mode: {:?}", std::mem::discriminant(&app_state.input_mode));
        if let InputMode::VoiceCommand(ref state) = app_state.input_mode {
            eprintln!("In VoiceCommand mode, state: {:?}", std::mem::discriminant(state));
            match state {
                VoiceCommandState::Selecting { ref interpretations, selected, .. } => {
                    eprintln!("Executing voice action, {} interpretations, selected: {}", interpretations.len(), selected);
                    ui::execute_voice_action(
                        &mut app_state,
                        &mut graph,
                        &ws_cmd_tx,
                        &mut media_cache,
                        &audio_signal,
                        &playback_state,
                        &mut boost_state,
                        &voice_channel,
                        ctx,
                    );
                }
                VoiceCommandState::AwaitingPermission { selected, .. } => {
                    // R3/A confirms the currently selected button
                    if *selected == 0 {
                        ui::grant_voice_permission(&mut app_state, &graph, &voice_channel, &voice_config.0);
                    } else {
                        app_state.input_mode = InputMode::Normal;
                        app_state.status = "Permission denied, voice command cancelled".to_string();
                    }
                }
                _ => {}
            }
        }
    }
    if cmds.voice_cancel {
        eprintln!("voice_cancel triggered");
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
        return Ok(()); // Skip the rest of the normal UI
    }

    // NORMAL MODE: Render full UI with panels
    // Top panel with URL bar (split into server and landmark)
    // Store server_rect for dropdown positioning (outside the panel closure)
    let mut server_rect_for_dropdown: Option<egui::Rect> = None;
    let mut should_connect = false;

    // Pre-process dropdown keyboard navigation BEFORE TextEdit consumes the keys
    // We use the previous frame's dropdown state to decide if we should intercept
    let dropdown_has_items = !app_state.server_dropdown.filtered_indices.is_empty();
    let dropdown_is_active = app_state.server_dropdown.is_active;
    let mut dropdown_selection_made: Option<String> = None;

    if app_state.server_bar_has_focus && dropdown_has_items {
        // Always consume arrow keys when dropdown is visible (to navigate it)
        let arrow_down = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
        let arrow_up = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
        // Only consume Enter if dropdown is actively being navigated
        let enter = if dropdown_is_active {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
        } else {
            false
        };
        let escape = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));

        // Gamepad navigation for dropdown (right stick Y-axis + A button or R3)
        let (gamepad_down, gamepad_up, gamepad_select) = if let Some(ref gp) = gamepad_snapshot {
            const STICK_DEADZONE: f32 = 0.5;
            let (_rx, ry) = gp.right_stick;
            (
                ry < -STICK_DEADZONE,  // Down
                ry > STICK_DEADZONE,   // Up
                gp.is_pressed(keybindings::key::GamepadKey::South)  // A/Cross
                    || gp.is_pressed(keybindings::key::GamepadKey::RightStick),  // R3
            )
        } else {
            (false, false, false)
        };

        let max_idx = app_state.server_dropdown.filtered_indices.len().saturating_sub(1);

        // Arrow keys or gamepad down activates the dropdown navigation
        if arrow_down || gamepad_down {
            app_state.server_dropdown.is_active = true;
            app_state.server_dropdown.selected_index = (app_state.server_dropdown.selected_index + 1).min(max_idx);
        }
        if arrow_up || gamepad_up {
            app_state.server_dropdown.is_active = true;
            app_state.server_dropdown.selected_index = app_state.server_dropdown.selected_index.saturating_sub(1);
        }
        // Enter or A button selects from dropdown only if dropdown is active
        if (enter || gamepad_select) && dropdown_is_active {
            if let Some(&server_idx) = app_state.server_dropdown.filtered_indices.get(app_state.server_dropdown.selected_index) {
                if let Some(ref services) = app_state.local_services {
                    if let Some(server) = services.servers.get(server_idx) {
                        dropdown_selection_made = Some(server.url.clone());
                    }
                }
            }
        }
        // Escape deactivates dropdown
        if escape {
            app_state.server_dropdown.is_active = false;
            app_state.server_dropdown.filtered_indices.clear();
            app_state.server_dropdown.selected_index = 0;
        }
    }

    // Apply dropdown selection (fills URL and triggers connect)
    if let Some(url) = dropdown_selection_made {
        app_state.server_input = url;
        app_state.server_dropdown.filtered_indices.clear();
        app_state.server_dropdown.selected_index = 0;
        app_state.server_dropdown.is_active = false;
        should_connect = true;
    }

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            // Calculate flexible widths
            let available = ui.available_width();
            // Reserve space for labels (~50 + ~70), buttons (~80 + ~40), spacing
            let reserved = 280.0;
            let input_space = (available - reserved).max(200.0);
            let server_width = input_space * 0.4;
            let landmark_width = input_space * 0.6;

            // Server input
            ui.label("Server:");
            let server_bar_id = egui::Id::new("server_bar");
            let should_focus = app_state.focus_url_bar_next_frame;
            let server_edit = egui::TextEdit::singleline(&mut app_state.server_input)
                .id(server_bar_id)
                .desired_width(server_width)
                .lock_focus(should_focus)
                .hint_text("ws://localhost:8080");
            let server_response = ui.add(server_edit);

            // Request focus AFTER the TextEdit is rendered (ensures widget is in used_ids)
            if should_focus {
                ui.ctx().memory_mut(|mem| mem.request_focus(server_bar_id));
                eprintln!("Requested focus on server_bar");
            }

            // Check focus state after potential request
            let has_focus = ui.ctx().memory(|mem| mem.has_focus(server_bar_id));
            app_state.server_bar_has_focus = has_focus;

            // Debug: print focus state when it changes or when we requested focus
            if should_focus || server_response.clicked() {
                eprintln!("server_bar: should_focus={}, clicked={}, has_focus={}, response.has_focus={}",
                    should_focus, server_response.clicked(), has_focus, server_response.has_focus());
            }

            // Select all text when focused via Ctrl+L, then clear the flag
            if should_focus && has_focus {
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), server_bar_id) {
                    let text_len = app_state.server_input.len();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(text_len),
                    )));
                    state.store(ui.ctx(), server_bar_id);
                }
                // Clear the flag now that focus is confirmed
                app_state.focus_url_bar_next_frame = false;
            }

            // Reset dropdown active state when user types (input changes)
            if server_response.changed() {
                app_state.server_dropdown.is_active = false;
                app_state.server_dropdown.selected_index = 0;
            }

            // Store rect for dropdown positioning
            server_rect_for_dropdown = Some(server_response.rect);

            ui.add_space(8.0);

            // Landmark input
            ui.label("Landmark:");
            let landmark_bar_id = egui::Id::new("landmark_bar");
            let landmark_edit = egui::TextEdit::singleline(&mut app_state.landmark_input)
                .id(landmark_bar_id)
                .desired_width(landmark_width)
                .hint_text("/");
            let landmark_response = ui.add(landmark_edit);
            app_state.landmark_bar_has_focus = landmark_response.has_focus();

            // Track combined URL bar focus state
            app_state.url_bar_has_focus = app_state.server_bar_has_focus || app_state.landmark_bar_has_focus;

            // Right-align buttons
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Copy button (renders first = rightmost)
                if ui.button("📋").clicked() {
                    let full_url = ui::url_utils::construct_full_url(&app_state.server_input, &app_state.landmark_input);
                    ui::url_utils::set_clipboard_text(&full_url);
                    app_state.status = "Copied URL to clipboard".to_string();
                }

                // Connect/Refresh button
                let button_label = if app_state.connected { "Refresh" } else { "Connect" };
                if ui.button(button_label).clicked() {
                    should_connect = true;
                }
            });

            // Enter pressed in either input triggers connection
            // But only if dropdown is empty (not selecting from autocomplete)
            let dropdown_empty = app_state.server_dropdown.filtered_indices.is_empty();
            let server_enter = server_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && dropdown_empty;
            let landmark_enter = landmark_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            if server_enter || landmark_enter {
                should_connect = true;
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

    // Server dropdown autocomplete logic
    // Step 1: Build filtered list of servers (extract data to avoid borrow issues)
    let dropdown_items: Vec<(usize, String, String)> = if app_state.server_bar_has_focus {
        if let Some(ref services) = app_state.local_services {
            let input_lower = app_state.server_input.to_lowercase();
            let filtered: Vec<_> = services.servers
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    input_lower.is_empty()
                    || s.name.to_lowercase().contains(&input_lower)
                    || s.url.to_lowercase().contains(&input_lower)
                })
                .take(8)
                .map(|(i, s)| (i, s.name.clone(), s.url.clone()))
                .collect();

            // If nothing matches, show all servers as fallback
            if filtered.is_empty() {
                services.servers
                    .iter()
                    .enumerate()
                    .take(8)
                    .map(|(i, s)| (i, s.name.clone(), s.url.clone()))
                    .collect()
            } else {
                filtered
            }
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Step 2: Update filtered indices and reset active state when needed
    if app_state.server_bar_has_focus {
        app_state.server_dropdown.filtered_indices = dropdown_items.iter().map(|(i, _, _)| *i).collect();
    } else {
        // Reset all dropdown state when losing focus
        app_state.server_dropdown.filtered_indices.clear();
        app_state.server_dropdown.selected_index = 0;
        app_state.server_dropdown.is_active = false;
    }

    // Step 3: Render dropdown (keyboard handled above before panel)
    let mut clicked_server_url: Option<String> = None;
    if app_state.server_bar_has_focus && !dropdown_items.is_empty() && !app_state.server_dropdown.filtered_indices.is_empty() {
        if let Some(server_rect) = server_rect_for_dropdown {
            let selected_index = app_state.server_dropdown.selected_index;
            let is_active = app_state.server_dropdown.is_active;

            egui::Area::new("server_dropdown".into())
                .fixed_pos(egui::pos2(server_rect.left(), server_rect.bottom() + 2.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    // Use different frame style based on active state
                    let frame = if is_active {
                        egui::Frame::popup(ui.style())
                            .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 149, 237))) // Blue border when active
                    } else {
                        egui::Frame::popup(ui.style())
                    };
                    frame.show(ui, |ui| {
                        // Show hint when not active
                        if !is_active {
                            ui.label(egui::RichText::new("↓ to navigate, Enter to select").small().weak());
                            ui.separator();
                        }
                        for (i, (_, name, url)) in dropdown_items.iter().enumerate() {
                            // Only highlight selection when dropdown is active
                            let selected = is_active && i == selected_index;
                            let text = format!("{} - {}", name, url);
                            if ui.selectable_label(selected, &text).clicked() {
                                clicked_server_url = Some(url.clone());
                            }
                        }
                    });
                });
        }
    }

    // Apply clicked selection (fills URL and triggers connect)
    if let Some(url) = clicked_server_url {
        app_state.server_input = url;
        app_state.server_dropdown.filtered_indices.clear();
        app_state.server_dropdown.selected_index = 0;
        app_state.server_dropdown.is_active = false;
        should_connect = true;
    }

    // Connect/refresh on button click, Enter, GlobalRefresh command, or voice command
    let voice_refresh = app_state.voice_refresh_pending;
    if voice_refresh {
        app_state.voice_refresh_pending = false;
    }
    if should_connect || cmd_refresh || voice_refresh {
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
            graph.landmark_mgr.clear();
            app_state.current_vertex = None;
            app_state.history.clear();

            // Clear focus from URL bar so user can navigate the graph
            ctx.memory_mut(|mem| mem.surrender_focus(egui::Id::new("server_bar")));
            ctx.memory_mut(|mem| mem.surrender_focus(egui::Id::new("landmark_bar")));
            app_state.server_bar_has_focus = false;
            app_state.landmark_bar_has_focus = false;
            app_state.url_bar_has_focus = false;

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

    // Bottom panel with navigation help
    let bottom_response = egui::TopBottomPanel::bottom("help_panel").show(ctx, |ui| {
        // Debug: print panel rect once
        static PRINTED_RECT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !PRINTED_RECT.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("Bottom panel clip_rect: {:?}", ui.clip_rect());
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let mut help_text = "↑↓←→ Nav | Enter=Click | Space=Record | I=Edit | Y=Yank | Ctrl+K=Keybindings".to_string();
            if gamepad::is_gamepad_connected() {
                help_text.push_str(" | L3=Gamepad Help");
            }
            ui.label(help_text);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // TTS mode indicator
                let tts_label = if app_state.tts_mode { "🔊 TTS ON" } else { "🔇 TTS" };
                let tts_response = ui.button(tts_label).on_hover_text("Toggle text-to-speech (Ctrl+T)");

                // Debug: print button rect once
                static PRINTED_BTN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                if !PRINTED_BTN.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    eprintln!("TTS button rect: {:?}", tts_response.rect);
                }

                // Debug: check interaction state
                if tts_response.hovered() {
                    eprintln!("TTS button hovered");
                }
                if tts_response.clicked() {
                    eprintln!("TTS button CLICKED!");
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
                // Elf button
                let elf_label = if app_state.show_elf_panel { "🧝 Elves ON" } else { "🧝 Elves" };
                if ui.button(elf_label).on_hover_text("Toggle elf panel (Ctrl+E)").clicked() {
                    app_state.show_elf_panel = !app_state.show_elf_panel;
                    if app_state.show_elf_panel {
                        app_state.show_bag_panel = false;
                        app_state.show_nav_panel = false;
                    }
                }
                ui.separator();
                // Voice settings button
                if ui.button("⚙ Settings").on_hover_text("Configure API keys and LLM model").clicked() {
                    app_state.show_voice_settings = true;
                    // Trigger model fetch if not already loaded
                    if app_state.model_fetch_state.models.is_empty() && !app_state.model_fetch_state.loading {
                        app_state.model_fetch_state.loading = true;
                        voice_command::fetch_models_from_requesty(model_fetch_tx.clone());
                    }
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
    match ui::render_command_bar(ctx, &mut app_state, &voice_channel, &voice_config.0) {
        ui::CommandBarAction::Execute(cmd) => {
            // Execute command through the standard command execution path
            let mut cmds = ui::CapturedCommands::default();
            cmds.add(cmd);
            ui::execute_commands(
                &cmds,
                &mut app_state,
                &mut graph,
                &ws_cmd_tx,
                &mut media_cache,
                &audio_signal,
                &playback_state,
                &mut boost_state,
                &voice_channel,
                ctx,
            );
        }
        ui::CommandBarAction::ExecuteScript(action) => {
            // Execute LLM-generated script through voice action path
            // First set up a temporary Selecting state for execute_voice_action
            use voice_command::{AgentInterpretation, VoiceCommandState};

            let interp = AgentInterpretation {
                action: action.clone(),
                confidence: 1.0,
                explanation: "Command bar LLM action".to_string(),
            };

            app_state.input_mode = InputMode::VoiceCommand(VoiceCommandState::Selecting {
                transcript: String::new(),
                interpretations: vec![interp],
                selected: 0,
            });

            ui::execute_voice_action(
                &mut app_state,
                &mut graph,
                &ws_cmd_tx,
                &mut media_cache,
                &audio_signal,
                &playback_state,
                &mut boost_state,
                &voice_channel,
                ctx,
            );
        }
        ui::CommandBarAction::None => {}
    }

    // Right panel for content - renders based on sidebar mode
    let sidebar_action = egui::SidePanel::right("preview_panel").min_width(400.0).show(ctx, |ui| {
        ui::render_sidebar_content(ui, ctx, &mut app_state, &graph, &mut media_cache, &ws_cmd_tx, &playback_state, &sidebar_gamepad_input)
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
                let action_id = app_state.next_action_id;
                if graph.landmark_mgr.watch_and_follow(&landmark, action_id, tx) {
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                }
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
        ui::SidebarContentAction::ElfAction(elf_action) => {
            handle_elf_action(elf_action, &mut app_state, &ws_cmd_tx, &graph, &elf_http_tx);
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
    ui::process_identification(ctx, &mut app_state, &ws_cmd_tx, &cmds);

    // Central panel showing grid view
    let grid_action = ui::render_grid_view(ctx, &mut app_state, &graph, &mut media_cache, &ws_cmd_tx);

    // Handle grid actions
    match grid_action {
        ui::GridAction::SaveInlineEdit { vertex_id, content } => {
            // Check if this is a new cell (placeholder) or existing cell
            if let InputMode::InlineEdit { is_new, .. } = app_state.input_mode {
                if is_new {
                    // New cell: send CreateVertex with the text content
                    // vertex_id is actually a local_id for new cells
                    let local_id = vertex_id;

                    if let Some(placeholder) = app_state.pending_audio_cells.get(&local_id) {
                        if let Some(ref tx) = ws_cmd_tx.0 {
                            let direction = placeholder.direction;
                            let from_vertex = placeholder.from_vertex;

                            let dir_byte = match direction {
                                state::EDGE_WEST => 0,
                                state::EDGE_EAST => 1,
                                state::EDGE_NORTH => 2,
                                state::EDGE_SOUTH => 3,
                                state::EDGE_UP => 4,
                                state::EDGE_DOWN => 5,
                                _ => 3, // default south
                            };

                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                            let text_bytes = content.into_bytes();

                            // Send CreateVertex to server
                            let _ = tx.send(WsCommand::CreateVertex {
                                action_id,
                                from_vertex,
                                direction: dir_byte,
                                layer: 0,
                                mime: "text/plain".to_string(),
                                data: text_bytes.clone(),
                            });

                            // Track pending creation
                            app_state.pending_creations.insert(action_id, state::PendingVertexCreation {
                                samples: Vec::new(),
                                sample_rate: 0,
                                data: text_bytes,
                                mime: "text/plain".to_string(),
                                local_placeholder_id: Some(local_id),
                            });

                            // Update the placeholder with action_id
                            if let Some(cell) = app_state.pending_audio_cells.get_mut(&local_id) {
                                cell.action_id = Some(action_id);
                            }

                            app_state.status = "Creating cell...".to_string();
                        }
                    }
                } else {
                    // Existing cell: send SetVertexLabel
                    if let Some(ref tx) = ws_cmd_tx.0 {
                        let action_id = app_state.next_action_id;
                        app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                        let text_bytes = content.into_bytes();
                        let _ = tx.send(WsCommand::SetVertexLabel {
                            action_id,
                            vertex_id,
                            layer: 0,
                            mime: "text/plain".to_string(),
                            data: text_bytes.clone(),
                        });
                        // Update local graph state optimistically
                        if let Some(vertex) = graph.vertices.get_mut(&vertex_id) {
                            vertex.layers.insert(0, crate::graph::LayerContent {
                                mime: "text/plain".to_string(),
                                data: text_bytes,
                            });
                        }
                        app_state.status = "Saved".to_string();
                    }
                }
            }
            app_state.input_mode = InputMode::Normal;
            app_state.text_input_buffer.clear();
            app_state.inline_edit_original = None;
        }
        ui::GridAction::ClickVertex | ui::GridAction::None => {}
    }

    // Voice command overlay (rendered on top of everything except gamepad help)
    if let InputMode::VoiceCommand(ref state) = app_state.input_mode {
        ui::render_voice_command_overlay(ctx, state, app_state.recording_start);
    }

    // Context menu overlay (gamepad, rendered on top)
    ui::render_context_menu(ctx, &app_state);

    // Gamepad help overlay (rendered last so it's on top)
    if app_state.show_gamepad_help {
        ui::render_gamepad_help_overlay(ctx);
    }

    // Voice settings dialog (modal, rendered on top of everything)
    if app_state.show_voice_settings {
        let mut config = voice_config.0.clone();
        let model_state = app_state.model_fetch_state.clone();
        let mut filter = app_state.model_filter.clone();
        let action = ui::render_voice_settings_dialog(
            ctx,
            &mut config,
            &model_state,
            &mut filter,
        );
        app_state.model_filter = filter;
        // Always update config so changes persist across frames (before Save)
        voice_config.0 = config;
        match action {
            ui::VoiceSettingsAction::Close => {
                app_state.show_voice_settings = false;
            }
            ui::VoiceSettingsAction::Save(new_config) => {
                if let Err(e) = new_config.save() {
                    app_state.status = format!("Failed to save voice settings: {}", e);
                } else {
                    app_state.status = "Voice settings saved".to_string();
                }
                voice_config.0 = new_config;
                app_state.show_voice_settings = false;
            }
            ui::VoiceSettingsAction::FetchModels => {
                if !app_state.model_fetch_state.loading {
                    app_state.model_fetch_state.loading = true;
                    app_state.model_fetch_state.error = None;
                    voice_command::fetch_models_from_requesty(model_fetch_tx.clone());
                }
            }
            ui::VoiceSettingsAction::None => {}
        }
    }

    Ok(())
}

/// Handle actions from the elf panel
fn handle_elf_action(
    action: sidebar::elf::ElfAction,
    app_state: &mut AppState,
    ws_cmd_tx: &WsCommandTx,
    graph: &GraphState,
    elf_http_tx: &ElfHttpTx,
) {
    use sidebar::elf::ElfAction;
    use crate::state::{TrustedElf, ElfTask};

    match action {
        ElfAction::AddElf(url) => {
            // Add new trusted elf
            let elf = TrustedElf::new(&url);
            app_state.trusted_elves.push(elf);
            app_state.status = format!("Added elf: {}", url);
            // Fetch manifest asynchronously
            elf_http::fetch_manifest_async(url, elf_http_tx.0.clone());
        }
        ElfAction::RemoveElf(index) => {
            if index < app_state.trusted_elves.len() {
                let removed = app_state.trusted_elves.remove(index);
                app_state.status = format!("Removed elf: {}", removed.url);
                // Reset selection if needed
                if app_state.elf_panel.selected_elf_index >= app_state.trusted_elves.len() {
                    app_state.elf_panel.selected_elf_index = app_state.trusted_elves.len().saturating_sub(1);
                }
            }
        }
        ElfAction::RefreshManifest(index) => {
            if let Some(elf) = app_state.trusted_elves.get(index) {
                app_state.status = format!("Refreshing manifest for {}...", elf.url);
                // Fetch manifest asynchronously
                elf_http::fetch_manifest_async(elf.url.clone(), elf_http_tx.0.clone());
            }
        }
        ElfAction::Summon { elf_index, command_index, directions, permissions } => {
            // Get current vertex and landmark for summoning
            let current_vertex = app_state.current_vertex;
            let current_landmark = graph.context_uri.clone().unwrap_or_default();

            if let (Some(vertex_id), Some(elf)) = (current_vertex, app_state.trusted_elves.get(elf_index)) {
                if let Some(manifest) = &elf.manifest {
                    if let Some(command) = manifest.commands.get(command_index) {
                        // Send introduce elf request via WebSocket
                        if let Some(ref tx) = ws_cmd_tx.0 {
                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                            let _ = tx.send(WsCommand::IntroduceElf {
                                action_id,
                                elf_url: elf.url.clone(),
                                command: command.name.clone(),
                                cursor_landmark: current_landmark.clone(),
                                cursor_vertex: vertex_id,
                                origin_landmark: current_landmark.clone(),
                                origin_vertex: vertex_id,
                                allowed_directions: directions,
                                max_depth: -1, // Unlimited depth
                                permissions,
                                params: std::collections::HashMap::new(),
                            });

                            // Track the task
                            app_state.active_elf_tasks.insert(action_id, ElfTask::new(
                                &elf.url,
                                &command.name,
                            ));

                            app_state.status = format!("Summoning {}::{}...", manifest.name, command.name);
                        } else {
                            app_state.status = "Not connected to server".to_string();
                        }
                    }
                }
            } else {
                app_state.status = "No vertex selected or elf not configured".to_string();
            }
        }
        ElfAction::Close => {
            app_state.show_elf_panel = false;
        }
    }
}

/// Process elf HTTP events (manifest fetches, summon responses)
fn process_elf_http_events(
    mut app_state: ResMut<AppState>,
    elf_http_rx: Res<ElfHttpRx>,
) {
    // Process all pending elf HTTP events
    while let Ok(event) = elf_http_rx.0.try_recv() {
        match event {
            elf_http::ElfHttpEvent::ManifestFetched { elf_url, manifest } => {
                eprintln!("Manifest fetched for {}: {}", elf_url, manifest.name);
                // Find the elf and update its manifest
                for elf in &mut app_state.trusted_elves {
                    if elf.url == elf_url {
                        elf.manifest = Some(manifest.clone());
                        elf.is_reachable = true;
                        elf.last_fetched = Some(std::time::Instant::now());
                        app_state.status = format!("Loaded elf: {}", manifest.name);
                        break;
                    }
                }
            }
            elf_http::ElfHttpEvent::ManifestFetchFailed { elf_url, error } => {
                eprintln!("Manifest fetch failed for {}: {}", elf_url, error);
                // Mark the elf as unreachable
                for elf in &mut app_state.trusted_elves {
                    if elf.url == elf_url {
                        elf.is_reachable = false;
                        break;
                    }
                }
                app_state.status = format!("Failed to load elf: {}", error);
            }
            elf_http::ElfHttpEvent::SummonAccepted { elf_url } => {
                app_state.status = format!("Elf summoned: {}", elf_url);
            }
            elf_http::ElfHttpEvent::SummonFailed { elf_url: _, error } => {
                app_state.status = format!("Elf summon failed: {}", error);
            }
        }
    }
}

fn handle_navigation(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    keys: Res<ButtonInput<bevy::prelude::KeyCode>>,
    mut contexts: EguiContexts,
    mut frames_to_skip: Local<u8>,
) {
    // bevy_egui 0.39 requires a few frames for initialization (see ui_system for details)
    if *frames_to_skip < 2 {
        *frames_to_skip += 1;
        return;
    }

    // Don't handle navigation when URL bar has focus
    if app_state.url_bar_has_focus {
        return;
    }

    // Don't handle navigation when in text input or inline edit mode
    if matches!(app_state.input_mode, InputMode::TextInput { .. } | InputMode::InlineEdit { .. }) {
        return;
    }

    // Don't handle navigation when command bar is open
    if app_state.show_command_bar {
        return;
    }

    // Don't handle navigation when panels with text inputs are shown
    // (elf panel, identity panel, identity setup)
    if app_state.show_elf_panel || app_state.show_identity_panel || app_state.pending_identity_setup.is_some() {
        if let Ok(ctx) = contexts.ctx_mut() {
            if ctx.wants_keyboard_input() {
                return;
            }
        }
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

    // Get gamepad snapshot for navigation
    let gp_snapshot = gamepad::get_gamepad_snapshot();

    // Check gamepad navigation (pressed this frame = just pressed for gamepad)
    let gamepad_edge: Option<usize> = if let Some(ref gp) = gp_snapshot {
        if resolver.command_pressed_gamepad(&Command::GraphNavigateNorth, gp) {
            Some(EDGE_NORTH)
        } else if resolver.command_pressed_gamepad(&Command::GraphNavigateSouth, gp) {
            Some(EDGE_SOUTH)
        } else if resolver.command_pressed_gamepad(&Command::GraphNavigateWest, gp) {
            Some(EDGE_WEST)
        } else if resolver.command_pressed_gamepad(&Command::GraphNavigateEast, gp) {
            Some(EDGE_EAST)
        } else if resolver.command_pressed_gamepad(&Command::GraphNavigateUp, gp) {
            Some(EDGE_UP)
        } else if resolver.command_pressed_gamepad(&Command::GraphNavigateDown, gp) {
            Some(EDGE_DOWN)
        } else {
            None
        }
    } else {
        None
    };

    // Check gamepad held (for key repeat)
    let gamepad_held: Option<usize> = if let Some(ref gp) = gp_snapshot {
        if resolver.command_held_gamepad(&Command::GraphNavigateNorth, gp) {
            Some(EDGE_NORTH)
        } else if resolver.command_held_gamepad(&Command::GraphNavigateSouth, gp) {
            Some(EDGE_SOUTH)
        } else if resolver.command_held_gamepad(&Command::GraphNavigateWest, gp) {
            Some(EDGE_WEST)
        } else if resolver.command_held_gamepad(&Command::GraphNavigateEast, gp) {
            Some(EDGE_EAST)
        } else if resolver.command_held_gamepad(&Command::GraphNavigateUp, gp) {
            Some(EDGE_UP)
        } else if resolver.command_held_gamepad(&Command::GraphNavigateDown, gp) {
            Some(EDGE_DOWN)
        } else {
            None
        }
    } else {
        None
    };

    // Check which navigation key is held (if any) - keyboard
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
        gamepad_held // Fall back to gamepad held
    };

    // Check for just pressed (initial press) - keyboard
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
        gamepad_edge // Fall back to gamepad pressed
    };

    // Check for history back command (keyboard)
    if resolver.command_just_pressed_bevy(context, &Command::GraphHistoryBack, &keys) {
        if let Some(prev_id) = app_state.history.pop() {
            app_state.current_vertex = Some(prev_id);
        }
        return;
    }

    // Check for history back command (gamepad)
    if let Some(ref gp) = gp_snapshot {
        if resolver.command_pressed_gamepad(&Command::GraphHistoryBack, gp) {
            if let Some(prev_id) = app_state.history.pop() {
                app_state.current_vertex = Some(prev_id);
            }
            return;
        }
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
            if target_id != 0 && graph.vertices.contains_key(&target_id) {
                // Move cursor to target normally
                // (Only if target vertex actually exists in the graph)
                app_state.history.push(current_id);
                app_state.current_vertex = Some(target_id);
            } else if target_id != 0 {
                // Edge points to a vertex that doesn't exist in the graph
                // This means it's unloaded content - show status
                app_state.status = "Loading...".to_string();
            } else {
                // target_id == 0: no edge in this direction
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
fn auto_play_audio_on_navigate(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    playback_state: Res<AudioPlaybackState>,
    preload_cache: Res<AudioPreloadCache>,
) {
    let current_id = app_state.current_vertex;
    let last_id = app_state.last_vertex;

    // Update last_vertex tracking
    if current_id != last_id {
        // Stop any playing audio when leaving a cell
        stop_audio(&playback_state);

        // Clear image modal when navigating away from the image's vertex
        if app_state.show_image_modal {
            if app_state.image_modal_vertex_id != current_id {
                app_state.show_image_modal = false;
                app_state.image_modal_vertex_id = None;
                app_state.sidebar.fullscreen = false;
            }
        }

        app_state.last_vertex = current_id;

        // If we navigated to a new vertex, check if it's audio
        if let Some(vertex_id) = current_id {
            // Skip auto-play for vertices we just recorded
            if app_state.skip_autoplay_vertex == Some(vertex_id) {
                app_state.skip_autoplay_vertex = None;
                return;
            }

            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                // Check all layers for audio to auto-play
                for layer in vertex.layers.values() {
                    if layer.mime.starts_with("audio/") && !layer.data.is_empty() {
                        // Auto-play the audio using fast path with preload cache
                        play_audio_fast(vertex_id, &layer.data, &layer.mime, &preload_cache, &playback_state);
                        return;
                    }
                }

                // TTS mode: find text in any layer and read aloud
                if app_state.tts_mode {
                    for layer in vertex.layers.values() {
                        if layer.mime.starts_with("text/")
                            && layer.mime != "text/gradesta-url"
                            && !layer.data.is_empty()
                        {
                            if let Ok(text) = String::from_utf8(layer.data.clone()) {
                                let text = text.trim();
                                if !text.is_empty() {
                                    tts::speak(text);
                                    return;
                                }
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
    mut graph: ResMut<GraphState>,
    ws_cmd_tx: Res<WsCommandTx>,
) {
    let Some(current_id) = app_state.current_vertex else { return };
    let Some(cmd_tx) = &ws_cmd_tx.0 else { return };

    // FIRST: Check if current vertex doesn't exist locally - request it via WatchLandmark
    if !graph.vertices.contains_key(&current_id) {
        // We navigated to a vertex that doesn't exist in our local graph
        // This happens at landmark boundaries - request the data
        let landmark_url = build_landmark_url(current_id, &app_state.base_ws_url);

        let action_id = app_state.next_action_id;
        if graph.landmark_mgr.watch_if_needed(&landmark_url, action_id, cmd_tx) {
            eprintln!("Requesting landmark for unknown vertex {}: {}", current_id, landmark_url);
            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
        }
        return;
    }

    // Extract data we need from the current vertex to avoid borrow conflicts
    let (portal_url, current_edges) = {
        let Some(current) = graph.vertices.get(&current_id) else { return };
        // Find any layer with text/gradesta-url mime type
        let portal_url = current.layers.values()
            .find(|l| l.mime == "text/gradesta-url")
            .map(|l| String::from_utf8_lossy(&l.data).to_string());
        (portal_url, current.edges)
    };

    // FIRST: If we're sitting on a portal, handle it
    // A portal is any vertex containing text/gradesta-url in any layer
    // If the vertex is ONLY a portal (no other content), auto-follow
    // If it has other content too, just preload the landmark

    if let Some(ref landmark_url) = portal_url {
        // Check if this is a "pure" portal (only has portal URL, no other meaningful content)
        let is_pure_portal = {
            let Some(current) = graph.vertices.get(&current_id) else { return };
            current.layers.values().all(|l| l.mime == "text/gradesta-url")
        };

        if is_pure_portal {
            // Pure portal - auto-follow
            // Check if this landmark was already loaded by looking up vertices associated with it
            if let Some(vertices) = graph.landmark_mgr.get_landmark_vertices(landmark_url) {
                // First, try to find the east neighbor of the portal (preferred direction for content)
                let east_id = current_edges[EDGE_EAST];
                if east_id != 0 {
                    if let Some(vertex) = graph.vertices.get(&east_id) {
                        // Check if neighbor has any non-portal content
                        let has_non_portal = vertex.layers.values().any(|l| l.mime != "text/gradesta-url");
                        if has_non_portal {
                            // Found content vertex to the east - jump to it
                            app_state.history.push(current_id);
                            app_state.current_vertex = Some(east_id);
                            graph.landmark_mgr.clear_follow(landmark_url);
                            app_state.loading_portal_vertex = None;
                            app_state.loading_portal_cell = None;
                            return;
                        }
                    }
                }

                // Fallback: find the first non-portal vertex in this landmark
                let vertices_copy = vertices.clone(); // Clone to avoid borrow conflict
                for &vid in &vertices_copy {
                    if let Some(vertex) = graph.vertices.get(&vid) {
                        let has_non_portal = vertex.layers.values().any(|l| l.mime != "text/gradesta-url");
                        if has_non_portal {
                            // Found a content vertex - jump to it
                            app_state.history.push(current_id);
                            app_state.current_vertex = Some(vid);
                            graph.landmark_mgr.clear_follow(landmark_url);
                            app_state.loading_portal_vertex = None;
                            app_state.loading_portal_cell = None;
                            return;
                        }
                    }
                }
            }

            // Not loaded yet - request it with auto-follow
            let action_id = app_state.next_action_id;
            if graph.landmark_mgr.watch_and_follow(landmark_url, action_id, cmd_tx) {
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            }

            // Set up loading indicator
            if graph.landmark_mgr.should_follow(landmark_url) {
                app_state.loading_portal_vertex = Some(current_id);
            }
            return;
        } else {
            // Has other content - just preload the landmark, don't auto-follow
            let action_id = app_state.next_action_id;
            if graph.landmark_mgr.watch_if_needed(landmark_url, action_id, cmd_tx) {
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
            }
            // Don't return - continue to preload neighbors
        }
    }

    // SECOND: Preload immediate neighbors (1 step away) that we don't have
    // Prioritize direction of navigation
    let priority = direction_priority_order(app_state.last_nav_direction);
    for &idx in &priority {
        let edge = current_edges[idx];
        if edge == 0 {
            continue;
        }
        if !graph.vertices.contains_key(&edge) {
            // This edge points to a vertex we don't have - request it
            let landmark_url = build_landmark_url(edge, &app_state.base_ws_url);
            let action_id = app_state.next_action_id;
            if graph.landmark_mgr.watch_if_needed(&landmark_url, action_id, cmd_tx) {
                eprintln!("Preloading nearby unknown vertex {}: {}", edge, landmark_url);
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                return; // Only one per frame
            }
        }
    }

    // THIRD: Collect vertices that are exactly 2 steps away
    // Prioritize direction of navigation
    let mut two_steps_away: Vec<u64> = Vec::new();

    // For each immediate neighbor (1 step)
    for &idx in &priority {
        let edge1 = current_edges[idx];
        if edge1 == 0 {
            continue;
        }
        if let Some(neighbor) = graph.vertices.get(&edge1) {
            // For each of that neighbor's neighbors (2 steps)
            for &idx2 in &priority {
                let edge2 = neighbor.edges[idx2];
                if edge2 != 0 && edge2 != current_id && !two_steps_away.contains(&edge2) {
                    two_steps_away.push(edge2);
                }
            }
        }
    }

    // Request unknown vertices that are 2 steps away
    for vid in &two_steps_away {
        if !graph.vertices.contains_key(vid) {
            let landmark_url = build_landmark_url(*vid, &app_state.base_ws_url);
            let action_id = app_state.next_action_id;
            if graph.landmark_mgr.watch_if_needed(&landmark_url, action_id, cmd_tx) {
                eprintln!("Preloading 2-step unknown vertex {}: {}", vid, landmark_url);
                app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                return; // Only one per frame
            }
        }
    }

    // Also check for portal vertices 2 steps away
    for vid in two_steps_away {
        if let Some(vertex) = graph.vertices.get(&vid) {
            // Check all layers for gradesta-url
            let landmark_url = vertex.layers.values()
                .find(|l| l.mime == "text/gradesta-url")
                .map(|l| String::from_utf8_lossy(&l.data).to_string());

            if let Some(landmark_url) = landmark_url {
                let action_id = app_state.next_action_id;
                if graph.landmark_mgr.watch_if_needed(&landmark_url, action_id, cmd_tx) {
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    return; // Only one per frame
                }
            }
        }
    }
}

/// Preload audio for neighboring cells to enable instant playback
fn preload_nearby_audio(
    app_state: Res<AppState>,
    graph: Res<GraphState>,
    mut preload_cache: ResMut<AudioPreloadCache>,
) {
    let Some(current_id) = app_state.current_vertex else { return };
    let Some(current) = graph.vertices.get(&current_id) else { return };

    // Process any completed pre-decodes
    while let Ok((vertex_id, decoded)) = preload_cache.decoded_rx.try_recv() {
        preload_cache.cache.insert(vertex_id, decoded);
        preload_cache.pending.remove(&vertex_id);
    }

    // LRU eviction: keep max 12 entries
    if preload_cache.cache.len() > 12 {
        let to_remove: Vec<_> = preload_cache.cache.keys()
            .filter(|&&id| id != current_id)
            .take(preload_cache.cache.len() - 8)
            .copied()
            .collect();
        for id in to_remove {
            preload_cache.cache.remove(&id);
        }
    }

    // Preload 1-step neighbors - prioritize direction of navigation
    let priority = direction_priority_order(app_state.last_nav_direction);
    for &idx in &priority {
        let edge = current.edges[idx];
        if edge == 0 { continue; }
        if preload_audio_if_needed(edge, &graph, &mut preload_cache) {
            return; // One per frame
        }
    }

    // Preload 2-step neighbors - prioritize same direction
    for &idx in &priority {
        let edge1 = current.edges[idx];
        if edge1 == 0 { continue; }
        if let Some(neighbor) = graph.vertices.get(&edge1) {
            for &idx2 in &priority {
                let edge2 = neighbor.edges[idx2];
                if edge2 != 0 && edge2 != current_id {
                    if preload_audio_if_needed(edge2, &graph, &mut preload_cache) {
                        return; // One per frame
                    }
                }
            }
        }
    }
}

/// Check if a vertex has audio and start pre-decoding if needed
fn preload_audio_if_needed(
    vertex_id: u64,
    graph: &GraphState,
    cache: &mut AudioPreloadCache,
) -> bool {
    // Skip if already cached or pending
    if cache.cache.contains_key(&vertex_id) || cache.pending.contains(&vertex_id) {
        return false;
    }

    if let Some(vertex) = graph.vertices.get(&vertex_id) {
        // Check all layers for audio content
        for layer in vertex.layers.values() {
            if layer.mime.starts_with("audio/") && !layer.data.is_empty() {
                // Skip large files (>5MB) to avoid memory bloat
                if layer.data.len() > 5_000_000 {
                    return false;
                }

                cache.pending.insert(vertex_id);
                predecode_audio_async(
                    vertex_id,
                    layer.data.clone(),
                    cache.decoded_tx.clone(),
                );
                return true;
            }
        }
    }
    false
}

/// Reset playback speed boost when it expires (10 seconds after last boost)
fn playback_boost_decay(
    mut boost_state: ResMut<PlaybackBoostState>,
) {
    // Only check if there's an active boost
    if boost_state.last_boost_time.is_some() && boost_state.is_expired() {
        boost_state.reset();
        // Reset both TTS and audio speed to normal
        tts::set_rate(1.0);
        set_audio_speed(1.0);
    }
}
