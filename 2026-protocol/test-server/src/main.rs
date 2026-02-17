//! Standalone test server for Gradesta protocol testing
//!
//! This is a minimal server that implements the Gradesta protocol with file-based
//! storage and no authentication. Used for testing elves and headless browsers.

mod protocol;
mod storage;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;

use crate::protocol::*;
use crate::storage::{NotesIndex, StorageClient};

/// Test server for Gradesta protocol
#[derive(Parser, Debug)]
#[command(name = "test-server")]
#[command(version, about)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8083)]
    port: u16,

    /// Storage directory
    #[arg(short, long, default_value = "/tmp/gradesta-test")]
    storage: String,
}

/// Elf invitation
#[derive(Clone, Debug)]
struct Invitation {
    token: String,
    region_origin: u64,
    permissions: u8,
    command: String,
    params: HashMap<String, String>,
    browser_conn_id: u64,
}

/// Shared server state
struct ServerState {
    storage: StorageClient,
    index: Mutex<NotesIndex>,
    /// Pending elf invitations: token -> invitation
    invitations: Mutex<HashMap<String, Invitation>>,
    /// Connection counter
    conn_counter: std::sync::atomic::AtomicU64,
    /// Server port for introduction tokens
    port: u16,
}

#[derive(Clone)]
struct AppState {
    inner: Arc<ServerState>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let storage_path = PathBuf::from(&args.storage);

    // Create storage directory
    std::fs::create_dir_all(&storage_path)?;

    let storage = StorageClient::new(&storage_path);
    let index = storage.load_index().await?;

    println!("Test Server v{}", env!("CARGO_PKG_VERSION"));
    println!("Storage: {}", storage_path.display());
    println!("Listening on: ws://localhost:{}/ws", args.port);
    println!();

    let state = AppState {
        inner: Arc::new(ServerState {
            storage,
            index: Mutex::new(index),
            invitations: Mutex::new(HashMap::new()),
            conn_counter: std::sync::atomic::AtomicU64::new(1),
            port: args.port,
        }),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_connection(socket, state))
}

async fn handle_connection(socket: WebSocket, state: AppState) {
    let conn_id = state.inner.conn_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    log::info!("New connection: {}", conn_id);

    if let Err(e) = handle_connection_inner(socket, state, conn_id).await {
        log::error!("Connection {} error: {}", conn_id, e);
    }

    log::info!("Connection {} closed", conn_id);
}

async fn handle_connection_inner(socket: WebSocket, state: AppState, conn_id: u64) -> Result<()> {
    let (mut write, mut read) = socket.split();

    // Wait for first message to determine connection type
    let first_msg = match read.next().await {
        Some(Ok(Message::Binary(data))) if !data.is_empty() => data,
        Some(Ok(_)) => vec![],
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()),
    };

    // Check if this is an elf connection
    if !first_msg.is_empty() && first_msg[0] == MSG_ELF_CONNECT {
        return handle_elf_connection(first_msg, state, &mut write, &mut read).await;
    }

    // Browser connection - no auth needed, go straight to browsing
    let identity = format!("local://conn-{}", conn_id);
    log::info!("Browser connection from {}", identity);

    // Process first message if present
    if !first_msg.is_empty() {
        handle_browser_message(&first_msg, &state, conn_id, &identity, &mut write).await?;
    }

    // Message loop
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Binary(data) = msg {
            if !data.is_empty() {
                handle_browser_message(&data, &state, conn_id, &identity, &mut write).await?;
            }
        }
    }

    Ok(())
}

