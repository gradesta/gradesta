use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gif::DecodeOptions;
use std::collections::{HashMap, HashSet};
use std::io::{self, Cursor, Write};
use std::net::TcpStream;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client, Message};
use url::Url;

mod identity;
mod video_player;
mod whisper;
use identity::{Identity, IdentityConfig};
use video_player::VideoPlayer;

const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
const MSG_CLIENT_CLICK_VERTEX: u8 = 0x84;
const MSG_CLIENT_IDENTIFICATION_RESPONSE: u8 = 0x90;
const MSG_CLIENT_IDENTIFICATION_REFUSED: u8 = 0x91;
const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
const MSG_SERVER_SET_EDGES: u8 = 0x03;
const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
const MSG_SERVER_LOG: u8 = 0x0F;
const MSG_SERVER_REQUEST_IDENTIFICATION: u8 = 0x10;

const EDGE_WEST: usize = 0;
const EDGE_EAST: usize = 1;
const EDGE_NORTH: usize = 2;
const EDGE_SOUTH: usize = 3;
const EDGE_UP: usize = 4;
const EDGE_DOWN: usize = 5;

/// Information about a stack of vertices connected via up/down edges
struct StackInfo {
    /// All vertices in the stack (top to bottom)
    vertices: Vec<u64>,
    /// Current position in the stack (0-indexed)
    current_index: usize,
    /// Total number of vertices in the stack
    total: usize,
}

/// Compute the stack of vertices connected via up/down edges
fn compute_stack(graph: &GraphState, vertex_id: u64) -> StackInfo {
    let mut stack = Vec::new();
    let mut visited = HashSet::new();

    // Find the top of the stack
    let mut top_id = vertex_id;
    while let Some(v) = graph.vertices.get(&top_id) {
        if v.edges[EDGE_UP] != 0 && !visited.contains(&v.edges[EDGE_UP]) {
            visited.insert(top_id);
            top_id = v.edges[EDGE_UP];
        } else {
            break;
        }
    }

    // Now traverse down from top, collecting all vertices
    visited.clear();
    let mut current = top_id;
    let mut current_index = 0;
    let mut found_index = false;

    while let Some(v) = graph.vertices.get(&current) {
        if visited.contains(&current) {
            break;
        }
        visited.insert(current);
        stack.push(current);

        if current == vertex_id {
            current_index = stack.len() - 1;
            found_index = true;
        }

        if v.edges[EDGE_DOWN] != 0 {
            current = v.edges[EDGE_DOWN];
        } else {
            break;
        }
    }

    if !found_index && !stack.is_empty() {
        current_index = 0;
    }

    StackInfo {
        total: stack.len(),
        vertices: stack,
        current_index,
    }
}

// Client -> Server messages for editing
const MSG_CLIENT_SET_VERTEX_LABEL: u8 = 0x85;
const MSG_CLIENT_CREATE_VERTEX: u8 = 0x86;
const MSG_CLIENT_DELETE_VERTEX: u8 = 0x87;

/// Input mode for the browser
#[derive(Clone, Debug, PartialEq, Eq, Default)]
enum InputMode {
    #[default]
    Normal,
    TextInput { direction: Option<usize> }, // direction to create new vertex, None = edit current
    Recording { direction: usize },          // recording audio to create new vertex in direction
}


