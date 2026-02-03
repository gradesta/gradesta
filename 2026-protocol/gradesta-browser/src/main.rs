use anyhow::{anyhow, Context, Result};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use crossbeam_channel::{unbounded, Receiver, Sender};
use gif::DecodeOptions;
use std::collections::{HashMap, HashSet};
use std::io::{self, Cursor, Write};
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::{client, Message};
use url::Url;

mod identity;
use identity::{Identity, IdentityConfig};

const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
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

#[derive(Clone, Debug, Default)]
struct Vertex {
    id: u64,
    label: Vec<u8>,
    mime: Option<String>,
    edges: [u64; 6],
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
    WatchLandmark(String),
    IdentificationResponse {
        action_id: u64,
        identity_url: String,
        signature: Vec<u8>,
    },
    IdentificationRefused {
        action_id: u64,
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
    nextcloud_url_input: String,
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
            nextcloud_url_input: "https://".to_string(),
        }
    }
}

const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 4.0;
const ZOOM_STEP: f32 = 0.1;

#[derive(Resource, Default)]
struct MediaCache {
    textures: HashMap<u64, egui::TextureHandle>,
    animated_gifs: HashMap<u64, AnimatedGif>,
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
    SetVertexLabel { vertex_id: u64, mime: String, data: Vec<u8> },
    SetEdges { vertex_id: u64, edges: [u64; 6] },
    Log { message: String },
    Connected { base_url: String },
    Error { message: String },
    RequestIdentification {
        action_id: u64,
        nonce: [u8; 32],
        timestamp: u64,
        reason: String,
    },
}

#[derive(Default, Clone)]
struct GridView {
    cells: HashMap<(i32, i32), u64>,
    positions: HashMap<u64, (i32, i32)>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

fn main() {
    let (net_tx, net_rx) = unbounded::<ServerEvent>();

    App::new()
        .insert_resource(NetRx(net_rx))
        .insert_resource(NetEventsTx(net_tx))
        .insert_resource(WsCommandTx(None))
        .insert_resource(GraphState::default())
        .insert_resource(AppState::default())
        .insert_resource(MediaCache::default())
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
            auto_expand_nearby_links,
        ))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn build_grid_view(graph: &GraphState, current_id: u64) -> GridView {
    let mut grid = GridView::default();
    let mut visited = HashSet::new();
    
    let mut top_id = current_id;
    while let Some(v) = graph.vertices.get(&top_id) {
        if v.edges[EDGE_NORTH] != 0 && graph.vertices.contains_key(&v.edges[EDGE_NORTH]) {
            top_id = v.edges[EDGE_NORTH];
        } else {
            break;
        }
    }
    
    build_column(&mut grid, graph, top_id, 0, &mut visited);
    
    let center_vertices: Vec<(i32, u64)> = grid.cells.iter()
        .filter(|((x, _), _)| *x == 0)
        .map(|((_, y), id)| (*y, *id))
        .collect();
    
    for (y, id) in center_vertices {
        if let Some(v) = graph.vertices.get(&id) {
            if v.edges[EDGE_WEST] != 0 && !visited.contains(&v.edges[EDGE_WEST]) {
                expand_column_recursive(&mut grid, graph, v.edges[EDGE_WEST], -1, y, &mut visited);
            }
            if v.edges[EDGE_EAST] != 0 && !visited.contains(&v.edges[EDGE_EAST]) {
                expand_column_recursive(&mut grid, graph, v.edges[EDGE_EAST], 1, y, &mut visited);
            }
        }
    }
    
    grid
}

