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

const MSG_CLIENT_WATCH_LANDMARK: u8 = 0x81;
const MSG_SERVER_SET_CONTEXT: u8 = 0x01;
const MSG_SERVER_SET_EDGES: u8 = 0x03;
const MSG_SERVER_SET_VERTEX_LABEL: u8 = 0x05;
const MSG_SERVER_LOG: u8 = 0x0F;

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            url_input: "ws://localhost:8080/ws?landmark=/home/".to_string(),
            status: "Enter URL and click Connect".to_string(),
            connected: false,
            current_vertex: None,
            history: Vec::new(),
            base_ws_url: None,
            requested_landmarks: HashSet::new(),
            following_portal: None,
        }
    }
}

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
        });
        ui.add_space(8.0);
    });

    // Bottom panel with navigation help
    egui::TopBottomPanel::bottom("help_panel").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Navigation: ↑↓←→ or WASD | PageUp/Down = Up/Down | Backspace = Back | Enter = Follow link");
        });
        ui.add_space(4.0);
    });

    // Right panel for content preview
    egui::SidePanel::right("preview_panel").min_width(400.0).show(ctx, |ui| {
        ui.heading("Content Preview");
        ui.separator();
        
        if let Some(current_id) = app_state.current_vertex {
            if let Some(vertex) = graph.vertices.get(&current_id) {
                render_vertex_content(ui, vertex, current_id, &mut media_cache, ctx);
            }
        } else {
            ui.label("No vertex selected");
        }
    });

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
        
        let cell_width = 160.0f32;
        let cell_height = 45.0f32;
        let padding = 4.0f32;
        
        let grid_width = (grid.max_x - grid.min_x + 1) as f32 * (cell_width + padding);
        let grid_height = (grid.max_y - grid.min_y + 1) as f32 * (cell_height + padding);
        
        let available = ui.available_size();
        let offset_x = (available.x - grid_width) / 2.0;
        let offset_y = 20.0;
        
        let painter = ui.painter();
        let base_pos = ui.min_rect().min + egui::vec2(offset_x.max(10.0), offset_y);
        
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
                        painter.line_segment([from, to], egui::Stroke::new(1.5, egui::Color32::from_rgb(70, 70, 80)));
                    }
                }
                
                if vertex.edges[EDGE_EAST] != 0 {
                    if let Some(&(tx, ty)) = grid.positions.get(&vertex.edges[EDGE_EAST]) {
                        let to_x = (tx - grid.min_x) as f32 * (cell_width + padding) + cell_width / 2.0;
                        let to_y = (ty - grid.min_y) as f32 * (cell_height + padding) + cell_height / 2.0;
                        let to = base_pos + egui::vec2(to_x, to_y);
                        painter.line_segment([from, to], egui::Stroke::new(1.5, egui::Color32::from_rgb(70, 70, 80)));
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
                    
                    // Different colors based on content type
                    let (bg_color, border_color) = if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        let mime = vertex.mime.as_deref().unwrap_or("");
                        if is_current {
                            (egui::Color32::from_rgb(50, 100, 70), egui::Color32::from_rgb(100, 200, 120))
                        } else if mime == "text/gradesta-url" {
                            (egui::Color32::from_rgb(60, 60, 90), egui::Color32::from_rgb(100, 100, 150))
                        } else if mime.starts_with("image/") {
                            (egui::Color32::from_rgb(70, 50, 70), egui::Color32::from_rgb(140, 100, 140))
                        } else if mime.starts_with("text/") {
                            (egui::Color32::from_rgb(50, 60, 70), egui::Color32::from_rgb(100, 120, 140))
                        } else if mime.starts_with("video/") || mime.starts_with("audio/") {
                            (egui::Color32::from_rgb(70, 60, 50), egui::Color32::from_rgb(140, 120, 100))
                        } else {
                            (egui::Color32::from_rgb(50, 50, 55), egui::Color32::from_rgb(80, 80, 90))
                        }
                    } else {
                        (egui::Color32::from_rgb(50, 50, 55), egui::Color32::from_rgb(80, 80, 90))
                    };
                    
                    painter.rect_filled(rect, 4.0, bg_color);
                    painter.rect_stroke(rect, 4.0, egui::Stroke::new(2.0, border_color));
                    
                    if let Some(vertex) = graph.vertices.get(&vertex_id) {
                        let label = String::from_utf8_lossy(&vertex.label);
                        let display_label: String = label.chars().take(18).collect();
                        let display_label = if label.len() > 18 {
                            format!("{}…", display_label)
                        } else {
                            display_label
                        };
                        
                        let mime = vertex.mime.as_deref().unwrap_or("");
                        let icon = if mime == "text/gradesta-url" {
                            "🌀 "
                        } else if mime == "text/x-url" {
                            "📎 "
                        } else if mime.starts_with("image/") {
                            "🖼 "
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
                            egui::FontId::proportional(13.0),
                            egui::Color32::WHITE,
                        );
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
) {
    let label = String::from_utf8_lossy(&vertex.label);
    
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.monospace(&*label);
    });
    
    if let Some(mime) = &vertex.mime {
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
            ui.label(format!("Loading: {}", url));
            
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
            
        } else if mime.starts_with("text/") {
            // Text content
            ui.heading("📄 Text Content");
            ui.add_space(8.0);
            let text = String::from_utf8_lossy(&vertex.label);
            egui::ScrollArea::vertical().max_height(500.0).show(ui, |ui| {
                ui.add(egui::TextEdit::multiline(&mut text.to_string())
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .interactive(false));
            });
            
        } else if mime.starts_with("image/") {
            // Image content
            ui.heading("🖼 Image");
            ui.add_space(8.0);
            
            if mime == "image/gif" {
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
                if let Some(tex) = get_or_load_texture(vertex_id, &vertex.label, mime, media_cache, ctx) {
                    let size = tex.size_vec2();
                    let max_size = egui::vec2(380.0, 400.0);
                    let scale = (max_size.x / size.x).min(max_size.y / size.y).min(1.0);
                    ui.image((tex.id(), size * scale));
                    ui.label(format!("{}x{}", size.x as u32, size.y as u32));
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

    let mut target_edge: Option<usize> = None;

    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        target_edge = Some(EDGE_NORTH);
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        target_edge = Some(EDGE_SOUTH);
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        target_edge = Some(EDGE_WEST);
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        target_edge = Some(EDGE_EAST);
    } else if keys.just_pressed(KeyCode::PageUp) {
        target_edge = Some(EDGE_UP);
    } else if keys.just_pressed(KeyCode::PageDown) {
        target_edge = Some(EDGE_DOWN);
    }

    if keys.just_pressed(KeyCode::Backspace) {
        if let Some(prev_id) = app_state.history.pop() {
            app_state.current_vertex = Some(prev_id);
        }
        return;
    }

    if let Some(edge_idx) = target_edge {
        let target_id = vertex.edges[edge_idx];
        if target_id != 0 {
            // Move cursor to target (even if it's a portal - auto_expand will handle following it)
            app_state.history.push(current_id);
            app_state.current_vertex = Some(target_id);
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
        }
    }
}