/// Layer content for a vertex
#[derive(Clone, Debug, Default)]
struct LayerContent {
    mime: String,
    data: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
struct Vertex {
    id: u64,
    /// Primary layer data (layer 0) for backwards compatibility
    label: Vec<u8>,
    mime: Option<String>,
    edges: [u64; 6],
    /// Additional layers (layer_id -> content)
    layers: HashMap<u32, LayerContent>,
    /// Edit mask: bit 0-5 for edge editability, bit 6 for label editability
    /// 0x7F = all editable, 0 = read-only
    edit_mask: u8,
}

#[derive(Resource, Default)]
struct GraphState {
    vertices: HashMap<u64, Vertex>,
    context_uri: Option<String>,
    pending_jump_context: Option<String>, // If set, jump to first non-portal vertex of this context
    landmark_vertices: HashMap<String, Vec<u64>>, // Maps landmark URI -> vertices that belong to it
    current_receiving_landmark: Option<String>, // Which landmark we're currently receiving data for
}

#[derive(Resource)]
struct NetRx(Receiver<ServerEvent>);

#[derive(Resource)]
struct NetEventsTx(Sender<ServerEvent>);

/// Commands sent TO the websocket thread
#[derive(Clone, Debug)]
enum WsCommand {
    WatchLandmark { action_id: u64, landmark: String },
    ClickVertex { action_id: u64, vertex_id: u64 },
    IdentificationResponse {
        action_id: u64,
        identity_url: String,
        signature: Vec<u8>,
    },
    IdentificationRefused {
        action_id: u64,
    },
    SetVertexLabel {
        action_id: u64,
        vertex_id: u64,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    CreateVertex {
        action_id: u64,
        from_vertex: u64,
        direction: u8,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    DeleteVertex {
        action_id: u64,
        vertex_id: u64,
    },
}

#[derive(Resource)]
struct WsCommandTx(Option<Sender<WsCommand>>);

#[derive(Resource)]
struct AppState {
    url_input: String,
    status: String,
    connected: bool,
    current_vertex: Option<u64>,
    last_vertex: Option<u64>, // For detecting navigation changes (used for auto-play)
    history: Vec<u64>,
    base_ws_url: Option<String>,
    requested_landmarks: HashSet<String>, // Track which landmarks we've already requested
    following_portal: Option<String>, // If set, we're waiting to jump to this landmark's first vertex
    // Key repeat state
    key_repeat_last_move: Option<Instant>,
    key_repeat_started: bool,
    // Content modal state
    show_text_modal: bool,
    text_modal_content: String,
    show_image_modal: bool,
    image_modal_vertex_id: Option<u64>,
    // Zoom state
    zoom_level: f32,
    // Identity state
    show_identity_panel: bool,
    identity_config: IdentityConfig,
    // Identity consent dialog
    pending_identification: Option<PendingIdentification>,
    selected_identity_index: usize,
    // Nextcloud login flow state
    nextcloud_login_state: Option<NextcloudLoginState>,
    pending_identity_setup: Option<PendingIdentitySetup>,
    nextcloud_url_input: String,
    // Bag (clipboard stack) for connecting non-adjacent cells
    bag: Vec<u64>,
    show_bag_panel: bool,
    // Input mode state
    input_mode: InputMode,
    text_input_buffer: String,
    // Audio recording state
    audio_samples: Arc<Mutex<Vec<f32>>>,
    audio_sample_rate: u32,
    recording_start: Option<Instant>,
    // Action ID counter (counts down from MAX to avoid collision with server IDs)
    next_action_id: u64,
    // Pending vertex creations: maps action_id -> data for populating vertex locally on ack
    pending_creations: HashMap<u64, PendingVertexCreation>,
    // Skip auto-play for this vertex (set after recording to avoid immediate playback)
    skip_autoplay_vertex: Option<u64>,
    // Last navigation direction (used to determine where new vertices are created)
    last_nav_direction: usize,
    // Focus URL bar on next frame (to avoid 'l' being typed when pressing Ctrl+L)
    focus_url_bar_next_frame: bool,
    // Video player state
    show_video_modal: bool,
    video_modal_vertex_id: Option<u64>,
}

/// Pending vertex creation data - waiting for server acknowledgment
struct PendingVertexCreation {
    /// Audio samples for transcription (empty for text)
    samples: Vec<f32>,
    sample_rate: u32,
    /// The data that was sent to the server (to populate vertex locally)
    data: Vec<u8>,
    /// MIME type of the data
    mime: String,
}

const KEY_REPEAT_DELAY: Duration = Duration::from_millis(400); // Initial delay before repeat starts
const KEY_REPEAT_RATE: Duration = Duration::from_millis(50);   // Rate of repeat once started

/// Pending identification request from a server
#[derive(Clone, Debug)]
struct PendingIdentification {
    action_id: u64,
    nonce: [u8; 32],
    timestamp: u64,
    reason: String,
    server_url: String,
}

/// State for ongoing Nextcloud login flow
#[derive(Clone, Debug)]
struct NextcloudLoginState {
    nextcloud_url: String,
    poll_endpoint: String,
    poll_token: String,
    started: Instant,
}

/// State after successful login, waiting for user to choose display name
#[derive(Clone, Debug)]
struct PendingIdentitySetup {
    nextcloud_url: String,
    username: String,
    app_password: String,
    display_name_input: String,
}

/// Action to take for identification request
enum IdentificationAction {
    Identify { remember: bool },
    Refuse,
}

impl Default for AppState {
    fn default() -> Self {
        // Try to load identity config
        let identity_config = IdentityConfig::load().unwrap_or_default();

        Self {
            url_input: "ws://localhost:8080/ws?landmark=/home/".to_string(),
            status: "Enter URL and click Connect".to_string(),
            connected: false,
            current_vertex: None,
            last_vertex: None,
            history: Vec::new(),
            base_ws_url: None,
            requested_landmarks: HashSet::new(),
            following_portal: None,
            key_repeat_last_move: None,
            key_repeat_started: false,
            show_text_modal: false,
            text_modal_content: String::new(),
            show_image_modal: false,
            image_modal_vertex_id: None,
            zoom_level: 1.0,
            show_identity_panel: false,
            identity_config,
            pending_identification: None,
            selected_identity_index: 0,
            nextcloud_login_state: None,
            pending_identity_setup: None,
            nextcloud_url_input: "https://".to_string(),
            bag: Vec::new(),
            show_bag_panel: false,
            input_mode: InputMode::Normal,
            text_input_buffer: String::new(),
            audio_samples: Arc::new(Mutex::new(Vec::new())),
            audio_sample_rate: 44100,
            recording_start: None,
            next_action_id: u64::MAX,
            pending_creations: HashMap::new(),
            skip_autoplay_vertex: None,
            last_nav_direction: EDGE_SOUTH, // Default to south
            focus_url_bar_next_frame: false,
            show_video_modal: false,
            video_modal_vertex_id: None,
        }
    }
}

const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 4.0;
const ZOOM_STEP: f32 = 0.1;

/// Decoded image ready to be uploaded to GPU
struct DecodedImage {
    id: u64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Resource)]
struct MediaCache {
    textures: HashMap<u64, egui::TextureHandle>,
    animated_gifs: HashMap<u64, AnimatedGif>,
    waveforms: HashMap<u64, Vec<f32>>, // Pre-computed waveform samples (0.0-1.0)
    /// Receiver for images decoded in background threads
    decoded_rx: Receiver<DecodedImage>,
    /// Sender for decoded images (cloned to background threads)
    decoded_tx: Sender<DecodedImage>,
    /// Set of vertex IDs currently being decoded (to avoid duplicate work)
    pending_decodes: HashSet<u64>,
    /// Active video players (vertex_id -> player)
    video_players: HashMap<u64, VideoPlayer>,
    /// Current video frame textures
    video_textures: HashMap<u64, egui::TextureHandle>,
}

impl Default for MediaCache {
    fn default() -> Self {
        let (decoded_tx, decoded_rx) = crossbeam_channel::unbounded();
        Self {
            textures: HashMap::new(),
            animated_gifs: HashMap::new(),
            waveforms: HashMap::new(),
            decoded_rx,
            decoded_tx,
            pending_decodes: HashSet::new(),
            video_players: HashMap::new(),
            video_textures: HashMap::new(),
        }
    }
}

/// Signal to stop audio recording and communicate sample rate
#[derive(Resource, Default)]
struct AudioRecordingSignal {
    should_stop: Arc<Mutex<bool>>,
    actual_sample_rate: Arc<Mutex<u32>>,
}

/// Signal to control audio playback
#[derive(Resource, Clone, Default)]
struct AudioPlaybackState {
    should_stop: Arc<Mutex<bool>>,
    playing_vertex: Arc<Mutex<Option<u64>>>,
}

#[derive(Clone)]
struct AnimatedGif {
    frames: Vec<egui::TextureHandle>,
    delays: Vec<Duration>,
    current_frame: usize,
    last_switch: Instant,
}

#[derive(Clone, Debug)]
enum ServerEvent {
    SetContext { uri: String },
    SetVertexLabel { vertex_id: u64, layer: u32, mime: String, data: Vec<u8> },
    SetEdges { vertex_id: u64, edges: [u64; 6], edit_mask: u8 },
    Log { action_id: u64, status: u32, vertex_id: u64, message: String },
    Connected { base_url: String },
    Disconnected { reason: String },
    Error { message: String },
    RequestIdentification {
        action_id: u64,
        nonce: [u8; 32],
        timestamp: u64,
        reason: String,
    },
    /// Local event: store a label locally and send to server
    LocalSetVertexLabel {
        action_id: u64,
        vertex_id: u64,
        layer: u32,
        mime: String,
        data: Vec<u8>,
    },
    /// HTTP stream content fetched (layer 3 -> layer 2)
    HttpStreamContentFetched {
        vertex_id: u64,
        mime: String,
        data: Vec<u8>,
    },
}

#[derive(Default, Clone)]
struct GridView {
    cells: HashMap<(i32, i32), u64>,
    positions: HashMap<u64, (i32, i32)>,
    /// Distance from current vertex for each cell (used to resolve overlapping branches)
    distances: HashMap<(i32, i32), u32>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
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
            ingest_server_events,
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

fn build_grid_view(graph: &GraphState, current_id: u64) -> GridView {
    let mut grid = GridView::default();
    let mut visited = HashSet::new();

    // Always ensure the current vertex is in the grid at (0, 0) with distance 0
    // This guarantees centering works correctly
    grid.cells.insert((0, 0), current_id);
    grid.positions.insert(current_id, (0, 0));
    grid.distances.insert((0, 0), 0);
    visited.insert(current_id);

    // If the current vertex exists in the graph, expand from it
    if let Some(current_vertex) = graph.vertices.get(&current_id) {
        // Expand north from current vertex
        let mut y = -1i32;
        let mut north_id = current_vertex.edges[EDGE_NORTH];
        while north_id != 0 && graph.vertices.contains_key(&north_id) && !visited.contains(&north_id) {
            let distance = (-y) as u32;
            grid.cells.insert((0, y), north_id);
            grid.positions.insert(north_id, (0, y));
            grid.distances.insert((0, y), distance);
            visited.insert(north_id);
            grid.min_y = grid.min_y.min(y);

            if let Some(v) = graph.vertices.get(&north_id) {
                north_id = v.edges[EDGE_NORTH];
            } else {
                break;
            }
            y -= 1;
        }

        // Expand south from current vertex
        let mut y = 1i32;
        let mut south_id = current_vertex.edges[EDGE_SOUTH];
        while south_id != 0 && graph.vertices.contains_key(&south_id) && !visited.contains(&south_id) {
            let distance = y as u32;
            grid.cells.insert((0, y), south_id);
            grid.positions.insert(south_id, (0, y));
            grid.distances.insert((0, y), distance);
            visited.insert(south_id);
            grid.max_y = grid.max_y.max(y);

            if let Some(v) = graph.vertices.get(&south_id) {
                south_id = v.edges[EDGE_SOUTH];
            } else {
                break;
            }
            y += 1;
        }
    }

    // Now expand west/east from all vertices in the center column
    // Sort by distance so closer vertices expand first and claim shared targets
    let mut center_vertices: Vec<(i32, u64, u32)> = grid.cells.iter()
        .filter(|((x, _), _)| *x == 0)
        .map(|((_, y), id)| (*y, *id, *grid.distances.get(&(0, *y)).unwrap_or(&u32::MAX)))
        .collect();
    center_vertices.sort_by_key(|(_, _, dist)| *dist);

    for (y, id, base_dist) in center_vertices {
        if let Some(v) = graph.vertices.get(&id) {
            if v.edges[EDGE_WEST] != 0 && !visited.contains(&v.edges[EDGE_WEST]) {
                expand_column_recursive_with_distance(&mut grid, graph, v.edges[EDGE_WEST], -1, y, base_dist + 1, &mut visited);
            }
            if v.edges[EDGE_EAST] != 0 && !visited.contains(&v.edges[EDGE_EAST]) {
                expand_column_recursive_with_distance(&mut grid, graph, v.edges[EDGE_EAST], 1, y, base_dist + 1, &mut visited);
            }
        }
    }

    grid
}

fn expand_column_recursive_with_distance(
    grid: &mut GridView,
    graph: &GraphState,
    start_id: u64,
    x: i32,
    start_y: i32,
    base_distance: u32,
    visited: &mut HashSet<u64>,
) {
    if !graph.vertices.contains_key(&start_id) {
        return;
    }

    // If this vertex is already placed somewhere in the grid, check if we're offering
    // a closer position. If so, move it. If not, skip.
    if let Some(&existing_pos) = grid.positions.get(&start_id) {
        let existing_dist = *grid.distances.get(&existing_pos).unwrap_or(&u32::MAX);
        if base_distance < existing_dist {
            // This is a closer path to this vertex - move it to the new position
            grid.cells.remove(&existing_pos);
            grid.distances.remove(&existing_pos);
            // Don't remove from positions yet - will be updated below
        } else {
            // Already placed at a closer or equal distance, don't expand from here
            return;
        }
    }

    // Simple approach: only place the single vertex at start_y, don't expand north/south.
    // This prevents unrelated graphs from expanding into each other while still allowing
    // navigation. The grid shows cells that are directly horizontally connected to the
    // center column.

    let pos = (x, start_y);

    // Check if the target cell already has a vertex at a closer distance
    let should_insert = match grid.distances.get(&pos) {
        None => true,
        Some(&existing_dist) => base_distance < existing_dist,
    };

    if !should_insert {
        // Cell already has a closer vertex, don't expand from here
        return;
    }

    // Remove old vertex from this cell if any
    if let Some(&old_vertex) = grid.cells.get(&pos) {
        if old_vertex != start_id {
            grid.positions.remove(&old_vertex);
        }
    }

    visited.insert(start_id);
    grid.cells.insert(pos, start_id);
    grid.positions.insert(start_id, pos);
    grid.distances.insert(pos, base_distance);

    grid.min_x = grid.min_x.min(x);
    grid.max_x = grid.max_x.max(x);
    grid.min_y = grid.min_y.min(start_y);
    grid.max_y = grid.max_y.max(start_y);

    // Continue expanding horizontally (but not vertically)
    if let Some(v) = graph.vertices.get(&start_id) {
        if x < 0 && v.edges[EDGE_WEST] != 0 {
            expand_column_recursive_with_distance(grid, graph, v.edges[EDGE_WEST], x - 1, start_y, base_distance + 1, visited);
        }
        if x > 0 && v.edges[EDGE_EAST] != 0 {
            expand_column_recursive_with_distance(grid, graph, v.edges[EDGE_EAST], x + 1, start_y, base_distance + 1, visited);
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
    let url_bar_id = egui::Id::new("url_bar");

    // Track Ctrl+L state - focus URL bar when Ctrl+L is released
    let ctrl_held = ctx.input(|i| i.modifiers.ctrl);
    let l_held = ctx.input(|i| i.key_down(egui::Key::L));

    if ctrl_held && l_held {
        // Ctrl+L is being held - mark that we want to focus
        app_state.focus_url_bar_next_frame = true;
    } else if app_state.focus_url_bar_next_frame && !l_held {
        // L was released - now focus the URL bar
        app_state.focus_url_bar_next_frame = false;
        ctx.memory_mut(|mem| mem.request_focus(url_bar_id));
    }

    // Check for Ctrl+C to copy URL to clipboard
    let copy_url = ctx.input(|i| i.key_pressed(egui::Key::C) && i.modifiers.ctrl);
    if copy_url {
        ctx.copy_text(app_state.url_input.clone());
        app_state.status = "Copied URL to clipboard".to_string();
    }

    ctx.input(|i| {
        // Escape to close modals
        if i.key_pressed(egui::Key::Escape) {
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
        // Ctrl+Enter to open modal with current content (but not when in text input mode - that's for submitting)
        if i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl && !matches!(app_state.input_mode, InputMode::TextInput { .. }) {
            if let Some(current_id) = app_state.current_vertex {
                if let Some(vertex) = graph.vertices.get(&current_id) {
                    let mime = vertex.mime.as_deref().unwrap_or("");
                    let primary_is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
                    let primary_is_text = mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url";

                    // Also check additional layers for images
                    let has_layer_image = vertex.layers.values()
                        .any(|l| l.mime.starts_with("image/") || is_image_data(&l.data));

                    if primary_is_text && !has_layer_image {
                        app_state.text_modal_content = String::from_utf8_lossy(&vertex.label).to_string();
                        app_state.show_text_modal = true;
                    } else if primary_is_image || has_layer_image {
                        app_state.image_modal_vertex_id = Some(current_id);
                        app_state.show_image_modal = true;
                    }
                }
            }
        }
        // Plain Enter to "click" the current vertex (send click message to server)
        if i.key_pressed(egui::Key::Enter) && !i.modifiers.ctrl && !i.modifiers.shift && !i.modifiers.alt && matches!(app_state.input_mode, InputMode::Normal) {
            if let Some(current_id) = app_state.current_vertex {
                if let Some(ref tx) = ws_cmd_tx.0 {
                    let action_id = app_state.next_action_id;
                    app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                    let _ = tx.send(WsCommand::ClickVertex { action_id, vertex_id: current_id });
                    app_state.status = format!("Clicked vertex {}", current_id);
                }
            }
        }
        // Ctrl++ / Ctrl+= to zoom in
        if i.modifiers.ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            app_state.zoom_level = (app_state.zoom_level + ZOOM_STEP).min(ZOOM_MAX);
        }
        // Ctrl+- to zoom out
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Minus) {
            app_state.zoom_level = (app_state.zoom_level - ZOOM_STEP).max(ZOOM_MIN);
        }
        // Ctrl+0 to reset zoom
        if i.modifiers.ctrl && i.key_pressed(egui::Key::Num0) {
            app_state.zoom_level = 1.0;
        }
        // Mouse wheel zoom (with Ctrl)
        if i.modifiers.ctrl && i.raw_scroll_delta.y != 0.0 {
            let delta = i.raw_scroll_delta.y * 0.001;
            app_state.zoom_level = (app_state.zoom_level + delta).clamp(ZOOM_MIN, ZOOM_MAX);
        }
        // Pinch zoom (touch/trackpad)
        if i.zoom_delta() != 1.0 {
            app_state.zoom_level = (app_state.zoom_level * i.zoom_delta()).clamp(ZOOM_MIN, ZOOM_MAX);
        }
        // Bag (clipboard) operations
        // Ctrl+B: Toggle bag panel
        if i.key_pressed(egui::Key::B) && i.modifiers.ctrl {
            app_state.show_bag_panel = !app_state.show_bag_panel;
        }
        // Y: Yank (copy) current vertex to bag
        if i.key_pressed(egui::Key::Y) && !i.modifiers.ctrl {
            if let Some(current_id) = app_state.current_vertex {
                // Don't add duplicates at the top
                if app_state.bag.last() != Some(&current_id) {
                    app_state.bag.push(current_id);
                    app_state.status = format!("Yanked vertex to bag (depth: {})", app_state.bag.len());
                }
            }
        }
        // Ctrl+Y: Pop from bag (remove top without connecting)
        if i.key_pressed(egui::Key::Y) && i.modifiers.ctrl {
            if let Some(_popped) = app_state.bag.pop() {
                app_state.status = format!("Popped from bag (depth: {})", app_state.bag.len());
            } else {
                app_state.status = "Bag is empty".to_string();
            }
        }
        // G: Go to top of bag (jump to that vertex)
        if i.key_pressed(egui::Key::G) && !i.modifiers.ctrl && app_state.input_mode == InputMode::Normal {
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
        // Delete key: Delete current vertex (if editable)
        if i.key_pressed(egui::Key::Delete) && !i.modifiers.ctrl && app_state.input_mode == InputMode::Normal && app_state.connected {
            if let Some(current_id) = app_state.current_vertex {
                if let Some(vertex) = graph.vertices.get(&current_id).cloned() {
                    // Check if vertex is editable (edit_mask != 0)
                    if vertex.edit_mask != 0 {
                        // Find a neighbor to navigate to after deletion
                        let next_vertex = vertex.edges.iter()
                            .find(|&&e| e != 0)
                            .copied();

                        // Send delete command
                        if let Some(ref tx) = ws_cmd_tx.0 {
                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                            let _ = tx.send(WsCommand::DeleteVertex { action_id, vertex_id: current_id });
                            app_state.status = "Deleting vertex...".to_string();

                            // Remove the vertex from local graph immediately (optimistic delete)
                            graph.vertices.remove(&current_id);
                            // Remove from landmark_vertices
                            for vertices in graph.landmark_vertices.values_mut() {
                                vertices.retain(|&id| id != current_id);
                            }
                            // Remove from history
                            app_state.history.retain(|&id| id != current_id);

                            // Navigate to a neighbor (or go back in history)
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
        // Text input mode shortcuts (only in Normal mode)
        if app_state.input_mode == InputMode::Normal && app_state.connected {
            // I: Insert text at current vertex (edit)
            if i.key_pressed(egui::Key::I) && !i.modifiers.ctrl && !i.modifiers.shift {
                if let Some(current_id) = app_state.current_vertex {
                    // Pre-fill with current content - check layer 1 first (transcript), then layer 0
                    if let Some(vertex) = graph.vertices.get(&current_id) {
                        let mime = vertex.mime.as_deref().unwrap_or("");
                        // Check for layer 1 text (transcript)
                        if let Some(layer1) = vertex.layers.get(&1) {
                            if layer1.mime.starts_with("text/") {
                                app_state.text_input_buffer = String::from_utf8_lossy(&layer1.data).to_string();
                            } else {
                                app_state.text_input_buffer.clear();
                            }
                        } else if mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url" {
                            // Layer 0 is text
                            app_state.text_input_buffer = String::from_utf8_lossy(&vertex.label).to_string();
                        } else {
                            // No text content - start fresh (will add as layer 1)
                            app_state.text_input_buffer.clear();
                        }
                    }
                    app_state.input_mode = InputMode::TextInput { direction: None };
                    app_state.status = "Text input mode (editing current vertex)".to_string();
                }
            }
            // Shift+Direction: Create new text vertex in that direction
            if i.modifiers.shift {
                let dir = if i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::W) {
                    Some(EDGE_NORTH)
                } else if i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::S) {
                    Some(EDGE_SOUTH)
                } else if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::A) {
                    Some(EDGE_WEST)
                } else if i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::D) {
                    Some(EDGE_EAST)
                } else if i.key_pressed(egui::Key::PageUp) {
                    Some(EDGE_UP)
                } else if i.key_pressed(egui::Key::PageDown) {
                    Some(EDGE_DOWN)
                } else {
                    None
                };
                if let Some(direction) = dir {
                    app_state.text_input_buffer.clear();
                    app_state.input_mode = InputMode::TextInput { direction: Some(direction) };
                    let dir_name = match direction {
                        EDGE_NORTH => "north",
                        EDGE_SOUTH => "south",
                        EDGE_WEST => "west",
                        EDGE_EAST => "east",
                        EDGE_UP => "up",
                        EDGE_DOWN => "down",
                        _ => "?",
                    };
                    app_state.status = format!("Text input mode (new vertex {})", dir_name);
                }
            }
            // Space bar: Push-to-talk recording
            // Hold space to record, release to stop and save
            // Records in the last navigation direction (creates note in same direction you were moving)
            if i.key_pressed(egui::Key::Space) && !i.modifiers.ctrl && !i.modifiers.shift {
                // Start recording when space is pressed
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
                app_state.status = "🔴 Recording... (release Space to save)".to_string();
            }
        }
        // Check for space release while recording (push-to-talk stop)
        if let InputMode::Recording { .. } = &app_state.input_mode {
            if i.key_released(egui::Key::Space) {
                // Signal to stop recording
                if let Ok(mut stop) = audio_signal.should_stop.lock() {
                    *stop = true;
                }
            }
        }
    });

    // Check if we should finalize recording (space was released)
    let should_finalize = if let InputMode::Recording { .. } = &app_state.input_mode {
        // Check if stop signal was set (indicates space was released)
        audio_signal.should_stop.lock().map(|s| *s).unwrap_or(false)
    } else {
        false
    };

    if should_finalize {
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

    // Apply zoom by scaling the UI - we do this manually in rendering instead of using pixels_per_point
    // because set_pixels_per_point causes layout issues

    // Check for F5 refresh (outside of any specific UI element)
    let f5_pressed = ctx.input(|i| i.key_pressed(egui::Key::F5));

    // Top panel with URL bar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("URL:");
            // Lock keyboard input while Ctrl+L is being pressed to prevent 'l' from being typed
            let lock_input = app_state.focus_url_bar_next_frame;
            let text_edit = egui::TextEdit::singleline(&mut app_state.url_input)
                .id(url_bar_id)
                .desired_width(600.0)
                .lock_focus(lock_input)
                .hint_text("ws://localhost:8080/ws?landmark=/home/");
            let response = ui.add(text_edit);

            // Show different button based on connection state
            let button_label = if app_state.connected { "Refresh" } else { "Connect" };
            let button_clicked = ui.button(button_label).clicked();
            // lost_focus() is true when Enter is pressed in a text field
            let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

            // Connect/refresh on button click, Enter, or F5
            if button_clicked || enter_pressed || f5_pressed {
                let url = app_state.url_input.trim().to_string();
                if url.is_empty() {
                    app_state.status = "URL is empty!".to_string();
                } else {
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
                        if let Err(err) = run_ws(url_clone, tx.clone(), cmd_rx) {
                            let _ = tx.send(ServerEvent::Error {
                                message: format!("{err:#}"),
                            });
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
            ui.label("↑↓←→ Nav | Enter=Click | Ctrl+Enter=View | Space=Record | I=Edit | Y=Yank | G=Bag | F5=Refresh");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔑 Identities").clicked() {
                    app_state.show_identity_panel = !app_state.show_identity_panel;
                }
                let id_count = app_state.identity_config.identities.len();
                if id_count > 0 {
                    ui.label(format!("{} identity(s)", id_count));
                }
                ui.separator();
                // Bag indicator
                let bag_count = app_state.bag.len();
                let bag_label = if bag_count > 0 {
                    format!("📋 Bag: {}", bag_count)
                } else {
                    "📋 Bag: empty".to_string()
                };
                if ui.button(&bag_label).clicked() {
                    app_state.show_bag_panel = !app_state.show_bag_panel;
                }
            });
        });
        ui.add_space(4.0);
    });