async fn handle_browser_message<W>(
    data: &[u8],
    state: &AppState,
    conn_id: u64,
    identity: &str,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    let msg_type = data[0];

    match msg_type {
        MSG_CLIENT_WATCH_LANDMARK => {
            let (action_id, landmark) = parse_watch_landmark(data)?;
            log::info!("WatchLandmark: action={} landmark={}", action_id, landmark);

            // Send context
            let ctx_msg = encode_set_context(action_id, &landmark);
            write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            // Send vertices from index
            let index = state.inner.index.lock().await;
            for vertex in &index.vertices {
                let vertex_id = vertex.id_hash();

                // Load content
                let content = state.inner.storage.download(&vertex.file).await
                    .unwrap_or_else(|_| b"(content not found)".to_vec());

                let label_msg = encode_set_vertex_label(action_id, vertex_id, &vertex.mime, &content);
                write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

                // Build edges
                let edges = index.build_edges(vertex.id);
                let edges_msg = encode_set_edges(action_id, vertex_id, edges, 0x7F);
                write.send(Message::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }

            // If empty, send a placeholder
            if index.vertices.is_empty() {
                let empty_id = 1u64;
                let label_msg = encode_set_vertex_label(action_id, empty_id, "text/plain", b"(empty - press 'i' to create)");
                write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                let edges_msg = encode_set_edges(action_id, empty_id, [0; 6], 0x7F);
                write.send(Message::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
        }

        MSG_CLIENT_SET_VERTEX_LABEL => {
            let (action_id, vertex_id, layer, mime, content) = parse_set_vertex_label(data)?;
            log::info!("SetVertexLabel: action={} vertex={} layer={} mime={}", action_id, vertex_id, layer, mime);

            // Find and update vertex
            let mut index = state.inner.index.lock().await;
            if let Some(vertex) = index.find_vertex_mut(vertex_id) {
                // Save content
                state.inner.storage.upload(&vertex.file, &content).await?;
                vertex.mime = mime;

                // Save index
                state.inner.storage.save_index(&index).await?;

                let log_msg = encode_log_message(action_id, 200, vertex_id, "OK");
                write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            } else {
                let log_msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
                write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
        }

        MSG_CLIENT_CREATE_VERTEX => {
            let (action_id, from_vertex, direction, layer, mime, content) = parse_create_vertex(data)?;
            log::info!("CreateVertex: action={} from={} dir={}", action_id, from_vertex, direction);

            let mut index = state.inner.index.lock().await;

            // Create new vertex
            let new_id = uuid::Uuid::new_v4();
            let file_path = format!("content/{}.{}", new_id, mime_to_ext(&mime));

            // Save content
            state.inner.storage.upload(&file_path, &content).await?;

            // Add vertex to index
            let vertex = storage::Vertex::new(new_id, &mime, &file_path);
            let vertex_hash = vertex.id_hash();
            index.vertices.push(vertex);

            // Add edge from source
            if from_vertex != 0 {
                if let Some(from_uuid) = index.find_uuid(from_vertex) {
                    index.add_edge(from_uuid, new_id, direction);
                }
            }

            // Save index
            state.inner.storage.save_index(&index).await?;

            // Send edges for new vertex
            let edges = index.build_edges(new_id);
            let edges_msg = encode_set_edges(action_id, vertex_hash, edges, 0x7F);
            write.send(Message::Binary(edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

            // Send updated edges for source
            if from_vertex != 0 {
                if let Some(from_uuid) = index.find_uuid(from_vertex) {
                    let from_edges = index.build_edges(from_uuid);
                    let from_edges_msg = encode_set_edges(action_id, from_vertex, from_edges, 0x7F);
                    write.send(Message::Binary(from_edges_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                }
            }

            let log_msg = encode_log_message(action_id, 200, vertex_hash, "Created");
            write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }

        MSG_CLIENT_CLICK_VERTEX => {
            let (action_id, vertex_id) = parse_click_vertex(data)?;
            log::info!("ClickVertex: action={} vertex={}", action_id, vertex_id);

            let index = state.inner.index.lock().await;
            if let Some(vertex) = index.find_vertex(vertex_id) {
                let content = state.inner.storage.download(&vertex.file).await
                    .unwrap_or_else(|_| b"(content not found)".to_vec());

                let label_msg = encode_set_vertex_label(action_id, vertex_id, &vertex.mime, &content);
                write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            } else {
                let log_msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
                write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            }
        }

        MSG_CLIENT_INTRODUCE_ELF => {
            let (action_id, elf_url, command, cursor_landmark, cursor_vertex, region, permissions, params) =
                parse_introduce_elf(data)?;
            log::info!("IntroduceElf: action={} elf_url={} command={}", action_id, elf_url, command);

            // Generate token
            let token = generate_token();

            // Store invitation
            {
                let mut invitations = state.inner.invitations.lock().await;
                invitations.insert(token.clone(), Invitation {
                    token: token.clone(),
                    region_origin: region.origin_vertex,
                    permissions,
                    command,
                    params,
                    browser_conn_id: conn_id,
                });
            }

            // Send token back - TODO: get actual port from config
            let server_ws_url = format!("ws://localhost:{}/ws", state.inner.port);
            let token_msg = encode_introduction_token(action_id, &token, &server_ws_url);
            write.send(Message::Binary(token_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
        }

        _ => {
            log::warn!("Unknown message type: 0x{:02x}", msg_type);
        }
    }

    Ok(())
}

async fn handle_elf_connection<W, R>(
    first_msg: Vec<u8>,
    state: AppState,
    write: &mut W,
    read: &mut R,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
    R: StreamExt<Item = Result<Message, axum::Error>> + Unpin,
{
    let token = parse_elf_connect(&first_msg)?;
    log::info!("Elf connecting with token: {}...", &token[..std::cmp::min(8, token.len())]);

    // Validate and consume invitation
    let invitation = {
        let mut invitations = state.inner.invitations.lock().await;
        invitations.remove(&token)
            .ok_or_else(|| anyhow!("Invalid invitation token"))?
    };

    log::info!("Elf invitation validated: command={}", invitation.command);

    // Send ELF_TASK
    let task_msg = encode_elf_task(
        &invitation.command,
        "local://notes",
        invitation.region_origin,
        invitation.region_origin,
        invitation.permissions,
        &invitation.params,
    );
    write.send(Message::Binary(task_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
    log::info!("Sent ELF_TASK to elf");

    // Handle elf messages
    while let Some(msg) = read.next().await {
        let msg = msg?;
        if let Message::Binary(data) = msg {
            if data.is_empty() {
                continue;
            }

            let msg_type = data[0];
            match msg_type {
                MSG_ELF_OUTPUT => {
                    let (output_type, output_data) = parse_elf_output(&data)?;
                    if output_type == 0 {
                        if let Ok(text) = String::from_utf8(output_data) {
                            log::info!("Elf output: {}", text.trim());
                        }
                    }
                }
                MSG_ELF_COMPLETE => {
                    let (status, message) = parse_elf_complete(&data)?;
                    log::info!("Elf complete: status={} message={}", status, message);
                    break;
                }
                MSG_CLIENT_SET_VERTEX_LABEL => {
                    // Check write permission
                    if (invitation.permissions & PERM_WRITE) == 0 {
                        let log_msg = encode_log_message(0, 403, 0, "Write permission denied");
                        write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                        continue;
                    }

                    let (action_id, vertex_id, layer, mime, content) = parse_set_vertex_label(&data)?;
                    log::info!("Elf SetVertexLabel: vertex={} mime={}", vertex_id, mime);

                    let mut index = state.inner.index.lock().await;
                    if let Some(vertex) = index.find_vertex_mut(vertex_id) {
                        state.inner.storage.upload(&vertex.file, &content).await?;
                        vertex.mime = mime;
                        state.inner.storage.save_index(&index).await?;

                        let log_msg = encode_log_message(action_id, 200, vertex_id, "OK");
                        write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    } else {
                        let log_msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
                        write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    }
                }
                MSG_CLIENT_CLICK_VERTEX => {
                    // Check read permission
                    if (invitation.permissions & PERM_READ) == 0 {
                        let log_msg = encode_log_message(0, 403, 0, "Read permission denied");
                        write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                        continue;
                    }

                    let (action_id, vertex_id) = parse_click_vertex(&data)?;
                    log::info!("Elf ClickVertex: vertex={}", vertex_id);

                    let index = state.inner.index.lock().await;
                    if let Some(vertex) = index.find_vertex(vertex_id) {
                        let content = state.inner.storage.download(&vertex.file).await
                            .unwrap_or_else(|_| b"(not found)".to_vec());

                        let label_msg = encode_set_vertex_label(action_id, vertex_id, &vertex.mime, &content);
                        write.send(Message::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    } else {
                        let log_msg = encode_log_message(action_id, 404, vertex_id, "Vertex not found");
                        write.send(Message::Binary(log_msg)).await.map_err(|e| anyhow!("{:?}", e))?;
                    }
                }
                _ => {
                    log::warn!("Unknown elf message type: 0x{:02x}", msg_type);
                }
            }
        }
    }

    log::info!("Elf connection closed");
    Ok(())
}

fn generate_token() -> String {
    use rand::Rng;
    let bytes: [u8; 32] = rand::thread_rng().gen();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

fn mime_to_ext(mime: &str) -> &str {
    match mime {
        "text/plain" => "txt",
        "text/markdown" => "md",
        "audio/ogg" => "ogg",
        "audio/webm" => "webm",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "bin",
    }
}
