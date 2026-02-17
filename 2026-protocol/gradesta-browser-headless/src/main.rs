//! Headless browser for testing Gradesta protocol
//!
//! Reads JSON commands from stdin, outputs events as JSON to stdout.

mod protocol;

use anyhow::{anyhow, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

static ACTION_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_action_id() -> u64 {
    ACTION_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Parser, Debug)]
#[command(name = "gradesta-browser-headless")]
#[command(about = "Headless browser for testing Gradesta protocol")]
struct Args {
    /// WebSocket URL to connect to
    #[arg(short, long, default_value = "ws://localhost:8083/ws")]
    url: String,
}

// ============================================================================
// Input Commands (stdin JSON)
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd")]
#[serde(rename_all = "snake_case")]
enum Command {
    WatchLandmark { landmark: String },
    CreateVertex {
        from: u64,
        direction: String,
        mime: String,
        content: String,
    },
    SetVertex {
        id: u64,
        mime: Option<String>,
        content: String,
    },
    ClickVertex { id: u64 },
    IntroduceElf {
        url: String,
        command: String,
        vertex: u64,
        landmark: Option<String>,
    },
    Quit,
}

// ============================================================================
// Output Events (stdout JSON)
// ============================================================================

#[derive(Debug, Serialize)]
#[serde(tag = "event")]
#[serde(rename_all = "snake_case")]
enum Event {
    Connected,
    Context {
        action_id: u64,
        landmark: String,
    },
    Vertex {
        action_id: u64,
        id: u64,
        layer: u32,
        mime: String,
        content: String,
    },
    Edges {
        action_id: u64,
        id: u64,
        west: u64,
        east: u64,
        north: u64,
        south: u64,
        up: u64,
        down: u64,
        edit_mask: u8,
    },
    Log {
        action_id: u64,
        status: u32,
        vertex_id: u64,
        message: String,
    },
    IntroductionToken {
        action_id: u64,
        token: String,
        server_ws_url: String,
    },
    Error {
        message: String,
    },
    Disconnected,
}

fn output_event(event: &Event) {
    if let Ok(json) = serde_json::to_string(event) {
        println!("{}", json);
    }
}

// ============================================================================
// State
// ============================================================================

struct BrowserState {
    current_landmark: Option<String>,
    vertices: HashMap<u64, VertexData>,
    pending_introductions: HashMap<u64, PendingIntroduction>,
}

struct VertexData {
    mime: String,
    content: Vec<u8>,
}

struct PendingIntroduction {
    elf_url: String,
    command: String,
}

impl BrowserState {
    fn new() -> Self {
        Self {
            current_landmark: None,
            vertices: HashMap::new(),
            pending_introductions: HashMap::new(),
        }
    }
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    log::info!("Connecting to {}", args.url);

    let (ws_stream, _) = connect_async(&args.url).await?;
    let (mut ws_write, mut ws_read) = ws_stream.split();

    output_event(&Event::Connected);

    let mut state = BrowserState::new();

    // Channel for stdin commands
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(100);