    // Right panel for content preview
    egui::SidePanel::right("preview_panel").min_width(400.0).show(ctx, |ui| {
        ui.heading("Content Preview");
        ui.separator();

        if let Some(current_id) = app_state.current_vertex {
            if let Some(vertex) = graph.vertices.get(&current_id) {
                render_vertex_content(ui, vertex, current_id, &mut media_cache, ctx, &mut app_state, &playback_state);
            }
        } else {
            ui.label("No vertex selected");
        }
    });

    // Text modal window (Ctrl+Enter to open, Escape to close)
    if app_state.show_text_modal {
        egui::Window::new("Text Viewer")
            .collapsible(false)
            .resizable(true)
            .default_size([800.0, 600.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Close (Esc)").clicked() {
                        app_state.show_text_modal = false;
                    }
                });
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
            });
    }

    // Image modal window (Ctrl+Enter to open, Escape to close)
    if app_state.show_image_modal {
        if let Some(vertex_id) = app_state.image_modal_vertex_id {
            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                // Prefer layer 2 content (full image) over layer 0 (thumbnail/label)
                let (image_data, mime): (&[u8], &str) = if let Some(layer2) = vertex.layers.get(&2) {
                    (&layer2.data, &layer2.mime)
                } else {
                    (&vertex.label, vertex.mime.as_deref().unwrap_or(""))
                };
                egui::Window::new("Image Viewer")
                    .collapsible(false)
                    .resizable(true)
                    .default_size([800.0, 600.0])
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Close (Esc)").clicked() {
                                app_state.show_image_modal = false;
                            }
                        });
                        ui.separator();

                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if mime == "image/gif" {
                                    if let Some(animated) = get_or_load_animated_gif(vertex_id, image_data, &mut media_cache, ctx) {
                                        let now = Instant::now();
                                        if now.duration_since(animated.last_switch) >= animated.delays[animated.current_frame] {
                                            let next_frame = (animated.current_frame + 1) % animated.frames.len();
                                            if let Some(anim) = media_cache.animated_gifs.get_mut(&vertex_id) {
                                                anim.current_frame = next_frame;
                                                anim.last_switch = now;
                                            }
                                        }
                                        let tex = &animated.frames[animated.current_frame];
                                        let size = tex.size_vec2();
                                        ui.image((tex.id(), size));
                                        ctx.request_repaint();
                                    }
                                } else {
                                    if let Some(tex) = get_or_load_texture(vertex_id, image_data, mime, &mut media_cache, ctx) {
                                        let size = tex.size_vec2();
                                        ui.image((tex.id(), size));
                                    }
                                }
                            });
                    });
            }
        }
    }

    // Video modal window (opens when video loads, Escape to close)
    if app_state.show_video_modal {
        if let Some(vertex_id) = app_state.video_modal_vertex_id {
            // Update video texture from decoded frames
            // First, collect frame data without holding player borrow
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

            // Now update texture without borrow conflict (skip empty frames)
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

            // Request continuous repaints while video is playing
            if is_playing {
                ctx.request_repaint();
            }

            egui::Window::new("Video Player")
                .collapsible(false)
                .resizable(true)
                .default_size([900.0, 600.0])
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    // Display video frame first (takes up most space)
                    let video_rect = if let Some(tex) = media_cache.video_textures.get(&vertex_id) {
                        // Reserve space for controls below (about 60px)
                        let available = ui.available_size() - egui::vec2(0.0, 60.0);
                        let tex_size = tex.size_vec2();
                        // Scale to fit available space while maintaining aspect ratio
                        let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                        let display_size = tex_size * scale;

                        ui.vertical_centered(|ui| {
                            ui.image((tex.id(), display_size));
                        });
                        Some(display_size)
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(200.0);
                            ui.label("Loading video...");
                            ui.add_space(200.0);
                        });
                        None
                    };

                    ui.add_space(8.0);

                    // Seek bar (same width as video, centered)
                    if let Some(player) = media_cache.video_players.get(&vertex_id) {
                        let pos = player.get_position();
                        let dur = player.duration;
                        let dur_secs = dur.as_secs_f32().max(0.1);
                        let mut pos_secs = pos.as_secs_f32();

                        let slider_width = video_rect.map(|r| r.x).unwrap_or(ui.available_width());

                        // Center the seek bar under the video
                        let available_width = ui.available_width();
                        let left_padding = ((available_width - slider_width) / 2.0).max(0.0);

                        ui.horizontal(|ui| {
                            ui.add_space(left_padding);
                            // Use a custom widget size to force the slider to be exactly the video width
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(slider_width, 20.0),
                                egui::Sense::click_and_drag()
                            );

                            if ui.is_rect_visible(rect) {
                                // Draw custom progress bar / seek bar
                                let progress = pos_secs / dur_secs;
                                let filled_width = rect.width() * progress;

                                // Background
                                ui.painter().rect_filled(
                                    rect,
                                    4.0,
                                    egui::Color32::from_gray(60)
                                );

                                // Filled portion
                                let filled_rect = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(filled_width, rect.height())
                                );
                                ui.painter().rect_filled(
                                    filled_rect,
                                    4.0,
                                    egui::Color32::from_rgb(100, 150, 255)
                                );

                                // Handle dragging
                                if response.dragged() || response.clicked() {
                                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                                        let relative_x = (pointer_pos.x - rect.left()) / rect.width();
                                        let new_pos = (relative_x.clamp(0.0, 1.0) * dur_secs) as f32;
                                        if (new_pos - pos_secs).abs() > 0.1 {
                                            pos_secs = new_pos;
                                        }
                                    }
                                }

                                // Draw position indicator (small circle)
                                let indicator_x = rect.left() + filled_width;
                                let indicator_center = egui::pos2(indicator_x, rect.center().y);
                                ui.painter().circle_filled(
                                    indicator_center,
                                    8.0,
                                    egui::Color32::WHITE
                                );
                            }
                        });

                        if pos_secs != pos.as_secs_f32() {
                            player.seek(Duration::from_secs_f32(pos_secs));
                        }
                    }

                    // Controls bar
                    ui.horizontal(|ui| {
                        // Play/Pause button
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
                            // Show position / duration
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
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✕ Close").clicked() {
                                // Stop player
                                if let Some(player) = media_cache.video_players.get(&vertex_id) {
                                    player.stop();
                                }
                                app_state.show_video_modal = false;
                            }
                        });
                    });
                });
        }
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

    // Awaiting Nextcloud activation modal (shown when login flow is in progress)
    let mut cancel_login = false;
    if let Some(ref login_state) = app_state.nextcloud_login_state {
        let nc_url = login_state.nextcloud_url.clone();
        egui::Window::new("⏳ Awaiting Nextcloud Activation")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label(format!("Connecting to: {}", nc_url));
                ui.add_space(10.0);
                ui.label("Please complete the login in your web browser.");
                ui.label("This dialog will close automatically when done.");
                ui.add_space(10.0);
                ui.spinner();
                ui.add_space(10.0);
                if ui.button("Cancel").clicked() {
                    cancel_login = true;
                }
            });
        // Request repaint to keep polling
        ctx.request_repaint();
    }
    if cancel_login {
        app_state.nextcloud_login_state = None;
    }

    // Display name prompt modal (shown after successful Nextcloud login)
    let mut identity_setup_action: Option<bool> = None; // Some(true) = create, Some(false) = cancel
    if let Some(ref mut pending_setup) = app_state.pending_identity_setup {
        egui::Window::new("🔑 Choose Display Name")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.label("Logged in successfully!");
                ui.add_space(5.0);
                ui.label("Choose a display name for this identity.");
                ui.label("(This is just a label - your real identity is the public key URL)");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label("Display Name:");
                    ui.text_edit_singleline(&mut pending_setup.display_name_input);
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Create Identity").clicked() {
                        identity_setup_action = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        identity_setup_action = Some(false);
                    }
                });
            });
    }
    // Handle identity setup action outside the borrow
    if let Some(create) = identity_setup_action {
        if create {
            if let Some(pending_setup) = app_state.pending_identity_setup.take() {
                // Setup identity (generate keys, upload, create share)
                let display_name = pending_setup.display_name_input.trim().to_string();
                let display_name = if display_name.is_empty() {
                    pending_setup.username.clone()
                } else {
                    display_name
                };
                match identity::setup_identity(&pending_setup.nextcloud_url, &pending_setup.username, &pending_setup.app_password, &display_name) {
                    Ok((signing_key, share_url)) => {
                        eprintln!("Identity created with display name: {}", display_name);
                        eprintln!("Identity URL (real identity): {}", share_url);
                        let new_identity = Identity {
                            display_name: display_name.clone(),
                            nextcloud_url: pending_setup.nextcloud_url,
                            username: pending_setup.username,
                            app_password: pending_setup.app_password,
                            share_url,
                            remembered_servers: Vec::new(),
                            signing_key: Some(signing_key),
                        };
                        app_state.identity_config.identities.push(new_identity);
                        let _ = app_state.identity_config.save();
                        app_state.status = format!("Identity '{}' created successfully!", display_name);
                    }
                    Err(e) => {
                        eprintln!("Failed to setup identity: {}", e);
                        app_state.status = format!("Failed to setup identity: {}", e);
                    }
                }
            }
        } else {
            app_state.pending_identity_setup = None;
            app_state.status = "Identity creation cancelled.".to_string();
        }
    }

    // Identity management panel
    if app_state.show_identity_panel && app_state.nextcloud_login_state.is_none() && app_state.pending_identity_setup.is_none() {
        egui::Window::new("🔑 Identity Management")
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        app_state.show_identity_panel = false;
                    }
                });
                ui.separator();

                // List existing identities
                ui.heading("Your Identities");
                if app_state.identity_config.identities.is_empty() {
                    ui.label("No identities configured. Add a Nextcloud account below.");
                } else {
                    let mut to_remove = None;
                    for (i, identity) in app_state.identity_config.identities.iter().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&identity.display_name);
                                ui.label("(display name)");
                                if ui.small_button("Remove").clicked() {
                                    to_remove = Some(i);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Identity URL:");
                                ui.monospace(&identity.share_url);
                            });
                        });
                    }
                    if let Some(i) = to_remove {
                        app_state.identity_config.identities.remove(i);
                        let _ = app_state.identity_config.save();
                    }
                }

                ui.separator();
                ui.heading("Add Nextcloud Account");

                ui.horizontal(|ui| {
                    ui.label("Nextcloud URL:");
                    ui.text_edit_singleline(&mut app_state.nextcloud_url_input);
                });

                if ui.button("Connect Nextcloud Account").clicked() {
                    let nc_url = app_state.nextcloud_url_input.trim().to_string();
                    if !nc_url.is_empty() {
                        match identity::initiate_nextcloud_login(&nc_url) {
                            Ok((login_url, poll_endpoint, poll_token)) => {
                                // Open browser for login in a separate thread (non-blocking)
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
                    }
                }
            });
    }

    // Bag (clipboard) panel
    if app_state.show_bag_panel {
        egui::Window::new("📋 Bag (Clipboard)")
            .collapsible(false)
            .resizable(true)
            .default_size([300.0, 300.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Close (Ctrl+B)").clicked() {
                        app_state.show_bag_panel = false;
                    }
                    if ui.button("Clear All").clicked() {
                        app_state.bag.clear();
                        app_state.status = "Bag cleared".to_string();
                    }
                });
                ui.separator();

                ui.label("Keyboard shortcuts:");
                ui.label("  Y = Yank current vertex to bag");
                ui.label("  Ctrl+Y = Pop from bag");
                ui.label("  G = Go to bag top");
                ui.separator();

                if app_state.bag.is_empty() {
                    ui.label("Bag is empty. Press Y to yank current vertex.");
                } else {
                    ui.label(format!("{} item(s) in bag:", app_state.bag.len()));
                    ui.add_space(4.0);

                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let mut remove_idx = None;
                            let mut jump_to = None;

                            // Show items with newest (top of stack) first
                            for (i, &vertex_id) in app_state.bag.iter().rev().enumerate() {
                                let stack_idx = app_state.bag.len() - 1 - i;
                                ui.horizontal(|ui| {
                                    let is_top = i == 0;
                                    let prefix = if is_top { "→ " } else { "  " };

                                    // Get vertex label if available
                                    let label = graph.vertices.get(&vertex_id)
                                        .map(|v| {
                                            let mime = v.mime.as_deref().unwrap_or("");
                                            let text = String::from_utf8_lossy(&v.label);
                                            let short: String = text.chars().take(20).collect();
                                            format!("[{}] {}", mime.split('/').last().unwrap_or("?"), short)
                                        })
                                        .unwrap_or_else(|| format!("vertex {}", vertex_id));

                                    ui.label(format!("{}{}", prefix, label));

                                    if ui.small_button("Go").clicked() {
                                        jump_to = Some(vertex_id);
                                    }
                                    if ui.small_button("×").clicked() {
                                        remove_idx = Some(stack_idx);
                                    }
                                });
                            }

                            if let Some(idx) = remove_idx {
                                app_state.bag.remove(idx);
                            }
                            if let Some(vid) = jump_to {
                                if let Some(current_id) = app_state.current_vertex {
                                    app_state.history.push(current_id);
                                }
                                app_state.current_vertex = Some(vid);
                            }
                        });
                }
            });
    }

    // Text input modal
    let mut submit_text = false;
    let mut cancel_text = false;
    if let InputMode::TextInput { direction } = app_state.input_mode.clone() {
        let title = if direction.is_some() {
            "✏️ New Text Note"
        } else {
            "✏️ Edit Text"
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size([500.0, 300.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Save (Ctrl+Enter)").clicked() {
                        submit_text = true;
                    }
                    if ui.button("Cancel (Esc)").clicked() {
                        cancel_text = true;
                    }
                });

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

                ui.separator();

                // Text input area
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .show(ui, |ui| {
                        let response = ui.add(
                            egui::TextEdit::multiline(&mut app_state.text_input_buffer)
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .font(egui::TextStyle::Monospace)
                        );
                        // Request focus on the text input
                        response.request_focus();
                    });

                // Handle Ctrl+Enter to submit and Escape to cancel
                let input = ui.input(|i| (
                    i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl,
                    i.key_pressed(egui::Key::Escape)
                ));
                if input.0 {
                    submit_text = true;
                }
                if input.1 {
                    cancel_text = true;
                }
            });
    }

    // Process text input submission/cancellation outside the UI closure
    if submit_text {
        if let InputMode::TextInput { direction } = app_state.input_mode.clone() {
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
                            // Store data for populating vertex locally on ack
                            app_state.pending_creations.insert(action_id, PendingVertexCreation {
                                samples: Vec::new(), // No audio samples for text
                                sample_rate: 0,
                                data: text_bytes,
                                mime: "text/plain".to_string(),
                            });
                            app_state.status = "Creating new note...".to_string();
                        } else {
                            // Update existing vertex
                            // Determine which layer to save to:
                            // - If primary is text, save to layer 0
                            // - If primary is not text (e.g., audio), save to layer 1 (transcript/annotation)
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
                            // Optimistically update local graph immediately
                            let _ = net_events.0.send(ServerEvent::SetVertexLabel {
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
    }
    if cancel_text {
        app_state.input_mode = InputMode::Normal;
        app_state.text_input_buffer.clear();
        app_state.status = "Text input cancelled".to_string();
    }

    // Audio recording indicator (push-to-talk style)
    let mut cancel_recording = false;
    if let InputMode::Recording { .. } = app_state.input_mode.clone() {
        let elapsed = app_state.recording_start
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);
        let elapsed_secs = elapsed.as_secs_f32();

        egui::Window::new("🎤 Recording")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("🔴");
                    ui.heading(format!("{:.1}s", elapsed_secs));
                });
                ui.label("Release Space to save");

                // Show audio level indicator
                let level = if let Ok(samples) = app_state.audio_samples.lock() {
                    if samples.len() > 1000 {
                        let recent: f32 = samples.iter().rev().take(1000)
                            .map(|s| s.abs())
                            .sum::<f32>() / 1000.0;
                        recent * 10.0 // Scale for visibility
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(level.min(1.0)));

                // Escape to cancel
                let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
                if escape_pressed {
                    cancel_recording = true;
                }
            });

        // Keep repainting to update the UI
        ctx.request_repaint();
    }

    // Process recording cancel (Escape pressed during recording)
    if cancel_recording {
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
    }

    // Identification consent dialog
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

            // Show consent dialog
            egui::Window::new("🔐 Identification Request")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Server requests identification");
                    ui.separator();

                    ui.label(format!("Server: {}", pending.server_url));
                    ui.label(format!("Reason: {}", pending.reason));
                    ui.separator();

                    if app_state.identity_config.identities.is_empty() {
                        ui.label("No identities configured.");
                        ui.label("You need to connect a Nextcloud account to identify yourself.");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("🔑 Connect Nextcloud Account").clicked() {
                                app_state.show_identity_panel = true;
                            }
                            if ui.button("Refuse (Esc)").clicked() {
                                id_action = Some(IdentificationAction::Refuse);
                            }
                        });
                    } else {
                        ui.label("Identify as (Tab to cycle):");
                        // Collect identity info first to avoid borrow conflict
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
                                    ui.label(format!("URL: {}", url));
                                });
                            }
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Identify (Enter)").clicked() {
                                id_action = Some(IdentificationAction::Identify { remember: false });
                            }
                            if ui.button("Identify + Remember").clicked() {
                                id_action = Some(IdentificationAction::Identify { remember: true });
                            }
                            if ui.button("Refuse (Esc)").clicked() {
                                id_action = Some(IdentificationAction::Refuse);
                            }
                        });
                    }
                });
        }
    }

    // Process identification action outside the UI closure
    if let Some(action) = id_action {
        if let Some(pending) = app_state.pending_identification.take() {
            match action {
                IdentificationAction::Identify { remember } => {
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

    // Central panel showing grid view
    egui::CentralPanel::default().show(ctx, |ui| {
        if !app_state.connected {
            ui.centered_and_justified(|ui| {
                ui.heading("Enter a WebSocket URL above and click Connect");
            });
            return;
        }

        let Some(current_id) = app_state.current_vertex else {
            ui.centered_and_justified(|ui| {
                ui.heading("Waiting for data from server...");
            });
            return;
        };

        let grid = build_grid_view(&graph, current_id);

        let zoom = app_state.zoom_level;
        let cell_width = 160.0f32 * zoom;
        let cell_height = 140.0f32 * zoom;
        let padding = 4.0f32 * zoom;
        let font_size = 13.0f32 * zoom;

        let available = ui.available_size();
        let panel_min = ui.min_rect().min;

        // Find the position of the current vertex in the grid
        let current_pos = grid.positions.get(&current_id).copied().unwrap_or((0, 0));

        // Calculate where the current cell would be in grid-local coordinates
        let current_cell_x = (current_pos.0 - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
        let current_cell_y = (current_pos.1 - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;

        // Calculate offset to center the current cell in the available space
        let offset_x = available.x / 2.0 - current_cell_x;
        let offset_y = available.y / 2.0 - current_cell_y;

        let painter = ui.painter();
        let base_pos = panel_min + egui::vec2(offset_x, offset_y);

        // Draw edge lines first (behind cells)
        // Use different colors for different connection types
        let line_color_ns = egui::Color32::from_rgb(80, 120, 100); // North-South (vertical)
        let line_color_ew = egui::Color32::from_rgb(100, 80, 120); // East-West (horizontal)
        let line_thickness = 2.5 * zoom;

        for ((x, y), &vertex_id) in &grid.cells {
            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                // Calculate cell position
                let from_cell_x = (*x - grid.min_x) as f32 * (cell_width + padding);
                let from_cell_y = (*y - grid.min_y) as f32 * (cell_height + padding);

                // Draw line to south neighbor
                if vertex.edges[EDGE_SOUTH] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_SOUTH]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = (ty - grid.min_y) as f32 * (cell_height + padding);

                        // Draw from bottom edge of source cell to top edge of target cell
                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width / 2.0, from_cell_y + cell_height);
                        let to_edge = base_pos + egui::vec2(to_cell_x + cell_width / 2.0, to_cell_y);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ns));
                    }
                }

                // Draw line to east neighbor
                if vertex.edges[EDGE_EAST] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_EAST]) {
                        let to_cell_x = (tx - grid.min_x) as f32 * (cell_width + padding);
                        let to_cell_y = (ty - grid.min_y) as f32 * (cell_height + padding);

                        // Draw from right edge of source cell to left edge of target cell
                        let from_edge = base_pos + egui::vec2(from_cell_x + cell_width, from_cell_y + cell_height / 2.0);
                        let to_edge = base_pos + egui::vec2(to_cell_x, to_cell_y + cell_height / 2.0);

                        painter.line_segment([from_edge, to_edge], egui::Stroke::new(line_thickness, line_color_ew));
                    }
                }
            }
        }
        
        // Draw cells
        for y in grid.min_y..=grid.max_y {
            for x in grid.min_x..=grid.max_x {
                if let Some(&vertex_id) = grid.cells.get(&(x, y)) {
                    let cell_x = (x - grid.min_x) as f32 * (cell_width + padding);
                    let cell_y = (y - grid.min_y) as f32 * (cell_height + padding);

                    let rect = egui::Rect::from_min_size(
                        base_pos + egui::vec2(cell_x, cell_y),
                        egui::vec2(cell_width, cell_height),
                    );

                    let is_current = vertex_id == current_id;

                    if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        let mime = vertex.mime.as_deref().unwrap_or("");
                        let primary_is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
                        let primary_is_audio = mime.starts_with("audio/");

                        // Collect all content types present
                        let mut has_image = primary_is_image;
                        let mut has_audio = primary_is_audio;
                        let mut has_text = mime.starts_with("text/") && !mime.contains("gradesta-url");
                        let mut text_content: Option<String> = None;

                        // Check additional layers for images, audio, text
                        for layer in vertex.layers.values() {
                            if layer.mime.starts_with("image/") || is_image_data(&layer.data) {
                                has_image = true;
                            } else if layer.mime.starts_with("audio/") {
                                has_audio = true;
                            } else if layer.mime.starts_with("text/") && !layer.mime.contains("gradesta-url") {
                                has_text = true;
                                if text_content.is_none() {
                                    text_content = String::from_utf8(layer.data.clone()).ok();
                                }
                            }
                        }

                        // For primary text content
                        if mime.starts_with("text/") && !mime.contains("gradesta-url") && text_content.is_none() {
                            text_content = String::from_utf8(vertex.label.clone()).ok();
                        }

                        // Different colors based on content type
                        let (bg_color, border_color) = if is_current {
                            (egui::Color32::from_rgb(50, 100, 70), egui::Color32::from_rgb(100, 200, 120))
                        } else if mime == "text/gradesta-url" {
                            (egui::Color32::from_rgb(60, 60, 90), egui::Color32::from_rgb(100, 100, 150))
                        } else if has_image {
                            (egui::Color32::from_rgb(40, 40, 45), egui::Color32::from_rgb(140, 100, 140))
                        } else if has_audio {
                            (egui::Color32::from_rgb(70, 60, 50), egui::Color32::from_rgb(140, 120, 100))
                        } else if has_text {
                            (egui::Color32::from_rgb(50, 60, 70), egui::Color32::from_rgb(100, 120, 140))
                        } else {
                            (egui::Color32::from_rgb(50, 50, 55), egui::Color32::from_rgb(80, 80, 90))
                        };

                        let corner_radius = 4.0 * zoom;

                        // Check for stacked cards (up/down connections)
                        let stack = compute_stack(&graph, vertex_id);
                        let has_stack = stack.total > 1;

                        // Draw stacked card shadows behind the main card
                        if has_stack {
                            let shadow_color = egui::Color32::from_rgba_unmultiplied(30, 30, 35, 180);
                            let cards_to_show = (stack.total - stack.current_index - 1).min(3); // Cards below current
                            for i in (1..=cards_to_show).rev() {
                                let offset = i as f32 * 4.0 * zoom;
                                let shadow_rect = rect.translate(egui::vec2(offset, offset));
                                painter.rect_filled(shadow_rect, corner_radius, shadow_color);
                                painter.rect_stroke(shadow_rect, corner_radius, egui::Stroke::new(1.0 * zoom, egui::Color32::from_rgb(60, 60, 65)));
                            }
                            // Also draw cards above (offset in opposite direction)
                            let cards_above = stack.current_index.min(3);
                            for i in (1..=cards_above).rev() {
                                let offset = i as f32 * 4.0 * zoom;
                                let shadow_rect = rect.translate(egui::vec2(-offset, -offset));
                                painter.rect_filled(shadow_rect, corner_radius, shadow_color);
                                painter.rect_stroke(shadow_rect, corner_radius, egui::Stroke::new(1.0 * zoom, egui::Color32::from_rgb(60, 60, 65)));
                            }
                        }

                        painter.rect_filled(rect, corner_radius, bg_color);
                        painter.rect_stroke(rect, corner_radius, egui::Stroke::new(if is_current { 3.0 * zoom } else { 2.0 * zoom }, border_color));

                        // Draw stack position badge if this vertex is part of a stack
                        if has_stack {
                            let badge_text = format!("{}/{}", stack.current_index + 1, stack.total);
                            let badge_font = egui::FontId::proportional(font_size * 0.6);
                            let badge_pos = egui::pos2(rect.right() - 4.0 * zoom, rect.top() + 4.0 * zoom);
                            // Draw badge background
                            let badge_rect = egui::Rect::from_center_size(
                                badge_pos + egui::vec2(-12.0 * zoom, 6.0 * zoom),
                                egui::vec2(28.0 * zoom, 14.0 * zoom),
                            );
                            painter.rect_filled(badge_rect, 3.0 * zoom, egui::Color32::from_rgba_unmultiplied(0, 0, 0, 180));
                            painter.text(
                                badge_pos,
                                egui::Align2::RIGHT_TOP,
                                badge_text,
                                badge_font,
                                egui::Color32::from_rgb(200, 200, 255),
                            );
                        }

                        // Draw direction arrow indicator on current cell
                        if is_current {
                            let arrow_color = egui::Color32::from_rgba_unmultiplied(100, 255, 150, 200);
                            let arrow_size = 12.0 * zoom;
                            let arrow_thickness = 3.0 * zoom;
                            let gap = 4.0 * zoom;

                            let (arrow_start, arrow_end) = match app_state.last_nav_direction {
                                EDGE_EAST => {
                                    let mid_y = rect.center().y;
                                    let start = egui::pos2(rect.right() + gap, mid_y);
                                    let end = egui::pos2(rect.right() + gap + arrow_size, mid_y);
                                    (start, end)
                                }
                                EDGE_WEST => {
                                    let mid_y = rect.center().y;
                                    let start = egui::pos2(rect.left() - gap, mid_y);
                                    let end = egui::pos2(rect.left() - gap - arrow_size, mid_y);
                                    (start, end)
                                }
                                EDGE_NORTH => {
                                    let mid_x = rect.center().x;
                                    let start = egui::pos2(mid_x, rect.top() - gap);
                                    let end = egui::pos2(mid_x, rect.top() - gap - arrow_size);
                                    (start, end)
                                }
                                EDGE_SOUTH => {
                                    let mid_x = rect.center().x;
                                    let start = egui::pos2(mid_x, rect.bottom() + gap);
                                    let end = egui::pos2(mid_x, rect.bottom() + gap + arrow_size);
                                    (start, end)
                                }
                                EDGE_UP => {
                                    // Draw up arrow at top-right corner
                                    let start = egui::pos2(rect.right() - 8.0 * zoom, rect.top() - gap);
                                    let end = egui::pos2(rect.right() - 8.0 * zoom, rect.top() - gap - arrow_size);
                                    (start, end)
                                }
                                EDGE_DOWN => {
                                    // Draw down arrow at bottom-right corner
                                    let start = egui::pos2(rect.right() - 8.0 * zoom, rect.bottom() + gap);
                                    let end = egui::pos2(rect.right() - 8.0 * zoom, rect.bottom() + gap + arrow_size);
                                    (start, end)
                                }
                                _ => {
                                    let mid_x = rect.center().x;
                                    let start = egui::pos2(mid_x, rect.bottom() + gap);
                                    let end = egui::pos2(mid_x, rect.bottom() + gap + arrow_size);
                                    (start, end)
                                }
                            };

                            // Draw arrow line
                            painter.line_segment([arrow_start, arrow_end], egui::Stroke::new(arrow_thickness, arrow_color));

                            // Draw arrowhead
                            let dir = (arrow_end - arrow_start).normalized();
                            let perp = egui::vec2(-dir.y, dir.x);
                            let head_size = 6.0 * zoom;
                            let head_base = arrow_end - dir * head_size;
                            let head_left = head_base + perp * head_size * 0.5;
                            let head_right = head_base - perp * head_size * 0.5;
                            painter.add(egui::Shape::convex_polygon(
                                vec![arrow_end, head_left, head_right],
                                arrow_color,
                                egui::Stroke::NONE,
                            ));
                        }

                        // Calculate layout sections based on what content we have
                        let inner_rect = rect.shrink(4.0 * zoom);
                        let num_sections = (has_image as usize) + (has_audio as usize) + (has_text as usize);
                        let section_height = if num_sections > 0 { inner_rect.height() / num_sections as f32 } else { inner_rect.height() };

                        let mut y_offset = 0.0;

                        // Render image section
                        if has_image {
                            let section_rect = egui::Rect::from_min_size(
                                inner_rect.min + egui::vec2(0.0, y_offset),
                                egui::vec2(inner_rect.width(), section_height),
                            );

                            // Try primary layer first, then additional layers
                            let (img_data, img_mime) = if primary_is_image {
                                (&vertex.label, mime)
                            } else {
                                vertex.layers.values()
                                    .find(|l| l.mime.starts_with("image/") || is_image_data(&l.data))
                                    .map(|l| (&l.data, l.mime.as_str()))
                                    .unwrap_or((&vertex.label, mime))
                            };

                            if let Some(tex) = get_or_load_texture(vertex_id, img_data, img_mime, &mut media_cache, ctx) {
                                let tex_size = tex.size_vec2();
                                let scale = (section_rect.width() / tex_size.x).min(section_rect.height() / tex_size.y);
                                let scaled_size = tex_size * scale;
                                let img_rect = egui::Rect::from_center_size(section_rect.center(), scaled_size);
                                painter.image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                            y_offset += section_height;
                        }

                        // Render audio waveform section
                        if has_audio {
                            let section_rect = egui::Rect::from_min_size(
                                inner_rect.min + egui::vec2(0.0, y_offset),
                                egui::vec2(inner_rect.width(), section_height),
                            ).shrink(2.0 * zoom);

                            let num_bars = (section_rect.width() / (3.0 * zoom)) as usize;
                            let num_bars = num_bars.max(10).min(64);

                            // Get audio data from primary or layers
                            let audio_data = if primary_is_audio {
                                &vertex.label
                            } else {
                                vertex.layers.values()
                                    .find(|l| l.mime.starts_with("audio/"))
                                    .map(|l| &l.data)
                                    .unwrap_or(&vertex.label)
                            };

                            if let Some(waveform) = get_or_generate_waveform(vertex_id, audio_data, &mut media_cache, num_bars) {
                                let waveform_color = if is_current {
                                    egui::Color32::from_rgb(100, 200, 255)
                                } else {
                                    egui::Color32::from_rgb(140, 120, 100)
                                };
                                draw_waveform(&painter, section_rect, waveform, waveform_color, egui::Color32::TRANSPARENT);
                            }

                            // Draw small speaker icon
                            painter.text(
                                egui::pos2(section_rect.left() + 2.0 * zoom, section_rect.top() + 2.0 * zoom),
                                egui::Align2::LEFT_TOP,
                                "🔊",
                                egui::FontId::proportional(font_size * 0.6),
                                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 150),
                            );
                            y_offset += section_height;
                        }

                        // Render text section
                        if has_text {
                            let section_rect = egui::Rect::from_min_size(
                                inner_rect.min + egui::vec2(0.0, y_offset),
                                egui::vec2(inner_rect.width(), section_height),
                            );

                            if let Some(ref text) = text_content {
                                let text_font_size = font_size * 0.9;
                                let char_width = text_font_size * 0.5;
                                let line_height = text_font_size * 1.2;
                                let chars_per_line = ((section_rect.width() - 4.0 * zoom) / char_width) as usize;
                                let chars_per_line = chars_per_line.max(5);
                                let max_lines = ((section_rect.height() - 4.0 * zoom) / line_height) as usize;
                                let max_lines = max_lines.max(1);

                                // Word wrap the text, up to tweet length (280 chars) before truncating
                                let max_chars = 280.min(chars_per_line * max_lines);
                                let text_to_wrap: String = text.chars().take(max_chars).collect();
                                let needs_ellipsis = text.chars().count() > max_chars;

                                // Simple word wrap
                                let mut lines: Vec<String> = Vec::new();
                                let mut current_line = String::new();

                                for word in text_to_wrap.split_whitespace() {
                                    if current_line.is_empty() {
                                        current_line = word.to_string();
                                    } else if current_line.chars().count() + 1 + word.chars().count() <= chars_per_line {
                                        current_line.push(' ');
                                        current_line.push_str(word);
                                    } else {
                                        lines.push(current_line);
                                        current_line = word.to_string();
                                        if lines.len() >= max_lines {
                                            break;
                                        }
                                    }
                                }
                                if !current_line.is_empty() && lines.len() < max_lines {
                                    lines.push(current_line);
                                }

                                // Add ellipsis to last line if truncated
                                if needs_ellipsis && !lines.is_empty() {
                                    let last = lines.last_mut().unwrap();
                                    if last.chars().count() + 1 <= chars_per_line {
                                        last.push('…');
                                    } else {
                                        // Truncate last word to make room
                                        let truncated: String = last.chars().take(chars_per_line - 1).collect();
                                        *last = format!("{}…", truncated);
                                    }
                                }

                                // Draw each line
                                let total_text_height = lines.len() as f32 * line_height;
                                let start_y = section_rect.center().y - total_text_height / 2.0 + line_height / 2.0;

                                for (i, line) in lines.iter().enumerate() {
                                    painter.text(
                                        egui::pos2(section_rect.center().x, start_y + i as f32 * line_height),
                                        egui::Align2::CENTER_CENTER,
                                        line,
                                        egui::FontId::proportional(text_font_size),
                                        egui::Color32::WHITE,
                                    );
                                }
                            }
                        }

                        // If no content at all, show placeholder
                        if !has_image && !has_audio && !has_text {
                            let icon = if mime == "text/gradesta-url" {
                                "🌀"
                            } else if mime == "text/x-url" {
                                "📎"
                            } else if mime.starts_with("video/") {
                                "🎬"
                            } else if !mime.is_empty() {
                                "📦"
                            } else {
                                "◻"
                            };

                            // For portals, also show destination
                            let display = if mime == "text/gradesta-url" {
                                let url = String::from_utf8_lossy(&vertex.label);
                                let short_url: String = url.chars().take(20).collect();
                                format!("{} {}", icon, if url.len() > 20 { format!("{}…", short_url) } else { short_url })
                            } else {
                                icon.to_string()
                            };

                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                display,
                                egui::FontId::proportional(font_size),
                                egui::Color32::WHITE,
                            );
                        }
                    } else {
                        // No vertex data - just draw empty cell
                        let corner_radius = 4.0 * zoom;
                        painter.rect_filled(rect, corner_radius, egui::Color32::from_rgb(50, 50, 55));
                        painter.rect_stroke(rect, corner_radius, egui::Stroke::new(2.0 * zoom, egui::Color32::from_rgb(80, 80, 90)));
                    }

                    // Detect clicks on cells (for toggle/click actions)
                    let response = ui.interact(rect, egui::Id::new(("cell", vertex_id)), egui::Sense::click());
                    if response.clicked() && !is_current {
                        // Send click message to server
                        if let Some(ref tx) = ws_cmd_tx.0 {
                            let action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);
                            let _ = tx.send(WsCommand::ClickVertex { action_id, vertex_id });
                            app_state.status = format!("Clicked vertex {}", vertex_id);
                        }
                    }
                }
            }
        }
    });
}

