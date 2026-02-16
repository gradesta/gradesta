//! Elf execution context
//!
//! Provides the interface for elves to interact with the graph and stream output.

use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::protocol::{encode_elf_connect, encode_elf_output, encode_elf_complete, parse_elf_task};
use crate::ElfTask;

/// Context for elf operations during task execution
pub struct ElfContext {
    /// The task being executed
    pub task: ElfTask,
    /// Channel for sending output
    output_tx: mpsc::Sender<OutputMessage>,
    /// Handle to the WebSocket connection task
    _ws_handle: tokio::task::JoinHandle<()>,
}

enum OutputMessage {
    Text(String),
    Binary(Vec<u8>),
    Complete { status: u32, message: String },
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

        // Create channel for output messages
        let (output_tx, mut output_rx) = mpsc::channel::<OutputMessage>(100);

        // Spawn task to forward output to WebSocket
        let ws_handle = tokio::spawn(async move {
            while let Some(msg) = output_rx.recv().await {
                let binary = match msg {
                    OutputMessage::Text(text) => encode_elf_output(0, text.as_bytes()),
                    OutputMessage::Binary(data) => encode_elf_output(1, &data),
                    OutputMessage::Complete { status, message } => {
                        let complete = encode_elf_complete(status, &message);
                        if let Err(e) = write.send(Message::Binary(complete)).await {
                            log::error!("Failed to send complete: {}", e);
                        }
                        break;
                    }
                };
                if let Err(e) = write.send(Message::Binary(binary)).await {
                    log::error!("Failed to send output: {}", e);
                    break;
                }
            }
        });

        Ok(ElfContext {
            task,
            output_tx,
            _ws_handle: ws_handle,
        })
    }

    /// Stream text output to the browser
    pub async fn output(&self, text: &str) -> Result<()> {
        self.output_tx.send(OutputMessage::Text(text.to_string())).await
            .map_err(|_| anyhow!("Output channel closed"))
    }

    /// Stream a line of text output (with newline)
    pub async fn output_line(&self, text: &str) -> Result<()> {
        self.output(&format!("{}\n", text)).await
    }

    /// Stream binary output to the browser
    pub async fn output_binary(&self, data: &[u8]) -> Result<()> {
        self.output_tx.send(OutputMessage::Binary(data.to_vec())).await
            .map_err(|_| anyhow!("Output channel closed"))
    }

    /// Signal task completion
    pub async fn complete(&self, status: u32, message: &str) -> Result<()> {
        self.output_tx.send(OutputMessage::Complete {
            status,
            message: message.to_string(),
        }).await
            .map_err(|_| anyhow!("Output channel closed"))
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
