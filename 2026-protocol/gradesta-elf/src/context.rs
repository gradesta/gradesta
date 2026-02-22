//! Elf execution context
//!
//! Provides the interface for elves to interact with the graph and stream output.

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::protocol::{
    encode_elf_connect, encode_elf_output, encode_elf_complete, parse_elf_task,
    encode_set_vertex_label, encode_click_vertex, encode_create_vertex,
    parse_server_set_vertex_label, parse_server_log_message, parse_server_set_edges,
    MSG_SERVER_SET_VERTEX_LABEL, MSG_SERVER_LOG_MESSAGE, MSG_SERVER_SET_EDGES,
    DIR_SOUTH,
};
use crate::ElfTask;

/// Context for elf operations during task execution
pub struct ElfContext {
    /// The task being executed
    pub task: ElfTask,
    /// Channel for sending commands to the WebSocket task
    cmd_tx: mpsc::Sender<WsCommand>,
    /// Handle to the WebSocket connection task
    _ws_handle: tokio::task::JoinHandle<()>,
    /// Action ID counter
    next_action_id: std::sync::atomic::AtomicU64,
}

/// Commands sent to the WebSocket handler task
enum WsCommand {
    /// Send output to browser
    Output { output_type: u8, data: Vec<u8> },
    /// Signal completion
    Complete { status: u32, message: String },
    /// Read a vertex and send response back
    ReadVertex { vertex_id: u64, response_tx: oneshot::Sender<Result<(String, Vec<u8>)>> },
    /// Set a vertex and send ack back
    SetVertex { vertex_id: u64, mime: String, content: Vec<u8>, response_tx: oneshot::Sender<Result<()>> },
    /// Create a vertex and return the new vertex ID
    CreateVertex { from_vertex: u64, direction: u8, mime: String, content: Vec<u8>, response_tx: oneshot::Sender<Result<u64>> },
}