fn render_vertex_content(
    ui: &mut egui::Ui,
    vertex: &Vertex,
    vertex_id: u64,
    media_cache: &mut MediaCache,
    ctx: &egui::Context,
    _app_state: &mut AppState,
    playback_state: &AudioPlaybackState,
) {
    let label = String::from_utf8_lossy(&vertex.label);

    if let Some(mime) = &vertex.mime {
        // Show type and size for all content
        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.monospace(mime);
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.label(format!("{} bytes", vertex.label.len()));
        });

        ui.separator();

        if mime == "text/gradesta-url" {
            // Portal to another landmark - should be auto-loading
            ui.heading("🌀 Loading...");
            let url = String::from_utf8_lossy(&vertex.label);
            ui.spinner();
            ui.add_space(8.0);
            let url_display: String = url.chars().take(60).collect();
            ui.label(format!("Loading: {}", url_display));

        } else if mime == "text/x-url" {
            // External URL - file too large to inline, open externally
            ui.heading("📎 External Content");
            ui.add_space(8.0);
            let url = String::from_utf8_lossy(&vertex.label);
            ui.label("This file is too large to display inline.");
            ui.add_space(4.0);
            ui.monospace(&*url);
            ui.add_space(8.0);
            if ui.button("🔗 Open with system handler").clicked() {
                // Open file:// URL with xdg-open
                let url_str = url.to_string();
                let _ = Command::new("xdg-open")
                    .arg(&url_str)
                    .spawn();
            }

        } else if mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url" {
            // Text content - show in constrained scroll area
            ui.heading("📄 Text Content");
            ui.add_space(4.0);
            ui.label("Ctrl+Enter to open in modal");
            ui.add_space(4.0);

            let text = String::from_utf8_lossy(&vertex.label);
            let available_height = ui.available_height().min(400.0).max(100.0);

            egui::ScrollArea::vertical()
                .max_height(available_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut text.to_string())
                            .desired_width(ui.available_width())
                            .desired_rows(10)
                            .font(egui::TextStyle::Monospace)
                            .interactive(false)
                    );
                });

        } else if mime.starts_with("image/") || is_image_data(&vertex.label) {
            // Image content
            ui.heading("🖼 Image");
            ui.add_space(4.0);
            ui.label("Ctrl+Enter to view full size");
            ui.add_space(4.0);

            let is_gif = mime == "image/gif" || (vertex.label.len() >= 6 && &vertex.label[0..6] == b"GIF89a" || vertex.label.len() >= 6 && &vertex.label[0..6] == b"GIF87a");

            if is_gif {
                // Animated GIF
                if let Some(animated) = get_or_load_animated_gif(vertex_id, &vertex.label, media_cache, ctx) {
                    // Update animation
                    let now = Instant::now();
                    if now.duration_since(animated.last_switch) >= animated.delays[animated.current_frame] {
                        let next_frame = (animated.current_frame + 1) % animated.frames.len();
                        if let Some(anim) = media_cache.animated_gifs.get_mut(&vertex_id) {
                            anim.current_frame = next_frame;
                            anim.last_switch = now;
                        }
                    }
                    
                    let tex = &animated.frames[animated.current_frame];
                    let size = tex.size_vec2();
                    let max_size = egui::vec2(380.0, 400.0);
                    let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                    ui.image((tex.id(), size * scale));
                    
                    ui.label(format!("Frame {}/{}", animated.current_frame + 1, animated.frames.len()));
                    ctx.request_repaint(); // Keep animating
                }
            } else {
                // Static image
                match get_or_load_texture(vertex_id, &vertex.label, mime, media_cache, ctx) {
                    Some(tex) => {
                        let size = tex.size_vec2();
                        let max_size = egui::vec2(380.0, 400.0);
                        let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                        ui.image((tex.id(), size * scale));
                        ui.label(format!("{}x{}", size.x as u32, size.y as u32));
                    }
                    None => {
                        ui.label("Failed to decode image");
                    }
                }
            }
            
        } else if mime.starts_with("audio/") {
            // Audio content - inline playback
            ui.heading("🔊 Audio");
            ui.add_space(4.0);
            ui.label(format!("Type: {}", mime));

            // Show file size
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(8.0);

            // Play button
            if ui.button("▶ Play").clicked() {
                play_audio(&vertex.label, mime, vertex_id, playback_state);
            }

            // Check for transcript - first in any layer with text/plain, then embedded in audio
            let transcript = vertex.layers.values()
                .find(|layer| layer.mime == "text/plain")
                .and_then(|layer| String::from_utf8(layer.data.clone()).ok())
                .or_else(|| extract_transcript(&vertex.label, mime));

            if let Some(transcript) = transcript {
                ui.add_space(8.0);
                ui.separator();
                ui.heading("📝 Transcript");
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.label(&transcript);
                    });
            }

            ui.add_space(8.0);
            if ui.button("📂 Open with external player").clicked() {
                open_with_external(&vertex.label, mime);
            }

        } else if mime.starts_with("video/") {
            // Video - offer to open externally
            ui.heading("🎬 Video");
            ui.add_space(8.0);
            ui.label("Video playback requires external player.");
            ui.label(format!("Type: {}", mime));
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(16.0);

            if ui.button("📂 Open with external player").clicked() {
                open_with_external(&vertex.label, mime);
            }

        } else {
            // Other binary content
            ui.heading("📦 Binary Content");
            ui.add_space(8.0);
            ui.label(format!("Type: {}", mime));
            ui.label(format!("Size: {} bytes", vertex.label.len()));
            ui.add_space(16.0);
            
            if ui.button("📂 Open with system default").clicked() {
                open_with_external(&vertex.label, mime);
            }
            
            // Show hex preview
            ui.add_space(16.0);
            ui.label("Hex preview:");
            let hex_preview: String = vertex.label.iter()
                .take(64)
                .map(|b| format!("{:02x} ", b))
                .collect();
            ui.monospace(&hex_preview);
            if vertex.label.len() > 64 {
                ui.label("...");
            }
        }
    } else {
        ui.label("No content type specified");
        ui.add_space(8.0);
        ui.label(&*label);
    }
}