    // Spawn stdin reader
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<Command>(line) {
                Ok(cmd) => {
                    if cmd_tx.send(cmd).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    output_event(&Event::Error {
                        message: format!("Invalid JSON: {}", e),
                    });
                }
            }
        }
    });

    loop {
        tokio::select! {
            // Handle WebSocket messages
            msg = ws_read.next() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        handle_server_message(&data, &mut state);
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        output_event(&Event::Disconnected);
                        break;
                    }
                    Some(Err(e)) => {
                        output_event(&Event::Error {
                            message: format!("WebSocket error: {}", e),
                        });
                        break;
                    }
                    _ => {}
                }
            }

            // Handle stdin commands
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(Command::Quit) | None => {
                        break;
                    }
                    Some(cmd) => {
                        if let Err(e) = handle_command(cmd, &mut state, &mut ws_write).await {
                            output_event(&Event::Error {
                                message: format!("Command error: {}", e),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_server_message(data: &[u8], state: &mut BrowserState) {
    if data.is_empty() {
        return;
    }

    match data[0] {
        protocol::MSG_SERVER_SET_CONTEXT => {
            if let Ok((action_id, landmark)) = protocol::parse_set_context(data) {
                state.current_landmark = Some(landmark.clone());
                output_event(&Event::Context { action_id, landmark });
            }
        }
        protocol::MSG_SERVER_SET_VERTEX_LABEL => {
            if let Ok((action_id, vertex_id, layer, mime, content)) = protocol::parse_set_vertex_label(data) {
                // Store vertex data
                state.vertices.insert(vertex_id, VertexData {
                    mime: mime.clone(),
                    content: content.clone(),
                });

                // Try to decode content as UTF-8 for output
                let content_str = String::from_utf8_lossy(&content).to_string();

                output_event(&Event::Vertex {
                    action_id,
                    id: vertex_id,
                    layer,
                    mime,
                    content: content_str,
                });
            }
        }
        protocol::MSG_SERVER_SET_EDGES => {
            if let Ok((action_id, vertex_id, edges, edit_mask)) = protocol::parse_set_edges(data) {
                output_event(&Event::Edges {
                    action_id,
                    id: vertex_id,
                    west: edges[0],
                    east: edges[1],
                    north: edges[2],
                    south: edges[3],
                    up: edges[4],
                    down: edges[5],
                    edit_mask,
                });
            }
        }
        protocol::MSG_SERVER_LOG_MESSAGE => {
            if let Ok((action_id, status, vertex_id, message)) = protocol::parse_log_message(data) {
                output_event(&Event::Log {
                    action_id,
                    status,
                    vertex_id,
                    message,
                });
            }
        }
        protocol::MSG_SERVER_INTRODUCTION_TOKEN => {
            if let Ok((action_id, token, server_ws_url)) = protocol::parse_introduction_token(data) {
                output_event(&Event::IntroductionToken {
                    action_id,
                    token: token.clone(),
                    server_ws_url: server_ws_url.clone(),
                });

                // If we have a pending introduction, POST to the elf
                if let Some(intro) = state.pending_introductions.remove(&action_id) {
                    let elf_url = intro.elf_url;
                    let command = intro.command;
                    tokio::spawn(async move {
                        if let Err(e) = summon_elf(&elf_url, &token, &server_ws_url, &command).await {
                            output_event(&Event::Error {
                                message: format!("Failed to summon elf: {}", e),
                            });
                        }
                    });
                }
            }
        }
        _ => {
            log::debug!("Unknown message type: 0x{:02X}", data[0]);
        }
    }
}

async fn handle_command<W>(
    cmd: Command,
    state: &mut BrowserState,
    ws_write: &mut W,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    match cmd {
        Command::WatchLandmark { landmark } => {
            let action_id = next_action_id();
            let msg = protocol::encode_watch_landmark(action_id, &landmark);
            ws_write.send(Message::Binary(msg)).await?;
            state.current_landmark = Some(landmark);
        }
        Command::CreateVertex { from, direction, mime, content } => {
            let action_id = next_action_id();
            let dir_byte = protocol::direction_to_byte(&direction)?;
            let msg = protocol::encode_create_vertex(
                action_id,
                from,
                dir_byte,
                0, // layer 0
                &mime,
                content.as_bytes(),
            );
            ws_write.send(Message::Binary(msg)).await?;
        }
        Command::SetVertex { id, mime, content } => {
            let action_id = next_action_id();
            let mime = mime.unwrap_or_else(|| "text/plain".to_string());
            let msg = protocol::encode_set_vertex_label(
                action_id,
                id,
                0, // layer 0
                &mime,
                content.as_bytes(),
            );
            ws_write.send(Message::Binary(msg)).await?;
        }
        Command::ClickVertex { id } => {
            let action_id = next_action_id();
            let msg = protocol::encode_click_vertex(action_id, id);
            ws_write.send(Message::Binary(msg)).await?;
        }
        Command::IntroduceElf { url, command, vertex, landmark } => {
            let action_id = next_action_id();
            let landmark = landmark
                .or_else(|| state.current_landmark.clone())
                .ok_or_else(|| anyhow!("No landmark set"))?;

            // Store pending introduction
            state.pending_introductions.insert(action_id, PendingIntroduction {
                elf_url: url.clone(),
                command: command.clone(),
            });

            let msg = protocol::encode_introduce_elf(
                action_id,
                &url,
                &command,
                &landmark,
                vertex,
                vertex, // region origin = cursor
                0x03,   // read + write permissions
            );
            ws_write.send(Message::Binary(msg)).await?;
        }
        Command::Quit => {
            // Handled in main loop
        }
    }
    Ok(())
}

async fn summon_elf(elf_url: &str, token: &str, server_ws_url: &str, command: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let summon_url = format!("{}/summon", elf_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "token": token,
        "server_ws_url": server_ws_url,
        "command": command,
        "params": {}
    });

    let response = client
        .post(&summon_url)
        .json(&body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!("Elf returned status: {}", response.status()));
    }

    Ok(())
}