impl ElfContext {
    /// Connect to the server with the given token and wait for the task
    pub async fn connect(server_ws_url: &str, token: &str) -> Result<Self> {
        log::info!("Connecting to server: {}", server_ws_url);

        let (ws_stream, _) = connect_async(server_ws_url).await
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Send ELF_CONNECT with token
        let connect_msg = encode_elf_connect(token);
        write.send(Message::Binary(connect_msg)).await
            .map_err(|e| anyhow!("Failed to send connect: {}", e))?;

        log::info!("Sent ELF_CONNECT, waiting for task...");

        // Wait for ELF_TASK
        let task = loop {
            match read.next().await {
                Some(Ok(Message::Binary(data))) => {
                    if !data.is_empty() && data[0] == crate::protocol::MSG_SERVER_ELF_TASK {
                        break parse_elf_task(&data)?;
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(anyhow!("WebSocket error: {}", e)),
                None => return Err(anyhow!("Connection closed before receiving task")),
            }
        };

        log::info!("Received task: command={}", task.command);

        // Create channel for commands to the WebSocket handler
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsCommand>(100);

        // Spawn task to handle WebSocket communication
        let ws_handle = tokio::spawn(async move {
            // Map to track pending read requests: vertex_id -> response channel
            let mut pending_reads: std::collections::HashMap<u64, oneshot::Sender<Result<(String, Vec<u8>)>>> = std::collections::HashMap::new();
            // Map to track pending write requests: action_id -> response channel
            let mut pending_writes: std::collections::HashMap<u64, oneshot::Sender<Result<()>>> = std::collections::HashMap::new();
            // Map to track pending create requests: action_id -> response channel
            let mut pending_creates: std::collections::HashMap<u64, oneshot::Sender<Result<u64>>> = std::collections::HashMap::new();
            let mut action_counter: u64 = 1;

            loop {
                tokio::select! {
                    // Handle commands from the elf
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(WsCommand::Output { output_type, data }) => {
                                let msg = encode_elf_output(output_type, &data);
                                if let Err(e) = write.send(Message::Binary(msg)).await {
                                    log::error!("Failed to send output: {}", e);
                                    break;
                                }
                            }
                            Some(WsCommand::Complete { status, message }) => {
                                let msg = encode_elf_complete(status, &message);
                                if let Err(e) = write.send(Message::Binary(msg)).await {
                                    log::error!("Failed to send complete: {}", e);
                                }
                                break;
                            }
                            Some(WsCommand::ReadVertex { vertex_id, response_tx }) => {
                                let action_id = action_counter;
                                action_counter += 1;
                                pending_reads.insert(vertex_id, response_tx);
                                let msg = encode_click_vertex(action_id, vertex_id);
                                if let Err(e) = write.send(Message::Binary(msg)).await {
                                    log::error!("Failed to send click: {}", e);
                                }
                            }
                            Some(WsCommand::SetVertex { vertex_id, mime, content, response_tx }) => {
                                let action_id = action_counter;
                                action_counter += 1;
                                pending_writes.insert(action_id, response_tx);
                                let msg = encode_set_vertex_label(action_id, vertex_id, &mime, &content);
                                if let Err(e) = write.send(Message::Binary(msg)).await {
                                    log::error!("Failed to send set vertex: {}", e);
                                }
                            }
                            Some(WsCommand::CreateVertex { from_vertex, direction, mime, content, response_tx }) => {
                                let action_id = action_counter;
                                action_counter += 1;
                                pending_creates.insert(action_id, response_tx);
                                let msg = encode_create_vertex(action_id, from_vertex, direction, &mime, &content);
                                if let Err(e) = write.send(Message::Binary(msg)).await {
                                    log::error!("Failed to send create vertex: {}", e);
                                }
                            }
                            None => break,
                        }
                    }
                    // Handle messages from the server
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Binary(data))) => {
                                if data.is_empty() {
                                    continue;
                                }
                                match data[0] {
                                    MSG_SERVER_SET_VERTEX_LABEL => {
                                        if let Ok((_, vertex_id, _, mime, content)) = parse_server_set_vertex_label(&data) {
                                            if let Some(tx) = pending_reads.remove(&vertex_id) {
                                                let _ = tx.send(Ok((mime, content)));
                                            }
                                        }
                                    }
                                    MSG_SERVER_LOG_MESSAGE => {
                                        if let Ok((action_id, status, vertex_id, message)) = parse_server_log_message(&data) {
                                            // Check if this is a response to a write
                                            if let Some(tx) = pending_writes.remove(&action_id) {
                                                if status == 200 {
                                                    let _ = tx.send(Ok(()));
                                                } else {
                                                    let _ = tx.send(Err(anyhow!("Server error {}: {}", status, message)));
                                                }
                                            }
                                            // Check if this is a response to a create (after SetEdges)
                                            if let Some(tx) = pending_creates.remove(&action_id) {
                                                if status == 200 {
                                                    let _ = tx.send(Ok(vertex_id));
                                                } else {
                                                    let _ = tx.send(Err(anyhow!("Create failed {}: {}", status, message)));
                                                }
                                            }
                                        }
                                    }
                                    MSG_SERVER_SET_EDGES => {
                                        // SetEdges comes before LogMessage for create, contains the new vertex ID
                                        // We don't need to handle it specially since LogMessage also contains the vertex ID
                                        log::debug!("Received SetEdges");
                                    }
                                    _ => {
                                        log::debug!("Ignoring message type: 0x{:02x}", data[0]);
                                    }
                                }
                            }
                            Some(Ok(_)) => continue,
                            Some(Err(e)) => {
                                log::error!("WebSocket error: {}", e);
                                break;
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Ok(ElfContext {
            task,
            cmd_tx,
            _ws_handle: ws_handle,
            next_action_id: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Stream text output to the browser
    pub async fn output(&self, text: &str) -> Result<()> {
        self.cmd_tx.send(WsCommand::Output {
            output_type: 0,
            data: text.as_bytes().to_vec(),
        }).await
            .map_err(|_| anyhow!("Command channel closed"))
    }

    /// Stream a line of text output (with newline)
    pub async fn output_line(&self, text: &str) -> Result<()> {
        self.output(&format!("{}\n", text)).await
    }

    /// Stream binary output to the browser
    pub async fn output_binary(&self, data: &[u8]) -> Result<()> {
        self.cmd_tx.send(WsCommand::Output {
            output_type: 1,
            data: data.to_vec(),
        }).await
            .map_err(|_| anyhow!("Command channel closed"))
    }

    /// Signal task completion
    pub async fn complete(&self, status: u32, message: &str) -> Result<()> {
        self.cmd_tx.send(WsCommand::Complete {
            status,
            message: message.to_string(),
        }).await
            .map_err(|_| anyhow!("Command channel closed"))
    }

    /// Read the content of a vertex
    /// Returns (mime_type, content)
    pub async fn read_vertex(&self, vertex_id: u64) -> Result<(String, Vec<u8>)> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(WsCommand::ReadVertex {
            vertex_id,
            response_tx: tx,
        }).await
            .map_err(|_| anyhow!("Command channel closed"))?;

        rx.await
            .map_err(|_| anyhow!("Response channel closed"))?
    }

    /// Read the cursor vertex content
    /// This returns the content that was sent with the task - no network request needed
    pub async fn read_cursor(&self) -> Result<(String, Vec<u8>)> {
        Ok((self.task.cursor_mime.clone(), self.task.cursor_content.clone()))
    }

    /// Set the content of a vertex
    pub async fn set_vertex(&self, vertex_id: u64, mime: &str, content: &[u8]) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(WsCommand::SetVertex {
            vertex_id,
            mime: mime.to_string(),
            content: content.to_vec(),
            response_tx: tx,
        }).await
            .map_err(|_| anyhow!("Command channel closed"))?;

        rx.await
            .map_err(|_| anyhow!("Response channel closed"))?
    }

    /// Set the cursor vertex content
    pub async fn set_cursor(&self, mime: &str, content: &[u8]) -> Result<()> {
        self.set_vertex(self.task.cursor_vertex, mime, content).await
    }

    /// Create a new vertex in a direction from another vertex
    /// Returns the new vertex ID
    pub async fn create_vertex(&self, from_vertex: u64, direction: u8, mime: &str, content: &[u8]) -> Result<u64> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(WsCommand::CreateVertex {
            from_vertex,
            direction,
            mime: mime.to_string(),
            content: content.to_vec(),
            response_tx: tx,
        }).await
            .map_err(|_| anyhow!("Command channel closed"))?;

        rx.await
            .map_err(|_| anyhow!("Response channel closed"))?
    }

    /// Create a vertex south of another vertex
    pub async fn create_south(&self, from_vertex: u64, mime: &str, content: &[u8]) -> Result<u64> {
        self.create_vertex(from_vertex, DIR_SOUTH, mime, content).await
    }

    /// Create a vertex south of the cursor
    pub async fn create_south_of_cursor(&self, mime: &str, content: &[u8]) -> Result<u64> {
        self.create_south(self.task.cursor_vertex, mime, content).await
    }

    /// Get the command name
    pub fn command(&self) -> &str {
        &self.task.command
    }

    /// Get the cursor vertex ID
    pub fn cursor_vertex(&self) -> u64 {
        self.task.cursor_vertex
    }

    /// Get a parameter value
    pub fn param(&self, key: &str) -> Option<&String> {
        self.task.params.get(key)
    }

    /// Get the region origin vertex
    pub fn region_origin(&self) -> u64 {
        self.task.region.origin_vertex
    }

    /// Check if a direction is allowed in the region
    pub fn region_allows_direction(&self, direction: u8) -> bool {
        (self.task.region.allowed_directions & direction) != 0
    }
}