fn open_with_external(data: &[u8], mime: &str) {
    // Determine file extension from mime type
    let ext = match mime {
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/ogg" => "ogv",
        "audio/mpeg" => "mp3",
        "audio/ogg" => "ogg",
        "audio/wav" => "wav",
        "application/pdf" => "pdf",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "bin",
    };
    
    // Write to temp file
    let temp_path = format!("/tmp/gradesta_preview_{}.{}", std::process::id(), ext);
    if let Ok(mut file) = std::fs::File::create(&temp_path) {
        if file.write_all(data).is_ok() {
            // Open with xdg-open (Linux)
            let _ = Command::new("xdg-open")
                .arg(&temp_path)
                .spawn();
        }
    }
}

/// Check if data looks like an image based on magic bytes
fn is_image_data(data: &[u8]) -> bool {
    if data.len() < 8 {
        return false;
    }
    // PNG
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return true;
    }
    // JPEG
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    // GIF
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return true;
    }
    // WebP
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        return true;
    }
    // BMP
    if data.starts_with(b"BM") {
        return true;
    }
    false
}

fn get_or_load_texture(
    id: u64,
    data: &[u8],
    _mime: &str,
    cache: &mut MediaCache,
    ctx: &egui::Context,
) -> Option<egui::TextureHandle> {
    // Check if already loaded
    if let Some(handle) = cache.textures.get(&id) {
        return Some(handle.clone());
    }

    // Check if decode is already in progress
    if cache.pending_decodes.contains(&id) {
        return None; // Still decoding
    }

    // For small images (< 100KB), decode synchronously to avoid flicker
    if data.len() < 100_000 {
        if let Ok(img) = image::load_from_memory(data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
            let handle = ctx.load_texture(format!("vertex_{}", id), image, egui::TextureOptions::default());
            cache.textures.insert(id, handle.clone());
            return Some(handle);
        }
        return None;
    }

    // For larger images, decode in background thread
    cache.pending_decodes.insert(id);
    let data = data.to_vec();
    let tx = cache.decoded_tx.clone();

    std::thread::spawn(move || {
        if let Ok(img) = image::load_from_memory(&data) {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let _ = tx.send(DecodedImage {
                id,
                width,
                height,
                rgba: rgba.into_raw(),
            });
        }
    });

    None // Not ready yet
}