fn build_column(grid: &mut GridView, graph: &GraphState, top_id: u64, x: i32, visited: &mut HashSet<u64>) {
    let mut y = 0i32;
    let mut current = top_id;
    
    while graph.vertices.contains_key(&current) && !visited.contains(&current) {
        visited.insert(current);
        grid.cells.insert((x, y), current);
        grid.positions.insert(current, (x, y));
        
        grid.min_x = grid.min_x.min(x);
        grid.max_x = grid.max_x.max(x);
        grid.min_y = grid.min_y.min(y);
        grid.max_y = grid.max_y.max(y);
        
        if let Some(v) = graph.vertices.get(&current) {
            if v.edges[EDGE_SOUTH] != 0 {
                current = v.edges[EDGE_SOUTH];
                y += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn expand_column_recursive(
    grid: &mut GridView,
    graph: &GraphState,
    start_id: u64,
    x: i32,
    start_y: i32,
    visited: &mut HashSet<u64>,
) {
    if visited.contains(&start_id) || !graph.vertices.contains_key(&start_id) {
        return;
    }
    
    let mut top_id = start_id;
    let mut offset_from_start = 0i32;
    while let Some(v) = graph.vertices.get(&top_id) {
        if v.edges[EDGE_NORTH] != 0 && graph.vertices.contains_key(&v.edges[EDGE_NORTH]) && !visited.contains(&v.edges[EDGE_NORTH]) {
            top_id = v.edges[EDGE_NORTH];
            offset_from_start -= 1;
        } else {
            break;
        }
    }
    
    let top_y = start_y + offset_from_start;
    let mut y = top_y;
    let mut current = top_id;
    
    while graph.vertices.contains_key(&current) && !visited.contains(&current) {
        if grid.cells.contains_key(&(x, y)) {
            break;
        }
        
        visited.insert(current);
        grid.cells.insert((x, y), current);
        grid.positions.insert(current, (x, y));
        
        grid.min_x = grid.min_x.min(x);
        grid.max_x = grid.max_x.max(x);
        grid.min_y = grid.min_y.min(y);
        grid.max_y = grid.max_y.max(y);
        
        if let Some(v) = graph.vertices.get(&current) {
            if x < 0 && v.edges[EDGE_WEST] != 0 && !visited.contains(&v.edges[EDGE_WEST]) {
                expand_column_recursive(grid, graph, v.edges[EDGE_WEST], x - 1, y, visited);
            }
            if x > 0 && v.edges[EDGE_EAST] != 0 && !visited.contains(&v.edges[EDGE_EAST]) {
                expand_column_recursive(grid, graph, v.edges[EDGE_EAST], x + 1, y, visited);
            }
            
            if v.edges[EDGE_SOUTH] != 0 {
                current = v.edges[EDGE_SOUTH];
                y += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

fn ui_system(
    mut contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    graph: Res<GraphState>,
    net_events: Res<NetEventsTx>,
    mut ws_cmd_tx: ResMut<WsCommandTx>,
    mut media_cache: ResMut<MediaCache>,
) {
    let ctx = contexts.ctx_mut();

    // Handle modal keyboard shortcuts and zoom
    ctx.input(|i| {
        // Escape to close modals
        if i.key_pressed(egui::Key::Escape) {
            if app_state.show_text_modal {
                app_state.show_text_modal = false;
            }
            if app_state.show_image_modal {
                app_state.show_image_modal = false;
            }
        }
        // Ctrl+Enter to open modal with current content
        if i.key_pressed(egui::Key::Enter) && i.modifiers.ctrl {
            if let Some(current_id) = app_state.current_vertex {
                if let Some(vertex) = graph.vertices.get(&current_id) {
                    let mime = vertex.mime.as_deref().unwrap_or("");
                    let is_image = mime.starts_with("image/") || is_image_data(&vertex.label);
                    let is_text = mime.starts_with("text/") && mime != "text/gradesta-url" && mime != "text/x-url";

                    if is_text {
                        app_state.text_modal_content = String::from_utf8_lossy(&vertex.label).to_string();
                        app_state.show_text_modal = true;
                    } else if is_image {
                        app_state.image_modal_vertex_id = Some(current_id);
                        app_state.show_image_modal = true;
                    }
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
    });

    // Apply zoom by scaling the UI - we do this manually in rendering instead of using pixels_per_point
    // because set_pixels_per_point causes layout issues

    // Top panel with URL bar
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("URL:");
            let text_edit = egui::TextEdit::singleline(&mut app_state.url_input)
                .desired_width(600.0)
                .hint_text("ws://localhost:8080/ws?landmark=/home/");
            let response = ui.add(text_edit);
            
            let connect_clicked = ui.button("Connect").clicked();
            let enter_pressed = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            
            if (connect_clicked || enter_pressed) && !app_state.connected {
                let url = app_state.url_input.trim().to_string();
                if url.is_empty() {
                    app_state.status = "URL is empty!".to_string();
                } else {
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
            ui.label("↑↓←→/WASD = Navigate | Ctrl+Enter = View | Ctrl+/- = Zoom | Ctrl+0 = Reset zoom | Esc = Close");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔑 Identities").clicked() {
                    app_state.show_identity_panel = !app_state.show_identity_panel;
                }
                let id_count = app_state.identity_config.identities.len();
                if id_count > 0 {
                    ui.label(format!("{} identity(s)", id_count));
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
                render_vertex_content(ui, vertex, current_id, &mut media_cache, ctx, &mut app_state);
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
                let mime = vertex.mime.as_deref().unwrap_or("");
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
                                    if let Some(animated) = get_or_load_animated_gif(vertex_id, &vertex.label, &mut media_cache, ctx) {
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
                                    if let Some(tex) = get_or_load_texture(vertex_id, &vertex.label, mime, &mut media_cache, ctx) {
                                        let size = tex.size_vec2();
                                        ui.image((tex.id(), size));
                                    }
                                }
                            });
                    });
            }
        }
    }

    // Identity management panel
    if app_state.show_identity_panel {
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
                        ui.horizontal(|ui| {
                            ui.label(format!("• {}", identity.display_name));
                            if ui.small_button("Remove").clicked() {
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = to_remove {
                        app_state.identity_config.identities.remove(i);
                        let _ = app_state.identity_config.save();
                    }
                }

                ui.separator();
                ui.heading("Add Nextcloud Account");

                // Check if login flow is in progress
                if let Some(ref login_state) = app_state.nextcloud_login_state {
                    ui.label(format!("Waiting for login to {}...", login_state.nextcloud_url));
                    ui.label("Please complete the login in your browser.");

                    // Poll for completion
                    if login_state.started.elapsed() > Duration::from_secs(1) {
                        match identity::poll_login_completion(&login_state.poll_endpoint, &login_state.poll_token) {
                            Ok(Some((server, username, app_password))) => {
                                app_state.status = format!("Logged in as {}@{}", username, server);

                                // Setup identity (generate keys, upload, create share)
                                match identity::setup_identity(&server, &username, &app_password) {
                                    Ok((signing_key, share_url)) => {
                                        let identity = Identity {
                                            display_name: format!("{}@{}", username, server.replace("https://", "").replace("http://", "")),
                                            nextcloud_url: server,
                                            username,
                                            app_password,
                                            share_url,
                                            remembered_servers: Vec::new(),
                                            signing_key: Some(signing_key),
                                        };
                                        app_state.identity_config.identities.push(identity);
                                        let _ = app_state.identity_config.save();
                                        app_state.status = "Identity created successfully!".to_string();
                                    }
                                    Err(e) => {
                                        app_state.status = format!("Failed to setup identity: {}", e);
                                    }
                                }
                                app_state.nextcloud_login_state = None;
                            }
                            Ok(None) => {
                                // Still waiting
                            }
                            Err(e) => {
                                app_state.status = format!("Login failed: {}", e);
                                app_state.nextcloud_login_state = None;
                            }
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        app_state.nextcloud_login_state = None;
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Nextcloud URL:");
                        ui.text_edit_singleline(&mut app_state.nextcloud_url_input);
                    });

                    if ui.button("Connect Nextcloud Account").clicked() {
                        let nc_url = app_state.nextcloud_url_input.trim().to_string();
                        if !nc_url.is_empty() {
                            match identity::initiate_nextcloud_login(&nc_url) {
                                Ok((login_url, poll_endpoint, poll_token)) => {
                                    // Open browser for login
                                    if let Err(e) = open::that(&login_url) {
                                        app_state.status = format!("Failed to open browser: {}", e);
                                    } else {
                                        app_state.nextcloud_login_state = Some(NextcloudLoginState {
                                            nextcloud_url: nc_url,
                                            poll_endpoint,
                                            poll_token,
                                            started: Instant::now(),
                                        });
                                        app_state.status = "Opening browser for Nextcloud login...".to_string();
                                    }
                                }
                                Err(e) => {
                                    app_state.status = format!("Failed to initiate login: {}", e);
                                }
                            }
                        }
                    }
                }
            });
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
                        ui.label("Add a Nextcloud account in Identity Management first.");
                        if ui.button("Refuse").clicked() {
                            id_action = Some(IdentificationAction::Refuse);
                        }
                    } else {
                        ui.label("Identify as:");
                        // Collect display names first to avoid borrow conflict
                        let display_names: Vec<String> = app_state.identity_config.identities
                            .iter()
                            .map(|id| id.display_name.clone())
                            .collect();
                        for (i, name) in display_names.iter().enumerate() {
                            ui.radio_value(&mut app_state.selected_identity_index, i, name);
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Identify").clicked() {
                                id_action = Some(IdentificationAction::Identify { remember: false });
                            }
                            if ui.button("Identify + Remember").clicked() {
                                id_action = Some(IdentificationAction::Identify { remember: true });
                            }
                            if ui.button("Refuse").clicked() {
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
                            if let Some(identity) = app_state.identity_config.identities.get_mut(idx) {
                                if !identity.remembered_servers.contains(&pending.server_url) {
                                    identity.remembered_servers.push(pending.server_url.clone());
                                    let _ = app_state.identity_config.save();
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
        let cell_width = 120.0f32 * zoom;
        let cell_height = 100.0f32 * zoom;
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
        for ((x, y), &vertex_id) in &grid.cells {
            if let Some(vertex) = graph.vertices.get(&vertex_id) {
                let from_x = (*x - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
                let from_y = (*y - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;
                let from = base_pos + egui::vec2(from_x, from_y);
                
                if vertex.edges[EDGE_SOUTH] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_SOUTH]) {
                        let to_x = (tx - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
                        let to_y = (ty - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;
                        let to = base_pos + egui::vec2(to_x, to_y);
                        painter.line_segment([from, to], egui::Stroke::new(1.5 * zoom, egui::Color32::from_rgb(70, 70, 80)));
                    }
                }

                if vertex.edges[EDGE_EAST] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_EAST]) {
                        let to_x = (tx - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
                        let to_y = (ty - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;
                        let to = base_pos + egui::vec2(to_x, to_y);
                        painter.line_segment([from, to], egui::Stroke::new(1.5 * zoom, egui::Color32::from_rgb(70, 70, 80)));
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
                        let is_image = mime.starts_with("image/") || is_image_data(&vertex.label);

                        // Different colors based on content type
                        let (bg_color, border_color) = if is_current {
                            (egui::Color32::from_rgb(50, 100, 70), egui::Color32::from_rgb(100, 200, 120))
                        } else if mime == "text/gradesta-url" {
                            (egui::Color32::from_rgb(60, 60, 90), egui::Color32::from_rgb(100, 100, 150))
                        } else if is_image {
                            (egui::Color32::from_rgb(40, 40, 45), egui::Color32::from_rgb(140, 100, 140))
                        } else if mime.starts_with("text/") {
                            (egui::Color32::from_rgb(50, 60, 70), egui::Color32::from_rgb(100, 120, 140))
                        } else if mime.starts_with("video/") || mime.starts_with("audio/") {
                            (egui::Color32::from_rgb(70, 60, 50), egui::Color32::from_rgb(140, 120, 100))
                        } else {
                            (egui::Color32::from_rgb(50, 50, 55), egui::Color32::from_rgb(80, 80, 90))
                        };

                        let corner_radius = 4.0 * zoom;
                        painter.rect_filled(rect, corner_radius, bg_color);
                        painter.rect_stroke(rect, corner_radius, egui::Stroke::new(if is_current { 3.0 * zoom } else { 2.0 * zoom }, border_color));

                        // For images, render the image in the cell
                        if is_image {
                            if let Some(tex) = get_or_load_texture(vertex_id, &vertex.label, mime, &mut media_cache, ctx) {
                                let tex_size = tex.size_vec2();
                                // Scale to fit in cell with some padding
                                let inner_rect = rect.shrink(4.0 * zoom);
                                let scale = (inner_rect.width() / tex_size.x).min(inner_rect.height() / tex_size.y);
                                let scaled_size = tex_size * scale;
                                let img_rect = egui::Rect::from_center_size(rect.center(), scaled_size);
                                painter.image(tex.id(), img_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                        } else {
                            // For non-images, show icon and label
                            let label = String::from_utf8_lossy(&vertex.label);
                            // Calculate max chars based on cell width (roughly 7px per char at font_size 13)
                            let char_width = font_size * 0.55;
                            let available_width = cell_width - 8.0 * zoom; // padding
                            let max_chars = ((available_width / char_width) as usize).saturating_sub(3); // reserve for icon + ellipsis
                            let max_chars = max_chars.max(5); // at least show something
                            let display_label: String = label.chars().take(max_chars).collect();
                            let display_label = if label.chars().count() > max_chars {
                                format!("{}…", display_label)
                            } else {
                                display_label
                            };

                            let icon = if mime == "text/gradesta-url" {
                                "🌀 "
                            } else if mime == "text/x-url" {
                                "📎 "
                            } else if mime.starts_with("video/") {
                                "🎬 "
                            } else if mime.starts_with("audio/") {
                                "🔊 "
                            } else if mime.starts_with("text/") {
                                "📄 "
                            } else if !mime.is_empty() {
                                "📦 "
                            } else {
                                ""
                            };

                            painter.text(
                                rect.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{}{}", icon, display_label),
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
            
        } else if mime.starts_with("video/") || mime.starts_with("audio/") {
            // Video/Audio - offer to open externally
            let icon = if mime.starts_with("video/") { "🎬" } else { "🔊" };
            ui.heading(format!("{} Media", icon));
            ui.add_space(8.0);
            ui.label("This content type cannot be played inline.");
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
    if let Some(handle) = cache.textures.get(&id) {
        return Some(handle.clone());
    }

    let img = image::load_from_memory(data).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba.as_raw());
    let handle = ctx.load_texture(format!("vertex_{}", id), image, egui::TextureOptions::default());
    cache.textures.insert(id, handle.clone());
    Some(handle)
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

    if should_move {
        if let Some(edge_idx) = target_edge {
            let target_id = vertex.edges[edge_idx];
            if target_id != 0 {
                // Move cursor to target (even if it's a portal - auto_expand will handle following it)
                app_state.history.push(current_id);
                app_state.current_vertex = Some(target_id);
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
    let Some(current) = graph.vertices.get(&current_id) else { return };

    // FIRST: If we're sitting on a portal, auto-follow it immediately
    if current.mime.as_deref() == Some("text/gradesta-url") {
        let landmark_url = String::from_utf8_lossy(&current.label).to_string();

        // Check if this landmark was already loaded by looking up vertices associated with it
        if let Some(vertices) = graph.landmark_vertices.get(&landmark_url) {
            // Find the first non-portal vertex in this landmark
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
            let _ = cmd_tx.send(WsCommand::WatchLandmark(landmark_url.clone()));
        }

        // Set up to jump when it loads
        if app_state.following_portal.is_none() {
            app_state.following_portal = Some(landmark_url);
        }
        return;
    }
    
    // Collect vertices that are exactly 2 steps away
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
    
    // Only request ONE new landmark per frame to avoid flooding
    for vid in two_steps_away {
        if let Some(vertex) = graph.vertices.get(&vid) {
            if vertex.mime.as_deref() == Some("text/gradesta-url") {
                let landmark_url = String::from_utf8_lossy(&vertex.label).to_string();
                
                // Skip if already requested
                if app_state.requested_landmarks.contains(&landmark_url) {
                    continue;
                }
                
                // Mark as requested and send - only one per frame
                app_state.requested_landmarks.insert(landmark_url.clone());
                let _ = cmd_tx.send(WsCommand::WatchLandmark(landmark_url));
                return; // Only one per frame
            }
        }
    }
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
                WsCommand::WatchLandmark(landmark) => {
                    eprintln!("SEND WatchLandmark action=0 uri={:?}", landmark);
                    let mut buf = Vec::with_capacity(1 + 8 + landmark.len());
                    buf.push(MSG_CLIENT_WATCH_LANDMARK);
                    buf.extend_from_slice(&0u64.to_be_bytes());
                    buf.extend_from_slice(landmark.as_bytes());
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
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.into()),
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
            let (mime, label) = read_null_terminated(rest)?;
            let label_preview: String = String::from_utf8_lossy(label).chars().take(40).collect();
            eprintln!("RECV SetVertexLabel action={} vertex={} mime={:?} label={:?}... ({} bytes)", 
                action_id, vertex_id, mime, label_preview, label.len());
            Ok(ServerEvent::SetVertexLabel { vertex_id, mime, data: label.to_vec() })
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
            eprintln!("RECV SetEdges action={} vertex={} W={} E={} N={} S={} U={} D={}", 
                action_id, vertex_id, edges[0], edges[1], edges[2], edges[3], edges[4], edges[5]);
            Ok(ServerEvent::SetEdges { vertex_id, edges })
        }
        MSG_SERVER_LOG => {
            let (action_id, rest) = read_u64(&data[1..])?;
            let status = rest.get(0).copied().unwrap_or(0);
            let (vertex_id, rest) = read_u64(&rest[1..])?;
            let message = String::from_utf8(rest.to_vec())?;
            eprintln!("RECV Log action={} status={} vertex={} msg={:?}", action_id, status, vertex_id, message);
            Ok(ServerEvent::Log { message })
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

fn read_null_terminated(data: &[u8]) -> Result<(String, &[u8])> {
    let pos = data.iter().position(|b| *b == 0).ok_or_else(|| anyhow!("no null"))?;
    let s = String::from_utf8(data[..pos].to_vec())?;
    Ok((s, &data[pos + 1..]))
}

fn ingest_server_events(
    mut graph: ResMut<GraphState>,
    mut app_state: ResMut<AppState>,
    rx: Res<NetRx>,
) {
    while let Ok(event) = rx.0.try_recv() {
        match event {
            ServerEvent::Connected { base_url } => {
                app_state.connected = true;
                app_state.base_ws_url = Some(base_url);
                app_state.status = "Connected!".to_string();
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
            ServerEvent::SetVertexLabel { vertex_id, mime, data } => {
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;
                entry.label = data;
                entry.mime = Some(mime.clone());

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
                if graph.pending_jump_context.is_some() && mime != "text/gradesta-url" {
                    if let Some(current) = app_state.current_vertex {
                        app_state.history.push(current);
                    }
                    app_state.current_vertex = Some(vertex_id);
                    graph.pending_jump_context = None;
                } else if app_state.current_vertex.is_none() {
                    app_state.current_vertex = Some(vertex_id);
                }
            }
            ServerEvent::SetEdges { vertex_id, edges } => {
                let entry = graph.vertices.entry(vertex_id).or_default();
                entry.id = vertex_id;
                entry.edges = edges;
                
                if app_state.current_vertex.is_none() {
                    app_state.current_vertex = Some(vertex_id);
                }
            }
            ServerEvent::Log { message } => {
                app_state.status = format!("Server: {message}");
            }
            ServerEvent::RequestIdentification { action_id, nonce, timestamp, reason } => {
                // Check if we have any identities configured
                if app_state.identity_config.identities.is_empty() {
                    app_state.status = "Server requests identification, but no identities configured".to_string();
                    // Auto-refuse if no identities
                    // (We'd need WsCommandTx here to send the refusal - will handle in UI)
                } else {
                    // Get server URL from current connection
                    let server_url = app_state.base_ws_url.clone().unwrap_or_default();

                    // Check if this server is remembered for any identity
                    let remembered_identity = app_state.identity_config.identities.iter()
                        .position(|id| id.remembered_servers.contains(&server_url));

                    if let Some(idx) = remembered_identity {
                        // Auto-identify with remembered identity
                        app_state.selected_identity_index = idx;
                        // Set pending so UI can handle it
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
                        // Show consent dialog
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
}