fn get_or_load_animated_gif(
    id: u64,
    data: &[u8],
    cache: &mut MediaCache,
    ctx: &egui::Context,
) -> Option<AnimatedGif> {
    if let Some(animated) = cache.animated_gifs.get(&id) {
        return Some(animated.clone());
    }

    let mut options = DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = options.read_info(Cursor::new(data)).ok()?;
    
    let mut frames = Vec::new();
    let mut delays = Vec::new();
    
    while let Ok(Some(frame)) = reader.read_next_frame() {
        let width = frame.width as usize;
        let height = frame.height as usize;
        let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &frame.buffer);
        let handle = ctx.load_texture(format!("gif_{}_{}", id, frames.len()), image, egui::TextureOptions::default());
        frames.push(handle);
        delays.push(Duration::from_millis((frame.delay as u64) * 10));
    }
    
    if frames.is_empty() {
        return None;
    }
    
    let animated = AnimatedGif {
        frames,
        delays,
        current_frame: 0,
        last_switch: Instant::now(),
    };
    cache.animated_gifs.insert(id, animated.clone());
    Some(animated)
}

fn handle_navigation(
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // Don't handle navigation when in text input mode
    if matches!(app_state.input_mode, InputMode::TextInput { .. }) {
        return;
    }

    let Some(current_id) = app_state.current_vertex else { return };
    let Some(vertex) = graph.vertices.get(&current_id) else { return };

    // Check which navigation key is held (if any)
    let held_edge: Option<usize> = if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
        Some(EDGE_NORTH)
    } else if keys.pressed(KeyCode::ArrowDown) || keys.pressed(KeyCode::KeyS) {
        Some(EDGE_SOUTH)
    } else if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
        Some(EDGE_WEST)
    } else if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
        Some(EDGE_EAST)
    } else if keys.pressed(KeyCode::PageUp) {
        Some(EDGE_UP)
    } else if keys.pressed(KeyCode::PageDown) {
        Some(EDGE_DOWN)
    } else {
        None
    };

    // Check for just pressed (initial press)
    let just_pressed_edge: Option<usize> = if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        Some(EDGE_NORTH)
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        Some(EDGE_SOUTH)
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        Some(EDGE_WEST)
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        Some(EDGE_EAST)
    } else if keys.just_pressed(KeyCode::PageUp) {
        Some(EDGE_UP)
    } else if keys.just_pressed(KeyCode::PageDown) {
        Some(EDGE_DOWN)
    } else {
        None
    };

    if keys.just_pressed(KeyCode::Backspace) {
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

/// Run audio recording in a thread until stop signal is set
/// Parse WAV header to get duration in seconds
fn parse_wav_duration(data: &[u8]) -> Option<f32> {
    if data.len() < 44 {
        return None;
    }
    // Check RIFF header
    if &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    // Find fmt chunk
    let mut pos = 12;
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos+4];
        let chunk_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?) as usize;

        if chunk_id == b"fmt " && chunk_size >= 16 {
            let channels = u16::from_le_bytes(data[pos+10..pos+12].try_into().ok()?) as u32;
            let sample_rate = u32::from_le_bytes(data[pos+12..pos+16].try_into().ok()?);
            let bits_per_sample = u16::from_le_bytes(data[pos+22..pos+24].try_into().ok()?) as u32;

            // Find data chunk
            pos += 8 + chunk_size;
            while pos + 8 < data.len() {
                let data_chunk_id = &data[pos..pos+4];
                let data_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?);

                if data_chunk_id == b"data" {
                    let bytes_per_sample = (bits_per_sample / 8) * channels;
                    if bytes_per_sample > 0 && sample_rate > 0 {
                        let num_samples = data_size / bytes_per_sample;
                        return Some(num_samples as f32 / sample_rate as f32);
                    }
                }
                pos += 8 + data_size as usize;
                if data_size % 2 == 1 {
                    pos += 1; // Pad to even
                }
            }
        }
        pos += 8 + chunk_size;
        if chunk_size % 2 == 1 {
            pos += 1; // Pad to even
        }
    }
    None
}

/// Extract transcript from audio data (OGG Vorbis comments or WAV 'trns' chunk)
fn extract_transcript(data: &[u8], mime: &str) -> Option<String> {
    if mime == "audio/ogg" || (data.len() >= 4 && &data[0..4] == b"OggS") {
        // Try to extract from OGG Vorbis comments
        extract_ogg_transcript(data)
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        // Try to extract from WAV 'trns' chunk
        extract_wav_transcript(data)
    } else {
        None
    }
}

/// Extract transcript from OGG Vorbis comments
fn extract_ogg_transcript(data: &[u8]) -> Option<String> {
    use lewton::inside_ogg::OggStreamReader;
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let reader = OggStreamReader::new(cursor).ok()?;

    // Look for TRANSCRIPT comment in Vorbis comments
    for (key, value) in reader.comment_hdr.comment_list.iter() {
        if key.eq_ignore_ascii_case("TRANSCRIPT") {
            return Some(value.clone());
        }
    }
    None
}

/// Extract transcript from WAV 'trns' chunk
fn extract_wav_transcript(data: &[u8]) -> Option<String> {
    if data.len() < 44 {
        return None;
    }
    // Find trns chunk
    let mut pos = 12;
    while pos + 8 < data.len() {
        let chunk_id = &data[pos..pos+4];
        let chunk_size = u32::from_le_bytes(data[pos+4..pos+8].try_into().ok()?) as usize;

        if chunk_id == b"trns" {
            let text_end = pos + 8 + chunk_size;
            if text_end <= data.len() {
                return String::from_utf8(data[pos+8..text_end].to_vec()).ok();
            }
        }
        pos += 8 + chunk_size;
        if chunk_size % 2 == 1 {
            pos += 1; // Pad to even
        }
    }
    None
}

/// Play audio data using rodio with stop signal support
fn play_audio(data: &[u8], _mime: &str, vertex_id: u64, state: &AudioPlaybackState) {
    // Stop any currently playing audio
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = true;
    }
    // Small delay to let the previous thread notice the stop signal
    thread::sleep(Duration::from_millis(50));

    // Reset signal for new playback
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = false;
    }
    if let Ok(mut playing) = state.playing_vertex.lock() {
        *playing = Some(vertex_id);
    }

    let data_vec = data.to_vec();
    let stop_signal = state.should_stop.clone();
    let playing_vertex = state.playing_vertex.clone();

    thread::spawn(move || {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;

        match OutputStream::try_default() {
            Ok((_stream, handle)) => {
                match Sink::try_new(&handle) {
                    Ok(sink) => {
                        let cursor = Cursor::new(data_vec);
                        match Decoder::new(cursor) {
                            Ok(source) => {
                                sink.append(source);
                                // Poll for stop signal - when signaled, drop sink to stop audio
                                while !sink.empty() {
                                    if let Ok(stop) = stop_signal.lock() {
                                        if *stop {
                                            // Drop sink and stream to stop audio immediately
                                            drop(sink);
                                            return;
                                        }
                                    }
                                    thread::sleep(Duration::from_millis(50));
                                }
                            }
                            Err(e) => eprintln!("Failed to decode audio: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to create audio sink: {}", e),
                }
            }
            Err(e) => eprintln!("Failed to get audio output: {}", e),
        }

        // Clear playing state when done
        if let Ok(mut playing) = playing_vertex.lock() {
            *playing = None;
        }
    });
}

/// Stop any currently playing audio
fn stop_audio(state: &AudioPlaybackState) {
    if let Ok(mut stop) = state.should_stop.lock() {
        *stop = true;
    }
}

/// Generate waveform data from audio for visualization
/// Returns a vector of normalized amplitude values (0.0 to 1.0) for display
fn generate_waveform(data: &[u8], num_bars: usize) -> Option<Vec<f32>> {
    use rodio::Decoder;
    use std::io::Cursor;

    let cursor = Cursor::new(data.to_vec());
    let decoder = Decoder::new(cursor).ok()?;

    // Collect all samples
    let samples: Vec<f32> = decoder
        .map(|s| (s as f32 / i16::MAX as f32).abs())
        .collect();

    if samples.is_empty() {
        return None;
    }

    // Downsample to num_bars by taking max amplitude in each chunk
    let chunk_size = samples.len() / num_bars;
    if chunk_size == 0 {
        return Some(samples);
    }

    let waveform: Vec<f32> = samples
        .chunks(chunk_size)
        .take(num_bars)
        .map(|chunk| {
            chunk.iter().cloned().fold(0.0f32, |a, b| a.max(b))
        })
        .collect();

    Some(waveform)
}

/// Get or generate waveform for an audio vertex
fn get_or_generate_waveform<'a>(
    vertex_id: u64,
    data: &[u8],
    media_cache: &'a mut MediaCache,
    num_bars: usize,
) -> Option<&'a Vec<f32>> {
    if !media_cache.waveforms.contains_key(&vertex_id) {
        if let Some(waveform) = generate_waveform(data, num_bars) {
            media_cache.waveforms.insert(vertex_id, waveform);
        }
    }
    media_cache.waveforms.get(&vertex_id)
}

/// Draw a waveform in a given rect
fn draw_waveform(
    painter: &egui::Painter,
    rect: egui::Rect,
    waveform: &[f32],
    color: egui::Color32,
    bg_color: egui::Color32,
) {
    if waveform.is_empty() {
        return;
    }

    // Draw background
    painter.rect_filled(rect, 2.0, bg_color);

    let bar_width = rect.width() / waveform.len() as f32;
    let center_y = rect.center().y;
    let half_height = rect.height() * 0.4; // Leave some margin

    for (i, &amplitude) in waveform.iter().enumerate() {
        let x = rect.left() + i as f32 * bar_width;
        let bar_height = amplitude * half_height;

        // Draw bar from center up and down (mirrored waveform)
        let bar_rect = egui::Rect::from_min_max(
            egui::pos2(x, center_y - bar_height),
            egui::pos2(x + bar_width * 0.8, center_y + bar_height),
        );
        painter.rect_filled(bar_rect, 1.0, color);
    }
}

fn run_audio_recording(samples: Arc<Mutex<Vec<f32>>>, stop_signal: Arc<Mutex<bool>>, sample_rate_out: Arc<Mutex<u32>>) -> Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or_else(|| anyhow!("No audio input device"))?;

    let config = device.default_input_config()
        .context("Failed to get default input config")?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;
    eprintln!("Audio: device={:?} rate={} channels={}", device.name(), sample_rate, channels);

    // Communicate actual sample rate back
    if let Ok(mut sr) = sample_rate_out.lock() {
        *sr = sample_rate;
    }

    let samples_clone = samples.clone();
    let err_fn = |err| eprintln!("Audio stream error: {}", err);

    // Helper to convert to mono by averaging channels
    let to_mono = move |data: &[f32], s: &mut Vec<f32>| {
        if channels == 1 {
            s.extend_from_slice(data);
        } else {
            // Average channels to mono
            for chunk in data.chunks(channels) {
                let sum: f32 = chunk.iter().sum();
                s.push(sum / channels as f32);
            }
        }
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        to_mono(data, &mut s);
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let samples_clone = samples.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        let float_data: Vec<f32> = data.iter().map(|&x| x as f32 / 32768.0).collect();
                        if channels == 1 {
                            s.extend(float_data);
                        } else {
                            for chunk in float_data.chunks(channels) {
                                let sum: f32 = chunk.iter().sum();
                                s.push(sum / channels as f32);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let samples_clone = samples.clone();
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut s) = samples_clone.lock() {
                        let float_data: Vec<f32> = data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0).collect();
                        if channels == 1 {
                            s.extend(float_data);
                        } else {
                            for chunk in float_data.chunks(channels) {
                                let sum: f32 = chunk.iter().sum();
                                s.push(sum / channels as f32);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => return Err(anyhow!("Unsupported sample format")),
    };

    stream.play()?;

    // Wait until stop signal
    loop {
        thread::sleep(Duration::from_millis(50));
        if let Ok(stop) = stop_signal.lock() {
            if *stop {
                break;
            }
        }
    }

    // Stream is dropped when function returns, stopping recording
    Ok(())
}

/// Encode audio samples to OGG Vorbis format
/// Returns the OGG data with embedded transcript in Vorbis comments
fn encode_ogg_vorbis(samples: &[f32], sample_rate: u32, transcript: Option<&str>) -> Result<Vec<u8>> {
    use vorbis_rs::VorbisEncoderBuilder;

    // Create output buffer - use a wrapper that owns the Vec
    let mut output: Vec<u8> = Vec::new();

    // Create encoder that writes to output
    let mut builder = VorbisEncoderBuilder::new(
        std::num::NonZeroU32::new(sample_rate).ok_or_else(|| anyhow!("Invalid sample rate"))?,
        std::num::NonZeroU8::new(1).unwrap(), // mono
        &mut output,
    )
    .context("Failed to create Vorbis encoder builder")?;

    // Add comments
    builder.comment_tag("ENCODER", "Gradesta Browser")
        .context("Failed to add encoder comment")?;
    if let Some(text) = transcript {
        builder.comment_tag("TRANSCRIPT", text)
            .context("Failed to add transcript comment")?;
    }

    let mut encoder = builder.build()
        .context("Failed to build Vorbis encoder")?;

    // Encode audio in chunks
    let chunk_size = 4096;
    for chunk in samples.chunks(chunk_size) {
        // vorbis_rs expects &[impl AsRef<[f32]>] for channels
        encoder.encode_audio_block([chunk])
            .context("Failed to encode audio block")?;
    }

    // Finish encoding - this flushes all data
    encoder.finish().context("Failed to finish encoding")?;

    eprintln!("Encoded OGG: {} samples -> {} bytes", samples.len(), output.len());

    if output.len() < 100 {
        return Err(anyhow!("OGG encoding produced suspiciously small output: {} bytes", output.len()));
    }

    Ok(output)
}

fn run_ws(uri: String, net_tx: Sender<ServerEvent>, cmd_rx: Receiver<WsCommand>) -> Result<()> {
    let url = Url::parse(&uri).context("Invalid URL")?;
    let host = url.host_str().ok_or_else(|| anyhow!("No host in URL"))?;
    let port = url.port_or_known_default().ok_or_else(|| anyhow!("No port"))?;
    
    eprintln!("Connecting to {}:{}", host, port);
    let stream = TcpStream::connect((host, port)).context("Failed to connect")?;
    let (mut socket, _) = client(url.clone(), stream).context("WebSocket handshake failed")?;
    eprintln!("Connected!");
    
    let base_url = format!("{}://{}:{}{}", url.scheme(), host, port, url.path());
    let _ = net_tx.send(ServerEvent::Connected { base_url });
    
    let landmark = url.query_pairs()
        .find(|(k, _)| k == "landmark")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| url.path().to_string());
    
    eprintln!("SEND WatchLandmark action=0 uri={:?}", landmark);
    let mut buf = Vec::with_capacity(1 + 8 + landmark.len());
    buf.push(MSG_CLIENT_WATCH_LANDMARK);
    buf.extend_from_slice(&0u64.to_be_bytes());
    buf.extend_from_slice(landmark.as_bytes());
    socket.send(Message::Binary(buf))?;
    
    socket.get_mut().set_nonblocking(true)?;
    
    loop {
        // Check for commands to send
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                WsCommand::WatchLandmark { action_id, landmark } => {
                    eprintln!("SEND WatchLandmark action={} uri={:?}", action_id, landmark);
                    let mut buf = Vec::with_capacity(1 + 8 + landmark.len());
                    buf.push(MSG_CLIENT_WATCH_LANDMARK);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(landmark.as_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::ClickVertex { action_id, vertex_id } => {
                    eprintln!("SEND ClickVertex action={} vertex={}", action_id, vertex_id);
                    // 0x84: Type (1) + Action ID (8) + Vertex ID (8)
                    let mut buf = Vec::with_capacity(1 + 8 + 8);
                    buf.push(MSG_CLIENT_CLICK_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::IdentificationResponse { action_id, identity_url, signature } => {
                    eprintln!("SEND IdentificationResponse action={} url={:?}", action_id, identity_url);
                    // Type (1) + Action ID (8) + Identity URL (null-terminated) + Signature (64)
                    let mut buf = Vec::with_capacity(1 + 8 + identity_url.len() + 1 + 64);
                    buf.push(MSG_CLIENT_IDENTIFICATION_RESPONSE);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(identity_url.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&signature);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::IdentificationRefused { action_id } => {
                    eprintln!("SEND IdentificationRefused action={}", action_id);
                    let mut buf = Vec::with_capacity(1 + 8);
                    buf.push(MSG_CLIENT_IDENTIFICATION_REFUSED);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::SetVertexLabel { action_id, vertex_id, layer, mime, data } => {
                    // 0x85: Type (1) + Action ID (8) + Vertex ID (8) + Layer (4) + MIME (null-terminated) + Data
                    eprintln!("SEND SetVertexLabel action={} vertex={} layer={} mime={:?} len={}", action_id, vertex_id, layer, mime, data.len());
                    let mut buf = Vec::with_capacity(1 + 8 + 8 + 4 + mime.len() + 1 + data.len());
                    buf.push(MSG_CLIENT_SET_VERTEX_LABEL);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    buf.extend_from_slice(&layer.to_be_bytes());
                    buf.extend_from_slice(mime.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&data);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::CreateVertex { action_id, from_vertex, direction, layer, mime, data } => {
                    // 0x86: Type (1) + Action ID (8) + From Vertex (8) + Direction (1) + Layer (4) + MIME (null-terminated) + Data
                    eprintln!("SEND CreateVertex action={} from={} dir={} layer={} mime={:?} len={}", action_id, from_vertex, direction, layer, mime, data.len());
                    let mut buf = Vec::with_capacity(1 + 8 + 8 + 1 + 4 + mime.len() + 1 + data.len());
                    buf.push(MSG_CLIENT_CREATE_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&from_vertex.to_be_bytes());
                    buf.push(direction);
                    buf.extend_from_slice(&layer.to_be_bytes());
                    buf.extend_from_slice(mime.as_bytes());
                    buf.push(0); // null terminator
                    buf.extend_from_slice(&data);
                    socket.send(Message::Binary(buf))?;
                }
                WsCommand::DeleteVertex { action_id, vertex_id } => {
                    // 0x87: Type (1) + Action ID (8) + Vertex ID (8)
                    eprintln!("SEND DeleteVertex action={} vertex={}", action_id, vertex_id);
                    let mut buf = Vec::with_capacity(1 + 8 + 8);
                    buf.push(MSG_CLIENT_DELETE_VERTEX);
                    buf.extend_from_slice(&action_id.to_be_bytes());
                    buf.extend_from_slice(&vertex_id.to_be_bytes());
                    socket.send(Message::Binary(buf))?;
                }
            }
        }
        
        // Read incoming messages
        match socket.read() {
            Ok(Message::Binary(data)) => {
                if let Ok(event) = parse_server_message(&data) {
                    let _ = net_tx.send(event);
                }
            }
            Ok(Message::Ping(data)) => {
                // Respond to ping with pong (standard WebSocket heartbeat)
                let _ = socket.send(Message::Pong(data));
            }
            Ok(Message::Close(frame)) => {
                let reason = frame
                    .map(|f| f.reason.to_string())
                    .unwrap_or_else(|| "Connection closed".to_string());
                let _ = net_tx.send(ServerEvent::Disconnected { reason });
                return Ok(());
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let _ = net_tx.send(ServerEvent::Disconnected {
                    reason: format!("Connection error: {}", e)
                });
                return Err(e.into());
            }
        }
    }
}

fn parse_server_message(data: &[u8]) -> Result<ServerEvent> {
    if data.is_empty() {
        return Err(anyhow!("empty"));
    }
    match data[0] {
        MSG_SERVER_SET_CONTEXT => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let uri = String::from_utf8(rest.to_vec())?;
            eprintln!("RECV SetContext action={} uri={:?}", action_id, uri);
            Ok(ServerEvent::SetContext { uri })
        }
        MSG_SERVER_SET_VERTEX_LABEL => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (vertex_id, rest) = read_u64(rest)?;
            let (layer, rest) = read_u32(rest)?;
            let (mime, label) = read_null_terminated(rest)?;
            let label_preview: String = String::from_utf8_lossy(label).chars().take(40).collect();
            eprintln!("RECV SetVertexLabel action={} vertex={} layer={} mime={:?} label={:?}... ({} bytes)",
                action_id, vertex_id, layer, mime, label_preview, label.len());
            Ok(ServerEvent::SetVertexLabel { vertex_id, layer, mime, data: label.to_vec() })
        }
        MSG_SERVER_SET_EDGES => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (vertex_id, rest) = read_u64(rest)?;
            let mut edges = [0u64; 6];
            let mut slice = rest;
            for edge in edges.iter_mut() {
                let (v, next) = read_u64(slice)?;
                *edge = v;
                slice = next;
            }
            // Read edit_mask (1 byte) if present, default to 0x7F (editable)
            let edit_mask = if !slice.is_empty() { slice[0] } else { 0x7F };
            eprintln!("RECV SetEdges action={} vertex={} W={} E={} N={} S={} U={} D={} edit_mask=0x{:02x}",
                action_id, vertex_id, edges[0], edges[1], edges[2], edges[3], edges[4], edges[5], edit_mask);
            Ok(ServerEvent::SetEdges { vertex_id, edges, edit_mask })
        }
        MSG_SERVER_LOG => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let (status, rest) = read_u32(rest)?;
            let (vertex_id, rest) = read_u64(rest)?;
            let message = String::from_utf8(rest.to_vec())?;
            eprintln!("RECV Log action={} status={} vertex={} msg={:?}", action_id, status, vertex_id, message);
            Ok(ServerEvent::Log { action_id, status, vertex_id, message })
        }
        MSG_SERVER_REQUEST_IDENTIFICATION => {
            // Type (1) + Action ID (8) + Nonce (32) + Timestamp (8) + Reason (string)
            if data.len() < 1 + 8 + 32 + 8 {
                return Err(anyhow!("Request identification message too short"));
            }
            let (action_id, rest) = read_u64(&data[1..])?;
            let mut nonce = [0u8; 32];
            nonce.copy_from_slice(&rest[..32]);
            let (timestamp, rest) = read_u64(&rest[32..])?;
            let reason = String::from_utf8(rest.to_vec()).unwrap_or_else(|_| "Unknown".to_string());
            eprintln!("RECV RequestIdentification action={} nonce={:?}... timestamp={} reason={:?}",
                action_id, &nonce[..8], timestamp, reason);
            Ok(ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason })
        }
        other => {
            eprintln!("RECV Unknown message type: 0x{:02x}", other);
            Err(anyhow!("unknown message type"))
        }
    }
}

fn read_u64(data: &[u8]) -> Result<(u64, &[u8])> {
    if data.len() < 8 {
        return Err(anyhow!("not enough bytes"));
    }
    let value = u64::from_be_bytes(data[..8].try_into()?);
    Ok((value, &data[8..]))
}

fn read_u32(data: &[u8]) -> Result<(u32, &[u8])> {
    if data.len() < 4 {
        return Err(anyhow!("not enough bytes"));
    }
    let value = u32::from_be_bytes(data[..4].try_into()?);
    Ok((value, &data[4..]))
}

fn read_null_terminated(data: &[u8]) -> Result<(String, &[u8])> {
    let pos = data.iter().position(|b| *b == 0).ok_or_else(|| anyhow!("no null"))?;
    let s = String::from_utf8(data[..pos].to_vec())?;
    Ok((s, &data[pos + 1..]))
}

fn ingest_server_events(
    mut graph: ResMut<GraphState>,
    mut app_state: ResMut<AppState>,
    rx: Res<NetRx>,
    net_tx: Res<NetEventsTx>,
    mut ws_cmd_tx: ResMut<WsCommandTx>,
    mut media_cache: ResMut<MediaCache>,
) {
    while let Ok(event) = rx.0.try_recv() {
        match event {
            ServerEvent::Connected { base_url } => {
                app_state.connected = true;
                app_state.base_ws_url = Some(base_url);
                app_state.status = "Connected!".to_string();
            }
            ServerEvent::Disconnected { reason } => {
                app_state.connected = false;
                app_state.status = format!("Disconnected: {reason}");
                ws_cmd_tx.0 = None;
            }
            ServerEvent::Error { message } => {
                app_state.status = format!("Error: {message}");
            }
            ServerEvent::SetContext { uri } => {
                // Track which landmark we're receiving data for
                graph.current_receiving_landmark = Some(uri.clone());

                // Initialize the vertex list for this landmark if not already present
                graph.landmark_vertices.entry(uri.clone()).or_insert_with(Vec::new);

                // Check if we're following a portal to this context
                let is_following = app_state.following_portal.as_ref()
                    .map(|p| p == &uri)
                    .unwrap_or(false);

                if is_following {
                    // We're arriving at a followed portal - clear it and mark that
                    // we should jump to the first vertex of this context
                    app_state.following_portal = None;
                    graph.pending_jump_context = Some(uri.clone());
                }

                graph.context_uri = Some(uri.clone());
                app_state.status = format!("Viewing: {uri}");
                // Update the URL bar to show current landmark
                if let Some(base) = &app_state.base_ws_url {
                    app_state.url_input = format!("{}?landmark={}", base, uri);
                }
            }
            ServerEvent::SetVertexLabel { vertex_id, layer, mime, data } => {
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;

                // Special handling for layer 3 HTTP stream URLs
                if layer == 3 && mime == "text/x-http-stream-url" {
                    // Parse the content: expected-mime\nurl
                    if let Ok(content) = String::from_utf8(data.clone()) {
                        if let Some((expected_mime, url)) = content.split_once('\n') {
                            let url = url.trim().to_string();
                            let expected_mime = expected_mime.trim().to_string();
                            let v_id = vertex_id;
                            let events_tx = net_tx.0.clone();

                            // Spawn background thread to fetch content via HTTP
                            thread::spawn(move || {
                                eprintln!("HTTP Fetch: Fetching {} from {}", expected_mime, url);
                                match reqwest::blocking::get(&url) {
                                    Ok(response) => {
                                        if response.status().is_success() {
                                            match response.bytes() {
                                                Ok(bytes) => {
                                                    eprintln!("HTTP Fetch: Got {} bytes for vertex {}", bytes.len(), v_id);
                                                    let _ = events_tx.send(ServerEvent::HttpStreamContentFetched {
                                                        vertex_id: v_id,
                                                        mime: expected_mime,
                                                        data: bytes.to_vec(),
                                                    });
                                                }
                                                Err(e) => eprintln!("HTTP Fetch: Failed to read body: {}", e),
                                            }
                                        } else {
                                            eprintln!("HTTP Fetch: HTTP error: {}", response.status());
                                        }
                                    }
                                    Err(e) => eprintln!("HTTP Fetch: Failed to fetch: {}", e),
                                }
                            });
                        }
                    }
                    // Still store the layer 3 content for reference
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
                        data,
                    });
                    return;
                }

                // Invalidate cached texture/media when content changes
                // This ensures updated images (e.g., full content replacing thumbnail) are re-loaded
                if mime.starts_with("image/") || crate::is_image_data(&data) {
                    media_cache.textures.remove(&vertex_id);
                    media_cache.animated_gifs.remove(&vertex_id);
                }

                // Store in appropriate layer
                if layer == 0 {
                    // Primary layer - backwards compatible
                    entry.label = data;
                    entry.mime = Some(mime.clone());
                } else {
                    // Secondary layers
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
                        data,
                    });
                }

                // Track which landmark this vertex belongs to
                if let Some(landmark) = graph.current_receiving_landmark.clone() {
                    if let Some(vertices) = graph.landmark_vertices.get_mut(&landmark) {
                        if !vertices.contains(&vertex_id) {
                            vertices.push(vertex_id);
                        }
                    }
                }

                // If we're waiting to jump to a new context, and this vertex is NOT a portal,
                // jump to it (skip portals since they're just links)
                if layer == 0 && graph.pending_jump_context.is_some() && mime != "text/gradesta-url" {
                    if let Some(current) = app_state.current_vertex {
                        app_state.history.push(current);
                    }
                    app_state.current_vertex = Some(vertex_id);
                    graph.pending_jump_context = None;
                } else if layer == 0 && app_state.current_vertex.is_none() {
                    app_state.current_vertex = Some(vertex_id);
                }
            }
            ServerEvent::SetEdges { vertex_id, edges, edit_mask } => {
                // Check if this is a deletion signal (all edges = 0, edit_mask = 0)
                // Only treat as deletion if the vertex already exists (to avoid false positives
                // from read-only vertices like calendar entries that have no edges)
                let vertex_exists = graph.vertices.contains_key(&vertex_id);
                let is_deletion = vertex_exists && edges.iter().all(|&e| e == 0) && edit_mask == 0;

                if is_deletion {
                    // Remove the vertex from the graph
                    graph.vertices.remove(&vertex_id);
                    // Remove from history
                    app_state.history.retain(|&id| id != vertex_id);
                    // Remove from landmark_vertices
                    for vertices in graph.landmark_vertices.values_mut() {
                        vertices.retain(|&id| id != vertex_id);
                    }
                    // If we're currently on this deleted vertex, navigate away
                    if app_state.current_vertex == Some(vertex_id) {
                        app_state.current_vertex = app_state.history.pop();
                    }
                    eprintln!("Vertex {} deleted from local graph", vertex_id);
                } else {
                    let entry = graph.vertices.entry(vertex_id).or_default();
                    entry.id = vertex_id;
                    entry.edit_mask = edit_mask;
                    // Merge edges: u64::MAX means "unchanged", skip those
                    for (i, &new_edge) in edges.iter().enumerate() {
                        if new_edge != u64::MAX {
                            entry.edges[i] = new_edge;
                        }
                    }

                    if app_state.current_vertex.is_none() {
                        app_state.current_vertex = Some(vertex_id);
                    }
                }
            }
            ServerEvent::Log { action_id, status, vertex_id, message } => {
                app_state.status = format!("Server [{}]: {}", status, message);
                // Handle edit acknowledgments
                if status == 200 {
                    eprintln!("Edit acknowledged: action={} vertex={} status={}", action_id, vertex_id, status);

                    // Check for pending vertex creation for this action_id
                    // Only navigate to the vertex if this was a CREATE (not an update like transcription)
                    if let Some(pending) = app_state.pending_creations.remove(&action_id) {
                        // Navigate to newly created vertex
                        if vertex_id != 0 {
                            if let Some(current) = app_state.current_vertex {
                                if current != vertex_id {
                                    app_state.history.push(current);
                                    app_state.current_vertex = Some(vertex_id);
                                    eprintln!("Navigating to newly created vertex {}", vertex_id);
                                }
                            } else {
                                app_state.current_vertex = Some(vertex_id);
                            }
                        }

                        // Populate the vertex with the data we sent (server doesn't echo it back)
                        let entry = graph.vertices.entry(vertex_id).or_default();
                        entry.id = vertex_id;
                        entry.label = pending.data.clone();
                        entry.mime = Some(pending.mime.clone());

                        // For audio, start transcription and skip auto-play
                        if pending.mime.starts_with("audio/") {
                            // Don't auto-play the audio we just recorded
                            app_state.skip_autoplay_vertex = Some(vertex_id);

                            eprintln!("Starting async transcription for vertex {} (action={})", vertex_id, action_id);
                            let event_tx = net_tx.0.clone();
                            let target_vertex = vertex_id;
                            // Allocate a new action_id for the SetVertexLabel
                            let transcript_action_id = app_state.next_action_id;
                            app_state.next_action_id = app_state.next_action_id.wrapping_sub(1);

                            // Spawn transcription in background
                            thread::spawn(move || {
                                if whisper::is_model_available() {
                                    match whisper::transcribe(&pending.samples, pending.sample_rate) {
                                        Ok(text) => {
                                            eprintln!("Transcription complete: {}", text);
                                            // Send transcript back to main thread to store locally and send to server
                                            let _ = event_tx.send(ServerEvent::LocalSetVertexLabel {
                                                action_id: transcript_action_id,
                                                vertex_id: target_vertex,
                                                layer: 1, // Convention: layer 1 for transcript
                                                mime: "text/plain".to_string(),
                                                data: text.into_bytes(),
                                            });
                                        }
                                        Err(e) => {
                                            eprintln!("Transcription failed: {}", e);
                                        }
                                    }
                                } else {
                                    eprintln!("Whisper model not available, skipping transcription");
                                }
                            });
                        }
                    }
                } else {
                    eprintln!("Edit failed: action={} vertex={} status={} msg={}", action_id, vertex_id, status, message);
                    // Remove any pending creation for failed actions
                    app_state.pending_creations.remove(&action_id);
                }
            }
            ServerEvent::LocalSetVertexLabel { action_id, vertex_id, layer, mime, data } => {
                // Store locally
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;
                if layer == 0 {
                    entry.label = data.clone();
                    entry.mime = Some(mime.clone());
                } else {
                    entry.layers.insert(layer, LayerContent {
                        mime: mime.clone(),
                        data: data.clone(),
                    });
                }

                // Send to server
                if let Some(ref tx) = ws_cmd_tx.0 {
                    let _ = tx.send(WsCommand::SetVertexLabel {
                        action_id,
                        vertex_id,
                        layer,
                        mime,
                        data,
                    });
                }
            }
            ServerEvent::HttpStreamContentFetched { vertex_id, mime, data } => {
                // For MP4 videos, use native video player
                if mime == "video/mp4" {
                    eprintln!("HTTP Fetch: Got MP4 video ({} bytes), starting native player", data.len());

                    match VideoPlayer::new(data.clone()) {
                        Ok(player) => {
                            media_cache.video_players.insert(vertex_id, player);
                            app_state.video_modal_vertex_id = Some(vertex_id);
                            app_state.show_video_modal = true;
                            eprintln!("HTTP Fetch: Video player started for vertex {}", vertex_id);
                        }
                        Err(e) => {
                            eprintln!("HTTP Fetch: Failed to create video player: {}", e);
                            app_state.status = format!("Video error: {}", e);
                        }
                    }
                    return;
                }

                // For other video formats, fall back to external player
                if mime.starts_with("video/") {
                    eprintln!("HTTP Fetch: Got video {} ({} bytes), launching external player", mime, data.len());

                    // Determine file extension from mime type
                    let ext = match mime.as_str() {
                        "video/webm" => "webm",
                        "video/quicktime" => "mov",
                        "video/x-matroska" => "mkv",
                        "video/ogg" => "ogv",
                        _ => "mp4",
                    };

                    // Write to temp file and launch mpv
                    match tempfile::Builder::new()
                        .prefix("gradesta-video-")
                        .suffix(&format!(".{}", ext))
                        .tempfile()
                    {
                        Ok(mut temp) => {
                            use std::io::Write;
                            if let Err(e) = temp.write_all(&data) {
                                eprintln!("HTTP Fetch: Failed to write temp file: {}", e);
                            } else {
                                let path = temp.path().to_owned();
                                // Keep the temp file around while mpv plays
                                let (file, file_path) = temp.keep().unwrap_or_else(|e| {
                                    eprintln!("Failed to keep temp file: {}", e);
                                    (std::fs::File::create(&path).unwrap(), path.clone())
                                });
                                drop(file); // Close file handle before opening

                                eprintln!("HTTP Fetch: Launching mpv for {}", file_path.display());
                                if let Err(e) = Command::new("mpv")
                                    .arg(&file_path)
                                    .spawn()
                                {
                                    eprintln!("HTTP Fetch: Failed to launch mpv: {}", e);
                                    // Fall back to xdg-open
                                    if let Err(e2) = open::that(&file_path) {
                                        eprintln!("HTTP Fetch: Failed to open with xdg-open: {}", e2);
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("HTTP Fetch: Failed to create temp file: {}", e),
                    }
                    return;
                }

                // Store fetched content in layer 2 (where full content normally goes)
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;

                // Invalidate cached texture/media when content changes
                if mime.starts_with("image/") || crate::is_image_data(&data) {
                    media_cache.textures.remove(&vertex_id);
                    media_cache.animated_gifs.remove(&vertex_id);
                }

                entry.layers.insert(2, LayerContent {
                    mime: mime.clone(),
                    data,
                });

                eprintln!("HTTP Fetch: Stored {} content in layer 2 for vertex {}", mime, vertex_id);
            }
            ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason } => {
                // Get server URL from current connection
                let server_url = app_state.base_ws_url.clone().unwrap_or_default();

                // Check if this server is remembered for any identity
                let remembered_identity = app_state.identity_config.identities.iter()
                    .position(|id| id.remembered_servers.contains(&server_url));

                if let Some(idx) = remembered_identity {
                    // Auto-identify with remembered identity
                    app_state.selected_identity_index = idx;
                    app_state.pending_identification = Some(PendingIdentification {
                        action_id,
                        nonce,
                        timestamp,
                        reason: reason.clone(),
                        server_url,
                    });
                    app_state.status = format!("Auto-identifying as {}...",
                        app_state.identity_config.identities[idx].display_name);
                } else {
                    // Show consent dialog (works even with no identities - dialog handles that case)
                    app_state.pending_identification = Some(PendingIdentification {
                        action_id,
                        nonce,
                        timestamp,
                        reason,
                        server_url,
                    });
                    app_state.status = "Server requests identification".to_string();
                }
            }
        }
    }
}
